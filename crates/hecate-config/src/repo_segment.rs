//! Hybrid repo directory segment under `hecate_root` using [`MetadataFile`].

use std::path::Path;

use hecate_core::{PathError, repo_default_segment};
use sha2::{Digest, Sha256};

use crate::MetadataFile;

/// Stable string for hashing: canonical or absolute `repo_root`, with `\\` normalized to `/`.
pub fn clone_identity_key(repo_root: &Path) -> String {
    let resolved = std::fs::canonicalize(repo_root)
        .or_else(|_| std::path::absolute(repo_root))
        .unwrap_or_else(|_| repo_root.to_path_buf());
    resolved.to_string_lossy().replace('\\', "/")
}

fn sha256_hex(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    format!("{:x}", h.finalize())
}

/// `true` if some **other** clone (any metadata key other than `exclude_clone_key`) has a
/// worktree whose path lies under `hecate_root.join(segment)` (same directory or nested).
pub fn segment_dir_used_by_other_clones(
    metadata: &MetadataFile,
    hecate_root: &Path,
    segment: &str,
    exclude_clone_key: &str,
) -> bool {
    let base = hecate_root.join(segment);
    for (clone_key, records) in &metadata.repos {
        if clone_key == exclude_clone_key {
            continue;
        }
        for r in records {
            if path_is_under_or_equal(&r.path, &base) {
                return true;
            }
        }
    }
    false
}

fn path_is_under_or_equal(path: &Path, base: &Path) -> bool {
    path == base || path.starts_with(base)
}

/// Choose the repo directory segment: default sanitized basename, or `{basename}-{hash...}`
/// when another clone already uses that folder under `hecate_root`.
pub fn choose_repo_segment(
    repo_root: &Path,
    hecate_root: &Path,
    metadata: &MetadataFile,
    current_clone_key: &str,
) -> Result<String, PathError> {
    let default_seg = repo_default_segment(repo_root)?;
    if !segment_dir_used_by_other_clones(metadata, hecate_root, &default_seg, current_clone_key) {
        return Ok(default_seg);
    }

    let identity = clone_identity_key(repo_root);
    let hash_hex = sha256_hex(&identity);
    let mut n: usize = 8;
    loop {
        let suffix_end = n.min(hash_hex.len());
        let candidate = format!("{}-{}", default_seg, &hash_hex[..suffix_end]);
        if !segment_dir_used_by_other_clones(metadata, hecate_root, &candidate, current_clone_key) {
            return Ok(candidate);
        }
        if suffix_end >= hash_hex.len() {
            break;
        }
        n = n.saturating_add(4);
    }

    Err(PathError::SegmentAllocationExhausted)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::*;
    use crate::WorktreeRecord;

    fn record(path: PathBuf) -> WorktreeRecord {
        WorktreeRecord {
            name: "w".into(),
            path,
            branch: "b".into(),
            base_branch: "main".into(),
            task: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: None,
            session: None,
        }
    }

    #[test]
    fn default_segment_when_no_collision() {
        let meta = MetadataFile::default();
        let seg = choose_repo_segment(
            Path::new("/home/me/hecate"),
            Path::new("/data/hroot"),
            &meta,
            "/home/me/hecate",
        )
        .unwrap();
        assert_eq!(seg, "hecate");
    }

    #[test]
    fn disambiguates_when_other_clone_uses_segment() {
        let hecate_root = Path::new("/data/hroot");
        let other_key = "/other/clone/hecate";
        let mut repos = HashMap::new();
        repos.insert(
            other_key.into(),
            vec![record(hecate_root.join("hecate").join("wt1"))],
        );
        let meta = MetadataFile {
            repos,
            ..Default::default()
        };

        let my_key = "/home/me/hecate";
        let seg = choose_repo_segment(Path::new(my_key), hecate_root, &meta, my_key).unwrap();
        assert!(seg.starts_with("hecate-"));
        assert!(seg.len() > "hecate".len());
    }

    #[test]
    fn own_clone_does_not_force_disambiguation() {
        let hecate_root = Path::new("/data/hroot");
        let my_key = "/home/me/hecate";
        let mut repos = HashMap::new();
        repos.insert(
            my_key.into(),
            vec![record(hecate_root.join("hecate").join("wt1"))],
        );
        let meta = MetadataFile {
            repos,
            ..Default::default()
        };

        let seg = choose_repo_segment(Path::new(my_key), hecate_root, &meta, my_key).unwrap();
        assert_eq!(seg, "hecate");
    }
}
