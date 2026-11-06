implode` [​](#mise-implode)

- **Usage:** `mise implode [-n --dry-run] [--config]`
- **Effect:** destructive — may delete or irreversibly overwrite
- **Source code:** [`src/cli/implode.rs`](https://github.com/jdx/mise/blob/main/src/cli/implode.rs)

Remove the mise CLI and all related data

Deletes the mise executable and the data (including installed tools), state, and cache directories. It also deletes the system-wide shared directory used by `mise install --system` (MISE\_SYSTEM\_DATA\_DIR, default /usr/local/share/mise), which other users on the machine may rely on. It asks before each removal; --yes skips the prompts. The config directory stays unless you pass --config.

If a package manager such as Homebrew or apt installed mise, uninstall it with that package manager instead. Remove the `mise activate` line from your shell startup file yourself. See <https://mise.jdx.dev/installing-mise.html#uninstalling>.

## Flags [​](#flags)

- **`-n --dry-run`** — Print what would be removed, without removing anything
- **`--config`** — Also remove the config directory
- **`-h --help`** — Print help

## Examples [​](#examples)

List what would be removed

```
mise implode --dry-run
```

## Related documentation [​](#related-documentation)

- [Uninstalling mise](https://mise.jdx.dev/installing-mise.html#uninstalling).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/implode.md)

Last updated:

Pager

[Previous pagemise generate tool-stub](https://mise.jdx.dev/cli/generate/tool-stub.html)

[Next pagemise install](https://mise.jdx.dev/cli/install.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)