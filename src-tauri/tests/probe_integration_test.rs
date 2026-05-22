//! Integration test for the system probe (Task B4).
//!
//! This is a black-box integration test: it runs `run_probe()` on the
//! actual host and asserts only the **shape** of the report (17 items,
//! 5 groups, all 5 driver factors are NOT auto-fixable). Specific values
//! depend on the host (Node version, network connectivity, etc.) and are
//! NOT asserted here.

use cc_switch_lib::services::system_probe::{
    is_auto_fixable, run_probe, FixAction, ProbeGroup, ProbeStatus, PROBE_VERSION,
};

#[tokio::test]
async fn integration_probe_returns_17_items_5_groups() {
    let report = run_probe()
        .await
        .expect("probe must succeed on integration host");

    assert_eq!(
        report.items.len(),
        17,
        "expected 17 dimensions, got {}",
        report.items.len()
    );
    assert_eq!(report.probe_version, PROBE_VERSION);

    let groups: std::collections::HashSet<ProbeGroup> =
        report.items.iter().map(|i| i.group).collect();
    assert_eq!(
        groups.len(),
        5,
        "groups should be exactly 5, got {groups:?}"
    );

    let count = |g: ProbeGroup| report.items.iter().filter(|i| i.group == g).count();
    assert_eq!(count(ProbeGroup::System), 4);
    assert_eq!(count(ProbeGroup::Runtime), 4);
    assert_eq!(count(ProbeGroup::Env), 5);
    assert_eq!(count(ProbeGroup::Network), 2);
    assert_eq!(count(ProbeGroup::Workdir), 2);
}

#[tokio::test]
async fn integration_five_driver_factors_are_not_auto_fixable() {
    let report = run_probe()
        .await
        .expect("probe must succeed on integration host");

    // The 5 informational "driver factors" per task spec.
    let driver_ids = ["defender", "admin", "psPolicy", "systemProxy", "rosetta"];
    for id in driver_ids {
        let it = report
            .items
            .iter()
            .find(|i| i.id == id)
            .unwrap_or_else(|| panic!("missing driver factor: {id}"));
        assert!(
            matches!(it.fix_action, Some(FixAction::ExternalLink { .. })),
            "driver factor {id} must use ExternalLink, got {:?}",
            it.fix_action
        );
        assert!(
            !is_auto_fixable(it),
            "driver factor {id} must NOT be auto-fixable"
        );
    }
}

#[tokio::test]
async fn integration_aggregate_status_is_well_formed() {
    let report = run_probe()
        .await
        .expect("probe must succeed on integration host");

    // The overall status must be one of the 5 known states.
    assert!(
        matches!(
            report.overall_status,
            ProbeStatus::Green
                | ProbeStatus::Yellow
                | ProbeStatus::Red
                | ProbeStatus::Missing
                | ProbeStatus::Unknown
        ),
        "overall_status must be a valid ProbeStatus"
    );

    // generated_at is recent.
    let now = chrono::Utc::now();
    let delta = now.signed_duration_since(report.generated_at).num_seconds().abs();
    assert!(delta < 60, "generated_at should be within last minute");

    // Every item has non-empty stable identifiers.
    for it in &report.items {
        assert!(!it.id.is_empty(), "item id must not be empty");
        assert!(!it.name_key.is_empty(), "item name_key must not be empty");
        assert!(
            !it.message_key.is_empty(),
            "item message_key must not be empty"
        );
    }
}

#[tokio::test]
async fn integration_probe_is_idempotent_shape() {
    // Running the probe twice must yield the same shape (counts/IDs),
    // even though specific status values may differ (e.g. network blips).
    let r1 = run_probe().await.expect("probe 1");
    let r2 = run_probe().await.expect("probe 2");
    assert_eq!(r1.items.len(), r2.items.len());

    let mut ids1: Vec<_> = r1.items.iter().map(|i| i.id.clone()).collect();
    let mut ids2: Vec<_> = r2.items.iter().map(|i| i.id.clone()).collect();
    ids1.sort();
    ids2.sort();
    assert_eq!(ids1, ids2, "probe ID set must be stable across runs");
}
