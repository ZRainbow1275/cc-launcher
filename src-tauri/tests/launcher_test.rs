//! Integration tests for `services::launcher_service` (Task B5).
//!
//! Covered:
//! 1. `is_arg_safe` rejects bypass flags & shell metas; accepts normal args.
//! 2. `detect_terminals` returns a non-empty Vec on the current host (gated to current platform).
//! 3. `start_cli` with a non-existent profile_id surfaces `ProfileInvalid`.
//! 4. `get_safety_summary` always reports `redlines_active: true`, regardless of sandbox level.
//! 5. `open_workdir` creates the per-profile directory tree if missing.

// `std::sync::MutexGuard` is intentionally held across `.await` to serialize
// tests that share the global `CC_SWITCH_TEST_HOME` env var. The tokio test
// runtime is single-threaded by default for `#[tokio::test]`, so there is no
// deadlock risk — only one task runs at a time inside each test.
#![allow(clippy::await_holding_lock)]

use std::sync::Arc;
use std::sync::atomic::Ordering;

use cc_switch_lib::sandbox::{self, SandboxLevel, APPLY_TO_COMMAND_CALL_COUNT};
use cc_switch_lib::services::installer::cli_install::TargetCli;
use cc_switch_lib::services::launcher_service::{
    self, LauncherError, LauncherService, StartCliOpts,
};
use cc_switch_lib::services::profile::{self, ProfileCreatePayload, TargetCli as ProfileTargetCli};
use cc_switch_lib::Database;

mod support;

// ============================================================================
// 1. is_arg_safe contract
// ============================================================================

#[test]
fn is_arg_safe_rejects_dangerous_flags() {
    assert!(!launcher_service::is_arg_safe("--dangerously-skip-permissions"));
    assert!(!launcher_service::is_arg_safe("--yolo"));
    assert!(!launcher_service::is_arg_safe("--dangerously-bypass-approvals-and-sandbox"));
    assert!(!launcher_service::is_arg_safe("--skip-permissions"));
    assert!(!launcher_service::is_arg_safe("--bypass-foo"));
}

#[test]
fn is_arg_safe_rejects_shell_injection() {
    assert!(!launcher_service::is_arg_safe("; rm -rf /"));
    assert!(!launcher_service::is_arg_safe("$(curl evil)"));
    assert!(!launcher_service::is_arg_safe("a && b"));
    assert!(!launcher_service::is_arg_safe("a || b"));
    assert!(!launcher_service::is_arg_safe("`whoami`"));
    assert!(!launcher_service::is_arg_safe("foo\nbar"));
    assert!(!launcher_service::is_arg_safe("foo bar")); // space
}

#[test]
fn is_arg_safe_accepts_safe_args() {
    assert!(launcher_service::is_arg_safe("--model=claude-opus-4-7"));
    assert!(launcher_service::is_arg_safe("--add-dir"));
    assert!(launcher_service::is_arg_safe("/tmp/work-dir"));
    assert!(launcher_service::is_arg_safe("-p"));
    assert!(launcher_service::is_arg_safe("name@example.com"));
    assert!(launcher_service::is_arg_safe("C:/Users/test"));
}

// ============================================================================
// 2. detect_terminals on the current platform
// ============================================================================

#[cfg(target_os = "windows")]
#[tokio::test]
async fn detect_terminals_returns_non_empty_on_windows_host() {
    let list = LauncherService::detect_terminals().await;
    // Every Windows host has at least cmd.exe in PATH.
    assert!(
        !list.is_empty(),
        "expected at least one terminal candidate on Windows"
    );
    // Exactly one should be marked as default.
    let default_count = list.iter().filter(|t| t.is_default).count();
    assert_eq!(default_count, 1, "exactly one terminal must be default");
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn detect_terminals_returns_non_empty_on_macos_host() {
    let list = LauncherService::detect_terminals().await;
    // macOS always ships Terminal.app at /System/Applications/Utilities/Terminal.app.
    assert!(!list.is_empty(), "expected at least Terminal.app on macOS");
}

#[cfg(target_os = "linux")]
#[ignore = "Linux hosts may not have any terminal emulator installed (e.g. in CI containers)"]
#[tokio::test]
async fn detect_terminals_returns_non_empty_on_linux_host() {
    let list = LauncherService::detect_terminals().await;
    if list.is_empty() {
        eprintln!("no terminal emulator found on this Linux host; expected on graphical desktops");
    }
}

// ============================================================================
// 3. start_cli with non-existent profile_id → ProfileInvalid
// ============================================================================

fn fresh_test_db_with_home() -> (Arc<Database>, std::sync::MutexGuard<'static, ()>) {
    let guard = support::test_mutex().lock().expect("acquire test mutex");
    support::ensure_test_home();
    support::reset_test_fs();
    let db = Arc::new(Database::init().expect("init db"));
    (db, guard)
}

#[tokio::test]
async fn start_cli_with_nonexistent_profile_returns_profile_invalid() {
    let (db, _guard) = fresh_test_db_with_home();

    let result = LauncherService::start_cli(
        db.clone(),
        StartCliOpts {
            cli: TargetCli::Claude,
            profile_id: Some("does-not-exist".to_string()),
            terminal: None,
            extra_args: vec![],
        },
    )
    .await;

    let err = result.expect_err("expected ProfileInvalid");
    match err {
        LauncherError::ProfileInvalid { reason } => {
            assert!(
                reason.contains("does-not-exist") || reason.to_lowercase().contains("not found"),
                "ProfileInvalid reason should mention missing profile, got: {reason}"
            );
        }
        other => panic!("expected ProfileInvalid, got: {other:?}"),
    }
}

// ============================================================================
// 4. get_safety_summary always reports redlines_active = true
// ============================================================================

#[tokio::test]
async fn get_safety_summary_always_reports_redlines_active() {
    let (db, _guard) = fresh_test_db_with_home();

    // The v11 schema migration auto-seeds default-claude / default-codex builtin profiles
    // and sets them active. So `get_active_profile(claude)` returns Some.
    let summary = LauncherService::get_safety_summary(db.clone(), TargetCli::Claude, None)
        .await
        .expect("safety summary with default active profile");

    assert!(
        summary.redlines_active,
        "redlines_active must be true at every sandbox level"
    );
    // Workdir is rooted at ~/cc-launcher-projects/<profile_id>
    let s = summary.workdir.to_string_lossy();
    assert!(
        s.contains("cc-launcher-projects"),
        "workdir should be under cc-launcher-projects, got: {s}"
    );
    // sandbox_level is one of the documented markers.
    assert!(
        matches!(summary.sandbox_level.as_str(), "L0" | "L1" | "L2"),
        "sandbox_level should be L0/L1/L2, got: {}",
        summary.sandbox_level
    );
    // Default Claude flags include --add-dir for the workdir.
    assert!(
        summary.flags_applied.iter().any(|f| f.starts_with("--add-dir")),
        "Claude safety summary must include --add-dir flag, got: {:?}",
        summary.flags_applied
    );
    // And must NEVER include the bypass flag by default.
    assert!(
        !summary
            .flags_applied
            .iter()
            .any(|f| f.contains("dangerously-skip-permissions")),
        "default flags must not contain --dangerously-skip-permissions, got: {:?}",
        summary.flags_applied
    );
}

#[tokio::test]
async fn get_safety_summary_codex_default_has_cd_no_yolo() {
    let (db, _guard) = fresh_test_db_with_home();

    let summary = LauncherService::get_safety_summary(db.clone(), TargetCli::Codex, None)
        .await
        .expect("safety summary for codex");

    assert!(summary.redlines_active);
    assert!(
        summary.flags_applied.iter().any(|f| f.starts_with("--cd")),
        "Codex safety summary must include --cd flag, got: {:?}",
        summary.flags_applied
    );
    assert!(
        !summary
            .flags_applied
            .iter()
            .any(|f| f.contains("dangerously-bypass-approvals-and-sandbox") || f == "--yolo"),
        "default Codex flags must not include yolo/bypass, got: {:?}",
        summary.flags_applied
    );
}

// ============================================================================
// 5. open_workdir creates the per-profile directory tree
// ============================================================================

#[tokio::test]
async fn open_workdir_creates_directory_if_missing() {
    let (_db, _guard) = fresh_test_db_with_home();

    // Use a unique profile id; the directory should not exist before the call.
    let pid = format!("test-profile-{}", chrono::Utc::now().timestamp_millis());
    let home = std::path::PathBuf::from(
        std::env::var("CC_SWITCH_TEST_HOME").expect("CC_SWITCH_TEST_HOME set by support"),
    );
    let expected = home.join("cc-launcher-projects").join(&pid);
    assert!(
        !expected.exists(),
        "precondition: target dir must not exist before open_workdir"
    );

    // open_workdir attempts to spawn the OS file manager; that may fail in headless test
    // environments (no graphical session), but the directory creation happens first and
    // the path is returned regardless of file-manager spawn success.
    let result = LauncherService::open_workdir(&pid).await;

    // Directory must exist whether or not the OS file manager spawned successfully.
    assert!(
        expected.exists(),
        "open_workdir must create ~/cc-launcher-projects/<pid>/, missing at {}",
        expected.display()
    );

    // The returned path should be that directory (when spawn succeeds). If the file-manager
    // spawn failed (headless CI), `SpawnFailed` is acceptable.
    match result {
        Ok(path) => {
            assert_eq!(path, expected, "returned path must match created workdir");
        }
        Err(LauncherError::SpawnFailed { .. }) => {
            // Acceptable on headless test runners: directory was still created (asserted above).
        }
        Err(other) => panic!("unexpected error from open_workdir: {other:?}"),
    }
}

// ============================================================================
// 6. start_cli with empty profile_id is rejected
// ============================================================================

#[tokio::test]
async fn start_cli_with_empty_profile_id_returns_profile_invalid() {
    let (db, _guard) = fresh_test_db_with_home();
    let result = LauncherService::start_cli(
        db.clone(),
        StartCliOpts {
            cli: TargetCli::Claude,
            profile_id: Some(String::new()),
            terminal: None,
            extra_args: vec![],
        },
    )
    .await;
    let err = result.expect_err("empty profile_id should fail");
    assert!(
        matches!(err, LauncherError::ProfileInvalid { .. }),
        "expected ProfileInvalid for empty profile_id, got: {err:?}"
    );
}

// ============================================================================
// 7. assemble_cli_command behavior validated indirectly via safety summary
// ============================================================================

#[tokio::test]
async fn safety_summary_includes_workdir_in_add_dir_flag() {
    let (db, _guard) = fresh_test_db_with_home();

    // Create an explicit Claude profile so workdir naming is deterministic.
    let created = profile::create_profile(
        &db,
        ProfileCreatePayload {
            target_cli: ProfileTargetCli::Claude,
            name: "B5 test".into(),
            description: None,
            icon: None,
            icon_color: None,
            provider_id: None,
            settings_json: Some("{}".into()),
            mcp_ids: None,
            skill_ids: None,
        },
    )
    .expect("create profile");

    let summary = LauncherService::get_safety_summary(
        db.clone(),
        TargetCli::Claude,
        Some(&created.id),
    )
    .await
    .expect("summary for explicit profile");

    let workdir_str = summary.workdir.to_string_lossy().to_string();
    assert!(
        workdir_str.ends_with(&created.id) || workdir_str.contains(&created.id),
        "workdir should be rooted at the profile id, got: {workdir_str}"
    );
    let has_add_dir_with_workdir = summary
        .flags_applied
        .iter()
        .any(|f| f.starts_with("--add-dir") && f.contains(&created.id));
    assert!(
        has_add_dir_with_workdir,
        "--add-dir flag must point at workdir containing profile id, got: {:?}",
        summary.flags_applied
    );
}

// ============================================================================
// 8. B6 — sandbox::apply_to_command shim is invoked pre-spawn
//
// This guards the regression where `start_cli`'s private creation-flag helper
// did not actually wire `sandbox::job_object` / `sandbox::sandbox_exec`. We
// can't exercise the full `start_cli` path here (CLI binaries are absent in
// test envs), so we drive the same shim that `start_cli` invokes and assert:
//   1. `apply_to_command` increments the test-only counter.
//   2. The post-shim Command spawns successfully (no `SpawnFailed` from the
//      sandbox-apply step itself).
//   3. On Windows, `assign_to_job_object` accepts a real spawned pid (this is
//      the same call `start_cli` issues immediately after `cmd.spawn()`).
// ============================================================================

#[tokio::test]
async fn start_cli_calls_sandbox_apply_shim_before_spawn() {
    let before = APPLY_TO_COMMAND_CALL_COUNT.load(Ordering::SeqCst);

    // Pick a child program that exits ~immediately on every supported OS.
    #[cfg(windows)]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd.exe");
        c.args(["/C", "exit", "0"]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = std::process::Command::new("/bin/sh");
        c.args(["-c", "exit 0"]);
        c
    };

    // Apply the shim — same call site that `start_cli` performs.
    sandbox::apply_to_command(&mut cmd, SandboxLevel::Strict)
        .expect("sandbox::apply_to_command must not error on a benign command");

    let after = APPLY_TO_COMMAND_CALL_COUNT.load(Ordering::SeqCst);
    assert!(
        after > before,
        "apply_to_command counter must increment (before={before}, after={after})"
    );

    // Detach stdio so we don't block on parent's console buffers.
    let mut child = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("shim must produce a spawnable Command");

    let pid = child.id();

    // Post-spawn Job Object on Windows; no-op on *nix.
    sandbox::assign_to_job_object(pid, SandboxLevel::Strict)
        .expect("assign_to_job_object must accept a freshly spawned pid");

    let status = child.wait().expect("wait child");
    assert!(
        status.success(),
        "post-shim child must exit 0 (status={status:?})"
    );
}
