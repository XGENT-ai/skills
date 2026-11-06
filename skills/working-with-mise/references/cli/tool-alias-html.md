tool-alias` [​](#mise-tool-alias)

- **Usage:** `mise tool-alias [-p --tool <TOOL>] [--no-header] [SUBCOMMAND]`
- **Aliases:** `alias`, `aliases`
- **Effect:** read-only
- **Source code:** [`src/cli/tool_alias/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/tool_alias/mod.rs)

Manage tool backend and version aliases

A backend alias makes a tool name install from another backend: `[tool_alias] ripgrep = "aqua:BurntSushi/ripgrep"`. A version alias names a version request: `[tool_alias.node.versions] project = "20"` lets you write `node@project`.

With no subcommand, lists version aliases (same as `mise tool-alias ls`; the flags below are passed to it). See <https://mise.jdx.dev/dev-tools/aliases.html>

## Flags [​](#flags)

- **`-p --tool <TOOL>`** — Filter aliases by tool
- **`--no-header`** — Do not print the table header
- **`-h --help`** — Print help

## Subcommands [​](#subcommands)

- [`mise tool-alias get <TOOL> <ALIAS>`](https://mise.jdx.dev/cli/tool-alias/get.html)
- [`mise tool-alias ls [--no-header] [TOOL]`](https://mise.jdx.dev/cli/tool-alias/ls.html)
- [`mise tool-alias set <ARGS>…`](https://mise.jdx.dev/cli/tool-alias/set.html)
- [`mise tool-alias unset <TOOL> [ALIAS]`](https://mise.jdx.dev/cli/tool-alias/unset.html)

## Related documentation [​](#related-documentation)

- [Tool version aliases](https://mise.jdx.dev/dev-tools/aliases.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/tool-alias.md)

Last updated:

Pager

[Previous pagemise tool](https://mise.jdx.dev/cli/tool.html)

[Next pagemise tool-alias get](https://mise.jdx.dev/cli/tool-alias/get.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)