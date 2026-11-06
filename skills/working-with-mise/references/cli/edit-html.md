edit` [​](#mise-edit)

- **Usage:** `mise edit [FLAGS] [PATH]`
- **Effect:** destructive — may delete or irreversibly overwrite
- **Source code:** [`src/cli/edit.rs`](https://github.com/jdx/mise/blob/main/src/cli/edit.rs)

Edit mise.toml interactively

Opens a terminal editor for PATH (default: mise.toml in the current directory) where you can add tools from the registry, pick versions, and set environment variables, tasks, and settings. It pre-fills tools it detects from files in the current directory, such as `.node-version`.

Without an interactive terminal, or with --yes, it writes a commented starter template instead. If PATH already exists it stops with an error rather than replace the file; pass --force to replace it. Use --dry-run to print the result without writing it.

## Arguments [​](#arguments)

- **`[PATH]`** — Config file to edit or create; defaults to mise.toml

## Flags [​](#flags)

- **`-g --global`** — Edit the global config file (\~/.config/mise/config.toml)
- **`-n --dry-run`** — Print the result instead of writing it to the file
- **`-f --force`** — Replace an existing file with the starter template when not opening the editor
- **`-t --tool-versions <FILE>`** — Copy the tools from this .tool-versions file into the config, without opening the editor
- **`-h --help`** — Print help

## Examples [​](#examples)

Edit mise.toml in the current directory

```
mise edit
```

Edit another config file

```
mise edit mise.local.toml
```

Edit the global config

```
mise edit -g
```

Copy the tools from .tool-versions into mise.toml

```
mise edit -t .tool-versions
```

Print the result instead of writing it

```
mise edit -n
```

Write a commented starter config to a new file without opening the editor

```
mise edit -y new.toml
```

Replace an existing mise.toml with the starter config

```
mise edit -y --force
```

## Related documentation [​](#related-documentation)

- [Configuration](https://mise.jdx.dev/configuration.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/edit.md)

Last updated:

Pager

[Previous pagemise dotfiles watch](https://mise.jdx.dev/cli/dotfiles/watch.html)

[Next pagemise en](https://mise.jdx.dev/cli/en.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)