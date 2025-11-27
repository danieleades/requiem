# req-mcp: Model Context Protocol Server for Requirements Management

An MCP server for discovering, navigating, and managing requirement graphs.

## Running

Set `REQ_ROOT` to point to your requirements directory (typically `docs/src/requirements`), then run:

```sh
cargo run --release --bin req-mcp
```

The server listens on stdin/stdout for JSON-RPC messages (MCP protocol).

## Tools

### Implemented
- **`list_requirement_kinds`**: List all requirement kinds
- **`list_requirements`**: List requirements by kind with optional substring filtering
- **`get_requirement`**: Fetch a requirement by HRID with title, body, parents, and children
- **`get_children`**: Get direct child requirements
- **`create_requirement_kind`**: Create a new requirement kind
- **`create_requirement`**: Create a new requirement with optional parent links
- **`review_requirement`**: Mark a suspect parent-child link as reviewed

### Not Yet Implemented
- `search_requirements`, `review`, `get_parents`, `get_ancestors`, `get_descendants`, `update_requirement`

## Local Setup

### Claude (claude-code)

Claude supports per-repo MCP configuration via `.mcp.json` **or** via its CLI.

**Local config (`.mcp.json` in repo root):**
```json
{
  "mcpServers": {
    "requiem": {
      "command": "cargo",
      "args": ["run", "-r", "--manifest-path", "{REPO_ROOT}/Cargo.toml", "--bin", "req-mcp"],
      "env": {
        "REQ_ROOT": "{REPO_ROOT}/docs/src/requirements"
      }
    }
  }
}
```
Open Claude in this directory; it will pick up the config. In chat, run `/mcp list` or ask the agent to use `req`.

**CLI add (from repo root):**
```sh
claude mcp add --transport stdio requiem --env REQ_ROOT=$(pwd)/docs/src/requirements --scope project -- cargo run -r --manifest-path $(pwd)/Cargo.toml --bin req-mcp
```

### Codex CLI

Codex uses `~/.codex/config.toml`. Add:

```toml
[mcp_servers.requiem]
command = "cargo"
args = ["run", "-r", "--manifest-path", "{REPO_ROOT}/Cargo.toml", "--bin", "req-mcp"]

[mcp_servers.requiem.env]
REQ_ROOT = "{REPO_ROOT}/docs/src/requirements"

[features]
rmcp_client = true
```

Restart Codex. Alternatively, add via CLI:

```sh
codex mcp add requiem --env REQ_ROOT=$(pwd)/docs/src/requirements -- cargo run -r --manifest-path $(pwd)/Cargo.toml --bin req-mcp
```
