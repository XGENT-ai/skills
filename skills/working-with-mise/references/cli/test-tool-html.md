test-tool` [​](#mise-test-tool)

- **Usage:** `mise test-tool [FLAGS] [TOOLS]…`
- **Source code:** [`src/cli/test_tool.rs`](https://github.com/jdx/mise/blob/main/src/cli/test_tool.rs)

Test that a tool installs and runs

Installs each tool and runs the `test` command from its registry entry (registry/\<tool>.toml), checking the output. Use it when adding or changing a registry entry. The minimum release age is ignored so that new releases can be tested right away.

Warning: before testing, this deletes every installed version of each tested tool, along with its cache and downloads directories, so the test installs from scratch. To keep your installs and caches, run it with throwaway MISE\_DATA\_DIR and MISE\_CACHE\_DIR directories.

Registry tools with no `test` entry, or that registry/ marks as unsupported on this OS, are skipped without output, and the command still exits 0. Pass --include-non-defined to run `<tool> --version` for tools with no `test` entry.

## Arguments [​](#arguments)

- **`[TOOLS]…`** — Tools to test

## Flags [​](#flags)

- **`-a --all`** — Test every registry tool that defines a `test` and supports this OS
- **`-j --jobs <JOBS>`** — Number of tools to test in parallel (default 4)

  **Environment Variable:** `MISE_TEST_TOOL_JOBS`
- **`--all-config`** — Test all tools specified in config files
- **`--include-non-defined`** — Also test registry tools that have no `test` entry by running `<tool> --version`

  These always report a failure that shows the output, for use when writing a test.
- **`--raw`** — Connect the install commands' stdin, stdout, and stderr to the terminal

  Implies `--jobs=1`.
- **`-h --help`** — Print help

## Examples [​](#examples)

Reinstall ripgrep and run its registry test

```
mise test-tool ripgrep
```

Test every tool in the config without touching your installs

```
MISE_DATA_DIR=$(mktemp -d) MISE_CACHE_DIR=$(mktemp -d) mise test-tool --all-config
```

## Related documentation [​](#related-documentation)

- [Testing registry tools](https://mise.jdx.dev/contributing/registry.html#tool-testing).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/test-tool.md)

Last updated:

Pager

[Previous pagemise tasks validate](https://mise.jdx.dev/cli/tasks/validate.html)

[Next pagemise token](https://mise.jdx.dev/cli/token.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)