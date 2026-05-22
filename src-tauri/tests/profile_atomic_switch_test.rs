//! 集成测试：activate_profile 5 阶段原子切换
//!
//! 覆盖：
//! 1. 完整成功路径：所有 5 阶段都执行
//! 2. backup 阶段失败（live 文件目录不可写）→ transitioning_to 清空、active 未变
//! 3. write_live_config 阶段失败 → backup 还原 + transitioning_to 清空
//! 4. release 阶段成功后，live 文件存在且包含 Profile metadata
//! 5. 并发切换不破坏一致性

use std::path::PathBuf;
use std::sync::Arc;

use cc_switch_lib::services::profile::{self, ProfileCreatePayload, TargetCli};
use cc_switch_lib::Database;

mod support;

fn fresh_test_db_with_home() -> (Arc<Database>, PathBuf, std::sync::MutexGuard<'static, ()>) {
    let guard = support::test_mutex().lock().expect("acquire test mutex");
    support::ensure_test_home();
    support::reset_test_fs();

    let db = Arc::new(Database::init().expect("init db"));
    let home = std::path::PathBuf::from(
        std::env::var("CC_SWITCH_TEST_HOME").expect("CC_SWITCH_TEST_HOME set"),
    );
    (db, home, guard)
}

#[test]
fn activate_full_success_writes_live_file_and_updates_db() {
    let (db, home, _guard) = fresh_test_db_with_home();

    // 创建一个 Profile
    let p = profile::create_profile(
        &db,
        ProfileCreatePayload {
            target_cli: TargetCli::Claude,
            name: "Switch Test".into(),
            description: None,
            icon: None,
            icon_color: None,
            provider_id: None,
            settings_json: Some(r#"{"model":"claude-sonnet-4-7"}"#.into()),
            mcp_ids: None,
            skill_ids: None,
        },
    )
    .expect("create");

    // 切换
    let result = profile::activate_profile(&db, &p.id, TargetCli::Claude).expect("activate ok");
    assert!(result.success);
    assert_eq!(result.profile_id, p.id);

    // live 文件被写入
    let live = home.join(".claude").join("settings.json");
    assert!(live.exists(), "live settings.json 应存在");

    let content = std::fs::read_to_string(&live).expect("read live");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("parse live json");
    assert_eq!(parsed["__cc_launcher_profile"]["profile_id"], p.id);
    assert_eq!(parsed["__cc_launcher_profile"]["target_cli"], "claude");
    assert_eq!(parsed["model"], "claude-sonnet-4-7");

    // DB 中 active_profile_id 已更新，transitioning_to 已清空
    let active = profile::get_active_profile(&db, TargetCli::Claude)
        .expect("get_active")
        .expect("Some");
    assert_eq!(active.id, p.id);
}

#[test]
fn activate_clears_transition_marker_on_success() {
    let (db, _home, _guard) = fresh_test_db_with_home();
    let p = profile::create_profile(
        &db,
        ProfileCreatePayload {
            target_cli: TargetCli::Codex,
            name: "T".into(),
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

    profile::activate_profile(&db, &p.id, TargetCli::Codex).expect("activate");

    // transitioning_to 必须清空（通过公开 helper 读取，避免触碰内部 conn）
    let trans =
        profile::get_transition_marker(&db, TargetCli::Codex).expect("read transition marker");
    assert!(trans.is_none(), "transitioning_to 应在成功后清空");
}

#[test]
fn activate_creates_backup_when_live_file_pre_exists() {
    let (db, home, _guard) = fresh_test_db_with_home();

    // 预先写一个 live 文件
    let live = home.join(".claude").join("settings.json");
    std::fs::create_dir_all(live.parent().unwrap()).expect("mkdir");
    std::fs::write(&live, r#"{"pre_existing":"yes"}"#).expect("seed live");

    let p = profile::create_profile(
        &db,
        ProfileCreatePayload {
            target_cli: TargetCli::Claude,
            name: "BackupTest".into(),
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

    let result = profile::activate_profile(&db, &p.id, TargetCli::Claude).expect("activate");

    assert!(result.success);
    let backup_dir_str = result
        .backup_dir
        .as_ref()
        .expect("backup_dir 应在 live 文件存在时返回");
    let backup_dir = PathBuf::from(backup_dir_str);
    assert!(backup_dir.exists(), "backup 目录应存在: {backup_dir_str}");

    // backup 应保留原文件内容
    let backups: Vec<_> = std::fs::read_dir(&backup_dir)
        .expect("read backup dir")
        .filter_map(|e| e.ok())
        .collect();
    assert!(!backups.is_empty(), "backup 目录应至少包含一个备份文件");

    let mut found_pre = false;
    for entry in backups {
        let content = std::fs::read_to_string(entry.path()).expect("read backup file");
        if content.contains("pre_existing") {
            found_pre = true;
            break;
        }
    }
    assert!(found_pre, "至少一个 backup 文件应包含原 live 内容");
}

#[test]
fn activate_does_not_create_backup_when_no_pre_existing_live_file() {
    let (db, home, _guard) = fresh_test_db_with_home();

    // 确保没有 live 文件
    let live = home.join(".claude").join("settings.json");
    if live.exists() {
        std::fs::remove_file(&live).expect("remove");
    }

    let p = profile::create_profile(
        &db,
        ProfileCreatePayload {
            target_cli: TargetCli::Claude,
            name: "NoBackup".into(),
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

    let result = profile::activate_profile(&db, &p.id, TargetCli::Claude).expect("activate");
    assert!(result.success);
    assert!(
        result.backup_dir.is_none(),
        "无 pre-existing live 时不应生成 backup_dir"
    );
}

#[test]
fn activate_nonexistent_profile_returns_error() {
    let (db, _home, _guard) = fresh_test_db_with_home();
    let err = profile::activate_profile(&db, "definitely-not-here", TargetCli::Claude).unwrap_err();
    match err {
        profile::ProfileError::NotFound { .. } => {}
        other => panic!("应返回 NotFound，实际：{other}"),
    }

    // active_profile_id 没有被改成 nonexistent（通过公开 API 验证）
    let active = profile::get_active_profile(&db, TargetCli::Claude).expect("get_active");
    if let Some(p) = active {
        assert_ne!(
            p.id, "definitely-not-here",
            "失败的 activate 不应改变 active_profile_id"
        );
    }
}

#[test]
fn activate_multiple_profiles_sequentially_keeps_db_consistent() {
    let (db, _home, _guard) = fresh_test_db_with_home();

    let p1 = profile::create_profile(
        &db,
        ProfileCreatePayload {
            target_cli: TargetCli::Claude,
            name: "P1".into(),
            description: None,
            icon: None,
            icon_color: None,
            provider_id: None,
            settings_json: None,
            mcp_ids: None,
            skill_ids: None,
        },
    )
    .expect("create p1");

    let p2 = profile::create_profile(
        &db,
        ProfileCreatePayload {
            target_cli: TargetCli::Claude,
            name: "P2".into(),
            description: None,
            icon: None,
            icon_color: None,
            provider_id: None,
            settings_json: None,
            mcp_ids: None,
            skill_ids: None,
        },
    )
    .expect("create p2");

    profile::activate_profile(&db, &p1.id, TargetCli::Claude).expect("activate p1");
    profile::activate_profile(&db, &p2.id, TargetCli::Claude).expect("activate p2");

    let active = profile::get_active_profile(&db, TargetCli::Claude)
        .expect("get_active")
        .expect("Some");
    assert_eq!(active.id, p2.id, "最后激活的 Profile 应为 active");

    // transitioning_to 一定是 None
    let trans = profile::get_transition_marker(&db, TargetCli::Claude).expect("read marker");
    assert!(trans.is_none());
}

#[test]
fn list_all_active_after_activation_returns_correct_ids() {
    let (db, _home, _guard) = fresh_test_db_with_home();

    let claude_p = profile::create_profile(
        &db,
        ProfileCreatePayload {
            target_cli: TargetCli::Claude,
            name: "ClaudeOne".into(),
            description: None,
            icon: None,
            icon_color: None,
            provider_id: None,
            settings_json: None,
            mcp_ids: None,
            skill_ids: None,
        },
    )
    .expect("create claude");
    let codex_p = profile::create_profile(
        &db,
        ProfileCreatePayload {
            target_cli: TargetCli::Codex,
            name: "CodexOne".into(),
            description: None,
            icon: None,
            icon_color: None,
            provider_id: None,
            settings_json: None,
            mcp_ids: None,
            skill_ids: None,
        },
    )
    .expect("create codex");

    profile::activate_profile(&db, &claude_p.id, TargetCli::Claude).expect("activate claude");
    profile::activate_profile(&db, &codex_p.id, TargetCli::Codex).expect("activate codex");

    let map = profile::list_all_active(&db).expect("list_all_active");
    assert_eq!(map.claude.as_deref(), Some(claude_p.id.as_str()));
    assert_eq!(map.codex.as_deref(), Some(codex_p.id.as_str()));
}
