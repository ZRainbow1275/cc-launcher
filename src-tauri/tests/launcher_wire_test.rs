//! D-5 wire-DTO round-trip tests.
//!
//! These tests guard the public contract surface at the Tauri cmd boundary
//! (see `commands::launcher_wire`). The internal service-layer types and
//! `tests/launcher_test.rs` remain unchanged — those assert safety invariants
//! on the internal `SafetySummary`. This file asserts the wire `JSON` keys
//! the frontend zod schemas (`src/lib/api/contracts.ts` and
//! `src/lib/api/mock/launcher.ts::SafetySummary`) expect.

#![allow(clippy::await_holding_lock)]

use std::path::PathBuf;
use std::sync::Arc;

use cc_switch_lib::sandbox;
use cc_switch_lib::services::installer::cli_install::TargetCli;
use cc_switch_lib::services::launcher_service::{LauncherError, LauncherService, SafetySummary};
use cc_switch_lib::Database;

// Re-export the wire DTOs as `pub` from the commands module crate-wide
// so integration tests can construct them. The module is registered in
// `src/commands/mod.rs`; reaching it from an integration test requires the
// `cc_switch_lib::launcher_wire` path.

mod support;

fn fresh_test_db_with_home() -> (Arc<Database>, std::sync::MutexGuard<'static, ()>) {
    let guard = support::test_mutex().lock().expect("acquire test mutex");
    support::ensure_test_home();
    support::reset_test_fs();
    let db = Arc::new(Database::init().expect("init db"));
    (db, guard)
}

// ============================================================================
// 1. WireSafetySummary serializes to camelCase keys matching
//    `src/lib/api/mock/launcher.ts::SafetySummary`.
// ============================================================================

#[tokio::test]
async fn wire_safety_summary_round_trip_has_camelcase_keys_and_active_redlines() {
    let (db, _guard) = fresh_test_db_with_home();

    // Drive the real service path to get a valid `SafetySummary`.
    let summary: SafetySummary =
        LauncherService::get_safety_summary(db.clone(), TargetCli::Claude, None)
            .await
            .expect("get_safety_summary with default active profile");

    // Project to the wire DTO. `l1_active_count` is computed independently of
    // the wire shape — here we re-derive it the same way the cmd handler does.
    let l1_active = sandbox::get_l1_rules(&db)
        .map(|rs| rs.into_iter().filter(|r| r.enabled).count())
        .unwrap_or(0);
    let wire = cc_switch_lib::launcher_wire::WireSafetySummary::project(
        summary,
        "default-claude".to_string(),
        TargetCli::Claude,
        l1_active,
    );

    let json = serde_json::to_value(&wire).expect("serialize WireSafetySummary");
    let obj = json
        .as_object()
        .expect("WireSafetySummary must serialize to a JSON object");

    // Contract camelCase keys per src/lib/api/mock/launcher.ts::SafetySummary.
    for key in [
        "profileId",
        "targetCli",
        "flags",
        "cwd",
        "cwdDisplay",
        "l1ActiveCount",
        "l2RedlineCount",
    ] {
        assert!(
            obj.contains_key(key),
            "WireSafetySummary JSON missing camelCase key `{key}`, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
    }

    // No snake_case leakage from internal shapes.
    for legacy in [
        "profile_id",
        "target_cli",
        "cwd_display",
        "l1_active_count",
        "l2_redline_count",
        "sandbox_level",
        "workdir",
        "flags_applied",
        "env_keys_set",
        "redlines_active",
    ] {
        assert!(
            !obj.contains_key(legacy),
            "WireSafetySummary JSON must not expose legacy key `{legacy}`"
        );
    }

    // The compile-time L2 redline registry is non-empty, so the wire surrogate
    // for `redlines_active: true` is always positive.
    let l2 = obj
        .get("l2RedlineCount")
        .and_then(|v| v.as_u64())
        .expect("l2RedlineCount must be a non-negative integer");
    assert!(
        l2 > 0,
        "wire `l2RedlineCount` must remain non-zero (redlines active), got: {l2}"
    );

    // targetCli is lowercase per contracts.ts (`"claude" | "codex"`).
    let target = obj
        .get("targetCli")
        .and_then(|v| v.as_str())
        .expect("targetCli must be a string");
    assert!(
        matches!(target, "claude" | "codex"),
        "targetCli wire value must be lowercase enum, got: {target}"
    );

    // Default Claude flags include `--add-dir` for the workdir — this is the
    // SAME safety invariant the legacy `tests/launcher_test.rs` asserts on
    // `summary.flags_applied`; here we re-assert at the wire layer.
    let flags = obj
        .get("flags")
        .and_then(|v| v.as_array())
        .expect("flags must be an array");
    assert!(
        flags
            .iter()
            .any(|f| f.as_str().map(|s| s.starts_with("--add-dir")).unwrap_or(false)),
        "wire `flags` must include --add-dir for Claude, got: {flags:?}"
    );
    // And must NEVER include the bypass flag by default.
    assert!(
        !flags.iter().any(|f| f
            .as_str()
            .map(|s| s.contains("dangerously-skip-permissions"))
            .unwrap_or(false)),
        "wire `flags` must not contain --dangerously-skip-permissions, got: {flags:?}"
    );
}

// ============================================================================
// 2. WireTerminalInfo serializes to the camelCase + kebab-id contract.
// ============================================================================

#[test]
fn wire_terminal_info_round_trip_has_kebab_id_and_camelcase_keys() {
    use cc_switch_lib::launcher_wire::WireTerminalInfo;
    use cc_switch_lib::services::launcher_service::{TerminalInfo, TerminalKind};

    let info = TerminalInfo {
        kind: TerminalKind::WindowsTerminal,
        binary_path: PathBuf::from("C:/Windows/System32/wt.exe"),
        display_name: "Windows Terminal".into(),
        is_default: true,
    };
    let wire: WireTerminalInfo = info.into();
    let json = serde_json::to_value(&wire).expect("serialize WireTerminalInfo");
    let obj = json.as_object().expect("JSON object");

    for key in ["id", "kind", "displayName", "path", "installed", "isDefault"] {
        assert!(
            obj.contains_key(key),
            "WireTerminalInfo missing key `{key}`, got: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
    }
    assert_eq!(
        obj.get("id").and_then(|v| v.as_str()),
        Some("wt"),
        "wire `id` must match contracts.ts TerminalKind (`wt` for Windows Terminal)"
    );
    assert_eq!(
        obj.get("kind").and_then(|v| v.as_str()),
        Some("wt"),
        "wire `kind` enum must serialize to the kebab id"
    );
    assert_eq!(
        obj.get("installed").and_then(|v| v.as_bool()),
        Some(true),
        "wire `installed` must be true for every emitted candidate"
    );
}

#[test]
fn wire_terminal_kind_serializes_per_contracts() {
    use cc_switch_lib::launcher_wire::WireTerminalKind;

    // Each variant must round-trip to its `contracts.ts::TerminalKind` value.
    let cases: &[(WireTerminalKind, &str)] = &[
        (WireTerminalKind::Wt, "\"wt\""),
        (WireTerminalKind::Cmd, "\"cmd\""),
        (WireTerminalKind::Powershell, "\"powershell\""),
        (WireTerminalKind::TerminalApp, "\"terminal-app\""),
        (WireTerminalKind::Iterm2, "\"iterm2\""),
        (WireTerminalKind::GnomeTerminal, "\"gnome-terminal\""),
        (WireTerminalKind::Konsole, "\"konsole\""),
        (WireTerminalKind::Xterm, "\"xterm\""),
    ];
    for (variant, expected) in cases {
        let json = serde_json::to_string(variant).expect("serialize WireTerminalKind");
        assert_eq!(
            &json, expected,
            "WireTerminalKind variant must serialize to contracts.ts wire value"
        );
    }
}

// ============================================================================
// 3. LauncherError → TypedError projection emits the codes the frontend
//    `src/lib/api/mock/fixtures/i18n.ts` consumes.
// ============================================================================

#[test]
fn typed_error_codes_match_i18n_fixture_constants() {
    use cc_switch_lib::launcher_wire::typed_error_from;

    let cases: &[(LauncherError, &str)] = &[
        (LauncherError::NodeMissing, "NODE_MISSING"),
        (
            LauncherError::CliMissing {
                cli: "claude".into(),
            },
            "CLI_MISSING",
        ),
        (
            LauncherError::ProfileInvalid {
                reason: "x".into(),
            },
            "PROFILE_NOT_FOUND",
        ),
        (LauncherError::TerminalNotFound, "NO_TERMINAL_AVAILABLE"),
        (
            LauncherError::WorkdirCreateFailed {
                path: "/tmp".into(),
                message: "denied".into(),
            },
            "WORKDIR_CREATE_FAILED",
        ),
        (
            LauncherError::SpawnFailed {
                message: "spawn failed".into(),
            },
            "SPAWN_FAILED",
        ),
        (
            LauncherError::Unknown {
                message: "x".into(),
            },
            "LAUNCH_FAILED",
        ),
    ];
    for (err, expected_code) in cases {
        let projected = typed_error_from(err);
        assert_eq!(
            &projected.code, expected_code,
            "TypedError.code for {err:?} must match the i18n fixture constant"
        );
        // LocalizedString must carry all three locales — the frontend i18next
        // layer falls back to en when the active locale's translation is
        // missing, but the contract requires the trio.
        assert!(
            !projected.message.zh.is_empty(),
            "TypedError.message.zh must be non-empty"
        );
        assert!(
            !projected.message.en.is_empty(),
            "TypedError.message.en must be non-empty"
        );
        assert!(
            !projected.message.ja.is_empty(),
            "TypedError.message.ja must be non-empty"
        );
    }
}

// ============================================================================
// 4. WireLaunchResult: failure envelope shape matches contracts.ts::LaunchResult
//    (the `success`-tagged form with inline error).
// ============================================================================

#[test]
fn wire_launch_result_failure_envelope_shape() {
    use cc_switch_lib::launcher_wire::WireLaunchResult;

    let res = WireLaunchResult::from_error(
        LauncherError::ProfileInvalid {
            reason: "does-not-exist".into(),
        },
        "p1".into(),
        TargetCli::Claude,
        "wt".into(),
        "C:/Users/test/cc-launcher-projects/p1".into(),
        "2026-05-23T00:00:00Z".into(),
    );
    let json = serde_json::to_value(&res).expect("serialize WireLaunchResult");
    let obj = json.as_object().expect("JSON object");

    for key in [
        "success",
        "profileId",
        "targetCli",
        "terminalId",
        "cwd",
        "launchedAt",
        "error",
    ] {
        assert!(
            obj.contains_key(key),
            "WireLaunchResult missing key `{key}`, got: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
    }
    assert_eq!(obj.get("success").and_then(|v| v.as_bool()), Some(false));
    assert!(
        obj.get("pid").is_none(),
        "pid must be omitted on failure (skip_serializing_if)"
    );
    let err = obj
        .get("error")
        .and_then(|v| v.as_object())
        .expect("error must be an object");
    assert_eq!(err.get("code").and_then(|v| v.as_str()), Some("PROFILE_NOT_FOUND"));
    let msg = err
        .get("message")
        .and_then(|v| v.as_object())
        .expect("error.message must be a LocalizedString object");
    for lang in ["zh", "en", "ja"] {
        assert!(
            msg.get(lang).and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false),
            "error.message.{lang} must be non-empty"
        );
    }
}

// ============================================================================
// 5. Regression: `launched_at` must be a Z-form ISO-8601 UTC string with
//    millisecond precision. The `commands::launcher::start_cli` impl builds
//    this via `chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)`.
//    Hardcoded timestamps in the other tests don't exercise that call site, so
//    this test runs the construction inline and asserts its shape.
// ============================================================================

#[test]
fn launched_at_uses_z_form_iso8601_millis() {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$").unwrap();
    assert!(
        re.is_match(&now),
        "launched_at must be Z-form ISO-8601 millis, got: {now}"
    );
    assert!(now.ends_with('Z'), "must end with Z, got: {now}");
    assert!(!now.contains('+'), "must not contain offset, got: {now}");
}
