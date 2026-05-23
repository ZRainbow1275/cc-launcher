//! NDJSON 沙盒审计日志。
//!
//! ## 写入语义
//! - 追加写（O_APPEND），每条事件一行 JSON，行尾 `\n`。
//! - **best-effort**：写入失败时**不**让上游调用方阻塞 —— 仅以 `log::warn!` 上报。
//!   这是因为审计日志失败不应阻断沙盒决策本身（red-line 拦截 / spawn 等）。
//!
//! ## 路径
//! - Windows: `%LOCALAPPDATA%\cc-switch\audit.log`
//! - macOS / Linux: `~/.cc-switch/audit.log`
//!
//! ## 轮转
//! - 当当前 `audit.log` 体积 >= `AUDIT_ROTATE_BYTES`（10MB）时：
//!   1. `audit.log.4` → 丢弃；`audit.log.3 → .4`；`.2 → .3`；`.1 → .2`；`audit.log → .1`
//!   2. 新建空 `audit.log` 继续追加。
//! - 保留最多 5 份历史（`.1` ~ `.5`）。

use crate::config::get_app_config_dir;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// 单文件最大体积 (10MB)
const AUDIT_ROTATE_BYTES: u64 = 10 * 1024 * 1024;
/// 保留的历史份数
const AUDIT_KEEP_HISTORY: usize = 5;

/// 事件类型（NDJSON 中 `event_type` 字段）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    L2Block,
    L1Block,
    L1Unlock,
    L1Toggle,
    LevelChange,
    SandboxSpawn,
}

/// 决策结果。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditDecision {
    Allowed,
    Blocked,
    Prompted,
    UnlockedByExpert,
    Updated,
}

/// 行为者：launcher 自动决策 vs. 用户操作。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditActor {
    Launcher,
    User,
    Cli,
}

/// 单条审计记录（写入 NDJSON 时整行序列化）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// RFC3339 带时区
    pub timestamp: String,
    pub event_type: AuditEventType,
    pub actor: AuditActor,
    pub decision: AuditDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// 是否在 spawn 前后成功应用了 OS 沙盒 shim（B6 引入）。
    /// 仅 `event_type = sandbox_spawn` 时由 launcher_service 写入；其他事件保持 None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_applied: Option<bool>,
}

impl AuditEntry {
    /// 构造一条 L2 拦截记录（最常用入口）。
    pub fn l2_block(rule_id: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            event_type: AuditEventType::L2Block,
            actor: AuditActor::Launcher,
            decision: AuditDecision::Blocked,
            rule_id: Some(rule_id.into()),
            matched_command: Some(truncate(command.into(), 1024)),
            pid: None,
            profile_id: None,
            cwd: None,
            note: None,
            sandbox_applied: None,
        }
    }

    pub fn l1_block(rule_id: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            event_type: AuditEventType::L1Block,
            actor: AuditActor::Launcher,
            decision: AuditDecision::Prompted,
            rule_id: Some(rule_id.into()),
            matched_command: Some(truncate(command.into(), 1024)),
            pid: None,
            profile_id: None,
            cwd: None,
            note: None,
            sandbox_applied: None,
        }
    }

    pub fn l1_unlock(rule_id: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            event_type: AuditEventType::L1Unlock,
            actor: AuditActor::User,
            decision: AuditDecision::UnlockedByExpert,
            rule_id: Some(rule_id.into()),
            matched_command: None,
            pid: None,
            profile_id: None,
            cwd: None,
            note: None,
            sandbox_applied: None,
        }
    }

    pub fn level_change(new_level: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            event_type: AuditEventType::LevelChange,
            actor: AuditActor::User,
            decision: AuditDecision::Updated,
            rule_id: None,
            matched_command: None,
            pid: None,
            profile_id: None,
            cwd: None,
            note: Some(new_level.into()),
            sandbox_applied: None,
        }
    }
}

fn truncate(mut s: String, max: usize) -> String {
    if s.len() <= max {
        return s;
    }
    // 找最近的字符边界，避免在 UTF-8 内截断
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
    s.push('…');
    s
}

/// 计算审计日志文件路径。
/// 复用 cc-switch 现有的 `get_app_config_dir()`：
/// - 默认 `~/.cc-switch/audit.log`
/// - 也可被 `app_store::set_app_config_dir_override` 覆盖（专家模式）
pub fn audit_log_path() -> PathBuf {
    let mut dir = get_app_config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        log::warn!("audit: failed to create config dir {}: {e}", dir.display());
    }
    dir.push("audit.log");
    dir
}

/// 追加一条审计记录（best-effort，永不 panic）。
pub fn append(entry: &AuditEntry) {
    let path = audit_log_path();
    if let Err(e) = append_inner(&path, entry) {
        log::warn!("audit: append to {} failed: {e}", path.display());
    }
}

fn append_inner(path: &Path, entry: &AuditEntry) -> std::io::Result<()> {
    // 先做轮转检测
    if let Ok(meta) = fs::metadata(path) {
        if meta.len() >= AUDIT_ROTATE_BYTES {
            if let Err(e) = rotate(path) {
                log::warn!("audit: rotate failed: {e}");
            }
        }
    }

    let mut line = serde_json::to_string(entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push('\n');

    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(line.as_bytes())?;
    f.flush()?;
    Ok(())
}

/// 轮转：audit.log -> audit.log.1，old .1 -> .2，... 保留 `AUDIT_KEEP_HISTORY` 份。
fn rotate(base: &Path) -> std::io::Result<()> {
    // 删除最老的
    let oldest = with_suffix(base, AUDIT_KEEP_HISTORY);
    if oldest.exists() {
        fs::remove_file(&oldest)?;
    }
    // 依次往后移
    for i in (1..AUDIT_KEEP_HISTORY).rev() {
        let src = with_suffix(base, i);
        let dst = with_suffix(base, i + 1);
        if src.exists() {
            fs::rename(&src, &dst)?;
        }
    }
    // base -> .1
    fs::rename(base, with_suffix(base, 1))?;
    Ok(())
}

fn with_suffix(base: &Path, n: usize) -> PathBuf {
    // audit.log → audit.log.1
    let mut s = base.as_os_str().to_owned();
    s.push(format!(".{n}"));
    PathBuf::from(s)
}

/// 审计查询选项。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditQueryOpts {
    /// 返回最多 N 条（按文件顺序的尾部）。
    pub limit: Option<usize>,
    /// 只返回指定 event_type 的条目。
    pub event_type: Option<AuditEventType>,
}

/// 读取审计日志（**只读当前文件**，不合并历史轮转份）。
pub fn read_entries(opts: &AuditQueryOpts) -> Vec<AuditEntry> {
    let path = audit_log_path();
    if !path.exists() {
        return Vec::new();
    }
    let Ok(f) = std::fs::File::open(&path) else {
        return Vec::new();
    };
    let reader = BufReader::new(f);
    let mut out: Vec<AuditEntry> = Vec::new();
    for line in reader.lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<AuditEntry>(&line) {
            Ok(entry) => {
                if let Some(want) = opts.event_type {
                    if entry.event_type != want {
                        continue;
                    }
                }
                out.push(entry);
            }
            Err(e) => {
                log::debug!("audit: skip malformed line: {e}");
            }
        }
    }
    if let Some(n) = opts.limit {
        if out.len() > n {
            let start = out.len() - n;
            out = out[start..].to_vec();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_within_limit() {
        assert_eq!(truncate("hello".to_string(), 10), "hello");
        let cut = truncate("a".repeat(50), 10);
        assert!(cut.ends_with('…'));
        assert!(cut.chars().count() <= 11); // 10 char + ellipsis
    }

    #[test]
    fn entry_serializes_to_single_line_json() {
        let entry = AuditEntry::l2_block("disk_wipe.rm_root", "rm -rf /");
        let line = serde_json::to_string(&entry).expect("serialize");
        assert!(!line.contains('\n'));
        // event_type must be snake_case
        assert!(line.contains("\"event_type\":\"l2_block\""));
        assert!(line.contains("\"decision\":\"blocked\""));
        assert!(line.contains("\"rule_id\":\"disk_wipe.rm_root\""));
        assert!(line.contains("\"actor\":\"launcher\""));
    }

    #[test]
    fn entry_roundtrips() {
        let entry = AuditEntry::l1_unlock("L1.sudo_runas");
        let s = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.rule_id.as_deref(), Some("L1.sudo_runas"));
        assert_eq!(parsed.event_type, AuditEventType::L1Unlock);
    }
}
