//! `hecate list` — show worktrees for the current clone from metadata.

use std::io::{self, Write};
use std::path::Path;

use hecate_config::{
    LoadOptions, ResolveHecateRootError, WorktreeRecord, clone_identity_key, load, read_metadata,
    resolve_hecate_root,
};
use hecate_git::GitRepo;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ListError {
    #[error(transparent)]
    Git(#[from] hecate_git::GitError),

    #[error(transparent)]
    Config(#[from] hecate_config::ConfigError),

    #[error(transparent)]
    ResolveRoot(#[from] ResolveHecateRootError),

    #[error(transparent)]
    Metadata(#[from] hecate_config::MetadataError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Serialize)]
struct ListJson<'a> {
    worktrees: &'a [WorktreeRecord],
}

/// Load registered worktrees for the main clone containing `cwd`.
pub fn worktrees_for_cwd(cwd: &Path) -> Result<Vec<WorktreeRecord>, ListError> {
    let git = GitRepo::discover(cwd)?;
    let opts = LoadOptions {
        repo_root: Some(git.root().to_path_buf()),
        config_home_override: None,
    };
    let cfg = load(&opts)?;
    let hecate_root = resolve_hecate_root(cfg.hecate_root.as_deref(), git.root())?;
    let clone_key = clone_identity_key(git.root());
    let meta = read_metadata(&hecate_root)?;
    Ok(meta.repos.get(&clone_key).cloned().unwrap_or_default())
}

pub fn run(cwd: &Path, json: bool) -> Result<(), ListError> {
    let records = worktrees_for_cwd(cwd)?;
    if json {
        let payload = ListJson {
            worktrees: records.as_slice(),
        };
        let s = serde_json::to_string_pretty(&payload)?;
        println!("{s}");
    } else {
        print_human(&records, io::stdout())?;
    }
    Ok(())
}

fn print_human(records: &[WorktreeRecord], mut out: impl Write) -> io::Result<()> {
    if records.is_empty() {
        writeln!(out, "No worktrees registered for this clone in metadata.")?;
        return Ok(());
    }

    for w in records {
        writeln!(out, "{}", w.name)?;
        writeln!(out, "  branch:      {}", w.branch)?;
        writeln!(out, "  base branch: {}", w.base_branch)?;
        match &w.task {
            Some(t) => writeln!(out, "  task:        {t}")?,
            None => writeln!(out, "  task:        —")?,
        }
        writeln!(out, "  path:        {}", w.path.display())?;
        writeln!(out, "  created:     {}", w.created_at)?;
        writeln!(out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_empty_line() {
        let mut buf = Vec::new();
        print_human(&[], &mut buf).unwrap();
        assert!(
            String::from_utf8(buf)
                .unwrap()
                .contains("No worktrees registered")
        );
    }
}
