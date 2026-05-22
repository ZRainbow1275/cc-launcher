//! Runtime group — 4 dimensions.
//!
//! Dimensions (all platforms):
//! 1. `node` — Node.js executable + version (target Node 20 LTS per D5+)
//! 2. `npm`  — npm executable + version
//! 3. `git`  — Git executable + version
//! 4. `path` — PATH entries / important runtime dirs
//!
//! Read-only: spawns `--version` subprocesses but never modifies state.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use serde_json::json;

use crate::services::system_probe::{FixAction, ProbeGroup, ProbeItem, ProbeStatus};

/// MVP target Node major version (per D5+).
const TARGET_NODE_LTS_MAJOR: u8 = 20;

/// Run all probes in the runtime group.
pub fn run_all() -> Vec<ProbeItem> {
    vec![probe_node(), probe_npm(), probe_git(), probe_path()]
}

/// Resolve the absolute path to an executable using `which`. Returns
/// `None` if not found.
fn locate(exe: &str) -> Option<PathBuf> {
    which::which(exe).ok()
}

/// Execute `<exe> --version` and return its stdout trimmed. We do NOT
/// shell out; we use std::process::Command for tight control.
fn run_version(path: &PathBuf) -> Option<String> {
    let mut cmd = Command::new(path);
    cmd.arg("--version");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let out = cmd.output().ok()?;
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

fn parse_node_major(v: &str) -> Option<u32> {
    // "v20.11.0" -> 20
    v.trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
}

fn probe_node() -> ProbeItem {
    let t0 = Instant::now();
    let mut item = ProbeItem {
        id: "node".into(),
        name_key: "probe.node.name".into(),
        status: ProbeStatus::Missing,
        value: serde_json::Value::Null,
        message_key: "probe.node.missing".into(),
        fix_action: Some(FixAction::InstallNode {
            target_lts_major: TARGET_NODE_LTS_MAJOR,
        }),
        elapsed_ms: 0,
        group: ProbeGroup::Runtime,
    };

    if let Some(path) = locate("node") {
        if let Some(version) = run_version(&path) {
            let major = parse_node_major(&version).unwrap_or(0);
            item.value = json!({ "version": version, "path": path.display().to_string() });
            item.status = if major >= TARGET_NODE_LTS_MAJOR as u32 {
                item.fix_action = None;
                item.message_key = "probe.node.green".into();
                ProbeStatus::Green
            } else if major >= 18 {
                item.message_key = "probe.node.yellow".into();
                ProbeStatus::Yellow
            } else {
                item.message_key = "probe.node.red".into();
                ProbeStatus::Red
            };
        }
    }

    item.elapsed_ms = t0.elapsed().as_millis() as u64;
    item
}

fn probe_npm() -> ProbeItem {
    let t0 = Instant::now();
    let mut item = ProbeItem {
        id: "npm".into(),
        name_key: "probe.npm.name".into(),
        status: ProbeStatus::Missing,
        value: serde_json::Value::Null,
        message_key: "probe.npm.missing".into(),
        // No standalone fix action — npm ships with Node.
        fix_action: None,
        elapsed_ms: 0,
        group: ProbeGroup::Runtime,
    };

    if let Some(path) = locate("npm") {
        if let Some(version) = run_version(&path) {
            let major = version
                .split('.')
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            item.value = json!({ "version": version, "path": path.display().to_string() });
            item.status = if major >= 10 {
                item.message_key = "probe.npm.green".into();
                ProbeStatus::Green
            } else if major >= 9 {
                item.message_key = "probe.npm.yellow".into();
                ProbeStatus::Yellow
            } else {
                item.message_key = "probe.npm.red".into();
                ProbeStatus::Red
            };
        }
    }

    item.elapsed_ms = t0.elapsed().as_millis() as u64;
    item
}

fn probe_git() -> ProbeItem {
    let t0 = Instant::now();
    let mut item = ProbeItem {
        id: "git".into(),
        name_key: "probe.git.name".into(),
        status: ProbeStatus::Missing,
        value: serde_json::Value::Null,
        message_key: "probe.git.missing".into(),
        fix_action: Some(FixAction::InstallGit),
        elapsed_ms: 0,
        group: ProbeGroup::Runtime,
    };

    if let Some(path) = locate("git") {
        if let Some(raw) = run_version(&path) {
            // "git version 2.43.0" -> "2.43.0"
            let version = raw
                .strip_prefix("git version ")
                .unwrap_or(&raw)
                .trim()
                .to_string();
            item.value = json!({ "version": version, "path": path.display().to_string() });
            item.status = ProbeStatus::Green;
            item.message_key = "probe.git.green".into();
            item.fix_action = None;
        }
    }

    item.elapsed_ms = t0.elapsed().as_millis() as u64;
    item
}

fn probe_path() -> ProbeItem {
    let t0 = Instant::now();
    let raw = std::env::var_os("PATH").unwrap_or_default();
    let entries: Vec<String> = std::env::split_paths(&raw)
        .map(|p| p.display().to_string())
        .collect();

    // Critical entries: must contain at least one of these for the runtime
    // group to be considered intact. We don't fail hard on absence — npm-global
    // is "yellow" because the launcher injects its own PATH when spawning CLI.
    let critical_keywords = ["node", "npm", "git"];
    let missing: Vec<String> = critical_keywords
        .iter()
        .filter(|kw| {
            !entries
                .iter()
                .any(|e| e.to_lowercase().contains(&kw.to_lowercase()))
        })
        .map(|kw| (*kw).to_string())
        .collect();

    let status = if missing.is_empty() {
        ProbeStatus::Green
    } else if missing.len() < critical_keywords.len() {
        ProbeStatus::Yellow
    } else {
        ProbeStatus::Red
    };

    let message_key = match status {
        ProbeStatus::Green => "probe.path.green",
        ProbeStatus::Yellow => "probe.path.yellow",
        _ => "probe.path.red",
    };

    let fix_action = if !missing.is_empty() {
        // Launcher will inject these into subprocess PATH only — read-only at host level.
        Some(FixAction::InjectPathEntries {
            entries: missing.clone(),
        })
    } else {
        None
    };

    ProbeItem {
        id: "path".into(),
        name_key: "probe.path.name".into(),
        status,
        value: json!({ "entries": entries, "missing": missing }),
        message_key: message_key.into(),
        fix_action,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Runtime,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_all_returns_4_runtime_items() {
        let items = run_all();
        assert_eq!(items.len(), 4);
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"node"));
        assert!(ids.contains(&"npm"));
        assert!(ids.contains(&"git"));
        assert!(ids.contains(&"path"));
        assert!(items.iter().all(|i| i.group == ProbeGroup::Runtime));
    }

    #[test]
    fn parse_node_major_handles_standard_format() {
        assert_eq!(parse_node_major("v20.11.0"), Some(20));
        assert_eq!(parse_node_major("v18.19.0"), Some(18));
        assert_eq!(parse_node_major("20.0.0"), Some(20));
        assert_eq!(parse_node_major(""), None);
        assert_eq!(parse_node_major("garbage"), None);
    }

    #[test]
    fn node_probe_yields_consistent_shape() {
        // Whether Node is installed depends on the host. We assert the
        // contract regardless.
        let it = probe_node();
        assert_eq!(it.id, "node");
        match it.status {
            ProbeStatus::Missing => {
                assert!(it.fix_action.is_some());
                assert!(matches!(
                    it.fix_action,
                    Some(FixAction::InstallNode {
                        target_lts_major: 20
                    })
                ));
            }
            ProbeStatus::Green => {
                assert!(it.fix_action.is_none());
                assert!(it.value.get("version").is_some());
            }
            ProbeStatus::Yellow | ProbeStatus::Red => {
                // Old Node — fix action stays InstallNode.
                assert!(matches!(
                    it.fix_action,
                    Some(FixAction::InstallNode {
                        target_lts_major: 20
                    })
                ));
            }
            ProbeStatus::Unknown => panic!("node should not return unknown"),
        }
    }

    #[test]
    fn git_probe_yields_consistent_shape() {
        let it = probe_git();
        assert_eq!(it.id, "git");
        match (it.status, &it.fix_action) {
            (ProbeStatus::Missing, Some(FixAction::InstallGit)) => {}
            (ProbeStatus::Green, None) => {
                let v = it.value.get("version").and_then(|x| x.as_str()).unwrap();
                assert!(!v.is_empty());
            }
            other => panic!("unexpected git probe state: {other:?}"),
        }
    }

    #[test]
    fn path_probe_reports_entries_and_missing() {
        let it = probe_path();
        assert_eq!(it.id, "path");
        let v = it.value.as_object().unwrap();
        assert!(v.contains_key("entries"));
        assert!(v.contains_key("missing"));
        let missing_list = v.get("missing").and_then(|x| x.as_array()).unwrap();
        if missing_list.is_empty() {
            assert_eq!(it.status, ProbeStatus::Green);
            assert!(it.fix_action.is_none());
        } else {
            assert!(it.fix_action.is_some());
            assert!(matches!(
                it.fix_action,
                Some(FixAction::InjectPathEntries { .. })
            ));
        }
    }
}
