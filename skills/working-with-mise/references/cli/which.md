which` [​](#mise-which)

- **Usage:** `mise which [FLAGS] [BIN_NAME]`
- **Effect:** read-only
- **Source code:** [`src/cli/which.rs`](https://github.com/jdx/mise/blob/main/src/cli/which.rs)

Show the path a tool's executable resolves to

Prints the real path of the executable mise would run for BIN\_NAME in the current directory, bypassing shims. Use `--version` or `--plugin` to see which tool version provides it.

## Arguments [​](#arguments)

- **`[BIN_NAME]`** — The executable to look up

## Flags [​](#flags)

- **`-t --tool <TOOL@VERSION>`** — Look in a specific tool version, such as `mise which npm --tool=node@20`
- **`--plugin`** — Show the name of the tool that provides the executable
- **`--version`** — Show the version instead of the path
- **`-h --help`** — Print help

## Examples [​](#examples)

Show the path of the node executable

```
mise which node
~/.local/share/mise/installs/node/20.0.0/bin/node
```

Show the tool that provides it

```
mise which node --plugin
node
```

Show the version that provides it

```
mise which node --version
20.0.0
```

## Related documentation [​](#related-documentation)

- [Shims and executable lookup](https://mise.jdx.dev/dev-tools/shims.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/which.md)

Last updated:

Pager

[Previous pagemise where](https://mise.jdx.dev/cli/where.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)