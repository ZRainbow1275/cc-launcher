//! Per-group probe modules for the CC Launcher system probe (D-Probe / B4).
//!
//! Each module probes one group of dimensions:
//! - `system`: OS / arch / CPU / memory / disk
//! - `runtime`: Node / npm / Git / PATH
//! - `env`: env conflicts / admin / PS policy / Defender / Rosetta
//! - `network`: HTTP proxy / npm registry reachability
//! - `workdir`: `~/cc-launcher-projects/` existence + write permission
//!
//! The aggregate is wired up in [`crate::services::system_probe::run_probe`].

pub mod env;
pub mod fix_actions;
pub mod network;
pub mod runtime;
pub mod system;
pub mod workdir;
