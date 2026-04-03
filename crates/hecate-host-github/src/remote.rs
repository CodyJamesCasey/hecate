//! Parse `owner` / `repo` from common `github.com` remote URL shapes.

/// `owner/repo` from TOML/CLI override (not a URL).
pub fn owner_repo_from_slash(spec: &str) -> Option<(String, String)> {
    let s = spec.trim();
    let (a, b) = s.split_once('/')?;
    if a.is_empty() || b.is_empty() || a.contains('/') || b.contains('/') {
        return None;
    }
    Some((a.to_string(), b.to_string()))
}

/// Best-effort parse for `https://github.com/o/r`, `git@github.com:o/r.git`, etc.
pub fn owner_repo_from_remote_url(url: &str) -> Option<(String, String)> {
    let u = url.trim();

    if let Some(rest) = u.strip_prefix("git@github.com:") {
        return split_owner_repo(rest);
    }
    if let Some(rest) = u.strip_prefix("ssh://git@github.com/") {
        return split_owner_repo(rest);
    }

    let lower = u.to_ascii_lowercase();
    let idx = lower.find("github.com/")?;
    let mut tail = &u[idx + "github.com/".len()..];
    if let Some(i) = tail.find('?') {
        tail = &tail[..i];
    }
    if let Some(i) = tail.find('#') {
        tail = &tail[..i];
    }
    split_owner_repo(tail.trim_end_matches('/'))
}

fn split_owner_repo(path: &str) -> Option<(String, String)> {
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_short_form() {
        assert_eq!(
            owner_repo_from_remote_url("git@github.com:foo/bar.git"),
            Some(("foo".into(), "bar".into()))
        );
    }

    #[test]
    fn https() {
        assert_eq!(
            owner_repo_from_remote_url("https://github.com/foo/bar"),
            Some(("foo".into(), "bar".into()))
        );
    }

    #[test]
    fn slash_spec() {
        assert_eq!(
            owner_repo_from_slash("myorg/hecate"),
            Some(("myorg".into(), "hecate".into()))
        );
    }
}
