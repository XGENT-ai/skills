installs` [​](#mise-installs)

- **Usage:** `mise installs [SUBCOMMAND]`
- **Effect:** read-only
- **Source code:** [`src/cli/installs/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/installs/mod.rs)

\[experimental] Inspect and choose identity-layout installations

With `install_layout = "identity"`, each installation lives in its own `<label>-<hash>` directory, and several installations of one version can exist side by side (different options, platforms, pinned artifacts, or a refreshed copy). These commands list them, choose which one mise uses when no lockfile pins one, and move installations made before you turned the layout on into it. With no subcommand, runs `mise installs ls`. See <https://mise.jdx.dev/dev-tools/install-layout.html>.

## Flags [​](#flags)

- **`-h --help`** — Print help

## Subcommands [​](#subcommands)

- [`mise installs ls [-J --json] [--no-header] [TOOL]…`](https://mise.jdx.dev/cli/installs/ls.html)
- [`mise installs migrate [-n --dry-run] [TOOL@VERSION]…`](https://mise.jdx.dev/cli/installs/migrate.html)
- [`mise installs select <INSTALLATION>`](https://mise.jdx.dev/cli/installs/select.html)

## Related documentation [​](#related-documentation)

- [Identity install layout](https://mise.jdx.dev/dev-tools/install-layout.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/installs.md)

Last updated:

Pager

[Previous pagemise install-into](https://mise.jdx.dev/cli/install-into.html)

[Next pagemise installs ls](https://mise.jdx.dev/cli/installs/ls.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)