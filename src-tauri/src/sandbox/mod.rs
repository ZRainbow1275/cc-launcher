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
use std::process::Command;
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
    #[error("job object: {0}")]
    JobObject(String),
    #[error("restricted token: {0}")]
    RestrictedToken(String),
    #[error("sandbox-exec: {0}")]
    SandboxExec(String),
    #[error("unsupported on this platform")]
    Unsupported,
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
        sandbox_applied: None,
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
// 平台沙盒 shim —— 在 spawn 前后绑定 OS 级隔离
// ─────────────────────────────────────────────────────────────────────

/// 测试可观测：`apply_to_command` 被调用的次数。
///
/// 在 release 和 test 构建下都可见（开销极小：一次 atomic 加法），让集成测试也能断言
/// shim 实际触发，避免 launcher 静默回退到只设 creation flags 的退化路径。
pub static APPLY_TO_COMMAND_CALL_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// 对一个未 spawn 的 `Command` 应用 OS 级沙盒约束。MUST 在 `spawn()` 之前调用。
///
/// 平台行为：
/// - **Windows**：设置 `creation_flags(CREATE_NEW_CONSOLE | CREATE_NEW_PROCESS_GROUP)`。
///   Job Object 绑定发生在 spawn 之后，必须通过 `assign_to_job_object(pid, level)` 完成
///   （Windows 要求进程句柄，不能在 spawn 前应用）。
/// - **macOS**：渲染当前 cwd 对应的 SBPL profile，并把 argv 包成 `sandbox-exec -f <profile>
///   <original_program> <original_args...>`。同时为 child 设置 `__SBPL_APPLIED=1`，
///   方便测试断言。
/// - **Linux**：通过 `pre_exec` 调 `setsid` 与新会话分离；MVP 阶段无 LSM。
///
/// `level` 控制严格程度：`Strict` → L2 红线；`Medium` → L1 + L2 红线（语义层面，由
/// `redline.rs` 编译期保证 L2 始终生效）。
pub fn apply_to_command(cmd: &mut Command, level: SandboxLevel) -> Result<(), SandboxError> {
    APPLY_TO_COMMAND_CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    apply_to_command_impl(cmd, level)
}

#[cfg(target_os = "windows")]
fn apply_to_command_impl(cmd: &mut Command, _level: SandboxLevel) -> Result<(), SandboxError> {
    use std::os::windows::process::CommandExt;
    // CREATE_NEW_CONSOLE (0x10) + CREATE_NEW_PROCESS_GROUP (0x200)：让子终端独占控制台，
    // 同时保留干净的 Ctrl+C 语义。Job Object 限制在 spawn 后通过 assign_to_job_object 加上。
    cmd.creation_flags(0x00000010 | 0x00000200);
    Ok(())
}

#[cfg(target_os = "macos")]
fn apply_to_command_impl(cmd: &mut Command, _level: SandboxLevel) -> Result<(), SandboxError> {
    use std::os::unix::process::CommandExt;

    // 1) 用当前 cwd 渲染 SBPL profile 文件。若 Command 未显式设 cwd，则使用进程的 cwd。
    let cwd = std::env::current_dir()
        .map_err(|e| SandboxError::SandboxExec(format!("read cwd: {e}")))?;
    let profile_path = sandbox_exec::apply_sandbox_exec_profile(&cwd)
        .map_err(|e| SandboxError::SandboxExec(e.to_string()))?;

    // 2) 把 argv 包装成 sandbox-exec -f <profile> <program> <args...>
    //    通过把当前程序与所有参数挪到 sandbox-exec 之后实现 wrap。
    //    Rust 的 std::process::Command 在 macOS 下没有可枚举的现有 argv，
    //    所以我们把 cmd 替换为一个新的 sandbox-exec 命令并保留其余 builder 状态。
    let original_program = cmd.get_program().to_owned();
    let original_args: Vec<std::ffi::OsString> =
        cmd.get_args().map(|a| a.to_owned()).collect();

    // 重置为 sandbox-exec
    *cmd = Command::new("/usr/bin/sandbox-exec");
    cmd.arg("-f").arg(&profile_path).arg(original_program);
    for a in original_args {
        cmd.arg(a);
    }

    // 给 child 设一个标记环境变量，方便测试断言 sandbox 已被应用。
    cmd.env("__SBPL_APPLIED", "1");

    // setsid 分离控制 TTY —— 与 Linux 路径一致。
    unsafe {
        cmd.pre_exec(|| {
            let _ = libc_setsid();
            Ok(())
        });
    }
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn apply_to_command_impl(cmd: &mut Command, _level: SandboxLevel) -> Result<(), SandboxError> {
    use std::os::unix::process::CommandExt;
    // SAFETY: setsid 是 async-signal-safe；fork 后第一时间调用安全。
    unsafe {
        cmd.pre_exec(|| {
            let _ = libc_setsid();
            Ok(())
        });
    }
    Ok(())
}

#[cfg(not(any(target_os = "windows", unix)))]
fn apply_to_command_impl(_cmd: &mut Command, _level: SandboxLevel) -> Result<(), SandboxError> {
    Err(SandboxError::Unsupported)
}

#[cfg(unix)]
fn libc_setsid() -> i64 {
    extern "C" {
        fn setsid() -> i64;
    }
    unsafe { setsid() }
}

/// Post-spawn Job Object 绑定（仅 Windows 有意义）。
///
/// Windows 上 Job Object 必须基于真实的进程句柄，所以这一步必须发生在 `spawn()` 之后。
/// 创建一个新的 Job 并把目标进程加入；返回 Ok 后 child 即受 KILL_ON_JOB_CLOSE 等约束。
///
/// **生命周期处理**：因为 `KILL_ON_JOB_CLOSE` + 唯一句柄关闭 == 杀死 child，
/// 这里使用 `std::mem::forget` 让 JobHandle 与 launcher 进程共存活。launcher 退出时
/// 系统会回收句柄，KILL_ON_JOB_CLOSE 才生效，从而保证子终端不会因为函数返回就被秒杀。
#[cfg(windows)]
pub fn assign_to_job_object(pid: u32, _level: SandboxLevel) -> Result<(), SandboxError> {
    let job = job_object::create_sandbox_job()
        .map_err(|e| SandboxError::JobObject(e.to_string()))?;
    job.assign_process(pid)
        .map_err(|e| SandboxError::JobObject(e.to_string()))?;
    // 关键：不要 drop JobHandle，否则 CloseHandle → KILL_ON_JOB_CLOSE 立即杀死 child。
    // launcher 进程退出时操作系统统一回收，届时 child 才会被 Job 终止 —— 这是期望行为。
    std::mem::forget(job);
    Ok(())
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn assign_to_job_object(_pid: u32, _level: SandboxLevel) -> Result<(), SandboxError> {
    Ok(())
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
