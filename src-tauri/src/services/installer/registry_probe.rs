//! Smart npm registry source picker.
//!
//! Probes 4 hard-coded registries in parallel using a *canary package path*
//! (root-path HEAD is unreliable — tencent returns 404 on `/`).
//! Returns the candidates sorted by observed latency (failures pushed to the back).
//!
//! Matches frontend `RegistryProbe` / `RegistryPickResult` contract in
//! `src/lib/api/contracts.ts`.

use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use futures::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Canary npm package used for the per-registry probe.
///
/// Picked because both `@anthropic-ai/claude-code` and `@openai/codex` exist on
/// all 4 mirrors; `@openai/codex` is shorter and slightly cheaper to fetch metadata for.
pub const CANARY_PKG: &str = "@openai/codex";

/// Probe network budget — both connect and total are short, per research.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(5);

/// 4-registry whitelist. Order is irrelevant since we sort by latency at the end.
///
/// **Compile-time hard-coded** — no Tauri command / config file may add/remove entries.
/// Future expansion goes via D9 (custom registry in expert mode), not here.
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
        Self::smart_pick_with_canary(CANARY_PKG, force_refresh).await
    }

    /// Same as [`smart_pick`] but with a configurable canary package — used by tests.
    pub async fn smart_pick_with_canary(
        canary: &str,
        force_refresh: bool,
    ) -> Result<RegistryPickResult, RegistryProbeError> {
        // Cache support is intentionally NOT yet wired — the design lives in
        // research §"缓存策略" but isn't part of the Phase B B2 contract.
        // Reserved for a follow-up Phase C polish.
        let _ = force_refresh;

        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            // Don't keep idle connections — prevents skew on next call.
            .pool_idle_timeout(Duration::from_millis(100))
            .user_agent("cc-switch-installer/1.0 (registry-probe)")
            .build()
            .map_err(|e| RegistryProbeError::ClientBuild(e.to_string()))?;

        let probes = REGISTRY_DEFS.iter().map(|def| {
            let client = client.clone();
            let url = format!(
                "{}/{}",
                def.url.trim_end_matches('/'),
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
            .find(|c| c.ok)
            .cloned()
            .ok_or(RegistryProbeError::AllUnreachable)?;

        Ok(RegistryPickResult {
            candidates,
            chosen: winner.url,
            chosen_name: winner.name,
            chosen_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            cached: false,
        })
    }
}

async fn probe_one(client: &Client, def: &RegistryDef, url: &str) -> RegistryProbe {
    let started = Instant::now();
    match client.get(url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let elapsed_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            let ok = status.is_success();
            RegistryProbe {
                name: def.name.to_string(),
                url: def.url.to_string(),
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
                name: def.name.to_string(),
                url: def.url.to_string(),
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
            RegistryDef {
                name: "fast",
                url: Box::leak(fast.url("").into_boxed_str()),
            },
            RegistryDef {
                name: "slow",
                url: Box::leak(slow.url("").into_boxed_str()),
            },
        ];
        let probes = defs.iter().map(|def| {
            let client = client.clone();
            let url = format!("{}/{}", def.url.trim_end_matches('/'), CANARY_PKG);
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
            RegistryDef {
                name: "good",
                url: Box::leak(good.url("").into_boxed_str()),
            },
            RegistryDef {
                name: "bad",
                url: bad_url,
            },
        ];
        let probes = defs.iter().map(|def| {
            let client = client.clone();
            let url = format!("{}/{}", def.url.trim_end_matches('/'), CANARY_PKG);
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
