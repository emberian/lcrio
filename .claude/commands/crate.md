---
argument-hint: <crate name or search query>
---

Use the `lcrio` CLI to search and browse Rust crates from the local panamax mirror of crates.io. The full index and all .crate files are available locally at ~/crates.io/full/.

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
