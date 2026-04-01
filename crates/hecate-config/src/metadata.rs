//! `{hecate_root}/metadata.json` — versioned registry of worktrees per main clone path.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current on-disk `version` field for [`MetadataFile`].
pub const METADATA_VERSION: u32 = 1;

const FILENAME: &str = "metadata.json";

/// Path to the metadata file under `hecate_root`.
pub fn metadata_path(hecate_root: &Path) -> PathBuf {
    hecate_root.join(FILENAME)
}

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("failed to read {}: {source}", path.display())]
    IoRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write {}: {source}", path.display())]
    IoWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid metadata JSON in {}: {source}", path.display())]
    JsonParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to serialize metadata: {0}")]
    JsonSerialize(#[from] serde_json::Error),

    #[error("unsupported metadata version {found} (only version {METADATA_VERSION} is supported)")]
    UnsupportedVersion { found: u32 },
}

/// Root document stored at `{hecate_root}/metadata.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetadataFile {
    /// Schema version; must be [`METADATA_VERSION`] for read/write in v1.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Main clone path (absolute, normalized by callers) → worktrees for that repo.
    #[serde(default)]
    pub repos: HashMap<String, Vec<WorktreeRecord>>,
}

fn default_version() -> u32 {
    METADATA_VERSION
}

impl Default for MetadataFile {
    fn default() -> Self {
        Self {
            version: METADATA_VERSION,
            repos: HashMap::new(),
        }
    }
}

/// One registered Git worktree under a given main clone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorktreeRecord {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub base_branch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// ISO 8601 / RFC 3339 timestamp string.
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<serde_json::Value>,
}

/// Read `metadata.json` under `hecate_root`. Missing file returns an empty [`MetadataFile`].
pub fn read_metadata(hecate_root: &Path) -> Result<MetadataFile, MetadataError> {
    let path = metadata_path(hecate_root);
    let data = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MetadataFile::default());
        }
        Err(source) => {
            return Err(MetadataError::IoRead {
                path: path.clone(),
                source,
            });
        }
    };

    let file: MetadataFile =
        serde_json::from_str(&data).map_err(|source| MetadataError::JsonParse {
            path: path.clone(),
            source,
        })?;

    if file.version != METADATA_VERSION {
        return Err(MetadataError::UnsupportedVersion {
            found: file.version,
        });
    }

    Ok(file)
}

/// Write `metadata.json` atomically (best-effort on all platforms).
pub fn write_metadata(hecate_root: &Path, file: &MetadataFile) -> Result<(), MetadataError> {
    if file.version != METADATA_VERSION {
        return Err(MetadataError::UnsupportedVersion {
            found: file.version,
        });
    }

    fs::create_dir_all(hecate_root).map_err(|source| MetadataError::IoWrite {
        path: hecate_root.to_path_buf(),
        source,
    })?;

    let dest = metadata_path(hecate_root);
    let tmp_name = format!(".{}.tmp.{}", FILENAME, process::id());
    let tmp = hecate_root.join(&tmp_name);

    let bytes = serde_json::to_vec_pretty(file)?;

    {
        let mut f = File::create(&tmp).map_err(|source| MetadataError::IoWrite {
            path: tmp.clone(),
            source,
        })?;
        f.write_all(&bytes)
            .map_err(|source| MetadataError::IoWrite {
                path: tmp.clone(),
                source,
            })?;
        f.sync_all().map_err(|source| MetadataError::IoWrite {
            path: tmp.clone(),
            source,
        })?;
    }

    if let Err(e) = replace_atomic(&tmp, &dest) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }

    Ok(())
}

fn replace_atomic(tmp: &Path, dest: &Path) -> Result<(), MetadataError> {
    #[cfg(windows)]
    {
        if dest.exists() {
            fs::remove_file(dest).map_err(|source| MetadataError::IoWrite {
                path: dest.to_path_buf(),
                source,
            })?;
        }
    }
    fs::rename(tmp, dest).map_err(|source| MetadataError::IoWrite {
        path: dest.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record() -> WorktreeRecord {
        WorktreeRecord {
            name: "wt-1".into(),
            path: PathBuf::from("/tmp/wt-1"),
            branch: "feat/x".into(),
            base_branch: "main".into(),
            task: Some("42".into()),
            created_at: "2026-04-01T12:00:00Z".into(),
            updated_at: None,
            session: None,
        }
    }

    #[test]
    fn read_missing_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let m = read_metadata(dir.path()).unwrap();
        assert_eq!(m, MetadataFile::default());
    }

    #[test]
    fn round_trip_empty() {
        let dir = tempfile::tempdir().unwrap();
        let empty = MetadataFile::default();
        write_metadata(dir.path(), &empty).unwrap();
        let got = read_metadata(dir.path()).unwrap();
        assert_eq!(got.version, METADATA_VERSION);
        assert!(got.repos.is_empty());
    }

    #[test]
    fn round_trip_with_repo() {
        let dir = tempfile::tempdir().unwrap();
        let mut file = MetadataFile::default();
        file.repos
            .insert("/home/me/projects/foo".into(), vec![sample_record()]);
        write_metadata(dir.path(), &file).unwrap();
        let got = read_metadata(dir.path()).unwrap();
        assert_eq!(got, file);
    }

    #[test]
    fn corrupt_json_errors() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(metadata_path(dir.path()), b"{ not json").unwrap();
        let err = read_metadata(dir.path()).unwrap_err();
        assert!(matches!(err, MetadataError::JsonParse { .. }));
    }

    #[test]
    fn unsupported_version_on_read() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(metadata_path(dir.path()), r#"{"version":99,"repos":{}}"#).unwrap();
        let err = read_metadata(dir.path()).unwrap_err();
        assert!(matches!(
            err,
            MetadataError::UnsupportedVersion { found: 99 }
        ));
    }

    #[test]
    fn write_rejects_bad_version() {
        let dir = tempfile::tempdir().unwrap();
        let bad = MetadataFile {
            version: 2,
            ..Default::default()
        };
        let err = write_metadata(dir.path(), &bad).unwrap_err();
        assert!(matches!(
            err,
            MetadataError::UnsupportedVersion { found: 2 }
        ));
    }
}
