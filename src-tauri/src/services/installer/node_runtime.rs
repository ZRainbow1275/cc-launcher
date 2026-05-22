//! Private Node 20 LTS runtime download / extract / verify.
//!
//! Falls into the strict "user-level, never touch system PATH" doctrine
//! (research §"Node.js 前置依赖策略").
//!
//! Target directories (via `dirs::data_local_dir()`):
//! - Windows: `%LOCALAPPDATA%\cc-switch\runtime\node\`
//! - macOS:   `~/Library/Application Support/cc-switch/runtime/node/`
//! - Linux:   `$XDG_DATA_HOME/cc-switch/runtime/node/`
//!
//! Install pipeline streams the 4 phases the frontend expects via Tauri Channel:
//! `probing-registry → installing-node → validating → completed` (or `failed`).
//!
//! Concrete steps inside `installing-node`:
//! 1. Resolve the *latest 20.x* version from `nodejs.org/dist/index.json`
//! 2. Download archive (.zip on Windows, .tar.xz on Unix) to a temp file
//! 3. Verify SHA-256 against `SHASUMS256.txt` for the same version
//! 4. Atomically swap into the runtime dir
//!
//! On any error: cleanup partial state and yield `phase=failed` with localized error.

use std::path::{Path, PathBuf};

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

use super::registry_probe::{RegistryProbeError, RegistryProbeService};

/// Frontend-facing `NodeStatus` (camelCase JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatus {
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub is_private_runtime: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub major_version: Option<u32>,
}

/// Localized progress message (mirrors frontend `LocalizedString`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedMessage {
    pub zh: String,
    pub en: String,
    pub ja: String,
}

/// Mirrors frontend `TypedError`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedError {
    pub code: String,
    pub message: LocalizedMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    pub retryable: bool,
}

/// Mirrors frontend `InstallPhase` enum (kebab-case wire values).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallPhase {
    ProbingRegistry,
    InstallingNode,
    InstallingCli,
    Validating,
    Completed,
    Failed,
}

/// Mirrors frontend `InstallProgress` (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub phase: InstallPhase,
    pub message: LocalizedMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TypedError>,
}

#[derive(Debug, Error)]
pub enum NodeRuntimeError {
    #[error("registry probe failed: {0}")]
    Registry(#[from] RegistryProbeError),
    #[error("HTTP fetch failed: {0}")]
    Http(String),
    #[error("IO error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("SHA-256 mismatch for {file}: expected {expected}, got {actual}")]
    ShaMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    #[error("archive extraction failed: {0}")]
    Extract(String),
    #[error("could not resolve a Node 20 LTS release")]
    NoRelease,
    #[error("data_local_dir unavailable on this platform")]
    NoDataDir,
    #[error("post-install validation failed: {0}")]
    Validate(String),
}

/// Single API surface for Node detection & install.
pub struct NodeRuntime;

impl NodeRuntime {
    /// Root for the private runtime tree (`<data_local>/cc-switch/runtime`).
    pub fn runtime_root() -> Result<PathBuf, NodeRuntimeError> {
        // Use CC_SWITCH_TEST_HOME for test isolation (mirrors `config::get_home_dir`).
        if let Ok(override_dir) = std::env::var("CC_SWITCH_TEST_HOME") {
            let trimmed = override_dir.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed).join("cc-switch").join("runtime"));
            }
        }
        let base = dirs::data_local_dir().ok_or(NodeRuntimeError::NoDataDir)?;
        Ok(base.join("cc-switch").join("runtime"))
    }

    /// Where the private Node tree lives.
    pub fn node_dir() -> Result<PathBuf, NodeRuntimeError> {
        Ok(Self::runtime_root()?.join("node"))
    }

    /// Path to the private node binary.
    pub fn node_binary() -> Result<PathBuf, NodeRuntimeError> {
        let dir = Self::node_dir()?;
        #[cfg(target_os = "windows")]
        {
            Ok(dir.join("node.exe"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(dir.join("bin").join("node"))
        }
    }

    /// Path to npm-cli.js inside the private Node tree (used to spawn npm portably).
    pub fn npm_cli_js() -> Result<PathBuf, NodeRuntimeError> {
        let dir = Self::node_dir()?;
        #[cfg(target_os = "windows")]
        {
            // Windows Node zip layout: node.exe + node_modules/npm/bin/npm-cli.js (top-level)
            Ok(dir
                .join("node_modules")
                .join("npm")
                .join("bin")
                .join("npm-cli.js"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Unix tarball layout: bin/node + lib/node_modules/npm/bin/npm-cli.js
            Ok(dir
                .join("lib")
                .join("node_modules")
                .join("npm")
                .join("bin")
                .join("npm-cli.js"))
        }
    }

    /// Lightweight detection — does NOT touch network.
    ///
    /// Reports `installed = true` only when the private runtime exists AND the
    /// `node` binary spawned successfully, mirroring frontend `detect_node` mock.
    pub async fn detect() -> NodeStatus {
        match Self::node_binary() {
            Ok(node) if node.exists() => {
                let version = Self::probe_node_version(&node).await;
                let major = version.as_deref().and_then(parse_major_version);
                NodeStatus {
                    installed: version.is_some(),
                    version,
                    path: Some(node.to_string_lossy().into_owned()),
                    is_private_runtime: true,
                    major_version: major,
                }
            }
            _ => NodeStatus {
                installed: false,
                version: None,
                path: None,
                is_private_runtime: true,
                major_version: None,
            },
        }
    }

    /// Returns `v20.11.0` (or similar) by spawning `node -v`.
    async fn probe_node_version(node: &Path) -> Option<String> {
        let output = tokio::process::Command::new(node)
            .arg("-v")
            .output()
            .await
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let s = String::from_utf8(output.stdout).ok()?;
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    /// Streaming install. `on_progress` receives the `InstallProgress` events.
    ///
    /// This is the *core* function. Outer commands (Tauri) wrap a `Channel` over it.
    pub async fn install<F>(mut on_progress: F) -> Result<NodeStatus, NodeRuntimeError>
    where
        F: FnMut(InstallProgress) + Send,
    {
        // Phase 1: probing-registry
        on_progress(progress(
            InstallPhase::ProbingRegistry,
            l(
                "正在选择镜像源...",
                "Probing npm registries...",
                "npm レジストリを探索中...",
            ),
            Some(5),
            None,
            None,
        ));

        // Node dist mirrors aren't on npm — we use the official site directly here.
        // (Smart-mirror for the Node binary itself is a future polish; npm mirror still
        // picked so the same pick can be reused for CLI install.)
        let pick = RegistryProbeService::smart_pick(false).await?;
        let chosen_registry = pick.chosen.clone();

        // Phase 2: installing-node — download + verify + extract.
        on_progress(progress(
            InstallPhase::InstallingNode,
            l(
                "正在下载 Node.js 20 LTS...",
                "Downloading Node.js 20 LTS...",
                "Node.js 20 LTS をダウンロード中...",
            ),
            Some(20),
            Some(chosen_registry.clone()),
            None,
        ));

        let runtime_root = Self::runtime_root()?;
        std::fs::create_dir_all(&runtime_root).map_err(|e| NodeRuntimeError::Io {
            path: runtime_root.display().to_string(),
            source: e,
        })?;

        let target_dir = Self::node_dir()?;

        // Best-effort cleanup of any partial prior install before we start.
        if target_dir.exists() {
            cleanup_dir(&target_dir);
        }

        let install_result = Self::run_install_pipeline(&target_dir, &mut on_progress).await;

        if let Err(ref err) = install_result {
            // Failure → cleanup partial + emit failed event.
            cleanup_dir(&target_dir);
            on_progress(progress(
                InstallPhase::Failed,
                l(
                    "Node.js 安装失败，已清理残留",
                    "Node.js install failed, partial files cleaned",
                    "Node.js のインストールに失敗しました。残骸を削除しました",
                ),
                Some(0),
                Some(chosen_registry.clone()),
                Some(TypedError {
                    code: "NODE_INSTALL_FAILED".to_string(),
                    message: l(
                        "Node.js 安装失败",
                        "Node.js installation failed",
                        "Node.js のインストールに失敗しました",
                    ),
                    cause: Some(err.to_string()),
                    retryable: true,
                }),
            ));
        }
        let _version = install_result?;

        // Phase 3: validating — spawn `node -v` against the new binary.
        on_progress(progress(
            InstallPhase::Validating,
            l(
                "正在校验 Node.js 安装...",
                "Validating Node.js installation...",
                "Node.js を検証中...",
            ),
            Some(90),
            Some(chosen_registry.clone()),
            None,
        ));

        let node_path = Self::node_binary()?;
        let detected_version = Self::probe_node_version(&node_path)
            .await
            .ok_or_else(|| NodeRuntimeError::Validate("`node -v` did not return output".into()))?;
        let major = parse_major_version(&detected_version).ok_or_else(|| {
            NodeRuntimeError::Validate(format!("cannot parse version: {detected_version}"))
        })?;
        if major < 20 {
            return Err(NodeRuntimeError::Validate(format!(
                "expected Node 20.x, got {detected_version}"
            )));
        }

        // Phase 4: completed.
        on_progress(progress(
            InstallPhase::Completed,
            l("安装完成", "Installation completed", "インストール完了"),
            Some(100),
            Some(chosen_registry),
            None,
        ));

        Ok(NodeStatus {
            installed: true,
            version: Some(detected_version),
            path: Some(node_path.to_string_lossy().into_owned()),
            is_private_runtime: true,
            major_version: Some(major),
        })
    }

    async fn run_install_pipeline<F>(
        target_dir: &Path,
        on_progress: &mut F,
    ) -> Result<String, NodeRuntimeError>
    where
        F: FnMut(InstallProgress) + Send,
    {
        // 1. Resolve latest Node 20 LTS version
        let version = resolve_latest_v20().await?;

        // 2. Determine archive URL + platform suffix
        let (archive_url, archive_name) = node_archive_url(&version)?;
        let shasum_url = format!("https://nodejs.org/dist/{version}/SHASUMS256.txt");

        // 3. Download archive
        let tmp_dir = tempfile::Builder::new()
            .prefix("cc-switch-node-")
            .tempdir()
            .map_err(|e| NodeRuntimeError::Io {
                path: "<tempfile>".into(),
                source: e,
            })?;
        let archive_path = tmp_dir.path().join(&archive_name);
        download_to(&archive_url, &archive_path).await?;

        // 4. Fetch SHASUMS and verify
        let shasums = fetch_text(&shasum_url).await?;
        let expected_sha = shasums
            .lines()
            .find_map(|line| {
                let mut parts = line.split_whitespace();
                let sha = parts.next()?;
                let file = parts.next()?;
                if file == archive_name {
                    Some(sha.to_string())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                NodeRuntimeError::Validate(format!(
                    "SHASUMS256.txt for {version} has no entry for {archive_name}"
                ))
            })?;

        on_progress(progress(
            InstallPhase::Validating,
            l(
                "正在校验下载内容...",
                "Verifying download integrity...",
                "ダウンロードを検証中...",
            ),
            Some(60),
            None,
            None,
        ));

        let actual_sha = sha256_file(&archive_path).await?;
        if !actual_sha.eq_ignore_ascii_case(&expected_sha) {
            return Err(NodeRuntimeError::ShaMismatch {
                file: archive_name.clone(),
                expected: expected_sha,
                actual: actual_sha,
            });
        }

        // 5. Extract archive into target_dir (atomic-ish: extract to tmp then rename)
        on_progress(progress(
            InstallPhase::InstallingNode,
            l(
                "正在解压 Node.js...",
                "Extracting Node.js archive...",
                "Node.js を展開中...",
            ),
            Some(75),
            None,
            None,
        ));
        let stage_dir = tmp_dir.path().join("stage");
        std::fs::create_dir_all(&stage_dir).map_err(|e| NodeRuntimeError::Io {
            path: stage_dir.display().to_string(),
            source: e,
        })?;
        extract_archive(&archive_path, &stage_dir)?;

        // Node archive root usually = "node-v20.x.y-<platform>-<arch>". Flatten it.
        let inner_root = single_root_dir(&stage_dir)?;
        // Move stage_dir/<inner_root> → target_dir
        if let Some(parent) = target_dir.parent() {
            std::fs::create_dir_all(parent).map_err(|e| NodeRuntimeError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        // Best-effort: clean target before rename to avoid Windows ERROR_DIR_NOT_EMPTY.
        if target_dir.exists() {
            cleanup_dir(target_dir);
        }
        std::fs::rename(&inner_root, target_dir).map_err(|e| NodeRuntimeError::Io {
            path: format!("{} -> {}", inner_root.display(), target_dir.display()),
            source: e,
        })?;

        Ok(version)
    }
}

fn progress(
    phase: InstallPhase,
    message: LocalizedMessage,
    percent: Option<u8>,
    registry: Option<String>,
    error: Option<TypedError>,
) -> InstallProgress {
    InstallProgress {
        phase,
        message,
        percent,
        registry,
        error,
    }
}

pub fn l(zh: &str, en: &str, ja: &str) -> LocalizedMessage {
    LocalizedMessage {
        zh: zh.to_string(),
        en: en.to_string(),
        ja: ja.to_string(),
    }
}

fn parse_major_version(v: &str) -> Option<u32> {
    let t = v.trim().trim_start_matches('v');
    t.split('.').next()?.parse::<u32>().ok()
}

/// Best-effort: remove a directory tree and ignore errors (we logged the cause already).
fn cleanup_dir(dir: &Path) {
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(dir) {
            log::warn!("cleanup_dir failed for {}: {}", dir.display(), e);
        }
    }
}

#[derive(Debug, Deserialize)]
struct DistIndexEntry {
    version: String,
    lts: serde_json::Value,
}

async fn resolve_latest_v20() -> Result<String, NodeRuntimeError> {
    let url = "https://nodejs.org/dist/index.json";
    let body = fetch_text(url).await?;
    let entries: Vec<DistIndexEntry> = serde_json::from_str(&body)
        .map_err(|e| NodeRuntimeError::Http(format!("parse dist/index.json failed: {e}")))?;
    let v20 = entries
        .into_iter()
        .find(|e| {
            // entry.version is "v20.x.y"
            e.version.starts_with("v20.") && matches!(&e.lts, serde_json::Value::String(_))
        })
        .ok_or(NodeRuntimeError::NoRelease)?;
    Ok(v20.version)
}

fn node_archive_url(version: &str) -> Result<(String, String), NodeRuntimeError> {
    #[cfg(target_os = "windows")]
    let suffix = if cfg!(target_arch = "aarch64") {
        "win-arm64.zip"
    } else {
        "win-x64.zip"
    };
    #[cfg(target_os = "macos")]
    let suffix = if cfg!(target_arch = "aarch64") {
        "darwin-arm64.tar.xz"
    } else {
        "darwin-x64.tar.xz"
    };
    #[cfg(target_os = "linux")]
    let suffix = if cfg!(target_arch = "aarch64") {
        "linux-arm64.tar.xz"
    } else {
        "linux-x64.tar.xz"
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let suffix: &str = return Err(NodeRuntimeError::NoRelease);

    let name = format!("node-{version}-{suffix}");
    let url = format!("https://nodejs.org/dist/{version}/{name}");
    Ok((url, name))
}

async fn fetch_text(url: &str) -> Result<String, NodeRuntimeError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("cc-switch-installer/1.0")
        .build()
        .map_err(|e| NodeRuntimeError::Http(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| NodeRuntimeError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(NodeRuntimeError::Http(format!(
            "GET {url} → HTTP {}",
            resp.status().as_u16()
        )));
    }
    resp.text()
        .await
        .map_err(|e| NodeRuntimeError::Http(e.to_string()))
}

async fn download_to(url: &str, dest: &Path) -> Result<(), NodeRuntimeError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .user_agent("cc-switch-installer/1.0")
        .build()
        .map_err(|e| NodeRuntimeError::Http(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| NodeRuntimeError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(NodeRuntimeError::Http(format!(
            "GET {url} → HTTP {}",
            resp.status().as_u16()
        )));
    }
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| NodeRuntimeError::Io {
            path: dest.display().to_string(),
            source: e,
        })?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| NodeRuntimeError::Http(e.to_string()))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| NodeRuntimeError::Io {
                path: dest.display().to_string(),
                source: e,
            })?;
    }
    file.flush().await.map_err(|e| NodeRuntimeError::Io {
        path: dest.display().to_string(),
        source: e,
    })?;
    Ok(())
}

async fn sha256_file(path: &Path) -> Result<String, NodeRuntimeError> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| NodeRuntimeError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .await
            .map_err(|e| NodeRuntimeError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn extract_archive(archive: &Path, dest: &Path) -> Result<(), NodeRuntimeError> {
    let name = archive
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    if name.ends_with(".zip") {
        extract_zip(archive, dest)
    } else if name.ends_with(".tar.xz") {
        extract_tar_xz(archive, dest)
    } else {
        Err(NodeRuntimeError::Extract(format!(
            "unsupported archive: {name}"
        )))
    }
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), NodeRuntimeError> {
    let file = std::fs::File::open(archive).map_err(|e| NodeRuntimeError::Io {
        path: archive.display().to_string(),
        source: e,
    })?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| NodeRuntimeError::Extract(format!("open zip: {e}")))?;
    zip.extract(dest)
        .map_err(|e| NodeRuntimeError::Extract(format!("extract zip: {e}")))?;
    Ok(())
}

fn extract_tar_xz(archive: &Path, dest: &Path) -> Result<(), NodeRuntimeError> {
    let file = std::fs::File::open(archive).map_err(|e| NodeRuntimeError::Io {
        path: archive.display().to_string(),
        source: e,
    })?;
    let xz = xz2::read::XzDecoder::new(file);
    let mut tar = tar::Archive::new(xz);
    tar.unpack(dest)
        .map_err(|e| NodeRuntimeError::Extract(format!("extract tar.xz: {e}")))?;
    Ok(())
}

/// After extraction Node archives usually contain a single top-level directory
/// named `node-v20.x.y-platform-arch`. Return its absolute path.
fn single_root_dir(parent: &Path) -> Result<PathBuf, NodeRuntimeError> {
    let mut iter = std::fs::read_dir(parent).map_err(|e| NodeRuntimeError::Io {
        path: parent.display().to_string(),
        source: e,
    })?;
    let mut found: Option<PathBuf> = None;
    for entry in iter.by_ref() {
        let entry = entry.map_err(|e| NodeRuntimeError::Io {
            path: parent.display().to_string(),
            source: e,
        })?;
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if found.is_some() {
                return Err(NodeRuntimeError::Extract(format!(
                    "expected single root dir in {}, found multiple",
                    parent.display()
                )));
            }
            found = Some(entry.path());
        }
    }
    found.ok_or_else(|| {
        NodeRuntimeError::Extract(format!("no extracted root in {}", parent.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_major_version_handles_v_prefix_and_digits() {
        assert_eq!(parse_major_version("v20.11.0"), Some(20));
        assert_eq!(parse_major_version("20.11.0"), Some(20));
        assert_eq!(parse_major_version("v18.19.1"), Some(18));
        assert_eq!(parse_major_version("not-a-version"), None);
        assert_eq!(parse_major_version(""), None);
    }

    #[test]
    fn node_archive_url_picks_platform_suffix() {
        let (url, name) = node_archive_url("v20.11.0").unwrap();
        assert!(url.contains("v20.11.0"));
        assert!(url.ends_with(&name));
        #[cfg(target_os = "windows")]
        assert!(name.contains("win-"));
        #[cfg(target_os = "macos")]
        assert!(name.contains("darwin-"));
    }
}
