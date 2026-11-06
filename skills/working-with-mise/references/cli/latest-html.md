latest` [​](#mise-latest)

- **Usage:** `mise latest [-i --installed] [--minimum-release-age <AGE>] <TOOL@VERSION>`
- **Effect:** read-only
- **Source code:** [`src/cli/latest.rs`](https://github.com/jdx/mise/blob/main/src/cli/latest.rs)

Print the latest version that matches a version request

`mise latest node` prints the latest release, and `mise latest node@22` the latest 22.x release. Each backend decides what "latest" means, for example whether prereleases count and how channels and refs resolve. Nothing is installed and no config changes.

## Arguments [​](#arguments)

- **`<TOOL@VERSION>`** — Tool, optionally with a version prefix, such as `node` or `node@22`

## Flags [​](#flags)

- **`-i --installed`** — Print the latest installed version instead of the latest available one
- **`--minimum-release-age <AGE>`** — Only consider versions released before a date or at least a duration ago

  Takes a date such as `2024-06-01` or a duration such as `90d` or `1y`. Overrides the `minimum_release_age` setting and tool option.
- **`-h --help`** — Print help

## Examples [​](#examples)

Print the latest node release

```
mise latest node
```

Print the latest node 22.x release

```
mise latest node@22
```

Print the latest installed node 22.x version

```
mise latest node@22 --installed
```

Skip releases from the last 30 days

```
mise latest node --minimum-release-age 30d
```

## Related documentation [​](#related-documentation)

- [Version requests](https://mise.jdx.dev/dev-tools/versions.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/latest.md)

Last updated:

Pager

[Previous pagemise installs select](https://mise.jdx.dev/cli/installs/select.html)

[Next pagemise link](https://mise.jdx.dev/cli/link.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)