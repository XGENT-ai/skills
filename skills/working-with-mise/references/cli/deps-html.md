deps` [​](#mise-deps)

- **Usage:** `mise deps [FLAGS] [PROVIDER] [SUBCOMMAND]`
- **Aliases:** `dep`, `prepare`
- **Effect:** modifies state
- **Source code:** [`src/cli/deps/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/deps/mod.rs)

\[experimental] Manage project dependencies

Installs project dependencies such as node\_modules, separately from the tools `mise install` manages. Each provider is one package-manager step: a built-in one such as npm or uv, or a custom `[deps.<name>]` command. A provider runs only when its inputs changed or its outputs are missing; `--explain` shows why.

With no subcommand, runs `mise deps install`. Providers with `auto = true` also run before `mise exec` and `mise run` unless you pass --no-deps.

## Arguments [​](#arguments)

- **`[PROVIDER]`** — Run only this provider, or explain it with --explain

## Flags [​](#flags)

- **`--explain`** — Show why a provider is fresh or stale, and exit non-zero unless it is fresh (requires a PROVIDER argument)
- **`-f --force`** — Run providers even when their outputs are up to date
- **`-n --dry-run`** — Show which providers would run, without running them
- **`--list`** — List configured providers with their status, sources, and outputs
- **`--monorepo`** — Also run providers from every config root in `[monorepo].config_roots`

  Requires `monorepo_root = true` and an explicit `config_roots` list in the monorepo root's config. Provider names then include their root, such as `//apps/api:uv`.

  **Environment Variable:** `MISE_MONOREPO`
- **`--only <PROVIDER>`** — Run only these providers
- **`--skip <PROVIDER>`** — Skip these providers
- **`-h --help`** — Print help

## Examples [​](#examples)

Run every provider that is out of date

```
mise deps
```

Show why the npm provider would or would not run

```
mise deps install npm --explain
```

Include providers from every monorepo config root

```
mise deps --monorepo
```

## Subcommands [​](#subcommands)

- [`mise deps add [-D --dev] <PACKAGES>…`](https://mise.jdx.dev/cli/deps/add.html)
- [`mise deps install [FLAGS] [PROVIDER]`](https://mise.jdx.dev/cli/deps/install.html)
- [`mise deps remove <PACKAGES>…`](https://mise.jdx.dev/cli/deps/remove.html)

Configuration:

```
# Built-in npm provider; needs package.json and package-lock.json
[deps.npm]
auto = true              # run before mise exec and mise run

# Custom provider
[deps.codegen]
auto = true
sources = ["schema/*.graphql"]
outputs = ["src/generated/"]
run = "npm run codegen"
```

See <https://mise.jdx.dev/dev-tools/deps.html>.

## Related documentation [​](#related-documentation)

- [Project dependencies](https://mise.jdx.dev/dev-tools/deps.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/deps.md)

Last updated:

Pager

[Previous pagemise deactivate](https://mise.jdx.dev/cli/deactivate.html)

[Next pagemise deps add](https://mise.jdx.dev/cli/deps/add.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)