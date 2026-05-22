//! 集成测试：cc-launcher v10 → v11 Schema 迁移
//!
//! 验收点：
//! 1. 从模拟 v10 schema 升级到 v11 后，4 张 Profile 表均存在
//! 2. 每个 CLI 获得一个 `default-<cli>` 内建 Profile（is_builtin=1）
//! 3. v10 上 `enabled_<cli>=1` 的 MCP/Skill 行被回填到对应 Default Profile 的关联表
//! 4. `cli_state.active_profile_id` 指向 Default Profile
//! 5. 重复运行迁移是 no-op（幂等性）

use rusqlite::Connection;

use cc_switch_lib::Database;

mod support;

/// 构造一个完整的 v10 schema 快照（来自 schema.rs::create_tables_on_conn + user_version=10）
fn seed_v10_schema(conn: &Connection) {
    // 重要：v10 schema 必须能跑过 create_tables_on_conn（其中包含 ALTER 容错），
    // 测试里直接调用现有 create_tables_on_conn 再手动写 user_version=10。
    Database::create_tables_on_conn(conn).expect("create v10 tables");
    // 模拟"未迁移 v11"的状态：rollback profiles 表（如果存在）+ user_version=10
    let _ = conn.execute("DROP TABLE IF EXISTS profile_skill", []);
    let _ = conn.execute("DROP TABLE IF EXISTS profile_mcp", []);
    let _ = conn.execute("DROP TABLE IF EXISTS cli_state", []);
    let _ = conn.execute("DROP TABLE IF EXISTS profiles", []);
    Database::set_user_version(conn, 10).expect("set v10");
}

#[test]
fn migration_v10_to_v11_creates_all_profile_tables() {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .expect("fk on");

    seed_v10_schema(&conn);
    assert_eq!(Database::get_user_version(&conn).unwrap(), 10);

    Database::apply_schema_migrations_on_conn(&conn).expect("apply migration");

    assert_eq!(Database::get_user_version(&conn).unwrap(), 11);

    // 4 张表必须存在
    for t in ["profiles", "profile_mcp", "profile_skill", "cli_state"] {
        assert!(
            Database::table_exists(&conn, t).expect("table_exists"),
            "{t} 表应在 v11 后存在"
        );
    }
}

#[test]
fn migration_backfills_default_profile_for_each_cli() {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .expect("fk on");
    seed_v10_schema(&conn);
    Database::apply_schema_migrations_on_conn(&conn).expect("migrate");

    // 每个 CLI 都应有一个 default-<cli> 内建 Profile
    for cli in ["claude", "codex", "gemini", "opencode", "hermes"] {
        let profile_id = format!("default-{cli}");
        let is_builtin: i64 = conn
            .query_row(
                "SELECT is_builtin FROM profiles WHERE id = ?1 AND target_cli = ?2",
                rusqlite::params![&profile_id, cli],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| panic!("default-{cli} 必须存在"));
        assert_eq!(is_builtin, 1, "default-{cli} 应为 is_builtin=1");
    }

    // cli_state 必须激活了对应 Default Profile
    for cli in ["claude", "codex", "gemini", "opencode", "hermes"] {
        let active: Option<String> = conn
            .query_row(
                "SELECT active_profile_id FROM cli_state WHERE target_cli = ?1",
                rusqlite::params![cli],
                |r| r.get(0),
            )
            .expect("query cli_state");
        assert_eq!(active.as_deref(), Some(format!("default-{cli}").as_str()));
    }
}

#[test]
fn migration_backfills_enabled_mcp_into_default_profile() {
    let _g = support::test_mutex().lock().unwrap();
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .expect("fk on");
    seed_v10_schema(&conn);

    // 在 v10 阶段插入一个 MCP，标记为 claude 启用、codex 禁用
    conn.execute(
        "INSERT INTO mcp_servers (id, name, server_config, enabled_claude, enabled_codex)
         VALUES ('mcp-1', 'Test MCP', '{}', 1, 0)",
        [],
    )
    .expect("seed mcp");

    Database::apply_schema_migrations_on_conn(&conn).expect("migrate");

    // claude 的 default profile 应包含 mcp-1
    let claude_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM profile_mcp
             WHERE profile_id = 'default-claude' AND target_cli = 'claude' AND mcp_id = 'mcp-1'",
            [],
            |r| r.get(0),
        )
        .expect("query claude");
    assert_eq!(claude_count, 1, "claude default profile 应包含 enabled MCP");

    // codex 的 default profile 不应包含 mcp-1（enabled_codex=0）
    let codex_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM profile_mcp
             WHERE profile_id = 'default-codex' AND target_cli = 'codex' AND mcp_id = 'mcp-1'",
            [],
            |r| r.get(0),
        )
        .expect("query codex");
    assert_eq!(
        codex_count, 0,
        "codex default profile 不应包含 disabled MCP"
    );
}

#[test]
fn migration_is_idempotent() {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .expect("fk on");
    seed_v10_schema(&conn);

    Database::apply_schema_migrations_on_conn(&conn).expect("first migrate");
    let first_default_claude_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM profiles WHERE id = 'default-claude'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(first_default_claude_count, 1);

    // 二次 migrate（应为 no-op）
    Database::apply_schema_migrations_on_conn(&conn).expect("second migrate idempotent");
    let after_default_claude_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM profiles WHERE id = 'default-claude'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        after_default_claude_count, 1,
        "重复迁移不应产生重复 default Profile"
    );

    assert_eq!(Database::get_user_version(&conn).unwrap(), 11);
}

#[test]
fn migration_with_no_active_provider_creates_profile_with_null_provider() {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .expect("fk on");
    seed_v10_schema(&conn);

    // 不插入任何 provider，直接 migrate
    Database::apply_schema_migrations_on_conn(&conn).expect("migrate");

    let provider_id: Option<String> = conn
        .query_row(
            "SELECT provider_id FROM profiles WHERE id = 'default-claude'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert!(provider_id.is_none(), "无 active provider 时应为 NULL");
}

#[test]
fn migration_preserves_active_provider_link() {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .expect("fk on");
    seed_v10_schema(&conn);

    // 在 v10 阶段插入两个 provider，其中 anthropic-1 是当前激活
    conn.execute(
        "INSERT INTO providers
            (id, app_type, name, settings_config, is_current)
         VALUES ('anthropic-1', 'claude', 'Anthropic', '{}', 1),
                ('other-1',     'claude', 'Other',     '{}', 0)",
        [],
    )
    .expect("seed providers");

    Database::apply_schema_migrations_on_conn(&conn).expect("migrate");

    let provider_id: Option<String> = conn
        .query_row(
            "SELECT provider_id FROM profiles WHERE id = 'default-claude'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(
        provider_id.as_deref(),
        Some("anthropic-1"),
        "default-claude 的 provider_id 应指向 is_current=1 的 provider"
    );
}
