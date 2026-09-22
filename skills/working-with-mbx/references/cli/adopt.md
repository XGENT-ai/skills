mbx adopt

`mbx adopt` [​](#mbx-adopt)

- **Usage:** `mbx adopt [-r --recursive] [--dry-run] [PATH]…`

Bring existing Cargo target directories under mbx management without deleting their contents.

mbx moves each directory under the managed root and leaves a `target` link in its place. The adopted directory then follows the usual managed-target collection policy.

## Arguments [​](#arguments)

- **`[PATH]…`** — Cargo checkouts to adopt, or directories to search with --recursive. Defaults to the current directory.

## Flags [​](#flags)

- **`-r --recursive`** — Search below each path for Cargo checkouts with a target directory. Hidden directories, target directories, and symbolic links are skipped.
- **`--dry-run`** — Report what would be adopted without moving anything.
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/adopt.rs)

Last updated:

Pager

[Next pageDocumentation](https://mr-boxington.jdx.dev/guide)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)