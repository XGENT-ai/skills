Self-updating tools [​](#self-updating-tools)

This page is for authors of tools that update themselves, such as coding agents, editors, and long-running CLIs. When mise installed your tool, update it through mise instead of your own updater.

A built-in updater doesn't know about mise's install directory. It either writes the new version over the files in `installs/<tool>/<version>/`, so the directory names one version and holds another, or installs a second copy elsewhere that mise doesn't manage. mise then reports the old version, offers an update that is already installed, and can't verify the files against the lockfile. Updating through mise avoids all of that, and the update follows the user's version request, [`minimum_release_age`](https://mise.jdx.dev/security.html#minimum-release-age), and `locked` setting.

Users who want mise to update tools on its own can set [`auto_update`](https://mise.jdx.dev/dev-tools/#automatic-tool-updates) instead. The steps below work with or without it.

## Detect a mise install [​](#detect-a-mise-install)

Resolve the real path of your executable (for a script, of the script, not of its interpreter) and walk up its parent directories. mise installed it if one of them contains either file:

| File | Layout | Tool name |
| --- | --- | --- |
| `.mise.backend.toml` | `installs/<tool>/<version>/`; the file is in `installs/<tool>/` | the `short` key |
| `.mise-install.toml` | the [identity layout](https://mise.jdx.dev/dev-tools/install-layout.html) | the `requested_as` key |

Pass that tool name to the commands below. Don't detect mise from environment variables: child processes inherit them, so a tool started by yours would think mise installed it too.

## Check for an update [​](#check-for-an-update)

sh

```
mise outdated --global claude --json
```

json

```
{
  "claude": {
    "name": "claude",
    "requested": "latest",
    "current": "2.1.3",
    "bump": null,
    "latest": "2.1.5",
    "source": {
      "type": "mise.toml",
      "path": "/home/me/.config/mise/config.toml"
    }
  }
}
```

An entry for the tool means a newer version is available within the global config's request; `{}` means it is up to date, or that the global config doesn't list the tool. `--global` checks the request in the user's global config even inside a project that sets its own version, which is the request `mise upgrade --global` moves. Show your own "update available" notice from this answer rather than from your release server, so it agrees with what mise will install.

## Install the update [​](#install-the-update)

sh

```
mise upgrade --global claude
```

This installs the newest version the request in the global or system config allows, into the system installs directory if the current version was [installed there](https://mise.jdx.dev/dev-tools/system-installs.html), and moves the global lockfile entry if one pins the tool. It runs as if from outside any project, so a project's config, lockfile, and `[env]` are neither read nor changed. Progress goes to stderr and a summary to stdout, and the command exits non-zero if the update fails. Run it non-interactively, with stdin closed. Updates run one at a time; one started from a hook of another mise update skips with a warning.

The version you upgraded from stays installed while any process still runs from it, and for [`upgrade.prune_after`](https://mise.jdx.dev/configuration/settings.html#upgrade.prune_after) after that, so running sessions keep working.

## Switch a running process to the new version [​](#switch-a-running-process-to-the-new-version)

A process can't change its own code while it runs, but it can replace itself. To move a long-running session onto the new version:

1. Find the executable mise runs in the session's directory: `mise which claude`. If it is the one you are running from, stop here. This is also how a project that pins its own version shows up: the global update doesn't change what runs there.
2. Wait for a safe point, such as between turns with no tool call running, and save the session.
3. Replace the process with `mise x -- claude <resume arguments>`, started in the session's working directory. On Unix, `exec` it so the process ID and terminal stay the same. Windows has no `exec`: start it as a child and exit.

Start the new version through `mise x`, not through your own path, `argv[0]`, or a `PATH` lookup. Those all lead back to the old version: with [shell activation](https://mise.jdx.dev/shell-setup.html), the `PATH` your process inherited names the old version's directory. `mise x` resolves the version again, and the processes the new session starts (MCP servers, hooks, subagents) get the new `PATH`.

Guard against loops: if the version you land on is the one you left, don't switch again.

## Summary [​](#summary)

- Don't write into mise's install directories, and don't install a second copy of the tool.
- Check with `mise outdated --global <tool> --json`, and update with `mise upgrade --global <tool>`.
- Don't bypass `minimum_release_age`: it is the user's security policy.
- Switch sessions with `mise x -- <tool> <resume arguments>` at a safe point.

[Edit this page](https://github.com/jdx/mise/edit/main/docs/dev-tools/self-updating-tools.md)

Last updated:

Pager

[Previous pageTool stubs](https://mise.jdx.dev/dev-tools/tool-stubs.html)

[Next pageLockfile (mise.lock)](https://mise.jdx.dev/dev-tools/mise-lock.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)