//! Claude / Codex 设置注入与篡改检测。
//!
//! PR2 of the sandbox unlock-and-injection task. Sandbox 的 L1 拦截只能拦住 cc-launcher
//! 自己 spawn 的 CLI 进程；当用户从原生终端直接跑 `claude --dangerously-skip-permissions`
//! 时，必须靠 `~/.claude/settings.json::disableBypassPermissionsMode = "disable"` 兜底。
//! 本模块负责：
//!   1. 启动时把基线写入 settings.json（含 deny rules）。
//!   2. CLI spawn 前做完整性校验，发现被篡改就重写一次。
//!   3. 用户解锁 `L1.claude_skip_permissions` 24h 期间临时移除 bypass block；过期后恢复。
//!   4. Codex 只能写最低基线（approval_policy="suggest"）。`--yolo` 在 CLI 侧无法禁用。
//!
//! Codex 写入仅保留最低基线 —— per research/codex-settings-schema.md, Codex 的
//! `--yolo` 命令行参数不受 config.toml 任何字段约束。

use std::path::Path;

use serde_json::{json, Value};
use toml_edit::{value as toml_value, DocumentMut};

use crate::config::{atomic_write, get_claude_settings_path};
use crate::error::AppError;
use crate::sandbox::L1Store;

/// 完整性校验时使用的固定 deny 条目；cc-launcher 始终保证这些条目存在于
/// `permissions.deny` 中（用户可以追加自定义条目，但不能移除以下任意一条）。
const MANAGED_DENY_RULES: &[&str] = &[
    "Bash(rm -rf /)",
    "Bash(rm -rf ~)",
    "Edit(~/.bashrc)",
    "Edit(~/.zshrc)",
    "Edit(~/.profile)",
];

const DISABLE_BYPASS_KEY: &str = "disableBypassPermissionsMode";
const DISABLE_BYPASS_VALUE: &str = "disable";
const L1_CLAUDE_SKIP_ID: &str = "L1.claude_skip_permissions";

/// 是否需要把 `disableBypassPermissionsMode` 写入 settings.json。
/// 仅当 L1.claude_skip_permissions 处于"未解锁"状态时才写。
fn is_skip_permissions_locked(l1_store: &L1Store) -> bool {
    match l1_store.list() {
        Ok(rules) => {
            let rule = rules.iter().find(|r| r.id == L1_CLAUDE_SKIP_ID);
            match rule {
                Some(r) => {
                    // unlocked 的语义对齐 launcher_service::collect_unlocked_l1_ids:
                    // enabled=false 或 unlocked_until 仍在未来都视作 unlocked。
                    if !r.enabled {
                        return false;
                    }
                    if let Some(until) = r.unlocked_until {
                        return until <= chrono::Utc::now();
                    }
                    true
                }
                // 规则缺失时按"已锁定"处理（fail closed）。
                None => true,
            }
        }
        // 读取失败时按"已锁定"处理 —— 宁可重复写入也不要漏掉兜底。
        Err(e) => {
            log::warn!("settings_injection: list L1 rules failed, fail-closed: {e}");
            true
        }
    }
}

/// 把 settings.json 注入到 `path`，根据 l1_store 决定是否写入 disableBypassPermissionsMode。
///
/// 不存在则创建 `{}`；存在则保留用户的所有键，只触碰 `disableBypassPermissionsMode`
/// 和 `permissions.deny` 数组（追加缺失的 managed 条目，不去重移除用户自定义条目）。
pub fn inject_claude_settings_at(path: &Path, l1_store: &L1Store) -> Result<(), AppError> {
    let mut value = read_or_default_settings(path)?;
    let lock = is_skip_permissions_locked(l1_store);
    apply_claude_baseline(&mut value, lock);
    write_pretty_json(path, &value)
}

/// 等价于 `inject_claude_settings_at(get_claude_settings_path(), ...)`。
pub fn inject_claude_settings(l1_store: &L1Store) -> Result<(), AppError> {
    inject_claude_settings_at(&get_claude_settings_path(), l1_store)
}

/// 解锁事件回调：移除 `disableBypassPermissionsMode` 键（如果存在）。
pub fn remove_bypass_block_at(path: &Path) -> Result<(), AppError> {
    let mut value = read_or_default_settings(path)?;
    if let Some(obj) = value.as_object_mut() {
        obj.remove(DISABLE_BYPASS_KEY);
    }
    write_pretty_json(path, &value)
}

pub fn remove_bypass_block() -> Result<(), AppError> {
    remove_bypass_block_at(&get_claude_settings_path())
}

/// 解锁过期回调：恢复 `disableBypassPermissionsMode = "disable"`。
pub fn restore_bypass_block_at(path: &Path) -> Result<(), AppError> {
    let mut value = read_or_default_settings(path)?;
    ensure_object(&mut value).insert(
        DISABLE_BYPASS_KEY.to_string(),
        Value::String(DISABLE_BYPASS_VALUE.to_string()),
    );
    write_pretty_json(path, &value)
}

pub fn restore_bypass_block() -> Result<(), AppError> {
    restore_bypass_block_at(&get_claude_settings_path())
}

/// 完整性校验：要求规则锁定时 disableBypassPermissionsMode 必须等于 "disable"，
/// 且所有 managed deny 规则都存在。返回 false 表示被篡改，调用方应再注入一次。
pub fn verify_claude_integrity_at(path: &Path, l1_store: &L1Store) -> bool {
    let value = match read_or_default_settings(path) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("settings_injection: verify read failed: {e}");
            return false;
        }
    };

    if is_skip_permissions_locked(l1_store) {
        let ok = value
            .get(DISABLE_BYPASS_KEY)
            .and_then(Value::as_str)
            .map(|s| s == DISABLE_BYPASS_VALUE)
            .unwrap_or(false);
        if !ok {
            return false;
        }
    }

    let deny = value
        .get("permissions")
        .and_then(|p| p.get("deny"))
        .and_then(Value::as_array);
    let Some(deny) = deny else {
        return false;
    };
    for rule in MANAGED_DENY_RULES {
        let present = deny
            .iter()
            .any(|v| v.as_str().map(|s| s == *rule).unwrap_or(false));
        if !present {
            return false;
        }
    }
    true
}

pub fn verify_claude_integrity(l1_store: &L1Store) -> bool {
    verify_claude_integrity_at(&get_claude_settings_path(), l1_store)
}

/// Codex 基线：保证 `approval_policy` 至少为 "suggest"。如果用户已显式设置任何值，
/// 不覆盖。仅在缺失时写入 —— per research, `--yolo` CLI flag 不受任何 config.toml
/// 字段约束，本函数是"尽力而为"的基线，不构成沙盒拦截。
pub fn inject_codex_baseline_at(path: &Path) -> Result<(), AppError> {
    let raw = if path.exists() {
        std::fs::read_to_string(path).map_err(|e| AppError::io(path, e))?
    } else {
        String::new()
    };

    let mut doc: DocumentMut = if raw.trim().is_empty() {
        DocumentMut::new()
    } else {
        raw.parse::<DocumentMut>()
            .map_err(|e| AppError::Config(format!("解析 codex config.toml 失败: {e}")))?
    };

    let has_policy = doc.get("approval_policy").is_some();
    if !has_policy {
        doc["approval_policy"] = toml_value("suggest");
        log::info!("settings_injection: 写入 Codex 基线 approval_policy=suggest");
    }

    let rendered = doc.to_string();
    atomic_write(path, rendered.as_bytes())
}

pub fn inject_codex_baseline() -> Result<(), AppError> {
    inject_codex_baseline_at(&crate::codex_config::get_codex_config_path())
}

// ─────────────────────────────────────────────────────────────────────
// 内部 helpers
// ─────────────────────────────────────────────────────────────────────

fn read_or_default_settings(path: &Path) -> Result<Value, AppError> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = std::fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str::<Value>(&text).map_err(|e| AppError::json(path, e))
}

fn ensure_object(value: &mut Value) -> &mut serde_json::Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }
    value.as_object_mut().expect("just ensured object")
}

fn apply_claude_baseline(value: &mut Value, lock_skip_permissions: bool) {
    let obj = ensure_object(value);

    if lock_skip_permissions {
        obj.insert(
            DISABLE_BYPASS_KEY.to_string(),
            Value::String(DISABLE_BYPASS_VALUE.to_string()),
        );
    } else {
        obj.remove(DISABLE_BYPASS_KEY);
    }

    // permissions.deny: 取或建对象，再 append managed 条目。
    let permissions = obj
        .entry("permissions".to_string())
        .or_insert_with(|| json!({}));
    if !permissions.is_object() {
        *permissions = json!({});
    }
    let perm_obj = permissions.as_object_mut().expect("just ensured object");
    let deny_entry = perm_obj
        .entry("deny".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !deny_entry.is_array() {
        *deny_entry = Value::Array(Vec::new());
    }
    let deny = deny_entry.as_array_mut().expect("just ensured array");
    for rule in MANAGED_DENY_RULES {
        let already = deny
            .iter()
            .any(|v| v.as_str().map(|s| s == *rule).unwrap_or(false));
        if !already {
            deny.push(Value::String((*rule).to_string()));
        }
    }
}

/// 4-space indent pretty-print；保留 serde_json 的对象 key 顺序（依赖 preserve_order
/// feature），不做字母重排 —— 这一点和 config::write_json_file 不同，因为 Claude Code
/// 自己写出来的文件就是带顺序的。
fn write_pretty_json(path: &Path, value: &Value) -> Result<(), AppError> {
    use serde::Serialize as _;
    let mut buf = Vec::with_capacity(128);
    let indent = b"    ";
    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent);
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value
        .serialize(&mut ser)
        .map_err(|e| AppError::JsonSerialize { source: e })?;
    atomic_write(path, &buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    use crate::database::Database;
    use crate::sandbox::L1Store;

    fn make_store() -> L1Store {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .expect("create settings table");
        L1Store::new(Arc::new(Database {
            conn: Mutex::new(conn),
        }))
    }

    fn read_value(path: &Path) -> Value {
        let text = std::fs::read_to_string(path).expect("read");
        serde_json::from_str(&text).expect("parse")
    }

    #[test]
    fn inject_creates_settings_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = make_store();

        inject_claude_settings_at(&path, &store).expect("inject");
        assert!(path.exists(), "settings.json should be created");

        let v = read_value(&path);
        assert_eq!(
            v.get(DISABLE_BYPASS_KEY).and_then(Value::as_str),
            Some("disable")
        );
        let deny = v
            .get("permissions")
            .and_then(|p| p.get("deny"))
            .and_then(Value::as_array)
            .expect("deny array");
        for rule in MANAGED_DENY_RULES {
            assert!(
                deny.iter().any(|x| x.as_str() == Some(*rule)),
                "deny should contain {rule}"
            );
        }
    }

    #[test]
    fn inject_preserves_existing_user_keys() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let existing = r#"{
    "env": {"FOO": "bar"},
    "model": "opus",
    "permissions": {
        "allow": ["Bash(ls)"]
    }
}"#;
        std::fs::write(&path, existing).unwrap();

        let store = make_store();
        inject_claude_settings_at(&path, &store).expect("inject");

        let v = read_value(&path);
        assert_eq!(
            v.get("env")
                .and_then(|e| e.get("FOO"))
                .and_then(Value::as_str),
            Some("bar"),
            "FOO key must survive"
        );
        assert_eq!(v.get("model").and_then(Value::as_str), Some("opus"));
        let allow = v
            .get("permissions")
            .and_then(|p| p.get("allow"))
            .and_then(Value::as_array)
            .expect("allow array");
        assert_eq!(allow.len(), 1);
        assert_eq!(allow[0].as_str(), Some("Bash(ls)"));
    }

    #[test]
    fn inject_sets_disable_bypass_when_locked() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = make_store();

        inject_claude_settings_at(&path, &store).expect("inject");
        let v = read_value(&path);
        assert_eq!(
            v.get(DISABLE_BYPASS_KEY).and_then(Value::as_str),
            Some("disable"),
            "must enforce disableBypassPermissionsMode while locked"
        );
    }

    #[test]
    fn inject_removes_disable_bypass_when_unlocked() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = make_store();

        // First inject baseline (locked → key present).
        inject_claude_settings_at(&path, &store).expect("inject1");

        // Now unlock the rule.
        store
            .unlock(L1_CLAUDE_SKIP_ID, "I UNDERSTAND")
            .expect("unlock");

        inject_claude_settings_at(&path, &store).expect("inject2");
        let v = read_value(&path);
        assert!(
            v.get(DISABLE_BYPASS_KEY).is_none(),
            "disableBypassPermissionsMode must be removed while unlocked"
        );
    }

    #[test]
    fn inject_appends_deny_rules_without_duplicates() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = make_store();

        inject_claude_settings_at(&path, &store).expect("inject1");
        inject_claude_settings_at(&path, &store).expect("inject2");

        let v = read_value(&path);
        let deny = v
            .get("permissions")
            .and_then(|p| p.get("deny"))
            .and_then(Value::as_array)
            .expect("deny");
        for rule in MANAGED_DENY_RULES {
            let count = deny.iter().filter(|x| x.as_str() == Some(*rule)).count();
            assert_eq!(count, 1, "deny rule {rule} must appear exactly once");
        }
    }

    #[test]
    fn verify_integrity_detects_tampering() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = make_store();

        inject_claude_settings_at(&path, &store).expect("inject");
        assert!(verify_claude_integrity_at(&path, &store), "fresh inject ok");

        // Tamper: remove disableBypassPermissionsMode key.
        let mut v = read_value(&path);
        v.as_object_mut().unwrap().remove(DISABLE_BYPASS_KEY);
        std::fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();
        assert!(
            !verify_claude_integrity_at(&path, &store),
            "missing disableBypassPermissionsMode must fail verify"
        );

        // Restore + tamper deny array.
        inject_claude_settings_at(&path, &store).expect("re-inject");
        let mut v = read_value(&path);
        v["permissions"]["deny"] = json!([]);
        std::fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();
        assert!(
            !verify_claude_integrity_at(&path, &store),
            "wiped deny array must fail verify"
        );
    }

    #[test]
    fn remove_and_restore_bypass_block_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = make_store();
        inject_claude_settings_at(&path, &store).expect("inject");

        remove_bypass_block_at(&path).expect("remove");
        let v = read_value(&path);
        assert!(v.get(DISABLE_BYPASS_KEY).is_none());

        restore_bypass_block_at(&path).expect("restore");
        let v = read_value(&path);
        assert_eq!(
            v.get(DISABLE_BYPASS_KEY).and_then(Value::as_str),
            Some("disable")
        );
    }

    #[test]
    fn inject_codex_baseline_sets_approval_policy() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        inject_codex_baseline_at(&path).expect("inject codex");
        let text = std::fs::read_to_string(&path).expect("read");
        let parsed: toml::Table = toml::from_str(&text).expect("parse toml");
        assert_eq!(
            parsed.get("approval_policy").and_then(|v| v.as_str()),
            Some("suggest")
        );
    }

    #[test]
    fn inject_codex_baseline_preserves_user_approval_policy() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "approval_policy = \"never\"\nmodel = \"gpt-5\"\n").unwrap();

        inject_codex_baseline_at(&path).expect("inject codex");
        let text = std::fs::read_to_string(&path).expect("read");
        let parsed: toml::Table = toml::from_str(&text).expect("parse");
        // 用户已设置就不覆盖
        assert_eq!(
            parsed.get("approval_policy").and_then(|v| v.as_str()),
            Some("never")
        );
        assert_eq!(parsed.get("model").and_then(|v| v.as_str()), Some("gpt-5"));
    }
}
