search` [​](#mise-search)

- **Usage:** `mise search [FLAGS] [NAME]`
- **Effect:** read-only
- **Source code:** [`src/cli/search.rs`](https://github.com/jdx/mise/blob/main/src/cli/search.rs)

Search for available tools

Searches the registry and the tool lists of installed backend plugins for tools matching NAME. When nothing matches, it also searches the aqua registry. Prefix NAME with `npm:`, `cargo:`, `gem:`, or `dotnet:` to also search that package registry, or pass `--all` to search every source.

Matching is fuzzy by default. Use `--match-type equal` or `--match-type contains` for stricter matches.

## Arguments [​](#arguments)

- **`[NAME]`** — The tool to search for

## Flags [​](#flags)

- **`-a --all`** — Search every source: the registry, aqua, installed backend plugins, and the npm, cargo, gem, and dotnet package registries
- **`-i --interactive`** — Show an interactive search menu
- **`-m --match-type <MATCH_TYPE>`** — Match type: equal, contains, or fuzzy

  **Choices:** `equal`, `contains`, `fuzzy`

  **Default:** `fuzzy`
- **`--no-header`** — Do not print the table header
- **`-h --help`** — Print help

## Examples [​](#examples)

Fuzzy-search the registry for jq

```
mise search jq
Tool  Description
jq    Command-line JSON processor. https://github.com/jqlang/jq
jqp   A TUI playground to experiment with jq. https://github.com/noahgorstein/jqp
jiq   jid on jq - interactive JSON query tool using jq expressions. https://github.com/fiatjaf/jiq
gojq  Pure Go implementation of jq. https://github.com/itchyny/gojq
```

Look up one npm package by its exact name

```
mise search --match-type equal npm:typescript-language-server
Tool                            Description
npm:typescript-language-server  Language Server Protocol (LSP) implementation for TypeScript using tsserver
```

Pick a tool from a filterable list

```
mise search --interactive
```

## Related documentation [​](#related-documentation)

- [Registry and explicit backends](https://mise.jdx.dev/registry.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/search.md)

Last updated:

Pager

[Previous pagemise run](https://mise.jdx.dev/cli/run.html)

[Next pagemise secrets](https://mise.jdx.dev/cli/secrets.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)