`mbx cache import` [​](#mbx-cache-import)

- **Usage:** `mbx cache import <ARCHIVE>`

Import a cache export into the local store.

A directory export is consumed: mbx moves its objects into the store and then removes the directory. When the export carries Cargo workspace state and you run the command from a matching checkout whose target directory is absent or empty, mbx restores that state too. mbx never replaces a non-empty target directory.

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