`mbx exec` [​](#mbx-exec)

- **Usage:** `mbx exec [--project-root <DIR>] <COMMAND>…`

Run a build command outside Cargo with its C and C++ compilations cached.

Options for exec, such as `--project-root`, go before the command. Everything after the command name belongs to the command, including a later `--`, as in `mbx exec cmake --build build -- -j8`.

## Arguments [​](#arguments)

- **`<COMMAND>…`** — Build command and its arguments.

## Flags [​](#flags)

- **`--project-root <DIR>`** — Directory that identifies the project across checkouts.

  Defaults to the enclosing Git, Jujutsu, Mercurial, or Sapling checkout, or to the working directory outside one.
- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/exec.rs)

Last updated:

Pager

[Previous pageprefetch](https://mr-boxington.jdx.dev/cli/prefetch)

[Next pagecache](https://mr-boxington.jdx.dev/cli/cache/)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)