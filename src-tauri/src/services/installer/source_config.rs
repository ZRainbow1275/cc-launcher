use thiserror::Error;
use url::Url;

use crate::services::installer::mirrors::{GIT_FOR_WINDOWS_MIRRORS, NODE_DIST_MIRRORS};
use crate::services::installer::registry_probe::RegistryDef;

/// Persisted installer source configuration.
///
/// `None` means "use the built-in mirror chain". `Some(url)` means "try this
/// exact source first, then fall back to the built-ins".
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallerSourceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm_registry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_dist_mirror: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_for_windows_mirror: Option<String>,
}

#[derive(Debug, Error)]
pub enum InstallerSourceConfigError {
    #[error("{field} must be a valid http(s) URL: {value}")]
    InvalidUrl { field: String, value: String },
    #[error("{field} must use http or https: {value}")]
    InvalidScheme { field: String, value: String },
}

pub const CUSTOM_SOURCE_NAME: &str = "custom";

impl InstallerSourceConfig {
    /// Normalize and validate all configured URLs.
    pub fn validated(&self) -> Result<Self, InstallerSourceConfigError> {
        Ok(Self {
            npm_registry: normalize_url(self.npm_registry.as_deref(), "npmRegistry")?,
            node_dist_mirror: normalize_url(self.node_dist_mirror.as_deref(), "nodeDistMirror")?,
            git_for_windows_mirror: normalize_url(
                self.git_for_windows_mirror.as_deref(),
                "gitForWindowsMirror",
            )?,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.npm_registry.is_none()
            && self.node_dist_mirror.is_none()
            && self.git_for_windows_mirror.is_none()
    }
}

fn normalize_url(
    value: Option<&str>,
    field: &str,
) -> Result<Option<String>, InstallerSourceConfigError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed = Url::parse(trimmed).map_err(|_| InstallerSourceConfigError::InvalidUrl {
        field: field.to_string(),
        value: trimmed.to_string(),
    })?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(InstallerSourceConfigError::InvalidScheme {
                field: field.to_string(),
                value: trimmed.to_string(),
            })
        }
    }
    let normalized = trimmed.trim_end_matches('/').to_string();
    Ok(Some(normalized))
}

/// One owned mirror endpoint. Built-in static entries are converted into this
/// shape so custom sources can be inserted at the front of the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorEndpoint {
    pub name: String,
    pub base: String,
}

impl MirrorEndpoint {
    pub fn new(name: impl Into<String>, base: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base: base.into(),
        }
    }

    pub fn from_custom(base: impl Into<String>) -> Self {
        Self::new(CUSTOM_SOURCE_NAME, base)
    }

    pub fn dist_index_url(&self) -> String {
        format!("{}/index.json", self.base)
    }

    pub fn archive_url(&self, version: &str, archive_name: &str) -> String {
        format!("{}/{}/{}", self.base, version, archive_name)
    }

    pub fn shasums_url(&self, version: &str) -> String {
        format!("{}/{}/SHASUMS256.txt", self.base, version)
    }
}

impl From<&super::mirrors::NodeDistMirror> for MirrorEndpoint {
    fn from(value: &super::mirrors::NodeDistMirror) -> Self {
        Self::new(value.name, value.base)
    }
}

impl From<&super::mirrors::GitForWindowsMirror> for MirrorEndpoint {
    fn from(value: &super::mirrors::GitForWindowsMirror) -> Self {
        Self::new(value.name, value.base)
    }
}

pub fn node_dist_mirror_chain(config: &InstallerSourceConfig) -> Vec<MirrorEndpoint> {
    let mut mirrors = Vec::new();
    if let Some(base) = config.node_dist_mirror.as_deref() {
        mirrors.push(MirrorEndpoint::from_custom(base));
    }
    mirrors.extend(NODE_DIST_MIRRORS.iter().map(MirrorEndpoint::from));
    mirrors
}

pub fn git_for_windows_mirror_chain(config: &InstallerSourceConfig) -> Vec<MirrorEndpoint> {
    let mut mirrors = Vec::new();
    if let Some(base) = config.git_for_windows_mirror.as_deref() {
        mirrors.push(MirrorEndpoint::from_custom(base));
    }
    mirrors.extend(GIT_FOR_WINDOWS_MIRRORS.iter().map(MirrorEndpoint::from));
    mirrors
}

pub fn registry_endpoint_chain(
    config: &InstallerSourceConfig,
    builtins: &[RegistryDef],
) -> Vec<MirrorEndpoint> {
    let mut endpoints = Vec::new();
    if let Some(base) = config.npm_registry.as_deref() {
        endpoints.push(MirrorEndpoint::from_custom(base));
    }
    endpoints.extend(
        builtins
            .iter()
            .map(|def| MirrorEndpoint::new(def.name, def.url)),
    );
    endpoints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_trims_configured_sources() {
        let config = InstallerSourceConfig {
            npm_registry: Some(" https://vps.example.com/npm/ ".into()),
            node_dist_mirror: Some("https://vps.example.com/node///".into()),
            git_for_windows_mirror: Some(String::new()),
        };

        let validated = config.validated().unwrap();

        assert_eq!(
            validated.npm_registry.as_deref(),
            Some("https://vps.example.com/npm")
        );
        assert_eq!(
            validated.node_dist_mirror.as_deref(),
            Some("https://vps.example.com/node")
        );
        assert_eq!(validated.git_for_windows_mirror, None);
    }

    #[test]
    fn rejects_non_http_sources() {
        let config = InstallerSourceConfig {
            npm_registry: Some("file:///tmp/npm".into()),
            ..InstallerSourceConfig::default()
        };

        let err = config.validated().unwrap_err();
        assert!(matches!(
            err,
            InstallerSourceConfigError::InvalidScheme { .. }
        ));
    }

    #[test]
    fn prepends_custom_sources_to_each_chain() {
        let config = InstallerSourceConfig {
            npm_registry: Some("https://vps.example.com/npm".into()),
            node_dist_mirror: Some("https://vps.example.com/node".into()),
            git_for_windows_mirror: Some("https://vps.example.com/git".into()),
        };
        let registries = registry_endpoint_chain(
            &config,
            &[RegistryDef {
                name: "builtin",
                url: "https://registry.example.com",
            }],
        );

        assert_eq!(node_dist_mirror_chain(&config)[0].name, CUSTOM_SOURCE_NAME);
        assert_eq!(
            node_dist_mirror_chain(&config)[0].base,
            "https://vps.example.com/node"
        );
        assert_eq!(
            git_for_windows_mirror_chain(&config)[0].base,
            "https://vps.example.com/git"
        );
        assert_eq!(registries[0].name, CUSTOM_SOURCE_NAME);
        assert_eq!(registries[0].base, "https://vps.example.com/npm");
        assert_eq!(registries[1].name, "builtin");
    }
}
