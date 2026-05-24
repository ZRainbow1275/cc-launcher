//! cc-launcher Onboarding & Settings (D-4 / Phase D).
//!
//! Backend counterparts for the frontend mocks at
//! `src/lib/api/mock/onboarding.ts` and `src/lib/api/mock/settings.ts`.
//! All persistence flows through the existing `settings` key/value table
//! (see `database/dao/settings.rs`). `install_git` is a stable stub —
//! the frontend should route to `apply_probe_fix({kind: "installGit"})`
//! when real auto-install is needed.

#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::store::AppState;
use crate::types::{LocalizedString, OperationResult};

const ONBOARDING_COMPLETED_KEY: &str = "cc_launcher.onboarding.completed";
const ONBOARDING_COMPLETED_AT_KEY: &str = "cc_launcher.onboarding.completed_at";
const ONBOARDING_ANSWERS_KEY: &str = "cc_launcher.onboarding.answers";
const UI_MODE_KEY: &str = "cc_launcher.ui_mode";
const LOCALE_KEY: &str = "cc_launcher.locale";

const DEFAULT_UI_MODE: UiMode = UiMode::Novice;
const DEFAULT_LOCALE: Locale = Locale::En;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PreferredCli {
    Claude,
    Codex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingAnswers {
    pub locale: Locale,
    pub ui_mode: UiMode,
    pub enable_sandbox: bool,
    pub accepted_redlines: bool,
    // TODO(E1-M5): tighten to Option<PreferredCli> once the protected
    // tests/onboarding_settings_test.rs is unfrozen — currently it constructs
    // this field with a raw string literal which would fail to compile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_cli: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    pub completed: bool,
    pub completed_at: Option<String>,
    pub answers: Option<OnboardingAnswers>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UiMode {
    Novice,
    Expert,
}

impl UiMode {
    fn as_str(&self) -> &'static str {
        match self {
            UiMode::Novice => "novice",
            UiMode::Expert => "expert",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "novice" => Some(UiMode::Novice),
            "expert" => Some(UiMode::Expert),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    Zh,
    En,
    Ja,
}

impl Locale {
    fn as_str(&self) -> &'static str {
        match self {
            Locale::Zh => "zh",
            Locale::En => "en",
            Locale::Ja => "ja",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "zh" => Some(Locale::Zh),
            "en" => Some(Locale::En),
            "ja" => Some(Locale::Ja),
            _ => None,
        }
    }
}

// ---------- internal helpers (operate on AppState directly) ----------

fn onboarding_get_state_internal(state: &AppState) -> Result<OnboardingState, AppError> {
    let completed = state.db.get_bool_flag(ONBOARDING_COMPLETED_KEY)?;
    let completed_at = state.db.get_setting(ONBOARDING_COMPLETED_AT_KEY)?;
    let answers = match state.db.get_setting(ONBOARDING_ANSWERS_KEY)? {
        Some(json) => serde_json::from_str::<OnboardingAnswers>(&json).ok(),
        None => None,
    };
    Ok(OnboardingState {
        completed,
        completed_at,
        answers,
    })
}

fn onboarding_complete_internal(
    state: &AppState,
    answers: Option<OnboardingAnswers>,
) -> Result<OperationResult, AppError> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    state.db.set_setting(ONBOARDING_COMPLETED_KEY, "true")?;
    state.db.set_setting(ONBOARDING_COMPLETED_AT_KEY, &now)?;

    if let Some(answers) = answers {
        let json = serde_json::to_string(&answers).map_err(|e| {
            AppError::Database(format!("序列化 onboarding answers 失败: {e}"))
        })?;
        state.db.set_setting(ONBOARDING_ANSWERS_KEY, &json)?;
        // Mirror locale/ui_mode into their own keys so settings_get_* sees them.
        state.db.set_setting(UI_MODE_KEY, answers.ui_mode.as_str())?;
        state.db.set_setting(LOCALE_KEY, answers.locale.as_str())?;
    }

    Ok(OperationResult::ok())
}

fn settings_get_ui_mode_internal(state: &AppState) -> Result<UiMode, AppError> {
    match state.db.get_setting(UI_MODE_KEY)? {
        Some(value) => Ok(UiMode::parse(&value).unwrap_or(DEFAULT_UI_MODE)),
        None => Ok(DEFAULT_UI_MODE),
    }
}

fn settings_set_ui_mode_internal(
    state: &AppState,
    mode: UiMode,
) -> Result<OperationResult, AppError> {
    state.db.set_setting(UI_MODE_KEY, mode.as_str())?;
    Ok(OperationResult::ok())
}

fn settings_get_locale_internal(state: &AppState) -> Result<Locale, AppError> {
    match state.db.get_setting(LOCALE_KEY)? {
        Some(value) => Ok(Locale::parse(&value).unwrap_or(DEFAULT_LOCALE)),
        None => Ok(DEFAULT_LOCALE),
    }
}

fn settings_set_locale_internal(
    state: &AppState,
    locale: Locale,
) -> Result<OperationResult, AppError> {
    state.db.set_setting(LOCALE_KEY, locale.as_str())?;
    Ok(OperationResult::ok())
}

// ---------- test hooks (callable from integration tests) ----------

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub fn onboarding_get_state_test_hook(state: &AppState) -> Result<OnboardingState, AppError> {
    onboarding_get_state_internal(state)
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub fn onboarding_complete_test_hook(
    state: &AppState,
    answers: Option<OnboardingAnswers>,
) -> Result<OperationResult, AppError> {
    onboarding_complete_internal(state, answers)
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub fn settings_get_ui_mode_test_hook(state: &AppState) -> Result<UiMode, AppError> {
    settings_get_ui_mode_internal(state)
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub fn settings_set_ui_mode_test_hook(
    state: &AppState,
    mode: UiMode,
) -> Result<OperationResult, AppError> {
    settings_set_ui_mode_internal(state, mode)
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub fn settings_get_locale_test_hook(state: &AppState) -> Result<Locale, AppError> {
    settings_get_locale_internal(state)
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub fn settings_set_locale_test_hook(
    state: &AppState,
    locale: Locale,
) -> Result<OperationResult, AppError> {
    settings_set_locale_internal(state, locale)
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub fn install_git_test_hook() -> Result<OperationResult, AppError> {
    Ok(install_git_stub())
}

fn install_git_stub() -> OperationResult {
    OperationResult {
        success: true,
        message: Some(LocalizedString {
            zh: "请使用系统包管理器安装 Git（或在系统体检中触发自动修复）".into(),
            en: "Please install Git via the system package manager (or trigger auto-fix in the System Probe)".into(),
            ja: "システムのパッケージマネージャで Git をインストールしてください（または System Probe の自動修復をご利用ください）".into(),
        }),
        error_code: None,
    }
}

// ---------- Tauri commands ----------

#[tauri::command]
pub async fn onboarding_get_state(
    state: tauri::State<'_, AppState>,
) -> Result<OnboardingState, String> {
    onboarding_get_state_internal(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn onboarding_complete(
    state: tauri::State<'_, AppState>,
    answers: Option<OnboardingAnswers>,
) -> Result<OperationResult, String> {
    onboarding_complete_internal(&state, answers).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_get_ui_mode(
    state: tauri::State<'_, AppState>,
) -> Result<UiMode, String> {
    settings_get_ui_mode_internal(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_set_ui_mode(
    state: tauri::State<'_, AppState>,
    mode: UiMode,
) -> Result<OperationResult, String> {
    settings_set_ui_mode_internal(&state, mode).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_get_locale(
    state: tauri::State<'_, AppState>,
) -> Result<Locale, String> {
    settings_get_locale_internal(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_set_locale(
    state: tauri::State<'_, AppState>,
    locale: Locale,
) -> Result<OperationResult, String> {
    settings_set_locale_internal(&state, locale).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_git() -> Result<OperationResult, String> {
    Ok(install_git_stub())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn ui_mode_serializes_lowercase() {
        let json = serde_json::to_string(&UiMode::Expert).unwrap();
        assert_eq!(json, "\"expert\"");
    }

    #[test]
    fn locale_serializes_lowercase() {
        let json = serde_json::to_string(&Locale::Zh).unwrap();
        assert_eq!(json, "\"zh\"");
    }

    #[test]
    fn operation_result_uses_camel_case_and_skips_missing_fields() {
        let value = serde_json::to_value(OperationResult::ok()).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.get("success"), Some(&serde_json::Value::Bool(true)));
        assert!(obj.get("message").is_none());
        assert!(obj.get("errorCode").is_none());
    }

    #[test]
    fn install_git_returns_localized_stub() {
        let result = install_git_stub();
        assert!(result.success);
        let msg = result.message.expect("install_git must provide a hint");
        assert!(!msg.zh.is_empty());
        assert!(!msg.en.is_empty());
        assert!(!msg.ja.is_empty());
    }

    #[test]
    fn onboarding_answers_round_trip_camel_case() {
        let payload = serde_json::json!({
            "locale": "zh",
            "uiMode": "novice",
            "enableSandbox": true,
            "acceptedRedlines": true,
            "preferredCli": "claude",
        });
        let parsed: OnboardingAnswers = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(parsed.locale, Locale::Zh);
        assert_eq!(parsed.ui_mode, UiMode::Novice);
        assert!(parsed.enable_sandbox);
        assert!(parsed.accepted_redlines);
        assert_eq!(parsed.preferred_cli.as_deref(), Some("claude"));

        let reserialized = serde_json::to_value(&parsed).unwrap();
        assert_eq!(reserialized.get("uiMode"), payload.get("uiMode"));
        assert_eq!(
            reserialized.get("acceptedRedlines"),
            payload.get("acceptedRedlines"),
        );
        assert_eq!(reserialized.get("preferredCli"), payload.get("preferredCli"));
    }
}
