`mbx cache export` [​](#mbx-cache-export)

- **Usage:** `mbx cache export [--group <GROUP>] [--format <FORMAT>] <ARCHIVE>`

Export the cache entries this checkout's last build produced or used.

The bundle also carries Cargo's scheduler state for each recorded workspace. Compiler outputs in that state point at cache objects in the bundle instead of being stored twice.

## Arguments [​](#arguments)

- **`<ARCHIVE>`** — Tar archive or directory to write.

## Flags [​](#flags)

- **`--group <GROUP>`** — CI group to export instead of this checkout's last build.

  The bundle covers every finished build that ran with `MBX_CACHE_EXPORT_GROUP` set to this value and has not been exported yet. A successful export marks those builds as exported, so exporting the same group again includes only builds that finished since, and fails when there are none. A failed export leaves them pending for a retry.
- **`--format <FORMAT>`** — Bundle layout: `tar` for a portable archive, or `directory` for a CI cache.

  Use `directory` with a backend that archives a directory itself, such as the GitHub Actions cache; a tar there would write every byte twice.

  **Default:** `tar`
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/cache.rs)

Last updated:

Pager

[Previous pagetrace](https://mr-boxington.jdx.dev/cli/cache/trace)

[Next pageimport](https://mr-boxington.jdx.dev/cli/cache/import)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)