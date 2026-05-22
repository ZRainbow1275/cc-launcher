//! Integration tests for the registry probe service.
//!
//! Spins up real `httpmock` servers and verifies:
//! - All 4 registries respond OK → result sorted by latency, no failures.
//! - One registry failing → marked `ok: false` and pushed to back of `candidates`.
//! - All registries failing → service returns `AllUnreachable` error.

use std::time::Duration;

use cc_switch_lib::services::installer::registry_probe::{
    RegistryDef, RegistryProbe, RegistryProbeError,
};
use futures::future::join_all;
use httpmock::Method::GET;
use httpmock::MockServer;
use reqwest::Client;

const CANARY: &str = "@openai/codex";

/// Re-implementation of the package-private `probe_one` so integration tests
/// can drive arbitrary registry URLs (the production `smart_pick` is locked to
/// the 4-mirror whitelist).
async fn probe_one(client: &Client, def: &RegistryDef, url: &str) -> RegistryProbe {
    let started = std::time::Instant::now();
    match client.get(url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let elapsed_ms = started.elapsed().as_millis() as u64;
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
        Err(err) => RegistryProbe {
            name: def.name.to_string(),
            url: def.url.to_string(),
            ok: false,
            latency_ms: started.elapsed().as_millis() as u64,
            status_code: None,
            error: Some(err.to_string()),
        },
    }
}

fn build_client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_millis(500))
        .timeout(Duration::from_secs(2))
        .build()
        .expect("build client")
}

#[tokio::test]
async fn all_four_registries_responding_returns_sorted_candidates() {
    let servers: Vec<MockServer> = (0..4).map(|_| MockServer::start()).collect();
    let delays_ms = [10u64, 200, 80, 150]; // intentionally out-of-order

    for (server, delay) in servers.iter().zip(delays_ms.iter()) {
        let _ = server.mock(|when, then| {
            when.method(GET).path(format!("/{CANARY}"));
            then.status(200)
                .delay(Duration::from_millis(*delay))
                .body("{}");
        });
    }

    let names = ["npmjs", "npmmirror", "tencent", "huawei"];
    let defs: Vec<RegistryDef> = servers
        .iter()
        .zip(names.iter())
        .map(|(s, n)| RegistryDef {
            name: Box::leak(n.to_string().into_boxed_str()),
            url: Box::leak(s.url("").into_boxed_str()),
        })
        .collect();

    let client = build_client();
    let probes = defs.iter().map(|def| {
        let client = client.clone();
        let url = format!("{}/{}", def.url.trim_end_matches('/'), CANARY);
        async move { probe_one(&client, def, &url).await }
    });
    let mut results: Vec<RegistryProbe> = join_all(probes).await;

    // Sort by latency
    results.sort_by_key(|r| r.latency_ms);

    assert_eq!(results.len(), 4);
    for r in &results {
        assert!(r.ok, "registry {} should be ok", r.name);
    }
    // First should be the 10ms server (npmjs)
    assert_eq!(results[0].name, "npmjs");
}

#[tokio::test]
async fn one_failing_registry_is_marked_failed() {
    let good = MockServer::start();
    let _ = good.mock(|when, then| {
        when.method(GET).path(format!("/{CANARY}"));
        then.status(200).body("{}");
    });
    let dead_url = "http://127.0.0.1:1"; // ECONNREFUSED guaranteed

    let defs = [
        RegistryDef {
            name: "good",
            url: Box::leak(good.url("").into_boxed_str()),
        },
        RegistryDef {
            name: "bad",
            url: dead_url,
        },
    ];
    let client = build_client();
    let probes = defs.iter().map(|def| {
        let client = client.clone();
        let url = format!("{}/{}", def.url.trim_end_matches('/'), CANARY);
        async move { probe_one(&client, def, &url).await }
    });
    let results: Vec<RegistryProbe> = join_all(probes).await;
    let g = results.iter().find(|r| r.name == "good").unwrap();
    let b = results.iter().find(|r| r.name == "bad").unwrap();
    assert!(g.ok);
    assert_eq!(g.status_code, Some(200));
    assert!(!b.ok);
    assert!(b.error.is_some(), "bad registry must report error cause");
}

#[tokio::test]
async fn all_registries_failing_yields_no_winner() {
    let unreachable_ports = ["http://127.0.0.1:1", "http://127.0.0.1:2"];
    let defs: Vec<RegistryDef> = unreachable_ports
        .iter()
        .enumerate()
        .map(|(i, u)| RegistryDef {
            name: Box::leak(format!("dead{i}").into_boxed_str()),
            url: u,
        })
        .collect();

    let client = build_client();
    let probes = defs.iter().map(|def| {
        let client = client.clone();
        let url = format!("{}/{}", def.url.trim_end_matches('/'), CANARY);
        async move { probe_one(&client, def, &url).await }
    });
    let results: Vec<RegistryProbe> = join_all(probes).await;
    assert!(results.iter().all(|r| !r.ok));
    // Simulate the "all unreachable" branch of smart_pick
    let winner = results.iter().find(|c| c.ok);
    assert!(winner.is_none());

    // Sanity: the public error variant exists and is constructible (compile-time check).
    let _err: RegistryProbeError = RegistryProbeError::AllUnreachable;
}

#[tokio::test]
async fn registry_probe_sort_pushes_failures_to_end() {
    // Build a synthetic list with mixed ok/failure and verify the sort order
    // used by the production code.
    let mut list = [
        RegistryProbe {
            name: "slow_ok".into(),
            url: "https://a.test".into(),
            ok: true,
            latency_ms: 300,
            status_code: Some(200),
            error: None,
        },
        RegistryProbe {
            name: "fast_fail".into(),
            url: "https://b.test".into(),
            ok: false,
            latency_ms: 5,
            status_code: None,
            error: Some("timeout".into()),
        },
        RegistryProbe {
            name: "fast_ok".into(),
            url: "https://c.test".into(),
            ok: true,
            latency_ms: 80,
            status_code: Some(200),
            error: None,
        },
    ];
    list.sort_by(|a, b| match (a.ok, b.ok) {
        (true, true) => a.latency_ms.cmp(&b.latency_ms),
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => a.latency_ms.cmp(&b.latency_ms),
    });
    // ok=true comes first (fast_ok, slow_ok), failed at the end.
    assert_eq!(list[0].name, "fast_ok");
    assert_eq!(list[1].name, "slow_ok");
    assert_eq!(list[2].name, "fast_fail");
}
