//! Core types and logic for plain-text requirements management.
//!
//! This crate provides the foundational types and operations for managing
//! requirements as markdown documents in a directory structure.

/// Domain types and business logic.
pub mod domain;
pub use domain::{
    Config, Hrid, LinkRequirementError, Requirement, RequirementView, SuspectLink, Tree,
};

/// Filesystem storage and directory management for requirements.
pub mod storage;
pub use storage::{hrid_from_path, AcceptResult, AddRequirementWithParentsError, Directory};
