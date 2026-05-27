//! Smart npm registry source picker.
//!
//! Probes 4 hard-coded registries in parallel using a *canary package path*
//! (root-path HEAD is unreliable — tencent returns 404 on `/`).
//! Returns the candidates sorted by observed latency (failures pushed to the back).
//!
//! Matches frontend `RegistryProbe` / `RegistryPickResult` contract in
//! `src/lib/api/contracts.ts`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::{DateTime, SecondsFormat, Utc};
use futures::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::source_config::{
    registry_endpoint_chain, InstallerSourceConfig, MirrorEndpoint, CUSTOM_SOURCE_NAME,
};

/// Canary npm package used for the per-registry probe.
///
/// Picked because both `@anthropic-ai/claude-code` and `@openai/codex` exist on
/// all 4 mirrors; `@openai/codex` is shorter and slightly cheaper to fetch metadata for.
pub const CANARY_PKG: &str = "@openai/codex";

/// Probe network budget — both connect and total are short, per research.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(5);

/// Cache freshness window — within 24h the probe is short-circuited.
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// On-disk cache filename inside `<runtime_root>/`.
const CACHE_FILENAME: &str = "registry_probe_cache.json";

/// 4-registry whitelist. Order is irrelevant since we sort by latency at the end.
///
/// The built-ins remain compile-time hard-coded, but a persisted custom source
/// can be inserted at the front of the chain via `InstallerSourceConfig`.
pub const REGISTRY_DEFS: &[RegistryDef] = &[
    RegistryDef {
        name: "npmjs",
        url: "https://registry.npmjs.org",
    },
    RegistryDef {
        name: "npmmirror",
        url: "https://registry.npmmirror.com",
    },
    RegistryDef {
        name: "tencent",
        url: "https://mirrors.tencent.com/npm",
    },
    RegistryDef {
        name: "huawei",
        url: "https://mirrors.huaweicloud.com/repository/npm",
    },
];

#[derive(Debug, Clone, Copy)]
pub struct RegistryDef {
    pub name: &'static str,
    pub url: &'static str,
}

/// Mirrors frontend `RegistryProbe` (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryProbe {
    pub name: String,
    pub url: String,
    pub ok: bool,
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Mirrors frontend `RegistryPickResult` (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPickResult {
    pub candidates: Vec<RegistryProbe>,
    pub chosen: String,
    pub chosen_name: String,
    pub chosen_at: String,
    pub cached: bool,
}

#[derive(Debug, Error)]
pub enum RegistryProbeError {
    #[error("registry probe failed: all candidates unreachable")]
    AllUnreachable,
    #[error("internal: failed to build HTTP client: {0}")]
    ClientBuild(String),
}

pub struct RegistryProbeService;

impl RegistryProbeService {
    /// Probe all registries with the default canary package.
    ///
    /// Returns the candidates list (sorted by latency, failed last) plus the
    /// winning registry (lowest latency, ok==true). Errors when *all* 4 fail.
    pub async fn smart_pick(force_refresh: bool) -> Result<RegistryPickResult, RegistryProbeError> {
        Self::smart_pick_with_config(&InstallerSourceConfig::default(), force_refresh).await
    }

    /// Same as [`smart_pick`] but with a configurable canary package — used by tests.
    pub async fn smart_pick_with_canary(
        canary: &str,
        force_refresh: bool,
    ) -> Result<RegistryPickResult, RegistryProbeError> {
        let endpoints = registry_endpoint_chain(&InstallerSourceConfig::default(), REGISTRY_DEFS);
        Self::smart_pick_inner(canary, force_refresh, &endpoints).await
    }

    /// Probe registries with a persisted source configuration.
    pub async fn smart_pick_with_config(
        config: &InstallerSourceConfig,
        force_refresh: bool,
    ) -> Result<RegistryPickResult, RegistryProbeError> {
        let endpoints = registry_endpoint_chain(config, REGISTRY_DEFS);
        Self::smart_pick_inner(CANARY_PKG, force_refresh, &endpoints).await
    }

    /// Internal entry point: accepts an injectable mirror list for test isolation.
    ///
    /// Implements the cache + fallback flow:
    /// 1. Fresh cache hit (age < 24h, force_refresh=false) → return cached (no network).
    /// 2. Run probes.
    /// 3. Probe success → persist to cache atomically, return `cached: false`.
    /// 4. All probes fail BUT a (stale) cache exists → return cached (degraded mode).
    /// 5. All probes fail AND no cache → `AllUnreachable`.
    pub async fn smart_pick_inner(
        canary: &str,
        force_refresh: bool,
        endpoints: &[MirrorEndpoint],
    ) -> Result<RegistryPickResult, RegistryProbeError> {
        let cache_path = cache_file_path(endpoints);

        // Step 1: fresh cache short-circuit.
        if !force_refresh {
            if let Some(path) = cache_path.as_ref() {
                if let Some(cached) = load_fresh_cache(path) {
                    return Ok(cached);
                }
            }
        }

        // Step 2: probe network.
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            // Don't keep idle connections — prevents skew on next call.
            .pool_idle_timeout(Duration::from_millis(100))
            .user_agent("cc-switch-installer/1.0 (registry-probe)")
            .build()
            .map_err(|e| RegistryProbeError::ClientBuild(e.to_string()))?;

        let probes = endpoints.iter().map(|def| {
            let client = client.clone();
            let url = format!(
                "{}/{}",
                def.base.trim_end_matches('/'),
                // Encode `@scope/name` keeping the slash (npm spec uses literal /).
                canary
            );
            async move { probe_one(&client, def, &url).await }
        });

        let mut candidates: Vec<RegistryProbe> = join_all(probes).await;

        // Sort: ok=true ascending by latency first, then failed (any order).
        candidates.sort_by(|a, b| match (a.ok, b.ok) {
            (true, true) => a.latency_ms.cmp(&b.latency_ms),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => a.latency_ms.cmp(&b.latency_ms),
        });

        let winner = candidates
            .iter()
            .find(|c| c.ok && c.name == CUSTOM_SOURCE_NAME)
            .cloned()
            .or_else(|| candidates.iter().find(|c| c.ok).cloned());

        match winner {
            Some(winner) => {
                let result = RegistryPickResult {
                    candidates,
                    chosen: winner.url,
                    chosen_name: winner.name,
                    chosen_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                    cached: false,
                };
                // Best-effort cache write: log on error but never fail the probe.
                if let Some(path) = cache_path.as_ref() {
                    if let Err(e) = write_cache_atomic(path, &result) {
                        log::warn!("registry_probe: cache write failed: {}", e);
                    }
                }
                Ok(result)
            }
            None => {
                // Step 4: all probes failed — try stale cache (any age).
                if let Some(path) = cache_path.as_ref() {
                    if let Some(mut stale) = read_cache_raw(path) {
                        log::warn!(
                            "registry_probe: all candidates unreachable, returning stale cache from {}",
                            stale.chosen_at
                        );
                        stale.cached = true;
                        return Ok(stale);
                    }
                }
                Err(RegistryProbeError::AllUnreachable)
            }
        }
    }
}

/// Resolve the on-disk cache path. Mirrors `NodeRuntime::runtime_root()` resolution
/// (honors `CC_SWITCH_TEST_HOME`, falls back to `dirs::data_local_dir()`).
///
/// Returns `None` if no data dir is available (extremely rare — only on broken
/// platforms). Caller silently degrades to "no cache" rather than crash.
fn cache_file_path(endpoints: &[MirrorEndpoint]) -> Option<PathBuf> {
    let filename = cache_filename(endpoints);
    if let Ok(override_dir) = std::env::var("CC_SWITCH_TEST_HOME") {
        let trimmed = override_dir.trim();
        if !trimmed.is_empty() {
            return Some(
                PathBuf::from(trimmed)
                    .join("cc-switch")
                    .join("runtime")
                    .join(filename),
            );
        }
    }
    dirs::data_local_dir().map(|base| base.join("cc-switch").join("runtime").join(filename))
}

fn cache_filename(endpoints: &[MirrorEndpoint]) -> String {
    if !endpoints.iter().any(|e| e.name == CUSTOM_SOURCE_NAME) {
        return CACHE_FILENAME.to_string();
    }

    let mut hasher = DefaultHasher::new();
    for endpoint in endpoints {
        endpoint.name.hash(&mut hasher);
        endpoint.base.hash(&mut hasher);
    }
    format!("registry_probe_cache_{:016x}.json", hasher.finish())
}

/// Read the cache file, return parsed result if and only if it's fresh (< CACHE_TTL).
/// Malformed cache → silently delete + return None (fall through to probe).
fn load_fresh_cache(path: &PathBuf) -> Option<RegistryPickResult> {
    let result = read_cache_raw(path)?;
    let parsed_at = match DateTime::parse_from_rfc3339(&result.chosen_at) {
        Ok(t) => t.with_timezone(&Utc),
        Err(_) => return None,
    };
    let age = Utc::now().signed_duration_since(parsed_at);
    let age_std = age.to_std().ok()?;
    if age_std < CACHE_TTL {
        let mut hit = result;
        hit.cached = true;
        Some(hit)
    } else {
        None
    }
}

/// Read + parse the cache file with no freshness check. Returns None when:
/// - file doesn't exist
/// - file unreadable
/// - JSON malformed (also deletes the file so the next call probes cleanly)
fn read_cache_raw(path: &PathBuf) -> Option<RegistryPickResult> {
    let contents = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<RegistryPickResult>(&contents) {
        Ok(r) => Some(r),
        Err(e) => {
            log::warn!(
                "registry_probe: malformed cache at {}, deleting: {}",
                path.display(),
                e
            );
            let _ = std::fs::remove_file(path);
            None
        }
    }
}

/// Atomic cache write: write to `<path>.tmp`, then rename.
/// Creates parent directory if missing. Windows-safe via `std::fs::rename`.
fn write_cache_atomic(path: &PathBuf, result: &RegistryPickResult) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(result).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

async fn probe_one(client: &Client, def: &MirrorEndpoint, url: &str) -> RegistryProbe {
    let started = Instant::now();
    match client.get(url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let elapsed_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            let ok = status.is_success();
            RegistryProbe {
                name: def.name.clone(),
                url: def.base.clone(),
                ok,
                latency_ms: elapsed_ms,
                status_code: Some(status.as_u16()),
                error: if ok {
                    None
                } else {
                    Some(format!("HTTP {}", status.as_u16()))
                },
            }
        }
        Err(err) => {
            let elapsed_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            let cause = if err.is_timeout() {
                "timeout".to_string()
            } else if err.is_connect() {
                "connect failed".to_string()
            } else {
                err.to_string()
            };
            RegistryProbe {
                name: def.name.clone(),
                url: def.base.clone(),
                ok: false,
                latency_ms: elapsed_ms,
                status_code: None,
                error: Some(cause),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    /// All 4 registries respond OK → result has 4 ok entries, sorted by latency.
    #[tokio::test]
    async fn all_responding_sorts_by_latency() {
        // Two mock servers acting as fast/slow registries (we only need any 1 ok winner).
        let fast = MockServer::start_async().await;
        let slow = MockServer::start_async().await;

        let _ = fast
            .mock_async(|when, then| {
                when.method(GET).path("/@openai/codex");
                then.status(200).body("{}");
            })
            .await;
        let _ = slow
            .mock_async(|when, then| {
                when.method(GET).path("/@openai/codex");
                then.status(200)
                    .delay(std::time::Duration::from_millis(200))
                    .body("{}");
            })
            .await;

        // Build a one-off probe against ad-hoc URLs (bypassing the static list).
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .build()
            .unwrap();
        let defs = [
            MirrorEndpoint::new("fast", Box::leak(fast.url("").into_boxed_str())),
            MirrorEndpoint::new("slow", Box::leak(slow.url("").into_boxed_str())),
        ];
        let probes = defs.iter().map(|def| {
            let client = client.clone();
            let url = format!("{}/{}", def.base.trim_end_matches('/'), CANARY_PKG);
            async move { probe_one(&client, def, &url).await }
        });
        let mut results: Vec<RegistryProbe> = futures::future::join_all(probes).await;
        results.sort_by_key(|r| r.latency_ms);

        assert_eq!(results.len(), 2);
        assert!(results[0].ok);
        assert!(results[1].ok);
        // Fast should come first
        assert_eq!(results[0].name, "fast");
    }

    // ---------- cache + fallback tests (G2) ----------
    //
    // All cache tests share the process-wide CC_SWITCH_TEST_HOME env var,
    // so they MUST run serially. Each test sets a unique tempdir.

    use serial_test::serial;
    use std::time::SystemTime;

    /// Helper: build a RegistryPickResult with a chosenAt N hours in the past.
    fn make_cached(name: &str, url: &str, hours_ago: i64) -> RegistryPickResult {
        let ts = Utc::now() - chrono::Duration::hours(hours_ago);
        RegistryPickResult {
            candidates: vec![RegistryProbe {
                name: name.to_string(),
                url: url.to_string(),
                ok: true,
                latency_ms: 42,
                status_code: Some(200),
                error: None,
            }],
            chosen: url.to_string(),
            chosen_name: name.to_string(),
            chosen_at: ts.to_rfc3339_opts(SecondsFormat::Millis, true),
            cached: false,
        }
    }

    /// Helper: write cache file to `<temp>/cc-switch/runtime/registry_probe_cache.json`.
    fn write_cache_to(temp: &std::path::Path, value: &RegistryPickResult) {
        let dir = temp.join("cc-switch").join("runtime");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(CACHE_FILENAME);
        std::fs::write(&path, serde_json::to_string_pretty(value).unwrap()).unwrap();
    }

    /// Helper: build a "guaranteed-fail" endpoint pointing at refused port.
    fn refused_def(name: &'static str) -> MirrorEndpoint {
        MirrorEndpoint::new(
            name,
            // 127.0.0.1:1 is reliably refused on all platforms.
            "http://127.0.0.1:1",
        )
    }

    /// 1. Fresh cache (1h old) short-circuits the probe — no network call.
    #[tokio::test]
    #[serial]
    async fn cache_fresh_skips_probe() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());

        let cached = make_cached("npmmirror", "https://registry.npmmirror.com", 1);
        write_cache_to(temp.path(), &cached);

        // Use a refused-port def: if the cache short-circuit fails, we'd see
        // ok=false and the all-fail branch. The cache hit MUST short-circuit.
        let defs = [refused_def("doomed")];
        let result = RegistryProbeService::smart_pick_inner(CANARY_PKG, false, &defs)
            .await
            .unwrap();

        assert!(result.cached, "fresh cache must return cached: true");
        assert_eq!(result.chosen_name, "npmmirror");
        assert_eq!(result.chosen, "https://registry.npmmirror.com");

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// A configured custom npm source must not reuse the default registry cache.
    #[tokio::test]
    #[serial]
    async fn custom_source_uses_config_specific_cache() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());

        let cached = make_cached("npmmirror", "https://registry.npmmirror.com", 1);
        write_cache_to(temp.path(), &cached);

        let custom = MockServer::start_async().await;
        let mock = custom
            .mock_async(|when, then| {
                when.method(GET).path("/@openai/codex");
                then.status(200).body("{}");
            })
            .await;
        let defs = [MirrorEndpoint::from_custom(Box::leak(
            custom.url("").into_boxed_str(),
        ))];

        let result = RegistryProbeService::smart_pick_inner(CANARY_PKG, false, &defs)
            .await
            .unwrap();

        assert!(!result.cached, "custom source must run its own probe");
        assert_eq!(result.chosen_name, CUSTOM_SOURCE_NAME);
        assert_eq!(result.chosen, defs[0].base.as_str());
        mock.assert_async().await;

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// 2. Expired cache (25h old) does NOT short-circuit — probe runs.
    #[tokio::test]
    #[serial]
    async fn cache_expired_triggers_probe() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());

        let stale = make_cached("npmmirror", "https://registry.npmmirror.com", 25);
        write_cache_to(temp.path(), &stale);

        // Live mock server to confirm probe ran.
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/@openai/codex");
                then.status(200).body("{}");
            })
            .await;
        let defs = [MirrorEndpoint::new(
            "live",
            Box::leak(server.url("").into_boxed_str()),
        )];

        let result = RegistryProbeService::smart_pick_inner(CANARY_PKG, false, &defs)
            .await
            .unwrap();

        assert!(!result.cached, "expired cache must NOT return cached: true");
        assert_eq!(result.chosen_name, "live");
        mock.assert_async().await;

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// 3. force_refresh=true bypasses even a fresh cache.
    #[tokio::test]
    #[serial]
    async fn force_refresh_bypasses_cache() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());

        // Pre-write fresh (1h) cache.
        let cached = make_cached("npmmirror", "https://registry.npmmirror.com", 1);
        write_cache_to(temp.path(), &cached);

        // Confirm probe runs by requiring a live mock hit.
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/@openai/codex");
                then.status(200).body("{}");
            })
            .await;
        let defs = [MirrorEndpoint::new(
            "live",
            Box::leak(server.url("").into_boxed_str()),
        )];

        let result = RegistryProbeService::smart_pick_inner(CANARY_PKG, true, &defs)
            .await
            .unwrap();

        assert!(!result.cached);
        assert_eq!(result.chosen_name, "live");
        mock.assert_async().await;

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// 4. All probes fail + a stale (48h) cache exists → return stale, cached=true.
    #[tokio::test]
    #[serial]
    async fn all_unreachable_with_stale_cache_returns_cache() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());

        let stale = make_cached("npmmirror", "https://registry.npmmirror.com", 48);
        write_cache_to(temp.path(), &stale);

        let defs = [refused_def("doomed1"), refused_def("doomed2")];
        let result = RegistryProbeService::smart_pick_inner(CANARY_PKG, false, &defs)
            .await
            .unwrap();

        assert!(result.cached, "stale-cache fallback must set cached: true");
        assert_eq!(result.chosen_name, "npmmirror");

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// 5. All probes fail + no cache file → AllUnreachable error.
    #[tokio::test]
    #[serial]
    async fn all_unreachable_no_cache_returns_error() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());

        let defs = [refused_def("doomed1"), refused_def("doomed2")];
        let err = RegistryProbeService::smart_pick_inner(CANARY_PKG, false, &defs)
            .await
            .unwrap_err();

        assert!(matches!(err, RegistryProbeError::AllUnreachable));

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// 6. Malformed cache JSON is tolerated — deleted + fall through to probe.
    #[tokio::test]
    #[serial]
    async fn malformed_cache_falls_through_to_probe() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());

        // Write garbage to cache file.
        let dir = temp.path().join("cc-switch").join("runtime");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(CACHE_FILENAME);
        std::fs::write(&path, "this is not json {{{").unwrap();

        // Probe should succeed via mock — no crash.
        let server = MockServer::start_async().await;
        let _ = server
            .mock_async(|when, then| {
                when.method(GET).path("/@openai/codex");
                then.status(200).body("{}");
            })
            .await;
        let defs = [MirrorEndpoint::new(
            "live",
            Box::leak(server.url("").into_boxed_str()),
        )];

        let result = RegistryProbeService::smart_pick_inner(CANARY_PKG, false, &defs)
            .await
            .unwrap();
        assert!(!result.cached);
        assert_eq!(result.chosen_name, "live");

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// 7. Successful probe writes cache atomically:
    /// `registry_probe_cache.json` exists, `.tmp` does NOT remain.
    #[tokio::test]
    #[serial]
    async fn cache_atomic_write() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());

        let server = MockServer::start_async().await;
        let _ = server
            .mock_async(|when, then| {
                when.method(GET).path("/@openai/codex");
                then.status(200).body("{}");
            })
            .await;
        let defs = [MirrorEndpoint::new(
            "live",
            Box::leak(server.url("").into_boxed_str()),
        )];

        // Subtract 2s to absorb filesystem mtime resolution (NTFS ~100ns, FAT 2s,
        // and clock jitter on slow runners). Without this absorbtion the assertion
        // is racy on Windows where mtime can land just before `before`.
        let before = SystemTime::now() - Duration::from_secs(2);
        let _ = RegistryProbeService::smart_pick_inner(CANARY_PKG, false, &defs)
            .await
            .unwrap();

        let dir = temp.path().join("cc-switch").join("runtime");
        let final_path = dir.join(CACHE_FILENAME);
        let tmp_path = dir.join("registry_probe_cache.json.tmp");

        assert!(
            final_path.exists(),
            "{} must exist after a successful probe",
            final_path.display()
        );
        assert!(
            !tmp_path.exists(),
            "{} must NOT remain after atomic rename",
            tmp_path.display()
        );

        // Sanity: file was actually written (mtime ≥ test start - 2s buffer).
        let meta = std::fs::metadata(&final_path).unwrap();
        assert!(meta.modified().unwrap() >= before);

        // Sanity: round-trip parse succeeds and chosen matches.
        let contents = std::fs::read_to_string(&final_path).unwrap();
        let parsed: RegistryPickResult = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed.chosen_name, "live");
        assert!(!parsed.cached, "persisted result records cached=false");

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// 1 failing registry is marked ok=false with an error message.
    #[tokio::test]
    async fn failing_registry_is_marked_failed() {
        let good = MockServer::start_async().await;
        let _ = good
            .mock_async(|when, then| {
                when.method(GET).path("/@openai/codex");
                then.status(200).body("{}");
            })
            .await;

        let bad_url = "http://127.0.0.1:1"; // guaranteed-refused port
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(300))
            .timeout(Duration::from_millis(800))
            .build()
            .unwrap();
        let defs = [
            MirrorEndpoint::new("good", Box::leak(good.url("").into_boxed_str())),
            MirrorEndpoint::new("bad", bad_url),
        ];
        let probes = defs.iter().map(|def| {
            let client = client.clone();
            let url = format!("{}/{}", def.base.trim_end_matches('/'), CANARY_PKG);
            async move { probe_one(&client, def, &url).await }
        });
        let results: Vec<RegistryProbe> = futures::future::join_all(probes).await;
        let good_r = results.iter().find(|r| r.name == "good").unwrap();
        let bad_r = results.iter().find(|r| r.name == "bad").unwrap();
        assert!(good_r.ok);
        assert!(!bad_r.ok);
        assert!(bad_r.error.is_some());
    }
}
