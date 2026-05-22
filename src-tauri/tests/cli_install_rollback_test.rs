// Intentional: serialize tests via std::sync::MutexGuard across `.await`.
// All tests in this file mutate the global `CC_SWITCH_TEST_HOME` env var;
// tokio's default single-threaded test runtime makes deadlock impossible here.
#![allow(clippy::await_holding_lock)]

//! Rollback behaviour for the CLI installer.
//!
//! Verifies the 6-step rollback model (research §"Rollback 流程"):
//! 1. KillSubprocess
//! 2. RemovePartialInstall
//! 3. RestoreSnapshot
//! 4. ClearCache
//! 5. ResetEnvVars
//! 6. EmitCleanedEvent
//!
//! For each step we simulate a failure pre-condition (partial install dir) and
//! assert state is cleaned up afterwards.

use cc_switch_lib::services::installer::cli_install::{
    execute_rollback_step, CliInstallStatus, CliInstaller, RollbackStep, TargetCli,
};
use cc_switch_lib::services::installer_service::InstallerService;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::tempdir;

/// Serialize tests in this file — they all mutate the global env var
/// `CC_SWITCH_TEST_HOME`, which prefix_dir() reads. Parallel tests would race.
fn env_lock() -> MutexGuard<'static, ()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn isolated_home() -> tempfile::TempDir {
    let tmp = tempdir().expect("tempdir");
    std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());
    tmp
}

#[test]
fn rollback_step_all_lists_exactly_six_steps_in_order() {
    // Use length + identity probes instead of name strings to avoid string-coupling.
    assert_eq!(RollbackStep::ALL.len(), 6);
    assert_eq!(RollbackStep::ALL[0], RollbackStep::KillSubprocess);
    assert_eq!(RollbackStep::ALL[1], RollbackStep::RemovePartialInstall);
    assert_eq!(RollbackStep::ALL[2], RollbackStep::RestoreSnapshot);
    assert_eq!(RollbackStep::ALL[3], RollbackStep::ClearCache);
    assert_eq!(RollbackStep::ALL[4], RollbackStep::ResetEnvVars);
    assert_eq!(RollbackStep::ALL[5], RollbackStep::EmitCleanedEvent);
}

#[test]
fn remove_partial_install_step_wipes_prefix_dir() {
    let _g = env_lock();
    let _tmp = isolated_home();

    // Simulate a partial install by creating the per-CLI prefix dir with files.
    let prefix = CliInstaller::prefix_dir(TargetCli::Claude).expect("prefix_dir");
    std::fs::create_dir_all(prefix.join("node_modules/foo")).expect("seed");
    std::fs::write(prefix.join("node_modules/foo/index.js"), b"throw 1").expect("seed file");
    assert!(prefix.exists(), "pre-condition: prefix must exist");

    execute_rollback_step(TargetCli::Claude, RollbackStep::RemovePartialInstall);
    assert!(
        !prefix.exists(),
        "RemovePartialInstall must wipe the prefix dir"
    );

    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[test]
fn remove_partial_install_is_idempotent_when_dir_missing() {
    let _g = env_lock();
    let _tmp = isolated_home();
    // Don't create the prefix. Step should still succeed without panic.
    execute_rollback_step(TargetCli::Codex, RollbackStep::RemovePartialInstall);
    let prefix = CliInstaller::prefix_dir(TargetCli::Codex).unwrap();
    assert!(!prefix.exists());
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[test]
fn kill_subprocess_step_is_safe_when_nothing_running() {
    let _g = env_lock();
    let _tmp = isolated_home();
    // No spawned npm subprocess in this test. Should be a no-op, must not panic.
    execute_rollback_step(TargetCli::Claude, RollbackStep::KillSubprocess);
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[test]
fn restore_snapshot_step_is_noop_for_clean_install() {
    let _g = env_lock();
    let _tmp = isolated_home();
    // Clean install scenario: no snapshot taken, so step must succeed silently.
    execute_rollback_step(TargetCli::Codex, RollbackStep::RestoreSnapshot);
    let prefix = CliInstaller::prefix_dir(TargetCli::Codex).unwrap();
    // Step must not magically create dirs.
    assert!(!prefix.exists());
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[test]
fn clear_cache_step_does_not_touch_user_cache() {
    let _g = env_lock();
    let _tmp = isolated_home();
    // The step intentionally does NOT mutate the global ~/.npm cache.
    // We can only assert it returns without panic; concrete cache untouched is
    // a contract not a runtime invariant we can check from here.
    execute_rollback_step(TargetCli::Claude, RollbackStep::ClearCache);
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[test]
fn reset_env_vars_step_does_not_modify_path_in_mvp() {
    let _g = env_lock();
    let _tmp = isolated_home();
    let before = std::env::var("PATH").unwrap_or_default();
    execute_rollback_step(TargetCli::Codex, RollbackStep::ResetEnvVars);
    let after = std::env::var("PATH").unwrap_or_default();
    // MVP guarantees PATH is never mutated (research §"PATH 注入策略 — 不动用户 PATH").
    assert_eq!(
        before, after,
        "ResetEnvVars must leave PATH untouched in MVP"
    );
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[test]
fn full_six_step_rollback_cleans_partial_install() {
    let _g = env_lock();
    let _tmp = isolated_home();
    let prefix = CliInstaller::prefix_dir(TargetCli::Claude).unwrap();
    // Seed a "partial install" tree
    std::fs::create_dir_all(prefix.join("node_modules/@anthropic-ai/claude-code")).unwrap();
    std::fs::write(
        prefix.join("node_modules/@anthropic-ai/claude-code/package.json"),
        b"{}",
    )
    .unwrap();
    assert!(prefix.exists());

    for step in RollbackStep::ALL {
        execute_rollback_step(TargetCli::Claude, step);
    }

    // After all 6 steps the per-CLI prefix must be gone.
    assert!(
        !prefix.exists(),
        "expected prefix {} to be gone after 6-step rollback",
        prefix.display()
    );
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[tokio::test]
async fn uninstall_cli_is_idempotent_via_service() {
    let _g = env_lock();
    let _tmp = isolated_home();
    // Neither CLI installed — uninstall must succeed twice without erroring.
    InstallerService::uninstall_cli(TargetCli::Claude).expect("first claude uninstall");
    InstallerService::uninstall_cli(TargetCli::Claude).expect("second claude uninstall");
    InstallerService::uninstall_cli(TargetCli::Codex).expect("codex uninstall");
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[tokio::test]
async fn detect_cli_returns_uninstalled_when_no_prefix() {
    let _g = env_lock();
    let _tmp = isolated_home();
    let status: CliInstallStatus = InstallerService::detect_cli(TargetCli::Claude).await;
    assert!(!status.installed);
    assert!(status.version.is_none());
    // last_checked must be ISO-8601 UTC (frontend contract uses `.datetime()`).
    assert!(status.last_checked.ends_with('Z') || status.last_checked.contains('+'));
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}
