mcp` [​](#mise-mcp)

- **Usage:** `mise mcp`
- **Source code:** [`src/cli/mcp.rs`](https://github.com/jdx/mise/blob/main/src/cli/mcp.rs)

Run the Model Context Protocol server over stdin/stdout

Exposes project tools, tasks, environment, and configuration to an MCP client. The client, such as an AI coding assistant, starts this command and exchanges messages with it. Start it in the project directory or pass `--cd`: the resources describe that project.

Resources (JSON):

- mise://tools: active tool versions (?include\_inactive=true adds the rest)
- mise://tasks: task definitions, including monorepo subproject tasks
- mise://env: environment variables
- mise://config: loaded config files and the project root

Environment resources contain real values, including secrets.

Tools:

- list\_commands: every mise command and its declared effect
- run\_task: run a task with your permissions; it can change files and call external services
- install\_tool: install a tool version, as `mise install` does

run\_task and install\_tool refuse a project whose config files are not trusted; run `mise trust` there after reviewing them.

See <https://mise.jdx.dev/mcp.html> for client configuration and access controls.

## Flags [​](#flags)

- **`-h --help`** — Print help

## Examples [​](#examples)

Serve a project from another directory, as an MCP client config does

```
mise --cd /path/to/project mcp
```

## Related documentation [​](#related-documentation)

- [AI assistants (MCP)](https://mise.jdx.dev/mcp.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/mcp.md)

Last updated:

Pager

[Previous pagemise lsp](https://mise.jdx.dev/cli/lsp.html)

[Next pagemise oci](https://mise.jdx.dev/cli/oci.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)