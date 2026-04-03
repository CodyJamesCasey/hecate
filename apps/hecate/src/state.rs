//! `hecate state` — repo + config + metadata summary.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use hecate_config::{
    LoadOptions, MetadataError, ResolveHecateRootError, clone_identity_key, load,
    load_without_env_hecate_root, metadata_path, read_metadata, resolve_hecate_root,
};
use hecate_git::{GitError, GitRepo};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error(transparent)]
    Git(#[from] GitError),

    #[error(transparent)]
    Config(#[from] hecate_config::ConfigError),

    #[error(transparent)]
    ResolveRoot(#[from] ResolveHecateRootError),

    #[error(transparent)]
    Metadata(#[from] MetadataError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Overrides for [`gather_opts`] (e.g. tests: isolated `config_home_override`).
#[derive(Debug, Clone)]
pub struct StateOptions {
    pub config_home_override: Option<PathBuf>,
    /// When `false`, **`HECATE_ROOT`** is not read (deterministic tests).
    pub use_process_hecate_env: bool,
}

impl Default for StateOptions {
    fn default() -> Self {
        Self {
            config_home_override: None,
            use_process_hecate_env: true,
        }
    }
}

/// Snapshot for display and JSON.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StateSnapshot {
    /// Git repository top-level (absolute).
    pub repo_root: PathBuf,
    /// Current branch short name, or `None` if detached or unknown.
    pub current_branch: Option<String>,
    /// `hecate_root` after config merge (before `resolve_hecate_root`), if any.
    pub hecate_root_configured: Option<PathBuf>,
    /// Absolute `hecate_root`, if configured.
    pub hecate_root_resolved: Option<PathBuf>,
    /// `{hecate_root}/metadata.json` when `hecate_root` is configured.
    pub metadata_path: Option<PathBuf>,
    /// Rows in metadata for this clone (`clone_identity_key`).
    pub worktree_count: usize,
}

pub fn gather(cwd: &Path) -> Result<StateSnapshot, StateError> {
    gather_opts(cwd, StateOptions::default())
}

pub fn gather_opts(cwd: &Path, state_opts: StateOptions) -> Result<StateSnapshot, StateError> {
    let git = GitRepo::discover(cwd)?;
    let repo_root = std::path::absolute(git.root())?;

    let current_branch = match git.current_branch() {
        Ok(b) => Some(b),
        Err(GitError::DetachedHead) => None,
        Err(e) => return Err(e.into()),
    };

    let opts = LoadOptions {
        repo_root: Some(git.root().to_path_buf()),
        config_home_override: state_opts.config_home_override,
    };
    let cfg = if state_opts.use_process_hecate_env {
        load(&opts)?
    } else {
        load_without_env_hecate_root(&opts)?
    };
    let hecate_root_configured = cfg.hecate_root.clone();

    let (hecate_root_resolved, metadata_path, worktree_count) =
        match resolve_hecate_root(cfg.hecate_root.as_deref(), git.root()) {
            Ok(root) => {
                let mp = metadata_path(&root);
                let meta = read_metadata(&root)?;
                let key = clone_identity_key(git.root());
                let n = meta.repos.get(&key).map(|v| v.len()).unwrap_or(0);
                (Some(root), Some(mp), n)
            }
            Err(ResolveHecateRootError::NotConfigured) => (None, None, 0),
            Err(e) => return Err(e.into()),
        };

    Ok(StateSnapshot {
        repo_root,
        current_branch,
        hecate_root_configured,
        hecate_root_resolved,
        metadata_path,
        worktree_count,
    })
}

pub fn run(cwd: &Path, json: bool) -> Result<(), StateError> {
    let snap = gather(cwd)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&snap)?);
    } else {
        print_human(&snap, io::stdout())?;
    }
    Ok(())
}

fn print_human(s: &StateSnapshot, mut out: impl Write) -> io::Result<()> {
    writeln!(out, "Repository:    {}", s.repo_root.display())?;
    match &s.current_branch {
        Some(b) => writeln!(out, "Branch:        {b}")?,
        None => writeln!(out, "Branch:        (detached HEAD)")?,
    }

    match &s.hecate_root_configured {
        Some(p) => writeln!(out, "hecate_root (configured): {}", p.display())?,
        None => writeln!(out, "hecate_root (configured): (not set)")?,
    }
    match &s.hecate_root_resolved {
        Some(p) => writeln!(out, "hecate_root (resolved):   {}", p.display())?,
        None => writeln!(out, "hecate_root (resolved):   (not set)")?,
    }
    match &s.metadata_path {
        Some(p) => writeln!(out, "Metadata file:            {}", p.display())?,
        None => writeln!(out, "Metadata file:            (not set)")?,
    }
    writeln!(out, "Tracked worktrees:        {}", s.worktree_count)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_includes_repo_line() {
        let snap = StateSnapshot {
            repo_root: PathBuf::from("/tmp/r"),
            current_branch: Some("main".into()),
            hecate_root_configured: None,
            hecate_root_resolved: None,
            metadata_path: None,
            worktree_count: 0,
        };
        let mut buf = Vec::new();
        print_human(&snap, &mut buf).unwrap();
        let t = String::from_utf8(buf).unwrap();
        assert!(t.contains("Repository:"));
        assert!(t.contains("main"));
    }
}
