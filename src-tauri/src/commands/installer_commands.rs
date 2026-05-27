//! Tauri commands for the installer service.
//!
//! Streaming commands use `tauri::ipc::Channel<InstallProgress>` so the
//! frontend can subscribe to progress events without polling.
//!
//! Names mirror the frontend mock surface in `src/lib/api/mock/installer.ts`.

use tauri::ipc::Channel;

use crate::services::installer::cli_install::{CliInstallStatus, InstallOpts, TargetCli};
use crate::services::installer::node_runtime::{InstallProgress, NodeStatus};
use crate::services::installer::registry_probe::RegistryPickResult;
use crate::services::installer_service::{InstallerError, InstallerService};
use crate::store::AppState;
use crate::types::OperationResult;

/// Detect whether a CLI is installed under the private prefix.
#[tauri::command]
pub async fn detect_cli(cli: TargetCli) -> Result<CliInstallStatus, InstallerError> {
    Ok(InstallerService::detect_cli(cli).await)
}

/// Detect the private Node runtime.
#[tauri::command]
pub async fn detect_node() -> Result<NodeStatus, InstallerError> {
    Ok(InstallerService::detect_node().await)
}

/// Streaming Node install.
///
/// Frontend subscribes via `Channel<InstallProgress>`; this command resolves
/// after the install completes (or fails — failure events are still sent through
/// the channel before the outer Result errors).
#[tauri::command]
pub async fn install_node(
    state: tauri::State<'_, AppState>,
    on_progress: Channel<InstallProgress>,
) -> Result<NodeStatus, InstallerError> {
    let source_config =
        super::onboarding_settings::settings_get_installer_source_config_internal(&state)
            .map_err(|e| InstallerError::Internal(e.to_string()))?;
    let channel = on_progress.clone();
    InstallerService::install_node_with_config(source_config, move |p| {
        if let Err(e) = channel.send(p) {
            log::warn!("install_node channel.send failed: {e}");
        }
    })
    .await
}

/// Streaming CLI install (Claude / Codex).
#[tauri::command]
pub async fn install_cli(
    state: tauri::State<'_, AppState>,
    cli: TargetCli,
    opts: Option<InstallOpts>,
    on_progress: Channel<InstallProgress>,
) -> Result<CliInstallStatus, InstallerError> {
    let source_config =
        super::onboarding_settings::settings_get_installer_source_config_internal(&state)
            .map_err(|e| InstallerError::Internal(e.to_string()))?;
    let channel = on_progress.clone();
    InstallerService::install_cli_with_config(
        cli,
        opts.unwrap_or_default(),
        source_config,
        move |p| {
            if let Err(e) = channel.send(p) {
                log::warn!("install_cli channel.send failed: {e}");
            }
        },
    )
    .await
}

/// Uninstall a CLI (idempotent — non-existent prefix is a no-op).
#[tauri::command]
pub async fn uninstall_cli(cli: TargetCli) -> Result<OperationResult, InstallerError> {
    InstallerService::uninstall_cli(cli)?;
    Ok(OperationResult::ok())
}

/// Pick the lowest-latency npm registry across the 4-mirror whitelist.
#[tauri::command]
pub async fn smart_pick_registry(
    state: tauri::State<'_, AppState>,
) -> Result<RegistryPickResult, InstallerError> {
    let source_config =
        super::onboarding_settings::settings_get_installer_source_config_internal(&state)
            .map_err(|e| InstallerError::Internal(e.to_string()))?;
    InstallerService::smart_pick_registry_with_config(&source_config).await
}
