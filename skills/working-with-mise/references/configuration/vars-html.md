Config variables [​](#config-variables)

Define shared values in `[vars]` and reference them with `{{ vars.NAME }}` in a [Tera template](https://mise.jdx.dev/templates.html). Use vars for values that mise needs to render config; use [`[env]`](https://mise.jdx.dev/environments/) for values that commands need as environment variables. mise does not export vars to child processes.

```
[vars]
node_version = "24"
test_mode = "headless"

[tools]
node = "{{ vars.node_version }}"

[tasks.test]
run = "echo {{ vars.test_mode | quote }}"
```

`mise run test` prints `headless`. The task reads `test_mode` through the `vars` template map; there is no `$test_mode` environment variable. The `quote` filter quotes for POSIX shells; see [template quoting](https://mise.jdx.dev/templates.html#string-manipulation).

Vars are available wherever mise renders templates in config, such as tool versions and options, task definitions, [hooks](https://mise.jdx.dev/hooks.html), [`watch_files`](https://mise.jdx.dev/hooks.html#watch-files-hook), [task includes](https://mise.jdx.dev/tasks/task-discovery.html), and [dotfile templates](https://mise.jdx.dev/dotfiles.html).

## Configuration hierarchy [​](#configuration-hierarchy)

Vars follow the [config file hierarchy](https://mise.jdx.dev/configuration.html#configuration-hierarchy): a higher-precedence file overrides a var of the same name. Define a default in global config:

\~/.config/mise/config.toml

```
[vars]
test_mode = "headless"
```

Then override it for your own checkout:

mise.local.toml

```
[vars]
test_mode = "headed"
```

## Value directives [​](#value-directives)

Vars accept the same value directives as [`[env]`](https://mise.jdx.dev/environments/), including defaults, required values, redaction, files, sources, and [secrets](https://mise.jdx.dev/environments/secrets/):

```
[vars]
test_mode = { default = "headless" }
api_token = { required = "Set api_token in mise.local.toml" }
_.file = { path = ".env.secret", redact = true }
```

- `default` uses a process environment variable with the same name when it is set and not empty, and the given value otherwise. Values from `[env]` are not used for this lookup.
- `required` fails when no value is supplied by the process environment or a higher-precedence config file, such as `mise.local.toml`. The string is shown as help in the error.
- `redact = true` hides the value in task output; here it applies to every value loaded from `.env.secret`.

See the [`env._` directive reference](https://mise.jdx.dev/environments/#env-directives) for the file, source, and plugin directive forms. Under `[vars]`, these directives fill `vars` instead of exporting environment variables.

## Ask once and remember [​](#prompt)

`prompt` makes a var a per-machine answer. Pair it with a `default` to offer a suggestion, or with `required` when there is none:

```
[vars.git_name]
default = "Ada Lovelace"
prompt = "Git author name"

[vars.git_email]
required = "Run mise vars prompt to set git_email"
prompt = "Git email"
```

Run `mise vars prompt` to be asked for each of these vars that has no answer yet; name vars to ask only for those (`mise vars prompt git_name`). Enter accepts the default. mise saves every answer, including an accepted default, in `$MISE_STATE_DIR/vars.toml`, so it is never asked again, never lands in your config or dotfiles history, and is available to every command that reads vars from then on.

sh

```
mise vars                       # list every var and where it comes from
mise vars --json                # the same as a JSON object; --json-extended adds sources
mise vars git_name=Ada          # save a value on this machine, no terminal needed
mise vars git_name              # print the value templates see
mise vars prompt                # ask for every unanswered prompt var
mise vars unset git_name        # forget a saved value, to be asked again
```

`mise bootstrap --prompt-vars` does the same inside a bootstrap run, which is how a fresh machine answers them with `mise bootstrap --adopt owner/dotfiles`: the config that declares the vars only exists once the checkout has been fetched. A `--dry-run` still saves the answers it asks for.

Only `mise vars prompt` and `--prompt-vars` ever ask. Everything else, including shell activation, tasks, and runs without a terminal, uses the saved answer and otherwise falls back to `default`, or fails as any other `required` var does.

A var resolves in this order, highest first: the process environment, a value in a higher-precedence config file such as `mise.local.toml`, the saved answer, the `default`. Answers are keyed by var name alone, so projects that use the same name share one answer on a machine. A value saved with `mise vars NAME=VALUE` works for any var name, with or without a `prompt`, and replaces that var's `default` on this machine. `prompt` works only in `[vars]`, not `[env]`.

## Task-local vars [​](#task-local-vars)

TOML tasks can define their own vars. A task-local value overrides a config var while that task is rendered, without changing the value anywhere else:

```
[vars]
test_mode = "headless"

[tasks.test]
vars = { test_mode = "headed" }
run = "echo {{ vars.test_mode | quote }}"
```

`mise run test` prints `headed`; other tasks still see `headless` unless they define their own value. See [task `vars`](https://mise.jdx.dev/tasks/task-configuration.html#task-vars).

## When vars are resolved [​](#when-vars-are-resolved)

mise resolves top-level `[vars]` entries while it loads config, before any task-local vars apply. Each entry can reference the entries resolved before it. The result is stored as a string, so later references do not evaluate the template again:

```
[vars]
mode = "headless"
args = "--mode={{ vars.mode }}"
```

`args` resolves to `--mode=headless`. If a higher-precedence file such as `mise.local.toml` sets `mode = "headed"`, `{{ vars.mode }}` renders `headed`, but `args` stays `--mode=headless`. To change `args`, override `args` itself or build the string in the task that uses it.

### What a task-local var can change [​](#what-a-task-local-var-can-change)

A task-local override changes direct references to that var in the task's templated fields, including fields inherited from a task template. It does not recalculate top-level vars that used the original value:

```
[vars]
mode = "headless"
args = "--mode={{ vars.mode }}"

[tasks.test]
vars = { mode = "headed" }
run = "echo {{ vars.args }} / {{ vars.mode }}"
```

`mise run test` prints:

text

```
--mode=headless / headed
```

To make the argument follow the task's mode, build it in `run`:

```
[tasks.test]
vars = { mode = "headed" }
run = "echo --mode={{ vars.mode }}"
```

This prints `--mode=headed`. When several tasks share the same command, put `run` in a [task template](https://mise.jdx.dev/tasks/templates.html#parameterizing-a-template-with-vars) and let each task supply its vars.

### Missing vars and defaults [​](#missing-vars-and-defaults)

A top-level var cannot reference a value that exists only in a task's `vars`. Without a fallback, the reference fails when mise loads the config. The Tera `default` filter supplies a fallback where the expression is rendered:

```
[vars]
args = "--mode={{ vars.mode | default(value='headless') }}"

[tasks.test]
vars = { mode = "headed" }
run = "echo {{ vars.args }} / {{ vars.mode }}"
```

This also prints `--mode=headless / headed`: the filter supplies `headless` while `[vars]` loads, and does not wait for the task to supply `mode`. Put expressions that depend on task-local values in the task or its template.

[Edit this page](https://github.com/jdx/mise/edit/main/docs/configuration/vars.md)

Last updated:

Pager

[Previous pageSettings](https://mise.jdx.dev/configuration/settings.html)

[Next pageTera templates](https://mise.jdx.dev/templates.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)