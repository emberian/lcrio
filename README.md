# lcrio

Search, browse, and read Rust crate source code offline. Works against a local [panamax](https://github.com/nickel-org/panamax) mirror or your existing `~/.cargo/registry/`.

## Install

```
cargo install --path .
```

This builds two binaries:

- **`lcrio`** — CLI tool
- **`lcrio-mcp`** — MCP server for Claude Code / AI tooling

## Source detection

lcrio auto-detects which crate source is available:

1. **Panamax mirror** at `~/crates.io/full/` (if `crates.io-index/` exists there)
2. **Cargo registry** at `~/.cargo/registry/` (fallback)

Override with `--source panamax` or `--source cargo`.

## Workspace filtering

When a `Cargo.lock` is found in or above the current directory, results are filtered to only crates in that lockfile. Disable with `--all`.

## CLI usage

```
lcrio search serde                     # name search (exact, prefix, contains, fuzzy)
lcrio search serde --dep               # crates that depend on serde
lcrio search async-trait --feature     # crates with this feature
lcrio search serde --json              # JSON output

lcrio info anyhow --latest             # latest version metadata
lcrio deps tokio 1.38.0               # dependency list
lcrio features tokio                   # feature flags

lcrio ls serde                         # list files in crate
lcrio ls serde --pattern "src/**/*.rs" # glob filter
lcrio cat serde src/lib.rs             # read a file

lcrio unpack serde                     # extract .crate, print path
lcrio clean                            # purge extraction cache
```

## MCP server

Add to your `.claude/mcp.json`:

```json
{
  "mcpServers": {
    "lcrio": {
      "command": "lcrio-mcp"
    }
  }
}
```

Exposes 6 tools:

| Tool | Description |
|------|-------------|
| `search_crates` | Search by name, dependency, or feature |
| `crate_info` | Versions, deps, and features in one call |
| `list_crate_files` | List files with optional glob filter |
| `read_crate_file` | Read a specific file from a crate |
| `unpack_crate` | Extract crate, return filesystem path |
| `clean_cache` | Purge extraction cache |

## Claude Code skill

A `/crate` skill is included in `.claude/commands/crate.md` for guided workflows:

```
/crate serde
```

This triggers a search-then-browse workflow. For individual operations, prefer the MCP tools directly.

## How it works

- **Index**: Reads the crates.io registry index (panamax flat files or cargo sparse cache) for metadata, versions, and dependency info.
- **Search**: Builds an in-memory name index, then scores by exact match, prefix, substring, and fuzzy (Levenshtein) similarity. Dependency and feature search scan index entries directly.
- **Browse**: Extracts `.crate` tarballs (gzipped tar) to `~/.cache/lcrio/unpacked/`. In cargo mode, uses pre-extracted sources from `~/.cargo/registry/src/` when available.
- **Workspace filter**: Parses `Cargo.lock` to restrict results to crates actually used by the current project.
