`mbx adopt` [​](#mbx-adopt)

- **Usage:** `mbx adopt [-r --recursive] [--dry-run] [PATH]…`

Move existing target directories under mbx management, contents intact.

mbx moves each directory under the managed target root and leaves a `target` link in its place. The adopted directory then follows the same collection policy as any other managed target.

## Arguments [​](#arguments)

- **`[PATH]…`** — Cargo checkouts to adopt, or directories to search with `--recursive`.

  Defaults to the current directory.

## Flags [​](#flags)

- **`-r --recursive`** — Search below each path for Cargo checkouts with a target directory.

  The search skips symbolic links and hidden or `target` directories.
- **`--dry-run`** — Show what would be adopted without moving anything.
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/adopt.rs)

Last updated:

Pager

[Previous pageclean](https://mr-boxington.jdx.dev/cli/clean)

[Next pagegc](https://mr-boxington.jdx.dev/cli/gc)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)