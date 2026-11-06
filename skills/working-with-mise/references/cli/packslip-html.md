packslip` [​](#mise-packslip)

- **Usage:** `mise packslip [SUBCOMMAND]`
- **Effect:** read-only
- **Source code:** [`src/cli/packslip/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/packslip/mod.rs)

Inspect and reset the signers mise has pinned for packslip tools

mise verifies a tool installed with the `packslip:` backend against the identity its project name implies, then remembers the signer it accepted, the way SSH remembers host keys. A later release from another signer, a weaker signing scheme, a repackager where the vendor signed before, or one that drops build provenance is refused. After you confirm the change with the publisher, run `mise packslip forget <PROJECT>`, and update mise.lock if it records the old signer.

With no subcommand, lists the pins (same as `mise packslip pins`). See <https://mise.jdx.dev/dev-tools/backends/packslip.html#pinned-signers>

## Flags [​](#flags)

- **`-h --help`** — Print help

## Subcommands [​](#subcommands)

- [`mise packslip forget <PROJECT>`](https://mise.jdx.dev/cli/packslip/forget.html)
- [`mise packslip pins [-J --json]`](https://mise.jdx.dev/cli/packslip/pins.html)

## Related documentation [​](#related-documentation)

- [Signer verification](https://mise.jdx.dev/dev-tools/packslip-verification.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/packslip.md)

Last updated:

Pager

[Previous pagemise outdated](https://mise.jdx.dev/cli/outdated.html)

[Next pagemise packslip forget](https://mise.jdx.dev/cli/packslip/forget.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)