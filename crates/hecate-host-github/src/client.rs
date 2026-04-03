//! GitHub via **`gh api`** (GitHub CLI auth — no `GITHUB_TOKEN` in hecate).

use hecate_host::Issue;
use serde::Deserialize;
use serde_json::Error as SerdeJsonError;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("gh failed: {stderr}")]
    Gh { stderr: String },

    #[error(
        "failed to run gh (install GitHub CLI from https://cli.github.com and run `gh auth login`): {0}"
    )]
    Io(#[from] std::io::Error),

    #[error("failed to parse gh JSON output: {0}")]
    Json(#[from] SerdeJsonError),

    #[error("use --repo OWNER/REPO or set `origin` to a github.com remote (got: {0})")]
    UnresolvedRepo(String),
}

#[derive(Debug, Deserialize)]
struct GhIssue {
    number: u64,
    title: String,
    state: String,
    html_url: String,
    body: Option<String>,
}

/// Fetch issue (or PR) metadata via **`gh api repos/{owner}/{repo}/issues/{number}`**.
///
/// Uses the same auth as **`gh`** (`gh auth login`); no direct HTTP or PAT in hecate.
pub fn fetch_issue(owner: &str, repo: &str, number: u64) -> Result<Issue, GitHubError> {
    let endpoint = format!("repos/{owner}/{repo}/issues/{number}");
    let output = Command::new("gh")
        .args(["api", &endpoint])
        .args(["--header", "Accept: application/vnd.github+json"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitHubError::Gh { stderr });
    }

    let gh: GhIssue = serde_json::from_slice(&output.stdout)?;
    Ok(Issue {
        number: gh.number,
        title: gh.title,
        state: gh.state,
        html_url: gh.html_url,
        body: gh.body,
    })
}
