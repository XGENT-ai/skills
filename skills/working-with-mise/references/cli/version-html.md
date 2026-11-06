version` [​](#mise-version)

- **Usage:** `mise version [-J --json]`
- **Aliases:** `v`
- **Effect:** read-only
- **Source code:** [`src/cli/version.rs`](https://github.com/jdx/mise/blob/main/src/cli/version.rs)

Show the version of mise

Prints the version, OS, architecture, and build date, and warns when a newer release is available. `mise --version`, `mise -V`, and `mise -v` with no other arguments print the same line.

## Flags [​](#flags)

- **`-J --json`** — Print the version information in JSON format
- **`-h --help`** — Print help

## Examples [​](#examples)

Show the version

```
mise version
2026.10.4 linux-x64 (2026-10-07)
```

Also show the latest release and the build time, as JSON

```
mise version --json
```

## Related documentation [​](#related-documentation)

- [Troubleshooting](https://mise.jdx.dev/troubleshooting.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/version.md)

Last updated:

Pager

[Previous pagemise vars unset](https://mise.jdx.dev/cli/vars/unset.html)

[Next pagemise watch](https://mise.jdx.dev/cli/watch.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)