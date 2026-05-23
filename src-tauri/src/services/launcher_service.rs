//! cc-launcher Launcher Service (Phase B Task B5 — D7)
//!
//! Orchestrates the final terminal-spawn pipeline:
//!
//!   profile (B1)  ──┐
//!   installer (B2) ─┼──► launcher_service ──► terminal spawn (detached)
//!   sandbox  (B3)  ─┘                                │
//!                                                    ▼
//!                                              audit NDJSON
//!
//! Hard invariants:
//! - **Spawn cwd MUST be `~/cc-launcher-projects/<profile_id>/`** — never the project dir.
//! - **Default flags never contain `--dangerously-skip-permissions` / `--yolo`**.
//!   These bypass flags are only emitted when the corresponding L1 rule is **unlocked**
//!   AND the profile explicitly opts in via `settings_json`. Even then `disableBypassPermissionsMode`
//!   in Claude managed settings takes priority and we still suppress the flag.
//! - **Workdir lock**: spawned process MUST start in the workdir, enforced via the terminal
//!   wrapper (`wt.exe --startingDirectory`, `osascript "cd <wd>;"`, `gnome-terminal --working-directory`).
//! - **Sandbox apply order**: `apply_sandbox_to_command` is called BEFORE `spawn()`. On
//!   Windows this is the `creation_flags(CREATE_NEW_CONSOLE | CREATE_NEW_PROCESS_GROUP)`
//!   step; on *nix it installs a `pre_exec` that calls `setsid` to detach from the
//!   controlling TTY. Failure to apply hardening aborts the launch.
//! - **No shell injection**: user-supplied extra args are validated via `is_arg_safe`;
//!   anything matching the deny list aborts the launch before the terminal wrapper is built.
//! - **Audit trail**: every successful `start_cli` writes one NDJSON line with
//!   `event_type = "sandbox_spawn"` containing cli, profile_id, terminal, flags, workdir.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::config::get_home_dir;
use crate::database::Database;
use crate::sandbox::audit::{self, AuditActor, AuditDecision, AuditEntry, AuditEventType};
use crate::sandbox::{self, SandboxLevel};
use crate::services::installer::cli_install::{CliInstaller, TargetCli};
use crate::services::installer::node_runtime::NodeRuntime;
use crate::services::profile::{self, Profile, TargetCli as ProfileTargetCli};

// ============================================================================
// Public types — wire format matches frontend launcher mock (camelCase)
// ============================================================================

/// Discovered terminal emulator candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalKind {
    WindowsTerminal,
    Cmd,
    PowerShell,
    MacTerminal,
    ITerm2,
    GnomeTerminal,
    Konsole,
    Xterm,
}

impl TerminalKind {
    pub fn as_wire(self) -> &'static str {
        match self {
            TerminalKind::WindowsTerminal => "windowsTerminal",
            TerminalKind::Cmd => "cmd",
            TerminalKind::PowerShell => "powerShell",
            TerminalKind::MacTerminal => "macTerminal",
            TerminalKind::ITerm2 => "iTerm2",
            TerminalKind::GnomeTerminal => "gnomeTerminal",
            TerminalKind::Konsole => "konsole",
            TerminalKind::Xterm => "xterm",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            TerminalKind::WindowsTerminal => "Windows Terminal",
            TerminalKind::Cmd => "Command Prompt",
            TerminalKind::PowerShell => "PowerShell",
            TerminalKind::MacTerminal => "Terminal.app",
            TerminalKind::ITerm2 => "iTerm2",
            TerminalKind::GnomeTerminal => "GNOME Terminal",
            TerminalKind::Konsole => "Konsole",
            TerminalKind::Xterm => "xterm",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInfo {
    pub kind: TerminalKind,
    pub binary_path: PathBuf,
    pub display_name: String,
    pub is_default: bool,
}

/// Launcher-level error with frontend-friendly serialization.
///
/// Serialized form is a discriminated union: `{ "kind": "...", ...payload }` — matches
/// the frontend `TypedError` shape consumed by the `start_cli` command result.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LauncherError {
    #[error("private Node runtime not installed")]
    NodeMissing,
    #[error("CLI not installed: {cli}")]
    #[serde(rename_all = "camelCase")]
    CliMissing { cli: String },
    #[error("profile invalid: {reason}")]
    #[serde(rename_all = "camelCase")]
    ProfileInvalid { reason: String },
    #[error("no usable terminal emulator found on this system")]
    TerminalNotFound,
    #[error("failed to create workdir {path}: {message}")]
    #[serde(rename_all = "camelCase")]
    WorkdirCreateFailed { path: String, message: String },
    #[error("failed to spawn terminal: {message}")]
    #[serde(rename_all = "camelCase")]
    SpawnFailed { message: String },
    #[error("unknown launcher error: {message}")]
    #[serde(rename_all = "camelCase")]
    Unknown { message: String },
}

impl From<LauncherError> for String {
    fn from(err: LauncherError) -> Self {
        err.to_string()
    }
}

/// Safety summary returned by `get_safety_summary` and embedded in `StartCliResult`.
///
/// `redlines_active` is always `true` — L2 hard redlines are compile-time enforced and
/// cannot be disabled at any sandbox level (see `sandbox::redline`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetySummary {
    pub sandbox_level: String,
    pub workdir: PathBuf,
    pub flags_applied: Vec<String>,
    pub env_keys_set: Vec<String>,
    pub redlines_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartCliOpts {
    pub cli: TargetCli,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub terminal: Option<TerminalKind>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartCliResult {
    pub pid: u32,
    pub workdir: PathBuf,
    pub terminal: TerminalKind,
    pub safety: SafetySummary,
}

// ============================================================================
// Service
// ============================================================================

pub struct LauncherService;

impl LauncherService {
    /// Probe the host for known terminal emulators.
    ///
    /// Always returns the platform-relevant set in priority order. The first installed
    /// entry is marked `is_default = true`. Empty Vec means "host has none"; callers must
    /// raise `TerminalNotFound`.
    pub async fn detect_terminals() -> Vec<TerminalInfo> {
        detect_terminals_impl()
    }

    /// Top-level launch pipeline. See module docs for invariants.
    pub async fn start_cli(
        db: Arc<Database>,
        opts: StartCliOpts,
    ) -> Result<StartCliResult, LauncherError> {
        // 1) Resolve profile.
        let profile = resolve_profile(&db, opts.cli, opts.profile_id.as_deref())?;

        // 2) Detect Node.
        let node = NodeRuntime::detect().await;
        if !node.installed {
            return Err(LauncherError::NodeMissing);
        }

        // 3) Detect CLI.
        let cli_status = CliInstaller::detect(opts.cli).await;
        if !cli_status.installed {
            return Err(LauncherError::CliMissing {
                cli: cli_name(opts.cli).to_string(),
            });
        }
        let cli_bin = cli_status
            .path
            .as_ref()
            .map(PathBuf::from)
            .ok_or_else(|| LauncherError::CliMissing {
                cli: cli_name(opts.cli).to_string(),
            })?;

        // 4) Pick terminal.
        let candidates = detect_terminals_impl();
        if candidates.is_empty() {
            return Err(LauncherError::TerminalNotFound);
        }
        let chosen =
            pick_terminal(&candidates, opts.terminal).ok_or(LauncherError::TerminalNotFound)?;

        // 5) Ensure workdir.
        let workdir = ensure_workdir(&profile.id, opts.cli)?;

        // 6) Validate user-supplied args.
        for arg in &opts.extra_args {
            if !is_arg_safe(arg) {
                return Err(LauncherError::ProfileInvalid {
                    reason: format!("unsafe extra arg rejected: {arg}"),
                });
            }
        }

        // 7) Assemble the CLI command vector — the launchee, not the terminal yet.
        let sandbox_level = sandbox::get_sandbox_level(&db).unwrap_or(SandboxLevel::Strict);
        let l1_unlocked = collect_unlocked_l1_ids(&db);
        let assembled = assemble_cli_command(
            opts.cli,
            &cli_bin,
            &workdir,
            &profile,
            &opts.extra_args,
            &l1_unlocked,
        );

        // 8) Compute env from profile.
        let env_pairs = compute_env(&profile, &workdir, opts.cli);

        // 9) Build OS-native terminal wrapper command.
        let mut term_cmd = build_terminal_command(&chosen.kind, &workdir, &assembled.cmdline);

        // Apply env to the outer terminal command — env vars propagate to the launched CLI.
        for (k, v) in &env_pairs {
            term_cmd.env(k, v);
        }

        // 10) Pre-spawn sandbox hardening:
        //   - Windows: creation flags (CREATE_NEW_CONSOLE | CREATE_NEW_PROCESS_GROUP).
        //   - macOS:   wrap argv with `sandbox-exec -f <profile>`.
        //   - Linux:   pre_exec setsid.
        // MUST be called before `spawn()` per the spec. Failure aborts the launch.
        sandbox::apply_to_command(&mut term_cmd, sandbox_level).map_err(|e| {
            LauncherError::SpawnFailed {
                message: format!("sandbox apply: {e}"),
            }
        })?;

        // 11) Spawn detached.
        let child = term_cmd
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| LauncherError::SpawnFailed {
                message: e.to_string(),
            })?;
        let pid = child.id();

        // 11b) Post-spawn Windows Job Object binding. No-op on *nix.
        if let Err(e) = sandbox::assign_to_job_object(pid, sandbox_level) {
            return Err(LauncherError::SpawnFailed {
                message: format!("sandbox apply (job object): {e}"),
            });
        }

        let safety = SafetySummary {
            sandbox_level: sandbox_level_to_marker(sandbox_level).to_string(),
            workdir: workdir.clone(),
            flags_applied: assembled.flags.clone(),
            env_keys_set: env_pairs.iter().map(|(k, _)| k.clone()).collect(),
            redlines_active: true,
        };

        // 12) Audit trail — record that sandbox was applied (post-spawn).
        write_spawn_audit(
            &profile.id,
            opts.cli,
            chosen.kind,
            &workdir,
            &assembled.flags,
            pid,
            /* sandbox_applied = */ true,
        );

        Ok(StartCliResult {
            pid,
            workdir,
            terminal: chosen.kind,
            safety,
        })
    }

    /// Open the workdir in the OS file manager.
    pub async fn open_workdir(profile_id: &str) -> Result<PathBuf, LauncherError> {
        if profile_id.is_empty() {
            return Err(LauncherError::ProfileInvalid {
                reason: "empty profile_id".into(),
            });
        }
        let mut root = get_home_dir();
        root.push("cc-launcher-projects");
        root.push(profile_id);
        std::fs::create_dir_all(&root).map_err(|e| LauncherError::WorkdirCreateFailed {
            path: root.display().to_string(),
            message: e.to_string(),
        })?;
        open_path_in_file_manager(&root)?;
        Ok(root)
    }

    /// Return what `start_cli` *would* do, without spawning anything.
    pub async fn get_safety_summary(
        db: Arc<Database>,
        cli: TargetCli,
        profile_id: Option<&str>,
    ) -> Result<SafetySummary, LauncherError> {
        let profile = resolve_profile(&db, cli, profile_id)?;
        let workdir = ensure_workdir(&profile.id, cli)?;
        let sandbox_level = sandbox::get_sandbox_level(&db).unwrap_or(SandboxLevel::Strict);
        let l1_unlocked = collect_unlocked_l1_ids(&db);

        // CLI bin path is allowed to be absent at preview time — the frontend uses this to
        // show which flags would be applied even before installing.
        let cli_bin = match CliInstaller::cli_binary(cli) {
            Ok(p) => p,
            Err(_) => PathBuf::from(cli_name(cli)),
        };
        let assembled =
            assemble_cli_command(cli, &cli_bin, &workdir, &profile, &[], &l1_unlocked);
        let env_pairs = compute_env(&profile, &workdir, cli);

        Ok(SafetySummary {
            sandbox_level: sandbox_level_to_marker(sandbox_level).to_string(),
            workdir,
            flags_applied: assembled.flags,
            env_keys_set: env_pairs.iter().map(|(k, _)| k.clone()).collect(),
            redlines_active: true,
        })
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn cli_name(cli: TargetCli) -> &'static str {
    match cli {
        TargetCli::Claude => "claude",
        TargetCli::Codex => "codex",
    }
}

/// Bridge the two TargetCli enums (installer vs profile). They share wire format but are
/// distinct nominal types.
fn to_profile_cli(cli: TargetCli) -> ProfileTargetCli {
    match cli {
        TargetCli::Claude => ProfileTargetCli::Claude,
        TargetCli::Codex => ProfileTargetCli::Codex,
    }
}

fn sandbox_level_to_marker(level: SandboxLevel) -> &'static str {
    // Map the existing `Strict | Medium` enum onto the L0/L1/L2 vocabulary used by the
    // safety summary contract. Redlines are always active regardless.
    match level {
        SandboxLevel::Strict => "L2",
        SandboxLevel::Medium => "L1",
    }
}

fn resolve_profile(
    db: &Arc<Database>,
    cli: TargetCli,
    profile_id: Option<&str>,
) -> Result<Profile, LauncherError> {
    let pcli = to_profile_cli(cli);
    match profile_id {
        Some(id) => match profile::get_profile(db, id, pcli) {
            Ok(p) => Ok(p),
            Err(profile::ProfileError::NotFound { .. }) => Err(LauncherError::ProfileInvalid {
                reason: format!("profile {id} not found"),
            }),
            Err(e) => Err(LauncherError::ProfileInvalid {
                reason: e.to_string(),
            }),
        },
        None => match profile::get_active_profile(db, pcli) {
            Ok(Some(p)) => Ok(p),
            Ok(None) => Err(LauncherError::ProfileInvalid {
                reason: format!("no active profile for cli {}", cli_name(cli)),
            }),
            Err(e) => Err(LauncherError::ProfileInvalid {
                reason: e.to_string(),
            }),
        },
    }
}

fn collect_unlocked_l1_ids(db: &Arc<Database>) -> BTreeSet<String> {
    match sandbox::get_l1_rules(db) {
        Ok(rules) => rules
            .into_iter()
            .filter(|r| {
                // "unlocked" = rule is either disabled (enabled=false) OR has a future
                // unlocked_until timestamp.
                if !r.enabled {
                    return true;
                }
                if let Some(until) = r.unlocked_until {
                    return until > chrono::Utc::now();
                }
                false
            })
            .map(|r| r.id)
            .collect(),
        Err(_) => BTreeSet::new(),
    }
}

/// Top-level workdir: `~/cc-launcher-projects/<profile_id>/`.
/// CLI sub-directory: `~/cc-launcher-projects/<profile_id>/.<cli>/`.
///
/// Both are created (idempotently). The CLI sub-dir lets each CLI have its own per-profile
/// scratch space without colliding when the user runs Claude and Codex on the same profile id.
fn ensure_workdir(profile_id: &str, cli: TargetCli) -> Result<PathBuf, LauncherError> {
    if profile_id.is_empty() {
        return Err(LauncherError::ProfileInvalid {
            reason: "empty profile_id".into(),
        });
    }
    let mut root = get_home_dir();
    root.push("cc-launcher-projects");
    root.push(profile_id);

    std::fs::create_dir_all(&root).map_err(|e| LauncherError::WorkdirCreateFailed {
        path: root.display().to_string(),
        message: e.to_string(),
    })?;

    let cli_sub = root.join(format!(".{}", cli_name(cli)));
    std::fs::create_dir_all(&cli_sub).map_err(|e| LauncherError::WorkdirCreateFailed {
        path: cli_sub.display().to_string(),
        message: e.to_string(),
    })?;

    Ok(root)
}

// ----------------------------------------------------------------------------
// Terminal discovery
// ----------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn detect_terminals_impl() -> Vec<TerminalInfo> {
    let mut out: Vec<TerminalInfo> = Vec::new();
    let candidates: &[(TerminalKind, &str)] = &[
        (TerminalKind::WindowsTerminal, "wt.exe"),
        (TerminalKind::PowerShell, "pwsh.exe"),
        (TerminalKind::PowerShell, "powershell.exe"),
        (TerminalKind::Cmd, "cmd.exe"),
    ];

    let mut seen_kinds: BTreeSet<&'static str> = BTreeSet::new();
    for (kind, exe) in candidates {
        if seen_kinds.contains(kind.as_wire()) {
            // Already recorded another binary for this kind (e.g. pwsh.exe before powershell.exe).
            continue;
        }
        if let Ok(path) = which::which(exe) {
            let is_default = out.is_empty();
            out.push(TerminalInfo {
                kind: *kind,
                binary_path: path,
                display_name: kind.display_name().to_string(),
                is_default,
            });
            seen_kinds.insert(kind.as_wire());
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn detect_terminals_impl() -> Vec<TerminalInfo> {
    let mut out: Vec<TerminalInfo> = Vec::new();
    let iterm = PathBuf::from("/Applications/iTerm.app");
    if iterm.exists() {
        out.push(TerminalInfo {
            kind: TerminalKind::ITerm2,
            binary_path: iterm,
            display_name: TerminalKind::ITerm2.display_name().to_string(),
            is_default: true,
        });
    }
    let term = PathBuf::from("/System/Applications/Utilities/Terminal.app");
    if term.exists() {
        let is_default = out.is_empty();
        out.push(TerminalInfo {
            kind: TerminalKind::MacTerminal,
            binary_path: term,
            display_name: TerminalKind::MacTerminal.display_name().to_string(),
            is_default,
        });
    }
    out
}

#[cfg(target_os = "linux")]
fn detect_terminals_impl() -> Vec<TerminalInfo> {
    let mut out: Vec<TerminalInfo> = Vec::new();
    for (kind, exe) in &[
        (TerminalKind::GnomeTerminal, "gnome-terminal"),
        (TerminalKind::Konsole, "konsole"),
        (TerminalKind::Xterm, "xterm"),
    ] {
        if let Ok(path) = which::which(exe) {
            let is_default = out.is_empty();
            out.push(TerminalInfo {
                kind: *kind,
                binary_path: path,
                display_name: kind.display_name().to_string(),
                is_default,
            });
        }
    }
    out
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn detect_terminals_impl() -> Vec<TerminalInfo> {
    Vec::new()
}

fn pick_terminal(
    candidates: &[TerminalInfo],
    preferred: Option<TerminalKind>,
) -> Option<TerminalInfo> {
    if let Some(want) = preferred {
        if let Some(found) = candidates.iter().find(|c| c.kind == want) {
            return Some(found.clone());
        }
        // Preferred terminal not installed → fall through and pick default.
    }
    candidates
        .iter()
        .find(|c| c.is_default)
        .cloned()
        .or_else(|| candidates.first().cloned())
}

// ----------------------------------------------------------------------------
// CLI command assembly
// ----------------------------------------------------------------------------

struct AssembledCli {
    /// Display vector of flags (no binary path, no values quoted) — for safety summary.
    flags: Vec<String>,
    /// Final argv that the terminal must invoke. argv[0] is the absolute CLI binary path.
    cmdline: Vec<String>,
}

fn assemble_cli_command(
    cli: TargetCli,
    cli_bin: &Path,
    workdir: &Path,
    profile: &Profile,
    extra_args: &[String],
    l1_unlocked: &BTreeSet<String>,
) -> AssembledCli {
    let mut flags: Vec<String> = Vec::new();
    let mut argv: Vec<String> = vec![cli_bin.display().to_string()];

    match cli {
        TargetCli::Claude => {
            // Workdir: always grant access to the per-profile workdir.
            argv.push("--add-dir".into());
            argv.push(workdir.display().to_string());
            flags.push(format!("--add-dir {}", workdir.display()));

            // Only emit --dangerously-skip-permissions when ALL of:
            //   1. Profile explicitly opts in (`claude_skip_permissions: true`)
            //   2. L1 rule `L1.claude_skip_permissions` is unlocked (note: this rule is
            //      hard-coded `unlockable=false`, so this branch is effectively dead by
            //      design — but kept for defense-in-depth).
            //   3. Profile doesn't set `disableBypassPermissionsMode`.
            if profile_opt_bool(profile, "claude_skip_permissions")
                && l1_unlocked.contains("L1.claude_skip_permissions")
                && !profile_opt_bool(profile, "disableBypassPermissionsMode")
            {
                argv.push("--dangerously-skip-permissions".into());
                flags.push("--dangerously-skip-permissions".into());
            }
        }
        TargetCli::Codex => {
            // Workdir: pin cwd so Codex's workspace-write sandbox is rooted here.
            argv.push("--cd".into());
            argv.push(workdir.display().to_string());
            flags.push(format!("--cd {}", workdir.display()));

            // Same dead-by-design path for Codex YOLO.
            if profile_opt_bool(profile, "codex_yolo")
                && l1_unlocked.contains("L1.codex_yolo")
            {
                argv.push("--dangerously-bypass-approvals-and-sandbox".into());
                flags.push("--dangerously-bypass-approvals-and-sandbox".into());
            }
        }
    }

    // User-supplied extra args — already validated by start_cli before reaching here.
    for arg in extra_args {
        argv.push(arg.clone());
        flags.push(arg.clone());
    }

    AssembledCli {
        flags,
        cmdline: argv,
    }
}

fn profile_opt_bool(profile: &Profile, key: &str) -> bool {
    let parsed: serde_json::Value =
        serde_json::from_str(&profile.settings_json).unwrap_or(serde_json::Value::Null);
    parsed.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn compute_env(profile: &Profile, workdir: &Path, cli: TargetCli) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = Vec::new();

    // Workdir-flavored env hint for the CLI (Claude reads CLAUDE_PROJECT_DIR; Codex uses cwd).
    match cli {
        TargetCli::Claude => {
            env.push((
                "CLAUDE_PROJECT_DIR".to_string(),
                workdir.display().to_string(),
            ));
        }
        TargetCli::Codex => {
            // Codex respects CODEX_HOME for its config dir — we intentionally DO NOT override
            // it here; we just hint cwd through the terminal wrapper.
        }
    }

    // Profile env_overrides: stored inside settings_json under "env_overrides".
    let parsed: serde_json::Value =
        serde_json::from_str(&profile.settings_json).unwrap_or(serde_json::Value::Null);
    if let Some(obj) = parsed.get("env_overrides").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                env.push((k.clone(), s.to_string()));
            }
        }
    }

    env
}

// ----------------------------------------------------------------------------
// Terminal wrapper command builders — strict OS-native argv (no shell concatenation).
// ----------------------------------------------------------------------------

fn build_terminal_command(kind: &TerminalKind, workdir: &Path, cli_argv: &[String]) -> Command {
    match kind {
        TerminalKind::WindowsTerminal => {
            // wt.exe new-tab --title "<t>" --startingDirectory "<wd>" -- <argv...>
            let mut cmd = Command::new("wt.exe");
            cmd.arg("new-tab")
                .arg("--title")
                .arg(format!("cc-launcher: {}", first_arg_basename(cli_argv)))
                .arg("--startingDirectory")
                .arg(workdir)
                .arg("--");
            for a in cli_argv {
                cmd.arg(a);
            }
            cmd
        }
        TerminalKind::PowerShell => {
            // powershell -NoLogo -NoExit -Command "Set-Location -LiteralPath <wd>; & '<bin>' <args>"
            let bin = pwsh_quote(cli_argv.first().map(String::as_str).unwrap_or(""));
            let mut script = format!(
                "Set-Location -LiteralPath {}; & {}",
                pwsh_quote(&workdir.display().to_string()),
                bin
            );
            for arg in cli_argv.iter().skip(1) {
                script.push(' ');
                script.push_str(&pwsh_quote(arg));
            }
            let mut cmd = Command::new("powershell.exe");
            cmd.arg("-NoLogo")
                .arg("-NoExit")
                .arg("-Command")
                .arg(script);
            cmd
        }
        TerminalKind::Cmd => {
            // cmd.exe /K "cd /D <wd> && <bin> <args...>"
            let mut script = String::from("cd /D ");
            script.push_str(&cmd_quote(&workdir.display().to_string()));
            script.push_str(" && ");
            for (i, a) in cli_argv.iter().enumerate() {
                if i > 0 {
                    script.push(' ');
                }
                script.push_str(&cmd_quote(a));
            }
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/K").arg(script);
            cmd
        }
        TerminalKind::MacTerminal => {
            // osascript -e 'tell application "Terminal" to do script "cd <wd>; <bin> <args>"'
            let inner = osa_inner_script(workdir, cli_argv);
            let mut cmd = Command::new("osascript");
            cmd.arg("-e").arg(format!(
                "tell application \"Terminal\" to do script \"{}\"",
                inner
            ));
            cmd
        }
        TerminalKind::ITerm2 => {
            let inner = osa_inner_script(workdir, cli_argv);
            let mut cmd = Command::new("osascript");
            cmd.arg("-e").arg(format!(
                "tell application \"iTerm\"\n create window with default profile command \"{}\"\nend tell",
                inner
            ));
            cmd
        }
        TerminalKind::GnomeTerminal => {
            // gnome-terminal --working-directory=<wd> -- bash -c "<bin> <args>; exec bash"
            let mut cmd = Command::new("gnome-terminal");
            cmd.arg(format!("--working-directory={}", workdir.display()))
                .arg("--")
                .arg("bash")
                .arg("-c")
                .arg(bash_payload(cli_argv));
            cmd
        }
        TerminalKind::Konsole => {
            let mut cmd = Command::new("konsole");
            cmd.arg("--workdir")
                .arg(workdir)
                .arg("-e")
                .arg("bash")
                .arg("-c")
                .arg(bash_payload(cli_argv));
            cmd
        }
        TerminalKind::Xterm => {
            let mut cmd = Command::new("xterm");
            cmd.arg("-e").arg("bash").arg("-c").arg(format!(
                "cd {} && {}",
                bash_single_quote(&workdir.display().to_string()),
                bash_payload(cli_argv)
            ));
            cmd
        }
    }
}

/// Build the bash payload string. argv joined with bash single-quote escaping.
fn bash_payload(cli_argv: &[String]) -> String {
    let mut out = String::new();
    for (i, a) in cli_argv.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&bash_single_quote(a));
    }
    out.push_str("; exec bash");
    out
}

fn bash_single_quote(s: &str) -> String {
    // POSIX shell single-quote escaping: close quote, escape literal quote, reopen.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn pwsh_quote(s: &str) -> String {
    // PowerShell single-quoted literal: escape ' by doubling.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn cmd_quote(s: &str) -> String {
    // CMD: wrap in double quotes; escape embedded " as "" (no shell metas allowed past
    // is_arg_safe gate).
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        if ch == '"' {
            out.push_str("\"\"");
        } else {
            out.push(ch);
        }
    }
    out.push('"');
    out
}

/// Build the inner double-quoted AppleScript string. Embedded " is doubled-escaped per
/// AppleScript convention (`\"`).
fn osa_inner_script(workdir: &Path, cli_argv: &[String]) -> String {
    let mut inner = format!("cd {}; ", bash_single_quote(&workdir.display().to_string()));
    for (i, a) in cli_argv.iter().enumerate() {
        if i > 0 {
            inner.push(' ');
        }
        inner.push_str(&bash_single_quote(a));
    }
    // Escape " inside the AppleScript string literal.
    inner.replace('"', "\\\"")
}

fn first_arg_basename(argv: &[String]) -> String {
    argv.first()
        .and_then(|p| Path::new(p).file_name())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "cli".to_string())
}

// ----------------------------------------------------------------------------
// Open workdir in OS file manager
// ----------------------------------------------------------------------------

fn open_path_in_file_manager(path: &Path) -> Result<(), LauncherError> {
    let mut cmd = file_manager_command(path);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| LauncherError::SpawnFailed {
            message: e.to_string(),
        })
}

#[cfg(target_os = "windows")]
fn file_manager_command(path: &Path) -> Command {
    let mut cmd = Command::new("explorer.exe");
    cmd.arg(path);
    cmd
}

#[cfg(target_os = "macos")]
fn file_manager_command(path: &Path) -> Command {
    let mut cmd = Command::new("open");
    cmd.arg(path);
    cmd
}

#[cfg(target_os = "linux")]
fn file_manager_command(path: &Path) -> Command {
    let mut cmd = Command::new("xdg-open");
    cmd.arg(path);
    cmd
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn file_manager_command(path: &Path) -> Command {
    let mut cmd = Command::new("echo");
    cmd.arg(path);
    cmd
}

// ----------------------------------------------------------------------------
// Argument safety
// ----------------------------------------------------------------------------

/// Strict allowlist: alphanumerics, dash, underscore, slash, dot, equals, colon,
/// at-sign, plus, comma, backslash. Anything else (incl. spaces, shell metas, control
/// chars) is unsafe.
///
/// Additionally, reject any arg whose leading token starts with a bypass-style prefix.
pub fn is_arg_safe(arg: &str) -> bool {
    if arg.is_empty() {
        return false;
    }
    let lower = arg.to_ascii_lowercase();
    // Reject bypass / yolo / skip-permissions flags entirely.
    if lower.starts_with("--dangerously")
        || lower.starts_with("--skip-permissions")
        || lower.starts_with("--yolo")
        || lower.starts_with("--bypass-")
        || lower.contains("dangerously-skip-permissions")
        || lower.contains("dangerously-bypass-approvals-and-sandbox")
    {
        return false;
    }
    // Reject shell metas.
    if arg.contains("&&")
        || arg.contains("||")
        || arg.contains(';')
        || arg.contains('`')
        || arg.contains("$(")
        || arg.contains('\n')
        || arg.contains('\r')
    {
        return false;
    }
    // Allow only the safe character class.
    arg.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '-' | '_' | '/' | '.' | '=' | ':' | '@' | '+' | ',' | '\\'
            )
    })
}

// ----------------------------------------------------------------------------
// Audit trail
// ----------------------------------------------------------------------------

fn write_spawn_audit(
    profile_id: &str,
    cli: TargetCli,
    terminal: TerminalKind,
    workdir: &Path,
    flags: &[String],
    pid: u32,
    sandbox_applied: bool,
) {
    let entry = AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_type: AuditEventType::SandboxSpawn,
        actor: AuditActor::Launcher,
        decision: AuditDecision::Allowed,
        rule_id: None,
        matched_command: Some(format!("{} {}", cli_name(cli), flags.join(" "))),
        pid: Some(pid),
        profile_id: Some(profile_id.to_string()),
        cwd: Some(workdir.display().to_string()),
        note: Some(format!(
            "terminal={};sandbox_applied={}",
            terminal.as_wire(),
            sandbox_applied
        )),
        sandbox_applied: Some(sandbox_applied),
    };
    audit::append(&entry);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arg_safe_rejects_dangerous_flags() {
        assert!(!is_arg_safe("--dangerously-skip-permissions"));
        assert!(!is_arg_safe("--dangerously-bypass-approvals-and-sandbox"));
        assert!(!is_arg_safe("--yolo"));
        assert!(!is_arg_safe("--skip-permissions"));
        assert!(!is_arg_safe("--bypass-something"));
    }

    #[test]
    fn arg_safe_rejects_shell_metas() {
        assert!(!is_arg_safe("; rm -rf /"));
        assert!(!is_arg_safe("$(curl evil)"));
        assert!(!is_arg_safe("foo && bar"));
        assert!(!is_arg_safe("a || b"));
        assert!(!is_arg_safe("`whoami`"));
        assert!(!is_arg_safe("foo\nbar"));
    }

    #[test]
    fn arg_safe_accepts_normal_args() {
        assert!(is_arg_safe("--model=claude-opus-4-7"));
        assert!(is_arg_safe("--add-dir"));
        assert!(is_arg_safe("/tmp/work-dir"));
        assert!(is_arg_safe("-p"));
        assert!(is_arg_safe("name@example.com"));
        assert!(is_arg_safe("a-b-c.txt"));
        assert!(is_arg_safe("C:/Users/test"));
    }

    #[test]
    fn arg_safe_rejects_empty_and_spaces() {
        assert!(!is_arg_safe(""));
        assert!(!is_arg_safe(" "));
        assert!(!is_arg_safe("foo bar"));
    }

    #[test]
    fn bash_single_quote_escapes_quote() {
        assert_eq!(bash_single_quote("foo"), "'foo'");
        assert_eq!(bash_single_quote("it's"), "'it'\\''s'");
        assert_eq!(bash_single_quote("/tmp/x"), "'/tmp/x'");
    }

    #[test]
    fn pwsh_quote_doubles_single_quotes() {
        assert_eq!(pwsh_quote("foo"), "'foo'");
        assert_eq!(pwsh_quote("it's"), "'it''s'");
    }

    #[test]
    fn cmd_quote_doubles_double_quotes() {
        assert_eq!(cmd_quote("foo"), "\"foo\"");
        assert_eq!(cmd_quote("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn sandbox_level_marker_maps_correctly() {
        assert_eq!(sandbox_level_to_marker(SandboxLevel::Strict), "L2");
        assert_eq!(sandbox_level_to_marker(SandboxLevel::Medium), "L1");
    }

    #[test]
    fn terminal_kind_wire_round_trip() {
        for kind in &[
            TerminalKind::WindowsTerminal,
            TerminalKind::Cmd,
            TerminalKind::PowerShell,
            TerminalKind::MacTerminal,
            TerminalKind::ITerm2,
            TerminalKind::GnomeTerminal,
            TerminalKind::Konsole,
            TerminalKind::Xterm,
        ] {
            assert!(!kind.as_wire().is_empty());
            assert!(!kind.display_name().is_empty());
        }
    }
}
