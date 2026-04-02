//! `hecate start <task>` — create branch + worktree and register in metadata.

use std::path::{Path, PathBuf};

use chrono::Utc;
use hecate_config::{
    LoadOptions, ResolveHecateRootError, WorktreeRecord, choose_repo_segment, clone_identity_key,
    load, read_metadata, resolve_hecate_root, write_metadata,
};
use hecate_core::{PathError, task_layout, worktree_directory};
use hecate_git::GitRepo;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StartError {
    #[error(transparent)]
    Git(#[from] hecate_git::GitError),

    #[error(transparent)]
    Config(#[from] hecate_config::ConfigError),

    #[error(transparent)]
    ResolveRoot(#[from] ResolveHecateRootError),

    #[error(transparent)]
    Path(#[from] PathError),

    #[error(transparent)]
    Metadata(#[from] hecate_config::MetadataError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("worktree path already exists: {0}", path.display())]
    PathExists { path: PathBuf },

    #[error(
        "this clone already has a registered worktree for this task (name `{name}`, branch `{branch}`)"
    )]
    AlreadyRegistered { name: String, branch: String },

    #[error("branch `{0}` already exists locally; remove it or use a different task label")]
    BranchExists(String),
}

pub fn run(task: &str, cwd: &Path) -> Result<(), StartError> {
    let git = GitRepo::discover(cwd)?;
    let base_branch = git.current_branch()?;

    let opts = LoadOptions {
        repo_root: Some(git.root().to_path_buf()),
        config_home_override: None,
    };
    let cfg = load(&opts)?;
    let hecate_root = resolve_hecate_root(cfg.hecate_root.as_deref(), git.root())?;

    let clone_key = clone_identity_key(git.root());
    let mut meta = read_metadata(&hecate_root)?;
    let repo_segment = choose_repo_segment(git.root(), &hecate_root, &meta, &clone_key)?;

    let (branch, worktree_name) = task_layout(task)?;

    if let Some(list) = meta.repos.get(&clone_key) {
        for r in list {
            if r.name == worktree_name || r.branch == branch {
                return Err(StartError::AlreadyRegistered {
                    name: r.name.clone(),
                    branch: r.branch.clone(),
                });
            }
        }
    }

    if git.local_branch_exists(&branch)? {
        return Err(StartError::BranchExists(branch));
    }

    let wt_path = worktree_directory(&hecate_root, &repo_segment, &worktree_name)?;
    if wt_path.exists() {
        return Err(StartError::PathExists { path: wt_path });
    }

    std::fs::create_dir_all(hecate_root.join(&repo_segment))?;

    git.worktree_add_branch(&wt_path, &branch, "HEAD")?;

    let path_abs = std::path::absolute(&wt_path)?;
    let record = WorktreeRecord {
        name: worktree_name.clone(),
        path: path_abs,
        branch: branch.clone(),
        base_branch,
        task: Some(task.trim().to_string()),
        created_at: Utc::now().to_rfc3339(),
        updated_at: None,
        session: None,
    };

    meta.repos.entry(clone_key).or_default().push(record);

    write_metadata(&hecate_root, &meta)?;

    println!("Created worktree at {}", wt_path.display());
    println!("Branch: {branch}");

    Ok(())
}
