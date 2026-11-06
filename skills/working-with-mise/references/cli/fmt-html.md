fmt` [​](#mise-fmt)

- **Usage:** `mise fmt [FLAGS]`
- **Effect:** modifies state
- **Source code:** [`src/cli/fmt.rs`](https://github.com/jdx/mise/blob/main/src/cli/fmt.rs)

Format mise TOML configuration

Formats the config files in the current directory in place; --all formats every config file mise loads. Use --check in CI to fail when a file is not formatted, or --stdin to format a document from stdin to stdout.

Formatting normalizes whitespace and writes TOML 1.1 syntax, including multiline inline tables. Top-level entries written as plain, dotted, or inline-table keys go in a standard order: min\_version, env\_file, env\_path,\_, env, vars, hooks, watch\_files, tools, other keys, tasks, task\_config, redactions, alias, plugins, settings. `[table]` sections and the keys inside any table keep the order they were written in.

Lists whose order has no effect are sorted: task `sources` and `outputs`, `task_config.global_inputs` and `input_groups`, and `redactions`. File pattern lists put `@group:` references first, then globs, then literal paths. A list with a `!` exclusion or a comment is left as written.

## Flags [​](#flags)

- **`-a --all`** — Format every config file mise loads, not only those in the current directory
- **`-c --check`** — Check whether the configs are formatted without rewriting them; exits 1 if any are not
- **`-s --stdin`** — Format TOML from stdin and print it to stdout, without touching any file
- **`-h --help`** — Print help

## Examples [​](#examples)

Format the config files in the current directory

```
mise fmt
```

Exit 1 if any config file needs formatting

```
mise fmt --check
```

Print a formatted copy without changing the file

```
mise fmt --stdin < mise.toml
```

## Related documentation [​](#related-documentation)

- [Configuration](https://mise.jdx.dev/configuration.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/fmt.md)

Last updated:

Pager

[Previous pagemise exec](https://mise.jdx.dev/cli/exec.html)

[Next pagemise generate](https://mise.jdx.dev/cli/generate.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)