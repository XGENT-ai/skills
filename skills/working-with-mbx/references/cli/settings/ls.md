`mbx settings ls` [​](#mbx-settings-ls)

- **Usage:** `mbx settings ls [KEY]`

List settings and their current values.

Values come from the environment, the global configuration file, and defaults. Workspace `.mbx.toml` settings are not included. If a setting whose default mbx computes when it runs has no value, `ls` shows it as unset, followed by a description of that default. The value of `remote.token` is not printed; `mbx settings get remote.token` prints it.

## Arguments [​](#arguments)

- **`[KEY]`** — Setting or group to list, such as `gc` or `gc.max_size`.

  Lists every setting when omitted.

## Flags [​](#flags)

- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/settings.rs)

Last updated:

Pager

[Previous pagesettings](https://mr-boxington.jdx.dev/cli/settings/)

[Next pageget](https://mr-boxington.jdx.dev/cli/settings/get)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)