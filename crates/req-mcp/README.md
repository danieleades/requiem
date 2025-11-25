# req-mcp: Model Context Protocol Server for Requirements Management

MCP server providing tools for discovering and navigating requirements graphs.

## Status

**WORK IN PROGRESS** - Minimal read-only MVP is implemented; editing tools remain stubbed.

Works with any MCP client (Claude Desktop, Codex CLI, and others) once the server is running on stdio.

## Dogfooding

If you're contributing with an AI agent, please enable this MCP server so the agent reads and navigates the project's own requirements. It keeps edits aligned with the spec and helps validate the server.

## Architecture

```
req-mcp/
├── src/
│   ├── main.rs       # Server entry point, loads requirements directory
│   └── state.rs      # Shared server state (Directory wrapped in Arc<RwLock>)
```

### What's Done

✅ **Workspace Structure**: Created `req-core` crate with domain & storage logic
✅ **State Management**: Server state loads `Directory` on startup
✅ **Async Runtime**: Tokio-based async setup
✅ **Logging**: Structured logging to stderr (stdout reserved for JSON-RPC)
✅ **MCP wiring**: Stdio JSON-RPC transport with rmcp tool router/handler
✅ **Discovery tooling**: `list_requirement_kinds`, `list_requirements`, `get_requirement`, `get_children`
✅ **Stubs exposed**: All other planned tools return a "not implemented" payload (keeps surface visible)
✅ **Compilation**: Code compiles successfully

### What's NOT Done

❌ **Search & review**: `search_requirements` and `review` are stubbed
❌ **Lineage beyond children**: `get_parents`, `get_ancestors`, `get_descendants` are stubbed
❌ **Editing**: Create/update/review tools are stubbed; no persistence paths wired
❌ **Testing**: Not tested with Claude Desktop/Codex clients yet

## Design Review (MCP best practices)

- **Transport/IO**: Plan remains stdio JSON-RPC; keep all human-readable logs on stderr (stdout reserved for protocol messages), which matches MCP guidance.
- **Typed tools**: Use `#[tool_router]` with parameter structs deriving `Serialize`, `Deserialize`, and `JsonSchema` so clients can render forms and validate input before sending.
- **Error handling**: Map domain errors (missing HRID, invalid filter) to user-facing messages instead of panics; avoid leaking filesystem paths in responses; prefer consistent error variants for client UX.
- **State & concurrency**: `Directory` is under `Arc<RwLock>`; keep tool bodies non-blocking and move any heavy disk/graph work to `spawn_blocking` to avoid stalling other requests.
- **Configuration**: Validate `REQ_ROOT` on startup, normalize to an absolute path, and return a friendly startup error if missing.
- **Contract clarity**: Keep tool names and response shapes stable across Claude, Codex, and other clients; document minimal guaranteed fields for each tool.

## Planned Tools

### Discovery Tools
- `list_requirements` - List with filters (kind, namespace, tags, orphans, leaves)
- `list_requirement_kinds` - List all kinds of requirements
- `get_requirement` - Get single requirement by HRID
- `search_requirements` - Text/regex search
- `review` - Get requirements needing review

### Navigation Tools
- `get_children` - Immediate or recursive children
- `get_parents` - Immediate or recursive parents
- `get_ancestors` - Transitive parents (all ancestors)
- `get_descendants` - Transitive children (all descendants)

### Editing Tools
- `create_requirement_kind` - Create new kind of requirement
- `create_requirement` - Create new requirement (including linking to parents)
- `update_requirement` - Update existing requirement (including linking to parents)
- `review_requirement` - Mark requirement as reviewed

## Minimal MVP scope

Implement a bootable server with just enough functionality to demonstrate end-to-end graph navigation, while exposing stubs for everything else:

- **Fully implement (real data):** `list_requirement_kinds`, `list_requirements` (basic filters: kind + optional substring search), `get_requirement` (include direct parent/child IDs), `get_children` (immediate only).
- **Stub (placeholder responses):** All other planned tools should return a short "Not implemented yet" message plus TODO metadata so clients see the full surface.
- **Server behavior:** Start from `REQ_ROOT`, fail fast with a friendly error if missing, log startup details to stderr, and serve JSON-RPC over stdio.

## Next Steps

1. **Finish navigation**
   - Implement `get_parents`, `get_ancestors`, and `get_descendants` using the in-memory graph.
   - Add structured outputs (parents/children with depth and summaries).

2. **Search & review**
   - Implement `search_requirements` (text + regex) and `review` filters.
   - Add pagination if needed for large graphs.

3. **Editing & persistence**
   - Wire create/update/review tools to `req-core` write paths and flush changes to disk.
   - Validate inputs with schemas and return user-friendly errors.

4. **Client smoke-tests (Claude, Codex, others)**
   - Launch with `cargo run --bin req-mcp` and `REQ_ROOT` pointing at `docs/src/requirements`.
   - Exercise the full read path: list kinds → list requirements for a kind → fetch one HRID → fetch its children.

5. **Dogfooding & polish**
   - Iterate using the project's own requirements to validate graph traversal.
   - Add simple tracing spans for each tool and ensure backpressure/log noise stay low.

## Dependencies

- `req-core` - Core domain logic (Tree, Directory, Hrid, Requirement)
- `rmcp` (0.8+) - Official Rust MCP SDK
- `tokio` - Async runtime
- `serde`, `serde_json` - Serialization
- `tracing` - Structured logging
- `anyhow` - Error handling

## References

- [MCP Specification](https://modelcontextprotocol.io/)
- [rmcp crate](https://crates.io/crates/rmcp)
- [Official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Building MCP Servers in Rust](https://mcpcat.io/guides/building-mcp-server-rust/)

## Client configuration

### Claude (claude-code)

Claude supports per-repo MCP configuration via `.mcp.json` **or** via its CLI.

**Local config (`.mcp.json` in repo root):**
```json
{
  "mcpServers": {
    "req": {
      "command": "cargo",
      "args": ["run", "-r", "--manifest-path", "__REPO_ROOT__/Cargo.toml", "--bin", "req-mcp"],
      "env": {
        "REQ_ROOT": "__REPO_ROOT__/docs/src/requirements"
      }
    }
  }
}
```
Open Claude in this directory; it will pick up the config. In chat, run `/mcp list` or ask the agent to use `req`.

**CLI add (from repo root):**
```sh
claude mcp add --transport stdio req --env REQ_ROOT=${pwd}/docs/src/requirements -- cargo run -r --manifest-path ${pwd}/Cargo.toml --bin req-mcp --scope project
```

### Codex CLI

Codex uses `~/.codex/config.toml`. Add:

```toml
[mcp_servers.req]
command = "cargo"
args = ["run", "-r", "--manifest-path", "__REPO_ROOT__/Cargo.toml", "--bin", "req-mcp"]

[mcp_servers.req.env]
REQ_ROOT = "__REPO_ROOT__/docs/src/requirements"

[features]
rmcp_client = true
```

Restart Codex. Alternatively, add via CLI:

```sh
codex mcp add req --env REQ_ROOT=${pwd}/docs/src/requirements -- cargo run -r --manifest-path ${pwd}/Cargo.toml --bin req-mcp
```
