cargo backend [​](#cargo-backend)

The `cargo` backend installs Rust command-line tools from [crates.io](https://crates.io/) or a Git repository. It downloads a prebuilt binary when cargo-binstall can find one and builds the crate with Cargo otherwise. It is for executables; add library crates to your project with `cargo add`.

## Requirements [​](#requirements)

Install Rust so mise can run `cargo install`. To skip compiling for crates that publish prebuilt binaries, add cargo-binstall too:

sh

```
mise use -g rust cargo-binstall
```

When `rust`, `cargo-binstall` or `sccache` is in your config, mise installs it before your Cargo tools. Source builds also need a linker and any native libraries the crate uses.

## Usage [​](#usage)

Declare Rust and eza together in the current project:

sh

```
mise use rust@stable cargo:eza
mise exec -- eza --version
```

This records both tools in `mise.toml`. Add `-g` to `mise use` for your global config.

```
[tools]
rust = "stable"
"cargo:eza" = "latest"
```

Run `mise ls-remote cargo:eza` to list releases, and pin one with `mise use cargo:eza@0.23.5`.

### Install from Git [​](#using-git)

Give a Git repository URL, or `owner/repo` for GitHub, and pick a tag, branch or commit as the version. Quote the argument:

sh

```
mise use 'cargo:https://github.com/eza-community/eza@tag:v0.23.5'
mise use 'cargo:eza-community/eza@branch:main'
mise use 'cargo:eza-community/eza@rev:<commit>'
```

mise runs `cargo install --git` with the matching `--tag`, `--branch` or `--rev`. A Git source lists only `HEAD` as a version, so `latest`, or no version at all, installs the default branch.

For a private repository, keep the credentials in git's credential helper rather than in the URL. mise records the tool without URL credentials, but cargo writes the `--git` URL it was given, credentials included, to `.crates.toml` in the install directory.

## How mise installs a crate [​](#how-mise-installs-a-crate)

mise uses the first method that applies:

1. A Git source always builds with `cargo install --git`.
2. The `features` option, or `default-features = false`, needs a source build, so mise runs `cargo install`.
3. If [`cargo.binstall`](https://mise.jdx.dev/dev-tools/backends/cargo.html#cargo.binstall) is on (the default) and `cargo-binstall` is installed through mise or on `PATH`, mise runs cargo-binstall with its compile strategy turned off. When cargo-binstall reports that no prebuilt binary exists (exit code 94), mise runs `cargo install` instead. Other cargo-binstall errors fail the install.
4. If cargo-binstall is not installed and [`cargo.binstall_native = true`](https://mise.jdx.dev/dev-tools/backends/cargo.html#cargo.binstall_native), mise tries its own prebuilt-binary installer and runs `cargo install` when it finds no binary.
5. Otherwise mise runs `cargo install`.

Set [`cargo.binstall_only = true`](https://mise.jdx.dev/dev-tools/backends/cargo.html#cargo.binstall_only) to fail instead of building from source; options that need a source build then fail too. Set `cargo.binstall = false` to always build. A prebuilt binary ignores your Cargo build configuration, so turn binstall off when that configuration must apply.

By default, mise also stops cargo-binstall from using the third-party [cargo-quickinstall](https://github.com/cargo-bins/cargo-quickinstall) artifact host, so it uses only artifacts published by the crate's authors. Set [`cargo.binstall_quickinstall = true`](https://mise.jdx.dev/dev-tools/backends/cargo.html#cargo.binstall_quickinstall) to allow it. mise's own installer never uses quickinstall.

## Tool options [​](#tool-options)

Set these on the tool's entry in `[tools]`, or inline, as in `'cargo:eza[locked=false]'`. Options every backend accepts are described under [tool options](https://mise.jdx.dev/dev-tools/#tool-options).

mise records the `features`, `default-features`, `bin`, `crate` and `locked` values with each installed version. Changing one reinstalls that version instead of reusing a binary built with other options. Feature names are normalized, so reordering them or switching between a string and an array does not trigger a reinstall.

| Option | Passed as | Skips cargo-binstall |
| --- | --- | --- |
| `features` | `--features` | Yes |
| `default-features = false` | `--no-default-features` | Yes |
| `bin` | `--bin` | No |
| `crate` | the crate name | No |
| `locked` | `--locked` | No |

### `install_env` [​](#install-env)

Set environment variables for `cargo install` or `cargo-binstall`:

```
[tools]
"cargo:eza" = { version = "latest", install_env = { CARGO_NET_GIT_FETCH_WITH_CLI = "true" } }
```

### `features` [​](#features)

Enable crate features:

```
[tools]
"cargo:sqlx-cli" = { version = "latest", features = ["postgres", "rustls"] }
```

### `default-features` [​](#default-features)

Turn off the crate's default features:

```
[tools]
"cargo:cargo-edit" = { version = "latest", default-features = false }
```

### `bin` [​](#bin)

Install one executable from a crate that has several:

```
[tools]
"cargo:sqlx-cli" = { version = "latest", bin = "sqlx" }
```

### `rename_exe` [​](#rename-exe)

Rename the installed executable, for example to avoid a clash with another tool on `PATH`:

```
[tools."cargo:https://github.com/Michael-F-Bryan/wit-lsp"]
version = "rev:c90addb21d37f2bd62a9474adca1dd8f6320d437"
crate = "wit-cli"
bin = "wit"
rename_exe = "wit-lsp"
```

### `crate` [​](#crate)

Choose the crate to install from a Git repository that contains several, such as a Cargo workspace:

```
[tools]
"cargo:https://github.com/astral-sh/ruff" = { version = "tag:0.6.0", crate = "ruff" }
```

### `locked` [​](#locked)

Build with the crate's own `Cargo.lock` (`cargo install --locked`). This is the default; set `false` to let Cargo resolve dependencies again:

```
[tools]
"cargo:eza" = { version = "latest", locked = false }
```

This is the crate's `Cargo.lock`, not [`mise.lock`](https://mise.jdx.dev/dev-tools/mise-lock.html), which records the version of the tool mise installed.

## Settings [​](#settings)

Set these under `[settings]` in a config file, with [`mise settings set`](https://mise.jdx.dev/cli/settings/set.html), or with a setting's environment variable where it has one. See [which value wins](https://mise.jdx.dev/configuration/settings.html#which-value-wins) when a setting is set in more than one place.

### `cargo.binstall`[Permalink to cargo.binstall](#cargo.binstall)

Install Cargo tools from cargo-binstall's prebuilt binaries when cargo-binstall is installed.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_CARGO_BINSTALL`

mise uses a `cargo-binstall` installed through mise or found on `PATH`, and runs `cargo install` when it has no prebuilt binary. A prebuilt binary ignores your Cargo build configuration, so set this to `false` when that configuration must apply. Install cargo-binstall with `mise use -g cargo-binstall`. See [How mise installs a crate](https://mise.jdx.dev/dev-tools/backends/cargo.html#how-mise-installs-a-crate).

### `cargo.binstall_native`[Permalink to cargo.binstall\_native](#cargo.binstall_native)

Use mise's own prebuilt-binary installer for Cargo tools when cargo-binstall is not installed.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_CARGO_BINSTALL_NATIVE`

The installer downloads binaries described by `package.metadata.binstall` in the crate's `Cargo.toml`, or named the usual way in the GitHub releases of the crate's repository. It runs `cargo install` instead when it finds no binary or the install uses Cargo build options that would change the result.

- `true`: try this installer before `cargo install`.
- `false`: never use it.
- unset: do not use it. From mise 2027.1.0, mise warns when a crate could have been installed this way, and from mise 2027.7.0 unset means `true`.

Setting [`cargo.binstall`](#cargo.binstall) to `false` turns this installer off too.

### `cargo.binstall_only`[Permalink to cargo.binstall\_only](#cargo.binstall_only)

Require prebuilt binaries from cargo-binstall instead of falling back to `cargo install`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_CARGO_BINSTALL_ONLY`

Installs fail when cargo-binstall is missing or turned off, has no prebuilt binary, or a tool option such as `features` needs `cargo install`. Tools from a Git source are unaffected; they always build with `cargo install --git`.

### `cargo.binstall_quickinstall`[Permalink to cargo.binstall\_quickinstall](#cargo.binstall_quickinstall)

Let cargo-binstall download third-party artifacts from cargo-quickinstall.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_CARGO_BINSTALL_QUICKINSTALL`

By default, mise passes `--disable-strategies compile,quick-install` to cargo-binstall, so it uses only artifacts that the crate's authors publish, never [cargo-quickinstall](https://github.com/cargo-bins/cargo-quickinstall) builds. With `true`, mise passes `--disable-strategies compile` instead. mise's own installer ([`cargo.binstall_native`](#cargo.binstall_native)) never uses quickinstall.

### `cargo.registry_name`[Permalink to cargo.registry\_name](#cargo.registry_name)

Cargo registry to install crates from instead of crates.io.

- Type: `string`
- Default: unset
- Environment variable: `MISE_CARGO_REGISTRY_NAME`

The name of an [alternate registry](https://doc.rust-lang.org/cargo/reference/registries.html#using-an-alternate-registry) defined in your Cargo configuration (`[registries.<name>]`), such as a private mirror. mise passes it to `cargo install` as `--registry`, and does not use its own prebuilt-binary installer while it is set.

## Troubleshooting [​](#troubleshooting)

| Symptom | What to do |
| --- | --- |
| Compiler or linker error | Read the first Cargo error and install the crate's native build dependencies. `features` always builds from source. |
| No executable installed | The crate must have a binary target. Use `bin`, or `crate` for a Git workspace. |
| A prebuilt binary ignores your Cargo config | Set `cargo.binstall = false` to build locally. |
| `cargo-binstall cannot honor cargo install-only tool option(s)` | `cargo.binstall_only` is set and the tool uses `features` or `default-features = false`. Remove the option or unset `cargo.binstall_only`. |

Implementation: [`src/backend/cargo.rs`](https://github.com/jdx/mise/blob/main/src/backend/cargo.rs).

[Edit this page](https://github.com/jdx/mise/edit/main/docs/dev-tools/backends/cargo.md)

Last updated:

Pager

[Previous pageasdf (legacy)](https://mise.jdx.dev/dev-tools/backends/asdf.html)

[Next pageconda](https://mise.jdx.dev/dev-tools/backends/conda.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)