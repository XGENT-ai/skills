en` [​](#mise-en)

- **Usage:** `mise en [-s --shell <SHELL>] [DIR]`
- **Source code:** [`src/cli/en.rs`](https://github.com/jdx/mise/blob/main/src/cli/en.rs)

Start a subshell with mise's tools and environment

This is an alternative to `mise activate` for starting a mise session explicitly. The new shell has the tools and environment variables from the config loaded. Unlike an activated shell, changing directories does not update the environment. Type `exit` to leave it.

## Arguments [​](#arguments)

- **`[DIR]`** — Directory to start the shell in

  **Default:** `.`

## Flags [​](#flags)

- **`-s --shell <SHELL>`** — Shell to start; defaults to $SHELL
- **`-h --help`** — Print help

## Examples [​](#examples)

Start a shell with the project's tools and environment

```
mise en
```

Start the shell in another project

```
mise en ~/work/api
```

Skip loading .bashrc

```
mise en -s "bash --norc"
```

Skip loading .zshrc

```
mise en -s "zsh -f"
```

## Related documentation [​](#related-documentation)

- [Shell setup](https://mise.jdx.dev/shell-setup.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/en.md)

Last updated:

Pager

[Previous pagemise edit](https://mise.jdx.dev/cli/edit.html)

[Next pagemise env](https://mise.jdx.dev/cli/env.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)