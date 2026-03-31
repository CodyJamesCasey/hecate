//! GitHub API and workflow integration (later stories).

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
