`mbx cache remove` [​](#mbx-cache-remove)

- **Usage:** `mbx cache remove [--interactive] [WORKSPACE]`

Remove a workspace's managed target, learned incremental state, and cache claims.

Provide exactly one of `<WORKSPACE>` or `--interactive`. mbx keeps a managed target or learned incremental state that a running command is using. Shared cache objects stay in the store for other workspaces and normal collection.

## Arguments [​](#arguments)

- **`[WORKSPACE]`** — Workspace root to forget.

## Flags [​](#flags)

- **`--interactive`** — Choose from a list of recorded workspaces to remove.

  Requires a terminal.
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/cache.rs)

Last updated:

Pager

[Previous pageimport](https://mr-boxington.jdx.dev/cli/cache/import)

[Next pageStability](https://mr-boxington.jdx.dev/stability)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)