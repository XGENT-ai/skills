`mbx settings unset` [​](#mbx-settings-unset)

- **Usage:** `mbx settings unset <KEY>`

Remove a setting from the global configuration file.

The setting then falls back to its default. An environment variable for the same setting still takes precedence. You can also unset a key mbx does not recognize, such as a misspelled one that stops mbx from loading, as long as the key holds a value rather than a table.

## Arguments [​](#arguments)

- **`<KEY>`** — Setting name, such as `gc.max_size`.

## Flags [​](#flags)

- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/settings.rs)

Last updated:

Pager

[Previous pageset](https://mr-boxington.jdx.dev/cli/settings/set)

[Next pagecompletion](https://mr-boxington.jdx.dev/cli/completion)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)