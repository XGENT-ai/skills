registry` [​](#mise-registry)

- **Usage:** `mise registry [FLAGS] [NAME]`
- **Effect:** read-only
- **Source code:** [`src/cli/registry.rs`](https://github.com/jdx/mise/blob/main/src/cli/registry.rs)

List registry shorthand names and their backends

The registry maps short names to backends. For example, `node` uses the built-in node backend (`core:node`). When a tool lists several backends, mise uses the first one that this platform and your settings allow. An explicit identifier such as `aqua:jqlang/jq`, a `[tool_alias]` entry, or a `MISE_BACKENDS_<TOOL>` environment variable overrides that choice.

This is not a list of every tool mise can install. Use an explicit identifier such as `github:owner/repo` for a source without a registry shorthand.

## Arguments [​](#arguments)

- **`[NAME]`** — Show only this tool's backends, in the order mise prefers them

## Flags [​](#flags)

- **`-b --backend <BACKEND>`** — Show only tools for this backend
- **`--hide-aliased`** — Hide names that are aliases of another registry tool
- **`-J --json`** — Output in JSON format
- **`--security`** — Include security features for each tool's backends in JSON output

  Requires --json. Features are merged across a tool's backends. Each backend is resolved separately, so this slows down a full listing.
- **`-h --help`** — Print help

## Examples [​](#examples)

List every registry tool and its backends

```
mise registry
```

Show the backends for one tool

```
mise registry jq
aqua:jqlang/jq asdf:mise-plugins/asdf-jq
```

List the tools that have an aqua backend

```
mise registry --backend aqua
```

Print the registry as JSON

```
mise registry --json
```

## Related documentation [​](#related-documentation)

- [Registry and explicit backends](https://mise.jdx.dev/registry.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/registry.md)

Last updated:

Pager

[Previous pagemise prune](https://mise.jdx.dev/cli/prune.html)

[Next pagemise reshim](https://mise.jdx.dev/cli/reshim.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)