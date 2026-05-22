//! 沙盒审计日志的端到端集成测试。
//!
//! 验证：
//! 1. NDJSON 行追加，行尾 `\n`，每行可独立 parse。
//! 2. 顺序保留（写入顺序 = 读出顺序）。
//! 3. 多种 event_type 共存时 `event_type` 过滤生效。

mod support;

use cc_switch_lib::sandbox::{self, audit, AuditEntry, AuditEventType, AuditQueryOpts};
use serial_test::serial;
use std::fs;

#[test]
#[serial]
fn audit_log_writes_five_entries_and_reads_back_in_order() {
    support::ensure_test_home();
    support::reset_test_fs();

    // 清掉测试 home 下可能残留的 audit.log
    let path = audit::audit_log_path();
    if path.exists() {
        let _ = fs::remove_file(&path);
    }

    let entries = vec![
        AuditEntry::l2_block("disk_wipe.rm_root", "rm -rf /"),
        AuditEntry::l1_block("L1.sudo_runas", "sudo apt install"),
        AuditEntry::l1_unlock("L1.sudo_runas"),
        AuditEntry::level_change("medium"),
        AuditEntry::l2_block("hosts.unix_write", "echo evil >> /etc/hosts"),
    ];

    for e in &entries {
        audit::append(e);
    }

    // 直接读文件验证 NDJSON 行格式
    let raw = fs::read_to_string(&path).expect("audit log must exist");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 5, "should be 5 NDJSON lines");

    for line in &lines {
        // 每行必须能独立 parse 成 AuditEntry
        let parsed: AuditEntry = serde_json::from_str(line).expect("parse NDJSON line");
        assert!(parsed.timestamp.contains('T') || parsed.timestamp.contains('-'));
    }

    // 通过公共 API 读回 —— 顺序应保留
    let all = sandbox::get_audit_log(&AuditQueryOpts::default());
    assert_eq!(all.len(), 5);
    assert_eq!(all[0].rule_id.as_deref(), Some("disk_wipe.rm_root"));
    assert_eq!(all[1].rule_id.as_deref(), Some("L1.sudo_runas"));
    assert_eq!(all[2].event_type, AuditEventType::L1Unlock);
    assert_eq!(all[3].event_type, AuditEventType::LevelChange);
    assert_eq!(all[4].rule_id.as_deref(), Some("hosts.unix_write"));
}

#[test]
#[serial]
fn audit_log_filter_by_event_type() {
    support::ensure_test_home();
    support::reset_test_fs();

    let path = audit::audit_log_path();
    if path.exists() {
        let _ = fs::remove_file(&path);
    }

    audit::append(&AuditEntry::l2_block("disk_wipe.rm_root", "rm -rf /"));
    audit::append(&AuditEntry::l1_unlock("L1.sudo_runas"));
    audit::append(&AuditEntry::l2_block("hosts.unix_write", "tee /etc/hosts"));
    audit::append(&AuditEntry::level_change("strict"));

    let only_l2 = sandbox::get_audit_log(&AuditQueryOpts {
        limit: None,
        event_type: Some(AuditEventType::L2Block),
    });
    assert_eq!(only_l2.len(), 2);
    for e in &only_l2 {
        assert_eq!(e.event_type, AuditEventType::L2Block);
    }
}

#[test]
#[serial]
fn audit_log_limit_returns_tail() {
    support::ensure_test_home();
    support::reset_test_fs();

    let path = audit::audit_log_path();
    if path.exists() {
        let _ = fs::remove_file(&path);
    }

    for i in 0..10 {
        audit::append(&AuditEntry::l2_block(
            format!("rule.{i}"),
            format!("cmd {i}"),
        ));
    }
    let tail = sandbox::get_audit_log(&AuditQueryOpts {
        limit: Some(3),
        event_type: None,
    });
    assert_eq!(tail.len(), 3);
    assert_eq!(tail[0].rule_id.as_deref(), Some("rule.7"));
    assert_eq!(tail[2].rule_id.as_deref(), Some("rule.9"));
}

#[test]
#[serial]
fn audit_log_check_redline_match_records_event() {
    support::ensure_test_home();
    support::reset_test_fs();

    let path = audit::audit_log_path();
    if path.exists() {
        let _ = fs::remove_file(&path);
    }

    // 命中应自动写入审计
    let m = sandbox::check_redline_match("rm -rf /");
    assert!(m.is_some());
    let log = sandbox::get_audit_log(&AuditQueryOpts::default());
    assert!(!log.is_empty(), "L2 block should be recorded");
    assert_eq!(log.last().unwrap().event_type, AuditEventType::L2Block);

    // 未命中不应产生事件
    let before = log.len();
    let m = sandbox::check_redline_match("ls -la");
    assert!(m.is_none());
    let after = sandbox::get_audit_log(&AuditQueryOpts::default()).len();
    assert_eq!(after, before, "safe command must not record audit");
}
