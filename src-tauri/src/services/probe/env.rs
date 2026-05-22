//! Env & permissions group — 5 dimensions.
//!
//! Dimensions:
//! 1. `envConflicts`  — ANTHROPIC / OPENAI / GEMINI env vars present (all platforms)
//! 2. `admin`         — admin / root state (informational, NOT auto-fixable)
//! 3. `psPolicy`      — PowerShell ExecutionPolicy (Windows, NOT auto-fixable)
//! 4. `defender`      — Windows Defender exclusion list status (Windows, NOT auto-fixable)
//! 5. `rosetta`       — Rosetta 2 installed (macOS arm64, NOT auto-fixable)
//!
//! 4 of these (Defender / Admin / PsPolicy / Rosetta) are "informational
//! driver factors" — they use [`FixAction::ExternalLink`] and are
//! excluded from "一键全修" by [`crate::services::system_probe::is_auto_fixable`].
//!
//! The 5th informational factor (`SystemProxy`) lives in the **network** group.
//!
//! Probes are read-only.

use std::time::Instant;

use serde_json::json;

use crate::services::env_checker;
use crate::services::system_probe::{FixAction, ProbeGroup, ProbeItem, ProbeStatus};

/// Run all probes in the env-and-permissions group. Returns exactly 5 items
/// across all platforms (cross-platform stubs fill in for OS-specific checks
/// so the contract stays stable).
pub fn run_all() -> Vec<ProbeItem> {
    vec![
        probe_env_conflicts(),
        probe_admin(),
        probe_ps_policy(),
        probe_defender(),
        probe_rosetta(),
    ]
}

fn probe_env_conflicts() -> ProbeItem {
    let t0 = Instant::now();
    let mut conflicts = Vec::new();
    for app in ["claude", "codex", "gemini"] {
        if let Ok(mut c) = env_checker::check_env_conflicts(app) {
            conflicts.append(&mut c);
        }
    }

    let count = conflicts.len();
    // Always green when no conflicts. When there are any, mark red and
    // surface the first one as a CleanEnvVar action; the UI iterates
    // through all of them by re-running the probe after each fix.
    let (status, message_key, fix_action) = if count == 0 {
        (
            ProbeStatus::Green,
            "probe.envConflicts.green".to_string(),
            None,
        )
    } else {
        let first = conflicts[0].var_name.clone();
        (
            ProbeStatus::Red,
            "probe.envConflicts.red".to_string(),
            Some(FixAction::CleanEnvVar { var_name: first }),
        )
    };

    ProbeItem {
        id: "envConflicts".into(),
        name_key: "probe.envConflicts.name".into(),
        status,
        value: json!({ "count": count, "conflicts": conflicts }),
        message_key,
        fix_action,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Env,
    }
}

// ------------------------ admin (informational) ------------------------

#[cfg(target_os = "windows")]
fn is_admin_native() -> bool {
    // Read-only probe: invoke `net session` which requires admin privileges
    // on Windows. Exit code 0 means we're elevated; anything else means we
    // aren't. This avoids any `unsafe` Win32 token calls.
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    Command::new("net")
        .arg("session")
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_admin_native() -> bool {
    // On Unix, $USER == "root" OR sudo escalation marker. We avoid unsafe
    // libc calls and rely on the `EUID` environment / `id -u` instead.
    if let Ok(out) = std::process::Command::new("id").arg("-u").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Ok(uid) = s.parse::<u32>() {
                return uid == 0;
            }
        }
    }
    // Fallback: $USER == "root"
    std::env::var("USER").map(|u| u == "root").unwrap_or(false)
}

fn probe_admin() -> ProbeItem {
    let t0 = Instant::now();
    let is_admin = is_admin_native();
    // Always informational — even when running as admin we surface a docs link.
    // Status is green when NOT admin (recommended), yellow when admin
    // (we suggest re-launching as normal user).
    let (status, message_key) = if is_admin {
        (ProbeStatus::Yellow, "probe.admin.yellow")
    } else {
        (ProbeStatus::Green, "probe.admin.green")
    };

    // All 5 driver factors are informational => ExternalLink ALWAYS.
    let docs_url = if cfg!(target_os = "windows") {
        "https://learn.microsoft.com/windows/security/identity-protection/user-account-control/user-account-control-overview"
    } else {
        "https://en.wikipedia.org/wiki/Sudo"
    };

    ProbeItem {
        id: "admin".into(),
        name_key: "probe.admin.name".into(),
        status,
        value: json!({ "isAdmin": is_admin }),
        message_key: message_key.into(),
        fix_action: Some(FixAction::ExternalLink {
            url: docs_url.into(),
            label_key: "probe.admin.docs".into(),
        }),
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Env,
    }
}

// --------------------- powershell policy (Win-only) --------------------

fn probe_ps_policy() -> ProbeItem {
    let t0 = Instant::now();

    #[cfg(target_os = "windows")]
    {
        let policy = read_ps_policy().unwrap_or_else(|| "Unknown".to_string());
        // "Restricted" blocks npm `.ps1` scripts → yellow + docs link.
        let status = match policy.as_str() {
            "Restricted" | "AllSigned" => ProbeStatus::Yellow,
            "Unknown" | "" => ProbeStatus::Unknown,
            _ => ProbeStatus::Green,
        };
        let message_key = match status {
            ProbeStatus::Green => "probe.psPolicy.green",
            ProbeStatus::Yellow => "probe.psPolicy.yellow",
            _ => "probe.psPolicy.unknown",
        };
        #[allow(clippy::needless_return)] // cfg-gated early return — non-Windows branch follows
        return ProbeItem {
            id: "psPolicy".into(),
            name_key: "probe.psPolicy.name".into(),
            status,
            value: json!({ "policy": policy }),
            message_key: message_key.into(),
            fix_action: Some(FixAction::ExternalLink {
                url: "https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_execution_policies".into(),
                label_key: "probe.psPolicy.docs".into(),
            }),
            elapsed_ms: t0.elapsed().as_millis() as u64,
            group: ProbeGroup::Env,
        };
    }

    #[cfg(not(target_os = "windows"))]
    ProbeItem {
        id: "psPolicy".into(),
        name_key: "probe.psPolicy.name".into(),
        status: ProbeStatus::Unknown,
        value: json!({ "policy": "n/a" }),
        message_key: "probe.psPolicy.notApplicable".into(),
        fix_action: Some(FixAction::ExternalLink {
            url: "https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_execution_policies".into(),
            label_key: "probe.psPolicy.docs".into(),
        }),
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Env,
    }
}

#[cfg(target_os = "windows")]
fn read_ps_policy() -> Option<String> {
    // We invoke powershell directly. This is read-only and short-lived.
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", "Get-ExecutionPolicy"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

// ----------------------- defender (Win-only, info) ---------------------

fn probe_defender() -> ProbeItem {
    let t0 = Instant::now();

    // We deliberately do NOT enumerate exclusion paths (requires admin +
    // Get-MpPreference). MVP only flags this as informational.
    #[cfg(target_os = "windows")]
    let (status, value) = (ProbeStatus::Unknown, json!({ "excluded": false }));
    #[cfg(not(target_os = "windows"))]
    let (status, value) = (
        ProbeStatus::Unknown,
        json!({ "excluded": false, "platform": "n/a" }),
    );

    ProbeItem {
        id: "defender".into(),
        name_key: "probe.defender.name".into(),
        status,
        value,
        message_key: "probe.defender.unknown".into(),
        fix_action: Some(FixAction::ExternalLink {
            url: "https://learn.microsoft.com/windows/security/threat-protection/microsoft-defender-antivirus/configure-exclusions-microsoft-defender-antivirus".into(),
            label_key: "probe.defender.docs".into(),
        }),
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Env,
    }
}

// ----------------------- rosetta (macOS arm64, info) -------------------

fn probe_rosetta() -> ProbeItem {
    let t0 = Instant::now();

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let (status, value, message_key) = {
        // `pgrep -q oahd` returns 0 if Rosetta's oahd daemon is running.
        let installed = std::process::Command::new("/usr/bin/pgrep")
            .args(["-q", "oahd"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        let s = if installed {
            ProbeStatus::Green
        } else {
            ProbeStatus::Yellow
        };
        let m = if installed {
            "probe.rosetta.green"
        } else {
            "probe.rosetta.yellow"
        };
        (s, json!({ "installed": installed }), m)
    };

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    let (status, value, message_key) = (
        ProbeStatus::Unknown,
        json!({ "installed": false, "platform": "n/a" }),
        "probe.rosetta.notApplicable",
    );

    ProbeItem {
        id: "rosetta".into(),
        name_key: "probe.rosetta.name".into(),
        status,
        value,
        message_key: message_key.into(),
        fix_action: Some(FixAction::ExternalLink {
            url: "https://support.apple.com/en-us/HT211861".into(),
            label_key: "probe.rosetta.docs".into(),
        }),
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Env,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_all_returns_5_env_items() {
        let items = run_all();
        assert_eq!(items.len(), 5);
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"envConflicts"));
        assert!(ids.contains(&"admin"));
        assert!(ids.contains(&"psPolicy"));
        assert!(ids.contains(&"defender"));
        assert!(ids.contains(&"rosetta"));
        assert!(items.iter().all(|i| i.group == ProbeGroup::Env));
    }

    #[test]
    fn four_driver_factors_use_external_link() {
        // Admin / PsPolicy / Defender / Rosetta — the 4 env-group informational factors.
        for it in run_all() {
            if it.id == "envConflicts" {
                continue; // not an informational factor
            }
            match it.fix_action {
                Some(FixAction::ExternalLink { .. }) => {}
                ref other => panic!(
                    "driver factor {} must have ExternalLink fix, got {other:?}",
                    it.id
                ),
            }
        }
    }

    #[test]
    fn env_conflicts_green_when_empty_red_when_nonempty() {
        // The shape contract: 0 -> green/no-fix; >0 -> red/CleanEnvVar.
        let it = probe_env_conflicts();
        let count = it.value.get("count").and_then(|x| x.as_u64()).unwrap_or(0);
        if count == 0 {
            assert_eq!(it.status, ProbeStatus::Green);
            assert!(it.fix_action.is_none());
        } else {
            assert_eq!(it.status, ProbeStatus::Red);
            assert!(matches!(it.fix_action, Some(FixAction::CleanEnvVar { .. })));
        }
    }

    #[test]
    fn admin_probe_always_has_external_link() {
        let it = probe_admin();
        assert!(matches!(
            it.fix_action,
            Some(FixAction::ExternalLink { .. })
        ));
    }

    #[test]
    fn ps_policy_probe_always_has_external_link() {
        let it = probe_ps_policy();
        assert!(matches!(
            it.fix_action,
            Some(FixAction::ExternalLink { .. })
        ));
    }

    #[test]
    fn defender_probe_always_has_external_link() {
        let it = probe_defender();
        assert!(matches!(
            it.fix_action,
            Some(FixAction::ExternalLink { .. })
        ));
    }

    #[test]
    fn rosetta_probe_always_has_external_link() {
        let it = probe_rosetta();
        assert!(matches!(
            it.fix_action,
            Some(FixAction::ExternalLink { .. })
        ));
    }
}
