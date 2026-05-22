//! cc-launcher Profile 服务（v11+）
//!
//! 实现 Per-CLI Profile 模型的完整 CRUD + 原子激活切换。
//! 设计文档：`.trellis/tasks/05-21-cc-launcher-mvp/research/profile-model.md`
//!
//! ## 核心契约（与 Phase A frontend mock 对齐）
//!
//! - `list_profiles` / `get_profile` / `create_profile` / `update_profile` / `delete_profile`
//! - `activate_profile` ：5 阶段原子切换（quiesce → backup → write_live → invalidate → release）
//! - `get_active_profile` / `list_all_active`
//! - `list_mcp_for_profile` / `list_skills_for_profile`
//!
//! Rust 端字段命名严格匹配 `src/lib/api/contracts.ts` 中的 zod schema（snake_case）。

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Utc;
use once_cell::sync::Lazy;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{atomic_write, get_claude_settings_path, get_home_dir};
use crate::database::Database;
use crate::error::AppError;

// ============================================================================
// Public types — 1:1 对齐前端 contracts.ts
// ============================================================================

/// 目标 CLI 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetCli {
    Claude,
    Codex,
}

impl TargetCli {
    pub fn as_str(self) -> &'static str {
        match self {
            TargetCli::Claude => "claude",
            TargetCli::Codex => "codex",
        }
    }

    pub fn from_str_strict(s: &str) -> Result<Self, AppError> {
        match s {
            "claude" => Ok(TargetCli::Claude),
            "codex" => Ok(TargetCli::Codex),
            other => Err(AppError::InvalidInput(format!(
                "未支持的 target_cli: {other}（cc-launcher MVP 仅支持 claude/codex）"
            ))),
        }
    }
}

/// Profile 主体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub target_cli: TargetCli,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_color: Option<String>,
    /// 关联的 Provider ID；前端 contract 允许 nullable（`z.string().nullable()`）
    pub provider_id: Option<String>,
    pub settings_json: String,
    pub sort_index: i64,
    pub is_builtin: bool,
    pub mcp_ids: Vec<String>,
    pub skill_ids: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Profile MCP 关联条目（用于 `list_mcp_for_profile`）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMcpEntry {
    pub profile_id: String,
    pub target_cli: TargetCli,
    pub mcp_id: String,
    pub sort_index: i64,
}

/// Profile Skill 关联条目（用于 `list_skills_for_profile`）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSkillEntry {
    pub profile_id: String,
    pub target_cli: TargetCli,
    pub skill_id: String,
    pub sort_index: i64,
}

/// 创建 Profile 的入参
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileCreatePayload {
    pub target_cli: TargetCli,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub icon_color: Option<String>,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub settings_json: Option<String>,
    #[serde(default)]
    pub mcp_ids: Option<Vec<String>>,
    #[serde(default)]
    pub skill_ids: Option<Vec<String>>,
}

/// 更新 Profile 的入参（所有字段可选）
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProfileUpdatePayload {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub icon_color: Option<String>,
    /// 使用 `Option<Option<String>>` 区分"未传"和"显式设为 null"两种语义
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "double_option"
    )]
    pub provider_id: Option<Option<String>>,
    #[serde(default)]
    pub settings_json: Option<String>,
    #[serde(default)]
    pub sort_index: Option<i64>,
    #[serde(default)]
    pub mcp_ids: Option<Vec<String>>,
    #[serde(default)]
    pub skill_ids: Option<Vec<String>>,
}

mod double_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    #[allow(dead_code)]
    pub fn serialize<S, T>(value: &Option<Option<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        value.serialize(serializer)
    }
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::deserialize(deserializer).map(Some)
    }
}

/// 切换 Profile 的结果（对应 contracts.ts::SwitchResult）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateResult {
    pub success: bool,
    pub profile_id: String,
    pub target_cli: TargetCli,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TypedError>,
    pub switched_at: String,
}

/// 与 contracts.ts::TypedError 对齐
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedError {
    pub code: String,
    pub message: LocalizedString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedString {
    pub zh: String,
    pub en: String,
    pub ja: String,
}

/// 所有 CLI 的当前激活映射（contracts.ts::ActiveProfileMap）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActiveProfileMap {
    pub claude: Option<String>,
    pub codex: Option<String>,
}

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("Profile 未找到: id={id}, target_cli={target_cli}")]
    NotFound { id: String, target_cli: String },
    #[error("内建 Profile 不可删除: {id}")]
    BuiltinProtected { id: String },
    #[error("Profile 正被激活中，不可删除: {id}")]
    ActiveProfile { id: String },
    #[error("数据库错误: {0}")]
    Database(String),
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("输入校验失败: {0}")]
    InvalidInput(String),
    #[error("Profile 切换失败 (阶段: {phase}): {cause}")]
    SwitchFailed { phase: String, cause: String },
}

impl From<AppError> for ProfileError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Database(s) => ProfileError::Database(s),
            AppError::InvalidInput(s) => ProfileError::InvalidInput(s),
            AppError::Io { path, source } => ProfileError::Io(format!("{path}: {source}")),
            AppError::IoContext { context, source } => {
                ProfileError::Io(format!("{context}: {source}"))
            }
            other => ProfileError::Database(other.to_string()),
        }
    }
}

impl From<rusqlite::Error> for ProfileError {
    fn from(err: rusqlite::Error) -> Self {
        ProfileError::Database(err.to_string())
    }
}

impl From<ProfileError> for String {
    fn from(err: ProfileError) -> Self {
        err.to_string()
    }
}

impl serde::Serialize for ProfileError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// ============================================================================
// 进程级 mutex —— 防止并发切换破坏 5 阶段算法的原子性
// ============================================================================

static SWITCH_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

// ============================================================================
// 路径帮助函数
// ============================================================================

/// 取 cc-switch 备份根目录 `~/.cc-switch/backups/profile-switch-<ts>/`
fn profile_backup_dir(timestamp_ms: i64) -> PathBuf {
    let mut p = get_home_dir();
    p.push(".cc-switch");
    p.push("backups");
    p.push(format!("profile-switch-{timestamp_ms}"));
    p
}

/// 取目标 CLI 的 live 配置文件路径
fn live_config_path(cli: TargetCli) -> PathBuf {
    match cli {
        TargetCli::Claude => get_claude_settings_path(),
        TargetCli::Codex => {
            // 用 codex_config helper（如果存在），否则用默认路径
            crate::codex_config::get_codex_config_path()
        }
    }
}

// ============================================================================
// 服务函数
// ============================================================================

/// 列出指定 CLI 的所有 Profile（按 sort_index, created_at 排序）
pub fn list_profiles(db: &Database, target_cli: TargetCli) -> Result<Vec<Profile>, ProfileError> {
    let conn = db_lock(db)?;
    list_profiles_on_conn(&conn, target_cli)
}

fn list_profiles_on_conn(
    conn: &Connection,
    target_cli: TargetCli,
) -> Result<Vec<Profile>, ProfileError> {
    let mut stmt = conn.prepare(
        "SELECT id, target_cli, name, description, icon, icon_color, provider_id,
                settings_json, sort_index, is_builtin, created_at, updated_at
         FROM profiles
         WHERE target_cli = ?1
         ORDER BY sort_index ASC, created_at ASC",
    )?;

    let rows = stmt.query_map(params![target_cli.as_str()], |row| {
        Ok(ProfileRow {
            id: row.get(0)?,
            target_cli: row.get::<_, String>(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            icon: row.get(4)?,
            icon_color: row.get(5)?,
            provider_id: row.get(6)?,
            settings_json: row.get(7)?,
            sort_index: row.get(8)?,
            is_builtin: row.get::<_, i64>(9)? != 0,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;

    let mut profiles = Vec::new();
    for row in rows {
        let r = row?;
        let mcp_ids = load_mcp_ids(conn, &r.id, target_cli)?;
        let skill_ids = load_skill_ids(conn, &r.id, target_cli)?;
        profiles.push(r.into_profile(mcp_ids, skill_ids)?);
    }
    Ok(profiles)
}

/// 获取单个 Profile（完整内容含 mcp_ids/skill_ids）
pub fn get_profile(
    db: &Database,
    id: &str,
    target_cli: TargetCli,
) -> Result<Profile, ProfileError> {
    let conn = db_lock(db)?;
    get_profile_on_conn(&conn, id, target_cli)
}

fn get_profile_on_conn(
    conn: &Connection,
    id: &str,
    target_cli: TargetCli,
) -> Result<Profile, ProfileError> {
    let row_opt = conn
        .query_row(
            "SELECT id, target_cli, name, description, icon, icon_color, provider_id,
                    settings_json, sort_index, is_builtin, created_at, updated_at
             FROM profiles
             WHERE id = ?1 AND target_cli = ?2",
            params![id, target_cli.as_str()],
            |row| {
                Ok(ProfileRow {
                    id: row.get(0)?,
                    target_cli: row.get::<_, String>(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    icon: row.get(4)?,
                    icon_color: row.get(5)?,
                    provider_id: row.get(6)?,
                    settings_json: row.get(7)?,
                    sort_index: row.get(8)?,
                    is_builtin: row.get::<_, i64>(9)? != 0,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .optional()?;

    let row = row_opt.ok_or(ProfileError::NotFound {
        id: id.to_string(),
        target_cli: target_cli.as_str().to_string(),
    })?;

    let mcp_ids = load_mcp_ids(conn, &row.id, target_cli)?;
    let skill_ids = load_skill_ids(conn, &row.id, target_cli)?;
    row.into_profile(mcp_ids, skill_ids)
}

/// 创建新 Profile
pub fn create_profile(
    db: &Database,
    payload: ProfileCreatePayload,
) -> Result<Profile, ProfileError> {
    if payload.name.trim().is_empty() {
        return Err(ProfileError::InvalidInput("name 不可为空".to_string()));
    }

    let mut conn = db_lock(db)?;
    let target_cli = payload.target_cli;

    // 计算 sort_index：当前 CLI 下 max(sort_index)+1
    let next_sort: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_index), -1) + 1 FROM profiles WHERE target_cli = ?1",
            params![target_cli.as_str()],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let id = format!("{}-{}", target_cli.as_str(), short_uuid());
    let now_ms = Utc::now().timestamp_millis();
    let settings_json = payload.settings_json.unwrap_or_else(|| "{}".to_string());

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO profiles
         (id, target_cli, name, description, icon, icon_color, provider_id,
          settings_json, sort_index, is_builtin, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?10)",
        params![
            &id,
            target_cli.as_str(),
            &payload.name,
            payload.description.as_deref(),
            payload.icon.as_deref(),
            payload.icon_color.as_deref(),
            payload.provider_id.as_deref(),
            &settings_json,
            next_sort,
            now_ms,
        ],
    )?;

    if let Some(mcp_ids) = payload.mcp_ids.as_ref() {
        replace_mcp_associations(&tx, &id, target_cli, mcp_ids)?;
    }
    if let Some(skill_ids) = payload.skill_ids.as_ref() {
        replace_skill_associations(&tx, &id, target_cli, skill_ids)?;
    }

    tx.commit()?;
    drop(conn);

    get_profile(db, &id, target_cli)
}

/// 更新 Profile（部分字段；mcp_ids/skill_ids 若传入则整体覆盖）
pub fn update_profile(
    db: &Database,
    id: &str,
    target_cli: TargetCli,
    payload: ProfileUpdatePayload,
) -> Result<Profile, ProfileError> {
    let mut conn = db_lock(db)?;
    let now_ms = Utc::now().timestamp_millis();

    // 先确认存在
    let existing: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM profiles WHERE id = ?1 AND target_cli = ?2",
            params![id, target_cli.as_str()],
            |r| r.get(0),
        )
        .optional()?;
    if existing.is_none() {
        return Err(ProfileError::NotFound {
            id: id.to_string(),
            target_cli: target_cli.as_str().to_string(),
        });
    }

    let tx = conn.transaction()?;

    // 动态构造 UPDATE
    let mut set_clauses: Vec<&'static str> = Vec::new();
    let mut sql_params: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(name) = payload.name.as_ref() {
        if name.trim().is_empty() {
            return Err(ProfileError::InvalidInput("name 不可为空".to_string()));
        }
        set_clauses.push("name = ?");
        sql_params.push(rusqlite::types::Value::Text(name.clone()));
    }
    if let Some(desc) = payload.description.as_ref() {
        set_clauses.push("description = ?");
        sql_params.push(rusqlite::types::Value::Text(desc.clone()));
    }
    if let Some(icon) = payload.icon.as_ref() {
        set_clauses.push("icon = ?");
        sql_params.push(rusqlite::types::Value::Text(icon.clone()));
    }
    if let Some(icon_color) = payload.icon_color.as_ref() {
        set_clauses.push("icon_color = ?");
        sql_params.push(rusqlite::types::Value::Text(icon_color.clone()));
    }
    if let Some(provider_opt) = payload.provider_id.as_ref() {
        set_clauses.push("provider_id = ?");
        match provider_opt {
            Some(pid) => sql_params.push(rusqlite::types::Value::Text(pid.clone())),
            None => sql_params.push(rusqlite::types::Value::Null),
        }
    }
    if let Some(settings_json) = payload.settings_json.as_ref() {
        set_clauses.push("settings_json = ?");
        sql_params.push(rusqlite::types::Value::Text(settings_json.clone()));
    }
    if let Some(sort_index) = payload.sort_index {
        set_clauses.push("sort_index = ?");
        sql_params.push(rusqlite::types::Value::Integer(sort_index));
    }

    // updated_at 总是刷新
    set_clauses.push("updated_at = ?");
    sql_params.push(rusqlite::types::Value::Integer(now_ms));

    if !set_clauses.is_empty() {
        let sql = format!(
            "UPDATE profiles SET {} WHERE id = ? AND target_cli = ?",
            set_clauses.join(", ")
        );
        sql_params.push(rusqlite::types::Value::Text(id.to_string()));
        sql_params.push(rusqlite::types::Value::Text(
            target_cli.as_str().to_string(),
        ));

        let refs: Vec<&dyn rusqlite::ToSql> = sql_params
            .iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();
        tx.execute(&sql, refs.as_slice())?;
    }

    if let Some(mcp_ids) = payload.mcp_ids.as_ref() {
        replace_mcp_associations(&tx, id, target_cli, mcp_ids)?;
    }
    if let Some(skill_ids) = payload.skill_ids.as_ref() {
        replace_skill_associations(&tx, id, target_cli, skill_ids)?;
    }

    tx.commit()?;
    drop(conn);

    get_profile(db, id, target_cli)
}

/// 删除 Profile（拒绝 builtin + 拒绝 active）
pub fn delete_profile(db: &Database, id: &str, target_cli: TargetCli) -> Result<(), ProfileError> {
    let conn = db_lock(db)?;

    // 1. 存在性 + builtin 校验
    let row_opt: Option<(bool,)> = conn
        .query_row(
            "SELECT is_builtin FROM profiles WHERE id = ?1 AND target_cli = ?2",
            params![id, target_cli.as_str()],
            |r| {
                let b: i64 = r.get(0)?;
                Ok((b != 0,))
            },
        )
        .optional()?;
    let (is_builtin,) = row_opt.ok_or(ProfileError::NotFound {
        id: id.to_string(),
        target_cli: target_cli.as_str().to_string(),
    })?;
    if is_builtin {
        return Err(ProfileError::BuiltinProtected { id: id.to_string() });
    }

    // 2. active 校验
    let active_id: Option<String> = conn
        .query_row(
            "SELECT active_profile_id FROM cli_state WHERE target_cli = ?1",
            params![target_cli.as_str()],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    if active_id.as_deref() == Some(id) {
        return Err(ProfileError::ActiveProfile { id: id.to_string() });
    }

    conn.execute(
        "DELETE FROM profiles WHERE id = ?1 AND target_cli = ?2",
        params![id, target_cli.as_str()],
    )?;
    Ok(())
}

/// 激活 Profile —— 5 阶段原子切换
///
/// 阶段：
/// 1. **quiesce** ：在 `cli_state.transitioning_to` 写入目标 id（标识"正在切换"）
/// 2. **backup** ：把当前 live 配置文件复制到 `~/.cc-switch/backups/profile-switch-<ts>/`
/// 3. **write_live_config** ：原子写新 live 配置（atomic_write）
/// 4. **invalidate_runtime** ：日志记录（实际事件发射由 command 层做，避免 service 依赖 AppHandle）
/// 5. **release** ：DB 事务设置 `active_profile_id = new_id, transitioning_to = NULL`
///
/// 任一阶段失败 → 还原 live 配置文件 + 清空 transitioning_to + 返回 SwitchFailed。
pub fn activate_profile(
    db: &Database,
    id: &str,
    target_cli: TargetCli,
) -> Result<ActivateResult, ProfileError> {
    // 进程级 mutex：避免并发切换
    let _guard = SWITCH_MUTEX
        .lock()
        .map_err(|e| ProfileError::Database(format!("切换 mutex 中毒: {e}")))?;

    // 提前确认 profile 存在
    let profile = get_profile(db, id, target_cli)?;
    let now_ms = Utc::now().timestamp_millis();
    let now_iso = Utc::now().to_rfc3339();
    let backup_dir = profile_backup_dir(now_ms);
    let live_path = live_config_path(target_cli);

    // === Phase 1: quiesce ===
    {
        let conn = db_lock(db)?;
        conn.execute(
            "UPDATE cli_state SET transitioning_to = ?1 WHERE target_cli = ?2",
            params![id, target_cli.as_str()],
        )
        .map_err(|e| ProfileError::SwitchFailed {
            phase: "quiesce".into(),
            cause: e.to_string(),
        })?;
    }

    // === Phase 2: backup（仅当 live 文件存在时） ===
    let backup_path: Option<PathBuf> = if live_path.exists() {
        std::fs::create_dir_all(&backup_dir).map_err(|e| ProfileError::SwitchFailed {
            phase: "backup".into(),
            cause: format!("创建备份目录失败: {e}"),
        })?;
        let backup_file = backup_dir.join(format!(
            "{}-{}",
            target_cli.as_str(),
            live_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "live.cfg".to_string())
        ));
        match std::fs::copy(&live_path, &backup_file) {
            Ok(_) => Some(backup_file),
            Err(e) => {
                // 备份失败 → 撤销 quiesce
                let _ = clear_transition_marker(db, target_cli);
                return Err(ProfileError::SwitchFailed {
                    phase: "backup".into(),
                    cause: format!("复制 live 文件到备份失败: {e}"),
                });
            }
        }
    } else {
        None
    };

    // === Phase 3: write_live_config ===
    let new_config = render_live_config(&profile);
    if let Err(e) = atomic_write(&live_path, new_config.as_bytes()) {
        // 回滚 quiesce
        let _ = clear_transition_marker(db, target_cli);
        // 还原 backup（如果有）
        if let Some(bp) = backup_path.as_ref() {
            let _ = std::fs::copy(bp, &live_path);
        }
        return Err(ProfileError::SwitchFailed {
            phase: "write_live_config".into(),
            cause: e.to_string(),
        });
    }

    // === Phase 4: invalidate_runtime（日志埋点；事件由 command 层发） ===
    log::info!(
        "Profile activated (cli={}, id={}, live_path={})",
        target_cli.as_str(),
        id,
        live_path.display()
    );

    // === Phase 5: release（DB 事务）===
    {
        let mut conn = db_lock(db)?;
        let tx = conn.transaction()?;
        if let Err(e) = tx.execute(
            "UPDATE cli_state
             SET active_profile_id = ?1,
                 transitioning_to = NULL,
                 last_switched_at = ?2
             WHERE target_cli = ?3",
            params![id, now_ms, target_cli.as_str()],
        ) {
            // 还原 backup
            drop(tx);
            drop(conn);
            let _ = clear_transition_marker(db, target_cli);
            if let Some(bp) = backup_path.as_ref() {
                let _ = std::fs::copy(bp, &live_path);
            }
            return Err(ProfileError::SwitchFailed {
                phase: "release".into(),
                cause: e.to_string(),
            });
        }
        tx.commit().map_err(|e| ProfileError::SwitchFailed {
            phase: "release".into(),
            cause: format!("commit 失败: {e}"),
        })?;
    }

    Ok(ActivateResult {
        success: true,
        profile_id: id.to_string(),
        target_cli,
        backup_dir: backup_path
            .as_ref()
            .map(|_| backup_dir.to_string_lossy().to_string()),
        error: None,
        switched_at: now_iso,
    })
}

fn clear_transition_marker(db: &Database, target_cli: TargetCli) -> Result<(), ProfileError> {
    let conn = db_lock(db)?;
    conn.execute(
        "UPDATE cli_state SET transitioning_to = NULL WHERE target_cli = ?1",
        params![target_cli.as_str()],
    )?;
    Ok(())
}

/// 获取指定 CLI 当前激活的 Profile（若有）
pub fn get_active_profile(
    db: &Database,
    target_cli: TargetCli,
) -> Result<Option<Profile>, ProfileError> {
    let active_id: Option<String> = {
        let conn = db_lock(db)?;
        conn.query_row(
            "SELECT active_profile_id FROM cli_state WHERE target_cli = ?1",
            params![target_cli.as_str()],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
    };
    match active_id {
        Some(id) => Ok(Some(get_profile(db, &id, target_cli)?)),
        None => Ok(None),
    }
}

/// 返回 `{ claude: Option<id>, codex: Option<id> }`（与 mock 一致）
pub fn list_all_active(db: &Database) -> Result<ActiveProfileMap, ProfileError> {
    let conn = db_lock(db)?;
    let mut map = ActiveProfileMap::default();

    let mut stmt = conn.prepare(
        "SELECT target_cli, active_profile_id FROM cli_state
         WHERE target_cli IN ('claude','codex')",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    for r in rows {
        let (cli, id) = r?;
        match cli.as_str() {
            "claude" => map.claude = id,
            "codex" => map.codex = id,
            _ => {}
        }
    }
    Ok(map)
}

/// 列出 Profile 关联的 MCP 条目
pub fn list_mcp_for_profile(
    db: &Database,
    id: &str,
    target_cli: TargetCli,
) -> Result<Vec<ProfileMcpEntry>, ProfileError> {
    let conn = db_lock(db)?;
    let mut stmt = conn.prepare(
        "SELECT profile_id, target_cli, mcp_id, sort_index
         FROM profile_mcp
         WHERE profile_id = ?1 AND target_cli = ?2
         ORDER BY sort_index ASC, mcp_id ASC",
    )?;
    let rows = stmt.query_map(params![id, target_cli.as_str()], |row| {
        Ok(ProfileMcpEntry {
            profile_id: row.get(0)?,
            target_cli: TargetCli::from_str_strict(&row.get::<_, String>(1)?)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.to_string().into()))?,
            mcp_id: row.get(2)?,
            sort_index: row.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// 读取指定 CLI 的 `transitioning_to` 标记（测试/诊断用）。
/// 若返回 `Some(id)` 表示当前正处于切换中（之前的切换可能中断未释放）。
pub fn get_transition_marker(
    db: &Database,
    target_cli: TargetCli,
) -> Result<Option<String>, ProfileError> {
    let conn = db_lock(db)?;
    let value: Option<Option<String>> = conn
        .query_row(
            "SELECT transitioning_to FROM cli_state WHERE target_cli = ?1",
            params![target_cli.as_str()],
            |r| r.get(0),
        )
        .optional()?;
    Ok(value.flatten())
}

/// 列出 Profile 关联的 Skill 条目
pub fn list_skills_for_profile(
    db: &Database,
    id: &str,
    target_cli: TargetCli,
) -> Result<Vec<ProfileSkillEntry>, ProfileError> {
    let conn = db_lock(db)?;
    let mut stmt = conn.prepare(
        "SELECT profile_id, target_cli, skill_id, sort_index
         FROM profile_skill
         WHERE profile_id = ?1 AND target_cli = ?2
         ORDER BY sort_index ASC, skill_id ASC",
    )?;
    let rows = stmt.query_map(params![id, target_cli.as_str()], |row| {
        Ok(ProfileSkillEntry {
            profile_id: row.get(0)?,
            target_cli: TargetCli::from_str_strict(&row.get::<_, String>(1)?)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.to_string().into()))?,
            skill_id: row.get(2)?,
            sort_index: row.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

// ============================================================================
// 内部辅助
// ============================================================================

#[derive(Debug)]
struct ProfileRow {
    id: String,
    target_cli: String,
    name: String,
    description: Option<String>,
    icon: Option<String>,
    icon_color: Option<String>,
    provider_id: Option<String>,
    settings_json: String,
    sort_index: i64,
    is_builtin: bool,
    created_at: i64,
    updated_at: i64,
}

impl ProfileRow {
    fn into_profile(
        self,
        mcp_ids: Vec<String>,
        skill_ids: Vec<String>,
    ) -> Result<Profile, ProfileError> {
        Ok(Profile {
            id: self.id,
            target_cli: TargetCli::from_str_strict(&self.target_cli)?,
            name: self.name,
            description: self.description,
            icon: self.icon,
            icon_color: self.icon_color,
            provider_id: self.provider_id,
            settings_json: self.settings_json,
            sort_index: self.sort_index,
            is_builtin: self.is_builtin,
            mcp_ids,
            skill_ids,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

fn load_mcp_ids(
    conn: &Connection,
    profile_id: &str,
    target_cli: TargetCli,
) -> Result<Vec<String>, ProfileError> {
    let mut stmt = conn.prepare(
        "SELECT mcp_id FROM profile_mcp
         WHERE profile_id = ?1 AND target_cli = ?2
         ORDER BY sort_index ASC, mcp_id ASC",
    )?;
    let rows = stmt.query_map(params![profile_id, target_cli.as_str()], |r| {
        r.get::<_, String>(0)
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn load_skill_ids(
    conn: &Connection,
    profile_id: &str,
    target_cli: TargetCli,
) -> Result<Vec<String>, ProfileError> {
    let mut stmt = conn.prepare(
        "SELECT skill_id FROM profile_skill
         WHERE profile_id = ?1 AND target_cli = ?2
         ORDER BY sort_index ASC, skill_id ASC",
    )?;
    let rows = stmt.query_map(params![profile_id, target_cli.as_str()], |r| {
        r.get::<_, String>(0)
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn replace_mcp_associations(
    tx: &rusqlite::Transaction<'_>,
    profile_id: &str,
    target_cli: TargetCli,
    mcp_ids: &[String],
) -> Result<(), ProfileError> {
    tx.execute(
        "DELETE FROM profile_mcp WHERE profile_id = ?1 AND target_cli = ?2",
        params![profile_id, target_cli.as_str()],
    )?;
    for (idx, mcp_id) in mcp_ids.iter().enumerate() {
        tx.execute(
            "INSERT OR IGNORE INTO profile_mcp
             (profile_id, target_cli, mcp_id, sort_index)
             VALUES (?1, ?2, ?3, ?4)",
            params![profile_id, target_cli.as_str(), mcp_id, idx as i64],
        )?;
    }
    Ok(())
}

fn replace_skill_associations(
    tx: &rusqlite::Transaction<'_>,
    profile_id: &str,
    target_cli: TargetCli,
    skill_ids: &[String],
) -> Result<(), ProfileError> {
    tx.execute(
        "DELETE FROM profile_skill WHERE profile_id = ?1 AND target_cli = ?2",
        params![profile_id, target_cli.as_str()],
    )?;
    for (idx, skill_id) in skill_ids.iter().enumerate() {
        tx.execute(
            "INSERT OR IGNORE INTO profile_skill
             (profile_id, target_cli, skill_id, sort_index)
             VALUES (?1, ?2, ?3, ?4)",
            params![profile_id, target_cli.as_str(), skill_id, idx as i64],
        )?;
    }
    Ok(())
}

/// 渲染 Profile 到 live 配置文件内容。
///
/// MVP 实现：以 settings_json 为基础，叠加 cc-launcher 元数据。
/// 真实的 CLI 配置生成（包含 MCP/Skills 等的完整 schema）由 D-CLI 决策中描述的 task 实现，
/// 此处仅写入一个安全且可识别的快照，保证文件存在且具有可解析的 JSON 结构。
fn render_live_config(profile: &Profile) -> String {
    // 解析 settings_json，失败则回退到空对象
    let mut value: serde_json::Value = serde_json::from_str(&profile.settings_json)
        .unwrap_or_else(|_| serde_json::Value::Object(Default::default()));

    let metadata = serde_json::json!({
        "profile_id": profile.id,
        "profile_name": profile.name,
        "target_cli": profile.target_cli.as_str(),
        "provider_id": profile.provider_id,
        "mcp_ids": profile.mcp_ids,
        "skill_ids": profile.skill_ids,
        "applied_at": Utc::now().to_rfc3339(),
    });

    if let serde_json::Value::Object(ref mut map) = value {
        map.insert("__cc_launcher_profile".to_string(), metadata);
    } else {
        // 非对象的 settings_json 包装为对象
        value = serde_json::json!({
            "settings": value,
            "__cc_launcher_profile": metadata,
        });
    }

    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
}

fn short_uuid() -> String {
    let id = Uuid::new_v4().simple().to_string();
    id.chars().take(8).collect()
}

/// 获取 db 锁的便捷封装（隐藏 Mutex poison 错误）
fn db_lock(db: &Database) -> Result<std::sync::MutexGuard<'_, Connection>, ProfileError> {
    db.conn
        .lock()
        .map_err(|e| ProfileError::Database(format!("Mutex 锁定失败: {e}")))
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Database {
        let db = Database::memory().expect("memory db");
        // 应用 schema 迁移以确保 v11 表已创建
        let conn = db.conn.lock().expect("lock");
        Database::apply_schema_migrations_on_conn(&conn).expect("migrate to v11");
        drop(conn);
        db
    }

    #[test]
    fn create_then_list_profile() {
        let db = memory_db();
        let created = create_profile(
            &db,
            ProfileCreatePayload {
                target_cli: TargetCli::Claude,
                name: "测试 Profile".into(),
                description: Some("test".into()),
                icon: Some("Sparkles".into()),
                icon_color: Some("#fff".into()),
                provider_id: None,
                settings_json: Some(r#"{"model":"claude-sonnet"}"#.into()),
                mcp_ids: Some(vec![]),
                skill_ids: Some(vec![]),
            },
        )
        .expect("create");
        assert_eq!(created.name, "测试 Profile");
        assert!(!created.is_builtin);

        let list = list_profiles(&db, TargetCli::Claude).expect("list");
        // v11 backfill creates default-claude builtin profile + the just-created one
        assert!(list.iter().any(|p| p.id == created.id));
        assert!(list.iter().any(|p| p.is_builtin));
    }

    #[test]
    fn get_profile_not_found_returns_err() {
        let db = memory_db();
        let err = get_profile(&db, "nonexistent", TargetCli::Claude).unwrap_err();
        assert!(matches!(err, ProfileError::NotFound { .. }));
    }

    #[test]
    fn update_profile_partial_fields() {
        let db = memory_db();
        let p = create_profile(
            &db,
            ProfileCreatePayload {
                target_cli: TargetCli::Codex,
                name: "原始".into(),
                description: None,
                icon: None,
                icon_color: None,
                provider_id: None,
                settings_json: None,
                mcp_ids: None,
                skill_ids: None,
            },
        )
        .expect("create");

        let updated = update_profile(
            &db,
            &p.id,
            TargetCli::Codex,
            ProfileUpdatePayload {
                name: Some("改名后".into()),
                description: Some("desc".into()),
                ..Default::default()
            },
        )
        .expect("update");

        assert_eq!(updated.name, "改名后");
        assert_eq!(updated.description.as_deref(), Some("desc"));
        // 更新时间已变化
        assert!(updated.updated_at >= p.updated_at);
    }

    #[test]
    fn update_profile_provider_id_explicit_null() {
        let db = memory_db();
        // 先种入一个真实 provider，使 FK 通过
        {
            let conn = db.conn.lock().expect("lock");
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, meta) \
                 VALUES (?1, 'claude', 'seed', '{}', '{}')",
                params!["some-pid"],
            )
            .expect("seed provider");
        }
        let p = create_profile(
            &db,
            ProfileCreatePayload {
                target_cli: TargetCli::Claude,
                name: "P".into(),
                description: None,
                icon: None,
                icon_color: None,
                provider_id: Some("some-pid".into()),
                settings_json: None,
                mcp_ids: None,
                skill_ids: None,
            },
        )
        .expect("create");

        // 显式设为 null
        let updated = update_profile(
            &db,
            &p.id,
            TargetCli::Claude,
            ProfileUpdatePayload {
                provider_id: Some(None),
                ..Default::default()
            },
        )
        .expect("update");
        assert!(updated.provider_id.is_none());
    }

    #[test]
    fn delete_profile_rejects_builtin() {
        let db = memory_db();
        // v11 迁移已为每个 CLI 生成一个 default-<cli> 内建 Profile
        let err = delete_profile(&db, "default-claude", TargetCli::Claude).unwrap_err();
        assert!(matches!(err, ProfileError::BuiltinProtected { .. }));
    }

    #[test]
    fn delete_profile_rejects_active() {
        let db = memory_db();
        let p = create_profile(
            &db,
            ProfileCreatePayload {
                target_cli: TargetCli::Claude,
                name: "P".into(),
                description: None,
                icon: None,
                icon_color: None,
                provider_id: None,
                settings_json: None,
                mcp_ids: None,
                skill_ids: None,
            },
        )
        .expect("create");
        activate_profile(&db, &p.id, TargetCli::Claude).expect("activate");

        let err = delete_profile(&db, &p.id, TargetCli::Claude).unwrap_err();
        assert!(matches!(err, ProfileError::ActiveProfile { .. }));
    }

    #[test]
    fn list_all_active_returns_map() {
        let db = memory_db();
        let result = list_all_active(&db).expect("list_all_active");
        // v11 迁移已激活 default-claude / default-codex
        assert_eq!(result.claude.as_deref(), Some("default-claude"));
        assert_eq!(result.codex.as_deref(), Some("default-codex"));
    }

    #[test]
    fn activate_profile_atomic_db_update() {
        let db = memory_db();
        let p = create_profile(
            &db,
            ProfileCreatePayload {
                target_cli: TargetCli::Codex,
                name: "Active".into(),
                description: None,
                icon: None,
                icon_color: None,
                provider_id: None,
                settings_json: None,
                mcp_ids: None,
                skill_ids: None,
            },
        )
        .expect("create");

        // 注意：activate 会写入 live 文件。为保证测试隔离，确保 CC_SWITCH_TEST_HOME 已设置。
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("CC_SWITCH_TEST_HOME", tmp.path());

        let r = activate_profile(&db, &p.id, TargetCli::Codex).expect("activate");
        assert!(r.success);
        assert_eq!(r.profile_id, p.id);

        let active = get_active_profile(&db, TargetCli::Codex)
            .expect("get_active")
            .expect("Some");
        assert_eq!(active.id, p.id);

        // transitioning_to 已清空
        let conn = db.conn.lock().expect("lock");
        let trans: Option<String> = conn
            .query_row(
                "SELECT transitioning_to FROM cli_state WHERE target_cli = 'codex'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert!(trans.is_none(), "transitioning_to 应被清空");
    }

    #[test]
    fn mcp_skill_association_replaces_on_update() {
        let db = memory_db();
        // 先插入两个 MCP server 行（profile_mcp FK 依赖）
        {
            let conn = db.conn.lock().expect("lock");
            conn.execute(
                "INSERT INTO mcp_servers (id, name, server_config) VALUES ('m1','M1','{}')",
                [],
            )
            .expect("seed m1");
            conn.execute(
                "INSERT INTO mcp_servers (id, name, server_config) VALUES ('m2','M2','{}')",
                [],
            )
            .expect("seed m2");
        }

        let p = create_profile(
            &db,
            ProfileCreatePayload {
                target_cli: TargetCli::Claude,
                name: "MCP test".into(),
                description: None,
                icon: None,
                icon_color: None,
                provider_id: None,
                settings_json: None,
                mcp_ids: Some(vec!["m1".into(), "m2".into()]),
                skill_ids: None,
            },
        )
        .expect("create");

        let entries = list_mcp_for_profile(&db, &p.id, TargetCli::Claude).expect("list");
        assert_eq!(entries.len(), 2);

        // 更新为只剩 m2
        update_profile(
            &db,
            &p.id,
            TargetCli::Claude,
            ProfileUpdatePayload {
                mcp_ids: Some(vec!["m2".into()]),
                ..Default::default()
            },
        )
        .expect("update");

        let entries = list_mcp_for_profile(&db, &p.id, TargetCli::Claude).expect("list2");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].mcp_id, "m2");
    }

    #[test]
    fn invalid_target_cli_string_rejected() {
        assert!(TargetCli::from_str_strict("gemini").is_err());
        assert!(TargetCli::from_str_strict("claude").is_ok());
        assert!(TargetCli::from_str_strict("codex").is_ok());
    }

    #[test]
    fn render_live_config_embeds_metadata() {
        let p = Profile {
            id: "x".into(),
            target_cli: TargetCli::Claude,
            name: "X".into(),
            description: None,
            icon: None,
            icon_color: None,
            provider_id: Some("p".into()),
            settings_json: r#"{"foo":1}"#.into(),
            sort_index: 0,
            is_builtin: false,
            mcp_ids: vec!["m".into()],
            skill_ids: vec![],
            created_at: 0,
            updated_at: 0,
        };
        let rendered = render_live_config(&p);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).expect("parse");
        assert_eq!(parsed["foo"], 1);
        assert_eq!(parsed["__cc_launcher_profile"]["profile_id"], "x");
        assert_eq!(parsed["__cc_launcher_profile"]["target_cli"], "claude");
    }
}
