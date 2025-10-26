//! Plain-text Requirements Management
//!
//! Requirements are markdown documents stored in a directory.

mod domain;
pub use domain::{EmptyStringError, Hrid, Requirement};

/// Filesystem storage and directory management for requirements.
pub mod storage;
pub use storage::{Directory, SuspectLink};
