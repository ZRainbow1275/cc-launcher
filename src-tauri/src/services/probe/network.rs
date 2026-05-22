//! Network group — 2 dimensions.
//!
//! Dimensions (all platforms):
//! 1. `systemProxy` — HTTP_PROXY / HTTPS_PROXY env vars (informational; NOT auto-fixable)
//! 2. `network`     — reachability of the 4 npm registries (latency, status)
//!
//! `systemProxy` is the 5th of the 5 informational "driver factors". It
//! uses [`FixAction::ExternalLink`] and is excluded from auto-fix.
//!
//! `network` is informational (the user fixes connectivity, not us).
//! Probes are read-only — we issue HEAD requests with a 3s timeout.

use std::time::{Duration, Instant};

use serde_json::json;

use crate::services::system_probe::{FixAction, ProbeGroup, ProbeItem, ProbeStatus};

const REGISTRIES: &[(&str, &str)] = &[
    ("npmjs", "https://registry.npmjs.org/"),
    ("npmmirror", "https://registry.npmmirror.com/"),
    ("tencent", "https://mirrors.tencent.com/npm/"),
    ("huawei", "https://mirrors.huaweicloud.com/repository/npm/"),
];

/// Run all probes in the network group.
pub async fn run_all() -> Vec<ProbeItem> {
    vec![probe_system_proxy(), probe_network_reachability().await]
}

fn probe_system_proxy() -> ProbeItem {
    let t0 = Instant::now();
    let http = std::env::var("HTTP_PROXY")
        .or_else(|_| std::env::var("http_proxy"))
        .ok();
    let https = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .ok();

    let has_any = http.is_some() || https.is_some();
    // Informational only — we surface a docs link explaining what a proxy
    // is and how it interacts with npm.
    let (status, message_key) = if has_any {
        (ProbeStatus::Yellow, "probe.systemProxy.yellow")
    } else {
        (ProbeStatus::Green, "probe.systemProxy.green")
    };

    let mut value = serde_json::Map::new();
    if let Some(h) = http {
        value.insert("http".into(), json!(h));
    }
    if let Some(h) = https {
        value.insert("https".into(), json!(h));
    }

    ProbeItem {
        id: "systemProxy".into(),
        name_key: "probe.systemProxy.name".into(),
        status,
        value: serde_json::Value::Object(value),
        message_key: message_key.into(),
        fix_action: Some(FixAction::ExternalLink {
            url: "https://docs.npmjs.com/cli/v10/using-npm/config#proxy".into(),
            label_key: "probe.systemProxy.docs".into(),
        }),
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Network,
    }
}

async fn probe_network_reachability() -> ProbeItem {
    let t0 = Instant::now();
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("network probe: client build failed: {e}");
            return ProbeItem {
                id: "network".into(),
                name_key: "probe.network.name".into(),
                status: ProbeStatus::Unknown,
                value: json!([]),
                message_key: "probe.network.unknown".into(),
                fix_action: None,
                elapsed_ms: t0.elapsed().as_millis() as u64,
                group: ProbeGroup::Network,
            };
        }
    };

    let futs = REGISTRIES.iter().map(|(name, url)| {
        let c = client.clone();
        let name = *name;
        let url = *url;
        async move {
            let start = Instant::now();
            // Per research §6 footnote: bare-registry HEAD against tencent
            // mirror root returns 404. We use HEAD on the well-known root —
            // the latency signal is what we care about, not exact status.
            let resp = c.head(url).send().await;
            let elapsed = start.elapsed().as_millis() as u64;
            match resp {
                Ok(r) => {
                    let code = r.status().as_u16();
                    // Accept any non-server-error response as "ok"; even 404
                    // proves the host is alive (e.g. tencent mirror).
                    let ok = code < 500;
                    (name.to_string(), ok, elapsed, Some(code))
                }
                Err(_) => (name.to_string(), false, elapsed, None),
            }
        }
    });

    let results = futures::future::join_all(futs).await;

    let reachable = results.iter().filter(|(_, ok, _, _)| *ok).count();
    let fast = results.iter().any(|(_, ok, ms, _)| *ok && *ms < 1000);
    let status = if reachable == 0 {
        ProbeStatus::Red
    } else if fast {
        ProbeStatus::Green
    } else {
        ProbeStatus::Yellow
    };

    let message_key = match status {
        ProbeStatus::Green => "probe.network.green",
        ProbeStatus::Yellow => "probe.network.yellow",
        _ => "probe.network.red",
    };

    let value = json!(results
        .into_iter()
        .map(|(name, ok, ms, code)| {
            let mut m = serde_json::Map::new();
            m.insert("name".into(), json!(name));
            m.insert("ok".into(), json!(ok));
            m.insert("latencyMs".into(), json!(ms));
            if let Some(c) = code {
                m.insert("statusCode".into(), json!(c));
            }
            serde_json::Value::Object(m)
        })
        .collect::<Vec<_>>());

    ProbeItem {
        id: "network".into(),
        name_key: "probe.network.name".into(),
        status,
        value,
        message_key: message_key.into(),
        fix_action: None,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Network,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_all_returns_2_network_items() {
        let items = run_all().await;
        assert_eq!(items.len(), 2);
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"systemProxy"));
        assert!(ids.contains(&"network"));
        assert!(items.iter().all(|i| i.group == ProbeGroup::Network));
    }

    #[test]
    fn system_proxy_always_uses_external_link() {
        let it = probe_system_proxy();
        assert_eq!(it.id, "systemProxy");
        assert!(matches!(
            it.fix_action,
            Some(FixAction::ExternalLink { .. })
        ));
        // Must be auto-fix-excluded (it's the 5th driver factor).
        assert!(!crate::services::system_probe::is_auto_fixable(&it));
    }

    #[tokio::test]
    async fn network_probe_returns_4_registry_results() {
        let it = probe_network_reachability().await;
        assert_eq!(it.id, "network");
        let arr = it.value.as_array().expect("value should be array");
        assert_eq!(arr.len(), 4);
        for entry in arr {
            let obj = entry.as_object().unwrap();
            assert!(obj.contains_key("name"));
            assert!(obj.contains_key("ok"));
            assert!(obj.contains_key("latencyMs"));
        }
    }
}
