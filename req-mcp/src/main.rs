//! MCP server for requirements management.
//!
//! This server provides tools for discovering and navigating requirements
//! using the Model Context Protocol (MCP).

mod server;
mod state;
mod tools;

use std::path::PathBuf;

use anyhow::{Context, Result};
use rmcp::{transport::stdio, ServiceExt};
use server::ReqMcpServer;
use state::ServerState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is reserved for JSON-RPC)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load requirements directory from environment variable
    let repo_root = std::env::var("REQ_ROOT").context("REQ_ROOT environment variable required")?;
    let root = PathBuf::from(repo_root);
    if !root.is_dir() {
        anyhow::bail!("REQ_ROOT '{}' is not a directory", root.display());
    }

    let canonical_root = root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", root.display()))?;

    tracing::info!("Loading requirements from {}", canonical_root.display());
    let state = ServerState::new(&canonical_root).with_context(|| {
        format!(
            "failed to load requirements from {}",
            canonical_root.display()
        )
    })?;

    // Count requirements for logging
    let count = state.directory.read().await.requirements().count();
    tracing::info!("Loaded {} requirements", count);

    let server = ReqMcpServer::new(state);

    tracing::info!("Starting MCP server over stdio");
    let service = server.serve(stdio()).await?;
    let quit_reason = service.waiting().await?;
    tracing::info!("Server stopped: {:?}", quit_reason);

    Ok(())
}
