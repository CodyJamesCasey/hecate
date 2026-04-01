use std::path::PathBuf;

use serde::Deserialize;

/// Fields accepted in `config.toml` (user or repo). Omitted keys use defaults.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FileConfig {
    /// Root directory for worktrees and `{hecate_root}/metadata.json`
    /// (`<hecate_root>/<repo>/<worktree-name>`).
    pub hecate_root: Option<PathBuf>,
}

impl FileConfig {
    pub fn merge(&mut self, other: FileConfig) {
        if other.hecate_root.is_some() {
            self.hecate_root = other.hecate_root;
        }
    }
}
