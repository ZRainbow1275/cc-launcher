//! Wire DTOs for the launcher Tauri command boundary (D-5).
//!
//! This module owns the public contract surface that the frontend consumes —
//! shapes match `src/lib/api/contracts.ts` and `src/lib/api/mock/launcher.ts`
//! exactly. The internal service-layer types in `services::launcher_service`
//! stay unchanged; we project from them at the Tauri cmd boundary so the
//! existing integration tests (which assert against the service-layer shape)
//! keep passing while the frontend adapter becomes a thin pass-through.
//!
//! Projection rules:
//! - `TerminalInfo` → `WireTerminalInfo` — adds `id` (= enum wire), renames
//!   `binary_path: PathBuf` → `path: String`, sets `installed: true` for every
//!   emitted candidate.
//! - `SafetySummary` → `WireSafetySummary` — drops `sandbox_level`,
//!   `env_keys_set`, `redlines_active` and `workdir: PathBuf`. Adds
//!   `profile_id`, `target_cli`, `cwd_display`, `l1_active_count`,
//!   `l2_redline_count`. `redlines_active: true` is preserved on the wire as
//!   `l2_redline_count = sandbox::redline::redlines().len()` (always > 0 by
//!   compile-time construction).
//! - `StartCliResult` → `WireLaunchResult` — encodes errors inline as
//!   `success: false, error: Some(TypedError)` so the frontend never has to
//!   `try/catch` a Tauri reject.
//! - `LauncherError` → `TypedError` — codes match the constants used by
//!   `src/lib/api/mock/fixtures/i18n.ts` (`NODE_MISSING`, `CLI_MISSING`,
//!   `PROFILE_NOT_FOUND`, `NO_TERMINAL_AVAILABLE`, etc.). English message text
//!   is taken from `LauncherError::Display`.

use serde::{Deserialize, Serialize};

use crate::sandbox;
use crate::services::installer::cli_install::TargetCli;
use crate::services::launcher_service::{
    LauncherError, SafetySummary, StartCliResult, TerminalInfo, TerminalKind,
};
use crate::services::profile::{LocalizedString, TypedError};

// ============================================================================
// Wire enums
// ============================================================================

/// Frontend `TargetCli` enum — lowercase wire values `claude` | `codex`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WireTargetCli {
    Claude,
    Codex,
}

impl From<TargetCli> for WireTargetCli {
    fn from(c: TargetCli) -> Self {
        match c {
            TargetCli::Claude => WireTargetCli::Claude,
            TargetCli::Codex => WireTargetCli::Codex,
        }
    }
}

/// Stable kebab/lower wire id for a terminal kind. Matches
/// `contracts.ts::TerminalKind` exactly.
pub fn terminal_kind_wire_id(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::WindowsTerminal => "wt",
        TerminalKind::Cmd => "cmd",
        TerminalKind::PowerShell => "powershell",
        TerminalKind::MacTerminal => "terminal-app",
        TerminalKind::ITerm2 => "iterm2",
        TerminalKind::GnomeTerminal => "gnome-terminal",
        TerminalKind::Konsole => "konsole",
        TerminalKind::Xterm => "xterm",
    }
}

/// Parse the inverse of `terminal_kind_wire_id`. Returns `None` for unknown
/// ids; callers fall back to the default candidate.
pub fn parse_terminal_wire_id(id: &str) -> Option<TerminalKind> {
    match id {
        "wt" => Some(TerminalKind::WindowsTerminal),
        "cmd" => Some(TerminalKind::Cmd),
        "powershell" => Some(TerminalKind::PowerShell),
        "terminal-app" => Some(TerminalKind::MacTerminal),
        "iterm2" => Some(TerminalKind::ITerm2),
        "gnome-terminal" => Some(TerminalKind::GnomeTerminal),
        "konsole" => Some(TerminalKind::Konsole),
        "xterm" => Some(TerminalKind::Xterm),
        _ => None,
    }
}

/// Wire form of `TerminalKind`. Serializes to the kebab/lower id strings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WireTerminalKind {
    #[serde(rename = "wt")]
    Wt,
    #[serde(rename = "cmd")]
    Cmd,
    #[serde(rename = "powershell")]
    Powershell,
    #[serde(rename = "terminal-app")]
    TerminalApp,
    #[serde(rename = "iterm2")]
    Iterm2,
    #[serde(rename = "gnome-terminal")]
    GnomeTerminal,
    #[serde(rename = "konsole")]
    Konsole,
    #[serde(rename = "xterm")]
    Xterm,
}

impl From<TerminalKind> for WireTerminalKind {
    fn from(k: TerminalKind) -> Self {
        match k {
            TerminalKind::WindowsTerminal => WireTerminalKind::Wt,
            TerminalKind::Cmd => WireTerminalKind::Cmd,
            TerminalKind::PowerShell => WireTerminalKind::Powershell,
            TerminalKind::MacTerminal => WireTerminalKind::TerminalApp,
            TerminalKind::ITerm2 => WireTerminalKind::Iterm2,
            TerminalKind::GnomeTerminal => WireTerminalKind::GnomeTerminal,
            TerminalKind::Konsole => WireTerminalKind::Konsole,
            TerminalKind::Xterm => WireTerminalKind::Xterm,
        }
    }
}

// ============================================================================
// Wire DTOs
// ============================================================================

/// Wire shape for `detect_terminals`. Matches `contracts.ts::TerminalCandidate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireTerminalInfo {
    pub id: String,
    pub kind: WireTerminalKind,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub installed: bool,
    pub is_default: bool,
}

impl From<TerminalInfo> for WireTerminalInfo {
    fn from(t: TerminalInfo) -> Self {
        WireTerminalInfo {
            id: terminal_kind_wire_id(t.kind).to_string(),
            kind: WireTerminalKind::from(t.kind),
            display_name: t.display_name,
            path: Some(t.binary_path.display().to_string()),
            // Only installed candidates are emitted by `detect_terminals_impl`,
            // so this flag is unconditionally `true` on the wire.
            installed: true,
            is_default: t.is_default,
        }
    }
}

/// Wire shape for `get_safety_summary`. Matches
/// `src/lib/api/mock/launcher.ts::SafetySummary`.
///
/// `l2_redline_count` is the wire-level surrogate for the service-layer
/// `redlines_active: bool`. It is non-zero by compile-time construction
/// (`sandbox::redline::redlines()` is a static `Lazy<Vec<L2Redline>>`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireSafetySummary {
    pub profile_id: String,
    pub target_cli: WireTargetCli,
    pub flags: Vec<String>,
    pub cwd: String,
    pub cwd_display: String,
    pub l1_active_count: usize,
    pub l2_redline_count: usize,
}

impl WireSafetySummary {
    /// Project the service-layer `SafetySummary` into the wire shape.
    ///
    /// The service-layer `SafetySummary` does not carry `profile_id` or
    /// `target_cli` (they're implicit in the caller's args), so they must be
    /// re-attached at the boundary. `l1_active_count` is computed by counting
    /// active L1 rules at projection time.
    pub fn project(
        summary: SafetySummary,
        profile_id: String,
        cli: TargetCli,
        l1_active_count: usize,
    ) -> Self {
        let cwd = summary.workdir.display().to_string();
        let cwd_display = shorten_with_home_prefix(&cwd);
        WireSafetySummary {
            profile_id,
            target_cli: WireTargetCli::from(cli),
            flags: summary.flags_applied,
            cwd,
            cwd_display,
            l1_active_count,
            l2_redline_count: sandbox::redline::redlines().len(),
        }
    }
}

/// Wire shape for `start_cli`. Matches `contracts.ts::LaunchResult`.
///
/// Errors are encoded inline (never via the Tauri error channel) so the
/// frontend can render localized messages without an additional try/catch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireLaunchResult {
    pub success: bool,
    pub profile_id: String,
    pub target_cli: WireTargetCli,
    pub terminal_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub cwd: String,
    pub launched_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TypedError>,
}

impl WireLaunchResult {
    /// Build the success envelope from the service result.
    pub fn from_service(
        ok: StartCliResult,
        profile_id: String,
        cli: TargetCli,
        launched_at: String,
    ) -> Self {
        WireLaunchResult {
            success: true,
            profile_id,
            target_cli: WireTargetCli::from(cli),
            terminal_id: terminal_kind_wire_id(ok.terminal).to_string(),
            pid: Some(ok.pid),
            cwd: ok.workdir.display().to_string(),
            launched_at,
            error: None,
        }
    }

    /// Build the failure envelope from a `LauncherError`.
    pub fn from_error(
        err: LauncherError,
        profile_id: String,
        cli: TargetCli,
        terminal_id: String,
        cwd: String,
        launched_at: String,
    ) -> Self {
        WireLaunchResult {
            success: false,
            profile_id,
            target_cli: WireTargetCli::from(cli),
            terminal_id,
            pid: None,
            cwd,
            launched_at,
            error: Some(typed_error_from(&err)),
        }
    }
}

// ============================================================================
// LauncherError → TypedError projection
// ============================================================================

/// Project a `LauncherError` into the contracts.ts `TypedError` shape.
///
/// Code values match the constants emitted by
/// `src/lib/api/mock/fixtures/i18n.ts` so the frontend i18n layer can resolve
/// localized messages by code. The `message` field carries a `LocalizedString`
/// fallback assembled from the service-layer `Display` impl (English) and
/// hand-translated zh/ja strings — the frontend prefers `code`-based lookup
/// but degrades gracefully when an unknown code is returned.
pub fn typed_error_from(err: &LauncherError) -> TypedError {
    match err {
        LauncherError::NodeMissing => TypedError {
            code: "NODE_MISSING".into(),
            message: LocalizedString {
                zh: "未检测到 Node.js，请先安装 Node.js".into(),
                en: "Node.js not found. Please install Node.js first.".into(),
                ja: "Node.js が見つかりません。先に Node.js をインストールしてください。".into(),
            },
            cause: None,
            retryable: false,
        },
        LauncherError::CliMissing { cli } => TypedError {
            code: "CLI_MISSING".into(),
            message: LocalizedString {
                zh: format!("未检测到 CLI: {cli}"),
                en: format!("CLI not installed: {cli}"),
                ja: format!("CLI が見つかりません: {cli}"),
            },
            cause: Some(cli.clone()),
            retryable: false,
        },
        LauncherError::ProfileInvalid { reason } => TypedError {
            code: "PROFILE_NOT_FOUND".into(),
            message: LocalizedString {
                zh: "目标 Profile 不存在".into(),
                en: "Target profile not found".into(),
                ja: "対象のプロファイルが見つかりません".into(),
            },
            cause: Some(reason.clone()),
            retryable: false,
        },
        LauncherError::TerminalNotFound => TypedError {
            code: "NO_TERMINAL_AVAILABLE".into(),
            message: LocalizedString {
                zh: "未检测到可用的系统终端".into(),
                en: "No system terminal available".into(),
                ja: "利用可能なシステムターミナルが見つかりません".into(),
            },
            cause: None,
            retryable: false,
        },
        LauncherError::WorkdirCreateFailed { path, message } => TypedError {
            code: "WORKDIR_CREATE_FAILED".into(),
            message: LocalizedString {
                zh: format!("无法创建工作目录: {path}"),
                en: format!("Failed to create workdir: {path}"),
                ja: format!("作業ディレクトリの作成に失敗しました: {path}"),
            },
            cause: Some(message.clone()),
            retryable: true,
        },
        LauncherError::SpawnFailed { message } => TypedError {
            code: "SPAWN_FAILED".into(),
            message: LocalizedString {
                zh: "终端启动失败".into(),
                en: "Failed to spawn terminal".into(),
                ja: "ターミナルの起動に失敗しました".into(),
            },
            cause: Some(message.clone()),
            retryable: true,
        },
        LauncherError::Unknown { message } => TypedError {
            code: "LAUNCH_FAILED".into(),
            message: LocalizedString {
                zh: "启动失败".into(),
                en: "Launch failed".into(),
                ja: "起動に失敗しました".into(),
            },
            cause: Some(message.clone()),
            retryable: false,
        },
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Replace the user's home dir prefix with `~` for friendlier display. Falls
/// back to the original path if the home dir cannot be resolved or the path is
/// outside the home tree.
fn shorten_with_home_prefix(absolute: &str) -> String {
    let home = crate::config::get_home_dir().display().to_string();
    if !home.is_empty() && absolute.starts_with(&home) {
        let mut out = String::from("~");
        out.push_str(&absolute[home.len()..]);
        out.replace('\\', "/")
    } else {
        absolute.to_string()
    }
}
