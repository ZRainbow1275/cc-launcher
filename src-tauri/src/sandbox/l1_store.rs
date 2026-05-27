//! L1 软拦截规则持久化层。
//!
//! 数据形态：`Vec<L1Rule>` 序列化为 JSON 字符串，以单 key 形式持久化到 `settings` 表
//! （key=`sandbox.l1_rules.v1`）。这样无需新增 schema migration，复用 cc-switch
//! 已有的 settings 通用键值存储。
//!
//! 默认值：7 条规则 —— rm_arbitrary / write_outside_cwd / sudo_runas /
//! curl_pipe_sh / claude_skip_permissions / codex_yolo / network_revshell。
//!
//! 解锁流程：
//! - `set_rule_enabled(id, enabled)` —— 仅当规则 `unlockable=true` 才允许 disable。
//! - `unlock_rule(id, keyword)` —— 校验关键词（locale 任一即可），设置 24h
//!   `unlocked_until` 戳，期间该条规则被视作未启用（注入到 CLI 配置时跳过 deny）。
//!
//! **关键词比对：BYTE-WISE 严格相等**，不做 Unicode case folding。

use crate::database::Database;
use crate::sandbox::SandboxError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const L1_RULES_KEY: &str = "sandbox.l1_rules.v1";
const UNLOCK_DURATION_MS: i64 = 24 * 60 * 60 * 1000;

/// 三语解锁关键词。**byte-wise 大小写敏感**，禁止 Unicode case folding。
const UNLOCK_KEYWORD_ZH: &str = "我已知晓";
const UNLOCK_KEYWORD_EN: &str = "I UNDERSTAND";
const UNLOCK_KEYWORD_JA: &str = "理解しました";

/// L1 规则分类（与 contracts.ts::L1RuleCategory 对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum L1Category {
    DangerousFilesystem,
    PrivilegeEscalation,
    NetworkExposure,
    SuspiciousCommand,
    CliRiskyFlag,
}

/// L1 软拦截规则（可被 expert 模式临时解锁）。
///
/// Serialized as camelCase with ISO 8601 datetime fields, matching
/// `contracts.ts::L1Rule`. Storage uses `DateTime<Utc>`; corrupt or legacy
/// entries (e.g. old i64 epoch ms storage) are reset to defaults by
/// [`L1Store::list`] so the on-disk format always matches the public schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct L1Rule {
    pub id: String,
    pub category: L1Category,
    /// 用于命令字符串匹配的 regex；运行时编译。
    pub pattern: String,
    /// 三语 i18n key（前端按 locale 渲染）
    pub title_key: String,
    pub description_key: String,
    /// 是否被启用（true=拦截，false=放行）
    pub enabled: bool,
    /// 是否允许被 expert 模式临时解锁；少数高危规则永远不能解锁。
    pub unlockable: bool,
    /// 临时解锁的失效时间；None 表示未临时解锁。
    pub unlocked_until: Option<DateTime<Utc>>,
    /// 最近一次状态变更时间。
    pub updated_at: DateTime<Utc>,
}

/// 解锁尝试的结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockResult {
    pub rule_id: String,
    pub success: bool,
    pub unlocked_until: Option<DateTime<Utc>>,
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

/// 出厂默认 L1 规则 (7 条)，按 PRD §D3 + research §L1 enforcement matrix 编写。
pub fn default_rules() -> Vec<L1Rule> {
    let ts = now();
    vec![
        L1Rule {
            id: "L1.rm_arbitrary".to_string(),
            category: L1Category::DangerousFilesystem,
            pattern: r"^rm\s+-[rf]+\s+[^./~\s]".to_string(),
            title_key: "sandbox.l1.rm_arbitrary.title".to_string(),
            description_key: "sandbox.l1.rm_arbitrary.desc".to_string(),
            enabled: true,
            unlockable: true,
            unlocked_until: None,
            updated_at: ts,
        },
        L1Rule {
            id: "L1.write_outside_cwd".to_string(),
            category: L1Category::DangerousFilesystem,
            pattern: r"(?i)>\s*[/~]|>\s*[a-z]:".to_string(),
            title_key: "sandbox.l1.write_outside_cwd.title".to_string(),
            description_key: "sandbox.l1.write_outside_cwd.desc".to_string(),
            enabled: true,
            unlockable: true,
            unlocked_until: None,
            updated_at: ts,
        },
        L1Rule {
            id: "L1.sudo_runas".to_string(),
            category: L1Category::PrivilegeEscalation,
            pattern: r"^(sudo\b|runas\s+/user)".to_string(),
            title_key: "sandbox.l1.sudo_runas.title".to_string(),
            description_key: "sandbox.l1.sudo_runas.desc".to_string(),
            enabled: true,
            unlockable: true,
            unlocked_until: None,
            updated_at: ts,
        },
        L1Rule {
            id: "L1.curl_pipe_sh".to_string(),
            category: L1Category::SuspiciousCommand,
            pattern: r"curl\s+[^|]+\|\s*(sh|bash|zsh)".to_string(),
            title_key: "sandbox.l1.curl_pipe_sh.title".to_string(),
            description_key: "sandbox.l1.curl_pipe_sh.desc".to_string(),
            enabled: true,
            unlockable: true,
            unlocked_until: None,
            updated_at: ts,
        },
        L1Rule {
            id: "L1.claude_skip_permissions".to_string(),
            category: L1Category::CliRiskyFlag,
            pattern: r"--dangerously-skip-permissions\b".to_string(),
            title_key: "sandbox.l1.claude_skip_permissions.title".to_string(),
            description_key: "sandbox.l1.claude_skip_permissions.desc".to_string(),
            enabled: true,
            // 用户可在知情同意 ("I UNDERSTAND") 下临时解锁 24h；settings_injection
            // 模块负责把 disableBypassPermissionsMode 写回 ~/.claude/settings.json
            // 以兜底直跑 claude CLI 的场景。
            unlockable: true,
            unlocked_until: None,
            updated_at: ts,
        },
        L1Rule {
            id: "L1.codex_yolo".to_string(),
            category: L1Category::CliRiskyFlag,
            pattern: r"--dangerously-bypass-approvals-and-sandbox|--yolo\b".to_string(),
            title_key: "sandbox.l1.codex_yolo.title".to_string(),
            description_key: "sandbox.l1.codex_yolo.desc".to_string(),
            enabled: true,
            unlockable: true,
            unlocked_until: None,
            updated_at: ts,
        },
        L1Rule {
            id: "L1.network_revshell".to_string(),
            category: L1Category::NetworkExposure,
            pattern: r"bash\s+-i\s+>&?\s+/dev/tcp/".to_string(),
            title_key: "sandbox.l1.network_revshell.title".to_string(),
            description_key: "sandbox.l1.network_revshell.desc".to_string(),
            enabled: true,
            unlockable: false,
            unlocked_until: None,
            updated_at: ts,
        },
    ]
}

/// 关键词校验：byte-wise 大小写敏感，禁止 Unicode case folding。
///
/// 接受三种 locale 关键词之一即视为有效；其它输入（含大小写不同的变体）一律拒绝。
pub fn is_valid_unlock_keyword(input: &str) -> bool {
    // 使用 `==`（byte-wise equality）而非 `eq_ignore_ascii_case`。
    input == UNLOCK_KEYWORD_ZH || input == UNLOCK_KEYWORD_EN || input == UNLOCK_KEYWORD_JA
}

/// L1 store 操作层。复用 `Database::get_setting` / `set_setting`。
pub struct L1Store {
    db: Arc<Database>,
}

impl L1Store {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 读取全部规则；如果存储为空则写入默认值并返回。
    pub fn list(&self) -> Result<Vec<L1Rule>, SandboxError> {
        let raw = self
            .db
            .get_setting(L1_RULES_KEY)
            .map_err(SandboxError::Storage)?;

        if let Some(json) = raw {
            match serde_json::from_str::<Vec<L1Rule>>(&json) {
                Ok(mut rules) => {
                    let now_ts = now();
                    let mut dirty = false;
                    // 清理已过期的临时解锁戳
                    for r in rules.iter_mut() {
                        if let Some(until) = r.unlocked_until {
                            if until <= now_ts {
                                r.unlocked_until = None;
                                dirty = true;
                            }
                        }
                    }
                    // 迁移：旧安装版本曾把 claude_skip_permissions / codex_yolo 烤成
                    // unlockable=false；现在用户可以在知情同意下解锁。force-update。
                    for r in rules.iter_mut() {
                        if (r.id == "L1.claude_skip_permissions" || r.id == "L1.codex_yolo")
                            && !r.unlockable
                        {
                            r.unlockable = true;
                            r.updated_at = now_ts;
                            dirty = true;
                        }
                    }
                    if dirty {
                        self.save(&rules)?;
                    }
                    Ok(rules)
                }
                Err(_) => {
                    // 损坏（含旧格式 i64 epoch ms）→ 重置为默认。
                    // 24h 临时解锁状态属于易失数据，丢失可接受。
                    let defaults = default_rules();
                    self.save(&defaults)?;
                    Ok(defaults)
                }
            }
        } else {
            let defaults = default_rules();
            self.save(&defaults)?;
            Ok(defaults)
        }
    }

    pub fn save(&self, rules: &[L1Rule]) -> Result<(), SandboxError> {
        let json = serde_json::to_string(rules)
            .map_err(|e| SandboxError::Internal(format!("serialize L1 rules: {e}")))?;
        self.db
            .set_setting(L1_RULES_KEY, &json)
            .map_err(SandboxError::Storage)
    }

    /// 切换规则启用状态。`unlockable=false` 的规则不允许 disable。
    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<L1Rule, SandboxError> {
        let mut rules = self.list()?;
        let idx = rules
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(|| SandboxError::RuleNotFound(id.to_string()))?;

        if !enabled && !rules[idx].unlockable {
            return Err(SandboxError::RuleNotUnlockable(id.to_string()));
        }

        rules[idx].enabled = enabled;
        rules[idx].updated_at = now();
        // 切换状态时清理临时解锁
        if enabled {
            rules[idx].unlocked_until = None;
        }
        self.save(&rules)?;
        Ok(rules[idx].clone())
    }

    /// 用 keyword 临时解锁规则 24 小时。
    /// - `unlockable=false` 的规则永远拒绝。
    /// - keyword 必须严格匹配三个 locale 关键词之一（byte-wise）。
    pub fn unlock(&self, id: &str, keyword: &str) -> Result<UnlockResult, SandboxError> {
        let mut rules = self.list()?;
        let idx = rules
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(|| SandboxError::RuleNotFound(id.to_string()))?;

        if !rules[idx].unlockable {
            return Err(SandboxError::RuleNotUnlockable(id.to_string()));
        }

        if !is_valid_unlock_keyword(keyword) {
            return Err(SandboxError::InvalidUnlockKeyword);
        }

        let now_ts = now();
        let until = now_ts + chrono::Duration::milliseconds(UNLOCK_DURATION_MS);
        rules[idx].unlocked_until = Some(until);
        rules[idx].updated_at = now_ts;
        self.save(&rules)?;

        Ok(UnlockResult {
            rule_id: id.to_string(),
            success: true,
            unlocked_until: Some(until),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn make_db() -> Arc<Database> {
        // 直接构造内存 SQLite + 必要的 settings 表
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .expect("create settings table");
        Arc::new(Database {
            conn: Mutex::new(conn),
        })
    }

    #[test]
    fn first_list_seeds_seven_default_rules() {
        let store = L1Store::new(make_db());
        let rules = store.list().expect("list");
        assert_eq!(rules.len(), 7);
        let ids: Vec<&str> = rules.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"L1.rm_arbitrary"));
        assert!(ids.contains(&"L1.write_outside_cwd"));
        assert!(ids.contains(&"L1.sudo_runas"));
        assert!(ids.contains(&"L1.curl_pipe_sh"));
        assert!(ids.contains(&"L1.claude_skip_permissions"));
        assert!(ids.contains(&"L1.codex_yolo"));
        assert!(ids.contains(&"L1.network_revshell"));
    }

    #[test]
    fn set_enabled_toggles_persisted_state() {
        let store = L1Store::new(make_db());
        let updated = store
            .set_enabled("L1.curl_pipe_sh", false)
            .expect("disable curl_pipe_sh");
        assert!(!updated.enabled);

        let rules = store.list().expect("re-list");
        let r = rules.iter().find(|r| r.id == "L1.curl_pipe_sh").unwrap();
        assert!(!r.enabled);
    }

    #[test]
    fn set_enabled_rejects_disabling_non_unlockable_rule() {
        let store = L1Store::new(make_db());
        let result = store.set_enabled("L1.network_revshell", false);
        assert!(matches!(result, Err(SandboxError::RuleNotUnlockable(_))));
    }

    #[test]
    fn set_enabled_allows_disabling_claude_skip_permissions() {
        let store = L1Store::new(make_db());
        let updated = store
            .set_enabled("L1.claude_skip_permissions", false)
            .expect("claude_skip_permissions is now unlockable, should disable");
        assert!(!updated.enabled);
    }

    #[test]
    fn unlock_with_valid_keyword_sets_24h_unlock() {
        let store = L1Store::new(make_db());
        let res = store
            .unlock("L1.sudo_runas", "我已知晓")
            .expect("unlock sudo_runas");
        assert!(res.success);
        let until = res.unlocked_until.expect("unlocked_until set");
        let now_ts = now();
        assert!(until > now_ts);
        // 至少应该是 23.5h 后到期
        assert!((until - now_ts).num_milliseconds() > 23 * 60 * 60 * 1000);
    }

    #[test]
    fn unlock_with_invalid_keyword_rejected() {
        let store = L1Store::new(make_db());
        let bad = store.unlock("L1.sudo_runas", "i understand");
        assert!(matches!(bad, Err(SandboxError::InvalidUnlockKeyword)));

        let bad = store.unlock("L1.sudo_runas", "I Understand");
        assert!(matches!(bad, Err(SandboxError::InvalidUnlockKeyword)));

        let bad = store.unlock("L1.sudo_runas", "");
        assert!(matches!(bad, Err(SandboxError::InvalidUnlockKeyword)));
    }

    #[test]
    fn unlock_each_locale_keyword_accepted() {
        for kw in ["我已知晓", "I UNDERSTAND", "理解しました"] {
            let store = L1Store::new(make_db());
            let res = store.unlock("L1.sudo_runas", kw).expect("unlock");
            assert!(res.success, "keyword `{kw}` should be accepted");
        }
    }

    #[test]
    fn unlock_non_unlockable_rule_rejected() {
        let store = L1Store::new(make_db());
        let res = store.unlock("L1.network_revshell", "I UNDERSTAND");
        assert!(matches!(res, Err(SandboxError::RuleNotUnlockable(_))));
    }

    #[test]
    fn unlock_claude_skip_permissions_accepted() {
        let store = L1Store::new(make_db());
        let res = store
            .unlock("L1.claude_skip_permissions", "I UNDERSTAND")
            .expect("claude_skip_permissions is now unlockable");
        assert!(res.success);
        assert!(res.unlocked_until.is_some());
    }

    #[test]
    fn unlock_codex_yolo_accepted() {
        let store = L1Store::new(make_db());
        let res = store
            .unlock("L1.codex_yolo", "I UNDERSTAND")
            .expect("codex_yolo is now unlockable");
        assert!(res.success);
        assert!(res.unlocked_until.is_some());
    }

    #[test]
    fn migration_force_updates_unlockable_for_legacy_persisted_rules() {
        // 模拟旧版本持久化：claude_skip_permissions + codex_yolo unlockable=false
        let store = L1Store::new(make_db());
        let mut rules = store.list().expect("seed");
        for r in rules.iter_mut() {
            if r.id == "L1.claude_skip_permissions" || r.id == "L1.codex_yolo" {
                r.unlockable = false;
            }
        }
        store.save(&rules).expect("save legacy shape");

        // 下次 list() 触发迁移
        let migrated = store.list().expect("list after migration");
        let claude = migrated
            .iter()
            .find(|r| r.id == "L1.claude_skip_permissions")
            .unwrap();
        let codex = migrated.iter().find(|r| r.id == "L1.codex_yolo").unwrap();
        let revshell = migrated
            .iter()
            .find(|r| r.id == "L1.network_revshell")
            .unwrap();
        assert!(
            claude.unlockable,
            "migration should flip claude_skip_permissions to unlockable"
        );
        assert!(
            codex.unlockable,
            "migration should flip codex_yolo to unlockable"
        );
        assert!(
            !revshell.unlockable,
            "network_revshell must remain permanently locked"
        );
    }

    #[test]
    fn expired_unlock_is_cleared_on_next_list() {
        let store = L1Store::new(make_db());
        // 注入一个已过期的 unlocked_until
        let mut rules = store.list().expect("seed");
        let idx = rules.iter().position(|r| r.id == "L1.sudo_runas").unwrap();
        rules[idx].unlocked_until = Some(now() - chrono::Duration::seconds(1));
        store.save(&rules).expect("save");

        let rules = store.list().expect("re-list");
        let r = rules.iter().find(|r| r.id == "L1.sudo_runas").unwrap();
        assert_eq!(
            r.unlocked_until, None,
            "expired unlock_until must be cleared on list()"
        );
    }

    #[test]
    fn keyword_validator_is_byte_wise() {
        assert!(is_valid_unlock_keyword("我已知晓"));
        assert!(is_valid_unlock_keyword("I UNDERSTAND"));
        assert!(is_valid_unlock_keyword("理解しました"));
        assert!(!is_valid_unlock_keyword("i understand"));
        assert!(!is_valid_unlock_keyword("I understand"));
        assert!(!is_valid_unlock_keyword(" I UNDERSTAND "));
        assert!(!is_valid_unlock_keyword("我已知曉")); // 繁体不同字符
    }
}
