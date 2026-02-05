---
argument-hint: <crate name or search query>
---

Browse and read Rust crate source code offline from a local mirror or cargo registry.

Prefer the lcrio MCP tools for individual operations — they avoid shell overhead:
- `search_crates` — search by name, dependency, or feature
- `crate_info` — versions, deps, and features in one call
- `list_crate_files` / `read_crate_file` — browse source
- `unpack_crate` / `clean_cache` — manage extraction cache

Use this skill's CLI workflow below when you need flags like `--json`, `--all`, or `--source`, or when the user explicitly invokes `/crate`.

## CLI reference

Global flags: `--source panamax|cargo` (force source), `--all` (disable workspace filter)

- `lcrio search <query> [--dep] [--feature] [--limit N] [--json]`
- `lcrio info <crate> [--latest]`
- `lcrio deps <crate> [version]`
- `lcrio features <crate> [version]`
- `lcrio unpack <crate> [version]`
- `lcrio ls <crate> [version] [--pattern "src/**/*.rs"]`
- `lcrio cat <crate> <path> [version]`
- `lcrio clean`

## Workflow for "$ARGUMENTS"

1. Search: `lcrio search $ARGUMENTS` to find matching crates
2. Info: `lcrio info <best_match> --latest` to see metadata
3. Browse: `lcrio ls <crate>` to see file structure, then `lcrio cat <crate> src/lib.rs` to read source
