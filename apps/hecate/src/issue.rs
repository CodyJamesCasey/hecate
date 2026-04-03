//! `hecate issue show` — GitHub issue lookup for numeric task refs.

use std::path::Path;

use hecate_git::GitRepo;
use hecate_host_github::{fetch_issue, resolve_owner_repo};
use serde_json::Error as JsonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IssueShowError {
    #[error(transparent)]
    Git(#[from] hecate_git::GitError),

    #[error(transparent)]
    GitHub(#[from] hecate_host_github::GitHubError),

    #[error(transparent)]
    Json(#[from] JsonError),
}

pub fn run_show(
    cwd: &Path,
    number: u64,
    json: bool,
    repo_override: Option<String>,
) -> Result<(), IssueShowError> {
    let git = GitRepo::discover(cwd)?;
    let origin_url = git.remote_url("origin")?;
    let (owner, repo) = resolve_owner_repo(&origin_url, repo_override.as_deref())?;
    let issue = fetch_issue(&owner, &repo, number)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&issue)?);
    } else {
        println!("#{} [{}] {}", issue.number, issue.state, issue.title);
        println!("{}", issue.html_url);
        if let Some(body) = &issue.body {
            if !body.is_empty() {
                println!();
                println!("{body}");
            }
        }
    }
    Ok(())
}
