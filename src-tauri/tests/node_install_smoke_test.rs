// Intentional: serialize tests via std::sync::MutexGuard across `.await`.
// All tests mutate `CC_SWITCH_TEST_HOME`; tokio's default single-threaded test
// runtime makes deadlock impossible here.
#![allow(clippy::await_holding_lock)]

//! Node install smoke test.
//!
//! Gated behind env var `CC_SWITCH_RUN_NODE_INSTALL_SMOKE=1` so CI doesn't
//! actually download Node 20 LTS (~30MB) on every run. Local devs can opt in
//! via `CC_SWITCH_RUN_NODE_INSTALL_SMOKE=1 cargo test`.
//!
//! Pure code paths (detection, path resolution) always run — those don't touch
//! the network or filesystem outside the test tempdir.

use cc_switch_lib::services::installer::node_runtime::{NodeRuntime, NodeStatus};
use cc_switch_lib::services::installer_service::InstallerService;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::tempdir;

/// Serialize tests that mutate `CC_SWITCH_TEST_HOME` — parallel tests would race.
fn env_lock() -> MutexGuard<'static, ()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn live_install_enabled() -> bool {
    std::env::var("CC_SWITCH_RUN_NODE_INSTALL_SMOKE")
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false)
}

#[tokio::test]
async fn detect_with_no_runtime_returns_not_installed() {
    let _g = env_lock();
    let tmp = tempdir().expect("tempdir");
    std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());
    let status: NodeStatus = NodeRuntime::detect().await;
    assert!(!status.installed);
    assert!(status.is_private_runtime);
    assert!(status.version.is_none());
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[tokio::test]
async fn runtime_root_lands_under_test_home() {
    let _g = env_lock();
    let tmp = tempdir().expect("tempdir");
    std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());
    let root = NodeRuntime::runtime_root().expect("runtime_root");
    assert!(root.starts_with(tmp.path()));
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[tokio::test]
async fn node_binary_path_has_platform_correct_suffix() {
    let _g = env_lock();
    let tmp = tempdir().expect("tempdir");
    std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());
    let bin = NodeRuntime::node_binary().expect("node_binary");
    #[cfg(target_os = "windows")]
    assert!(bin.to_string_lossy().ends_with("node.exe"));
    #[cfg(not(target_os = "windows"))]
    assert!(bin.to_string_lossy().ends_with("bin/node"));
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[tokio::test]
async fn installer_service_detect_node_matches_runtime() {
    let _g = env_lock();
    let tmp = tempdir().expect("tempdir");
    std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());
    let svc_status = InstallerService::detect_node().await;
    let direct_status = NodeRuntime::detect().await;
    assert_eq!(svc_status.installed, direct_status.installed);
    assert_eq!(
        svc_status.is_private_runtime,
        direct_status.is_private_runtime
    );
    std::env::remove_var("CC_SWITCH_TEST_HOME");
}

#[tokio::test]
async fn live_install_succeeds_when_enabled() {
    if !live_install_enabled() {
        eprintln!("skipping live Node install (set CC_SWITCH_RUN_NODE_INSTALL_SMOKE=1)");
        return;
    }
    let _g = env_lock();
    let tmp = tempdir().expect("tempdir");
    std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());

    let mut events = Vec::new();
    let result = InstallerService::install_node(|p| events.push(p)).await;
    std::env::remove_var("CC_SWITCH_TEST_HOME");

    assert!(result.is_ok(), "install_node failed: {result:?}");
    let status = result.unwrap();
    assert!(status.installed);
    assert_eq!(status.major_version, Some(20));
    assert!(status.is_private_runtime);
}
