sync` [​](#mise-sync)

- **Usage:** `mise sync <SUBCOMMAND>`
- **Effect:** read-only
- **Source code:** [`src/cli/sync/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/sync/mod.rs)

Use tool versions installed by other version managers

Links node versions from nvm, nodenv, or Homebrew, python versions from pyenv or uv, and ruby versions from Homebrew into mise, so `mise use` can select them without reinstalling.

## Flags [​](#flags)

- **`-h --help`** — Print help

## Subcommands [​](#subcommands)

- [`mise sync node [FLAGS]`](https://mise.jdx.dev/cli/sync/node.html)
- [`mise sync python [--pyenv] [--uv]`](https://mise.jdx.dev/cli/sync/python.html)
- [`mise sync ruby <--brew>`](https://mise.jdx.dev/cli/sync/ruby.html)

## Related documentation [​](#related-documentation)

- [Development tools](https://mise.jdx.dev/dev-tools/).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/sync.md)

Last updated:

Pager

[Previous pagemise ssh](https://mise.jdx.dev/cli/ssh.html)

[Next pagemise sync node](https://mise.jdx.dev/cli/sync/node.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)