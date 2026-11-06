`mbx reserve` [​](#mbx-reserve)

- **Usage:** `mbx reserve [FLAGS] <COMMAND>…`

Hold scheduler capacity while an external command runs.

This is for work such as a VM, Docker build, or another compiler that consumes the same machine resources but does not run through mbx. It waits for the requested capacity, runs the command, and releases that capacity whenever the command or its mbx parent exits.

`--memory` is charged against `scheduler.memory`, so it needs that setting to be enabled. Specify one or both resource flags, as in `mbx reserve --memory 6GiB --cpus 4 -- ./run-windows-build-in-vm.sh`.

## Arguments [​](#arguments)

- **`<COMMAND>…`** — Command and arguments to run while capacity is reserved.

## Flags [​](#flags)

- **`--cpus <N>`** — CPU cores to hold while the command runs.
- **`--memory <SIZE>`** — Memory to hold from the configured scheduler budget.
- **`--priority <PRIORITY>`** — Whether this reservation yields capacity to normal-priority builds.

  **Default:** `normal`
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/reserve.rs)

Last updated:

Pager

[Next pageDocumentation](https://mr-boxington.jdx.dev/guide)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)