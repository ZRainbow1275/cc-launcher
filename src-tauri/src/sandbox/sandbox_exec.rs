//! macOS sandbox-exec profile 生成。仅在 `target_os = "macos"` 下编译。
//!
//! Profile 模板参考 research/sandbox-pattern.md §5。MVP 走最小覆盖面：
//! - 默认 deny
//! - 允许执行已发现的二进制 + 子进程继承沙盒
//! - 允许 TTY / pseudo-tty（交互式 CLI 必备）
//! - 允许读 /usr /System /Library /private/var/folders
//! - 写入仅限 `cwd` 子树 + temp 目录
//! - 显式拒绝 /etc/hosts、cc-switch 自身目录、/System、/Library/LaunchDaemons

#![cfg(target_os = "macos")]

use crate::sandbox::SandboxError;
use std::path::{Path, PathBuf};

/// 渲染 sandbox-exec profile 内容（SBPL TinyScheme 语法）。
///
/// `cwd` 必须是已规范化（canonical）的绝对路径；调用方负责。
pub fn render_profile(cwd: &Path) -> String {
    // 注意：单引号在 SBPL 字符串字面量中需要正确转义。
    // 我们 escape 反斜杠和双引号 —— 是绝对路径，所以一般只可能含 `"`。
    let cwd_escaped = escape_sbpl_string(&cwd.to_string_lossy());

    format!(
        r#"(version 1)

; cc-launcher MVP sandbox profile (auto-generated)
(deny default)

; 进程相关：允许 fork + exec，子进程继承本沙盒
(allow process-fork)
(allow process-exec)
(allow signal (target same-sandbox))

; TTY / pseudo-tty —— 交互式 CLI 必需
(allow pseudo-tty)
(allow file-read* file-write* file-ioctl (literal "/dev/ptmx"))
(allow file-read* file-write*
  (require-all (regex #"^/dev/ttys[0-9]+") (extension "com.apple.sandbox.pty")))
(allow file-read* file-write* (literal "/dev/null") (literal "/dev/tty"))

; 系统读：系统库 + 框架 + 用户偏好缓存
(allow file-read*
  (subpath "/usr")
  (subpath "/bin")
  (subpath "/sbin")
  (subpath "/System")
  (subpath "/Library")
  (subpath "/private/var/folders")
  (subpath "/private/var/db/timezone")
  (subpath "/private/etc")
  (subpath "{cwd}")
)

; 写入：仅允许工作目录 + temp
(allow file-write*
  (subpath "{cwd}")
  (subpath "/private/var/folders")
  (subpath "/private/tmp")
)

; 显式拒绝：即使在 read/write 白名单内也保护这些路径
(deny file-write*
  (literal "/etc/hosts")
  (literal "/private/etc/hosts")
  (subpath "/Library/LaunchDaemons")
  (subpath "/Library/LaunchAgents")
  (subpath "/System")
  (regex #"^/Users/[^/]+/\.cc-switch")
)

; mach 服务（必须的最小集 —— TUI / I18n 需要 opendirectoryd）
(allow mach-lookup)

; sysctl 读（hw.* / kern.* —— CLI 可能查 CPU/内存）
(allow sysctl-read)

; 网络：MVP 默认放行（CLI 需要拉模型 / 下包）
(allow network*)

; 兼容 file-issue-extension （seatbelt 内部用于 PTY ownership）
(allow file-issue-extension)
"#,
        cwd = cwd_escaped
    )
}

fn escape_sbpl_string(s: &str) -> String {
    // SBPL 字符串字面量用双引号包裹。反斜杠和双引号需要转义。
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out
}

/// 把渲染好的 profile 写到临时文件并返回路径。调用方在 spawn 后清理。
pub fn apply_sandbox_exec_profile(cwd: &Path) -> Result<PathBuf, SandboxError> {
    use std::io::Write;

    let canonical = cwd
        .canonicalize()
        .map_err(|e| SandboxError::Internal(format!("canonicalize cwd: {e}")))?;

    let profile = render_profile(&canonical);

    // 创建唯一文件名，避免多 launcher 实例冲突
    let mut path = std::env::temp_dir();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    path.push(format!("cc-launcher-sb-{nonce}.sb"));

    let mut f = std::fs::File::create(&path)
        .map_err(|e| SandboxError::Internal(format!("create profile file: {e}")))?;
    f.write_all(profile.as_bytes())
        .map_err(|e| SandboxError::Internal(format!("write profile: {e}")))?;
    f.flush()
        .map_err(|e| SandboxError::Internal(format!("flush profile: {e}")))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn render_profile_contains_required_sections() {
        let cwd = PathBuf::from("/Users/foo/cc-launcher-projects/abc");
        let p = render_profile(&cwd);
        assert!(p.contains("(deny default)"));
        assert!(p.contains("(allow pseudo-tty)"));
        assert!(p.contains("(allow process-fork)"));
        assert!(p.contains("(allow process-exec)"));
        assert!(p.contains("\"/etc/hosts\""));
        assert!(p.contains("/Users/foo/cc-launcher-projects/abc"));
    }

    #[test]
    fn escape_handles_quotes_and_backslashes() {
        assert_eq!(escape_sbpl_string("a\"b"), "a\\\"b");
        assert_eq!(escape_sbpl_string("a\\b"), "a\\\\b");
        assert_eq!(escape_sbpl_string("clean"), "clean");
    }

    #[test]
    fn apply_writes_profile_to_temp() {
        // 使用 temp_dir 作为 cwd，保证 canonicalize 通过
        let cwd = std::env::temp_dir();
        let path = apply_sandbox_exec_profile(&cwd).expect("apply profile");
        assert!(path.exists(), "profile file must exist");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("(deny default)"));
        let _ = std::fs::remove_file(&path);
    }
}
