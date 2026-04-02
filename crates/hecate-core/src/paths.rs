//! Worktree layout under `hecate_root`: `<hecate_root>/<repo-segment>/<worktree-name>`.
//!
//! **Sanitization** (repo basename and worktree name share these rules):
//! - Trim leading/trailing ASCII whitespace.
//! - Reject empty results, `.`, `..`, any `..` substring, NUL, and `/` or `\\`.
//! - Map each character: ASCII alphanumeric, `_`, `.`, and `-` kept; ASCII spaces become
//!   `-`; other ASCII becomes `-`; non-ASCII becomes `-`.
//! - Collapse runs of `-`, then trim `-` from both ends.
//! - If the result matches a Windows reserved device name (case-insensitive), prefix `_`.
//!
//! Callers that build metadata keys should still use [`std::fs::canonicalize`] where
//! appropriate; this module only performs string/path joining for on-disk segments.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Upper bound on segment length after sanitization (avoids absurd directory names).
const MAX_SEGMENT_LEN: usize = 200;

/// Errors from segment validation or layout joins.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathError {
    #[error("repository path has no usable final directory name (cannot derive segment)")]
    NoRepoBasename,

    #[error("segment is empty or whitespace-only")]
    EmptySegment,

    #[error("segment contains path separators, NUL, or parent-dir sequence (..)")]
    InvalidSegment,

    #[error("segment exceeds maximum length ({MAX_SEGMENT_LEN}) after sanitization")]
    TooLong,

    /// No free repo segment after expanding the hash suffix (extremely unlikely).
    #[error("could not allocate a unique repo segment under hecate_root")]
    SegmentAllocationExhausted,
}

/// Sanitize a single path segment (repo folder name or worktree name).
pub fn sanitize_segment(raw: &str) -> Result<String, PathError> {
    let trimmed = raw.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r'));
    if trimmed.is_empty() {
        return Err(PathError::EmptySegment);
    }
    if trimmed.contains('\0')
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return Err(PathError::InvalidSegment);
    }

    let mut out = String::with_capacity(trimmed.len());
    for c in trimmed.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.' | '-' => out.push(c),
            ' ' => out.push('-'),
            _ if c.is_ascii() => out.push('-'),
            _ => out.push('-'),
        }
    }

    while out.contains("--") {
        out = out.replace("--", "-");
    }
    let out = out.trim_matches('-').to_string();

    if out.is_empty() || out == "." || out == ".." {
        return Err(PathError::EmptySegment);
    }

    let mut out = if windows_reserved(&out) {
        let mut s = String::with_capacity(out.len() + 1);
        s.push('_');
        s.push_str(&out);
        s
    } else {
        out
    };

    if out.len() > MAX_SEGMENT_LEN {
        return Err(PathError::TooLong);
    }

    // Lowercase for stable segments across case-insensitive filesystems.
    out.make_ascii_lowercase();

    Ok(out)
}

/// Default repo directory segment: sanitized last normal path component of `repo_root`.
pub fn repo_default_segment(repo_root: &Path) -> Result<String, PathError> {
    let base = repo_root
        .components()
        .rev()
        .find_map(|c| match c {
            std::path::Component::Normal(s) => Some(s),
            _ => None,
        })
        .ok_or(PathError::NoRepoBasename)?;
    sanitize_segment(&base.to_string_lossy())
}

/// Resolved worktree directory: `hecate_root` / `repo_segment` / `worktree_name`.
///
/// Both segment arguments are validated through [`sanitize_segment`].
pub fn worktree_directory(
    hecate_root: &Path,
    repo_segment: &str,
    worktree_name: &str,
) -> Result<PathBuf, PathError> {
    let r = sanitize_segment(repo_segment)?;
    let w = sanitize_segment(worktree_name)?;
    Ok(hecate_root.join(r).join(w))
}

fn windows_reserved(name: &str) -> bool {
    let u = name.to_ascii_uppercase();
    matches!(
        u.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM0"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT0"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_trims_and_lowercases() {
        assert_eq!(sanitize_segment("  Hecate  ").unwrap(), "hecate");
    }

    #[test]
    fn sanitize_rejects_separators_and_dotdot() {
        assert_eq!(sanitize_segment("a/b"), Err(PathError::InvalidSegment));
        assert_eq!(sanitize_segment("a\\b"), Err(PathError::InvalidSegment));
        assert_eq!(sanitize_segment(".."), Err(PathError::InvalidSegment));
        assert_eq!(sanitize_segment("foo..bar"), Err(PathError::InvalidSegment));
    }

    #[test]
    fn sanitize_maps_special_chars() {
        assert_eq!(sanitize_segment("my branch!").unwrap(), "my-branch");
    }

    #[test]
    fn reserved_name_gets_underscore_prefix() {
        assert_eq!(sanitize_segment("CON").unwrap(), "_con");
        assert_eq!(sanitize_segment("com1").unwrap(), "_com1");
    }

    #[test]
    fn repo_default_segment_last_component() {
        assert_eq!(
            repo_default_segment(Path::new("/home/me/projects/hecate")).unwrap(),
            "hecate"
        );
        assert_eq!(
            repo_default_segment(Path::new("C:/src/foo/bar")).unwrap(),
            "bar"
        );
    }

    #[test]
    fn repo_default_segment_root_fails() {
        assert_eq!(
            repo_default_segment(Path::new("/")).unwrap_err(),
            PathError::NoRepoBasename
        );
    }

    #[test]
    fn worktree_directory_joins() {
        let root = Path::new("/data/hecate-root");
        let p = worktree_directory(root, "myrepo", "wt-1").unwrap();
        assert_eq!(p, PathBuf::from("/data/hecate-root/myrepo/wt-1"));
    }
}
