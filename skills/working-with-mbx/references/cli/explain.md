`mbx explain` [​](#mbx-explain)

- **Usage:** `mbx explain [--last] [CARGO_COMMAND] [CARGO_ARGS]…`

Explain cache bypasses, or replay the last build and diagnose its misses.

With a Cargo command, mbx runs it and explains every compilation that bypassed the cache, grouped by cause, as in `mbx explain build --workspace`. With `--last`, mbx runs nothing: it reads this workspace's most recent recorded build and explains each miss against an earlier recording of that compilation unit, preferring this workspace's own.

## Arguments [​](#arguments)

- **`[CARGO_COMMAND]`** — Cargo subcommand, such as `build`, to run and check for cache bypasses.
- **`[CARGO_ARGS]…`** — Arguments to pass to the Cargo subcommand.

## Flags [​](#flags)

- **`--last`** — Explain this workspace's most recent recorded build without running Cargo.

  mbx reads the newest build recorded for the Cargo workspace that contains the current directory and fails when there is none; it never picks a newer build from another checkout. It compares each miss with the most recent earlier recording of that compilation unit: from this workspace when it has one, otherwise from another checkout of the same project (one whose `Cargo.lock` is identical).
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/explain.rs)

Last updated:

Pager

[Previous pagedoctor](https://mr-boxington.jdx.dev/cli/doctor)

[Next pageanalyze](https://mr-boxington.jdx.dev/cli/analyze)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)