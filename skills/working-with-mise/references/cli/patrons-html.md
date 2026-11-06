patrons` [​](#mise-patrons)

- **Usage:** `mise patrons [-J --json] [--refresh]`
- **Effect:** read-only
- **Source code:** [`src/cli/patrons.rs`](https://github.com/jdx/mise/blob/main/src/cli/patrons.rs)

Show the individuals supporting mise as Patron-tier members

Fetches the list from <https://jdx.dev/patrons.json> and caches it for a day; pass `--refresh` to fetch it now. If the fetch fails, the cached list is used. Terminals that support OSC 8 hyperlinks show each name as a link.

To appear here, become a patron at <https://jdx.dev/sponsors.html>.

## Flags [​](#flags)

- **`-J --json`** — Output in JSON format
- **`--refresh`** — Fetch the list now instead of using the cached copy
- **`-h --help`** — Print help

## Examples [​](#examples)

List the patrons

```
mise patrons
```

Fetch the list now and print it as JSON

```
mise patrons --refresh -J
```

## Related documentation [​](#related-documentation)

- [Supporting mise](https://mise.jdx.dev/about.html#supporting-mise).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/patrons.md)

Last updated:

Pager

[Previous pagemise packslip pins](https://mise.jdx.dev/cli/packslip/pins.html)

[Next pagemise plugins](https://mise.jdx.dev/cli/plugins.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)