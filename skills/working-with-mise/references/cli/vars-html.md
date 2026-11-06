vars` [​](#mise-vars)

- **Usage:** `mise vars [FLAGS] [NAME[=VALUE]]… [SUBCOMMAND]`
- **Effect:** modifies state
- **Source code:** [`src/cli/vars/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/vars/mod.rs)

List, set, and ask for config vars

Like `mise set`: with no arguments, lists every var and where it comes from; `mise vars NAME=VALUE` saves a value on this machine and `mise vars NAME` prints one. Saved values live in `$MISE_STATE_DIR/vars.toml`, outside every config file, and sit above a var's `default` but below any value a config file sets. `mise vars prompt` asks for the `[vars]` that declare a `prompt`; only it and `mise bootstrap --prompt-vars` ever ask. See <https://mise.jdx.dev/configuration/vars.html#prompt>

## Arguments [​](#arguments)

- **`[NAME[=VALUE]]…`** — Values to save ( `NAME=VALUE`) or to print ( `NAME`)

## Flags [​](#flags)

- **`-J --json`** — Output in JSON format
- **`--json-extended`** — Output in JSON format with sources
- **`--no-header`** — Do not print the table header
- **`-h --help`** — Print help

## Examples [​](#examples)

Save a value on this machine without asking

```
mise vars git_name="Ada Lovelace"
```

Print the saved value of git\_name

```
mise vars git_name
```

List every var and its source

```
mise vars
```

## Subcommands [​](#subcommands)

- [`mise vars ls [FLAGS]`](https://mise.jdx.dev/cli/vars/ls.html)
- [`mise vars prompt [NAME[=VALUE]]…`](https://mise.jdx.dev/cli/vars/prompt.html)
- [`mise vars unset [NAME]…`](https://mise.jdx.dev/cli/vars/unset.html)

## Related documentation [​](#related-documentation)

- [Config variables](https://mise.jdx.dev/configuration/vars.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/vars.md)

Last updated:

Pager

[Previous pagemise use](https://mise.jdx.dev/cli/use.html)

[Next pagemise vars ls](https://mise.jdx.dev/cli/vars/ls.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)