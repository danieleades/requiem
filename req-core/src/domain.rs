//! Domain models for requirements management.
//!
//! This module contains the core domain types including requirements,
//! human-readable identifiers (HRIDs), and configuration.

pub mod requirement;
pub use requirement::Requirement;

pub mod config;
pub use config::{Config, KindMetadata};

pub mod hrid;
pub use hrid::{Error as HridError, FormattedHrid, Hrid};

pub mod tree;
pub use tree::{AcceptLinkError, LinkRequirementError, SuspectLink, Tree, TreeInsertError};

pub mod requirement_view;
pub use requirement_view::RequirementView;

pub(crate) mod requirement_data;
