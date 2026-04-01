use std::env;
use std::path::{Path, PathBuf};

use crate::ResolvedConfig;
use crate::error::ConfigError;
use crate::types::FileConfig;

const ENV_HECATE_ROOT: &str = "HECATE_ROOT";

/// Where to look for config files.
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// Git repo root; when set, merges `.hecate/config.toml` after the user file.
    pub repo_root: Option<PathBuf>,
    /// When set, used instead of [`dirs::config_dir`] (e.g. tests).
    pub config_home_override: Option<PathBuf>,
}

/// Loads and merges config files, then applies `HECATE_ROOT` if set.
///
/// **Precedence (later wins for `hecate_root`):** user file, then repo file,
/// then `HECATE_ROOT`.
pub fn load(options: &LoadOptions) -> Result<ResolvedConfig, ConfigError> {
    let from_env = env::var_os(ENV_HECATE_ROOT).map(PathBuf::from);
    load_merged(options, from_env)
}

/// Like [`load`], but uses `env_hecate_root` instead of reading the process
/// environment (for tests and embedding).
pub(crate) fn load_merged(
    options: &LoadOptions,
    env_hecate_root: Option<PathBuf>,
) -> Result<ResolvedConfig, ConfigError> {
    let mut acc = FileConfig::default();

    if let Some(cfg) = load_user_file(options)? {
        acc.merge(cfg);
    }
    if let Some(cfg) = load_repo_file(options)? {
        acc.merge(cfg);
    }

    if let Some(v) = env_hecate_root {
        acc.hecate_root = Some(v);
    }

    Ok(ResolvedConfig {
        hecate_root: acc.hecate_root,
    })
}

fn user_config_path(options: &LoadOptions) -> Result<PathBuf, ConfigError> {
    let base = options
        .config_home_override
        .clone()
        .or_else(dirs::config_dir)
        .ok_or(ConfigError::NoUserConfigDir)?;
    Ok(base.join("hecate").join("config.toml"))
}

fn load_user_file(options: &LoadOptions) -> Result<Option<FileConfig>, ConfigError> {
    read_layer(&user_config_path(options)?)
}

fn load_repo_file(options: &LoadOptions) -> Result<Option<FileConfig>, ConfigError> {
    let Some(root) = &options.repo_root else {
        return Ok(None);
    };
    let path = root.join(".hecate").join("config.toml");
    read_layer(&path)
}

fn read_layer(path: &Path) -> Result<Option<FileConfig>, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(s) => toml::from_str(&s)
            .map(Some)
            .map_err(|source| ConfigError::Toml {
                path: path.to_path_buf(),
                source,
            }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ConfigError::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConfigError;
    use std::fs;

    fn write_user(home: &Path, contents: &str) {
        let dir = home.join("hecate");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.toml"), contents).unwrap();
    }

    #[test]
    fn empty_config_when_no_files() {
        let tmp = tempfile::tempdir().unwrap();
        let opts = LoadOptions {
            config_home_override: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let c = load_merged(&opts, None).unwrap();
        assert!(c.hecate_root.is_none());
    }

    #[test]
    fn legacy_worktree_base_key_still_loads() {
        let tmp = tempfile::tempdir().unwrap();
        write_user(tmp.path(), r#"worktree_base = "/legacy""#);
        let opts = LoadOptions {
            config_home_override: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let c = load_merged(&opts, None).unwrap();
        assert_eq!(c.hecate_root, Some(PathBuf::from("/legacy")));
    }

    #[test]
    fn user_file_sets_hecate_root() {
        let tmp = tempfile::tempdir().unwrap();
        write_user(
            tmp.path(),
            r#"
hecate_root = "/from/user"
"#,
        );
        let opts = LoadOptions {
            config_home_override: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let c = load_merged(&opts, None).unwrap();
        assert_eq!(c.hecate_root, Some(PathBuf::from("/from/user")));
    }

    #[test]
    fn repo_overrides_user() {
        let home = tempfile::tempdir().unwrap();
        write_user(home.path(), r#"hecate_root = "/from/user""#);
        let repo = tempfile::tempdir().unwrap();
        let hecate = repo.path().join(".hecate");
        fs::create_dir_all(&hecate).unwrap();
        fs::write(hecate.join("config.toml"), r#"hecate_root = "/from/repo""#).unwrap();

        let opts = LoadOptions {
            config_home_override: Some(home.path().to_path_buf()),
            repo_root: Some(repo.path().to_path_buf()),
        };
        let c = load_merged(&opts, None).unwrap();
        assert_eq!(c.hecate_root, Some(PathBuf::from("/from/repo")));
    }

    #[test]
    fn env_overrides_files() {
        let home = tempfile::tempdir().unwrap();
        write_user(home.path(), r#"hecate_root = "/from/user""#);
        let repo = tempfile::tempdir().unwrap();
        let hecate = repo.path().join(".hecate");
        fs::create_dir_all(&hecate).unwrap();
        fs::write(hecate.join("config.toml"), r#"hecate_root = "/from/repo""#).unwrap();

        let opts = LoadOptions {
            config_home_override: Some(home.path().to_path_buf()),
            repo_root: Some(repo.path().to_path_buf()),
        };
        let c = load_merged(&opts, Some(PathBuf::from("/from/env"))).unwrap();

        assert_eq!(c.hecate_root, Some(PathBuf::from("/from/env")));
    }

    #[test]
    fn invalid_toml_errors() {
        let tmp = tempfile::tempdir().unwrap();
        write_user(tmp.path(), "not toml {{{");
        let opts = LoadOptions {
            config_home_override: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        assert!(load_merged(&opts, None).is_err());
    }

    #[test]
    fn invalid_toml_yields_toml_error_variant() {
        let tmp = tempfile::tempdir().unwrap();
        write_user(tmp.path(), "hecate_root = ");
        let opts = LoadOptions {
            config_home_override: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let err = load_merged(&opts, None).unwrap_err();
        assert!(
            matches!(err, ConfigError::Toml { .. }),
            "expected ConfigError::Toml, got {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("invalid TOML"),
            "message should mention TOML: {msg}"
        );
    }

    /// `config.toml` must be a file; if it is a directory, reading fails with `Read`.
    #[test]
    fn user_config_path_is_directory_yields_read_error() {
        let tmp = tempfile::tempdir().unwrap();
        let hecate = tmp.path().join("hecate");
        let bogus = hecate.join("config.toml");
        fs::create_dir_all(&bogus).unwrap();

        let opts = LoadOptions {
            config_home_override: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let err = load_merged(&opts, None).unwrap_err();
        assert!(
            matches!(err, ConfigError::Read { .. }),
            "expected ConfigError::Read, got {err:?}"
        );
    }
}
