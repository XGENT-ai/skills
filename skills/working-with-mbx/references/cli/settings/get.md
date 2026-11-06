`mbx settings get` [​](#mbx-settings-get)

- **Usage:** `mbx settings get <KEY>`

Print the current value of one setting.

The value comes from the environment, the global configuration file, or the setting's fixed default. Workspace `.mbx.toml` settings are not included. A setting with neither a value nor a fixed default prints nothing. That includes a setting whose default mbx computes when it runs, such as `gc.max_size`; `mbx settings ls` describes those defaults.

## Arguments [​](#arguments)

- **`<KEY>`** — Setting name, such as `gc.max_size`.

## Flags [​](#flags)

- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/settings.rs)

Last updated:

Pager

[Previous pagels](https://mr-boxington.jdx.dev/cli/settings/ls)

[Next pageset](https://mr-boxington.jdx.dev/cli/settings/set)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)