daemons` [​](#mise-daemons)

- **Usage:** `mise daemons [--json] [SUBCOMMAND]`
- **Aliases:** `daemon`
- **Effect:** read-only
- **Source code:** [`src/cli/daemons.rs`](https://github.com/jdx/mise/blob/main/src/cli/daemons.rs)

\[experimental] Manage project daemons with pitchfork

Declare long-running processes in `[daemons]`, either as commands or as service presets such as `postgres` and `redis`, and mise runs them under pitchfork with the project's tools and environment. Requires `experimental = true`.

With no subcommand, lists the project's daemons like `mise daemons ls`.

See <https://mise.jdx.dev/daemons.html>

## Flags [​](#flags)

- **`--json`** — Output in JSON format, including ports, URLs, and data directories
- **`-h --help`** — Print help

## Examples [​](#examples)

Start the default group, or every project daemon

```
mise daemons start
```

Show the last 50 lines of api's output

```
mise daemons logs api -n 50
```

Stop every registered project daemon

```
mise daemons stop
```

## Subcommands [​](#subcommands)

- [`mise daemons logs [ARGS]…`](https://mise.jdx.dev/cli/daemons/logs.html)
- [`mise daemons ls [--json]`](https://mise.jdx.dev/cli/daemons/ls.html)
- [`mise daemons providers [SUBCOMMAND]`](https://mise.jdx.dev/cli/daemons/providers.html)
- [`mise daemons prune [-n --dry-run]`](https://mise.jdx.dev/cli/daemons/prune.html)
- [`mise daemons register`](https://mise.jdx.dev/cli/daemons/register.html)
- [`mise daemons restart [ARGS]…`](https://mise.jdx.dev/cli/daemons/restart.html)
- [`mise daemons start [ARGS]…`](https://mise.jdx.dev/cli/daemons/start.html)
- [`mise daemons status [ARGS]…`](https://mise.jdx.dev/cli/daemons/status.html)
- [`mise daemons stop [ARGS]…`](https://mise.jdx.dev/cli/daemons/stop.html)
- [`mise daemons tui [ARGS]…`](https://mise.jdx.dev/cli/daemons/tui.html)
- [`mise daemons urls [--json]`](https://mise.jdx.dev/cli/daemons/urls.html)

## Related documentation [​](#related-documentation)

- [Daemons](https://mise.jdx.dev/daemons.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/daemons.md)

Last updated:

Pager

[Previous pagemise config set](https://mise.jdx.dev/cli/config/set.html)

[Next pagemise daemons logs](https://mise.jdx.dev/cli/daemons/logs.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)