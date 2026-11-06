untrust` [​](#mise-untrust)

- **Usage:** `mise untrust [CONFIG_FILE]`
- **Effect:** modifies state
- **Source code:** [`src/cli/untrust.rs`](https://github.com/jdx/mise/blob/main/src/cli/untrust.rs)

Remove explicit trust for a config

mise asks again before loading the parts of the file that can run code. With no file, untrusts the nearest config in this directory or a parent. Same as `mise trust --untrust`.

## Arguments [​](#arguments)

- **`[CONFIG_FILE]`** — The config file to untrust

## Flags [​](#flags)

- **`-h --help`** — Print help

## Examples [​](#examples)

Stop trusting a specific file

```
mise untrust ~/src/app/mise.toml
```

## Related documentation [​](#related-documentation)

- [Configuration trust](https://mise.jdx.dev/security.html#configuration-trust).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/untrust.md)

Last updated:

Pager

[Previous pagemise unset](https://mise.jdx.dev/cli/unset.html)

[Next pagemise unuse](https://mise.jdx.dev/cli/unuse.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)