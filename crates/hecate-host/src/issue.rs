//! Code-host–agnostic issue representation (filled by adapters such as GitHub).

use serde::{Deserialize, Serialize};

/// A single issue (or pull request on hosts that expose PRs through the issues API).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub html_url: String,
    pub body: Option<String>,
}
