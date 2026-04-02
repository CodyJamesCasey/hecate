//! Domain models and application services.

pub mod paths;
pub mod task;

pub use paths::{PathError, repo_default_segment, sanitize_segment, worktree_directory};
pub use task::task_layout;

/// Sanity check that the workspace links; replaced as features land.
pub const fn workspace_ok() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_ok_is_true() {
        assert!(workspace_ok());
    }
}
