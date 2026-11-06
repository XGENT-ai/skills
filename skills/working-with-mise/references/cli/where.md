where` [​](#mise-where)

- **Usage:** `mise where <TOOL@VERSION>`
- **Effect:** read-only
- **Source code:** [`src/cli/where.rs`](https://github.com/jdx/mise/blob/main/src/cli/where.rs)

Show the install directory of a tool version

Fails if no matching version is installed.

## Arguments [​](#arguments)

- **`<TOOL@VERSION>`** — Tool to look up, such as ruby@3

  With a version prefix, shows the newest installed version that matches it. Otherwise, shows the active installed version.

## Flags [​](#flags)

- **`-h --help`** — Print help

## Examples [​](#examples)

Show the newest installed node 20.x

```
mise where node@20
~/.local/share/mise/installs/node/20.0.0
```

Show the active node, or the newest installed node if no config requests one

```
mise where node
~/.local/share/mise/installs/node/20.0.0
```

## Related documentation [​](#related-documentation)

- [Development tools](https://mise.jdx.dev/dev-tools/).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/where.md)

Last updated:

Pager

[Previous pagemise watch](https://mise.jdx.dev/cli/watch.html)

[Next pagemise which](https://mise.jdx.dev/cli/which.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)