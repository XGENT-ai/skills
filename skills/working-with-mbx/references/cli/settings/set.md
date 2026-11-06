`mbx settings set` [​](#mbx-settings-set)

- **Usage:** `mbx settings set <KEY> <VALUE>`

Write a setting to the global configuration file.

mbx writes nothing unless the value has the setting's type, is one of its allowed values, and loads as that setting. List settings take comma-separated items, such as `mbx settings set target.keep ~/src,/work`. Edit table settings, such as `linker.profiles`, in the file directly. Settings read only from the environment, such as `verify`, cannot be set this way. Comments and formatting elsewhere in the file are kept. An environment variable for the same setting still takes precedence.

## Arguments [​](#arguments)

- **`<KEY>`** — Setting name, such as `gc.max_size`.
- **`<VALUE>`** — New value.

## Flags [​](#flags)

- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/settings.rs)

Last updated:

Pager

[Previous pageget](https://mr-boxington.jdx.dev/cli/settings/get)

[Next pageunset](https://mr-boxington.jdx.dev/cli/settings/unset)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)