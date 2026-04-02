//! `hecate rm` — remove a linked worktree and drop its metadata entry.

use std::fs;
use std::path::{Path, PathBuf};

use hecate_config::{
    LoadOptions, ResolveHecateRootError, WorktreeRecord, clone_identity_key, load, read_metadata,
    resolve_hecate_root, write_metadata,
};
use hecate_git::GitRepo;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RmError {
    #[error(transparent)]
    Git(#[from] hecate_git::GitError),

    #[error(transparent)]
    Config(#[from] hecate_config::ConfigError),

    #[error(transparent)]
    ResolveRoot(#[from] ResolveHecateRootError),

    #[error(transparent)]
    Metadata(#[from] hecate_config::MetadataError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("no worktree matched `{0}` for this clone in metadata")]
    NotFound(String),

    #[error("this clone has no worktrees registered in metadata")]
    NoWorktreesRegistered,
}

fn absolute_norm(path: &Path, cwd: &Path) -> PathBuf {
    let p = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    std::path::absolute(&p).unwrap_or(p)
}

fn path_matches_record(record_path: &Path, user: &Path, cwd: &Path) -> bool {
    let u = absolute_norm(user, cwd);
    let r = std::path::absolute(record_path).unwrap_or_else(|_| record_path.to_path_buf());
    if u == r {
        return true;
    }
    match (fs::canonicalize(&u), fs::canonicalize(record_path)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn find_index(
    list: &[WorktreeRecord],
    name: Option<&str>,
    path: Option<&Path>,
    cwd: &Path,
) -> Option<usize> {
    if let Some(n) = name {
        return list.iter().position(|r| r.name == n);
    }
    let p = path?;
    list.iter()
        .position(|r| path_matches_record(&r.path, p, cwd))
}

pub fn run(
    cwd: &Path,
    name: Option<String>,
    path: Option<PathBuf>,
    force: bool,
) -> Result<(), RmError> {
    let git = GitRepo::discover(cwd)?;
    let opts = LoadOptions {
        repo_root: Some(git.root().to_path_buf()),
        config_home_override: None,
    };
    let cfg = load(&opts)?;
    let hecate_root = resolve_hecate_root(cfg.hecate_root.as_deref(), git.root())?;
    let clone_key = clone_identity_key(git.root());
    let mut meta = read_metadata(&hecate_root)?;

    let list = meta
        .repos
        .get_mut(&clone_key)
        .ok_or(RmError::NoWorktreesRegistered)?;

    let idx = find_index(list.as_slice(), name.as_deref(), path.as_deref(), cwd)
        .ok_or_else(|| RmError::NotFound(label(name.as_deref(), path.as_deref())))?;

    let record = list[idx].clone();
    git.worktree_remove(&record.path, force)?;

    list.remove(idx);
    if list.is_empty() {
        meta.repos.remove(&clone_key);
    }

    write_metadata(&hecate_root, &meta)?;

    println!(
        "Removed worktree `{}` ({})",
        record.name,
        record.path.display()
    );
    Ok(())
}

fn label(name: Option<&str>, path: Option<&Path>) -> String {
    match (name, path) {
        (Some(n), None) => n.to_string(),
        (None, Some(p)) => p.display().to_string(),
        _ => "(no target)".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_matches_absolute_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("wt");
        fs::create_dir_all(&sub).unwrap();
        let canon = fs::canonicalize(&sub).unwrap();
        assert!(path_matches_record(
            &canon,
            tmp.path().join("wt").as_path(),
            tmp.path()
        ));
    }
}
