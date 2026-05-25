//! System probe (D-Probe / Task B4).
//!
//! Implements the 17-dimension, 5-group system probe described in
//! `.trellis/tasks/05-21-cc-launcher-mvp/research/system-probe.md`. The
//! exported types serialize 1:1 with the frontend contract at
//! `src/lib/api/contracts.ts` (`SystemProbeReport` / `ProbeItem` /
//! `FixAction` / `ProbeGroup` / `ProbeStatus`).
//!
//! The probe itself is **read-only** — no probe function mutates host state.
//! Only [`apply_fix`] writes anything, and each `FixAction` variant is
//! either reversible or clearly informational.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::services::probe::{env as env_probe, fix_actions, network, runtime, system, workdir};
use crate::types::{LocalizedString, TypedError};

/// Three-color + missing/unknown status used by every probe dimension.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProbeStatus {
    Green,
    Yellow,
    Red,
    Missing,
    Unknown,
}

/// Logical group a probe item belongs to. Mirrors `ProbeGroup` in the
/// frontend contract.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProbeGroup {
    System,
    Runtime,
    Env,
    Network,
    Workdir,
}

/// Single one-click fix action attached to a [`ProbeItem`].
///
/// Variants with side effects (`InstallNode` / `InstallGit` /
/// `CleanEnvVar` / `InjectPathEntries`) are considered auto-fixable.
/// `OpenHomeDir` is informational/educational but harmless so it is also
/// auto-fixable. `ExternalLink` is **never** auto-fixable — it always
/// represents one of the five "system-level permission and interception
/// factors" (Defender / Admin / PsPolicy / SystemProxy / Rosetta) where
/// MVP only points the user at documentation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum FixAction {
    InstallNode { target_lts_major: u8 },
    InstallGit,
    CleanEnvVar { var_name: String },
    OpenHomeDir,
    InjectPathEntries { entries: Vec<String> },
    ExternalLink { url: String, label_key: String },
}

/// A single probed dimension. Mirrors `ProbeItem` in the frontend contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeItem {
    pub id: String,
    pub name_key: String,
    pub status: ProbeStatus,
    pub value: serde_json::Value,
    pub message_key: String,
    pub fix_action: Option<FixAction>,
    pub elapsed_ms: u64,
    pub group: ProbeGroup,
}

/// Aggregate probe report. Mirrors `SystemProbeReport` in the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemProbeReport {
    pub overall_status: ProbeStatus,
    pub items: Vec<ProbeItem>,
    pub generated_at: DateTime<Utc>,
    pub probe_version: u32,
}

/// Progress event emitted by [`apply_fix`]. Mirrors `FixProgress` in the
/// frontend contract — `message` is a structured `LocalizedString` and
/// `error` is a structured `TypedError`. The backend constructs both from
/// existing i18n keys + error codes; the key itself doubles as the fallback
/// text for all three locales so callers can substitute translations
/// without losing information when a key is missing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixProgress {
    pub fix_id: String,
    pub phase: FixPhase,
    pub message: LocalizedString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TypedError>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FixPhase {
    Starting,
    Running,
    Validating,
    Completed,
    Failed,
}

/// Errors returned by the probe / fix layer.
///
/// `Execution` and `Fix` variants are part of the public API contract
/// (future probe phases / installer integration may surface them); we
/// allow dead-code so they remain reachable from re-exports without
/// triggering the lint.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ProbeError {
    #[error("probe execution failed: {0}")]
    Execution(String),
    #[error("fix action failed: {0}")]
    Fix(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Probe schema version. Bump whenever the dimension set or `value` shape
/// changes so the frontend can invalidate caches.
pub const PROBE_VERSION: u32 = 1;

/// Stable `fixId` used in `FixProgress` events. Mirrors the
/// `fixIdFor(action)` helper in `src/lib/api/mock/system-probe.ts`.
pub fn fix_id_for(action: &FixAction) -> String {
    match action {
        FixAction::InstallNode { target_lts_major } => {
            format!("fix-installNode-{target_lts_major}")
        }
        FixAction::InstallGit => "fix-installGit".to_string(),
        FixAction::CleanEnvVar { var_name } => format!("fix-cleanEnvVar-{var_name}"),
        FixAction::OpenHomeDir => "fix-openHomeDir".to_string(),
        FixAction::InjectPathEntries { .. } => "fix-injectPathEntries".to_string(),
        FixAction::ExternalLink { label_key, .. } => format!("fix-externalLink-{label_key}"),
    }
}

/// Returns `true` iff this probe item is eligible for "一键全修" (bulk
/// auto-fix). The five informational "driver" factors (Defender / Admin /
/// PsPolicy / SystemProxy / Rosetta) all use [`FixAction::ExternalLink`]
/// and are therefore excluded by construction. An item without any fix
/// action is also not auto-fixable.
pub fn is_auto_fixable(item: &ProbeItem) -> bool {
    match &item.fix_action {
        None => false,
        Some(FixAction::ExternalLink { .. }) => false,
        Some(_) => true,
    }
}

/// Aggregate the overall status from individual items.
fn aggregate(items: &[ProbeItem]) -> ProbeStatus {
    if items
        .iter()
        .any(|i| matches!(i.status, ProbeStatus::Red | ProbeStatus::Missing))
    {
        ProbeStatus::Red
    } else if items.iter().any(|i| i.status == ProbeStatus::Yellow) {
        ProbeStatus::Yellow
    } else {
        ProbeStatus::Green
    }
}

/// Runs all 17 probe dimensions across the 5 groups and returns an
/// aggregated [`SystemProbeReport`].
///
/// The probe is fully read-only.
pub async fn run_probe() -> Result<SystemProbeReport, ProbeError> {
    // Synchronous probes — these are cheap and self-contained.
    let mut items: Vec<ProbeItem> = Vec::with_capacity(17);
    items.extend(system::run_all());
    items.extend(runtime::run_all());
    items.extend(env_probe::run_all());
    items.extend(workdir::run_all());

    // Network reachability is async (HTTP HEAD against 4 registries).
    items.extend(network::run_all().await);

    debug_assert_eq!(items.len(), 17, "probe must return exactly 17 dimensions");
    debug_assert!(
        [
            ProbeGroup::System,
            ProbeGroup::Runtime,
            ProbeGroup::Env,
            ProbeGroup::Network,
            ProbeGroup::Workdir,
        ]
        .iter()
        .all(|g| items.iter().any(|it| it.group == *g)),
        "probe must cover all 5 groups"
    );

    Ok(SystemProbeReport {
        overall_status: aggregate(&items),
        items,
        generated_at: Utc::now(),
        probe_version: PROBE_VERSION,
    })
}

/// Build a `LocalizedString` from an i18n key. The key doubles as the
/// fallback for every locale so the frontend can substitute the translation
/// without losing information when a key is missing.
///
/// NOTE: The returned `LocalizedString` is a *key-envelope*, not a localized
/// payload. The three locale fields all carry the raw i18n key (e.g.
/// `"fix.starting"`). The frontend MUST call `i18n.t(localized.zh)` (or
/// equivalent) before rendering — displaying `.zh`/`.en`/`.ja` directly will
/// leak the key into the UI. See `apply_fix` / `FixProgress.message`.
fn localize_key(key: &str) -> LocalizedString {
    LocalizedString {
        zh: key.to_string(),
        en: key.to_string(),
        ja: key.to_string(),
    }
}

/// Apply a [`FixAction`] and stream [`FixProgress`] events on the returned
/// channel. The receiver is closed when the action terminates (either
/// `Completed` or `Failed`).
pub fn apply_fix(action: FixAction) -> mpsc::UnboundedReceiver<FixProgress> {
    let (tx, rx) = mpsc::unbounded_channel();
    let fix_id = fix_id_for(&action);

    tokio::spawn(async move {
        let send = |phase: FixPhase,
                    message_key: &str,
                    percent: Option<u8>,
                    error: Option<TypedError>| {
            let _ = tx.send(FixProgress {
                fix_id: fix_id.clone(),
                phase,
                message: localize_key(message_key),
                percent,
                error,
            });
        };

        send(FixPhase::Starting, "fix.starting", Some(5), None);
        send(FixPhase::Running, "fix.running", Some(50), None);

        let outcome = fix_actions::apply(&action).await;

        match outcome {
            Ok(()) => {
                send(FixPhase::Validating, "fix.validating", Some(90), None);
                send(FixPhase::Completed, "fix.completed", Some(100), None);
            }
            Err(e) => {
                let code = e.code().to_string();
                let typed = TypedError {
                    code: code.clone(),
                    message: localize_key(&code),
                    cause: Some(e.to_string()),
                    retryable: false,
                };
                send(FixPhase::Failed, "fix.failed", None, Some(typed));
                log::warn!("apply_fix failed: action={action:?}, error={e}");
            }
        }
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(group: ProbeGroup, status: ProbeStatus, fix: Option<FixAction>) -> ProbeItem {
        ProbeItem {
            id: "x".into(),
            name_key: "probe.x.name".into(),
            status,
            value: serde_json::Value::Null,
            message_key: "probe.x.msg".into(),
            fix_action: fix,
            elapsed_ms: 0,
            group,
        }
    }

    #[test]
    fn aggregate_red_when_any_red() {
        let items = vec![
            item(ProbeGroup::System, ProbeStatus::Green, None),
            item(ProbeGroup::Runtime, ProbeStatus::Red, None),
            item(ProbeGroup::Env, ProbeStatus::Yellow, None),
        ];
        assert_eq!(aggregate(&items), ProbeStatus::Red);
    }

    #[test]
    fn aggregate_red_when_any_missing() {
        let items = vec![
            item(ProbeGroup::System, ProbeStatus::Green, None),
            item(ProbeGroup::Runtime, ProbeStatus::Missing, None),
        ];
        assert_eq!(aggregate(&items), ProbeStatus::Red);
    }

    #[test]
    fn aggregate_yellow_when_any_yellow_no_red() {
        let items = vec![
            item(ProbeGroup::System, ProbeStatus::Green, None),
            item(ProbeGroup::Env, ProbeStatus::Yellow, None),
        ];
        assert_eq!(aggregate(&items), ProbeStatus::Yellow);
    }

    #[test]
    fn aggregate_green_when_all_green() {
        let items = vec![
            item(ProbeGroup::System, ProbeStatus::Green, None),
            item(ProbeGroup::Network, ProbeStatus::Green, None),
        ];
        assert_eq!(aggregate(&items), ProbeStatus::Green);
    }

    #[test]
    fn fix_id_matches_mock_contract() {
        assert_eq!(
            fix_id_for(&FixAction::InstallNode {
                target_lts_major: 20
            }),
            "fix-installNode-20"
        );
        assert_eq!(fix_id_for(&FixAction::InstallGit), "fix-installGit");
        assert_eq!(
            fix_id_for(&FixAction::CleanEnvVar {
                var_name: "ANTHROPIC_API_KEY".into()
            }),
            "fix-cleanEnvVar-ANTHROPIC_API_KEY"
        );
        assert_eq!(fix_id_for(&FixAction::OpenHomeDir), "fix-openHomeDir");
        assert_eq!(
            fix_id_for(&FixAction::InjectPathEntries { entries: vec![] }),
            "fix-injectPathEntries"
        );
        assert_eq!(
            fix_id_for(&FixAction::ExternalLink {
                url: "https://example.com".into(),
                label_key: "probe.defender.docs".into()
            }),
            "fix-externalLink-probe.defender.docs"
        );
    }

    #[test]
    fn is_auto_fixable_excludes_external_link() {
        // The 5 informational driver factors all use ExternalLink.
        let driver_factors = [
            ("defender", "https://learn.microsoft.com/defender"),
            ("admin", "https://example.com/admin"),
            ("psPolicy", "https://learn.microsoft.com/powershell"),
            ("systemProxy", "https://example.com/proxy"),
            ("rosetta", "https://support.apple.com/HT211861"),
        ];
        for (id, url) in driver_factors {
            let it = ProbeItem {
                id: id.into(),
                name_key: "probe.x.name".into(),
                status: ProbeStatus::Unknown,
                value: serde_json::Value::Null,
                message_key: "probe.x.msg".into(),
                fix_action: Some(FixAction::ExternalLink {
                    url: url.into(),
                    label_key: format!("probe.{id}.docs"),
                }),
                elapsed_ms: 0,
                group: ProbeGroup::Env,
            };
            assert!(
                !is_auto_fixable(&it),
                "driver factor {id} must NOT be auto-fixable"
            );
        }
    }

    #[test]
    fn is_auto_fixable_includes_install_actions() {
        let it = ProbeItem {
            id: "node".into(),
            name_key: "probe.node.name".into(),
            status: ProbeStatus::Missing,
            value: serde_json::Value::Null,
            message_key: "probe.node.missing".into(),
            fix_action: Some(FixAction::InstallNode {
                target_lts_major: 20,
            }),
            elapsed_ms: 0,
            group: ProbeGroup::Runtime,
        };
        assert!(is_auto_fixable(&it));
    }

    #[test]
    fn is_auto_fixable_handles_none() {
        let it = ProbeItem {
            id: "cpu".into(),
            name_key: "probe.cpu.name".into(),
            status: ProbeStatus::Green,
            value: serde_json::Value::Null,
            message_key: "probe.cpu.green".into(),
            fix_action: None,
            elapsed_ms: 0,
            group: ProbeGroup::System,
        };
        assert!(!is_auto_fixable(&it));
    }

    #[tokio::test]
    async fn run_probe_returns_17_items_across_5_groups() {
        let report = run_probe().await.expect("probe should not fail");
        assert_eq!(
            report.items.len(),
            17,
            "expected exactly 17 dimensions, got {}",
            report.items.len()
        );

        let groups: std::collections::HashSet<ProbeGroup> =
            report.items.iter().map(|i| i.group).collect();
        assert_eq!(groups.len(), 5, "expected exactly 5 groups, got {groups:?}");

        // Verify the counts per group: 4/4/5/2/2 = 17.
        let count = |g: ProbeGroup| report.items.iter().filter(|i| i.group == g).count();
        assert_eq!(count(ProbeGroup::System), 4, "system group should have 4");
        assert_eq!(count(ProbeGroup::Runtime), 4, "runtime group should have 4");
        assert_eq!(count(ProbeGroup::Env), 5, "env group should have 5");
        assert_eq!(count(ProbeGroup::Network), 2, "network group should have 2");
        assert_eq!(count(ProbeGroup::Workdir), 2, "workdir group should have 2");

        // The 5 informational driver factors must be present and unfixable.
        let driver_ids = ["defender", "admin", "psPolicy", "systemProxy", "rosetta"];
        for id in driver_ids {
            let it = report
                .items
                .iter()
                .find(|i| i.id == id)
                .unwrap_or_else(|| panic!("driver factor `{id}` missing"));
            assert!(
                !is_auto_fixable(it),
                "driver factor `{id}` MUST NOT be auto-fixable"
            );
        }
    }

    #[test]
    fn probe_version_is_pinned() {
        assert_eq!(PROBE_VERSION, 1);
    }
}
