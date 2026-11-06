secrets` [​](#mise-secrets)

- **Usage:** `mise secrets [-J --json] [--no-header] [SUBCOMMAND]`
- **Effect:** read-only
- **Source code:** [`src/cli/secrets/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/secrets/mod.rs)

\[experimental] Inspect the project's secrets source

With no subcommand, lists secret names (same as `mise secrets ls`; the flags below are passed to it). See <https://mise.jdx.dev/environments/secrets/fnox.html>

## Flags [​](#flags)

- **`-J --json`** — Output in JSON format
- **`--no-header`** — Do not print the table header
- **`-h --help`** — Print help

## Subcommands [​](#subcommands)

- [`mise secrets ls [-J --json] [--no-header]`](https://mise.jdx.dev/cli/secrets/ls.html)

## Related documentation [​](#related-documentation)

- [fnox secrets](https://mise.jdx.dev/environments/secrets/fnox.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/secrets.md)

Last updated:

Pager

[Previous pagemise search](https://mise.jdx.dev/cli/search.html)

[Next pagemise secrets ls](https://mise.jdx.dev/cli/secrets/ls.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)