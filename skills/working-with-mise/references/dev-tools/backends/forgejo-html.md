forgejo backend [​](#forgejo-backend)

The `forgejo` backend installs a tool from the release assets of a Forgejo repository. It uses [Codeberg](https://codeberg.org) unless you set `api_url` for another Forgejo or Gitea server. It shares its implementation with the [github backend](https://mise.jdx.dev/dev-tools/backends/github.html) and accepts the same [tool options](https://mise.jdx.dev/dev-tools/backends/github.html#tool-options); this page covers what is different.

## Usage [​](#usage)

Forgejo Runner publishes Linux binaries on code.forgejo.org, not Codeberg, so this example sets `api_url`. On Linux, install it in the current project and run it:

sh

```
mise use 'forgejo:forgejo/runner[api_url=https://code.forgejo.org/api/v1,bin=forgejo-runner]'
mise exec -- forgejo-runner --version
```

Quote the argument so the shell does not expand the brackets. This adds the following entry to `mise.toml`. Add `-g` to `mise use` for a global tool.

```
[tools]
"forgejo:forgejo/runner" = { version = "latest", api_url = "https://code.forgejo.org/api/v1", bin = "forgejo-runner" }
```

This installs the runner executable; registering it with a server and running it as a service are separate steps. For a repository on Codeberg, leave out `api_url`.

## Differences from the github backend [​](#differences-from-the-github-backend)

| Behavior | github | forgejo |
| --- | --- | --- |
| Default `api_url` | `https://api.github.com` | `https://codeberg.org/api/v1` |
| Checksums found automatically | The digest GitHub reports, or a checksum file in the release | A checksum file in the release |
| GitHub artifact attestations and SLSA provenance | Checked | Not available; pin artifacts with `checksum` or [mise.lock](https://mise.jdx.dev/dev-tools/mise-lock.html) |

Everything else works as on GitHub. Version listing reads the same number of releases, and [`prerelease`](https://mise.jdx.dev/dev-tools/backends/github.html#prerelease) uses Forgejo's prerelease flag: `latest` resolves to the release Forgejo marks as latest unless `prerelease = true` is set.

## Authentication [​](#authentication)

Public repositories need no token. For a private repository, another server or rate limits, set `MISE_FORGEJO_TOKEN` or another token source; see [GitHub, GitLab, and Forgejo tokens](https://mise.jdx.dev/dev-tools/github-tokens.html). Run `mise token forgejo` to see which token mise uses.

## Other Forgejo servers [​](#other-forgejo-servers)

Set `api_url` to the server's API and provide a token for that host, for example with `MISE_FORGEJO_ENTERPRISE_TOKEN`; see [self-managed hosts](https://mise.jdx.dev/dev-tools/github-tokens.html#github-enterprise). mise downloads release assets through that server.

### `api_url` [​](#api-url)

The base URL of the Forgejo API. It defaults to `https://codeberg.org/api/v1`.

```
[tools]
"forgejo:user/repo" = { version = "latest", api_url = "https://forgejo.mycompany.com/api/v1" }
```

## Tool options [​](#tool-options)

The `forgejo` backend accepts the [github backend's tool options](https://mise.jdx.dev/dev-tools/backends/github.html#tool-options) for selecting assets, choosing versions, extracting files and verifying downloads, with the differences listed above. The GitHub-only options `github_attestations`, `slsa_signer_identity` and `slsa_signer_issuer` have no effect here.

## Settings [​](#settings)

The [`forgejo` settings](https://mise.jdx.dev/configuration/settings.html#forgejo) choose the token mise sends; see [GitHub, GitLab, and Forgejo tokens](https://mise.jdx.dev/dev-tools/github-tokens.html).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/dev-tools/backends/forgejo.md)

Last updated:

Pager

[Previous pagedotnet](https://mise.jdx.dev/dev-tools/backends/dotnet.html)

[Next pagegem](https://mise.jdx.dev/dev-tools/backends/gem.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)