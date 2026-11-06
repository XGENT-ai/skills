link` [​](#mise-link)

- **Usage:** `mise link [-f --force] <TOOL@VERSION> <PATH>`
- **Aliases:** `ln`
- **Effect:** modifies state
- **Source code:** [`src/cli/link.rs`](https://github.com/jdx/mise/blob/main/src/cli/link.rs)

Symlink a tool version into mise

Use this to register an install that was compiled by hand or built with another tool. mise uses the version exactly as written, such as `node@24.11.0` or a name of your own like `node@brew`; `latest`, aliases, and channels are rejected.

## Arguments [​](#arguments)

- **`<TOOL@VERSION>`** — Tool and version to create a symlink for
- **`<PATH>`** — Directory of the existing installation, such as `~/.nvm/versions/node/v24.11.0`

## Flags [​](#flags)

- **`-f --force`** — Replace an existing installation of this version
- **`-h --help`** — Print help

## Examples [​](#examples)

Link a node that node-build built into \~/.nodes/24.11.0

```
mise link node@24.11.0 ~/.nodes/24.11.0
```

Register Homebrew's node as node@brew; then run `mise use node@brew`

```
mise link node@brew "$(brew --prefix node)"
```

## Related documentation [​](#related-documentation)

- [Development tools](https://mise.jdx.dev/dev-tools/).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/link.md)

Last updated:

Pager

[Previous pagemise latest](https://mise.jdx.dev/cli/latest.html)

[Next pagemise lock](https://mise.jdx.dev/cli/lock.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)