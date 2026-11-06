dotfiles` [​](#mise-dotfiles)

- **Usage:** `mise dotfiles [--local] <SUBCOMMAND>`
- **Aliases:** `dot`
- **Effect:** read-only
- **Source code:** [`src/cli/dotfiles/mod.rs`](https://github.com/jdx/mise/blob/main/src/cli/dotfiles/mod.rs)

Deploy, track, and sync dotfiles

A `[dotfiles]` entry deploys a file (from a source with `symlink`, `symlink-each`, `copy`, or `template`, or from inline `content`), removes one (`mode = "absent"`), sets an existing path's `permissions`, edits part of a file, or tracks a live file where it is (`mode = "track"`). mise saves tracked files to a local Git history that you can browse and roll back, and syncs it with other machines through an origin repository. `mise dot` is a shorter alias.

Deploy files with `add`, `apply`, `diff`, `status`, and `unapply`. Track and restore files with `track`, `save`, `history`, `rollback`, and `undo`. Sync them with `origin`, `sync`, and `pull`.

See <https://mise.jdx.dev/dotfiles.html> and <https://mise.jdx.dev/dotfiles/history.html>

## Flags [​](#flags)

- **`--local`** — Work on this machine's local-only history ( `mode = "track-local"`), which is never shared
- **`-h --help`** — Print help

## Examples [​](#examples)

Start managing \~/.zshrc from your dotfiles directory

```
mise dot add ~/.zshrc
```

Deploy every \[dotfiles] entry

```
mise dot apply
```

Show the state of your dotfiles and their history

```
mise dot status
```

Save the history of a directory in place

```
mise dot track ~/.config/nvim
```

List saved checkpoints

```
mise dot history
```

## Subcommands [​](#subcommands)

- [`mise dotfiles add [FLAGS] [TARGET]…`](https://mise.jdx.dev/cli/dotfiles/add.html)
- [`mise dotfiles apply [FLAGS] [TARGET]…`](https://mise.jdx.dev/cli/dotfiles/apply.html)
- [`mise dotfiles capture [--label <LABEL>] <-- COMMAND>…`](https://mise.jdx.dev/cli/dotfiles/capture.html)
- [`mise dotfiles conflicts [--difftool] [--tool <TOOL>] [PATH]…`](https://mise.jdx.dev/cli/dotfiles/conflicts.html)
- [`mise dotfiles diff [--prompt-secrets] [TARGET]…`](https://mise.jdx.dev/cli/dotfiles/diff.html)
- [`mise dotfiles edit [FLAGS] <TARGET>`](https://mise.jdx.dev/cli/dotfiles/edit.html)
- [`mise dotfiles exclude <GLOB>`](https://mise.jdx.dev/cli/dotfiles/exclude.html)
- [`mise dotfiles history [FLAGS] [SUBCOMMAND]`](https://mise.jdx.dev/cli/dotfiles/history.html)
- [`mise dotfiles include <GLOB>`](https://mise.jdx.dev/cli/dotfiles/include.html)
- [`mise dotfiles notify`](https://mise.jdx.dev/cli/dotfiles/notify.html)
- [`mise dotfiles origin [--remove] [SUBCOMMAND]`](https://mise.jdx.dev/cli/dotfiles/origin.html)
- [`mise dotfiles paths [FLAGS]`](https://mise.jdx.dev/cli/dotfiles/paths.html)
- [`mise dotfiles pull [FLAGS]`](https://mise.jdx.dev/cli/dotfiles/pull.html)
- [`mise dotfiles recover [--keep-current] [-y --yes] [OPERATION]`](https://mise.jdx.dev/cli/dotfiles/recover.html)
- [`mise dotfiles rollback [FLAGS] [PATH]…`](https://mise.jdx.dev/cli/dotfiles/rollback.html)
- [`mise dotfiles save [FLAGS] [PATH]…`](https://mise.jdx.dev/cli/dotfiles/save.html)
- [`mise dotfiles status [FLAGS] [TARGET]…`](https://mise.jdx.dev/cli/dotfiles/status.html)
- [`mise dotfiles sync [FLAGS]`](https://mise.jdx.dev/cli/dotfiles/sync.html)
- [`mise dotfiles track [FLAGS] <PATH>…`](https://mise.jdx.dev/cli/dotfiles/track.html)
- [`mise dotfiles unapply [FLAGS] [TARGET]…`](https://mise.jdx.dev/cli/dotfiles/unapply.html)
- [`mise dotfiles undo [-n --dry-run] [-y --yes] [REF]`](https://mise.jdx.dev/cli/dotfiles/undo.html)
- [`mise dotfiles untrack <PATH>…`](https://mise.jdx.dev/cli/dotfiles/untrack.html)
- [`mise dotfiles watch [--once] [-J --json]`](https://mise.jdx.dev/cli/dotfiles/watch.html)

## Related documentation [​](#related-documentation)

- [Dotfiles](https://mise.jdx.dev/dotfiles.html).
- [All commands](https://mise.jdx.dev/cli/).
- [Global flags and argument syntax](https://mise.jdx.dev/cli/#global-flags).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/cli/dotfiles.md)

Last updated:

Pager

[Previous pagemise doctor project](https://mise.jdx.dev/cli/doctor/project.html)

[Next pagemise dotfiles add](https://mise.jdx.dev/cli/dotfiles/add.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)