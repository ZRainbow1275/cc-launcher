//! Tauri commands for the system probe (Task B4).
//!
//! Two commands:
//! - `probe_system` — runs all 17 dimensions and returns a [`SystemProbeReport`].
//! - `apply_probe_fix` — applies a [`FixAction`] and emits a stream of
//!   [`FixProgress`] events on the `probe:fix-progress` channel.
//!
//! The streaming channel uses Tauri's [`Channel`] which is the supported
//! mechanism for async streaming results (alternative to events). The
//! mock contract at `src/lib/api/mock/system-probe.ts` returns an
//! `AsyncIterable<FixProgress>`; the frontend adapter wraps this Channel
//! into an async generator.

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

use crate::services::system_probe::{self, FixAction, FixProgress, ProbeItem, SystemProbeReport};
use crate::store::AppState;

/// Run the full 17-dimension probe. Returns a fully populated report.
#[tauri::command]
pub async fn probe_system() -> Result<SystemProbeReport, String> {
    system_probe::run_probe()
        .await
        .map_err(|e| format!("probe_system failed: {e}"))
}

/// Filter helper exposed to the frontend. Useful for the "一键全修"
/// button — the UI doesn't need to mirror our `is_auto_fixable` logic in
/// TS; it asks Rust which items qualify.
#[derive(Debug, Serialize, Deserialize)]
pub struct AutoFixCandidate {
    pub item_id: String,
    pub fix_action: FixAction,
}

/// Filter a list of probe items down to the auto-fixable subset.
/// The frontend "一键全修" loops over the returned candidates and calls
/// [`apply_probe_fix`] for each one.
#[tauri::command]
pub fn probe_auto_fix_candidates(items: Vec<ProbeItem>) -> Vec<AutoFixCandidate> {
    items
        .into_iter()
        .filter(system_probe::is_auto_fixable)
        .filter_map(|i| {
            i.fix_action.clone().map(|fa| AutoFixCandidate {
                item_id: i.id,
                fix_action: fa,
            })
        })
        .collect()
}

/// Apply a single [`FixAction`] and stream progress on `channel`. The
/// channel is closed once the action terminates.
#[tauri::command]
pub async fn apply_probe_fix(
    state: tauri::State<'_, AppState>,
    action: FixAction,
    channel: Channel<FixProgress>,
) -> Result<(), String> {
    let source_config =
        super::onboarding_settings::settings_get_installer_source_config_internal(&state)
            .map_err(|e| e.to_string())?;
    let mut rx = system_probe::apply_fix(action, source_config);
    while let Some(progress) = rx.recv().await {
        if let Err(e) = channel.send(progress) {
            log::warn!("apply_probe_fix: channel send failed: {e}");
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::system_probe::{FixAction, ProbeGroup, ProbeItem, ProbeStatus};

    fn green_item(id: &str, group: ProbeGroup, fix: Option<FixAction>) -> ProbeItem {
        ProbeItem {
            id: id.into(),
            name_key: format!("probe.{id}.name"),
            status: ProbeStatus::Green,
            value: serde_json::Value::Null,
            message_key: format!("probe.{id}.green"),
            fix_action: fix,
            elapsed_ms: 0,
            group,
        }
    }

    #[test]
    fn auto_fix_candidates_filters_out_external_link() {
        let items = vec![
            green_item(
                "node",
                ProbeGroup::Runtime,
                Some(FixAction::InstallNode {
                    target_lts_major: 20,
                }),
            ),
            green_item(
                "defender",
                ProbeGroup::Env,
                Some(FixAction::ExternalLink {
                    url: "https://example.com".into(),
                    label_key: "probe.defender.docs".into(),
                }),
            ),
            green_item(
                "workdirExists",
                ProbeGroup::Workdir,
                Some(FixAction::CreateWorkdir {
                    path: "/tmp/cc-launcher-projects".into(),
                }),
            ),
            green_item("cpu", ProbeGroup::System, None),
        ];
        let cands = probe_auto_fix_candidates(items);
        assert_eq!(cands.len(), 2);
        assert_eq!(cands[0].item_id, "node");
        assert_eq!(cands[1].item_id, "workdirExists");
    }

    #[test]
    fn auto_fix_candidates_handles_empty() {
        assert!(probe_auto_fix_candidates(vec![]).is_empty());
    }
}
