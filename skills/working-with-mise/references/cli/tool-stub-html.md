tool-stub` [​](#mise-tool-stub)

- **Usage:** `mise tool-stub <FILE> [ARGS]…`
- **Source code:** [`src/cli/tool_stub.rs`](https://github.com/jdx/mise/blob/main/src/cli/tool_stub.rs)

Run a tool stub file

A tool stub is an executable file whose shebang is `#!/usr/bin/env -S mise tool-stub` and whose body is TOML naming a tool, version, and executable. Running the stub runs this command, which installs the tool if needed and runs it with the stub's arguments. You rarely call it directly; create stubs with `mise generate tool-stub`.

```
#!/usr/bin/env -S mise tool-stub
# Node.js 24

tool = "node"
version = "24"
bin = "node"
```

See <https://mise.jdx.dev/dev-tools/tool-stubs.html>

## Arguments [​](#arguments)

- **`<FILE>`** — Path to the TOML tool stub file to execute

  Every field is optional: `version` defaults to `latest`, `tool` defaults to the stub's file name (or `http:<file name>` when the stub sets `url` or platform URLs), and `bin` defaults to the stub's file name. A stub can also set backend-specific options.
- **`[ARGS]…`** — Arguments to pass to the tool

  Every argument after the stub path is passed to the tool unchanged, including a literal `--`.

## Related documentation [​](#related-documentation)

- [Portable tool stubs](https://mise.jdx.dev/dev-tools/tool-stubs.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/tool-stub.md)

Last updated:

Pager

[Previous pagemise tool-alias unset](https://mise.jdx.dev/cli/tool-alias/unset.html)

[Next pagemise trust](https://mise.jdx.dev/cli/trust.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)