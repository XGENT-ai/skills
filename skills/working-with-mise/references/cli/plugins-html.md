plugins` [​](#mise-plugins)

- **Usage:** `mise plugins [FLAGS] [SUBCOMMAND]`
- **Aliases:** `p`, `plugin`, `plugin-list`
- **Effect:** read-only
- **Source code:** [`src/cli/plugins/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/plugins/mod.rs)

Manage plugins

Plugins add tools, backends, environment directives, or bootstrap package managers to mise. Most tools need no plugin because most registry tools use built-in backends, and mise installs a missing plugin when a configured tool needs it.

With no subcommand, lists installed plugins (same as `mise plugins ls`; the flags below are passed to it). See <https://mise.jdx.dev/plugins.html>

## Flags [​](#flags)

- **`-c --core`** — List the core tools built into mise instead of installed plugins
- **`-u --urls`** — Show each plugin's Git URL, ref, and commit
- **`--user`** — List installed plugins

  This is the default behavior but can be used with --core to show core and user plugins
- **`-h --help`** — Print help

## Subcommands [​](#subcommands)

- [`mise plugins install [FLAGS] [NEW_PLUGIN] [GIT_URL]`](https://mise.jdx.dev/cli/plugins/install.html)
- [`mise plugins link [-f --force] <NAME> [DIR]`](https://mise.jdx.dev/cli/plugins/link.html)
- [`mise plugins ls [FLAGS]`](https://mise.jdx.dev/cli/plugins/ls.html)
- [`mise plugins ls-remote [-u --urls] [--only-names]`](https://mise.jdx.dev/cli/plugins/ls-remote.html)
- [`mise plugins uninstall [-a --all] [-p --purge] [PLUGIN]…`](https://mise.jdx.dev/cli/plugins/uninstall.html)
- [`mise plugins update [-j --jobs <JOBS>] [PLUGIN]…`](https://mise.jdx.dev/cli/plugins/update.html)

## Related documentation [​](#related-documentation)

- [Plugins](https://mise.jdx.dev/plugins.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/plugins.md)

Last updated:

Pager

[Previous pagemise patrons](https://mise.jdx.dev/cli/patrons.html)

[Next pagemise plugins install](https://mise.jdx.dev/cli/plugins/install.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)