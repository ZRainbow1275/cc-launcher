//! Private Node 20 LTS + npm-only CLI installer.
//!
//! Implementation aligned with `.trellis/tasks/05-21-cc-launcher-mvp/research/cli-install-strategy.md`
//! and frontend contract in `src/lib/api/contracts.ts`.
//!
//! Sub-modules:
//! - `registry_probe`: 4-registry parallel HEAD/GET probe + smart sort.
//! - `node_runtime`: private Node 20 LTS download + SHA-256 verify + extract.
//! - `cli_install`: npm prefix isolation install + 6-step rollback + version validation.

pub mod cli_install;
pub mod mirrors;
pub mod node_runtime;
#[cfg(target_os = "windows")]
pub mod portable_git;
pub mod registry_probe;

// Re-exports kept for the top-level `services::installer::*` ergonomics.
// Marked `#[allow(unused_imports)]` because consumers may pin to a deeper path.
#[allow(unused_imports)]
pub use cli_install::{CliInstaller, InstallOpts, TargetCli};
#[allow(unused_imports)]
pub use mirrors::{
    detect_arch, GitForWindowsMirror, NodeDistMirror, GIT_FOR_WINDOWS_MIRRORS, NODE_DIST_MIRRORS,
};
#[allow(unused_imports)]
pub use node_runtime::NodeRuntime;
#[allow(unused_imports)]
pub use registry_probe::{RegistryProbeService, REGISTRY_DEFS};
