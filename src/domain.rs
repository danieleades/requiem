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
pub use hrid::{Error as HridError, FormattedHrid, Hrid};

/// In-memory tree structure for requirements.
pub mod tree;
pub use tree::{AcceptLinkError, SuspectLink, Tree, TreeInsertError};

/// Borrowed view of requirements for efficient access.
pub mod requirement_view;
pub use requirement_view::RequirementView;

/// Decomposed requirement data.
pub(crate) mod requirement_data;
