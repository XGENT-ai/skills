tool` [​](#mise-tool)

- **Usage:** `mise tool [FLAGS] <TOOL>`
- **Effect:** read-only
- **Source code:** [`src/cli/tool.rs`](https://github.com/jdx/mise/blob/main/src/cli/tool.rs)

Show information about a tool

Shows the tool's backend, its description and project URL when known, the installed, active, and requested versions, the config file that requests it, its tool options, and the backend's security features. Pass a field flag such as `--backend` to print one field, or `--json` for all of them.

## Arguments [​](#arguments)

- **`<TOOL>`** — The tool, such as `node` or `aqua:jqlang/jq`

## Flags [​](#flags)

- **`-J --json`** — Output in JSON format
- **`--active`** — Only show active versions
- **`--backend`** — Only show backend field
- **`--config-source`** — Only show config source
- **`--description`** — Only show description field
- **`--installed`** — Only show installed versions
- **`--requested`** — Only show requested versions
- **`--tool-options`** — Only show tool options
- **`--url`** — Only show the project URL from the registry
- **`-h --help`** — Print help

## Examples [​](#examples)

Show everything mise knows about node

```
mise tool node
Backend:            core:node
Installed Versions: 22.0.0 24.0.0
Active Version:     24.0.0
Requested Version:  24
Config Source:      ~/.config/mise/config.toml
Tool Options:       [none]
Security:           checksum (sha256), gpg
```

Print only the backend

```
mise tool node --backend
core:node
```

## Related documentation [​](#related-documentation)

- [Development tools](https://mise.jdx.dev/dev-tools/).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/tool.md)

Last updated:

Pager

[Previous pagemise token gitlab](https://mise.jdx.dev/cli/token/gitlab.html)

[Next pagemise tool-alias](https://mise.jdx.dev/cli/tool-alias.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)