//! 沙盒模块的 Tauri 命令层。
//!
//! 关键设计：
//! - **L1 / level**: 通过 `tauri::State<AppState>` 拿到 `Arc<Database>` 持久化。
//! - **L2**: 静态硬编码，命令只读暴露；**不存在**任何 setter / mutator。
//! - **audit**: 只暴露读接口；写入由 sandbox 模块内部完成。

#![allow(non_snake_case)]

use crate::sandbox::{
    self, AuditEntry, AuditQueryOpts, L1Rule, L2RedlineDto, SandboxLevel, UnlockResult,
};
use crate::store::AppState;
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
    sandbox::set_l1_rule(&state.db, &rule_id, enabled).map_err(Into::into)
}

#[tauri::command]
pub async fn sandbox_unlock_l1_rule(
    state: State<'_, AppState>,
    rule_id: String,
    keyword: String,
) -> Result<UnlockResult, String> {
    sandbox::unlock_l1_rule(&state.db, &rule_id, &keyword).map_err(Into::into)
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
pub async fn sandbox_set_level(state: State<'_, AppState>, level: String) -> Result<bool, String> {
    let parsed = match level.as_str() {
        "strict" => SandboxLevel::Strict,
        "medium" => SandboxLevel::Medium,
        _ => return Err(format!("invalid sandbox level: {level}")),
    };
    sandbox::set_sandbox_level(&state.db, parsed)
        .map(|_| true)
        .map_err(Into::into)
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
