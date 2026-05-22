//! Tauri commands for the launcher service (Task B5).
//!
//! Names mirror `src/lib/api/mock/launcher.ts` so the frontend can swap mock for
//! real Tauri invokes without contract changes.

use tauri::State;

use crate::services::installer::cli_install::TargetCli;
use crate::services::launcher_service::{
    LauncherError, LauncherService, SafetySummary, StartCliOpts, StartCliResult, TerminalInfo,
};
use crate::store::AppState;

/// Probe the host for installed terminal emulators.
#[tauri::command]
pub async fn detect_terminals() -> Result<Vec<TerminalInfo>, LauncherError> {
    Ok(LauncherService::detect_terminals().await)
}

/// Spawn the configured CLI inside the user's preferred terminal, in the per-profile workdir.
///
/// Failures (missing Node / CLI / profile / terminal) are returned as typed `LauncherError`
/// values so the frontend can surface localized messages.
#[tauri::command]
pub async fn start_cli(
    opts: StartCliOpts,
    state: State<'_, AppState>,
) -> Result<StartCliResult, LauncherError> {
    LauncherService::start_cli(state.db.clone(), opts).await
}

/// Open the per-profile workdir in the OS native file manager and return its absolute path.
#[tauri::command]
pub async fn open_workdir(profile_id: String) -> Result<String, LauncherError> {
    let path = LauncherService::open_workdir(&profile_id).await?;
    Ok(path.display().to_string())
}

/// Preview what `start_cli` would do — does not spawn.
#[tauri::command]
pub async fn get_safety_summary(
    cli: TargetCli,
    profile_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<SafetySummary, LauncherError> {
    LauncherService::get_safety_summary(state.db.clone(), cli, profile_id.as_deref()).await
}
