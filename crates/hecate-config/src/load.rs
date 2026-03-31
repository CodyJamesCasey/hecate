use std::env;
use std::path::{Path, PathBuf};

use crate::ResolvedConfig;
use crate::error::ConfigError;
use crate::types::FileConfig;

const ENV_WORKTREE_BASE: &str = "HECATE_WORKTREE_BASE";

/// Where to look for config files.
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// Git repo root; when set, merges `.hecate/config.toml` after the user file.
    pub repo_root: Option<PathBuf>,
    /// When set, used instead of [`dirs::config_dir`] (e.g. tests).
    pub config_home_override: Option<PathBuf>,
}

/// Loads and merges config files, then applies `HECATE_WORKTREE_BASE` if set.
///
/// **Precedence (later wins for `worktree_base`):** user file, then repo file,
/// then `HECATE_WORKTREE_BASE`.
pub fn load(options: &LoadOptions) -> Result<ResolvedConfig, ConfigError> {
    let from_env = env::var_os(ENV_WORKTREE_BASE).map(PathBuf::from);
    load_merged(options, from_env)
}

/// Like [`load`], but uses `env_worktree_base` instead of reading the process
/// environment (for tests and embedding).
pub(crate) fn load_merged(
    options: &LoadOptions,
    env_worktree_base: Option<PathBuf>,
) -> Result<ResolvedConfig, ConfigError> {
    let mut acc = FileConfig::default();

    if let Some(cfg) = load_user_file(options)? {
        acc.merge(cfg);
    }
    if let Some(cfg) = load_repo_file(options)? {
        acc.merge(cfg);
    }

    if let Some(v) = env_worktree_base {
        acc.worktree_base = Some(v);
    }

    Ok(ResolvedConfig {
        worktree_base: acc.worktree_base,
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
        assert!(c.worktree_base.is_none());
    }

    #[test]
    fn user_file_sets_worktree_base() {
        let tmp = tempfile::tempdir().unwrap();
        write_user(
            tmp.path(),
            r#"
worktree_base = "/from/user"
"#,
        );
        let opts = LoadOptions {
            config_home_override: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let c = load_merged(&opts, None).unwrap();
        assert_eq!(c.worktree_base, Some(PathBuf::from("/from/user")));
    }

    #[test]
    fn repo_overrides_user() {
        let home = tempfile::tempdir().unwrap();
        write_user(home.path(), r#"worktree_base = "/from/user""#);
        let repo = tempfile::tempdir().unwrap();
        let hecate = repo.path().join(".hecate");
        fs::create_dir_all(&hecate).unwrap();
        fs::write(
            hecate.join("config.toml"),
            r#"worktree_base = "/from/repo""#,
        )
        .unwrap();

        let opts = LoadOptions {
            config_home_override: Some(home.path().to_path_buf()),
            repo_root: Some(repo.path().to_path_buf()),
        };
        let c = load_merged(&opts, None).unwrap();
        assert_eq!(c.worktree_base, Some(PathBuf::from("/from/repo")));
    }

    #[test]
    fn env_overrides_files() {
        let home = tempfile::tempdir().unwrap();
        write_user(home.path(), r#"worktree_base = "/from/user""#);
        let repo = tempfile::tempdir().unwrap();
        let hecate = repo.path().join(".hecate");
        fs::create_dir_all(&hecate).unwrap();
        fs::write(
            hecate.join("config.toml"),
            r#"worktree_base = "/from/repo""#,
        )
        .unwrap();

        let opts = LoadOptions {
            config_home_override: Some(home.path().to_path_buf()),
            repo_root: Some(repo.path().to_path_buf()),
        };
        let c = load_merged(&opts, Some(PathBuf::from("/from/env"))).unwrap();

        assert_eq!(c.worktree_base, Some(PathBuf::from("/from/env")));
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
}
