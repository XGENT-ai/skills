backends` [​](#mise-backends)

- **Usage:** `mise backends [SUBCOMMAND]`
- **Aliases:** `b`, `backend`, `backend-list`
- **Effect:** read-only
- **Source code:** [`src/cli/backends/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/backends/mod.rs)

Manage backends

A backend is where mise installs a tool from: a package registry such as npm or cargo, a release host such as GitHub, or a plugin. Each tool uses one, either written as a prefix (`npm:prettier`) or chosen by the registry. With no subcommand, lists the built-in backends. See <https://mise.jdx.dev/dev-tools/backends/>.

## Flags [​](#flags)

- **`-h --help`** — Print help

## Subcommands [​](#subcommands)

- [`mise backends ls`](https://mise.jdx.dev/cli/backends/ls.html)
- [`mise backends switch [-n --dry-run] [-g --global] [TOOL@VERSION]…`](https://mise.jdx.dev/cli/backends/switch.html)

Deprecation:

The `mise b` alias is deprecated and will be removed in mise 2027.4.0; use `mise backends`.

## Related documentation [​](#related-documentation)

- [Choosing backends](https://mise.jdx.dev/dev-tools/backends/).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/backends.md)

Last updated:

Pager

[Previous pagemise activate](https://mise.jdx.dev/cli/activate.html)

[Next pagemise backends ls](https://mise.jdx.dev/cli/backends/ls.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)