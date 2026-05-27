//! Mirror chains for Node.js distribution + Git for Windows.
//!
//! Doctrine: the **novice** target user is assumed to be on a pristine Chinese
//! network (great firewall + no homebrew + no nvm). Hitting `nodejs.org/dist/`
//! first will likely time out and lose 60+ seconds before the chain can even
//! retry. So China mirrors are listed first; official endpoints last as
//! fallback for users outside the GFW.
//!
//! Also exposes `detect_arch()` which (a) translates Rust's `std::env::consts::ARCH`
//! to the suffix that Node.js publishes archives under, and (b) on macOS detects
//! Rosetta 2 translation so a process running x86_64-under-arm64 still pulls
//! the native arm64 binary.

/// One entry in the Node.js distribution mirror chain.
///
/// `base` is the prefix that contains `index.json`, per-version directories,
/// and `SHASUMS256.txt`. The mirror is expected to mirror the canonical
/// `nodejs.org/dist/` layout 1:1.
pub struct NodeDistMirror {
    pub name: &'static str,
    pub base: &'static str,
}

impl NodeDistMirror {
    /// URL of `index.json` (used to resolve the latest 20.x version).
    pub fn dist_index_url(&self) -> String {
        format!("{}/index.json", self.base)
    }

    /// URL of a specific archive (e.g. `node-v20.11.0-linux-x64.tar.xz`)
    /// for a given version directory (e.g. `v20.11.0`).
    pub fn archive_url(&self, version: &str, archive_name: &str) -> String {
        format!("{}/{}/{}", self.base, version, archive_name)
    }

    /// URL of `SHASUMS256.txt` for a given version. MUST be fetched from the
    /// *same* mirror as the archive — different mirrors may serve different
    /// builds (rare, but possible during rollouts).
    pub fn shasums_url(&self, version: &str) -> String {
        format!("{}/{}/SHASUMS256.txt", self.base, version)
    }
}

/// Ordered list of Node.js distribution mirrors. China mirrors first, official
/// last as fallback. The pipeline tries each in order until one succeeds.
pub const NODE_DIST_MIRRORS: &[NodeDistMirror] = &[
    NodeDistMirror {
        name: "npmmirror",
        base: "https://npmmirror.com/mirrors/node",
    },
    NodeDistMirror {
        name: "huawei",
        base: "https://mirrors.huaweicloud.com/nodejs",
    },
    NodeDistMirror {
        name: "tsinghua",
        base: "https://mirrors.tuna.tsinghua.edu.cn/nodejs-release",
    },
    NodeDistMirror {
        name: "official",
        base: "https://nodejs.org/dist",
    },
];

/// Git for Windows mirror chain. Used by G3 (Windows PortableGit install).
pub struct GitForWindowsMirror {
    pub name: &'static str,
    pub base: &'static str,
}

pub const GIT_FOR_WINDOWS_MIRRORS: &[GitForWindowsMirror] = &[
    GitForWindowsMirror {
        name: "npmmirror",
        base: "https://npmmirror.com/mirrors/git-for-windows",
    },
    GitForWindowsMirror {
        name: "huawei",
        base: "https://mirrors.huaweicloud.com/git-for-windows",
    },
    GitForWindowsMirror {
        name: "github",
        base: "https://github.com/git-for-windows/git/releases/download",
    },
];

/// Returns the *effective* architecture suffix Node.js publishes under:
/// `"x64"`, `"arm64"`, `"x86"`, or the raw `std::env::consts::ARCH` if unknown.
///
/// On macOS, detects whether the current process is running under Rosetta 2
/// translation via `sysctl sysctl.proc_translated`. When true, `std::env::consts::ARCH`
/// reports `x86_64` even on Apple Silicon, which would cause us to download an
/// x64 binary and lock the user into Rosetta forever. We prefer the native arch
/// so the next launch (post-Rosetta-removal) Just Works.
pub fn detect_arch() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        if is_rosetta_translated() {
            return "arm64";
        }
    }
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    }
}

#[cfg(target_os = "macos")]
fn is_rosetta_translated() -> bool {
    use std::process::Command;
    Command::new("sysctl")
        .args(["-n", "sysctl.proc_translated"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_dist_mirrors_ordered_china_first() {
        assert!(NODE_DIST_MIRRORS.len() >= 4, "expected at least 4 mirrors");
        assert_eq!(NODE_DIST_MIRRORS[0].name, "npmmirror");
        assert_eq!(
            NODE_DIST_MIRRORS.last().expect("non-empty").name,
            "official"
        );
        // huawei + tsinghua somewhere between npmmirror and official
        let names: Vec<&str> = NODE_DIST_MIRRORS.iter().map(|m| m.name).collect();
        assert!(names.contains(&"huawei"));
        assert!(names.contains(&"tsinghua"));
    }

    #[test]
    fn archive_url_builds_correctly() {
        let m = &NODE_DIST_MIRRORS[0]; // npmmirror
        let url = m.archive_url("v20.11.0", "node-v20.11.0-linux-x64.tar.xz");
        assert_eq!(
            url,
            "https://npmmirror.com/mirrors/node/v20.11.0/node-v20.11.0-linux-x64.tar.xz"
        );

        let index = m.dist_index_url();
        assert_eq!(index, "https://npmmirror.com/mirrors/node/index.json");

        let shasums = m.shasums_url("v20.11.0");
        assert_eq!(
            shasums,
            "https://npmmirror.com/mirrors/node/v20.11.0/SHASUMS256.txt"
        );
    }

    #[test]
    fn official_mirror_uses_canonical_base() {
        let official = NODE_DIST_MIRRORS
            .iter()
            .find(|m| m.name == "official")
            .expect("official mirror missing");
        assert_eq!(official.base, "https://nodejs.org/dist");
    }

    #[test]
    fn detect_arch_returns_known_value() {
        let arch = detect_arch();
        // Must be one of the suffixes Node.js publishes under, OR the raw
        // ARCH constant on unexpected platforms (we don't pretend to know
        // every fringe arch).
        let known = ["x64", "arm64", "x86", "armv7l", "ppc64le", "s390x"];
        let raw_passthrough = arch == std::env::consts::ARCH;
        assert!(
            known.contains(&arch) || raw_passthrough,
            "detect_arch returned unexpected value: {arch}"
        );
    }

    #[test]
    fn git_for_windows_mirrors_ordered_china_first() {
        assert!(GIT_FOR_WINDOWS_MIRRORS.len() >= 3);
        assert_eq!(GIT_FOR_WINDOWS_MIRRORS[0].name, "npmmirror");
        assert_eq!(
            GIT_FOR_WINDOWS_MIRRORS.last().expect("non-empty").name,
            "github"
        );
    }
}
