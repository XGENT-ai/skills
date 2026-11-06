asdf backend (legacy) [​](#asdf-backend-legacy)

The `asdf` backend installs tools with [asdf](https://asdf-vm.com/) plugins: Git repositories of shell scripts that list a tool's versions and install them. Use it when an existing plugin does something no other backend can. For a new plugin, write a [tool plugin](https://mise.jdx.dev/tool-plugin-development.html) instead.

Legacy

asdf plugins are shell scripts that run with your permissions whenever mise lists versions, installs or activates the tool. Read a plugin before you use it, and prefer the [backends](https://mise.jdx.dev/dev-tools/backends/#which-backend-to-use) that install a publisher's release directly.

## Requirements [​](#requirements)

asdf plugins need Bash and the Unix tools their scripts call, often `curl`, `git` and `tar`. mise does not use the asdf backend on Windows: it skips `asdf:` tools there, and a registry short name moves on to its next backend. On Windows, use a native backend or a [vfox plugin](https://mise.jdx.dev/dev-tools/backends/vfox.html).

## Usage [​](#usage)

Install the AWS CLI through its asdf plugin and run it:

sh

```
mise use asdf:MetricMike/asdf-awscli@2
mise exec -- aws --version
```

This writes the following to `mise.toml`:

```
[tools]
"asdf:MetricMike/asdf-awscli" = "2"
```

The part after `asdf:` is a GitHub `owner/repo` or a full Git URL, such as `asdf:https://gitlab.com/wt0f/asdf-ripgrep`. mise clones the plugin the first time it needs it. `mise registry <tool>` shows whether a registry short name still falls back to an asdf plugin.

To use a plugin under a short name of your own, install it with [`mise plugins install`](https://mise.jdx.dev/cli/plugins/install.html) or list it under `[plugins]`; see [asdf plugins (legacy)](https://mise.jdx.dev/asdf-legacy-plugins.html). That page also covers writing and maintaining asdf plugins and porting them to Lua.

## Tool options [​](#tool-options)

Set these on the tool's entry in `[tools]`. Options every backend accepts are described under [tool options](https://mise.jdx.dev/dev-tools/#tool-options).

### `install_env` [​](#install-env)

Set environment variables for the plugin's install scripts:

```
[tools]
"asdf:owner/asdf-tool" = { version = "latest", install_env = { MAKEFLAGS = "-j8" } }
```

### `depends` [​](#depends)

Tools listed in `depends` are installed first, and their executables come before the rest of `PATH` for the plugin's `bin/download` and `bin/install` scripts:

```
[tools]
python = "3.14"
"asdf:owner/asdf-tool" = { version = "latest", depends = ["python"] }
```

Other mise tools are not added to that `PATH`, so list every mise tool the scripts run. `depends` does not add a tool to your config: the dependency also needs its own entry in `[tools]`. See [tool dependencies](https://mise.jdx.dev/dev-tools/#tool-dependencies).

### Plugin options [​](#plugin-options)

Any other key in the tool's entry reaches the plugin's scripts as an environment variable named `MISE_TOOL_OPTS__` followed by the key in uppercase. For example, `mirror = "eu"` becomes `MISE_TOOL_OPTS__MIRROR=eu`. The plugin's README lists the options it reads.

## Troubleshooting [​](#troubleshooting)

| Symptom | What to do |
| --- | --- |
| An `asdf:` tool is skipped on Windows | mise does not run asdf plugins on Windows. Use a native backend or a [vfox plugin](https://mise.jdx.dev/dev-tools/backends/vfox.html). |
| A plugin script fails | Run `MISE_DEBUG=1 mise install <tool>` for more detail, then install the commands the script calls. |
| `backend asdf is disabled by disable_backends` | Remove `asdf` from [`disable_backends`](https://mise.jdx.dev/dev-tools/backends/#disable-backends). |

Implementation: [`src/backend/asdf.rs`](https://github.com/jdx/mise/blob/main/src/backend/asdf.rs).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/dev-tools/backends/asdf.md)

Last updated:

Pager

[Previous pageaqua](https://mise.jdx.dev/dev-tools/backends/aqua.html)

[Next pagecargo](https://mise.jdx.dev/dev-tools/backends/cargo.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)