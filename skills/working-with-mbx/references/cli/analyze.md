`mbx analyze` [​](#mbx-analyze)

- **Usage:** `mbx analyze`

Rank the uncached compiler time of this workspace's last build by cause.

Each cause comes with what would remove it. mbx reads the newest recorded build of the Cargo workspace that contains the current directory, so it runs nothing. A crate that rebuilt only because a dependency changed is counted under the change that started it. The report ends with the critical path (the chain of units the build waited on) and the time only one unit was running.

## Flags [​](#flags)

- **`-h --help`** — Print help

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/crates/mbx/src/cli/analyze.rs)

Last updated:

Pager

[Previous pageexplain](https://mr-boxington.jdx.dev/cli/explain)

[Next pageclean](https://mr-boxington.jdx.dev/cli/clean)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)