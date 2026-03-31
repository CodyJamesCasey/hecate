use std::path::PathBuf;

use serde::Deserialize;

/// Fields accepted in `config.toml` (user or repo). Omitted keys use defaults.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FileConfig {
    /// Base directory for canonical worktree paths (`<base>/<repo>/<name>`).
    pub worktree_base: Option<PathBuf>,
}

impl FileConfig {
    pub fn merge(&mut self, other: FileConfig) {
        if other.worktree_base.is_some() {
            self.worktree_base = other.worktree_base;
        }
    }
}
