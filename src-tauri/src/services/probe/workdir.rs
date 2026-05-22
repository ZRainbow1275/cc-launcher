//! Workdir group — 2 dimensions.
//!
//! Dimensions (all platforms):
//! 1. `workdirExists`    — `~/cc-launcher-projects/` exists
//! 2. `workdirWritable`  — can write a sentinel file inside the directory
//!
//! Probes are read-only at the host level: when the directory does not
//! exist we report `missing` and surface an [`FixAction::OpenHomeDir`]
//! so the user (or the launcher init) can create it explicitly. We do
//! NOT auto-create here.

use std::time::Instant;

use serde_json::json;

use crate::services::system_probe::{FixAction, ProbeGroup, ProbeItem, ProbeStatus};

/// Folder name relative to `~` for the per-profile workdir.
const WORKDIR_NAME: &str = "cc-launcher-projects";

/// Run all probes in the workdir group.
pub fn run_all() -> Vec<ProbeItem> {
    let workdir = dirs::home_dir().map(|h| h.join(WORKDIR_NAME));
    vec![
        probe_workdir_exists(&workdir),
        probe_workdir_writable(&workdir),
    ]
}

fn probe_workdir_exists(workdir: &Option<std::path::PathBuf>) -> ProbeItem {
    let t0 = Instant::now();
    let (status, value, message_key, fix_action) = match workdir {
        Some(p) => {
            let exists = p.exists();
            let status = if exists {
                ProbeStatus::Green
            } else {
                ProbeStatus::Missing
            };
            let msg = if exists {
                "probe.workdirExists.green"
            } else {
                "probe.workdirExists.missing"
            };
            // When missing, point the user at their home dir so they can
            // create / verify the path themselves.
            let fix = if exists {
                None
            } else {
                Some(FixAction::OpenHomeDir)
            };
            (
                status,
                json!({ "path": p.display().to_string(), "exists": exists }),
                msg.to_string(),
                fix,
            )
        }
        None => (
            ProbeStatus::Unknown,
            serde_json::Value::Null,
            "probe.workdirExists.unknown".into(),
            None,
        ),
    };

    ProbeItem {
        id: "workdirExists".into(),
        name_key: "probe.workdirExists.name".into(),
        status,
        value,
        message_key,
        fix_action,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Workdir,
    }
}

fn probe_workdir_writable(workdir: &Option<std::path::PathBuf>) -> ProbeItem {
    let t0 = Instant::now();
    let (status, value, message_key, fix_action) = match workdir {
        Some(p) => {
            // We do NOT create the directory if missing — that would
            // violate the "read-only probe" invariant. We test by probing
            // metadata or by writing into a tempfile under it ONLY if it
            // exists.
            if !p.exists() {
                (
                    ProbeStatus::Missing,
                    json!({ "path": p.display().to_string(), "writable": false }),
                    "probe.workdirWritable.missing".to_string(),
                    Some(FixAction::OpenHomeDir),
                )
            } else {
                let writable = check_writable(p);
                let status = if writable {
                    ProbeStatus::Green
                } else {
                    ProbeStatus::Red
                };
                let msg = if writable {
                    "probe.workdirWritable.green"
                } else {
                    "probe.workdirWritable.red"
                };
                let fix = if writable {
                    None
                } else {
                    Some(FixAction::OpenHomeDir)
                };
                (
                    status,
                    json!({ "path": p.display().to_string(), "writable": writable }),
                    msg.to_string(),
                    fix,
                )
            }
        }
        None => (
            ProbeStatus::Unknown,
            serde_json::Value::Null,
            "probe.workdirWritable.unknown".into(),
            None,
        ),
    };

    ProbeItem {
        id: "workdirWritable".into(),
        name_key: "probe.workdirWritable.name".into(),
        status,
        value,
        message_key,
        fix_action,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::Workdir,
    }
}

/// Returns true if we can create and delete a sentinel file inside `dir`.
/// On failure (permission denied, read-only volume), returns false.
/// This is the only place we touch the filesystem; the sentinel is wiped
/// immediately so the probe remains side-effect-free from the user's POV.
fn check_writable(dir: &std::path::Path) -> bool {
    use std::fs;
    use std::io::Write;

    let sentinel = dir.join(format!(".cc-launcher-probe-{}", std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut f = fs::File::create(&sentinel)?;
        f.write_all(b"probe")?;
        f.flush()?;
        Ok(())
    })();

    // Best-effort cleanup regardless of write outcome.
    let _ = fs::remove_file(&sentinel);
    result.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn run_all_returns_2_workdir_items() {
        let items = run_all();
        assert_eq!(items.len(), 2);
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"workdirExists"));
        assert!(ids.contains(&"workdirWritable"));
        assert!(items.iter().all(|i| i.group == ProbeGroup::Workdir));
    }

    #[test]
    fn workdir_exists_green_when_dir_present() {
        let tmp = TempDir::new().expect("tempdir");
        let it = probe_workdir_exists(&Some(tmp.path().to_path_buf()));
        assert_eq!(it.id, "workdirExists");
        assert_eq!(it.status, ProbeStatus::Green);
        assert!(it.fix_action.is_none());
    }

    #[test]
    fn workdir_exists_missing_when_dir_absent() {
        let bogus = std::env::temp_dir().join("cc-launcher-test-no-such-dir-xyz123-zzz");
        // Ensure it really doesn't exist.
        let _ = std::fs::remove_dir_all(&bogus);
        let it = probe_workdir_exists(&Some(bogus));
        assert_eq!(it.status, ProbeStatus::Missing);
        assert!(matches!(it.fix_action, Some(FixAction::OpenHomeDir)));
    }

    #[test]
    fn workdir_writable_green_when_writable() {
        let tmp = TempDir::new().expect("tempdir");
        let it = probe_workdir_writable(&Some(tmp.path().to_path_buf()));
        assert_eq!(it.id, "workdirWritable");
        assert_eq!(it.status, ProbeStatus::Green);
        let v = it.value.as_object().unwrap();
        assert_eq!(v.get("writable").and_then(|x| x.as_bool()), Some(true));
    }

    #[test]
    fn workdir_writable_missing_when_dir_absent() {
        let bogus = std::env::temp_dir().join("cc-launcher-test-no-such-dir-xyz999-yyy");
        let _ = std::fs::remove_dir_all(&bogus);
        let it = probe_workdir_writable(&Some(bogus));
        assert_eq!(it.status, ProbeStatus::Missing);
    }

    #[test]
    fn check_writable_returns_false_on_nonexistent_dir() {
        let bogus = std::env::temp_dir().join("cc-launcher-no-dir-zz123");
        let _ = std::fs::remove_dir_all(&bogus);
        assert!(!check_writable(&bogus));
    }
}
