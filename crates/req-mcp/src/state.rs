//! Shared server state for the MCP server.

use requiem_core::Directory;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared state for the MCP server.
///
/// This contains the loaded requirements directory and is wrapped in Arc<RwLock>
/// for thread-safe access across async tasks.
#[derive(Clone)]
pub struct ServerState {
    /// The requirements directory loaded on startup.
    pub directory: Arc<RwLock<Directory>>,
}

impl ServerState {
    /// Create a new server state by loading the requirements directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be loaded.
    pub fn new(root: impl AsRef<Path>) -> anyhow::Result<Self> {
        let directory = Directory::new(root.as_ref().to_path_buf())?;
        Ok(Self {
            directory: Arc::new(RwLock::new(directory)),
        })
    }
}
