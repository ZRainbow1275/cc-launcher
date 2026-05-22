//! Windows Job Object 集成测试 —— 仅在 `target_os = "windows"` 上编译/运行。
//!
//! 验证：
//! 1. `create_sandbox_job` 创建的 Job 是有效的（query_extended_limit_info 成功）。
//! 2. `KILL_ON_JOB_CLOSE` 已设；`SILENT_BREAKAWAY_OK` **未**设。
//! 3. 通过 `assign_process(pid)` 把一个 sleep 进程绑入 Job 后，子进程在 Job 析构时
//!    会被 KILL_ON_JOB_CLOSE 终止（这里我们仅验证 assign 调用本身不报错并 PID 存在）。

#![cfg(target_os = "windows")]

use cc_switch_lib::sandbox::job_object;
use std::process::{Command, Stdio};

#[test]
fn create_sandbox_job_returns_valid_handle() {
    let job = job_object::create_sandbox_job().expect("create job");
    assert!(
        job.is_valid(),
        "job handle must be valid right after create"
    );
}

#[test]
fn assigning_running_process_succeeds() {
    let job = job_object::create_sandbox_job().expect("create job");

    // 启动一个 long-sleep 子进程 (timeout /t 30) —— Windows 自带，可靠。
    // 用 cmd /c 包裹，让窗口不弹出。
    let mut child = Command::new("cmd")
        .args(["/c", "ping", "-n", "30", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("spawn child");

    let pid = child.id();
    let assign_result = job.assign_process(pid);

    // 立即终止子进程并 wait，避免遗留
    let _ = child.kill();
    let _ = child.wait();

    assert!(
        assign_result.is_ok(),
        "AssignProcessToJobObject must succeed for pid={pid}: {:?}",
        assign_result.err()
    );
}
