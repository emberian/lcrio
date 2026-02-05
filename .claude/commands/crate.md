---
argument-hint: <crate name or search query>
---

Use the `lcrio` CLI to search and browse Rust crates from a local source (panamax mirror or cargo registry). Auto-detects which source is available: panamax at ~/crates.io/full/ if present, otherwise ~/.cargo/registry/. Workspace filtering is on by default when a Cargo.lock is found nearby.

Global flags:

- `--source panamax|cargo` — Force a specific crate source (default: auto-detect)
- `--all` — Disable workspace filtering (show all available crates)

Available commands:

- `lcrio search <query>` — Search for crates by name (exact, prefix, contains, fuzzy)
  - `--dep` — Find crates that depend on the query crate
  - `--feature` — Find crates with this feature name
  - `--limit N` — Max results (default 20)
  - `--json` — Output as JSON

- `lcrio info <crate> [--latest]` — Show version history and metadata

- `lcrio deps <crate> [version]` — List dependencies (default: latest version)

- `lcrio features <crate> [version]` — List features

- `lcrio unpack <crate> [version]` — Extract .crate and print path

- `lcrio ls <crate> [version] [--pattern "src/**/*.rs"]` — List files in a crate

- `lcrio cat <crate> <path> [version]` — Read a specific file from a crate

- `lcrio clean` — Purge extraction cache

## Workflow for "$ARGUMENTS"

1. Search: `lcrio search $ARGUMENTS` to find matching crates
2. Info: `lcrio info <best_match> --latest` to see metadata
3. Browse: `lcrio ls <crate>` to see file structure, then `lcrio cat <crate> src/lib.rs` to read source
