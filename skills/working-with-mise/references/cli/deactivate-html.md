deactivate` [​](#mise-deactivate)

- **Usage:** `mise deactivate`
- **Effect:** read-only
- **Source code:** [`src/cli/deactivate.rs`](https://github.com/jdx/mise/blob/main/src/cli/deactivate.rs)

Print the script to disable mise in the current shell session

In a shell where mise is activated, run `mise deactivate`: the activation function evaluates the script for you. Calling the executable directly (`command mise deactivate`) only prints the script, so evaluate it yourself as in the examples below. New shells still activate mise; remove the line from your shell startup file to stop that.

## Flags [​](#flags)

- **`-h --help`** — Print help

## Examples [​](#examples)

Turn mise off in an activated shell

```
mise deactivate
```

Bash or Zsh, calling the executable rather than the activation function

```
eval "$(command mise deactivate)"
```

Fish, calling the executable rather than the activation function

```
command mise deactivate | source
```

## Related documentation [​](#related-documentation)

- [Shell setup](https://mise.jdx.dev/shell-setup.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/deactivate.md)

Last updated:

Pager

[Previous pagemise daemons urls](https://mise.jdx.dev/cli/daemons/urls.html)

[Next pagemise deps](https://mise.jdx.dev/cli/deps.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)