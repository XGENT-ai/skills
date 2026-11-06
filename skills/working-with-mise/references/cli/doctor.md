doctor` [​](#mise-doctor)

- **Usage:** `mise doctor [-J --json] [SUBCOMMAND]`
- **Aliases:** `dr`
- **Effect:** read-only
- **Source code:** [`src/cli/doctor/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/doctor/mod.rs)

Check your mise setup for problems

Prints mise's version, whether shell activation and shims are set up, its directories, the config files, tools, and environment in use, and its settings, then lists any problems and warnings with a suggested fix. Exits with status 1 when it finds a problem. Include this output when you report a bug.

## Flags [​](#flags)

- **`-J --json`** — Output in JSON format
- **`-h --help`** — Print help

## Examples [​](#examples)

Check the setup and list problems

```
mise doctor
```

Print the report as JSON

```
mise doctor --json
```

## Subcommands [​](#subcommands)

- [`mise doctor path [-f --full]`](https://mise.jdx.dev/cli/doctor/path.html)
- [`mise doctor project [-J --json]`](https://mise.jdx.dev/cli/doctor/project.html)

## Related documentation [​](#related-documentation)

- [Troubleshooting](https://mise.jdx.dev/troubleshooting.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/doctor.md)

Last updated:

Pager

[Previous pagemise deps remove](https://mise.jdx.dev/cli/deps/remove.html)

[Next pagemise doctor path](https://mise.jdx.dev/cli/doctor/path.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)