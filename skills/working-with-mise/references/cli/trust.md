trust` [​](#mise-trust)

- **Usage:** `mise trust [FLAGS] [CONFIG_FILE]`
- **Effect:** modifies state
- **Source code:** [`src/cli/trust.rs`](https://github.com/jdx/mise/blob/main/src/cli/trust.rs)

Mark a config file as trusted

Trust lets mise load the parts of a config file that can run code or change your environment. Without it, mise prompts, skips the file, or fails with an untrusted-config error when it cannot prompt.

A file with no template syntax that contains only `min_version`, `[tools]` entries with plain version strings (or arrays of them), and `[tasks]` in which no task lists `secrets` does not need trust unless paranoid mode is on. Commands that run project code (`mise run`, task shorthand such as `mise build`, `mise install`, `mise exec`, `mise watch`, and `mise daemons start|restart|register`) trust their active config automatically.

In paranoid mode, `mise trust` with no path also trusts the env files and scripts that config reads (`env._.file`, `env._.source` and the `env_file` setting), which have trust of their own. Give a file's path to trust only that file.

Trusting a config file in a repository's main checkout also trusts the same path in its git worktrees. Paranoid mode (<https://mise.jdx.dev/paranoid.html>) turns off automatic and worktree trust and requires trust, tied to the file's contents, for every config outside the global and system config. See <https://mise.jdx.dev/security.html#configuration-trust>

## Arguments [​](#arguments)

- **`[CONFIG_FILE]`** — The config file whose trust status to change

## Flags [​](#flags)

- **`-a --all`** — Trust all config files in the current directory, its parents, and its subdirectories

  Subdirectories are walked respecting .gitignore, skipping hidden directories and common build and dependency directories (node\_modules, vendor, target, dist, build).
- **`--ignore`** — Do not trust this config and ignore it in the future
- **`--show`** — Show the trust status of config files in the current directory and its parents

  Does not trust or untrust any files.
- **`--untrust`** — Remove explicit trust for this config
- **`-h --help`** — Print help

## Examples [​](#examples)

Trust the nearest untrusted config in this directory or a parent, and in paranoid mode the env files it reads

```
mise trust
```

Trust a specific file

```
mise trust ~/src/app/mise.toml
```

List configs here and above with their trust status

```
mise trust --show
```

## Related documentation [​](#related-documentation)

- [Configuration trust](https://mise.jdx.dev/security.html#configuration-trust).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/trust.md)

Last updated:

Pager

[Previous pagemise tool-stub](https://mise.jdx.dev/cli/tool-stub.html)

[Next pagemise uninstall](https://mise.jdx.dev/cli/uninstall.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)