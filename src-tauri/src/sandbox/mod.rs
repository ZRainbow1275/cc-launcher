//! # 沙盒模块 (Phase B Task B3)
//!
//! 实现 PRD §D3 "双层禁令" 模型：
//! - **L1 软拦截** (`l1_store`): 可解锁、可临时关闭，存储在 SQLite settings 表。
//! - **L2 硬红线** (`redline`): 编译期 `Lazy<Vec<...>>` 硬编码，**无任何运行时修改 API**。
//!
//! 平台特定的 OS 沙盒分层：
//! - Windows: `job_object` + `restricted_token`
//! - macOS: `sandbox_exec`
//!
//! 所有决策（block / unlock / level change）通过 `audit` 写入 NDJSON 审计日志。
//!
//! ## 安全不变量（编译期 + 设计期）
//! 1. L2 redline list 是 `static REDLINES: Lazy<Vec<L2Redline>>`，**没有** mut setter。
//! 2. 解锁关键词比对 byte-wise 大小写敏感（`l1_store::is_valid_unlock_keyword`）。
//! 3. Windows Job Object: `KILL_ON_JOB_CLOSE = on` / `SILENT_BREAKAWAY_OK = off`。
//! 4. 审计日志 best-effort —— 写失败不阻塞沙盒主决策路径。

pub mod audit;
pub mod l1_store;
pub mod redline;

#[cfg(target_os = "windows")]
pub mod job_object;
#[cfg(target_os = "windows")]
pub mod restricted_token;

#[cfg(target_os = "macos")]
pub mod sandbox_exec;

use crate::database::Database;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub use audit::{AuditDecision, AuditEntry, AuditEventType, AuditQueryOpts};
pub use l1_store::{L1Category, L1Rule, L1Store, UnlockResult};
pub use redline::{L2Category, L2Match, L2Redline};

/// 沙盒级别 —— 与 contracts.ts::SandboxLevel 严格对齐。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SandboxLevel {
    /// 严格 —— 默认；L1 全开 + L2 + OS sandbox 全开。
    #[default]
    Strict,
    /// 中等 —— 允许部分 L1 解锁，但 L2 + OS sandbox 仍然生效。
    Medium,
}

impl SandboxLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            SandboxLevel::Strict => "strict",
            SandboxLevel::Medium => "medium",
        }
    }
}

const SANDBOX_LEVEL_KEY: &str = "sandbox.level.v1";

/// 沙盒模块的错误类型 —— 与 cc-switch 现有 `AppError` 互操作。
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("L1 rule not found: {0}")]
    RuleNotFound(String),
    #[error("L1 rule cannot be unlocked: {0}")]
    RuleNotUnlockable(String),
    #[error("invalid unlock keyword")]
    InvalidUnlockKeyword,
    #[error("storage: {0}")]
    Storage(#[from] AppError),
    #[error("internal: {0}")]
    Internal(String),
    #[cfg(target_os = "windows")]
    #[error("windows API: {0}")]
    WindowsApi(String),
}

impl From<SandboxError> for String {
    fn from(err: SandboxError) -> Self {
        err.to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────
// 公共 API（被 Tauri commands 调用）
// ─────────────────────────────────────────────────────────────────────

/// L1 规则列表 —— 包含临时解锁状态。
pub fn get_l1_rules(db: &Arc<Database>) -> Result<Vec<L1Rule>, SandboxError> {
    L1Store::new(db.clone()).list()
}

/// 切换某条 L1 规则的启用状态。
pub fn set_l1_rule(db: &Arc<Database>, id: &str, enabled: bool) -> Result<L1Rule, SandboxError> {
    let store = L1Store::new(db.clone());
    let updated = store.set_enabled(id, enabled)?;
    audit::append(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_type: AuditEventType::L1Toggle,
        actor: audit::AuditActor::User,
        decision: AuditDecision::Updated,
        rule_id: Some(id.to_string()),
        matched_command: None,
        pid: None,
        profile_id: None,
        cwd: None,
        note: Some(format!("enabled={enabled}")),
    });
    Ok(updated)
}

/// 用 keyword 临时解锁 L1 规则（24h 内有效）。
/// **关键词比对 byte-wise 大小写敏感**，详见 `l1_store::is_valid_unlock_keyword`。
pub fn unlock_l1_rule(
    db: &Arc<Database>,
    id: &str,
    keyword: &str,
) -> Result<UnlockResult, SandboxError> {
    let store = L1Store::new(db.clone());
    let result = store.unlock(id, keyword)?;
    audit::append(&AuditEntry::l1_unlock(id));
    Ok(result)
}

/// L2 红线列表（**只读，编译期固定**）。
pub fn get_l2_redlines() -> Vec<L2RedlineDto> {
    redline::redlines().iter().map(L2RedlineDto::from).collect()
}

/// 对单条命令字符串做 L2 红线检查 —— 命中即记录审计 + 返回 match。
pub fn check_redline_match(command: &str) -> Option<L2Match> {
    let m = redline::check(command);
    if let Some(ref hit) = m {
        audit::append(&AuditEntry::l2_block(
            hit.id.to_string(),
            command.to_string(),
        ));
    }
    m
}

/// 当前沙盒级别。
pub fn get_sandbox_level(db: &Arc<Database>) -> Result<SandboxLevel, SandboxError> {
    let raw = db
        .get_setting(SANDBOX_LEVEL_KEY)
        .map_err(SandboxError::Storage)?;
    Ok(match raw.as_deref() {
        Some("medium") => SandboxLevel::Medium,
        _ => SandboxLevel::Strict, // 默认 strict
    })
}

/// 设置沙盒级别（持久化 + 审计）。
pub fn set_sandbox_level(db: &Arc<Database>, level: SandboxLevel) -> Result<(), SandboxError> {
    db.set_setting(SANDBOX_LEVEL_KEY, level.as_str())
        .map_err(SandboxError::Storage)?;
    audit::append(&AuditEntry::level_change(level.as_str()));
    Ok(())
}

/// 读取审计日志（按 opts 过滤）。
pub fn get_audit_log(opts: &AuditQueryOpts) -> Vec<AuditEntry> {
    audit::read_entries(opts)
}

// ─────────────────────────────────────────────────────────────────────
// 序列化层 —— 前端友好的 DTO
// ─────────────────────────────────────────────────────────────────────

/// L2 红线的前端 DTO（与 contracts.ts::L2Redline 对齐）。
///
/// 字段使用 camelCase 时，请在 Tauri command 层做 #[serde(rename_all="camelCase")]
/// 转换；这里保留 snake_case 与 Rust 风格统一。
#[derive(Debug, Clone, Serialize)]
pub struct L2RedlineDto {
    pub id: String,
    pub category: String,
    pub pattern: String,
    #[serde(rename = "descriptionKey")]
    pub description_key: String,
    #[serde(rename = "matchType")]
    pub match_type: String,
}

impl From<&L2Redline> for L2RedlineDto {
    fn from(r: &L2Redline) -> Self {
        Self {
            id: r.id.to_string(),
            category: r.category.as_str().to_string(),
            pattern: r.pattern_src.to_string(),
            description_key: r.description_key.to_string(),
            match_type: redline::MatchType::Regex.as_str().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_l2_redlines_returns_exactly_sixteen() {
        let list = get_l2_redlines();
        assert_eq!(list.len(), 16);
    }

    #[test]
    fn check_redline_match_returns_match_on_dangerous_command() {
        let m = check_redline_match("rm -rf /");
        assert!(m.is_some());
        assert_eq!(m.unwrap().id, "disk_wipe.rm_root");
    }

    #[test]
    fn check_redline_match_returns_none_on_safe_command() {
        assert!(check_redline_match("ls -la").is_none());
    }

    #[test]
    fn sandbox_level_default_is_strict() {
        assert_eq!(SandboxLevel::default(), SandboxLevel::Strict);
        assert_eq!(SandboxLevel::Strict.as_str(), "strict");
        assert_eq!(SandboxLevel::Medium.as_str(), "medium");
    }

    #[test]
    fn no_public_api_can_mutate_redlines() {
        // 编译期保证：仅有 redline::redlines() 这一个公共入口，返回 &'static [L2Redline]。
        // 这里仅做断言以提醒：如果有人添加 mut 入口，请重新审视安全模型。
        let r1 = redline::redlines();
        let r2 = redline::redlines();
        // 同一 static 引用 → 指针相等
        assert_eq!(r1.as_ptr(), r2.as_ptr());
    }
}
