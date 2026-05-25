//! 沙盒模块的 Tauri 命令层。
//!
//! 关键设计：
//! - **L1 / level**: 通过 `tauri::State<AppState>` 拿到 `Arc<Database>` 持久化。
//! - **L2**: 静态硬编码，命令只读暴露；**不存在**任何 setter / mutator。
//! - **audit**: 只暴露读接口；写入由 sandbox 模块内部完成。

#![allow(non_snake_case)]

use crate::sandbox::{self, AuditEntry, AuditQueryOpts, L1Rule, L2RedlineDto, SandboxLevel};
use crate::store::AppState;
use crate::types::OperationResult;
use serde::Serialize;
use tauri::State;

/// L2 命中匹配（前端 DTO）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct L2MatchDto {
    pub id: String,
    pub category: String,
    pub description_key: String,
    pub evidence: String,
}

#[tauri::command]
pub async fn sandbox_get_l1_rules(state: State<'_, AppState>) -> Result<Vec<L1Rule>, String> {
    sandbox::get_l1_rules(&state.db).map_err(Into::into)
}

#[tauri::command]
pub async fn sandbox_set_l1_rule(
    state: State<'_, AppState>,
    rule_id: String,
    enabled: bool,
) -> Result<L1Rule, String> {
    let updated = sandbox::set_l1_rule(&state.db, &rule_id, enabled).map_err(String::from)?;

    // 状态切回 enabled=true 时，把 disableBypassPermissionsMode 写回 settings.json；
    // 关掉时移除。该操作仅对 L1.claude_skip_permissions 有意义。失败仅记录。
    if rule_id == "L1.claude_skip_permissions" {
        let result = if enabled {
            crate::services::settings_injection::restore_bypass_block()
        } else {
            crate::services::settings_injection::remove_bypass_block()
        };
        if let Err(e) = result {
            log::warn!("sandbox_set_l1_rule: bypass-block sync failed: {e}");
        }
    }
    Ok(updated)
}

/// Unlock an L1 rule with the 24h keyword. Returns `OperationResult` on
/// success; the frontend refetches via `sandbox_get_l1_rules` to pick up
/// the updated `unlockedUntil` timestamp on the affected rule.
#[tauri::command]
pub async fn sandbox_unlock_l1_rule(
    state: State<'_, AppState>,
    rule_id: String,
    keyword: String,
) -> Result<OperationResult, String> {
    sandbox::unlock_l1_rule(&state.db, &rule_id, &keyword).map_err(String::from)?;

    // L1.claude_skip_permissions 解锁后，必须把 ~/.claude/settings.json 里的
    // disableBypassPermissionsMode 暂时移除，否则 CLI 自身仍会拒绝 bypass。
    // 失败仅记录日志，避免影响解锁主流程。
    if rule_id == "L1.claude_skip_permissions" {
        if let Err(e) = crate::services::settings_injection::remove_bypass_block() {
            log::warn!("sandbox_unlock_l1_rule: remove_bypass_block failed: {e}");
        }
    }
    Ok(OperationResult::ok())
}

#[tauri::command]
pub async fn sandbox_list_l2_redlines() -> Result<Vec<L2RedlineDto>, String> {
    Ok(sandbox::get_l2_redlines())
}

#[tauri::command]
pub async fn sandbox_check_redline_match(command: String) -> Result<Option<L2MatchDto>, String> {
    Ok(sandbox::check_redline_match(&command).map(|m| L2MatchDto {
        id: m.id.to_string(),
        category: m.category.to_string(),
        description_key: m.description_key.to_string(),
        evidence: m.evidence,
    }))
}

#[tauri::command]
pub async fn sandbox_get_level(state: State<'_, AppState>) -> Result<String, String> {
    sandbox::get_sandbox_level(&state.db)
        .map(|l| l.as_str().to_string())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn sandbox_set_level(
    state: State<'_, AppState>,
    level: String,
) -> Result<OperationResult, String> {
    let parsed = match level.as_str() {
        "strict" => SandboxLevel::Strict,
        "medium" => SandboxLevel::Medium,
        _ => return Err(format!("invalid sandbox level: {level}")),
    };
    sandbox::set_sandbox_level(&state.db, parsed).map_err(String::from)?;
    Ok(OperationResult::ok())
}

#[tauri::command]
pub async fn sandbox_get_audit_log(
    limit: Option<usize>,
    event_type: Option<String>,
) -> Result<Vec<AuditEntry>, String> {
    let event = event_type.and_then(|e| match e.as_str() {
        "l2_block" => Some(sandbox::AuditEventType::L2Block),
        "l1_block" => Some(sandbox::AuditEventType::L1Block),
        "l1_unlock" => Some(sandbox::AuditEventType::L1Unlock),
        "l1_toggle" => Some(sandbox::AuditEventType::L1Toggle),
        "level_change" => Some(sandbox::AuditEventType::LevelChange),
        "sandbox_spawn" => Some(sandbox::AuditEventType::SandboxSpawn),
        _ => None,
    });
    let opts = AuditQueryOpts {
        limit,
        event_type: event,
    };
    Ok(sandbox::get_audit_log(&opts))
}
