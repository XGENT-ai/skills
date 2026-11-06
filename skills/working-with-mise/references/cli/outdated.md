outdated` [​](#mise-outdated)

- **Usage:** `mise outdated [FLAGS] [TOOL@VERSION]…`
- **Effect:** read-only
- **Source code:** [`src/cli/outdated.rs`](https://github.com/jdx/mise/blob/main/src/cli/outdated.rs)

Show outdated tool versions

Lists tools whose installed version is older than the newest version their config request allows. For `node = "20"`, that is the newest 20.x release; pass `--bump` to compare against the newest release overall instead. Run `mise upgrade` to install the newer versions.

## Arguments [​](#arguments)

- **`[TOOL@VERSION]…`** — Tools to check, such as `node@20 python@3.10`

  Checks every tool in the global and project configs when omitted.

## Flags [​](#flags)

- **`-b --bump`** — Compare against the newest release overall, not only the configured range

  With `node = "20"` in your config, `mise outdated` reports the newest 20.x release. With this flag the `latest` column shows the newest release overall, such as 24.1.0, and a `bump` column shows the request `mise upgrade --bump` would write, such as `24`.
- **`-J --json`** — Output in JSON format
- **`--inactive`** — Also check installed tools that the current config does not request

  By default, `mise outdated` checks only tools that come from the current config.
- **`--global`** — Only check tools defined in the global config

  Reports the requests in the global and system config (\~/.config/mise/config.toml, /etc/mise/config.toml), even where a project config or a `MISE_<TOOL>_VERSION` environment variable sets its own version of the tool: what `mise upgrade --global` would install.
- **`--local`** — Deprecated alias for --no-global

  Removed in mise 2027.7.0.
- **`--no-global`** — Only check tools defined in project config files

  Skips tools defined in the global config (\~/.config/mise/config.toml) and tools set through `MISE_<TOOL>_VERSION` environment variables.
- **`--no-header`** — Do not print the table header
- **`-h --help`** — Print help

## Examples [​](#examples)

Show tools with a newer version inside their configured range

```
mise outdated
name    requested  current  latest  source
node    20         20.0.0   20.1.0  ~/src/app/mise.toml
python  3.11       3.11.0   3.11.1  ~/src/app/mise.toml
```

Compare against the newest release overall and show the request for it

```
mise outdated --bump
name  requested  current  bump  latest  source
node  20         20.0.0   24    24.1.0  ~/src/app/mise.toml
```

Print the same information as JSON

```
mise outdated --json
{
  "node": {
    "name": "node",
    "requested": "20",
    "current": "20.0.0",
    "bump": null,
    "latest": "20.1.0",
    "source": {
      "type": "mise.toml",
      "path": "/home/me/src/app/mise.toml"
    }
  }
}
```

Skip tools that only the global config requests

```
mise outdated --no-global
```

Check the global config's request for claude, even inside a project that sets its own

```
mise outdated --global claude --json
```

Deprecation:

The `-l` shorthand for `--bump` is deprecated and will be removed in mise 2027.8.5. Use `-b` or `--bump` instead.

The `--local` flag is deprecated in favor of `--no-global`. mise warns about it from 2027.1.0 and removes it in 2027.7.0.

## Related documentation [​](#related-documentation)

- [Upgrading tools](https://mise.jdx.dev/dev-tools/#upgrade-tools).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/outdated.md)

Last updated:

Pager

[Previous pagemise oci run](https://mise.jdx.dev/cli/oci/run.html)

[Next pagemise packslip](https://mise.jdx.dev/cli/packslip.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)