//! Domain models for requirements management.
//!
//! This module contains the core domain types including requirements,
//! human-readable identifiers (HRIDs), and configuration.

/// Requirement domain model and persistence.
pub mod requirement;
pub use requirement::Requirement;

mod config;
pub use config::Config;

/// Human-readable identifier (HRID) types and parsing.
pub mod hrid;
pub(crate) use hrid::Error as HridError;
pub use hrid::{Hrid, KindString};

/// In-memory tree structure for requirements.
pub(crate) mod tree;
pub use tree::{LinkError, SuspectLink, Tree};

/// Borrowed view of requirements for efficient access.
pub mod requirement_view;
pub use requirement_view::RequirementView;

/// Decomposed requirement data.
pub(crate) mod requirement_data;
