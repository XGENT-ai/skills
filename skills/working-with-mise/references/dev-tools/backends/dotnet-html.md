dotnet backend [​](#dotnet-backend)

The `dotnet` backend installs .NET command-line tool packages from [NuGet](https://www.nuget.org/) with `dotnet tool install`. The unprefixed `dotnet` tool installs the SDK itself; see the [.NET guide](https://mise.jdx.dev/lang/dotnet.html) for SDK versions and `global.json`.

## Requirements [​](#requirements)

Install a .NET SDK, plus the runtime the tool package targets. A newer SDK does not guarantee that an older tool runs, because .NET's runtime selection rules still apply. Run `mise exec -- dotnet --list-runtimes` to see what is installed. When `dotnet` is in your config, mise installs it before your .NET tools.

## Usage [​](#usage)

Install .NET 8 and a GitVersion release built for it:

sh

```
mise use dotnet@8 dotnet:GitVersion.Tool@6.0.5
mise exec -- dotnet-gitversion /version
```

This writes both entries to `mise.toml`. Add `-g` to `mise use` for your global config.

```
[tools]
dotnet = "8"
"dotnet:GitVersion.Tool" = "6.0.5"
```

To choose another release, run `mise ls-remote dotnet:GitVersion.Tool` and check that release's runtime requirements. `mise use dotnet:GitVersion.Tool` without a version records `latest`.

mise installs each tool into its own directory with `--tool-path`. It does not create or update a project's `.config/dotnet-tools.json` manifest.

## Private feeds [​](#private-feeds)

mise lists versions from the NuGet service index in [`dotnet.registry_url`](https://mise.jdx.dev/configuration/settings.html#dotnet.registry_url), but `dotnet tool install` downloads the package using its own `NuGet.Config` and credentials. For a private feed, add it to `NuGet.Config` and point `dotnet.registry_url` at its service index; changing the setting alone does not add an install source.

The two steps authenticate separately. mise's version listing sends [`netrc`](https://mise.jdx.dev/configuration/settings.html#netrc) credentials to a matching host with HTTP Basic authentication, so a password-protected Nexus or Artifactory feed needs a `~/.netrc` entry for it:

text

```
machine nexus.example.com
  login myuser
  password mytoken
```

`dotnet tool install` does not read netrc. Give the feed's credentials to NuGet in `NuGet.Config`, either as a `packageSourceCredentials` entry or with `dotnet nuget add source`.

## Tool options [​](#tool-options)

Set these on the tool's entry in `[tools]`, or inline, as in `'dotnet:GitVersion.Tool[prerelease=true]'`. Options every backend accepts are described under [tool options](https://mise.jdx.dev/dev-tools/#tool-options).

### `install_env` [​](#install-env)

Set environment variables for `dotnet tool install`:

```
[tools]
"dotnet:GitVersion.Tool" = { version = "latest", install_env = { DOTNET_NOLOGO = "1" } }
```

### `prerelease` [​](#prerelease)

By default, NuGet prerelease versions are left out of `mise ls-remote` and `latest`. Set `prerelease = true` to include them:

```
[tools]
"dotnet:GitVersion.Tool" = { version = "latest", prerelease = true }
```

To include prereleases for every tool, set the [`prereleases`](https://mise.jdx.dev/configuration/settings.html#prereleases) setting instead.

## Settings [​](#settings)

Two settings apply to `dotnet:` tools. The other `dotnet.*` settings configure the .NET SDK; see the [.NET guide](https://mise.jdx.dev/lang/dotnet.html#settings).

Set these under `[settings]` in a config file, with [`mise settings set`](https://mise.jdx.dev/cli/settings/set.html), or with a setting's environment variable where it has one. See [which value wins](https://mise.jdx.dev/configuration/settings.html#which-value-wins) when a setting is set in more than one place.

### `dotnet.package_flags`[Permalink to dotnet.package\_flags](#dotnet.package_flags)deprecated

Flags for every `dotnet:` tool; the only flag, `prerelease`, includes prereleases.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_DOTNET_PACKAGE_FLAGS` (comma-separated)
- Deprecated: Use the `prerelease = true` tool option or the global `prereleases` setting instead.

Because it applies to every tool, `prerelease` here also overrides a tool's own `prerelease = false`.

### `dotnet.registry_url`[Permalink to dotnet.registry\_url](#dotnet.registry_url)

NuGet service index used to discover dotnet tool versions.

- Type: `URL`
- Default: `"https://api.nuget.org/v3/index.json"`
- Environment variable: `MISE_DOTNET_REGISTRY_URL`

Installation runs `dotnet tool install`, which reads its own `NuGet.Config` and credentials. Configure a private feed there as well; this setting does not add an installation source to the dotnet CLI.

mise 2026.11.0 and later warn when `dotnet.package_flags` is set, and mise 2027.11.0 removes it.

## Troubleshooting [​](#troubleshooting)

| Symptom | What to check |
| --- | --- |
| SDK not found | Run `mise exec -- dotnet --info` and check any `global.json` that constrains SDK selection. |
| Required framework missing | Install a runtime or SDK the tool supports, or choose a tool release that targets the runtime you have. |
| Package not found | Check that the package is a .NET tool and that both version listing and installation can reach its feed. |

Implementation: [`src/backend/dotnet.rs`](https://github.com/jdx/mise/blob/main/src/backend/dotnet.rs).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/dev-tools/backends/dotnet.md)

Last updated:

Pager

[Previous pageconda](https://mise.jdx.dev/dev-tools/backends/conda.html)

[Next pageforgejo](https://mise.jdx.dev/dev-tools/backends/forgejo.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)