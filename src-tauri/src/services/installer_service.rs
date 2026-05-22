//! Installer orchestrator — single entry point used by Tauri commands.
//!
//! Sits on top of the `installer/` sub-modules and exposes high-level operations
//! that map 1:1 to the frontend mock contract in `src/lib/api/mock/installer.ts`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::installer::cli_install::{
    CliInstallError, CliInstallStatus, CliInstaller, InstallOpts, TargetCli,
};
use super::installer::node_runtime::{InstallProgress, NodeRuntime, NodeRuntimeError, NodeStatus};
use super::installer::registry_probe::{
    RegistryPickResult, RegistryProbeError, RegistryProbeService,
};

#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstallerError {
    #[error("network unreachable: {0}")]
    Network(String),
    #[error("node runtime: {0}")]
    Node(String),
    #[error("cli install: {0}")]
    Cli(String),
    #[error("io: {0}")]
    Io(String),
    #[error("internal: {0}")]
    Internal(String),
}

impl From<RegistryProbeError> for InstallerError {
    fn from(err: RegistryProbeError) -> Self {
        match err {
            RegistryProbeError::AllUnreachable => InstallerError::Network(err.to_string()),
            RegistryProbeError::ClientBuild(_) => InstallerError::Internal(err.to_string()),
        }
    }
}

impl From<NodeRuntimeError> for InstallerError {
    fn from(err: NodeRuntimeError) -> Self {
        match err {
            NodeRuntimeError::Registry(_) => InstallerError::Network(err.to_string()),
            NodeRuntimeError::Http(_) => InstallerError::Network(err.to_string()),
            NodeRuntimeError::Io { .. } => InstallerError::Io(err.to_string()),
            _ => InstallerError::Node(err.to_string()),
        }
    }
}

impl From<CliInstallError> for InstallerError {
    fn from(err: CliInstallError) -> Self {
        match err {
            CliInstallError::NodeRuntime(inner) => InstallerError::from(inner),
            CliInstallError::Io { .. } => InstallerError::Io(err.to_string()),
            _ => InstallerError::Cli(err.to_string()),
        }
    }
}

pub struct InstallerService;

impl InstallerService {
    /// Detect a CLI's install state (private prefix only).
    pub async fn detect_cli(cli: TargetCli) -> CliInstallStatus {
        CliInstaller::detect(cli).await
    }

    /// Detect the private Node runtime.
    pub async fn detect_node() -> NodeStatus {
        NodeRuntime::detect().await
    }

    /// Pick the lowest-latency npm registry.
    pub async fn smart_pick_registry() -> Result<RegistryPickResult, InstallerError> {
        Ok(RegistryProbeService::smart_pick(false).await?)
    }

    /// Streaming Node install. Callers feed progress events into Tauri Channels.
    pub async fn install_node<F>(on_progress: F) -> Result<NodeStatus, InstallerError>
    where
        F: FnMut(InstallProgress) + Send,
    {
        Ok(NodeRuntime::install(on_progress).await?)
    }

    /// Streaming CLI install. Callers feed progress events into Tauri Channels.
    pub async fn install_cli<F>(
        cli: TargetCli,
        opts: InstallOpts,
        on_progress: F,
    ) -> Result<CliInstallStatus, InstallerError>
    where
        F: FnMut(InstallProgress) + Send,
    {
        Ok(CliInstaller::install(cli, opts, on_progress).await?)
    }

    /// Uninstall = wipe the per-CLI prefix dir.
    pub fn uninstall_cli(cli: TargetCli) -> Result<(), InstallerError> {
        Ok(CliInstaller::uninstall(cli)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::installer::registry_probe::REGISTRY_DEFS;

    #[test]
    fn registry_defs_match_research_whitelist() {
        let names: Vec<&str> = REGISTRY_DEFS.iter().map(|d| d.name).collect();
        assert_eq!(names, vec!["npmjs", "npmmirror", "tencent", "huawei"]);
    }

    #[test]
    fn registry_defs_use_https_only() {
        for def in REGISTRY_DEFS {
            assert!(
                def.url.starts_with("https://"),
                "registry {} must be https",
                def.name
            );
        }
    }

    #[tokio::test]
    async fn detect_node_returns_uninstalled_when_runtime_missing() {
        // Point CC_SWITCH_TEST_HOME to an empty tmpdir → no private runtime exists.
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());
        let status = InstallerService::detect_node().await;
        assert!(!status.installed);
        assert_eq!(status.major_version, None);
        assert!(status.is_private_runtime);
        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    #[tokio::test]
    async fn detect_cli_returns_uninstalled_when_prefix_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());
        let status = InstallerService::detect_cli(TargetCli::Claude).await;
        assert!(!status.installed);
        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    #[tokio::test]
    async fn uninstall_is_idempotent_when_nothing_installed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());
        // Both CLIs — neither installed — should succeed.
        InstallerService::uninstall_cli(TargetCli::Claude).unwrap();
        InstallerService::uninstall_cli(TargetCli::Codex).unwrap();
        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }
}
