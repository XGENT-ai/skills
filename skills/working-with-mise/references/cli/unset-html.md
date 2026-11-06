unset` [​](#mise-unset)

- **Usage:** `mise unset [FLAGS] [ENV_KEY]…`
- **Effect:** modifies state
- **Source code:** [`src/cli/unset.rs`](https://github.com/jdx/mise/blob/main/src/cli/unset.rs)

Remove environment variables from mise.toml

Edits the same file `mise set` writes: the lowest-precedence TOML file in the nearest config directory, or the global config when run in your home directory. Use `--global` or `--file` to pick another file.

## Arguments [​](#arguments)

- **`[ENV_KEY]…`** — Environment variables to remove, such as NODE\_ENV

## Flags [​](#flags)

- **`-f --file <FILE>`** — The TOML file to edit instead of the default target

  Can be a file or a directory. For a directory, mise uses the config file in it, or one named by MISE\_DEFAULT\_CONFIG\_FILENAME (default `mise.toml`): [https://mise.jdx.dev/configuration/settings.html#default\_config\_filename](https://mise.jdx.dev/configuration/settings.html#default_config_filename) To move the global config file, set MISE\_GLOBAL\_CONFIG\_FILE: [https://mise.jdx.dev/configuration/settings.html#global\_config\_file](https://mise.jdx.dev/configuration/settings.html#global_config_file)

  **Aliases:** `--path`
- **`-g --global`** — Use the global config file
- **`--override`** — Use the global override file, `config.override.toml`

  See <https://mise.jdx.dev/configuration.html#override-file>
- **`-h --help`** — Print help

## Examples [​](#examples)

Remove NODE\_ENV from the project config

```
mise unset NODE_ENV
```

Remove NODE\_ENV from the global config

```
mise unset -g NODE_ENV
```

## Related documentation [​](#related-documentation)

- [Environment variables](https://mise.jdx.dev/environments/).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/unset.md)

Last updated:

Pager

[Previous pagemise uninstall](https://mise.jdx.dev/cli/uninstall.html)

[Next pagemise untrust](https://mise.jdx.dev/cli/untrust.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)