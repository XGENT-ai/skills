bin-paths` [​](#mise-bin-paths)

- **Usage:** `mise bin-paths [--bin-names] [-J --json] [TOOL@VERSION]…`
- **Effect:** read-only
- **Source code:** [`src/cli/bin_paths.rs`](https://github.com/jdx/mise/blob/main/src/cli/bin_paths.rs)

List the bin directories of the active tools

Prints the bin directories of the tool versions the current config selects. It does not include `_.path` entries from \[env]; `mise doctor path` lists everything mise adds to PATH.

## Arguments [​](#arguments)

- **`[TOOL@VERSION]…`** — Only list these tools, such as `ruby@3`

## Flags [​](#flags)

- **`--bin-names`** — Print executable names instead of bin directories
- **`-J --json`** — Print executables as JSON objects with `name`, `path`, and `symlink` (implies --bin-names)
- **`-h --help`** — Print help

## Examples [​](#examples)

Print the bin directory of every active tool

```
mise bin-paths
```

Print only node's bin directory

```
mise bin-paths node
```

List the executables the active tools provide

```
mise bin-paths --bin-names
```

## Related documentation [​](#related-documentation)

- [Shims and executable lookup](https://mise.jdx.dev/dev-tools/shims.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/bin-paths.md)

Last updated:

Pager

[Previous pagemise backends switch](https://mise.jdx.dev/cli/backends/switch.html)

[Next pagemise bootstrap](https://mise.jdx.dev/cli/bootstrap.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)