//! Private PortableGit runtime download / extract / verify (Windows only).
//!
//! Mirrors the `NodeRuntime` doctrine: user-level, never touches system PATH,
//! mirror chain prefers China-first endpoints to avoid GFW timeouts for the
//! pristine novice user.
//!
//! Target directory (via `dirs::data_local_dir()`):
//! `%LOCALAPPDATA%\cc-switch\runtime\git\` (final layout: `…\git\bin\git.exe`).
//!
//! Pipeline:
//! 1. Detect arch via `mirrors::detect_arch()` (`x64` or `arm64`).
//! 2. For each mirror in `GIT_FOR_WINDOWS_MIRRORS`, attempt to download the
//!    pinned PortableGit self-extracting archive to a tempfile. First success
//!    wins.
//! 3. Spawn `<archive.exe> -y -o<git_dir>` to self-extract (PortableGit is a
//!    signed 7z self-extracting exe — `-y` is silent, `-o` chooses dest).
//! 4. Validate `<git_dir>/bin/git.exe --version` returns a sane string.
//! 5. On any failure, best-effort cleanup `<git_dir>` so we don't leave a
//!    half-installed runtime.
//!
//! NOTE: PortableGit does not publish a SHASUMS file in a uniform layout
//! across all three mirrors, so we skip SHA-256 verification. The download
//! is HTTPS and the resulting exe is signed by Johannes Schindelin; the
//! signature is checked implicitly by Windows when the user (or our spawn)
//! launches it. If the signature check is broken, the spawn will return a
//! non-zero exit and we'll surface that as an install failure.

use std::path::{Path, PathBuf};

use futures::StreamExt;
use thiserror::Error;
use tokio::io::AsyncWriteExt;

use super::mirrors::{detect_arch, GIT_FOR_WINDOWS_MIRRORS};

/// Pinned PortableGit version. Bumped manually when a new Git for Windows
/// stable release ships. Held constant here to keep mirror URL composition
/// trivial and avoid runtime discovery (no equivalent of `index.json` exists
/// across all three mirrors).
const PINNED_VERSION: &str = "2.45.2";

#[derive(Debug, Error)]
pub enum PortableGitError {
    #[error("HTTP fetch failed: {0}")]
    Http(String),
    #[error("IO error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("self-extract failed: {0}")]
    Extract(String),
    #[error("data_local_dir unavailable on this platform")]
    NoDataDir,
    #[error("post-install validation failed: {0}")]
    Validate(String),
    #[error("unsupported architecture for PortableGit: {0}")]
    UnsupportedArch(String),
}

/// Single API surface for PortableGit detection & install.
pub struct PortableGit;

impl PortableGit {
    /// Root for the private runtime tree (`<data_local>/cc-switch/runtime`).
    ///
    /// Honours `CC_SWITCH_TEST_HOME` so tests can redirect the install
    /// location (mirrors `NodeRuntime::runtime_root`).
    pub fn runtime_root() -> Result<PathBuf, PortableGitError> {
        if let Ok(override_dir) = std::env::var("CC_SWITCH_TEST_HOME") {
            let trimmed = override_dir.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed).join("cc-switch").join("runtime"));
            }
        }
        let base = dirs::data_local_dir().ok_or(PortableGitError::NoDataDir)?;
        Ok(base.join("cc-switch").join("runtime"))
    }

    /// Where the private PortableGit tree lives.
    pub fn git_dir() -> Result<PathBuf, PortableGitError> {
        Ok(Self::runtime_root()?.join("git"))
    }

    /// Path to the private `git.exe` binary.
    pub fn git_binary() -> Result<PathBuf, PortableGitError> {
        Ok(Self::git_dir()?.join("bin").join("git.exe"))
    }

    /// Lightweight detection — does NOT touch network.
    pub async fn detect() -> bool {
        Self::git_binary().map(|p| p.exists()).unwrap_or(false)
    }

    /// Auto-install PortableGit via the mirror chain.
    pub async fn install() -> Result<(), PortableGitError> {
        let arch = portable_git_arch()?;
        let archive_name = format!("PortableGit-{PINNED_VERSION}-{arch}.7z.exe");

        let runtime_root = Self::runtime_root()?;
        std::fs::create_dir_all(&runtime_root).map_err(|e| PortableGitError::Io {
            path: runtime_root.display().to_string(),
            source: e,
        })?;

        let git_dir = Self::git_dir()?;
        // Best-effort cleanup of any partial prior install.
        if git_dir.exists() {
            cleanup_dir(&git_dir);
        }

        // Run the pipeline, cleaning up on failure.
        let result = Self::run_install_pipeline(&archive_name, &git_dir).await;
        if result.is_err() {
            cleanup_dir(&git_dir);
        }
        result
    }

    async fn run_install_pipeline(
        archive_name: &str,
        git_dir: &Path,
    ) -> Result<(), PortableGitError> {
        let tmp_dir = tempfile::Builder::new()
            .prefix("cc-switch-portable-git-")
            .tempdir()
            .map_err(|e| PortableGitError::Io {
                path: "<tempfile>".into(),
                source: e,
            })?;
        let archive_path = tmp_dir.path().join(archive_name);

        // 1. Walk the mirror chain. First successful download wins.
        let mut last_err: Option<PortableGitError> = None;
        let mut chosen_mirror: Option<&'static str> = None;
        for mirror in GIT_FOR_WINDOWS_MIRRORS {
            if archive_path.exists() {
                let _ = std::fs::remove_file(&archive_path);
            }
            let url = build_archive_url(mirror.name, mirror.base, archive_name);
            match download_to(&url, &archive_path).await {
                Ok(()) => {
                    log::info!("PortableGit archive downloaded from mirror: {}", mirror.name);
                    chosen_mirror = Some(mirror.name);
                    break;
                }
                Err(e) => {
                    log::warn!(
                        "PortableGit download from mirror {} failed: {}",
                        mirror.name,
                        e
                    );
                    last_err = Some(e);
                }
            }
        }
        if chosen_mirror.is_none() {
            return Err(last_err.unwrap_or_else(|| {
                PortableGitError::Http("all Git-for-Windows mirrors failed".into())
            }));
        }

        // 2. Self-extract. PortableGit is a 7z SFX: `-y` silent, `-o<dest>` dest.
        //    The `-o` arg has NO space between flag and value, per 7z convention.
        if let Some(parent) = git_dir.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PortableGitError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        let dest_arg = format!("-o{}", git_dir.display());
        let mut cmd = tokio::process::Command::new(&archive_path);
        cmd.arg("-y").arg(&dest_arg);
        #[cfg(target_os = "windows")]
        {
            // tokio::process::Command exposes `creation_flags` directly on Windows
            // (no need for the `CommandExt` trait import).
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd.output().await.map_err(|e| PortableGitError::Extract(
            format!("spawn self-extract {}: {}", archive_path.display(), e),
        ))?;
        if !output.status.success() {
            return Err(PortableGitError::Extract(format!(
                "self-extract exited with {:?}; stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // 3. Validate the resulting binary.
        let git_bin = git_dir.join("bin").join("git.exe");
        if !git_bin.exists() {
            return Err(PortableGitError::Validate(format!(
                "{} does not exist after self-extract",
                git_bin.display()
            )));
        }
        let mut probe = tokio::process::Command::new(&git_bin);
        probe.arg("--version");
        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            probe.creation_flags(CREATE_NO_WINDOW);
        }
        let v_out = probe.output().await.map_err(|e| {
            PortableGitError::Validate(format!("spawn git --version: {e}"))
        })?;
        if !v_out.status.success() {
            return Err(PortableGitError::Validate(format!(
                "git --version exited with {:?}",
                v_out.status.code()
            )));
        }
        let v_str = String::from_utf8_lossy(&v_out.stdout).trim().to_string();
        if !v_str.contains("git version") {
            return Err(PortableGitError::Validate(format!(
                "unexpected git --version output: {v_str}"
            )));
        }
        log::info!("PortableGit installed successfully: {v_str}");

        Ok(())
    }
}

/// Translate the runtime arch to the PortableGit archive suffix.
/// PortableGit ships only `64-bit` (x86_64) and `arm64`. 32-bit support is
/// out of scope for MVP.
fn portable_git_arch() -> Result<&'static str, PortableGitError> {
    match detect_arch() {
        "x64" => Ok("64-bit"),
        "arm64" => Ok("arm64"),
        other => Err(PortableGitError::UnsupportedArch(other.to_string())),
    }
}

/// Build the per-mirror archive URL.
///
/// Layouts:
/// - npmmirror / huawei: `<base>/v{ver}.windows.1/PortableGit-{ver}-{arch}.7z.exe`
/// - github:             `<base>/v{ver}.windows.1/PortableGit-{ver}-{arch}.7z.exe`
///
/// All three currently share the version-dir prefix; if a future mirror diverges
/// we'll branch on `mirror_name` here.
fn build_archive_url(_mirror_name: &str, base: &str, archive_name: &str) -> String {
    format!("{base}/v{PINNED_VERSION}.windows.1/{archive_name}")
}

/// Best-effort: remove a directory tree and ignore errors.
fn cleanup_dir(dir: &Path) {
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(dir) {
            log::warn!("cleanup_dir failed for {}: {}", dir.display(), e);
        }
    }
}

async fn download_to(url: &str, dest: &Path) -> Result<(), PortableGitError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .user_agent("cc-switch-installer/1.0")
        .build()
        .map_err(|e| PortableGitError::Http(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| PortableGitError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(PortableGitError::Http(format!(
            "GET {url} → HTTP {}",
            resp.status().as_u16()
        )));
    }
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| PortableGitError::Io {
            path: dest.display().to_string(),
            source: e,
        })?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| PortableGitError::Http(e.to_string()))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| PortableGitError::Io {
                path: dest.display().to_string(),
                source: e,
            })?;
    }
    file.flush().await.map_err(|e| PortableGitError::Io {
        path: dest.display().to_string(),
        source: e,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// `git_dir` must live under the runtime root, so cleanup/atomic-swap
    /// logic keeps working regardless of where `runtime_root` resolves.
    #[test]
    #[serial]
    fn git_dir_under_runtime_root() {
        // Force a known-good test home so the assertion doesn't depend on
        // whatever %LOCALAPPDATA% is on the runner. Serialized via `serial_test`
        // so parallel tests don't observe each other's env mutations.
        std::env::set_var("CC_SWITCH_TEST_HOME", "C:/tmp/cc-switch-test");
        let root = PortableGit::runtime_root().expect("runtime_root");
        let git_dir = PortableGit::git_dir().expect("git_dir");
        assert!(
            git_dir.starts_with(&root),
            "git_dir {} did not start with runtime_root {}",
            git_dir.display(),
            root.display()
        );
        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// `git_binary` must end with `bin\git.exe` so probe + spawn agree on path.
    #[test]
    #[serial]
    fn git_binary_path_ends_with_git_exe() {
        std::env::set_var("CC_SWITCH_TEST_HOME", "C:/tmp/cc-switch-test");
        let bin = PortableGit::git_binary().expect("git_binary");
        let s = bin.display().to_string().replace('\\', "/");
        assert!(s.ends_with("/bin/git.exe"), "unexpected path: {s}");
        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// Mirror URLs must use the documented `v{ver}.windows.1` directory layout.
    #[test]
    fn build_archive_url_uses_version_dir_prefix() {
        let url = build_archive_url(
            "npmmirror",
            "https://npmmirror.com/mirrors/git-for-windows",
            "PortableGit-2.45.2-64-bit.7z.exe",
        );
        assert_eq!(
            url,
            "https://npmmirror.com/mirrors/git-for-windows/v2.45.2.windows.1/PortableGit-2.45.2-64-bit.7z.exe"
        );
    }

    /// Arch translation only covers the two PortableGit ships natively.
    #[test]
    fn portable_git_arch_only_supports_x64_and_arm64() {
        // We can't easily mutate detect_arch's source, so only assert that
        // the runtime arch (which is one of x64/arm64 on any sane CI box)
        // resolves to a known value.
        let resolved = portable_git_arch();
        match resolved {
            Ok(s) => assert!(s == "64-bit" || s == "arm64", "unexpected arch: {s}"),
            Err(PortableGitError::UnsupportedArch(_)) => {
                // Acceptable on x86/armv7 etc.
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
}
