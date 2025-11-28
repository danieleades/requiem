# CLI Command Reference

Current command set for the `req` CLI. The default command is `status`.

> Early-stage note: Cycle detection and full structure validation are not implemented yet. The `validate` command currently reports suspect links, stale parent HRIDs, and path drift only.

## Quick Map

- `status` — counts by kind + suspect/path drift summary (default)
- `create` — create a requirement (namespaces via dash-separated KIND)
- `link` / `unlink` — manage parent-child links
- `review` — list suspect links; `--accept` to update fingerprints
- `sync` — update stored parent HRIDs and/or move files to canonical paths
- `list` — filtered listings with relationship views (`parents`, `children`, `tree`, etc.)
- `show` — pretty detail view of a single requirement
- `validate` — health checks (paths/HRID drift/suspect links for now)
- `complete` — generate shell completion scripts (bash, zsh, fish, PowerShell)
- `init`, `kind`, `rename`, `move`, `delete`, `config`, `diagnose paths`

## Global Synopsis

```
req [OPTIONS] <COMMAND>
```

Running `req` with no subcommand executes `req status`.

### Getting Help

```bash
req --help
req create --help
req review --help
req sync --help
```

### Global Options

- `-v, --verbose` — increase logging (repeatable, up to TRACE)
- `-r, --root <PATH>` — requirements directory (default: `.`)

Examples:

```bash
req -v status                  # INFO logging
req --root ./docs/src/requirements create USR --title "Example"
```

## Commands

### status

```
req status [--output table|json] [--quiet]
```

Shows counts by kind plus suspect-link and path-drift totals. Exits with code `2` when suspect links or path issues exist (CI-friendly).

### create

```
req create <KIND> [--parent <PARENT>...] [--title <TITLE>] [--body <BODY>]
```

Creates a requirement with the next ID for the KIND. KIND supports namespaces via dash-separated tokens (e.g., `AUTH-USR`). Templates in `.req/templates/` are used when no title/body is supplied. HRID is stored in the first heading, not in frontmatter.

### link / unlink

```
req link <CHILD> <PARENT>
req unlink <CHILD> <PARENT>
```

Creates or removes parent-child relationships. Multiple parents per child are supported.

### review

```
req review [--child <HRID>] [--parent <HRID>] [--kind <KIND>] [--quiet]
req review --accept [--all] [--yes]
```

Lists suspect links (fingerprint drift or missing parents). Exits `2` when suspects exist. Use `--accept` to update fingerprints after review; `--all` accepts every suspect link.

### sync

```
req sync [--what parents|paths|all] [--check] [--dry-run] [--yes] [--quiet]
```

`parents` updates stored parent HRIDs in children. `paths` moves files to canonical locations (respecting namespace/path mode). `all` does both. `--check`/`--dry-run` are non-destructive; exit `2` when drift is found.

### validate

```
req validate [--check <TYPE>...] [--fix] [--dry-run] [--output table|json|summary]
```

Currently checks for path drift, stale parent HRIDs, and suspect links. Structure/cycle/broken-reference checks are TODO.

### list

```
req list [HRID...] [--kind <KIND>...] [--namespace <NS>...] [--tag <TAG>...]
        [--contains <TEXT>|--regex <RE>] [--view <VIEW>] [--output table|json|csv]
```

Filters by kind/namespace/tags/text. Relationship views include `parents`, `children`, `ancestors`, `descendants`, `tree`, and `context`. Default limit is 200 rows.

### show

```
req show <HRID> [--with-children] [--with-parents] [--output table|json]
```

Displays a single requirement with parents/children and metadata. Options vary; use `--help` for full list.

### init

```
req init [--kinds <KIND>...]
```

Creates `.req/config.toml` and `.req/templates/`. Adding kinds here is optional; you can also manage them later via `req kind`.

### kind

```
req kind add <KIND>... [--description <TEXT>]
req kind remove <KIND>...
req kind list
```

Registers allowed kinds and optional descriptions in `.req/config.toml`.

### rename / move / delete

- `req rename <HRID> <NEW_HRID>` — rename HRID and heading
- `req move <HRID> <PATH>` — move file to a new path
- `req delete <HRID> [--cascade|--orphan] [--dry-run] [--yes]` — delete with safety flags

### diagnose

```
req diagnose paths
```

Reports files that are not at their canonical paths (namespaces/path mode aware).

### complete

```
req complete <SHELL>
```

Generates shell completion scripts for the specified shell. Supported shells are `bash`, `zsh`, `fish`, and `powershell`. Output the completion script to stdout and install it to your shell's completion directory.

**Examples:**

```bash
# Generate bash completions
req complete bash > req.bash
sudo install -Dm644 req.bash /usr/share/bash-completion/completions/req

# Generate zsh completions
req complete zsh > _req
mkdir -p ~/.local/share/zsh/site-functions
mv _req ~/.local/share/zsh/site-functions/

# Generate fish completions
req complete fish > req.fish
mkdir -p ~/.config/fish/completions
mv req.fish ~/.config/fish/completions/

# Generate PowerShell completions
req complete powershell >> $PROFILE
```

See the [Shell Completions](../guide/installation.md#shell-completions) section in the installation guide for detailed setup instructions.
