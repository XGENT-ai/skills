mbx cache export

`mbx cache export` [​](#mbx-cache-export)

- **Usage:** `mbx cache export [--group <GROUP>] [--format <FORMAT>] <ARCHIVE>`

Export the cache closure of this checkout's last build. The export includes Cargo scheduler state for recorded workspaces, with compiler outputs referenced from the content-addressed closure instead of duplicated.

## Arguments [​](#arguments)

- **`<ARCHIVE>`** — Tar archive or directory to write.

## Flags [​](#flags)

- **`--group <GROUP>`** — Export every build that set MBX\_CACHE\_EXPORT\_GROUP to this CI group.
- **`--format <FORMAT>`** — Bundle layout: tar for a portable archive, or directory for a transport that archives a directory itself, such as the GitHub Actions cache.

  **Default:** `tar`
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/cache.rs)

Last updated:

Pager

[Previous pagetrace](https://mr-boxington.jdx.dev/cli/cache/trace)

[Next pageimport](https://mr-boxington.jdx.dev/cli/cache/import)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)