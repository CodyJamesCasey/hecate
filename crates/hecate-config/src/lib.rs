//! Configuration paths, merging, and (later) `.hecate/metadata.json`.
//!
//! User config: `{config_dir}/hecate/config.toml` ([`dirs::config_dir`], i.e.
//! `XDG_CONFIG_HOME/hecate/config.toml` on Linux).
//! Repo config: `{repo_root}/.hecate/config.toml`.
//!
//! Precedence: **`HECATE_ROOT`** overrides TOML; **repo TOML** overrides **user
//! TOML**; missing files are ignored.

mod error;
mod load;
mod types;

pub use error::ConfigError;
pub use load::{LoadOptions, load};

use std::path::PathBuf;

/// Fully merged configuration from all layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub hecate_root: Option<PathBuf>,
}
