//! GitHub integration for hecate via the **`gh`** CLI.

mod client;
mod remote;

pub use client::{GitHubError, fetch_issue};
pub use remote::{owner_repo_from_remote_url, owner_repo_from_slash};

/// Resolve `owner` / `repo` from `--repo OWNER/REPO` or a `git` remote URL.
pub fn resolve_owner_repo(
    origin_url: &str,
    repo_override: Option<&str>,
) -> Result<(String, String), GitHubError> {
    if let Some(spec) = repo_override {
        return owner_repo_from_slash(spec).ok_or_else(|| {
            GitHubError::UnresolvedRepo(format!("invalid --repo {spec:?} (expected OWNER/REPO)"))
        });
    }
    owner_repo_from_remote_url(origin_url).ok_or_else(|| {
        GitHubError::UnresolvedRepo(format!(
            "could not parse github.com owner/repo from remote URL {origin_url:?}"
        ))
    })
}

/// Keeps `hecate-host-github` wired to `hecate-host` in the dependency graph.
pub fn host_crate_linked() -> &'static str {
    hecate_host::LINK_CHECK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_dependency_resolves() {
        assert_eq!(host_crate_linked(), "hecate-host");
    }
}
