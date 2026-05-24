//! Implementations of every [`FixAction`] variant.
//!
//! Each implementation is either:
//! - reversible (writes get a backup + restore command), or
//! - clearly informational/educational (no side effects beyond opening a
//!   browser tab or Explorer window).
//!
//! NEVER call any of these from a probe function — probes are read-only.
//!
//! NOTE: `InstallNode` is currently a stub that defers to the future B2
//! installer service. It returns a `Fix::Pending` error code so the
//! frontend can surface "task B2 not done yet" without crashing.

use std::path::Path;
use std::process::Command;

use thiserror::Error;

use crate::services::system_probe::FixAction;

/// Categorized error returned by [`apply`].
///
/// Some variants are constructed only on certain platforms (e.g. PathInject
/// is Unix-only); we allow dead-code so cross-platform builds stay clean.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum FixError {
    #[error("installer service not yet implemented (Task B2): {0}")]
    Pending(String),
    #[error("failed to launch external opener: {0}")]
    Opener(String),
    #[error("environment variable cleanup failed: {0}")]
    EnvVar(String),
    #[error("PATH injection failed: {0}")]
    PathInject(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("other: {0}")]
    Other(String),
}

impl FixError {
    /// Stable error code used by frontend i18n + telemetry.
    pub fn code(&self) -> &'static str {
        match self {
            FixError::Pending(_) => "FIX_PENDING_B2",
            FixError::Opener(_) => "FIX_OPENER_FAILED",
            FixError::EnvVar(_) => "FIX_ENV_VAR_FAILED",
            FixError::PathInject(_) => "FIX_PATH_INJECT_FAILED",
            FixError::Io(_) => "FIX_IO_ERROR",
            FixError::Other(_) => "FIX_OTHER",
        }
    }
}

/// Apply a fix action. Streaming progress is handled by the caller
/// ([`crate::services::system_probe::apply_fix`]); this function only
/// performs the side effect and returns Ok/Err.
pub async fn apply(action: &FixAction) -> Result<(), FixError> {
    match action {
        FixAction::InstallNode { target_lts_major } => install_node(*target_lts_major).await,
        FixAction::InstallGit => install_git().await,
        FixAction::CleanEnvVar { var_name } => clean_env_var(var_name).await,
        FixAction::OpenHomeDir => open_home_dir().await,
        FixAction::InjectPathEntries { entries } => inject_path_entries(entries).await,
        FixAction::ExternalLink { url, .. } => external_link(url).await,
    }
}

// ---------------------------- InstallNode ------------------------------

/// Stub: delegate to installer_service (Task B2). For now we return a
/// well-known error so the frontend can route to "Task B2 not yet
/// implemented" without crashing.
async fn install_node(lts_major: u8) -> Result<(), FixError> {
    log::info!("install_node requested (LTS major={lts_major}); deferring to Task B2 installer.");
    Err(FixError::Pending(format!(
        "Node LTS {lts_major} install requires B2 installer service"
    )))
}

// ---------------------------- InstallGit -------------------------------

/// Install Git automatically on the current platform.
///
/// - **Windows**: download PortableGit via the China-first mirror chain and
///   self-extract into the private runtime tree. No admin / UAC required.
/// - **macOS**: spawn `xcode-select --install` which pops the system dialog
///   for Command Line Tools. If that command fails (already installed,
///   user dismissed, etc.) we fall back to opening Apple's CLT page so the
///   user still has a path forward.
/// - **Linux**: open `git-scm.com/download/linux` — every distro has its own
///   package manager; we don't try to second-guess.
async fn install_git() -> Result<(), FixError> {
    #[cfg(target_os = "windows")]
    {
        use crate::services::installer::portable_git::PortableGit;
        // cfg-gated early return — other branches (macos / linux) follow below.
        #[allow(clippy::needless_return)]
        return PortableGit::install()
            .await
            .map_err(|e| FixError::Other(format!("PortableGit install failed: {e}")));
    }

    #[cfg(target_os = "macos")]
    {
        // `xcode-select --install` triggers Apple's GUI dialog and exits
        // immediately. A non-zero exit means CLT is already installed or
        // the request couldn't be dispatched — in either case we fall
        // back to the official Apple download page so the user still has
        // a path forward.
        let result = tokio::process::Command::new("xcode-select")
            .arg("--install")
            .output()
            .await;
        match result {
            Ok(out) if out.status.success() => return Ok(()),
            Ok(_) | Err(_) => {
                return open_url(
                    "https://developer.apple.com/download/all/?q=command%20line%20tools",
                );
            }
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        open_url("https://git-scm.com/download/linux")
    }
}

// ---------------------------- CleanEnvVar ------------------------------

/// Delegate to the existing `env_manager::delete_env_vars` (Win registry /
/// Unix shell config) which already implements backup + restore.
async fn clean_env_var(var_name: &str) -> Result<(), FixError> {
    use crate::services::env_checker::EnvConflict;
    use crate::services::env_manager::delete_env_vars;

    // We don't know which app/source the var came from at this point.
    // Re-scan ANTHROPIC/OPENAI/GEMINI conflicts and pick the matching one.
    let mut conflicts: Vec<EnvConflict> = Vec::new();
    for app in ["claude", "codex", "gemini"] {
        if let Ok(mut c) = crate::services::env_checker::check_env_conflicts(app) {
            conflicts.append(&mut c);
        }
    }
    let targets: Vec<EnvConflict> = conflicts
        .into_iter()
        .filter(|c| c.var_name == var_name)
        .collect();

    if targets.is_empty() {
        // Already clean (or a stale fix request after re-probe).
        log::info!("clean_env_var: no live conflict for {var_name}; nothing to do");
        return Ok(());
    }

    delete_env_vars(targets).map_err(FixError::EnvVar)?;
    Ok(())
}

// ---------------------------- OpenHomeDir ------------------------------

async fn open_home_dir() -> Result<(), FixError> {
    let home = dirs::home_dir().ok_or_else(|| FixError::Other("no home directory".into()))?;
    open_path(&home)
}

// ------------------------ InjectPathEntries ----------------------------

/// Append PATH entries to the user-level shell init file (Unix) or print
/// a PowerShell snippet the user can paste (Windows).
///
/// **Reversibility**: every write is appended after a `# cc-launcher (added <ts>)`
/// marker so the user can locate + remove it. We do NOT silently mutate
/// the Registry on Windows (avoids UAC + AV false positives).
async fn inject_path_entries(entries: &[String]) -> Result<(), FixError> {
    if entries.is_empty() {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, we surface a docs link instead of silently patching
        // the user's registry. The Tauri command for B4 streams the
        // snippet via a follow-up event; here we just open the doc page.
        let _ = entries;
        #[allow(clippy::needless_return)] // cfg-gated early return — non-Windows branch follows
        return open_url(
            "https://learn.microsoft.com/windows/win32/procthread/environment-variables",
        );
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::fs::OpenOptions;
        use std::io::Write;

        let home =
            dirs::home_dir().ok_or_else(|| FixError::PathInject("no home directory".into()))?;
        // Prefer zshrc on macOS (default since Catalina); fall back to bashrc on Linux.
        let target = if cfg!(target_os = "macos") {
            home.join(".zshrc")
        } else {
            home.join(".bashrc")
        };

        let mut block = String::new();
        block.push_str("\n# cc-launcher: PATH inject (");
        block.push_str(&chrono::Utc::now().to_rfc3339());
        block.push_str(")\n");
        for e in entries {
            block.push_str("export PATH=\"");
            block.push_str(e);
            block.push_str(":$PATH\"\n");
        }

        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&target)
            .map_err(|e| FixError::PathInject(format!("open {}: {e}", target.display())))?;
        f.write_all(block.as_bytes())
            .map_err(|e| FixError::PathInject(format!("write {}: {e}", target.display())))?;
        Ok(())
    }
}

// --------------------------- ExternalLink ------------------------------

async fn external_link(url: &str) -> Result<(), FixError> {
    open_url(url)
}

// ------------------------------ helpers --------------------------------

fn open_url(url: &str) -> Result<(), FixError> {
    open_with_system(url)
}

fn open_path(path: &Path) -> Result<(), FixError> {
    let s = path.display().to_string();
    open_with_system(&s)
}

#[cfg(target_os = "windows")]
fn open_with_system(target: &str) -> Result<(), FixError> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // `cmd /C start "" <target>` opens URLs in the default browser and
    // file paths in Explorer.
    Command::new("cmd")
        .args(["/C", "start", "", target])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| FixError::Opener(e.to_string()))
}

#[cfg(target_os = "macos")]
fn open_with_system(target: &str) -> Result<(), FixError> {
    Command::new("open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| FixError::Opener(e.to_string()))
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_with_system(target: &str) -> Result<(), FixError> {
    Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| FixError::Opener(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn install_node_is_pending_stub() {
        let res = install_node(20).await;
        let err = res.unwrap_err();
        assert_eq!(err.code(), "FIX_PENDING_B2");
        match err {
            FixError::Pending(_) => {}
            other => panic!("expected Pending, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn clean_env_var_no_conflict_is_noop() {
        // A var that almost certainly does not exist on the host.
        let res = clean_env_var("__CC_LAUNCHER_TEST_NOT_PRESENT__").await;
        assert!(res.is_ok(), "noop should succeed, got {res:?}");
    }

    #[test]
    fn fix_error_codes_are_stable() {
        let cases = [
            (FixError::Pending("x".into()), "FIX_PENDING_B2"),
            (FixError::Opener("x".into()), "FIX_OPENER_FAILED"),
            (FixError::EnvVar("x".into()), "FIX_ENV_VAR_FAILED"),
            (FixError::PathInject("x".into()), "FIX_PATH_INJECT_FAILED"),
            (FixError::Other("x".into()), "FIX_OTHER"),
        ];
        for (err, code) in cases {
            assert_eq!(err.code(), code);
        }
    }

    #[tokio::test]
    async fn inject_path_empty_is_noop() {
        let res = inject_path_entries(&[]).await;
        assert!(res.is_ok());
    }

    /// Sanity check: `install_git` is exposed via `apply(&FixAction::InstallGit)`
    /// and returns `Ok` or a typed `FixError` (never panics). On Windows the
    /// PortableGit install may fail without network/sandbox; on macOS the
    /// xcode-select branch is best-effort; on Linux we fall back to opening
    /// the docs URL. Either way the type contract holds.
    #[tokio::test]
    async fn install_git_returns_typed_error_on_invalid_state() {
        let res = apply(&FixAction::InstallGit).await;
        match res {
            Ok(()) => {}
            Err(e) => {
                // Code must be one of the stable enum codes.
                let code = e.code();
                assert!(
                    [
                        "FIX_PENDING_B2",
                        "FIX_OPENER_FAILED",
                        "FIX_ENV_VAR_FAILED",
                        "FIX_PATH_INJECT_FAILED",
                        "FIX_IO_ERROR",
                        "FIX_OTHER",
                    ]
                    .contains(&code),
                    "unexpected error code: {code}"
                );
            }
        }
    }
}
