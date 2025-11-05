//! Plain-text Requirements Management
//!
//! Requirements are markdown documents stored in a directory.

pub mod domain;
pub use domain::{Config, Hrid, LinkError, Requirement, RequirementView, SuspectLink, Tree};

/// Filesystem storage and directory management for requirements.
pub mod storage;
pub use storage::{AcceptResult, Directory};
