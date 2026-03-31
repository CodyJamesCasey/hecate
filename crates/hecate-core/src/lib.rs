//! Domain models and application services.

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
