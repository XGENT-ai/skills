mbx gc

`mbx gc` [​](#mbx-gc)

- **Usage:** `mbx gc [FLAGS]`

Collect learned incremental state and managed targets, then evict cached objects to fit budgets.

A missing cached object is rebuilt when it is needed again.

## Flags [​](#flags)

- **`--max-size <SIZE>`** — Size the store may occupy afterwards, for example 20GiB. Defaults to the configured budget.
- **`--json`** — Print a stable machine-readable report.
- **`--dry-run`** — Show what collection would remove without changing any files.
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/gc.rs)

Last updated:

Pager

[Previous pageclean](https://mr-boxington.jdx.dev/cli/clean)

[Next pagetui](https://mr-boxington.jdx.dev/cli/tui)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)