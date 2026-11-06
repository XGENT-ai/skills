go backend [​](#go-backend)

The `go` backend builds Go command-line packages with `go install`. Use the import path of the executable package, which may end in `/cmd/<name>` or include a major-version suffix such as `/v4`. Libraries belong in your application's `go.mod`.

## Requirements [​](#requirements)

Install Go. mise builds each tool with the Go version configured for the project, and installs `go` first when it is in your config. Some packages also need Git, a C compiler (for cgo), or native libraries.

## Usage [​](#usage)

Install Go and Hivemind in the current project:

sh

```
mise use go@1.26 go:github.com/DarthSim/hivemind
mise exec -- hivemind --help
```

This writes both tools to `mise.toml`. Add `-g` to `mise use` for your global config.

```
[tools]
go = "1.26"
"go:github.com/DarthSim/hivemind" = "latest"
```

Run `mise ls-remote go:github.com/DarthSim/hivemind` to list versions, and pin one with `mise use go:github.com/DarthSim/hivemind@1.1.0`. mise writes the executable into its own install directory, not your usual `GOBIN`.

### Pinned versions [​](#pinned-versions)

You can pin any module version, including an unreleased pseudo-version:

```
[tools]
"go:github.com/grafana/oats" = "v0.7.1-0.20260703092802-96201f1b8136"
```

To resolve an unreleased revision directly from version control instead of the module proxy, combine the pin with [`install_env`](https://mise.jdx.dev/dev-tools/backends/go.html#install-env):

```
[tools."go:github.com/grafana/oats"]
version = "v0.7.1-0.20260703092802-96201f1b8136"
install_env = { GOPROXY = "direct", GONOSUMDB = "github.com/grafana/oats" }
```

## Private modules [​](#private-modules)

Private modules use Go's normal Git authentication; mise's GitHub token does not replace it. Set `GOPRIVATE` in your environment or in mise's `[env]` so that mise asks Go for versions instead of querying the public proxy. mise does not read values set only with `go env -w`.

```
[env]
GOPRIVATE = "github.com/acme/*"
```

Go also uses `GOPRIVATE` as the default for `GONOPROXY` and `GONOSUMDB`. If you set those variables yourself, set each one for the proxy and checksum-database behavior you need.

## Release age [​](#release-age)

With [`minimum_release_age`](https://mise.jdx.dev/configuration/settings.html#minimum_release_age), mise checks each candidate version's release date before it picks one. For a private module with many releases, which mise reads through `go list`, the first resolution can be slow.

If reading a version's date fails, for example because the proxy is unreachable or a version-control host times out, mise warns and accepts that version. On a bad network, a version newer than your cutoff can therefore be installed; the warning tells you when that happened.

## Tool options [​](#tool-options)

Set these on the tool's entry in `[tools]`. Options every backend accepts are described under [tool options](https://mise.jdx.dev/dev-tools/#tool-options).

### `install_env` [​](#install-env)

Set environment variables for `go install`. mise still sets `GOBIN` to the tool's install directory.

```
[tools]
"go:github.com/DarthSim/hivemind" = { version = "latest", install_env = { CGO_ENABLED = "0" } }
```

`install_env` does not affect version listing, so put `GOPRIVATE` in `[env]` instead; see [private modules](#private-modules).

### `tags` [​](#tags)

Set Go build tags, passed as `go install -tags`. Give several as an array:

```
[tools]
"go:github.com/golang-migrate/migrate/v4/cmd/migrate" = { version = "latest", tags = ["postgres", "mysql"] }
```

A single tag can be a string: `tags = "postgres"`.

## Troubleshooting [​](#troubleshooting)

| Symptom | What to do |
| --- | --- |
| `go install` says it is not a main package | Use the import path of the executable package, not the repository root or a library package. |
| A private module cannot be found | Check that `GOPRIVATE` is set where mise can see it and that Git can authenticate to the host. |
| Compiler or Go version error | Use a Go version the package supports and install any native build dependencies. |

Implementation: [`src/backend/go.rs`](https://github.com/jdx/mise/blob/main/src/backend/go.rs).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/dev-tools/backends/go.md)

Last updated:

Pager

[Previous pagegitlab](https://mise.jdx.dev/dev-tools/backends/gitlab.html)

[Next pagehttp](https://mise.jdx.dev/dev-tools/backends/http.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)