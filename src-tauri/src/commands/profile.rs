//! cc-launcher Profile 命令层（v11+）
//!
//! 将 `services::profile` 暴露给前端，命令名严格匹配 `src/lib/api/mock/profile.ts` 的方法名：
//! - `profile_list` / `profile_get` / `profile_create` / `profile_update` / `profile_delete`
//! - `profile_activate` / `profile_get_active` / `profile_list_all_active`
//! - `profile_list_mcp` / `profile_list_skills`
//!
//! 切换成功后通过 `profile-changed` 事件通知前端失效查询缓存。

use tauri::{AppHandle, Emitter, State};

use crate::services::profile::{
    self, ActivateResult, ActiveProfileMap, Profile, ProfileCreatePayload, ProfileMcpEntry,
    ProfileSkillEntry, ProfileUpdatePayload, TargetCli,
};
use crate::store::AppState;
#[allow(unused_imports)]
use crate::types::{OperationResult, ProfileQueryResult};

/// 包装：将 TargetCli 字符串映射为强类型
fn parse_cli(s: &str) -> Result<TargetCli, String> {
    TargetCli::from_str_strict(s).map_err(|e| e.to_string())
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProfileChangedPayload<'a> {
    cli: &'a str,
    profile_id: &'a str,
    kind: &'a str,
}

/// 最佳努力发射 profile-changed 事件 —— 失败不影响命令成功。
fn emit_profile_changed(app: &AppHandle, cli: &str, profile_id: &str, kind: &str) {
    let _ = app.emit(
        "profile-changed",
        ProfileChangedPayload {
            cli,
            profile_id,
            kind,
        },
    );
}

#[tauri::command]
pub fn profile_list(
    target_cli: String,
    state: State<'_, AppState>,
) -> Result<Vec<Profile>, String> {
    let cli = parse_cli(&target_cli)?;
    profile::list_profiles(&state.db, cli).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_get(
    id: String,
    target_cli: String,
    state: State<'_, AppState>,
) -> Result<Option<Profile>, String> {
    let cli = parse_cli(&target_cli)?;
    match profile::get_profile(&state.db, &id, cli) {
        Ok(p) => Ok(Some(p)),
        Err(profile::ProfileError::NotFound { .. }) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn profile_create(
    payload: ProfileCreatePayload,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Profile, String> {
    let created = profile::create_profile(&state.db, payload).map_err(|e| e.to_string())?;
    emit_profile_changed(&app, created.target_cli.as_str(), &created.id, "created");
    Ok(created)
}

#[tauri::command]
pub fn profile_update(
    id: String,
    target_cli: String,
    payload: ProfileUpdatePayload,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Profile, String> {
    let cli = parse_cli(&target_cli)?;
    let updated =
        profile::update_profile(&state.db, &id, cli, payload).map_err(|e| e.to_string())?;
    emit_profile_changed(&app, cli.as_str(), &updated.id, "updated");
    Ok(updated)
}

#[tauri::command]
pub fn profile_delete(
    id: String,
    target_cli: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<OperationResult, String> {
    let cli = parse_cli(&target_cli)?;
    profile::delete_profile(&state.db, &id, cli).map_err(|e| e.to_string())?;
    emit_profile_changed(&app, cli.as_str(), &id, "deleted");
    Ok(OperationResult::ok())
}

#[tauri::command]
pub fn profile_activate(
    id: String,
    target_cli: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ActivateResult, String> {
    let cli = parse_cli(&target_cli)?;
    let result = profile::activate_profile(&state.db, &id, cli).map_err(|e| e.to_string())?;

    // Phase 4 invalidate_runtime —— 通知前端失效查询缓存（best-effort）。
    if result.success {
        emit_profile_changed(&app, cli.as_str(), &id, "activated");
    }

    Ok(result)
}

#[tauri::command]
pub fn profile_get_active(
    target_cli: String,
    state: State<'_, AppState>,
) -> Result<ProfileQueryResult, String> {
    let cli = parse_cli(&target_cli)?;
    let profile = profile::get_active_profile(&state.db, cli).map_err(|e| e.to_string())?;
    Ok(ProfileQueryResult::new(profile))
}

#[tauri::command]
pub fn profile_list_all_active(state: State<'_, AppState>) -> Result<ActiveProfileMap, String> {
    profile::list_all_active(&state.db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_list_mcp(
    id: String,
    target_cli: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProfileMcpEntry>, String> {
    let cli = parse_cli(&target_cli)?;
    profile::list_mcp_for_profile(&state.db, &id, cli).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_list_skills(
    id: String,
    target_cli: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProfileSkillEntry>, String> {
    let cli = parse_cli(&target_cli)?;
    profile::list_skills_for_profile(&state.db, &id, cli).map_err(|e| e.to_string())
}
