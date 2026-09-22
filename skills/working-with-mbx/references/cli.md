mbx

`mbx` [​](#mbx)

**Usage:** `mbx [+TOOLCHAIN] <SUBCOMMAND>`

**Version:** 1.15.0

- **Usage:** `mbx [+TOOLCHAIN] <SUBCOMMAND>`

## Arguments [​](#arguments)

- **`[+TOOLCHAIN]`** — Toolchain to run under, named the way rustup names it: `mbx +1.91 check`.

## Flags [​](#flags)

- **`-h --help`** — Print help
- **`-V --version`** — Print version

## Subcommands [​](#subcommands)

- [`mbx completion <SHELL>`](https://mr-boxington.jdx.dev/cli/completion)
- [`mbx doctor [--json]`](https://mr-boxington.jdx.dev/cli/doctor)
- [`mbx explain [--last] [CARGO_COMMAND] [CARGO_ARGS]…`](https://mr-boxington.jdx.dev/cli/explain)
- [`mbx setup [FLAGS]`](https://mr-boxington.jdx.dev/cli/setup)
- [`mbx gc [FLAGS]`](https://mr-boxington.jdx.dev/cli/gc)
- [`mbx cache <SUBCOMMAND>`](https://mr-boxington.jdx.dev/cli/cache)
- [`mbx cache trace <SESSION>`](https://mr-boxington.jdx.dev/cli/cache/trace)
- [`mbx cache dir [--json]`](https://mr-boxington.jdx.dev/cli/cache/dir)
- [`mbx cache stats [--json]`](https://mr-boxington.jdx.dev/cli/cache/stats)
- [`mbx cache projects`](https://mr-boxington.jdx.dev/cli/cache/projects)
- [`mbx cache largest [--limit <LIMIT>]`](https://mr-boxington.jdx.dev/cli/cache/largest)
- [`mbx cache verify`](https://mr-boxington.jdx.dev/cli/cache/verify)
- [`mbx cache export [--group <GROUP>] [--format <FORMAT>] <ARCHIVE>`](https://mr-boxington.jdx.dev/cli/cache/export)
- [`mbx cache import <ARCHIVE>`](https://mr-boxington.jdx.dev/cli/cache/import)
- [`mbx cache remove [--interactive] [WORKSPACE]`](https://mr-boxington.jdx.dev/cli/cache/remove)
- [`mbx clean [WORKSPACE]`](https://mr-boxington.jdx.dev/cli/clean)
- [`mbx adopt [-r --recursive] [--dry-run] [PATH]…`](https://mr-boxington.jdx.dev/cli/adopt)
- [`mbx tui [--once]`](https://mr-boxington.jdx.dev/cli/tui)
- [`mbx stats [--json]`](https://mr-boxington.jdx.dev/cli/stats)
- [`mbx prefetch <CARGO_ARGS>…`](https://mr-boxington.jdx.dev/cli/prefetch)
- [`mbx exec [--project-root <DIR>] <COMMAND>…`](https://mr-boxington.jdx.dev/cli/exec)

## Configuration [​](#configuration)

- [Settings](https://mr-boxington.jdx.dev/configuration#settings)

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/mod.rs)

Last updated:

Pager

[Previous pageConfiguration](https://mr-boxington.jdx.dev/configuration)

[Next pagesetup](https://mr-boxington.jdx.dev/cli/setup)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)