`mbx clean` [​](#mbx-clean)

- **Usage:** `mbx clean [--under <ROOT>] [WORKSPACE]`

Remove a workspace's managed target and learned incremental state.

mbx also removes the checkout's `target` link when it points at that managed target. Shared store objects and the workspace's cache claims stay, so a later build can restore matching outputs; `mbx cache remove` also forgets the claims. A managed target or learned incremental state that a running command is using is kept, with a warning. Use `mbx clean --under <ROOT>` to remove every recorded workspace at or below an absolute path, including nested worktrees. `--under` and the positional workspace path cannot be used together.

## Arguments [​](#arguments)

- **`[WORKSPACE]`** — Workspace root whose managed target and learned incremental state are removed. Defaults to the current workspace.

## Flags [​](#flags)

- **`--under <ROOT>`** — Remove managed targets and learned incremental state recorded at or below this absolute path. The path does not need to exist. Mutually exclusive with WORKSPACE.
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/clean.rs)

Last updated:

Pager

[Previous pageanalyze](https://mr-boxington.jdx.dev/cli/analyze)

[Next pageadopt](https://mr-boxington.jdx.dev/cli/adopt)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)