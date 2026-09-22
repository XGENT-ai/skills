mbx explain

`mbx explain` [​](#mbx-explain)

- **Usage:** `mbx explain [--last] [CARGO_COMMAND] [CARGO_ARGS]…`

Explain cache bypasses, or replay the last build and diagnose its misses.

## Arguments [​](#arguments)

- **`[CARGO_COMMAND]`** — Cargo subcommand to run under diagnostics.
- **`[CARGO_ARGS]…`** — Arguments to pass to the Cargo subcommand.

## Flags [​](#flags)

- **`--last`** — Explain the most recent recorded build without running Cargo again.
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/explain.rs)

Last updated:

Pager

[Previous pagedoctor](https://mr-boxington.jdx.dev/cli/doctor)

[Next pageclean](https://mr-boxington.jdx.dev/cli/clean)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)