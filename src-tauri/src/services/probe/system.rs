//! System resources group — 4 dimensions.
//!
//! Dimensions (all platforms):
//! 1. `os`              — OS name + version
//! 2. `cpu`             — physical core count + brand
//! 3. `memory`          — total + available memory
//! 4. `disk`            — free space on the disk that holds `~`
//!
//! Thresholds and contract per
//! `.trellis/tasks/05-21-cc-launcher-mvp/research/system-probe.md §3`.

use std::time::Instant;

use serde_json::json;
use sysinfo::{Disks, System};

use crate::services::system_probe::{ProbeGroup, ProbeItem, ProbeStatus};

/// Run all probes in the system-resources group.
pub fn run_all() -> Vec<ProbeItem> {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();

    vec![
        probe_os(),
        probe_cpu(&sys),
        probe_memory(&sys),
        probe_disk(),
    ]
}

fn probe_os() -> ProbeItem {
    let t0 = Instant::now();
    let name = System::name().unwrap_or_else(|| "unknown".into());
    let version = System::os_version().unwrap_or_else(|| "unknown".into());
    let long_version = System::long_os_version().unwrap_or_else(|| name.clone());
    let arch: &'static str = std::env::consts::ARCH;
    let bits: u8 = if cfg!(target_pointer_width = "64") {
        64
    } else {
        32
    };

    // We do not attempt to enforce a hard OS-version threshold here (Tauri 2
    // already requires Win 10 19041+ / macOS 12+ at build time). The probe
    // just reports values; the UI surfaces educational guidance.
    ProbeItem {
        id: "os".into(),
        name_key: "probe.os.name".into(),
        status: ProbeStatus::Green,
        value: json!({
            "name": name,
            "version": version,
            "longVersion": long_version,
            "arch": arch,
            "bits": bits,
        }),
        message_key: "probe.os.green".into(),
        fix_action: None,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::System,
    }
}

fn probe_cpu(sys: &System) -> ProbeItem {
    let t0 = Instant::now();
    let physical = System::physical_core_count().unwrap_or(0);
    let brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_default();

    let status = if physical >= 4 {
        ProbeStatus::Green
    } else if physical >= 2 {
        ProbeStatus::Yellow
    } else if physical == 0 {
        // sysinfo couldn't determine cores — Apple Silicon early versions etc.
        ProbeStatus::Unknown
    } else {
        ProbeStatus::Red
    };

    let message_key = match status {
        ProbeStatus::Green => "probe.cpu.green",
        ProbeStatus::Yellow => "probe.cpu.yellow",
        ProbeStatus::Red => "probe.cpu.red",
        _ => "probe.cpu.unknown",
    };

    ProbeItem {
        id: "cpu".into(),
        name_key: "probe.cpu.name".into(),
        status,
        value: json!({ "physicalCores": physical, "brand": brand }),
        message_key: message_key.into(),
        fix_action: None,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::System,
    }
}

fn probe_memory(sys: &System) -> ProbeItem {
    let t0 = Instant::now();
    let total = sys.total_memory();
    let available = sys.available_memory();
    let total_gb = total as f64 / 1_073_741_824.0;
    let avail_gb = available as f64 / 1_073_741_824.0;
    let percent_free = if total > 0 {
        ((available as f64 / total as f64) * 100.0).round() as u32
    } else {
        0
    };

    // Threshold per research §3: total >= 8 GiB green, 4-8 GiB yellow, < 4 GiB red.
    // We combine total + available into one dimension to keep the count at 17.
    let status = if total_gb >= 8.0 && avail_gb >= 2.0 {
        ProbeStatus::Green
    } else if total_gb >= 4.0 && avail_gb >= 1.0 {
        ProbeStatus::Yellow
    } else if total == 0 {
        ProbeStatus::Unknown
    } else {
        ProbeStatus::Red
    };

    let message_key = match status {
        ProbeStatus::Green => "probe.memory.green",
        ProbeStatus::Yellow => "probe.memory.yellow",
        ProbeStatus::Red => "probe.memory.red",
        _ => "probe.memory.unknown",
    };

    ProbeItem {
        id: "memory".into(),
        name_key: "probe.memory.name".into(),
        status,
        value: json!({
            "totalGb": round2(total_gb),
            "availableGb": round2(avail_gb),
            "percentFree": percent_free,
        }),
        message_key: message_key.into(),
        fix_action: None,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::System,
    }
}

fn probe_disk() -> ProbeItem {
    let t0 = Instant::now();
    let home = dirs::home_dir().unwrap_or_default();
    let disks = Disks::new_with_refreshed_list();

    let mut best: Option<&sysinfo::Disk> = None;
    let mut best_match_len = 0usize;
    for d in disks.iter() {
        let mp = d.mount_point();
        if home.starts_with(mp) {
            let len = mp.to_string_lossy().len();
            if len >= best_match_len {
                best_match_len = len;
                best = Some(d);
            }
        }
    }

    let (status, value, message_key, fix_action) = match best {
        Some(d) => {
            let avail_gb = d.available_space() as f64 / 1_073_741_824.0;
            let status = if avail_gb >= 5.0 {
                ProbeStatus::Green
            } else if avail_gb >= 2.0 {
                ProbeStatus::Yellow
            } else {
                ProbeStatus::Red
            };
            let msg = match status {
                ProbeStatus::Green => "probe.disk.green",
                ProbeStatus::Yellow => "probe.disk.yellow",
                _ => "probe.disk.red",
            };
            let fix = None;
            (
                status,
                json!({
                    "availableGb": round2(avail_gb),
                    "mount": d.mount_point().to_string_lossy(),
                }),
                msg.to_string(),
                fix,
            )
        }
        None => (
            ProbeStatus::Unknown,
            serde_json::Value::Null,
            "probe.disk.unknown".into(),
            None,
        ),
    };

    ProbeItem {
        id: "disk".into(),
        name_key: "probe.disk.name".into(),
        status,
        value,
        message_key,
        fix_action,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        group: ProbeGroup::System,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_all_returns_4_items_in_system_group() {
        let items = run_all();
        assert_eq!(items.len(), 4);
        assert!(items.iter().all(|i| i.group == ProbeGroup::System));
    }

    #[test]
    fn os_probe_is_green_with_arch_and_bits() {
        let it = probe_os();
        assert_eq!(it.id, "os");
        assert_eq!(it.status, ProbeStatus::Green);
        let v = it.value.as_object().expect("value should be object");
        assert!(v.contains_key("arch"));
        assert!(v.contains_key("bits"));
        assert!(v.contains_key("longVersion"));
    }

    #[test]
    fn cpu_probe_yellow_below_4_cores() {
        // We can't easily mock sysinfo, but we can assert the bucketing
        // function semantically by inspecting the actual host probe — the
        // value layout must be stable.
        let mut sys = System::new();
        sys.refresh_cpu_all();
        let it = probe_cpu(&sys);
        assert_eq!(it.id, "cpu");
        assert_eq!(it.group, ProbeGroup::System);
        let cores = it
            .value
            .get("physicalCores")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        match it.status {
            ProbeStatus::Green => assert!(cores >= 4),
            ProbeStatus::Yellow => assert!((2..4).contains(&cores)),
            ProbeStatus::Red => assert_eq!(cores, 1),
            ProbeStatus::Unknown => assert_eq!(cores, 0),
            ProbeStatus::Missing => panic!("cpu should never be missing"),
        }
    }

    #[test]
    fn memory_probe_has_expected_value_shape() {
        let mut sys = System::new();
        sys.refresh_memory();
        let it = probe_memory(&sys);
        assert_eq!(it.id, "memory");
        let v = it.value.as_object().expect("value should be object");
        assert!(v.contains_key("totalGb"));
        assert!(v.contains_key("availableGb"));
        assert!(v.contains_key("percentFree"));
    }

    #[test]
    fn disk_probe_returns_known_status() {
        let it = probe_disk();
        assert_eq!(it.id, "disk");
        // We do not assert green/yellow/red because the test host's disk
        // state is unknown. Low disk is informational because the launcher
        // cannot safely free disk space for the user.
        match (it.status, &it.fix_action) {
            (ProbeStatus::Green, None) => {}
            (ProbeStatus::Yellow, None) => {}
            (ProbeStatus::Red, None) => {}
            (ProbeStatus::Unknown, None) => {}
            other => panic!("unexpected disk status/fix combo: {other:?}"),
        }
    }
}
