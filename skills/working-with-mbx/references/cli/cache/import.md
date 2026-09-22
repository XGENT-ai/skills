mbx cache import

`mbx cache import` [​](#mbx-cache-import)

- **Usage:** `mbx cache import <ARCHIVE>`

Import a cache export into the local store. A directory export is consumed: its objects are moved into the store and the directory is removed. If the export contains Cargo workspace state and the command runs from a matching checkout with an absent or empty target directory, restore that state as well. A non-empty target directory is never replaced.

## Arguments [​](#arguments)

- **`<ARCHIVE>`** — Tar archive or directory to import.

## Flags [​](#flags)

- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/cache.rs)

Last updated:

Pager

[Previous pageexport](https://mr-boxington.jdx.dev/cli/cache/export)

[Next pageremove](https://mr-boxington.jdx.dev/cli/cache/remove)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)