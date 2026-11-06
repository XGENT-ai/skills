tasks` [​](#mise-tasks)

- **Usage:** `mise tasks [FLAGS] [TASK] [SUBCOMMAND]`
- **Aliases:** `t`, `task`
- **Effect:** read-only
- **Source code:** [`src/cli/tasks/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/tasks/mod.rs)

Manage tasks

With no subcommand, lists tasks like `mise tasks ls` and accepts the same flags. With a task name, shows that task like `mise tasks info`. To run a task, use `mise run`.

## Arguments [​](#arguments)

- **`[TASK]`** — Task to show, as `mise tasks info` does

## Flags [​](#flags)

- **`-g --global`** — Only show global tasks
- **`-J --json`** — Output in JSON format
- **`-l --local`** — Deprecated alias for --no-global

  Removed in mise 2027.7.0.
- **`--no-global`** — Only show non-global tasks
- **`-x --extended`** — Show all columns
- **`-a --all`** — Include tasks from every project in the monorepo, not only the current directory's hierarchy
- **`--hidden`** — Show hidden tasks
- **`--name-only`** — Print only task names, one per line (for fzf and similar tools)
- **`--tag <TAG>`** — Only show tasks that have this tag; repeat to accept any of several
- **`--no-header`** — Do not print the table header
- **`--sort <COLUMN>`** — Column to sort by (default: name)

  **Choices:** `name`, `alias`, `description`, `source`
- **`--sort-order <SORT_ORDER>`** — Sort direction (default: asc)

  **Choices:** `asc`, `desc`
- **`-h --help`** — Print help

## Examples [​](#examples)

List tasks

```
mise tasks
```

Show the build task

```
mise tasks build
```

Include hidden tasks, sorted by source

```
mise tasks --hidden --sort source
```

## Subcommands [​](#subcommands)

- [`mise tasks add [FLAGS] <TASK> [-- RUN]…`](https://mise.jdx.dev/cli/tasks/add.html)
- [`mise tasks deps [FLAGS] [TASKS]…`](https://mise.jdx.dev/cli/tasks/deps.html)
- [`mise tasks edit [-p --path] <TASK>`](https://mise.jdx.dev/cli/tasks/edit.html)
- [`mise tasks graph [FLAGS]`](https://mise.jdx.dev/cli/tasks/graph.html)
- [`mise tasks info [-J --json] <TASK>`](https://mise.jdx.dev/cli/tasks/info.html)
- [`mise tasks ls [FLAGS]`](https://mise.jdx.dev/cli/tasks/ls.html)
- [`mise tasks rm [FLAGS] <TASKS>…`](https://mise.jdx.dev/cli/tasks/rm.html)
- [`mise tasks set [FLAGS] <TASK> [-- RUN]…`](https://mise.jdx.dev/cli/tasks/set.html)
- [`mise tasks validate [--errors-only] [--json] [TASKS]…`](https://mise.jdx.dev/cli/tasks/validate.html)

## Related documentation [​](#related-documentation)

- [Task configuration reference](https://mise.jdx.dev/tasks/task-configuration.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/tasks.md)

Last updated:

Pager

[Previous pagemise sync ruby](https://mise.jdx.dev/cli/sync/ruby.html)

[Next pagemise tasks add](https://mise.jdx.dev/cli/tasks/add.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)