//! Task token parsing for branch names and worktree directory segments.

use crate::paths::{PathError, sanitize_segment};

/// Git branch name (`task/<slug>`) and worktree directory name (`<slug>`) for a CLI task.
///
/// The slug is [`sanitize_segment`] of the trimmed task string (e.g. `42` → `task/42` / `42`).
pub fn task_layout(task: &str) -> Result<(String, String), PathError> {
    let slug = sanitize_segment(task.trim())?;
    Ok((format!("task/{slug}"), slug))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_task() {
        let (b, w) = task_layout("42").unwrap();
        assert_eq!(b, "task/42");
        assert_eq!(w, "42");
    }

    #[test]
    fn slug_task() {
        let (b, w) = task_layout("Fix the thing").unwrap();
        assert_eq!(b, "task/fix-the-thing");
        assert_eq!(w, "fix-the-thing");
    }
}
