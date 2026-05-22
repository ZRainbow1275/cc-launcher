//! macOS sandbox-exec 集成测试 —— 仅在 `target_os = "macos"` 上编译/运行。
//!
//! 验证：
//! 1. `render_profile` 生成的 SBPL 文本是 sandbox-exec 可接受的语法。
//! 2. `apply_sandbox_exec_profile` 生成的 profile 文件能被 `sandbox-exec -n test -f ...`
//!    解析（dry-run 模式：不实际限制本进程）。
//!
//! 注意：`sandbox-exec` 已被 Apple 标 deprecated 但仍可用；如未来 macOS 移除，本测试
//! 需要相应调整。

#![cfg(target_os = "macos")]

use cc_switch_lib::sandbox::sandbox_exec;
use std::process::Command;

#[test]
fn render_profile_syntax_is_accepted_by_sandbox_exec() {
    let cwd = std::env::temp_dir();
    let profile_path = sandbox_exec::apply_sandbox_exec_profile(&cwd).expect("write profile");

    // sandbox-exec 的 dry-run 形式：实际执行 /usr/bin/true（无害），
    // 如果 profile 语法错则会返回 non-zero exit code（通常 65）。
    let output = Command::new("/usr/bin/sandbox-exec")
        .args([
            "-f",
            profile_path.to_str().expect("profile path utf8"),
            "/usr/bin/true",
        ])
        .output()
        .expect("invoke sandbox-exec");

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let _ = std::fs::remove_file(&profile_path);

    assert!(
        output.status.success(),
        "sandbox-exec rejected profile (exit={:?}, stderr={stderr})",
        output.status.code()
    );
}

#[test]
fn render_profile_contains_required_directives() {
    let cwd = std::env::temp_dir();
    let p = sandbox_exec::render_profile(&cwd);

    // 关键指令必须出现
    assert!(p.contains("(version 1)"));
    assert!(p.contains("(deny default)"));
    assert!(p.contains("(allow process-fork)"));
    assert!(p.contains("(allow process-exec)"));
    assert!(p.contains("(allow pseudo-tty)"));

    // 关键保护
    assert!(p.contains("\"/etc/hosts\""));
    assert!(p.contains("LaunchDaemons"));
    assert!(p.contains("\\.cc-switch"));
}
