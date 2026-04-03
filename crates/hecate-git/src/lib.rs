//! Subprocess `git` integration (v1).

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("failed to run git: {0}")]
    Io(#[from] std::io::Error),

    #[error("git command failed: {stderr}\ncommand: {cmd:?}")]
    Failed { cmd: Vec<String>, stderr: String },

    #[error("not a git repository (or git failed to read it)")]
    NotARepository,

    #[error("detached HEAD; check out a branch before running this command")]
    DetachedHead,

    #[error("utf-8 decode of git output failed")]
    Utf8,
}

fn git_output(workdir: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .current_dir(workdir)
        .args(args)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Failed {
            cmd: std::iter::once("git".to_string())
                .chain(args.iter().map(|s| (*s).to_string()))
                .collect(),
            stderr,
        });
    }

    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|_| GitError::Utf8)
}

/// Open repository at `root` (top-level directory).
#[derive(Debug, Clone)]
pub struct GitRepo {
    root: PathBuf,
}

impl GitRepo {
    /// Use `git rev-parse --show-toplevel` starting from `start` (e.g. current directory).
    pub fn discover(start: &Path) -> Result<Self, GitError> {
        let top = git_output(start, &["rev-parse", "--show-toplevel"]).map_err(|e| {
            if matches!(e, GitError::Failed { .. }) {
                GitError::NotARepository
            } else {
                e
            }
        })?;
        Ok(Self {
            root: PathBuf::from(top),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `git remote get-url <name>` (e.g. `origin`).
    pub fn remote_url(&self, remote: &str) -> Result<String, GitError> {
        git_output(self.root(), &["remote", "get-url", remote])
    }

    /// Short name of the checked-out branch, or [`GitError::DetachedHead`].
    pub fn current_branch(&self) -> Result<String, GitError> {
        let out = git_output(self.root(), &["symbolic-ref", "-q", "--short", "HEAD"]).map_err(
            |e| match e {
                GitError::Failed { .. } => GitError::DetachedHead,
                other => other,
            },
        )?;
        if out.is_empty() {
            return Err(GitError::DetachedHead);
        }
        Ok(out)
    }

    /// Whether `refs/heads/{branch}` exists.
    pub fn local_branch_exists(&self, branch: &str) -> Result<bool, GitError> {
        let refname = format!("refs/heads/{branch}");
        let out = Command::new("git")
            .current_dir(self.root())
            .args(["rev-parse", "--verify", &refname])
            .output()?;
        Ok(out.status.success())
    }

    /// `git worktree add -b <new_branch> <path> <start_point>`.
    pub fn worktree_add_branch(
        &self,
        path: &Path,
        new_branch: &str,
        start_point: &str,
    ) -> Result<(), GitError> {
        let output = Command::new("git")
            .current_dir(self.root())
            .args(["worktree", "add", "-b", new_branch])
            .arg(path)
            .arg(start_point)
            .output()?;

        if !output.status.success() {
            return Err(GitError::Failed {
                cmd: vec![
                    "git".into(),
                    "worktree".into(),
                    "add".into(),
                    "-b".into(),
                    new_branch.into(),
                    path.display().to_string(),
                    start_point.into(),
                ],
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(())
    }

    /// `git worktree remove [--force] <path>` (path is the linked checkout directory).
    pub fn worktree_remove(&self, path: &Path, force: bool) -> Result<(), GitError> {
        let mut cmd = Command::new("git");
        cmd.current_dir(self.root())
            .args(["worktree", "remove"])
            .arg(path);
        if force {
            cmd.arg("--force");
        }
        let output = cmd.output()?;

        if !output.status.success() {
            return Err(GitError::Failed {
                cmd: {
                    let mut v = vec!["git".into(), "worktree".into(), "remove".into()];
                    if force {
                        v.push("--force".into());
                    }
                    v.push(path.display().to_string());
                    v
                },
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn init_repo(path: &Path) {
        let status = Command::new("git")
            .current_dir(path)
            .args(["init", "-b", "main"])
            .status()
            .expect("git init");
        assert!(status.success(), "git init failed — is git installed?");
        Command::new("git")
            .current_dir(path)
            .args(["config", "user.email", "hecate-test@example.com"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(path)
            .args(["config", "user.name", "hecate-test"])
            .status()
            .unwrap();
        fs::write(path.join("README.md"), "x\n").unwrap();
        Command::new("git")
            .current_dir(path)
            .args(["add", "README.md"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(path)
            .args(["commit", "-m", "init"])
            .status()
            .unwrap();
    }

    #[test]
    fn discover_and_branch_and_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        init_repo(tmp.path());
        let repo = GitRepo::discover(tmp.path()).unwrap();
        assert_eq!(repo.root(), tmp.path());
        assert_eq!(repo.current_branch().unwrap(), "main");
        assert!(!repo.local_branch_exists("task/1").unwrap());

        let wt = tmp.path().join("wt-side");
        repo.worktree_add_branch(&wt, "task/1", "HEAD").unwrap();
        assert!(wt.join("README.md").exists());
        assert!(repo.local_branch_exists("task/1").unwrap());

        repo.worktree_remove(&wt, false).unwrap();
        assert!(!wt.exists());
    }

    #[test]
    fn discover_non_repo_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let err = GitRepo::discover(tmp.path()).unwrap_err();
        assert!(matches!(err, GitError::NotARepository));
    }
}
