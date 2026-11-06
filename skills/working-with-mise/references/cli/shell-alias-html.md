shell-alias` [​](#mise-shell-alias)

- **Usage:** `mise shell-alias [--no-header] [SUBCOMMAND]`
- **Effect:** read-only
- **Source code:** [`src/cli/shell_alias/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/shell_alias/mod.rs)

Manage shell aliases

Shell aliases are defined under `[shell_alias]` in mise.toml. In a shell where mise is activated, mise sets them when you enter the directory and removes them when you leave. With no subcommand, lists them (same as `mise shell-alias ls`). See <https://mise.jdx.dev/shell-aliases.html>

## Flags [​](#flags)

- **`--no-header`** — Do not print the table header
- **`-h --help`** — Print help

## Subcommands [​](#subcommands)

- [`mise shell-alias get <shell_alias>`](https://mise.jdx.dev/cli/shell-alias/get.html)
- [`mise shell-alias ls [--no-header]`](https://mise.jdx.dev/cli/shell-alias/ls.html)
- [`mise shell-alias set <shell_alias> [COMMAND]`](https://mise.jdx.dev/cli/shell-alias/set.html)
- [`mise shell-alias unset <shell_alias>`](https://mise.jdx.dev/cli/shell-alias/unset.html)

## Related documentation [​](#related-documentation)

- [Shell aliases](https://mise.jdx.dev/shell-aliases.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/shell-alias.md)

Last updated:

Pager

[Previous pagemise shell](https://mise.jdx.dev/cli/shell.html)

[Next pagemise shell-alias get](https://mise.jdx.dev/cli/shell-alias/get.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)