`mbx prefetch` [​](#mbx-prefetch)

- **Usage:** `mbx prefetch <CARGO_ARGS>…`

Download predicted actions from the remote cache without running Cargo.

The predictions are the actions that earlier builds on this platform recorded for the same `Cargo.lock`; when that lockfile has none, mbx borrows an earlier lockfile's. `mbx prefetch` needs `remote.url` and a `remote.mode` that reads (`read-only` or `read-write`).

## Arguments [​](#arguments)

- **`<CARGO_ARGS>…`** — Cargo command and arguments, such as `build --workspace --release`.

  mbx uses them only to find the workspace. It selects the predictions by the workspace's `Cargo.lock`, not by the command, so `build`, `test`, and `clippy` share them whatever their profile or features.

## Flags [​](#flags)

- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/prefetch.rs)

Last updated:

Pager

[Previous pagestats](https://mr-boxington.jdx.dev/cli/stats)

[Next pageexec](https://mr-boxington.jdx.dev/cli/exec)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)