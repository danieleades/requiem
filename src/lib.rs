//! Plain-text Requirements Management
//!
//! Requirements are markdown documents stored in a directory.

mod domain;
pub use domain::{EmptyStringError, Hrid, Requirement};

pub mod storage;
pub use storage::{Directory, SuspectLink};
