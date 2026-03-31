use std::path::PathBuf;

use thiserror::Error;

/// Errors while resolving config paths or parsing TOML.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(
        "could not resolve user config directory (set XDG_CONFIG_HOME or use a platform with a known config location)"
    )]
    NoUserConfigDir,

    #[error("failed to read {}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid TOML in {}: {source}", path.display())]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}
