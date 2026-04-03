//! Task and pull-request abstractions shared across code hosts.

pub mod issue;

pub use issue::Issue;

/// Marker that this crate is part of the workspace graph.
pub const LINK_CHECK: &str = "hecate-host";
