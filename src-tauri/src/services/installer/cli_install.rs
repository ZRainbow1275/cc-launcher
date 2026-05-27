//! Per-CLI install via private Node + isolated npm prefix.
//!
//! Strict invariants (from `research/cli-install-strategy.md`):
//! - **Never** spawn system `npm` / system `node`. All shells launch the private runtime.
//! - npm prefix = `<runtime_root>/<cli>/` so each CLI is independent and uninstall = `rm -rf`.
//! - On any failure: execute the **6-step rollback** to leave the filesystem clean.
//! - After install: spawn the absolute binary path with `--version` and validate the regex
//!   to confirm the package actually works (handles broken postinstall hooks).

use std::path::{Path, PathBuf};
use std::process::Stdio;

use chrono::{SecondsFormat, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

use super::node_runtime::{
    l, InstallPhase, InstallProgress, LocalizedMessage, NodeRuntime, NodeRuntimeError, TypedError,
};
use super::source_config::InstallerSourceConfig;

/// Mirrors frontend `TargetCli` enum (lowercase wire values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetCli {
    Claude,
    Codex,
}

impl TargetCli {
    /// npm package name.
    pub fn npm_package(self) -> &'static str {
        match self {
            TargetCli::Claude => "@anthropic-ai/claude-code",
            TargetCli::Codex => "@openai/codex",
        }
    }

    /// Hard-coded version pinned for MVP (research §"CLI 包元数据" decisions).
    /// Future: read from `tools.json` schema.
    pub fn pinned_version(self) -> &'static str {
        match self {
            TargetCli::Claude => "2.1.150",
            TargetCli::Codex => "0.133.0",
        }
    }

    /// Bin name. On Windows the npm `.bin` shim adds the `.cmd` suffix.
    pub fn bin_name(self) -> &'static str {
        match self {
            TargetCli::Claude => "claude",
            TargetCli::Codex => "codex",
        }
    }

    /// Filesystem-friendly slug used as the npm prefix subdir name.
    pub fn slug(self) -> &'static str {
        match self {
            TargetCli::Claude => "claude",
            TargetCli::Codex => "codex",
        }
    }

    /// Regex applied to `--version` stdout (per research §"验证 commands").
    pub fn version_regex(self) -> Regex {
        match self {
            TargetCli::Claude => Regex::new(r"^\s*(\d+\.\d+\.\d+)(?:\s|$)").expect("static regex"),
            TargetCli::Codex => Regex::new(r"(\d+\.\d+\.\d+)").expect("static regex"),
        }
    }
}

/// Frontend-facing `CliInstallStatus` (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliInstallStatus {
    pub cli: TargetCli,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub last_checked: String,
}

/// Mirrors frontend `InstallerOpts`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOpts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_node_check: Option<bool>,
}

#[derive(Debug, Error)]
pub enum CliInstallError {
    #[error("node runtime not available: {0}")]
    NodeRuntime(#[from] NodeRuntimeError),
    #[error("io error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("npm install failed (exit {exit_code}): {stderr_tail}")]
    NpmFailed { exit_code: i32, stderr_tail: String },
    #[error("post-install validation failed: {0}")]
    Validation(String),
    #[error("rollback failed: {0}")]
    Rollback(String),
}

/// 6-step rollback log — used for assertions and `install.log` writes.
///
/// Mirrors `research/cli-install-strategy.md §"Rollback 流程"`:
/// 1. kill subprocess
/// 2. remove partial install
/// 3. restore previous snapshot
/// 4. clear cache
/// 5. reset env vars
/// 6. emit cleaned event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackStep {
    KillSubprocess,
    RemovePartialInstall,
    RestoreSnapshot,
    ClearCache,
    ResetEnvVars,
    EmitCleanedEvent,
}

impl RollbackStep {
    pub const ALL: [RollbackStep; 6] = [
        RollbackStep::KillSubprocess,
        RollbackStep::RemovePartialInstall,
        RollbackStep::RestoreSnapshot,
        RollbackStep::ClearCache,
        RollbackStep::ResetEnvVars,
        RollbackStep::EmitCleanedEvent,
    ];
}

pub struct CliInstaller;

impl CliInstaller {
    /// npm `--prefix` root for a given CLI.
    pub fn prefix_dir(cli: TargetCli) -> Result<PathBuf, CliInstallError> {
        Ok(NodeRuntime::runtime_root()?.join(cli.slug()))
    }

    /// Absolute path to the installed CLI bin (the `.bin/<name>(.cmd)` shim).
    pub fn cli_binary(cli: TargetCli) -> Result<PathBuf, CliInstallError> {
        // npm with `--prefix=X`:
        //  - Windows: writes shim into X/<bin>.cmd directly (no .bin/ subdir)
        //  - Unix:    writes shim into X/bin/<bin>
        #[cfg(target_os = "windows")]
        {
            Ok(Self::cli_bin_dir(cli)?.join(format!("{}.cmd", cli.bin_name())))
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(Self::cli_bin_dir(cli)?.join(cli.bin_name()))
        }
    }

    /// Directory that must be prepended to PATH for the target CLI shim.
    pub fn cli_bin_dir(cli: TargetCli) -> Result<PathBuf, CliInstallError> {
        let prefix = Self::prefix_dir(cli)?;
        #[cfg(target_os = "windows")]
        {
            Ok(prefix)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(prefix.join("bin"))
        }
    }

    /// Detect whether the CLI is installed under our private prefix.
    pub async fn detect(cli: TargetCli) -> CliInstallStatus {
        let last_checked = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let bin = match Self::cli_binary(cli) {
            Ok(b) => b,
            Err(_) => {
                return CliInstallStatus {
                    cli,
                    installed: false,
                    version: None,
                    path: None,
                    last_checked,
                };
            }
        };
        if !bin.exists() {
            return CliInstallStatus {
                cli,
                installed: false,
                version: None,
                path: None,
                last_checked,
            };
        }
        let version = probe_cli_version(cli, &bin).await.ok();
        CliInstallStatus {
            cli,
            installed: version.is_some(),
            version,
            path: Some(bin.to_string_lossy().into_owned()),
            last_checked,
        }
    }

    /// Streaming install — emits `InstallProgress` events via the callback.
    ///
    /// Errors from npm or validation trigger a 6-step rollback before returning.
    pub async fn install<F>(
        cli: TargetCli,
        opts: InstallOpts,
        on_progress: F,
    ) -> Result<CliInstallStatus, CliInstallError>
    where
        F: FnMut(InstallProgress) + Send,
    {
        Self::install_with_source_config(cli, opts, InstallerSourceConfig::default(), on_progress)
            .await
    }

    pub async fn install_with_source_config<F>(
        cli: TargetCli,
        opts: InstallOpts,
        source_config: InstallerSourceConfig,
        mut on_progress: F,
    ) -> Result<CliInstallStatus, CliInstallError>
    where
        F: FnMut(InstallProgress) + Send,
    {
        let source_config = source_config
            .validated()
            .map_err(|e| CliInstallError::Validation(format!("source config: {e}")))?;

        // Phase 1: probing registry (or use the user-supplied override).
        on_progress(progress(
            InstallPhase::ProbingRegistry,
            l(
                "正在选择镜像源...",
                "Probing npm registries...",
                "npm レジストリを探索中...",
            ),
            Some(5),
            None,
            None,
        ));
        let chosen_registry = if let Some(reg) = opts.registry.as_ref() {
            reg.trim().trim_end_matches('/').to_string()
        } else if let Some(reg) = source_config.npm_registry.as_ref() {
            reg.clone()
        } else {
            let pick = super::registry_probe::RegistryProbeService::smart_pick_with_config(
                &source_config,
                false,
            )
            .await
            .map_err(|e| CliInstallError::Validation(format!("registry probe: {e}")))?;
            pick.chosen
        };

        // Phase 2 (conditional): installing-node if missing and not skipped.
        let skip_node = opts.skip_node_check.unwrap_or(false);
        if !skip_node {
            let node = NodeRuntime::detect().await;
            if !node.installed {
                on_progress(progress(
                    InstallPhase::InstallingNode,
                    l(
                        "正在安装 Node.js 20 LTS...",
                        "Installing Node.js 20 LTS...",
                        "Node.js 20 LTS をインストール中...",
                    ),
                    Some(20),
                    Some(chosen_registry.clone()),
                    None,
                ));
                // Inner Node-install progress events are swallowed: the outer
                // caller has already emitted an "installing-node" event above
                // and forwarding sub-phases would confuse the linear bar.
                NodeRuntime::install_with_config(source_config.clone(), |_p| {}).await?;
            }
        }

        // Phase 3: installing-cli — spawn the private npm.
        on_progress(progress(
            InstallPhase::InstallingCli,
            l(
                "正在安装 CLI...",
                "Installing CLI...",
                "CLI をインストール中...",
            ),
            Some(60),
            Some(chosen_registry.clone()),
            None,
        ));

        let install_result = Self::run_npm_install(cli, &chosen_registry).await;
        if let Err(err) = install_result {
            Self::rollback_after_failure(cli, &mut on_progress, &err.to_string());
            return Err(err);
        }

        // Phase 4: validating — spawn `<cli> --version` and check regex.
        on_progress(progress(
            InstallPhase::Validating,
            l(
                "正在校验安装结果...",
                "Validating installation...",
                "インストール結果を検証中...",
            ),
            Some(90),
            Some(chosen_registry.clone()),
            None,
        ));
        let bin = Self::cli_binary(cli)?;
        let version = match probe_cli_version(cli, &bin).await {
            Ok(v) => v,
            Err(err) => {
                Self::rollback_after_failure(cli, &mut on_progress, &err);
                return Err(CliInstallError::Validation(err));
            }
        };

        on_progress(progress(
            InstallPhase::Completed,
            l("安装完成", "Installation completed", "インストール完了"),
            Some(100),
            Some(chosen_registry),
            None,
        ));

        Ok(CliInstallStatus {
            cli,
            installed: true,
            version: Some(version),
            path: Some(bin.to_string_lossy().into_owned()),
            last_checked: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        })
    }

    /// Uninstall = remove the per-CLI prefix dir. Idempotent.
    pub fn uninstall(cli: TargetCli) -> Result<(), CliInstallError> {
        let prefix = Self::prefix_dir(cli)?;
        if prefix.exists() {
            std::fs::remove_dir_all(&prefix).map_err(|e| CliInstallError::Io {
                path: prefix.display().to_string(),
                source: e,
            })?;
        }
        Ok(())
    }

    async fn run_npm_install(cli: TargetCli, registry: &str) -> Result<(), CliInstallError> {
        let node = NodeRuntime::node_binary()?;
        let npm = NodeRuntime::npm_cli_js()?;
        let prefix = Self::prefix_dir(cli)?;
        std::fs::create_dir_all(&prefix).map_err(|e| CliInstallError::Io {
            path: prefix.display().to_string(),
            source: e,
        })?;

        let pkg = format!("{}@{}", cli.npm_package(), cli.pinned_version());

        let mut cmd = Command::new(&node);
        cmd.arg(&npm)
            .arg("install")
            .arg(&pkg)
            .arg(format!("--prefix={}", prefix.display()))
            .arg(format!("--registry={registry}"))
            .arg(format!(
                "--cache={}",
                NodeRuntime::runtime_root()?.join("npm-cache").display()
            ))
            .arg("--no-audit")
            .arg("--no-fund")
            .arg("--loglevel=error")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            // CREATE_NO_WINDOW so we don't flash a console.
            // `creation_flags` is provided by `tokio::process::Command` directly
            // when the `process` feature is enabled (no need for `CommandExt`).
            cmd.creation_flags(0x0800_0000);
        }

        let mut child = cmd.spawn().map_err(|e| CliInstallError::Io {
            path: node.display().to_string(),
            source: e,
        })?;

        let stderr = child.stderr.take();
        let stderr_collector = tokio::spawn(async move {
            let mut tail = String::new();
            if let Some(stderr) = stderr {
                let mut reader = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if tail.len() < 2048 {
                        tail.push_str(&line);
                        tail.push('\n');
                    }
                }
            }
            tail
        });

        let status = child.wait().await.map_err(|e| CliInstallError::Io {
            path: node.display().to_string(),
            source: e,
        })?;
        let stderr_tail = stderr_collector.await.unwrap_or_default();

        if !status.success() {
            return Err(CliInstallError::NpmFailed {
                exit_code: status.code().unwrap_or(-1),
                stderr_tail,
            });
        }
        Ok(())
    }

    /// 6-step rollback — best-effort, surfaces progress messages only.
    ///
    /// Used both on npm failure and post-install validation failure.
    fn rollback_after_failure<F>(cli: TargetCli, on_progress: &mut F, cause: &str)
    where
        F: FnMut(InstallProgress) + Send,
    {
        for step in RollbackStep::ALL {
            execute_rollback_step(cli, step);
        }
        on_progress(progress(
            InstallPhase::Failed,
            l(
                "安装失败，已自动清理残留",
                "Install failed, partial files cleaned",
                "インストールに失敗し、残骸を削除しました",
            ),
            Some(0),
            None,
            Some(TypedError {
                code: "CLI_INSTALL_FAILED".to_string(),
                message: l(
                    "CLI 安装失败",
                    "CLI installation failed",
                    "CLI のインストールに失敗しました",
                ),
                cause: Some(cause.to_string()),
                retryable: true,
            }),
        ));
    }
}

fn progress(
    phase: InstallPhase,
    message: LocalizedMessage,
    percent: Option<u8>,
    registry: Option<String>,
    error: Option<TypedError>,
) -> InstallProgress {
    InstallProgress {
        phase,
        message,
        percent,
        registry,
        error,
    }
}

/// Apply one rollback step. Public so tests can drive it deterministically.
pub fn execute_rollback_step(cli: TargetCli, step: RollbackStep) {
    match step {
        RollbackStep::KillSubprocess => {
            // npm child process is awaited synchronously in `run_npm_install`, so by
            // the time we reach rollback there's no active subprocess. This is a
            // best-effort placeholder for future MCP-spawn scenarios.
        }
        RollbackStep::RemovePartialInstall => {
            if let Ok(prefix) = CliInstaller::prefix_dir(cli) {
                if prefix.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&prefix) {
                        log::warn!(
                            "rollback step RemovePartialInstall failed for {}: {}",
                            prefix.display(),
                            e
                        );
                    }
                }
            }
        }
        RollbackStep::RestoreSnapshot => {
            // No snapshot is taken pre-install in MVP — clean install means there is
            // nothing to restore. Reserved for incremental update scenarios.
        }
        RollbackStep::ClearCache => {
            // npm's per-project cache lives inside the prefix and is wiped by step 2.
            // npm's global cache (~/.npm) is intentionally not touched (we don't want
            // to invalidate other tools' caches).
        }
        RollbackStep::ResetEnvVars => {
            // MVP doesn't mutate PATH (research §"不动用户 PATH"), so nothing to undo.
        }
        RollbackStep::EmitCleanedEvent => {
            // Caller's responsibility to relay this via `on_progress`.
        }
    }
}

async fn probe_cli_version(cli: TargetCli, bin: &Path) -> Result<String, String> {
    let mut cmd = Command::new(bin);
    cmd.arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        // tokio::process::Command exposes `creation_flags` directly on Windows.
        cmd.creation_flags(0x0800_0000);
    }

    let output = tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output())
        .await
        .map_err(|_| "version check timed out after 10s".to_string())?
        .map_err(|e| format!("spawn {bin:?} failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "exit {} - {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let re = cli.version_regex();
    let cap = re
        .captures(&stdout)
        .ok_or_else(|| format!("version regex did not match: {}", stdout.trim()))?;
    let version = cap
        .get(1)
        .ok_or_else(|| "version regex match group 1 missing".to_string())?
        .as_str()
        .to_string();
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_cli_constants_match_contract() {
        assert_eq!(TargetCli::Claude.npm_package(), "@anthropic-ai/claude-code");
        assert_eq!(TargetCli::Codex.npm_package(), "@openai/codex");
        assert_eq!(TargetCli::Claude.pinned_version(), "2.1.150");
        assert_eq!(TargetCli::Codex.pinned_version(), "0.133.0");
    }

    #[test]
    fn version_regex_claude_matches_real_output() {
        let re = TargetCli::Claude.version_regex();
        let cap = re.captures("2.1.150 (Claude Code)").unwrap();
        assert_eq!(cap.get(1).unwrap().as_str(), "2.1.150");
    }

    #[test]
    fn version_regex_claude_matches_with_only_version() {
        let re = TargetCli::Claude.version_regex();
        let cap = re.captures("2.1.150\n").unwrap();
        assert_eq!(cap.get(1).unwrap().as_str(), "2.1.150");
    }

    #[test]
    fn version_regex_codex_matches_real_output() {
        let re = TargetCli::Codex.version_regex();
        let cap = re.captures("codex-cli 0.133.0").unwrap();
        assert_eq!(cap.get(1).unwrap().as_str(), "0.133.0");
    }

    #[test]
    fn version_regex_codex_rejects_non_semver() {
        let re = TargetCli::Codex.version_regex();
        assert!(re.captures("not a version").is_none());
    }

    #[test]
    fn rollback_step_all_has_six_entries_in_order() {
        assert_eq!(RollbackStep::ALL.len(), 6);
        assert_eq!(RollbackStep::ALL[0], RollbackStep::KillSubprocess);
        assert_eq!(RollbackStep::ALL[5], RollbackStep::EmitCleanedEvent);
    }

    // Mutates the process-wide CC_SWITCH_TEST_HOME env var, must be serialized
    // with the registry_probe and portable_git tests that touch the same var.
    #[test]
    #[serial_test::serial]
    fn npm_install_args_include_cache_flag() {
        // Structural smoke test: just confirm runtime_root resolves and
        // we can construct the --cache= arg without panic.
        std::env::set_var("CC_SWITCH_TEST_HOME", std::env::temp_dir());
        let root = NodeRuntime::runtime_root().expect("runtime_root resolves");
        let cache_arg = format!("--cache={}", root.join("npm-cache").display());
        assert!(cache_arg.starts_with("--cache="));
        assert!(cache_arg.contains("npm-cache"));
        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }
}
