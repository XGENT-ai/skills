install-into` [​](#mise-install-into)

- **Usage:** `mise install-into <TOOL@VERSION> <PATH>`
- **Effect:** destructive — may delete or irreversibly overwrite
- **Source code:** [`src/cli/install_into.rs`](https://github.com/jdx/mise/blob/main/src/cli/install_into.rs)

Install a tool version into a directory outside mise

Downloads or builds TOOL@VERSION into PATH for use without mise. mise does not track the result, so it does not appear in `mise ls`.

If PATH is not empty, mise asks before deleting its contents; --yes deletes them without asking.

## Arguments [​](#arguments)

- **`<TOOL@VERSION>`** — Tool and version to install, such as `node@24`
- **`<PATH>`** — Directory to install into; its existing contents are deleted

## Flags [​](#flags)

- **`-h --help`** — Print help

## Examples [​](#examples)

Install node 24 into ./mynode and run it

```
mise install-into node@24 ./mynode && ./mynode/bin/node -v
v24.11.0
```

## Related documentation [​](#related-documentation)

- [Development tools](https://mise.jdx.dev/dev-tools/).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/install-into.md)

Last updated:

Pager

[Previous pagemise install](https://mise.jdx.dev/cli/install.html)

[Next pagemise installs](https://mise.jdx.dev/cli/installs.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)