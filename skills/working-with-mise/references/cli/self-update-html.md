self-update` [​](#mise-self-update)

- **Usage:** `mise self-update [FLAGS] [VERSION]`
- **Effect:** modifies state
- **Source code:** [`src/cli/self_update.rs`](https://github.com/jdx/mise/blob/main/src/cli/self_update.rs)

Update mise itself

Installs the newest stable release, or VERSION, from GitHub Releases and verifies its signature before replacing the running binary. Without VERSION, releases younger than the minimum release age are skipped: the `self_update.minimum_release_age` setting, then `minimum_release_age`, else 24h. Installed plugins are updated too unless you pass `--no-plugins`. Set `self_update.auto` to update automatically.

If a package manager such as Homebrew or apt installed mise, update it with that package manager instead: <https://mise.jdx.dev/installing-mise.html#updating> Packagers can disable this command: <https://mise.jdx.dev/packaging.html>

## Arguments [​](#arguments)

- **`[VERSION]`** — Update to a specific version, skipping the minimum release age

## Flags [​](#flags)

- **`--minimum-release-age <MINIMUM_RELEASE_AGE>`** — Minimum release age for this update, such as 7d

  Defaults to the self\_update.minimum\_release\_age setting, then the minimum\_release\_age setting, else 24h.
- **`-f --force`** — Update even if already up to date

  Also updates when the package that installed mise has disabled self-update.
- **`-y --yes`** — Skip confirmation prompt
- **`--no-plugins`** — Do not update installed plugins
- **`-h --help`** — Print help

## Examples [​](#examples)

Update to the newest eligible release

```
mise self-update
```

Install a specific release

```
mise self-update 2026.10.4
```

Update without prompting or touching plugins

```
mise self-update -y --no-plugins
```

## Related documentation [​](#related-documentation)

- [Updating mise](https://mise.jdx.dev/installing-mise.html#updating).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/self-update.md)

Last updated:

Pager

[Previous pagemise secrets ls](https://mise.jdx.dev/cli/secrets/ls.html)

[Next pagemise set](https://mise.jdx.dev/cli/set.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)