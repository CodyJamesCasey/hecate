//! Resolve configured `hecate_root` to an absolute path.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Failed to resolve `hecate_root` from config and repo root.
#[derive(Debug, Error)]
pub enum ResolveHecateRootError {
    #[error("hecate_root is not configured")]
    NotConfigured,

    #[error("failed to resolve absolute hecate_root path: {0}")]
    Absolute(#[from] std::io::Error),
}

/// Turn configured `hecate_root` into an absolute path.
///
/// Relative values are resolved against `repo_root` (per PRD: repo `config.toml`
/// paths are relative to the repository root).
pub fn resolve_hecate_root(
    configured: Option<&Path>,
    repo_root: &Path,
) -> Result<PathBuf, ResolveHecateRootError> {
    let Some(raw) = configured else {
        return Err(ResolveHecateRootError::NotConfigured);
    };
    if raw.as_os_str().is_empty() {
        return Err(ResolveHecateRootError::NotConfigured);
    }

    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        repo_root.join(raw)
    };

    Ok(std::path::absolute(joined)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_errors() {
        assert!(matches!(
            resolve_hecate_root(None, Path::new("/repo")).unwrap_err(),
            ResolveHecateRootError::NotConfigured
        ));
    }

    #[test]
    fn absolute_passes_through() {
        let p = resolve_hecate_root(Some(Path::new("/var/hecate")), Path::new("/repo")).unwrap();
        assert!(p.is_absolute());
        assert!(p.ends_with("hecate"));
    }

    #[test]
    fn relative_joins_repo() {
        let repo = std::path::absolute(Path::new(".")).unwrap();
        let p = resolve_hecate_root(Some(Path::new(".hecate-parking")), &repo).unwrap();
        assert!(p.is_absolute());
        assert!(p.ends_with(".hecate-parking"));
    }
}
