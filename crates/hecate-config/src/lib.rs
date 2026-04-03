//! Configuration paths, TOML merging, and `{hecate_root}/metadata.json`.
//!
//! User config: `{config_dir}/hecate/config.toml` ([`dirs::config_dir`], i.e.
//! `XDG_CONFIG_HOME/hecate/config.toml` on Linux).
//! Repo config: `{repo_root}/.hecate/config.toml`.
//!
//! Precedence: **`HECATE_ROOT`** overrides TOML; **repo TOML** overrides **user
//! TOML**; missing files are ignored.

mod error;
mod load;
mod metadata;
mod repo_segment;
mod resolve;
mod types;

pub use error::ConfigError;
pub use load::{LoadOptions, load, load_without_env_hecate_root};
pub use metadata::{
    METADATA_VERSION, MetadataError, MetadataFile, WorktreeRecord, metadata_path, read_metadata,
    write_metadata,
};
pub use repo_segment::{choose_repo_segment, clone_identity_key, segment_dir_used_by_other_clones};
pub use resolve::{ResolveHecateRootError, resolve_hecate_root};

use std::path::PathBuf;

/// Fully merged configuration from all layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub hecate_root: Option<PathBuf>,
}
