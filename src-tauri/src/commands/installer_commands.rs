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
    on_progress: Channel<InstallProgress>,
) -> Result<NodeStatus, InstallerError> {
    let channel = on_progress.clone();
    InstallerService::install_node(move |p| {
        if let Err(e) = channel.send(p) {
            log::warn!("install_node channel.send failed: {e}");
        }
    })
    .await
}

/// Streaming CLI install (Claude / Codex).
#[tauri::command]
pub async fn install_cli(
    cli: TargetCli,
    opts: Option<InstallOpts>,
    on_progress: Channel<InstallProgress>,
) -> Result<CliInstallStatus, InstallerError> {
    let channel = on_progress.clone();
    InstallerService::install_cli(cli, opts.unwrap_or_default(), move |p| {
        if let Err(e) = channel.send(p) {
            log::warn!("install_cli channel.send failed: {e}");
        }
    })
    .await
}

/// Uninstall a CLI (idempotent — non-existent prefix is a no-op).
#[tauri::command]
pub async fn uninstall_cli(cli: TargetCli) -> Result<(), InstallerError> {
    InstallerService::uninstall_cli(cli)
}

/// Pick the lowest-latency npm registry across the 4-mirror whitelist.
#[tauri::command]
pub async fn smart_pick_registry() -> Result<RegistryPickResult, InstallerError> {
    InstallerService::smart_pick_registry().await
}
