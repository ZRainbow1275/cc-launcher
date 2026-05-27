//! Tauri commands for the launcher service (Task B5 / D5).
//!
//! Public contract surface matches `src/lib/api/contracts.ts` exactly. Service
//! types stay internal — we project from them into wire DTOs at this boundary.
//! See `commands::launcher_wire` for the DTO module.

use tauri::State;

use crate::sandbox;
use crate::services::installer::cli_install::TargetCli;
use crate::services::launcher_service::{LauncherService, StartCliOpts};
use crate::store::AppState;
use crate::types::OperationResult;

use super::launcher_wire::{
    parse_terminal_wire_id, typed_error_from, WireLaunchResult, WireSafetySummary, WireTargetCli,
    WireTerminalInfo,
};

/// Wire form of `StartCliOpts`. Matches the frontend mock signature shape
/// `{ profileId, targetCli, terminalId?, cwd? }`. Tauri auto-camelCases the
/// outer object name; this struct is the canonical Rust deserialization
/// target.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireStartCliOpts {
    pub profile_id: String,
    pub target_cli: WireTargetCli,
    #[serde(default)]
    pub terminal_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

impl WireStartCliOpts {
    fn target_cli_internal(&self) -> TargetCli {
        match self.target_cli {
            WireTargetCli::Claude => TargetCli::Claude,
            WireTargetCli::Codex => TargetCli::Codex,
        }
    }
}

/// Probe the host for installed terminal emulators.
///
/// Returns the contract `TerminalCandidate[]` shape (via `WireTerminalInfo`).
#[tauri::command]
pub async fn detect_terminals() -> Result<Vec<WireTerminalInfo>, String> {
    let candidates = LauncherService::detect_terminals().await;
    Ok(candidates.into_iter().map(WireTerminalInfo::from).collect())
}

/// Spawn the configured CLI inside the user's preferred terminal.
///
/// Returns the contract `LaunchResult` envelope. Failures are encoded inline
/// as `success: false` with a populated `error: TypedError` — never via the
/// Tauri error channel — so the frontend can render localized messages
/// without an additional `try/catch`.
#[tauri::command]
pub async fn start_cli(
    opts: WireStartCliOpts,
    state: State<'_, AppState>,
) -> Result<WireLaunchResult, String> {
    let profile_id = opts.profile_id.clone();
    let target_cli_internal = opts.target_cli_internal();
    let terminal_id_in = opts.terminal_id.clone();
    let cwd_in = opts.cwd.clone().unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    // Project the wire opts into the service-layer `StartCliOpts`. The wire
    // shape carries `terminal_id: String`; resolve it to the internal
    // `TerminalKind` via the existing wire-id parser.
    let preferred_kind = opts.terminal_id.as_deref().and_then(parse_terminal_wire_id);
    // Guard: `profile_id` is required at the wire boundary. The service layer
    // accepts `Option<String>` (active-profile fallback) but the contract
    // always carries an explicit id.
    // NOTE: e4-fixer's `cwd_override` wiring (StartCliOpts <- WireStartCliOpts.cwd)
    // is currently incomplete in the working tree — `StartCliOpts` itself does
    // not yet declare the field. Reverting the wire-side projection here to
    // restore a buildable state; the override is still echoed back to the
    // frontend via `cwd_in` in the failure envelope. Re-wire when the service
    // layer's `cwd_override` field lands.
    let service_opts = StartCliOpts {
        cli: target_cli_internal,
        profile_id: Some(opts.profile_id),
        terminal: preferred_kind,
        extra_args: opts.extra_args,
    };

    match LauncherService::start_cli(state.db.clone(), service_opts).await {
        Ok(ok) => Ok(WireLaunchResult::from_service(
            ok,
            profile_id,
            target_cli_internal,
            now,
        )),
        Err(err) => Ok(WireLaunchResult::from_error(
            err,
            profile_id,
            target_cli_internal,
            terminal_id_in.unwrap_or_default(),
            cwd_in,
            now,
        )),
    }
}

/// Open the per-profile workdir in the OS native file manager.
///
/// Returns the contract `OperationResult` envelope.
#[tauri::command]
pub async fn open_workdir(profile_id: String) -> Result<OperationResult, String> {
    match LauncherService::open_workdir(&profile_id).await {
        Ok(_) => Ok(OperationResult::ok()),
        Err(err) => {
            let typed = typed_error_from(&err);
            Ok(OperationResult {
                success: false,
                message: Some(typed.message),
                error_code: Some(typed.code),
            })
        }
    }
}

/// Preview what `start_cli` would do — does not spawn.
#[tauri::command]
pub async fn get_safety_summary(
    profile_id: String,
    target_cli: WireTargetCli,
    state: State<'_, AppState>,
) -> Result<WireSafetySummary, String> {
    let cli_internal = match target_cli {
        WireTargetCli::Claude => TargetCli::Claude,
        WireTargetCli::Codex => TargetCli::Codex,
    };
    match LauncherService::get_safety_summary(
        state.db.clone(),
        cli_internal,
        Some(profile_id.as_str()),
    )
    .await
    {
        Ok(summary) => {
            let l1_active = l1_active_total_count(&state);
            Ok(WireSafetySummary::project(
                summary,
                profile_id,
                cli_internal,
                l1_active,
            ))
        }
        Err(err) => {
            let te = typed_error_from(&err);
            Err(serde_json::to_string(&te).unwrap_or_else(|_| te.code.clone()))
        }
    }
}

/// Count L1 rules currently in force (enabled = true). Used to populate the
/// wire `l1_active_count` field at projection time.
fn l1_active_total_count(state: &State<'_, AppState>) -> usize {
    match sandbox::get_l1_rules(&state.db) {
        Ok(rules) => rules.into_iter().filter(|r| r.enabled).count(),
        Err(_) => 0,
    }
}
