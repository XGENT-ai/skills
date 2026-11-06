settings` [​](#mise-settings)

- **Usage:** `mise settings [FLAGS] [SETTING] [VALUE] [SUBCOMMAND]`
- **Effect:** modifies state
- **Source code:** [`src/cli/settings/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/settings/mod.rs)

Manage settings

With no arguments, lists the settings set in config files (same as `mise settings ls`). With SETTING, prints its effective value. With SETTING=VALUE or SETTING VALUE, writes it to the global config (\~/.config/mise/config.toml), or to the nearest project config with `--local`. Every setting is described at <https://mise.jdx.dev/configuration/settings.html>

## Arguments [​](#arguments)

- **`[SETTING]`** — Only show this setting, or the settings under this prefix such as `python`
- **`[VALUE]`** — The value to set

## Global Flags [​](#global-flags)

- **`-l --local`** — Use only the nearest project config

  Reads only its explicit settings, or writes to it instead of the global config.

## Flags [​](#flags)

- **`-a --all`** — List all settings
- **`-J --json`** — Output in JSON format
- **`-T --toml`** — Output in TOML format
- **`--json-extended`** — Output in JSON format with sources
- **`-h --help`** — Print help

## Examples [​](#examples)

List settings set in config files

```
mise settings
```

Show the effective value of one setting

```
mise settings jobs
```

Set jobs in the global config

```
mise settings jobs=4
```

Set a setting in the project config

```
mise settings --local node.mirror_url https://npmmirror.com/mirrors/node/
```

## Subcommands [​](#subcommands)

- [`mise settings add [-l --local] [--override] <SETTING> [VALUE]`](https://mise.jdx.dev/cli/settings/add.html)
- [`mise settings get [-l --local] <SETTING>`](https://mise.jdx.dev/cli/settings/get.html)
- [`mise settings ls [FLAGS] [SETTING]`](https://mise.jdx.dev/cli/settings/ls.html)
- [`mise settings set [-l --local] [--override] <SETTING> [VALUE]`](https://mise.jdx.dev/cli/settings/set.html)
- [`mise settings unset [-l --local] [--override] <KEY>`](https://mise.jdx.dev/cli/settings/unset.html)

## Related documentation [​](#related-documentation)

- [Settings reference](https://mise.jdx.dev/configuration/settings.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/settings.md)

Last updated:

Pager

[Previous pagemise set](https://mise.jdx.dev/cli/set.html)

[Next pagemise settings add](https://mise.jdx.dev/cli/settings/add.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)