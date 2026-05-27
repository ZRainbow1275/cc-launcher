//! cc-launcher D-4: onboarding + settings + install_git command coverage.
//!
//! Each test isolates state via a fresh `AppState` and the shared
//! `test_mutex` to prevent concurrent test bleed-through.

use cc_switch_lib::services::installer::InstallerSourceConfig;
use cc_switch_lib::{
    install_git_test_hook, onboarding_complete_test_hook, onboarding_get_state_test_hook,
    settings_get_installer_source_config_test_hook, settings_get_locale_test_hook,
    settings_get_ui_mode_test_hook, settings_reset_installer_source_config_test_hook,
    settings_set_installer_source_config_test_hook, settings_set_locale_test_hook,
    settings_set_ui_mode_test_hook, Locale, OnboardingAnswers, UiMode,
};

#[path = "support.rs"]
mod support;
use support::{create_test_state, ensure_test_home, reset_test_fs, test_mutex};

#[test]
fn onboarding_get_state_returns_default_when_unset() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let state = create_test_state().expect("create test state");

    let result = onboarding_get_state_test_hook(&state).expect("get state");

    assert!(!result.completed, "default completed must be false");
    assert!(
        result.completed_at.is_none(),
        "default completed_at must be None"
    );
    assert!(result.answers.is_none(), "default answers must be None");
}

#[test]
fn onboarding_complete_persists_completion_and_answers() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let state = create_test_state().expect("create test state");

    let answers = OnboardingAnswers {
        locale: Locale::Zh,
        ui_mode: UiMode::Expert,
        enable_sandbox: true,
        accepted_redlines: true,
        preferred_cli: Some("claude".to_string()),
    };

    let op = onboarding_complete_test_hook(&state, Some(answers.clone())).expect("complete");
    assert!(op.success, "complete must report success=true");

    let after = onboarding_get_state_test_hook(&state).expect("get state");
    assert!(after.completed, "completed should now be true");
    let timestamp = after.completed_at.expect("completed_at must be set");
    assert!(
        timestamp.contains('T') || timestamp.contains('-'),
        "completed_at should be ISO-8601-like, got {timestamp}"
    );

    let stored = after.answers.expect("answers must be persisted");
    assert_eq!(stored.locale, Locale::Zh);
    assert_eq!(stored.ui_mode, UiMode::Expert);
    assert!(stored.enable_sandbox);
    assert!(stored.accepted_redlines);
    assert_eq!(stored.preferred_cli.as_deref(), Some("claude"));

    // onboarding.complete must also mirror locale + ui_mode into their own keys
    let mirrored_mode = settings_get_ui_mode_test_hook(&state).expect("get ui mode");
    assert_eq!(mirrored_mode, UiMode::Expert);
    let mirrored_locale = settings_get_locale_test_hook(&state).expect("get locale");
    assert_eq!(mirrored_locale, Locale::Zh);
}

#[test]
fn settings_ui_mode_round_trip() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let state = create_test_state().expect("create test state");

    // Default before any write must be "novice"
    let initial = settings_get_ui_mode_test_hook(&state).expect("get ui mode");
    assert_eq!(initial, UiMode::Novice, "default ui_mode must be novice");

    let op = settings_set_ui_mode_test_hook(&state, UiMode::Expert).expect("set ui mode");
    assert!(op.success);
    let after = settings_get_ui_mode_test_hook(&state).expect("get ui mode");
    assert_eq!(after, UiMode::Expert);
}

#[test]
fn settings_locale_default_is_en_and_round_trips() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let state = create_test_state().expect("create test state");

    let initial = settings_get_locale_test_hook(&state).expect("get locale");
    assert_eq!(
        initial,
        Locale::En,
        "default locale must be en (no sys-locale dep)"
    );

    let op = settings_set_locale_test_hook(&state, Locale::Ja).expect("set locale");
    assert!(op.success);
    let after = settings_get_locale_test_hook(&state).expect("get locale");
    assert_eq!(after, Locale::Ja);
}

#[test]
fn install_git_returns_localized_stub() {
    let _guard = test_mutex().lock().expect("acquire test mutex");

    let op = install_git_test_hook().expect("install_git stub");
    assert!(op.success, "install_git stub must report success=true");
    let message = op.message.expect("install_git must include a message");
    assert!(!message.zh.is_empty(), "zh message must be populated");
    assert!(!message.en.is_empty(), "en message must be populated");
    assert!(!message.ja.is_empty(), "ja message must be populated");
    assert!(op.error_code.is_none(), "no error_code expected");
}

#[test]
fn installer_source_config_round_trips_and_resets() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let state = create_test_state().expect("create test state");

    let initial =
        settings_get_installer_source_config_test_hook(&state).expect("get source config");
    assert!(initial.is_empty(), "default source config should be empty");

    let config = InstallerSourceConfig {
        npm_registry: Some("https://vps.example.com/npm/".into()),
        node_dist_mirror: Some("https://vps.example.com/node/".into()),
        git_for_windows_mirror: Some("https://vps.example.com/git/".into()),
    };

    let op = settings_set_installer_source_config_test_hook(&state, config).expect("set config");
    assert!(op.success);

    let stored =
        settings_get_installer_source_config_test_hook(&state).expect("get stored source config");
    assert_eq!(
        stored.npm_registry.as_deref(),
        Some("https://vps.example.com/npm")
    );
    assert_eq!(
        stored.node_dist_mirror.as_deref(),
        Some("https://vps.example.com/node")
    );
    assert_eq!(
        stored.git_for_windows_mirror.as_deref(),
        Some("https://vps.example.com/git")
    );

    let reset =
        settings_reset_installer_source_config_test_hook(&state).expect("reset source config");
    assert!(reset.success);
    let after_reset =
        settings_get_installer_source_config_test_hook(&state).expect("get reset source config");
    assert!(after_reset.is_empty());
}
