Settings [​](#settings)

Settings change how mise itself behaves, such as how many jobs run at once, how downloads are verified and how tasks print output. Set them with `mise settings` or under `[settings]` in a config file. Variables for your own programs belong in [`[env]`](https://mise.jdx.dev/environments/) instead.

## Change a setting [​](#change-a-setting)

[`mise settings set`](https://mise.jdx.dev/cli/settings/set.html) writes to the global config, `~/.config/mise/config.toml`. Add `--local` to write to the project config instead, normally `mise.toml` (see [which file mise writes to](https://mise.jdx.dev/configuration.html#target-file-for-write-operations)):

sh

```
mise settings jobs=4          # global config
mise settings --local jobs=2  # project config
mise settings unset --local jobs  # remove the project value
mise settings add disable_hints python_multi  # append to a list setting
```

The same settings in TOML:

```
[settings]
jobs = 4

[settings.node]
compile = true # the node.compile setting
```

A dotted name such as `node.compile` is a table under `[settings]`. Each entry in the [reference](#reference) lists the setting's environment variable, usually `MISE_` followed by the name in upper case with dots replaced by underscores, such as `MISE_NODE_COMPILE`.

To see what mise uses:

sh

```
mise settings get jobs           # the value in effect here
mise settings ls                 # values set in config files, with each file
mise settings ls --all           # every setting, including defaults
```

## Which value wins [​](#which-value-wins)

When a setting is set in more than one place, mise uses the first of these that sets it:

1. A command-line flag, such as `--jobs` or `--yes` (see [global flags](https://mise.jdx.dev/cli/#global-flags))
2. The setting's environment variable, such as `MISE_JOBS`
3. `[settings]` in project config, starting with the file nearest the current directory; `mise.local.toml` wins over `mise.toml` in the same directory
4. `[settings]` in the global config, `~/.config/mise/config.toml`
5. `[settings]` in the system config, `/etc/mise/config.toml`
6. The default

[`minimum_release_age_excludes`](https://mise.jdx.dev/configuration/settings.html#minimum_release_age_excludes) is the exception: it combines the lists from every config file, and its environment variable replaces the combined list. In [safe mode](https://mise.jdx.dev/security.html#safe-mode), mise ignores `[settings]` in project config.

## Settings with restricted locations [​](#settings-with-restricted-locations)

Most settings work in every place listed above. Three kinds do not, and the reference marks each of them with a badge.

### Global-only settings [​](#global-only-settings)

Settings with the global only badge take effect only in the global or system config, or in their environment variable where they have one, so a repository you clone cannot set them. Most of them control what mise runs, trusts, downloads or writes on your machine, or whether it asks before acting, such as [`yes`](https://mise.jdx.dev/configuration/settings.html#yes), [`trusted_config_paths`](https://mise.jdx.dev/configuration/settings.html#trusted_config_paths), [`paranoid`](https://mise.jdx.dev/configuration/settings.html#paranoid), [`github.credential_command`](https://mise.jdx.dev/configuration/settings.html#github.credential_command), [`self_update.auto`](https://mise.jdx.dev/configuration/settings.html#self_update.auto) and [`shims_dir`](https://mise.jdx.dev/configuration/settings.html#shims_dir). In project config, mise ignores them and prints a warning:

text

```
mise WARN  yes in non-global config /home/me/app/mise.toml is ignored for security reasons
```

The [`write_targets`](https://mise.jdx.dev/configuration/settings.html#write_targets) settings matter only to commands that write global config, so for them the warning waits until such a command would have used the ignored value, and names the file it writes instead.

`mise settings --local` and `mise settings add --local` refuse these settings. Drop `--local` to write them to the global config, or use the environment variable.

### Settings read before config files [​](#early-initialization)

Settings with the miserc badge decide which config files mise loads, so mise reads them before any `mise.toml`: [`env`](https://mise.jdx.dev/configuration/settings.html#env), [`auto_env`](https://mise.jdx.dev/configuration/settings.html#auto_env), [`env_conf_d`](https://mise.jdx.dev/configuration/settings.html#env_conf_d), [`ceiling_paths`](https://mise.jdx.dev/configuration/settings.html#ceiling_paths), [`ignored_config_paths`](https://mise.jdx.dev/configuration/settings.html#ignored_config_paths), [`override_config_filenames`](https://mise.jdx.dev/configuration/settings.html#override_config_filenames) and [`override_tool_versions_filenames`](https://mise.jdx.dev/configuration/settings.html#override_tool_versions_filenames). Set them in a [`.miserc.toml` file](https://mise.jdx.dev/configuration.html#miserc), without a `[settings]` table, or with their environment variables. `env` also has the `-E`/`--env` flag.

.miserc.toml

toml

```
env = ["development"]
```

Under `[settings]` in `mise.toml` or the global config they have no effect. `mise settings` refuses them and names the `miserc.toml` file or variable to use, so edit `.miserc.toml` yourself.

### Environment-only settings [​](#environment-only-settings)

Settings with the env only badge name the config files themselves, so they work only as environment variables: [`default_config_filename`](https://mise.jdx.dev/configuration/settings.html#default_config_filename), [`default_tool_versions_filename`](https://mise.jdx.dev/configuration/settings.html#default_tool_versions_filename), [`global_config_file`](https://mise.jdx.dev/configuration/settings.html#global_config_file), [`global_config_root`](https://mise.jdx.dev/configuration/settings.html#global_config_root) and [`system_config_file`](https://mise.jdx.dev/configuration/settings.html#system_config_file). `mise settings` refuses them, and mise ignores them, with a warning, in every config file, including the global one. A `.miserc.toml` ignores them without a warning.

Environment variables that are not settings, such as `MISE_DATA_DIR`, are listed in [`MISE_*` variables](https://mise.jdx.dev/configuration/environment-variables.html).

## Reference [​](#reference)

Groups: [`age`](#age), [`aqua`](#aqua), [`cargo`](#cargo), [`conda`](#conda), [`dotfiles`](#dotfiles), [`dotnet`](#dotnet), [`erlang`](#erlang), [`forgejo`](#forgejo), [`github`](#github), [`github_relay`](#github_relay), [`gitlab`](#gitlab), [`go`](#go), [`history`](#history), [`hook_env`](#hook_env), [`java`](#java), [`node`](#node), [`npm`](#npm), [`oci`](#oci), [`otel`](#otel), [`packslip`](#packslip), [`prune`](#prune), [`pypi`](#pypi), [`python`](#python), [`ruby`](#ruby), [`rust`](#rust), [`sandbox`](#sandbox), [`self_update`](#self_update), [`shims`](#shims), [`skills`](#skills), [`sops`](#sops), [`spm`](#spm), [`status`](#status), [`swift`](#swift), [`system_packages`](#system_packages), [`task`](#task), [`tool_update`](#tool_update), [`upgrade`](#upgrade), [`write_targets`](#write_targets), [`zig`](#zig)

### `activate_aggressive`[Permalink to activate\_aggressive](#activate_aggressive)

Keep mise's tool directories ahead of `PATH` entries added after `mise activate`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_ACTIVATE_AGGRESSIVE`

By default, a directory added to `PATH` after `mise activate` takes precedence over mise's tools. With this in `~/.zshrc`:

sh

```
eval "$(mise activate zsh)"
PATH="/some/other/python:$PATH"
```

`python` runs `/some/other/python`, not the version from `mise.toml`. Move `mise activate` to the end of your shell startup file, or set `activate_aggressive = true` to keep mise's tools first.

### `activate_shims`[Permalink to activate\_shims](#activate_shims)

Let `mise activate` add the shims directories to `PATH` for auto-install and lazy tools.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_ACTIVATE_SHIMS`

Set to `false` to keep the user and system shims directories out of `PATH` during `mise activate` and later prompts. Restart your shell after changing it.

Shims provide missing-version auto-installation and first-use installation of lazy tools, so without them those commands may not be found; run `mise install` or `mise exec` instead. Installed tools still get their own `bin` directories on `PATH`.

Command wrappers, such as `cargo` through [mr-boxington](https://github.com/jdx/mr-boxington), use a separate `command-wrappers/bin` directory and stay active. `mise activate --shims` adds shims whatever this is set to. See [Shims](https://mise.jdx.dev/dev-tools/shims.html#activate-shims).

### `age`[Permalink to age](#age)

#### `age.identity_files`[Permalink to age.identity\_files](#age.identity_files)

Age identity files to try when decrypting.

- Type: `path[]`
- Default: unset
- Environment variable: `MISE_AGE_IDENTITY_FILES` (separated by `:` ( `;` on Windows))

Used for age-encrypted `[env]` values and encrypted dotfiles. See [Decryption identities](https://mise.jdx.dev/environments/secrets/age.html#decryption-identities) for the order mise tries identities in.

#### `age.key_file`[Permalink to age.key\_file](#age.key_file)

Age private key file used to decrypt and as the default recipient when encrypting.

- Type: `path`
- Default: `"~/.config/mise/age.txt"`
- Environment variable: `MISE_AGE_KEY_FILE`

Used for age-encrypted `[env]` values and encrypted dotfiles. See [Decryption identities](https://mise.jdx.dev/environments/secrets/age.html#decryption-identities).

#### `age.ssh_identity_files`[Permalink to age.ssh\_identity\_files](#age.ssh_identity_files)

SSH private keys to try when decrypting age values.

- Type: `path[]`
- Default: unset
- Environment variable: `MISE_AGE_SSH_IDENTITY_FILES` (separated by `:` ( `;` on Windows))

Used for age-encrypted `[env]` values and encrypted dotfiles. See [Decryption identities](https://mise.jdx.dev/environments/secrets/age.html#decryption-identities) for the order mise tries identities in.

#### `age.strict`[Permalink to age.strict](#age.strict)

Fail when an `[env]` age value cannot be decrypted, instead of skipping it.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_AGE_STRICT`

Decryption fails when no identity is found, none can decrypt the value, or the payload is invalid. Set to `false` to skip those variables and resolve the rest of the environment.

### `all_compile`[Permalink to all\_compile](#all_compile)

Compile Node.js, Python, Ruby and Erlang from source instead of using precompiled builds.

- Type: `boolean`
- Default: `true` on Alpine and NixOS, `false` elsewhere
- Environment variable: `MISE_ALL_COMPILE`

Sets the default for [`node.compile`](#node.compile), [`python.compile`](#python.compile), [`ruby.compile`](#ruby.compile) and [`erlang.compile`](#erlang.compile); a tool's own setting takes precedence. Other tools are not affected.

The automatic `true` on Alpine and NixOS is deprecated. Installs that rely on it have warned since mise 2026.8.0, and from mise 2027.8.0 the default is `false` everywhere. Set `all_compile = true` to keep building from source.

### `always_keep_download`[Permalink to always\_keep\_download](#always_keep_download)deprecated

Keep downloaded archives and sources after installing, for debugging.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_ALWAYS_KEEP_DOWNLOAD`
- Deprecated: Finished downloads are kept in the download cache instead; see download\_cache.

mise keeps the files under `~/.local/share/mise/downloads` so you can inspect them while debugging a backend or plugin. Downloads that go through mise's HTTP client are kept in [`download_cache`](#download_cache) instead, under their own names, so this is only needed for files an asdf plugin script downloads itself.

### `always_keep_install`[Permalink to always\_keep\_install](#always_keep_install)

Keep a failed installation's partial install directory and downloads, for debugging.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_ALWAYS_KEEP_INSTALL`

### `aqua`[Permalink to aqua](#aqua)

#### `aqua.baked_registry`[Permalink to aqua.baked\_registry](#aqua.baked_registry)

Use the aqua registry built into mise.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_AQUA_BAKED_REGISTRY`

Set to `false` to resolve aqua packages only from [`aqua.registries`](#aqua.registries), or, when none are configured, from the official registry downloaded from GitHub.

#### `aqua.cosign`[Permalink to aqua.cosign](#aqua.cosign)

Verify Cosign signatures for aqua packages that declare them.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_AQUA_COSIGN`

mise verifies the signatures itself; no `cosign` binary is needed.

#### `aqua.github_attestations`[Permalink to aqua.github\_attestations](#aqua.github_attestations)

Verify GitHub artifact attestations for aqua packages that declare them.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_AQUA_GITHUB_ATTESTATIONS`

Has no effect when the global [`github_attestations`](https://mise.jdx.dev/configuration/settings.html#github_attestations) setting is `false`.

#### `aqua.minisign`[Permalink to aqua.minisign](#aqua.minisign)

Verify Minisign signatures for aqua packages that declare them.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_AQUA_MINISIGN`

mise verifies the signatures itself; no `minisign` binary is needed.

#### `aqua.registries`[Permalink to aqua.registries](#aqua.registries)

Aqua registries to search, in order, before the baked-in registry.

- Type: `string[]`
- Default: unset
- Environment variable: `MISE_AQUA_REGISTRIES` (comma-separated)

Each entry is a repository URL, a URL to a `registry.yaml` file, an absolute `file://` URL, or a path relative to the config file that sets it. For a repository or directory, mise reads `registry.yaml` at its root, or `registry.yml` when that is missing.

Relative paths resolve only from config files, so a path in `MISE_AQUA_REGISTRIES` must be an absolute `file://` URL. `mise settings` writes the value into a config file, so a relative path set that way resolves against that file's config root (`$HOME` for the global config).

While [`aqua.baked_registry`](#aqua.baked_registry) is on, the baked-in registry is the fallback for packages these registries do not define. With [`registry_floating`](https://mise.jdx.dev/configuration/settings.html#registry_floating), mise also downloads the official aqua registry and checks it after the configured registries and before the baked-in one, falling back to the baked-in registry if the download fails. Downloaded registries are cached for [`aqua.registry_cache_ttl`](#aqua.registry_cache_ttl); `file://` registries are read directly. See [Custom registry](https://mise.jdx.dev/dev-tools/backends/aqua.html#custom-registry).

#### `aqua.registry_cache_ttl`[Permalink to aqua.registry\_cache\_ttl](#aqua.registry_cache_ttl)

How long a downloaded aqua registry stays fresh before mise downloads it again.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"1w"`
- Environment variable: `MISE_AQUA_REGISTRY_CACHE_TTL`

Set to `0s` to download remote registries every time mise loads them. Local `file://` registries are always read directly.

#### `aqua.registry_url`[Permalink to aqua.registry\_url](#aqua.registry_url)deprecated

URL of one aqua registry repository to search before the baked-in registry.

- Type: `string`
- Default: unset
- Environment variable: `MISE_AQUA_REGISTRY_URL`
- Deprecated: Use aqua.registries instead.

When both are set, [`aqua.registries`](#aqua.registries) takes precedence.

#### `aqua.slsa`[Permalink to aqua.slsa](#aqua.slsa)

Verify SLSA provenance for aqua packages that declare it.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_AQUA_SLSA`

Has no effect when the global [`slsa`](https://mise.jdx.dev/configuration/settings.html#slsa) setting is `false`.

### `arch`[Permalink to arch](#arch)

Architecture of the precompiled binaries mise downloads.

- Type: `string`
- Default: host architecture
- Environment variable: `MISE_ARCH`

`x86_64` and `amd64` mean `x64`, and `aarch64` means `arm64`; other values, such as `arm`, `loongarch64` or `riscv64`, are used as written. Set it to install binaries for another architecture, for example on a machine that runs x64 binaries under emulation.

### `auto_env`[Permalink to auto\_env](#auto_env)miserc

Automatically enable platform config environments (unix, {os}, {os}-{arch}).

- Type: `boolean`
- Default: `false` ( `true` from mise 2027.6.0)
- Environment variable: `MISE_AUTO_ENV`
- Set in: a [`.miserc.toml`](https://mise.jdx.dev/configuration.html#miserc) file or `MISE_AUTO_ENV`. mise reads it before config files load, so `[settings]` in `mise.toml` has no effect. See [settings read before config files](https://mise.jdx.dev/configuration/settings.html#early-initialization).

When enabled, mise treats these as active config environments, in addition to any explicit `MISE_ENV`: `unix` on Unix-family platforms, the OS name (`linux`, `macos` or `windows`), and `{os}-{arch}`, such as `macos-arm64` or `windows-x64`. Architecture names follow mise's spelling: `x86_64` becomes `x64` and `aarch64` becomes `arm64`.

Config files such as `mise.windows.toml`, `mise/config.macos-arm64.toml` or `mise.unix.toml` then load automatically, and matching `mise.<env>.lock` lockfiles are selected. Platform environments rank below explicit `MISE_ENV` entries: `unix` < `{os}` < `{os}-{arch}` < explicit `MISE_ENV` entries. They do not change the `{{ mise_env }}` template variable or the `MISE_ENV` variable passed to subprocesses.

Set it to `false` to keep today's behavior after the default changes. See [Platform environments](https://mise.jdx.dev/configuration/environments.html#platform-environments).

### `auto_install`[Permalink to auto\_install](#auto_install)

Install missing tools automatically when a command needs them.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_AUTO_INSTALL`

When `false`, mise never installs automatically: this also turns off [`exec_auto_install`](#exec_auto_install), [`not_found_auto_install`](#not_found_auto_install) and [`task.run_auto_install`](#task.run_auto_install). To skip only some tools, use [`auto_install_disable_tools`](#auto_install_disable_tools). See [Auto-install mechanisms](https://mise.jdx.dev/dev-tools/#auto-install-mechanisms).

### `auto_install_disable_tools`[Permalink to auto\_install\_disable\_tools](#auto_install_disable_tools)

Tools that automatic installs skip, such as `node`.

- Type: `string[]`
- Default: unset
- Environment variable: `MISE_AUTO_INSTALL_DISABLE_TOOLS` (comma-separated)

Applies to every automatic install, such as from `mise exec`, `mise run` or the command-not-found handler. Install these tools yourself with `mise install`.

### `auto_update`[Permalink to auto\_update](#auto_update)deprecatedglobal only

Old name for `self_update.auto`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_AUTO_UPDATE`
- Set in: global or system config, or `MISE_AUTO_UPDATE`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).
- Deprecated: Use self\_update.auto instead.

mise still reads this setting and `MISE_AUTO_UPDATE` when [`self_update.auto`](#self_update.auto) is not set.

### `auto_update_check_duration`[Permalink to auto\_update\_check\_duration](#auto_update_check_duration)deprecatedglobal only

Old name for `self_update.check_duration`.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"7d"`
- Environment variable: `MISE_AUTO_UPDATE_CHECK_DURATION`
- Set in: global or system config, or `MISE_AUTO_UPDATE_CHECK_DURATION`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).
- Deprecated: Use self\_update.check\_duration instead.

mise still reads this setting and `MISE_AUTO_UPDATE_CHECK_DURATION` when [`self_update.check_duration`](#self_update.check_duration) is not set.

### `cache_prune_age`[Permalink to cache\_prune\_age](#cache_prune_age)

Delete cache files that have not been accessed for this long.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"30d"`
- Environment variable: `MISE_CACHE_PRUNE_AGE`

mise deletes such files from time to time on its own, and [`mise cache prune`](https://mise.jdx.dev/cli/cache/prune.html) uses the same age. Set to `0s` to keep cache files indefinitely; both the automatic sweep and `mise cache prune` then skip pruning.

This covers the cache directories only. Cached environments expire by [`env_cache_ttl`](#env_cache_ttl) instead. See [Cache auto-pruning](https://mise.jdx.dev/cache-behavior.html#cache-auto-pruning).

### `cargo`[Permalink to cargo](#cargo)

#### `cargo.binstall`[Permalink to cargo.binstall](#cargo.binstall)

Install Cargo tools from cargo-binstall's prebuilt binaries when cargo-binstall is installed.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_CARGO_BINSTALL`

mise uses a `cargo-binstall` installed through mise or found on `PATH`, and runs `cargo install` when it has no prebuilt binary. A prebuilt binary ignores your Cargo build configuration, so set this to `false` when that configuration must apply. Install cargo-binstall with `mise use -g cargo-binstall`. See [How mise installs a crate](https://mise.jdx.dev/dev-tools/backends/cargo.html#how-mise-installs-a-crate).

#### `cargo.binstall_native`[Permalink to cargo.binstall\_native](#cargo.binstall_native)

Use mise's own prebuilt-binary installer for Cargo tools when cargo-binstall is not installed.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_CARGO_BINSTALL_NATIVE`

The installer downloads binaries described by `package.metadata.binstall` in the crate's `Cargo.toml`, or named the usual way in the GitHub releases of the crate's repository. It runs `cargo install` instead when it finds no binary or the install uses Cargo build options that would change the result.

- `true`: try this installer before `cargo install`.
- `false`: never use it.
- unset: do not use it. From mise 2027.1.0, mise warns when a crate could have been installed this way, and from mise 2027.7.0 unset means `true`.

Setting [`cargo.binstall`](#cargo.binstall) to `false` turns this installer off too.

#### `cargo.binstall_only`[Permalink to cargo.binstall\_only](#cargo.binstall_only)

Require prebuilt binaries from cargo-binstall instead of falling back to `cargo install`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_CARGO_BINSTALL_ONLY`

Installs fail when cargo-binstall is missing or turned off, has no prebuilt binary, or a tool option such as `features` needs `cargo install`. Tools from a Git source are unaffected; they always build with `cargo install --git`.

#### `cargo.binstall_quickinstall`[Permalink to cargo.binstall\_quickinstall](#cargo.binstall_quickinstall)

Let cargo-binstall download third-party artifacts from cargo-quickinstall.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_CARGO_BINSTALL_QUICKINSTALL`

By default, mise passes `--disable-strategies compile,quick-install` to cargo-binstall, so it uses only artifacts that the crate's authors publish, never [cargo-quickinstall](https://github.com/cargo-bins/cargo-quickinstall) builds. With `true`, mise passes `--disable-strategies compile` instead. mise's own installer ([`cargo.binstall_native`](#cargo.binstall_native)) never uses quickinstall.

#### `cargo.registry_name`[Permalink to cargo.registry\_name](#cargo.registry_name)

Cargo registry to install crates from instead of crates.io.

- Type: `string`
- Default: unset
- Environment variable: `MISE_CARGO_REGISTRY_NAME`

The name of an [alternate registry](https://doc.rust-lang.org/cargo/reference/registries.html#using-an-alternate-registry) defined in your Cargo configuration (`[registries.<name>]`), such as a private mirror. mise passes it to `cargo install` as `--registry`, and does not use its own prebuilt-binary installer while it is set.

### `ceiling_paths`[Permalink to ceiling\_paths](#ceiling_paths)miserc

Directories where mise stops searching parent directories for config files.

- Type: `path[]`
- Default: `[]`
- Environment variable: `MISE_CEILING_PATHS` (separated by `:` ( `;` on Windows))
- Set in: a [`.miserc.toml`](https://mise.jdx.dev/configuration.html#miserc) file or `MISE_CEILING_PATHS`. mise reads it before config files load, so `[settings]` in `mise.toml` has no effect. See [settings read before config files](https://mise.jdx.dev/configuration/settings.html#early-initialization).

By default, mise looks for config files in the current directory and every parent up to the filesystem root. The search stops at a ceiling directory, which is not searched itself: with `MISE_CEILING_PATHS=/home/user`, `/home/user/projects/app/mise.toml` loads but `/home/user/mise.toml` does not. This matches Git's `GIT_CEILING_DIRECTORIES`.

`mise settings` and `mise settings add` refuse this setting, because mise reads it before config files load; put it in a `.miserc.toml` file or the environment variable instead.

### `color`[Permalink to color](#color)

Use color in mise's terminal output.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_COLOR`

### `color_theme`[Permalink to color\_theme](#color_theme)

Color theme for interactive prompts, such as the task picker in `mise run`.

- Type: `string`
- Default: `"default"`
- Environment variable: `MISE_COLOR_THEME`
- Choices:
  - `auto`: `base16` on light terminals (detected from `COLORFGBG`), `charm` otherwise
  - `default`: same as `auto`
  - `charm`: suits dark terminals
  - `base16`: suits light terminals
  - `catppuccin`
  - `dracula`

With colors off (`color = false`, `NO_COLOR` or `CLICOLOR=0`), prompts are unstyled whatever this is set to.

### `conda`[Permalink to conda](#conda)

#### `conda.channel`[Permalink to conda.channel](#conda.channel)

Channel for `conda:` tools that do not set their own.

- Type: `string`
- Default: `"conda-forge"`
- Environment variable: `MISE_CONDA_CHANNEL`

The package and all of its dependencies must come from this one channel. Override it for one tool with the [`channel`](https://mise.jdx.dev/dev-tools/backends/conda.html#channel) tool option.

### `default_config_filename`[Permalink to default\_config\_filename](#default_config_filename)env only

Extra project config filename mise reads, and the name it gives new config files.

- Type: `string`
- Default: `"mise.toml"`
- Environment variable: `MISE_DEFAULT_CONFIG_FILENAME`
- Set in: `MISE_DEFAULT_CONFIG_FILENAME` only. mise reads it before config files load and ignores it in any config file. See [environment-only settings](https://mise.jdx.dev/configuration/settings.html#environment-only-settings).

mise still reads `mise.toml` and the other standard names. To move the global config, use [`global_config_file`](#global_config_file); to replace the list of names mise reads, use [`override_config_filenames`](#override_config_filenames).

### `default_tool_versions_filename`[Permalink to default\_tool\_versions\_filename](#default_tool_versions_filename)env only

Extra filename mise reads as a `.tool-versions` file.

- Type: `string`
- Default: `".tool-versions"`
- Environment variable: `MISE_DEFAULT_TOOL_VERSIONS_FILENAME`
- Set in: `MISE_DEFAULT_TOOL_VERSIONS_FILENAME` only. mise reads it before config files load and ignores it in any config file. See [environment-only settings](https://mise.jdx.dev/configuration/settings.html#environment-only-settings).

`.tool-versions` files are still read; to stop reading them, use [`override_tool_versions_filenames`](#override_tool_versions_filenames).

### `disable_backends`[Permalink to disable\_backends](#disable_backends)

Backends mise ignores when resolving and installing tools, such as `asdf` or `pypi`.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_DISABLE_BACKENDS` (comma-separated)

Tools already installed with a disabled backend stay on disk and work again once you re-enable it. A vfox backend plugin can be disabled by its plugin name.

### `disable_default_registry`[Permalink to disable\_default\_registry](#disable_default_registry)

Stop mapping short names such as `php` to the asdf and vfox plugins listed in the registry.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_DISABLE_DEFAULT_REGISTRY`

### `disable_hints`[Permalink to disable\_hints](#disable_hints)

Hints to stop showing, by ID; `*` turns off every hint.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_DISABLE_HINTS` (comma-separated)

mise shows each hint once and records it in `$MISE_STATE_DIR/hints`. To turn one off before it appears, or all of them:

sh

```
mise settings add disable_hints python_multi
mise settings add disable_hints '*'
```

Hint IDs include `tools_missing`, `python_multi`, `python_precompiled`, `ruby_precompiled`, `upgrade_auto_prune` and `skills`.

### `disable_tools`[Permalink to disable\_tools](#disable_tools)

Tools mise ignores even when a config file lists them.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_DISABLE_TOOLS` (comma-separated)

Ignored when [`enable_tools`](#enable_tools) is set.

### `disable_update_warning`[Permalink to disable\_update\_warning](#disable_update_warning)

Suppress warnings when a newer mise version is available.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_DISABLE_UPDATE_WARNING`

Hides the update notice in `mise version`, `mise --version` and `mise doctor`. Other warnings are unaffected. It does not turn off `mise self-update` or automatic updates with [`self_update.auto`](#self_update.auto).

### `dotfiles`[Permalink to dotfiles](#dotfiles)

#### `dotfiles.default_mode`[Permalink to dotfiles.default\_mode](#dotfiles.default_mode)

Mode for `[dotfiles]` entries that do not set `mode`.

- Type: `string`
- Default: `"symlink"`
- Environment variable: `MISE_DOTFILES_DEFAULT_MODE`
- Choices:
  - `symlink`: link the target to its source
  - `symlink-each`: create directories and link each file in them
  - `copy`: copy the source to the target
  - `template`: render the source as a template

Applies to entries without `mode`, including string entries such as `"~/.vimrc" = "vimrc"`, and to `mise dot add` without `--mode`. `track` and `absent` cannot be the default; set them on each entry. An unsupported value falls back to `symlink` with a warning. See [Modes](https://mise.jdx.dev/dotfiles/managed.html#modes).

#### `dotfiles.relative_symlinks`[Permalink to dotfiles.relative\_symlinks](#dotfiles.relative_symlinks)

Create dotfile symlinks with relative targets instead of absolute ones.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_DOTFILES_RELATIVE_SYMLINKS`

Applies to `symlink` and `symlink-each` entries; an entry's `relative` key overrides it. Relative links keep working when the home directory is mounted at a different path on another machine or the tree is moved, as long as the target and source keep their positions relative to each other. Ignored on Windows, where directory links are junctions, which cannot be relative. See [relative](https://mise.jdx.dev/dotfiles/managed.html#relative).

#### `dotfiles.root`[Permalink to dotfiles.root](#dotfiles.root)

Directory that holds dotfile sources for entries without a `source`.

- Type: `path`
- Default: `"~/.dotfiles"`
- Environment variable: `MISE_DOTFILES_ROOT`

`~/.zshrc` reads its source from `<root>/.zshrc`, and `mise dot add` saves new sources here. Relative `[dotfile_groups]` roots and the relative keys of destination-variant entries also resolve against it. See [Whole-file entries](https://mise.jdx.dev/dotfiles/managed.html#whole-file-entries).

In a config file, the value may be a template such as `{{ mise_config_dir }}/home`, which keeps the sources beside a dotfiles repository adopted as the global config. A relative result resolves against the config root.

#### `dotfiles.windows_symlinks`[Permalink to dotfiles.windows\_symlinks](#dotfiles.windows_symlinks)

On Windows, deploy dotfile files as symbolic links instead of copies.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_DOTFILES_WINDOWS_SYMLINKS`

By default `symlink-each` copies each file on Windows, and `symlink` links a file only when Windows allows it and otherwise copies it. With this on, both create real file symbolic links and fail when Windows refuses, rather than leaving a copy that no longer follows its source. Creating them needs Developer Mode or an elevated shell. Directory links are always junctions. Ignored on other platforms. See [Windows](https://mise.jdx.dev/dotfiles/managed.html#windows).

### `dotnet`[Permalink to dotnet](#dotnet)

#### `dotnet.cli_telemetry_optout`[Permalink to dotnet.cli\_telemetry\_optout](#dotnet.cli_telemetry_optout)

Set `DOTNET_CLI_TELEMETRY_OPTOUT` to opt out of .NET CLI telemetry.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_DOTNET_CLI_TELEMETRY_OPTOUT`

`true` sets `DOTNET_CLI_TELEMETRY_OPTOUT=1` and `false` sets it to `0`. When unset, mise leaves the variable alone and .NET uses its own default.

#### `dotnet.dotnet_root`[Permalink to dotnet.dotnet\_root](#dotnet.dotnet_root)

Shared directory that every .NET SDK version installs into.

- Type: `path`
- Default: `$DOTNET_ROOT` when set, otherwise `~/.local/share/mise/dotnet-root`
- Environment variable: `MISE_DOTNET_ROOT`

SDK versions install side by side here, matching .NET's own multi-version layout. Ignored when [`dotnet.isolated`](#dotnet.isolated) is `true`.

#### `dotnet.isolated`[Permalink to dotnet.isolated](#dotnet.isolated)

Install each .NET SDK version in its own isolated directory.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_DOTNET_ISOLATED`

With `true`, each SDK version installs under mise's installs directory like other tools, and `dotnet --list-sdks` shows only the active one. With `false`, every version shares [`dotnet.dotnet_root`](#dotnet.dotnet_root) and `dotnet --list-sdks` lists all of them.

#### `dotnet.package_flags`[Permalink to dotnet.package\_flags](#dotnet.package_flags)deprecated

Flags for every `dotnet:` tool; the only flag, `prerelease`, includes prereleases.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_DOTNET_PACKAGE_FLAGS` (comma-separated)
- Deprecated: Use the `prerelease = true` tool option or the global `prereleases` setting instead.

Because it applies to every tool, `prerelease` here also overrides a tool's own `prerelease = false`.

#### `dotnet.registry_url`[Permalink to dotnet.registry\_url](#dotnet.registry_url)

NuGet service index used to discover dotnet tool versions.

- Type: `URL`
- Default: `"https://api.nuget.org/v3/index.json"`
- Environment variable: `MISE_DOTNET_REGISTRY_URL`

Installation runs `dotnet tool install`, which reads its own `NuGet.Config` and credentials. Configure a private feed there as well; this setting does not add an installation source to the dotnet CLI.

### `download_cache`[Permalink to download\_cache](#download_cache)

Reuse finished downloads instead of fetching them again.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_DOWNLOAD_CACHE`

mise keeps finished downloads in `$MISE_CACHE_DIR/downloads-cas`, named by their hash, and uses one again in two cases:

- **The server confirms it is unchanged.** The next request for the same URL carries the `ETag` or `Last-Modified` value the server sent, and a `304 Not Modified` answer means the kept file is used instead of a new download. Servers that send no validator are not cached.
- **A tool pinned its checksum**, in its options or in `mise.lock`, and the kept file was downloaded from the same URL. It is used with no request at all. A file another URL served is not, even when it has that checksum, and neither is anything for a checksum `mise lock` took from mise-versions without downloading (see [`mise.lock`](https://mise.jdx.dev/dev-tools/mise-lock.html#checksums)).

The `http`, `github`, `aqua` and `packslip` backends and the core tools' precompiled archives pin and install from the stored file, linked into place where the filesystem allows, so the archive takes no extra space and is not copied; across devices it is copied. The core tools pin with the checksum in `mise.lock`. Node's source tarball is kept the same way and compiled each time. Other source builds, such as Ruby and Python through `ruby-build` and `python-build`, download their own sources, which are not kept. Every other download that goes through mise's HTTP client uses the first case and is copied into place.

With [`locked`](#locked) or [`paranoid`](#paranoid), only downloads with a pinned checksum are reused or kept, since only those are checked afterwards. Restoring `downloads-cas` in CI then gives a cache you can trust, which a restored install directory is not.

A kept file is hashed again before it is used, and one that no longer matches is dropped and downloaded again. The recorded hash lives beside the file, so this catches damage but does not stop someone who can write the cache from replacing both. A pinned checksum is always checked after the download, so pin one if the cache is shared. The cache holds up to [`download_cache_max_size`](#download_cache_max_size); the least recently used entries go first. Extracted files are not covered; see [Restored caches and installed tools](https://mise.jdx.dev/dev-tools/mise-lock.html#restored-caches).

### `download_cache_max_size`[Permalink to download\_cache\_max\_size](#download_cache_max_size)

Maximum total size of the download cache, such as `500MB` or `10GiB`.

- Type: `string`
- Default: `"2GiB"`
- Environment variable: `MISE_DOWNLOAD_CACHE_MAX_SIZE`

After storing a download, mise removes the least recently used ones until the cache in `$MISE_CACHE_DIR/downloads-cas` fits. A download used in the last ten minutes is kept, since another install may be placing it. `0` means no limit. See [`download_cache`](#download_cache).

### `enable_tools`[Permalink to enable\_tools](#enable_tools)

Allowlist of configured tools mise uses; other tools are ignored.

- Type: `string[]`
- Default: unset
- Environment variable: `MISE_ENABLE_TOOLS` (comma-separated)

When unset, mise uses every configured tool except those in [`disable_tools`](#disable_tools). Once set, this list is the complete allowlist and `disable_tools` is ignored. An empty list (`[]`, or `MISE_ENABLE_TOOLS=` with nothing after it) disables every tool.

### `env`[Permalink to env](#env)miserc

Config environments to load, such as `development` for `mise.development.toml`.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_ENV` (comma-separated)
- Set in: a [`.miserc.toml`](https://mise.jdx.dev/configuration.html#miserc) file or `MISE_ENV`. mise reads it before config files load, so `[settings]` in `mise.toml` has no effect. See [settings read before config files](https://mise.jdx.dev/configuration/settings.html#early-initialization).

Selects which [config environments](https://mise.jdx.dev/configuration/environments.html) load, so `mise.development.toml` applies only when `development` is set. Separate several with commas, such as `MISE_ENV=ci,test`; later ones take precedence. The `-E`/`--env` flag sets it for one command.

### `env_cache`[Permalink to env\_cache](#env_cache)experimental

Cache computed environments on disk for nested mise commands.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_ENV_CACHE`

Caches environment variables and `PATH` so nested commands such as `mise exec -- mise env` skip recomputing them. Entries expire after [`env_cache_ttl`](#env_cache_ttl) or when config files, tool versions, settings, or files that env plugins watch change. Edits to dotenv files or `_.source` scripts may not invalidate an entry; set `MISE_ENV_CACHE=0` for a command that must recompute. Configurations with secrets (an `age` value or a sops-encrypted file) are never cached, and neither are those with `redact = true` unless [`env_cache_redacted`](#env_cache_redacted) is on. See [Environment cache](https://mise.jdx.dev/cache-behavior.html#environment-caching).

### `env_cache_redacted`[Permalink to env\_cache\_redacted](#env_cache_redacted)experimental

Let the environment cache hold values marked `redact = true`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_ENV_CACHE_REDACTED`

By default a config with a `redact = true` directive is never written to the [environment cache](#env_cache), so mise recomputes it on every command. That is slow when such a directive sources a slow script. With this on, mise caches the environment and keeps masking the redacted keys on a cache hit.

The cache file holds the secret values on disk, encrypted with a key that `mise activate` puts in its shell's environment. Every process started from that shell inherits the key and can decrypt the file. After [`env_cache_ttl`](#env_cache_ttl) mise stops using an entry, but the file stays until mise loads it again or you run `mise cache clear`.

`age` and sops values, plugin results with `cacheable = false`, redacted `[vars]`, and redacted `default` and `required` directives are still never cached. See [Environment cache](https://mise.jdx.dev/cache-behavior.html#environment-caching).

### `env_cache_ttl`[Permalink to env\_cache\_ttl](#env_cache_ttl)

How long a cached environment stays valid.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"1h"`
- Environment variable: `MISE_ENV_CACHE_TTL`

Entries are also invalidated when config files, tool versions, settings or watched files change.

### `env_conf_d`[Permalink to env\_conf\_d](#env_conf_d)miserc

Enable environment-specific filenames in conf.d directories.

- Type: `boolean`
- Default: `false` ( `true` from mise 2027.8.10)
- Environment variable: `MISE_ENV_CONF_D`
- Set in: a [`.miserc.toml`](https://mise.jdx.dev/configuration.html#miserc) file or `MISE_ENV_CONF_D`. mise reads it before config files load, so `[settings]` in `mise.toml` has no effect. See [settings read before config files](https://mise.jdx.dev/configuration/settings.html#early-initialization).

With this on, a `conf.d` fragment with an environment in its name loads only when that environment is active: `tools.development.toml` loads only with `development`, and `tools.development.local.toml` is its local variant. It applies to every `conf.d` directory: `mise/conf.d`, `.mise/conf.d`, `.config/mise/conf.d`, `~/.config/mise/conf.d` and `/etc/mise/conf.d`. With it off, every non-hidden TOML file in `conf.d` loads.

Dots in the names of unconditional fragments are deprecated; rename `node.tools.toml` to `node-tools.toml` before the default changes. See [conf.d environments](https://mise.jdx.dev/configuration/environments.html#conf-d-environments).

### `env_file`[Permalink to env\_file](#env_file)

Dotenv file names to load from the current directory and each parent directory.

- Type: `path[]`
- Default: unset
- Environment variable: `MISE_ENV_FILE` (separated by `:` ( `;` on Windows))

mise loads every file with one of these names, such as `.env` and `.env.local`, between the filesystem root and the current directory. When a variable appears in more than one file, the file nearest the current directory wins, and within one directory a name later in the list wins. An absolute path loads that one file. For a dotenv file that belongs to one project, use [`_.file`](https://mise.jdx.dev/environments/#env-file) in `mise.toml` instead.

`MISE_ENV_FILE` takes the names separated by `:` (`;` on Windows).

These values are applied after every config file's `[env]`, so they override it, except for a file that an `_.file` entry has already loaded, which this setting skips. An [`_.file`](https://mise.jdx.dev/environments/#env-file) entry, including one with `find_up = true` in a global config, is an ordinary `[env]` entry that a project's own `[env]` values can override.

### `env_sets`[Permalink to env\_sets](#env_sets)miserc

Named sets of config environments, which `env` and `-E` can name.

- Type: `IndexMap<String, Vec<String>>`
- Default: unset
- Set in: a [`.miserc.toml`](https://mise.jdx.dev/configuration.html#miserc) file. mise reads it before config files load, so `[settings]` in `mise.toml` has no effect. See [settings read before config files](https://mise.jdx.dev/configuration/settings.html#early-initialization).

Defines names that stand for several [config environments](https://mise.jdx.dev/configuration/environments.html). Put them in a shared `miserc.toml` and select them by name:

`env`, `MISE_ENV` and `-E` expand a set name to its environments, in the order the set lists them, before mise reads any config. `{{ mise_env }}` and the `MISE_ENV` that mise exports hold the expanded list. A name that is not a set is an environment, so a machine can still list environments one by one. A set expands one level: a set cannot contain another set, and a set may list an environment of its own name (`server` above). An environment that appears twice keeps its first position.

A project `.miserc.toml` cannot define a set named like one the system or global `miserc.toml` defines, or like an environment it selects; among the other files, the closest one wins for each name.

### `env_shell_expand`[Permalink to env\_shell\_expand](#env_shell_expand)

Expand shell-style variables such as `$FOO` and `${BAR:-default}` in `[env]` values.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_ENV_SHELL_EXPAND`

Expansion runs after Tera templates render, so both syntaxes can be mixed. An undefined variable stays as written and logs a warning; write `${MISSING:-}` to expand it to an empty string without one. Set to `false` to leave `$` sequences untouched. See [Shell-style variable expansion](https://mise.jdx.dev/environments/#shell-style-variable-expansion).

### `erlang`[Permalink to erlang](#erlang)

#### `erlang.compile`[Permalink to erlang.compile](#erlang.compile)

Compile Erlang from source instead of using a precompiled build.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_ERLANG_COMPILE`

`true` always builds from source with kerl. `false` uses only precompiled builds and fails when none exists for the platform. When unset, [`all_compile`](https://mise.jdx.dev/configuration/settings.html#all_compile) decides; if that is unset too, mise uses a precompiled build when one is available and builds from source otherwise.

#### `erlang.precompiled_os`[Permalink to erlang.precompiled\_os](#erlang.precompiled_os)

Ubuntu release whose precompiled Erlang build from builds.hex.pm mise installs on Linux.

- Type: `string`
- Default: unset
- Environment variable: `MISE_ERLANG_PRECOMPILED_OS`
- Choices:
  - `ubuntu-20.04`
  - `ubuntu-22.04`
  - `ubuntu-24.04`
  - `ubuntu-26.04`

Applies to glibc Linux targets, including `mise lock` for Linux platforms from another OS, which otherwise assumes `ubuntu-24.04`. When unset on Linux, mise derives it from `$ImageOS` (GitHub Actions) or `/etc/os-release`, and uses precompiled builds automatically only for Ubuntu 20.04, 22.04 and 24.04. Set it to `ubuntu-26.04` on that release, or to one of these Ubuntu releases on another glibc distribution to use that release's build; other values are rejected. musl Linux always builds from source.

### `exec_auto_install`[Permalink to exec\_auto\_install](#exec_auto_install)

Install missing tools automatically when you run `mise exec`.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_EXEC_AUTO_INSTALL`

Has no effect when [`auto_install`](#auto_install) is `false`.

### `experimental`[Permalink to experimental](#experimental)

Enable experimental features, which may change or be removed in any release.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_EXPERIMENTAL`

Turns on features that are still being designed. An experimental feature can change or disappear in any release; once it leaves experimental, changes follow the normal deprecation process. A feature's docs say when it needs this setting.

Some new behavior also ships behind this flag before it becomes the default, so enabling it helps catch problems early. If something breaks after you enable it, check whether it still happens with `experimental = false`, then [open an issue](https://github.com/jdx/mise/issues).

### `extra_ca_certs`[Permalink to extra\_ca\_certs](#extra_ca_certs)global only

PEM bundle of CA certificates mise trusts in addition to its usual roots.

- Type: `path`
- Default: unset
- Environment variable: `MISE_EXTRA_CA_CERTS`
- Set in: global or system config, or `MISE_EXTRA_CA_CERTS`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Use it when mise's HTTPS traffic passes through a proxy whose CA is not in the system trust store, such as a traffic-inspecting proxy that generates a new CA for each run. The certificates are added to the roots mise already trusts, on every platform, so the file only needs the extra CAs. Unlike `SSL_CERT_FILE`, it never replaces the system store.

sh

```
MISE_EXTRA_CA_CERTS=/path/to/proxy-ca.pem mise install
```

It applies to the connections mise makes with its own HTTP client: downloads, version lookups, GitHub API calls, attestation verification, vfox plugins and the GitHub relay. The package manager embedded in the `npm:` backend reads `NODE_EXTRA_CA_CERTS` instead, and programs mise runs, such as `git`, `uv` or a tool's installer, read their own certificate settings.

The path must be absolute or start with `~/`: a relative path would resolve against the current directory, so mise ignores it. If the file cannot be read or holds a certificate mise cannot use, mise warns and keeps its usual roots: the system store, `SSL_CERT_FILE` where that replaces it, or the bundled Mozilla roots when neither loads. This setting is only read from the environment and the global config, so a project's `mise.toml` cannot add a trusted CA. See [TLS certificates](https://mise.jdx.dev/faq.html#tls-certificates).

### `extract_max_ratio`[Permalink to extract\_max\_ratio](#extract_max_ratio)global only

How many times its own size a downloaded archive or compressed file may unpack to.

- Type: `integer`
- Default: `100`
- Environment variable: `MISE_EXTRACT_MAX_RATIO`
- Set in: global or system config, or `MISE_EXTRACT_MAX_RATIO`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

A small download can be built to unpack to far more data than the disk holds: 1 GiB of zeroes is under 1 KiB of bzip2. When mise unpacks a tool's archive or compressed file, it stops, and the install fails, once the output is larger than 256 MiB and more than this many times the size of the download. Tool archives rarely unpack to more than 10 times their size.

Raise it for a tool that legitimately unpacks to more, such as an archive of mostly empty disk images. `0` means no limit.

### `fetch_remote_versions_cache`[Permalink to fetch\_remote\_versions\_cache](#fetch_remote_versions_cache)

How long a tool's cached list of available versions stays fresh.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"1h"`
- Environment variable: `MISE_FETCH_REMOTE_VERSIONS_CACHE`

`mise outdated`, `mise upgrade`, `mise ls-remote`, `mise lock` and `mise latest` fetch the list again once it is older than this, 1 hour by default.

When this is not set, commands that only need a version matching a request, such as `mise install`, `mise use` and `mise run`, keep a list for the tool's [`minimum_release_age`](#minimum_release_age) less an hour, the most a versions-host snapshot lags: 23 hours by default, 5 hours for a tool with `minimum_release_age = "6h"`, and never less than 1 hour or more than 24 hours. A release newer than the list could not be picked before it reached that age anyway. This applies only to lists that date every version; others are refetched after 1 hour. Setting this value applies it to every command.

Commands that prefer offline data, such as shims, `mise activate`, `mise exec`, `mise env` and `mise ls`, use the cached list whatever its age, as does every command when [`prefer_offline`](#prefer_offline) is on. The same duration controls when remote `include` fragments that point at a branch or tag are refreshed. See [When mise refreshes version lists](https://mise.jdx.dev/cache-behavior.html#version-list-refresh).

### `fetch_remote_versions_timeout`[Permalink to fetch\_remote\_versions\_timeout](#fetch_remote_versions_timeout)

Timeout for looking up a tool's available versions.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"20s"`
- Environment variable: `MISE_FETCH_REMOTE_VERSIONS_TIMEOUT`

Applies to HTTP requests to version sources and to version-listing commands that backends run, such as `go list` or an asdf plugin's scripts. A bare number is seconds.

Commands that prefer offline data, such as shims, shell activation, `mise exec`, `mise env` and `mise ls`, cap this at 3 seconds and do not retry, so they do not hang when the network is down. `mise lock`, `mise ls-remote`, `mise outdated` and `mise upgrade` always use the full timeout.

### `forgejo`[Permalink to forgejo](#forgejo)

#### `forgejo.credential_command`[Permalink to forgejo.credential\_command](#forgejo.credential_command)global only

Command that prints a Forgejo token on stdout.

- Type: `string`
- Default: `""`
- Environment variable: `MISE_FORGEJO_CREDENTIAL_COMMAND`
- Set in: global or system config, or `MISE_FORGEJO_CREDENTIAL_COMMAND`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

mise runs the command with the default inline shell ([`unix_default_inline_shell_args`](https://mise.jdx.dev/configuration/settings.html#unix_default_inline_shell_args) or [`windows_default_inline_shell_args`](https://mise.jdx.dev/configuration/settings.html#windows_default_inline_shell_args)) and uses its trimmed output as the token. The command receives `MISE_CREDENTIAL_HOST`, the host that needs a token, so one command can serve several self-hosted Forgejo servers, and `MISE_CREDENTIAL_PROVIDER=forgejo`. mise tries it right after the token environment variables; if it fails or prints nothing, mise moves on to the next source. It does not need [`forgejo.use_git_credentials`](#forgejo.use_git_credentials).

See [Credential command](https://mise.jdx.dev/dev-tools/github-tokens.html#credential-command) and [Token priority](https://mise.jdx.dev/dev-tools/github-tokens.html#token-priority).

#### `forgejo.fj_cli_tokens`[Permalink to forgejo.fj\_cli\_tokens](#forgejo.fj_cli_tokens)

Read Forgejo tokens that the fj CLI stores in its `keys.json` file.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_FORGEJO_FJ_CLI_TOKENS`

See [CLI logins](https://mise.jdx.dev/dev-tools/github-tokens.html#cli-logins) for where mise looks for the file and [Token priority](https://mise.jdx.dev/dev-tools/github-tokens.html#token-priority) for when it is used.

#### `forgejo.use_git_credentials`[Permalink to forgejo.use\_git\_credentials](#forgejo.use_git_credentials)

Get Forgejo tokens from your Git credential helpers with `git credential fill`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_FORGEJO_USE_GIT_CREDENTIALS`

This reaches tokens kept by helpers such as the macOS Keychain or Windows Credential Manager. mise tries it last, after the environment variables, [`forgejo.credential_command`](#forgejo.credential_command), `forgejo_tokens.toml` and fj CLI tokens, and caches the result per host while it runs. See [Git credential helpers](https://mise.jdx.dev/dev-tools/github-tokens.html#git-credential-helpers).

### `github`[Permalink to github](#github)

#### `github.credential_command`[Permalink to github.credential\_command](#github.credential_command)global only

Command that prints a GitHub token on stdout.

- Type: `string`
- Default: `""`
- Environment variable: `MISE_GITHUB_CREDENTIAL_COMMAND`
- Set in: global or system config, or `MISE_GITHUB_CREDENTIAL_COMMAND`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

mise runs the command with the default inline shell ([`unix_default_inline_shell_args`](https://mise.jdx.dev/configuration/settings.html#unix_default_inline_shell_args) or [`windows_default_inline_shell_args`](https://mise.jdx.dev/configuration/settings.html#windows_default_inline_shell_args)) and uses its trimmed output as the token. The command receives `MISE_CREDENTIAL_HOST`, the host that needs a token, so one command can serve github.com and GitHub Enterprise hosts, and `MISE_CREDENTIAL_PROVIDER=github`. mise tries it right after the token environment variables and before native GitHub OAuth; if it fails or prints nothing, mise moves on to the next source. It does not need [`github.use_git_credentials`](#github.use_git_credentials).

See [Credential command](https://mise.jdx.dev/dev-tools/github-tokens.html#credential-command) and [Token priority](https://mise.jdx.dev/dev-tools/github-tokens.html#token-priority).

#### `github.enterprise_hosts`[Permalink to github.enterprise\_hosts](#github.enterprise_hosts)global only

Extra GitHub Enterprise hosts that receive your GitHub token on their `/api/v3` URLs.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_GITHUB_ENTERPRISE_HOSTS` (comma-separated)
- Set in: global or system config, or `MISE_GITHUB_ENTERPRISE_HOSTS`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

mise sends the GitHub token to `api.github.com` and `api.*.ghe.com`, and to an `/api/v3` URL only on the origin of the tool's [`api_url`](https://mise.jdx.dev/dev-tools/backends/github.html#api-url). A GitHub Enterprise Server URL that reaches mise any other way, such as an `http:` tool, an aqua package or a remote task file under `https://github.mycompany.com/api/v3/`, gets no token. List the hosts that should get one:

The token is sent over `https` only, to any port of a listed host, and only on paths under `/api/v3`. mise ignores this setting in project config, since a repository could otherwise list a host of its choosing. See [GitHub Enterprise](https://mise.jdx.dev/dev-tools/github-tokens.html#github-enterprise).

#### `github.gh_cli_tokens`[Permalink to github.gh\_cli\_tokens](#github.gh_cli_tokens)

Read GitHub tokens that the gh CLI stores in its `hosts.yml` file.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_GITHUB_GH_CLI_TOKENS`

gh stores a token per host, so this also covers GitHub Enterprise hosts. Tokens that gh keeps in the system keyring are not in `hosts.yml`; see [If gh keeps its token in the system keyring](https://mise.jdx.dev/dev-tools/github-tokens.html#gh-keyring). See [CLI logins](https://mise.jdx.dev/dev-tools/github-tokens.html#cli-logins) for where mise looks for the file and [Token priority](https://mise.jdx.dev/dev-tools/github-tokens.html#token-priority) for when it is used.

#### `github.github_attestations`[Permalink to github.github\_attestations](#github.github_attestations)

Verify GitHub artifact attestations for github backend tools.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_GITHUB_GITHUB_ATTESTATIONS`

Attestations are checked only when this setting, the global [`github_attestations`](https://mise.jdx.dev/configuration/settings.html#github_attestations) setting and the tool's own `github_attestations` option are all `true`; set the option to `false` to skip one tool. Tools with a custom `api_url`, such as those on GitHub Enterprise Server, are not checked, because that API has no attestations endpoint.

#### `github.oauth_api_url`[Permalink to github.oauth\_api\_url](#github.oauth_api_url)

GitHub API base URL mise uses to validate native OAuth tokens.

- Type: `string`
- Default: `"https://api.github.com"`
- Environment variable: `MISE_GITHUB_OAUTH_API_URL`

#### `github.oauth_auth_url`[Permalink to github.oauth\_auth\_url](#github.oauth_auth_url)

GitHub OAuth endpoint base URL for the native device flow.

- Type: `string`
- Default: `"https://github.com/login"`
- Environment variable: `MISE_GITHUB_OAUTH_AUTH_URL`

Change it for a GitHub Enterprise host that supports device flow.

#### `github.oauth_cache_get`[Permalink to github.oauth\_cache\_get](#github.oauth_cache_get)global only

Command that prints the native GitHub OAuth refresh tokens stored by github.oauth\_cache\_set.

- Type: `string`
- Default: `""`
- Environment variable: `MISE_GITHUB_OAUTH_CACHE_GET`
- Set in: global or system config, or `MISE_GITHUB_OAUTH_CACHE_GET`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

By default mise keeps the long-lived, rotating OAuth refresh token in `github-oauth-tokens.toml` in its state directory, readable by anything running as your user. Set this and [`github.oauth_cache_set`](#github.oauth_cache_set) together to keep the refresh token in a password manager or keychain instead; the file then holds only the short-lived access token.

mise runs the command with the default inline shell ([`unix_default_inline_shell_args`](https://mise.jdx.dev/configuration/settings.html#unix_default_inline_shell_args) or [`windows_default_inline_shell_args`](https://mise.jdx.dev/configuration/settings.html#windows_default_inline_shell_args)) and reads the stored value from its stdout. It runs only when the access token has to be refreshed, about once every eight hours, and when mise stores a new token, not on every mise invocation. Empty output means nothing is stored yet. If the command fails when mise needs to refresh, mise skips the refresh and keeps the cached token, so a locked vault never costs you the session. While storing a new token, a failure counts as an empty store only if the cache file still holds every refresh token, since many tools fail on an entry that does not exist yet. The command gets no host, because one value holds the tokens for every host. It is stopped after 60 seconds.

See [Keeping the refresh token out of the cache file](https://mise.jdx.dev/dev-tools/github-tokens.html#keeping-the-refresh-token-out-of-the-cache-file).

#### `github.oauth_cache_secure_enclave`[Permalink to github.oauth\_cache\_secure\_enclave](#github.oauth_cache_secure_enclave)global only

Keep the native GitHub OAuth refresh token behind Touch ID in the macOS Secure Enclave.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_GITHUB_OAUTH_CACHE_SECURE_ENCLAVE`
- Set in: global or system config, or `MISE_GITHUB_OAUTH_CACHE_SECURE_ENCLAVE`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

macOS only. mise seals the long-lived, rotating OAuth refresh token to the Secure Enclave and keeps the sealed value in `github-oauth-tokens.sealed` in its state directory; the cache file then holds only the short-lived access token. Unsealing needs Touch ID, so mise asks for it only when the access token has to be refreshed, about once every eight hours, and never on other invocations. The sealed value works only on this Mac and stops working if the enrolled fingerprints change, in which case `mise token github --oauth` authorizes again.

It needs a Mac with Touch ID and the Developer ID signed binary from mise's own releases. Other builds, such as Homebrew, and machines without Touch ID warn and keep the refresh token in the cache file. A failed or cancelled prompt never discards the cached token: mise skips the refresh and tries again later. When set, it takes precedence over [`github.oauth_cache_get`](#github.oauth_cache_get) and [`github.oauth_cache_set`](#github.oauth_cache_set).

See [Keeping the refresh token out of the cache file](https://mise.jdx.dev/dev-tools/github-tokens.html#keeping-the-refresh-token-out-of-the-cache-file).

#### `github.oauth_cache_set`[Permalink to github.oauth\_cache\_set](#github.oauth_cache_set)global only

Command that stores the native GitHub OAuth refresh tokens, read from its stdin.

- Type: `string`
- Default: `""`
- Environment variable: `MISE_GITHUB_OAUTH_CACHE_SET`
- Set in: global or system config, or `MISE_GITHUB_OAUTH_CACHE_SET`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

The counterpart of [`github.oauth_cache_get`](#github.oauth_cache_get); set both or neither. mise writes the complete value to the command's stdin, never to its arguments, each time a refresh token is issued, because GitHub rotates it on every refresh. If the command fails, mise keeps the refresh token in the cache file instead of losing it, and warns.

#### `github.oauth_client_id`[Permalink to github.oauth\_client\_id](#github.oauth_client_id)

GitHub App client ID that turns on native OAuth device-flow tokens.

- Type: `string`
- Default: `""`
- Environment variable: `MISE_GITHUB_OAUTH_CLIENT_ID`

With a client ID, mise creates short-lived GitHub App user access tokens through GitHub's device flow, without a private key, client secret, personal access token or credential helper. Run `mise token github --oauth` once to authorize; mise caches the token, reuses it for GitHub API calls and refreshes it when it can. See [Native GitHub OAuth](https://mise.jdx.dev/dev-tools/github-tokens.html#native-github-oauth).

#### `github.oauth_copy_code`[Permalink to github.oauth\_copy\_code](#github.oauth_copy_code)

Copy the one-time device code to the clipboard during the native GitHub OAuth device flow.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_GITHUB_OAUTH_COPY_CODE`

Only the device code is copied, never the token. mise uses `pbcopy` on macOS, `clip.exe` on Windows and WSL, and `wl-copy`, `xclip` or `xsel` elsewhere. If none is available the code is still printed.

When unset, mise asks whether to copy the code if it is running in a terminal. Set it to `true` to always copy and `false` to never ask. Without a terminal, nothing is copied unless it is `true`.

#### `github.oauth_export_env`[Permalink to github.oauth\_export\_env](#github.oauth_export_env)

Environment variable that mise exports the native GitHub OAuth token under; empty turns the export off.

- Type: `string`
- Default: `"GITHUB_TOKEN"`
- Environment variable: `MISE_GITHUB_OAUTH_EXPORT_ENV`

When [`github.oauth_client_id`](#github.oauth_client_id) is set and a cached or refreshable token is available, mise exports the token under this variable through `mise activate`, `mise env` and `mise exec`, so tools that read it, such as `gh`, can use it. A value set in mise's `[env]` takes precedence; a value already in your shell environment is replaced. Git credential helpers and Cargo do not read this variable. Set it to `GH_TOKEN` for gh's preferred name.

#### `github.oauth_open_browser`[Permalink to github.oauth\_open\_browser](#github.oauth_open_browser)

Open the verification page in your browser during the native GitHub OAuth device flow.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_GITHUB_OAUTH_OPEN_BROWSER`

#### `github.oauth_scopes`[Permalink to github.oauth\_scopes](#github.oauth_scopes)

OAuth scopes requested for native GitHub device-flow tokens.

- Type: `string`
- Default: `""`
- Environment variable: `MISE_GITHUB_OAUTH_SCOPES`

A GitHub App user access token gets its access from the app's permissions and installations, so this usually stays empty.

#### `github.slsa`[Permalink to github.slsa](#github.slsa)

Verify SLSA provenance for github backend tools that name a signer.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_GITHUB_SLSA`

Provenance is checked only for tools that set [`slsa_signer_identity` and `slsa_signer_issuer`](https://mise.jdx.dev/dev-tools/backends/github.html#slsa-signer-identity-and-slsa-signer-issuer), and only while the global [`slsa`](https://mise.jdx.dev/configuration/settings.html#slsa) setting is `true`.

#### `github.use_git_credentials`[Permalink to github.use\_git\_credentials](#github.use_git_credentials)

Get GitHub tokens from your Git credential helpers with `git credential fill`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_GITHUB_USE_GIT_CREDENTIALS`

This reaches tokens kept by helpers such as the macOS Keychain or Windows Credential Manager. mise tries it last, after the environment variables, [`github.credential_command`](#github.credential_command), native GitHub OAuth, `github_tokens.toml` and gh CLI tokens, and caches the result per host while it runs. See [Git credential helpers](https://mise.jdx.dev/dev-tools/github-tokens.html#git-credential-helpers).

### `github_attestations`[Permalink to github\_attestations](#github_attestations)

Verify GitHub artifact attestations for downloads whose publisher provides them.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_GITHUB_ATTESTATIONS`

When `false`, the aqua and github backends skip attestation checks whatever [`aqua.github_attestations`](#aqua.github_attestations) and [`github.github_attestations`](#github.github_attestations) say. For precompiled Python and Ruby, this is the default that [`python.github_attestations`](#python.github_attestations) and [`ruby.github_attestations`](#ruby.github_attestations) override.

### `github_relay`[Permalink to github\_relay](#github_relay)

#### `github_relay.concurrency`[Permalink to github\_relay.concurrency](#github_relay.concurrency)global only

Maximum relayed GitHub requests that run at once, from 1 to 32; more requests fail instead of waiting.

- Type: `integer`
- Default: `8`
- Environment variable: `MISE_GITHUB_RELAY_CONCURRENCY`
- Set in: global or system config, or `MISE_GITHUB_RELAY_CONCURRENCY`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Applies to `mise ssh` and `mise bootstrap remote` sessions started with `--github-relay-read-only`. See [Limits](https://mise.jdx.dev/bootstrap/github-relay.html#limits).

#### `github_relay.log_format`[Permalink to github\_relay.log\_format](#github_relay.log_format)global only

Format of GitHub relay request logs and the end-of-session summary.

- Type: `string`
- Default: `"text"`
- Environment variable: `MISE_GITHUB_RELAY_LOG_FORMAT`
- Set in: global or system config, or `MISE_GITHUB_RELAY_LOG_FORMAT`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).
- Choices:
  - `text`
  - `jsonl`: one JSON event per line

`--github-relay-log-format` overrides it for one session. See [Logging](https://mise.jdx.dev/bootstrap/github-relay.html#logging).

#### `github_relay.log_requests`[Permalink to github\_relay.log\_requests](#github_relay.log_requests)global only

Log each relayed GitHub request, without credentials, on this machine's stderr.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_GITHUB_RELAY_LOG_REQUESTS`
- Set in: global or system config, or `MISE_GITHUB_RELAY_LOG_REQUESTS`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

`--github-relay-log-requests` and `--github-relay-no-log-requests` override it for one session. The setting never turns the relay on. See [Logging](https://mise.jdx.dev/bootstrap/github-relay.html#logging).

#### `github_relay.max_duration`[Permalink to github\_relay.max\_duration](#github_relay.max_duration)global only

How long a GitHub relay lends access; `0s` lasts until the session ends.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"0s"`
- Environment variable: `MISE_GITHUB_RELAY_MAX_DURATION`
- Set in: global or system config, or `MISE_GITHUB_RELAY_MAX_DURATION`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

`--github-relay-max-duration` overrides it for one session. See [When access ends](https://mise.jdx.dev/bootstrap/github-relay.html#when-access-ends).

#### `github_relay.request_timeout`[Permalink to github\_relay.request\_timeout](#github_relay.request_timeout)global only

Time limit for one relayed GitHub request, including streaming its response.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"5m"`
- Environment variable: `MISE_GITHUB_RELAY_REQUEST_TIMEOUT`
- Set in: global or system config, or `MISE_GITHUB_RELAY_REQUEST_TIMEOUT`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Applies to `mise ssh` and `mise bootstrap remote` sessions started with `--github-relay-read-only`. See [Limits](https://mise.jdx.dev/bootstrap/github-relay.html#limits).

### `gitlab`[Permalink to gitlab](#gitlab)

#### `gitlab.credential_command`[Permalink to gitlab.credential\_command](#gitlab.credential_command)global only

Command that prints a GitLab token on stdout.

- Type: `string`
- Default: `""`
- Environment variable: `MISE_GITLAB_CREDENTIAL_COMMAND`
- Set in: global or system config, or `MISE_GITLAB_CREDENTIAL_COMMAND`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

mise runs the command with the default inline shell ([`unix_default_inline_shell_args`](https://mise.jdx.dev/configuration/settings.html#unix_default_inline_shell_args) or [`windows_default_inline_shell_args`](https://mise.jdx.dev/configuration/settings.html#windows_default_inline_shell_args)) and uses its trimmed output as the token. The command receives `MISE_CREDENTIAL_HOST`, the host that needs a token, so one command can serve gitlab.com and self-managed GitLab hosts, and `MISE_CREDENTIAL_PROVIDER=gitlab`. mise tries it right after the token environment variables; if it fails or prints nothing, mise moves on to the next source. It does not need [`gitlab.use_git_credentials`](#gitlab.use_git_credentials).

See [Credential command](https://mise.jdx.dev/dev-tools/github-tokens.html#credential-command) and [Token priority](https://mise.jdx.dev/dev-tools/github-tokens.html#token-priority).

#### `gitlab.glab_cli_tokens`[Permalink to gitlab.glab\_cli\_tokens](#gitlab.glab_cli_tokens)

Read GitLab tokens that the glab CLI stores in its `config.yml` file.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_GITLAB_GLAB_CLI_TOKENS`

See [CLI logins](https://mise.jdx.dev/dev-tools/github-tokens.html#cli-logins) for where mise looks for the file and [Token priority](https://mise.jdx.dev/dev-tools/github-tokens.html#token-priority) for when it is used.

#### `gitlab.use_git_credentials`[Permalink to gitlab.use\_git\_credentials](#gitlab.use_git_credentials)

Get GitLab tokens from your Git credential helpers with `git credential fill`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_GITLAB_USE_GIT_CREDENTIALS`

This reaches tokens kept by helpers such as the macOS Keychain or Windows Credential Manager. mise tries it last, after the environment variables, [`gitlab.credential_command`](#gitlab.credential_command), `gitlab_tokens.toml` and glab CLI tokens, and caches the result per host while it runs. See [Git credential helpers](https://mise.jdx.dev/dev-tools/github-tokens.html#git-credential-helpers).

### `global_config_file`[Permalink to global\_config\_file](#global_config_file)env only

One file to use as the global config instead of the config files in `~/.config/mise`.

- Type: `path`
- Default: unset
- Environment variable: `MISE_GLOBAL_CONFIG_FILE`
- Set in: `MISE_GLOBAL_CONFIG_FILE` only. mise reads it before config files load and ignores it in any config file. See [environment-only settings](https://mise.jdx.dev/configuration/settings.html#environment-only-settings).

When unset, mise reads `config.toml`, `conf.d/*.toml` and the other config files in `~/.config/mise` (or `MISE_CONFIG_DIR`), and global writes such as `mise use -g` go to `config.toml` there. When set, mise reads and writes only this file.

### `global_config_root`[Permalink to global\_config\_root](#global_config_root)env only

Directory used as `config_root` for the global config, such as for `{{config_root}}` in templates.

- Type: `path`
- Default: `$HOME`
- Environment variable: `MISE_GLOBAL_CONFIG_ROOT`
- Set in: `MISE_GLOBAL_CONFIG_ROOT` only. mise reads it before config files load and ignores it in any config file. See [environment-only settings](https://mise.jdx.dev/configuration/settings.html#environment-only-settings).

### `go`[Permalink to go](#go)

#### `go.default_packages_file`[Permalink to go.default\_packages\_file](#go.default_packages_file)deprecated

File listing Go packages to install with every Go version.

- Type: `path`
- Default: `"~/.default-go-packages"`
- Environment variable: `MISE_GO_DEFAULT_PACKAGES_FILE`
- Deprecated: Default go package files are deprecated. Use tool-level postinstall hooks for packages that should be installed into every go version, or use the go: backend for CLI tools.

#### `go.download_mirror`[Permalink to go.download\_mirror](#go.download_mirror)

Base URL that mise downloads Go archives and their checksums from.

- Type: `string`
- Default: `"https://dl.google.com/go"`
- Environment variable: `MISE_GO_DOWNLOAD_MIRROR`

#### `go.repo`[Permalink to go.repo](#go.repo)

Git repository mise lists Go versions from.

- Type: `URL`
- Default: `"https://github.com/golang/go"`
- Environment variable: `MISE_GO_REPO`

mise reads the repository's `go*` tags to find Go versions when it does not get them from the [versions host](https://mise.jdx.dev/configuration/settings.html#use_versions_host); set `use_versions_host = false` to always list from this repository. Downloads come from [`go.download_mirror`](#go.download_mirror).

#### `go.set_gobin`[Permalink to go.set\_gobin](#go.set_gobin)

Set `GOBIN` so `go install` puts binaries in the active Go version's directory.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_GO_SET_GOBIN`

When unset, mise sets `GOBIN` to the active Go version's `bin` directory unless `GOBIN` is already set in your environment. `true` always sets it, replacing your `GOBIN`. `false` never sets it, so `go install` uses your own `GOBIN` or Go's default, `${GOPATH:-$HOME/go}/bin`.

#### `go.set_gopath`[Permalink to go.set\_gopath](#go.set_gopath)deprecated

Set `GOPATH` to the `packages` directory inside the active Go install.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_GO_SET_GOPATH`
- Deprecated: Set GOPATH in \[env] instead. When true, this setting still sets GOPATH to the Go install's `packages` directory.

#### `go.set_goroot`[Permalink to go.set\_goroot](#go.set_goroot)

Set `GOROOT` to the active Go version's install directory.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_GO_SET_GOROOT`

#### `go.skip_checksum`[Permalink to go.skip\_checksum](#go.skip_checksum)

Skip SHA-256 verification of Go downloads.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_GO_SKIP_CHECKSUM`

### `gpg_verify`[Permalink to gpg\_verify](#gpg_verify)

Verify OpenPGP signatures of Node.js and Swift downloads.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_GPG_VERIFY`

Sets the default for [`node.gpg_verify`](#node.gpg_verify) and [`swift.gpg_verify`](#swift.gpg_verify); a tool's own setting takes precedence. mise checks signatures itself, so no `gpg` binary is needed. When unset, mise verifies the signed `SHASUMS256.txt` whenever [`node.verify`](#node.verify) checks a Node.js download, and Swift downloads on Linux. Set to `false` to skip both.

### `history`[Permalink to history](#history)

#### `history.allow_plaintext_history`[Permalink to history.allow\_plaintext\_history](#history.allow_plaintext_history)global only

Allow publishing or applying history that holds plaintext copies of files now marked for encryption, or publishing versions that look like they contain secrets.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_HISTORY_ALLOW_PLAINTEXT_HISTORY`
- Set in: global or system config, or `MISE_HISTORY_ALLOW_PLAINTEXT_HISTORY`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

By default, the history watcher, `mise dot sync` and `mise dot pull` refuse to publish or apply history in which a file now marked for encryption was once saved in plain text, or to publish saved versions with lines that look like secrets, such as `export API_TOKEN=...`. With `true`, they skip those checks, so those older versions can reach this machine or the setup repository. New saves still follow each file's encryption policy. For a single publish, run `mise dot sync --allow-plaintext-history` instead. See [Allow plaintext history](https://mise.jdx.dev/dotfiles/encryption.html#allow-plaintext-history).

#### `history.describe_command`[Permalink to history.describe\_command](#history.describe_command)global only

Command, such as an AI agent, that writes the description of each checkpoint the history watcher saves.

- Type: `string`
- Default: `""`
- Environment variable: `MISE_HISTORY_DESCRIBE_COMMAND`
- Set in: global or system config, or `MISE_HISTORY_DESCRIBE_COMMAND`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

mise saves the checkpoint first, then runs the command once with a JSON object on stdin: `uuid`, `trigger`, the computed `description`, the changed paths (`added`, `modified`, `removed`) and a unified `diff` of changed unencrypted files, cut at 64 KiB. The line the command prints, up to 200 characters, replaces the computed description. If the command fails, prints nothing, or runs longer than 30 seconds, the computed description stays. Leave it empty to keep computed descriptions.

The diff is sent to the command. Excluded files are never named, and encrypted contents are never sent. See [Descriptions from an agent](https://mise.jdx.dev/dotfiles/history.html#descriptions-from-an-agent).

#### `history.enabled`[Permalink to history.enabled](#history.enabled)

Record Git checkpoints of tracked files around bootstrap operations and on explicit saves.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_HISTORY_ENABLED`

Once you track files, or history already exists, `mise bootstrap` and `mise bootstrap <part> apply` record a checkpoint before and after they change anything, and `mise dot save` records one on demand. Dry runs never record. Checkpoints live in `$MISE_STATE_DIR/history/repo.git`; browse them with `mise dot history`.

Set to `false` to stop recording; existing checkpoints are kept. See [Saving](https://mise.jdx.dev/dotfiles/history.html#saving).

#### `history.fetch_interval`[Permalink to history.fetch\_interval](#history.fetch_interval)

How often the history watcher fetches from the setup repository.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"15m"`
- Environment variable: `MISE_HISTORY_FETCH_INTERVAL`

Values below one second count as one second. To stop automatic fetching, set [`history.sync`](#history.sync) to `manual`.

#### `history.notify`[Permalink to history.notify](#history.notify)

Show a desktop notification when a sync conflict pauses sharing.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_HISTORY_NOTIFY`

mise notifies once, when conflicts first pause sharing for the setup; retries stay silent until sharing recovers. Other problems, such as a watcher that cannot save, appear only in `mise doctor` and `mise dot status`. A missing or failing notifier never holds up history or sync.

Linux needs `notify-send`. On macOS, the first notification asks for permission, so mise does not appear in System Settings > Notifications until then; run `mise dot notify` to send a test and get the prompt now. Unofficial macOS builds, such as Homebrew's, never show notifications and warn when you connect a setup repository. Other platforms only log the conflict. See [Conflict notifications](https://mise.jdx.dev/dotfiles/sync.html#conflict-notifications).

#### `history.sync`[Permalink to history.sync](#history.sync)

What the history watcher shares with a connected setup repository on its own.

- Type: `string`
- Default: `"sync"`
- Environment variable: `MISE_HISTORY_SYNC`
- Choices:
  - `sync`: publish saves, fetch periodically and apply incoming changes
  - `fetch-only`: fetch periodically and never publish; apply only when you run `mise dot pull`
  - `manual`: no automatic network activity; `mise dot sync` publishes and fetches, and `mise dot pull` applies, when you run them

`mise dot origin set` asks for a mode when you connect a repository, or takes it from `--sync`. In `sync` mode, mise publishes at most every [`history.sync_interval`](#history.sync_interval) after a save and fetches every [`history.fetch_interval`](#history.fetch_interval); failures back off from one minute up to an hour, and local saves continue. A conflict pauses publishing and applying for the whole setup until you resolve it, while local saves and fetches go on. `mise dot sync` and `mise dot pull` run on request in every mode, but nothing is ever published in `fetch-only` mode.

Applying incoming changes never runs `mise bootstrap`, installs packages or renders templates. When incoming configuration changes what this machine declares, `mise dot status` tells you to run `mise bootstrap`. See [Choose a sync mode](https://mise.jdx.dev/dotfiles/sync.html#choose-a-sync-mode).

#### `history.sync_interval`[Permalink to history.sync\_interval](#history.sync_interval)

Minimum time between automatic publishes to the setup repository after a save.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"5m"`
- Environment variable: `MISE_HISTORY_SYNC_INTERVAL`

#### `history.watch.debounce`[Permalink to history.watch.debounce](#history.watch.debounce)

How long a changed file must stay unchanged before the history watcher saves it.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"2s"`
- Environment variable: `MISE_HISTORY_WATCH_DEBOUNCE`

A file that keeps changing gets a longer interval of its own, up to [`history.watch.max_interval`](#history.watch.max_interval), without delaying other files.

#### `history.watch.max_interval`[Permalink to history.watch.max\_interval](#history.watch.max_interval)

Longest save interval the history watcher uses for a file that changes constantly.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"24h"`
- Environment variable: `MISE_HISTORY_WATCH_MAX_INTERVAL`

A file that keeps being rewritten has its own save interval doubled, up to this value, so it is still saved, ever more rarely; other files are never delayed. A settled file is saved promptly again, and a sustained quiet period resets its interval. mise never excludes a file or switches it to manual saving on its own: exclude logs, caches and databases with `mise dot exclude`, or track a configuration file that changes constantly with `autosave = false` and save it with `mise dot save`. See [Adaptive scheduling](https://mise.jdx.dev/dotfiles/reference.html#adaptive-scheduling).

#### `history.watch.reconcile`[Permalink to history.watch.reconcile](#history.watch.reconcile)

How often the history watcher rescans all tracked files for changes it missed.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"10m"`
- Environment variable: `MISE_HISTORY_WATCH_RECONCILE`

Set to `0s` to turn off periodic rescans; the watcher still rescans at startup and when configuration changes.

### `hook_env`[Permalink to hook\_env](#hook_env)

#### `hook_env.cache_ttl`[Permalink to hook\_env.cache\_ttl](#hook_env.cache_ttl)

How long the shell hook reuses its last config file check, for slow filesystems such as NFS.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"0s"`
- Environment variable: `MISE_HOOK_ENV_CACHE_TTL`

The shell hook checks config files at every prompt, which can be slow on a network filesystem with a cold cache. A positive value such as `5s` reuses the last check for that long; `0s` checks every time. While a check is reused, a newly created config file goes unnoticed until the time runs out; `mise hook-env --force` skips the cache. See [Slow shell prompts](https://mise.jdx.dev/troubleshooting.html#slow-shell-prompts).

#### `hook_env.chpwd_only`[Permalink to hook\_env.chpwd\_only](#hook_env.chpwd_only)

Run the shell hook's full config check only when you change directories, not at every prompt.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_HOOK_ENV_CHPWD_ONLY`

This cuts filesystem work on slow filesystems such as NFS. Changes to config files take effect after your next `cd`; `mise hook-env --force` runs a full check. See [Slow shell prompts](https://mise.jdx.dev/troubleshooting.html#slow-shell-prompts).

### `http_download_timeout`[Permalink to http\_download\_timeout](#http_download_timeout)

Total time allowed to download one file, including retries.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"30m"`
- Environment variable: `MISE_HTTP_DOWNLOAD_TIMEOUT`

Covers every attempt and backoff delay; [`http_timeout`](#http_timeout) separately catches stalled connections. When the server supports byte ranges and sends a strong `ETag` or `Last-Modified` value, mise keeps a partial download and resumes it on the next attempt, even in a later run; otherwise the download starts over. Changing the URL or request headers discards the partial file.

The same limit bounds reading the body of a response that mise parses instead of saving, such as a checksum file or a version list, so a server that sends it a byte at a time cannot hold a command forever.

### `http_retries`[Permalink to http\_retries](#http_retries)

Number of retries after a transient HTTP failure.

- Type: `integer`
- Default: `3`
- Environment variable: `MISE_HTTP_RETRIES`

mise retries only transient errors: HTTP 5xx, 408 (Request Timeout), 429 (Too Many Requests) and network failures such as a refused connection, a timeout or a dropped response body. Other 4xx responses, such as 404, are not retried. Retries back off with jitter, roughly 200ms, 1s, 4s and 15s. Set to `0` to turn retries off.

Commands that prefer offline data, including shims and shell activation, never retry remote lookups: they make one attempt when cached data is not enough, then fail or fall back. Downloading a tool they install, such as the first `mise exec`, still retries. When a retry succeeds, mise logs a warning with the original error.

### `http_timeout`[Permalink to http\_timeout](#http_timeout)

Timeout for connecting or waiting between reads during HTTP requests.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"30s"`
- Environment variable: `MISE_HTTP_TIMEOUT`

The read timer restarts whenever data arrives, so a long download is limited by [`http_download_timeout`](#http_download_timeout) instead.

### `idiomatic_version_file_disable_files`[Permalink to idiomatic\_version\_file\_disable\_files](#idiomatic_version_file_disable_files)

Idiomatic version files to ignore for one tool, as `tool:filename` pairs.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_IDIOMATIC_VERSION_FILE_DISABLE_FILES` (comma-separated)

Turns off one file for one tool without disabling that tool's other version files or other tools that read the same file. To keep `.nvmrc` but stop node from reading `devEngines.runtime` in `package.json`:

sh

```
mise settings add idiomatic_version_file_disable_files node:package.json
```

### `idiomatic_version_file_enable_tools`[Permalink to idiomatic\_version\_file\_enable\_tools](#idiomatic_version_file_enable_tools)

Tools that read idiomatic version files such as `.node-version` or `.ruby-version`.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_IDIOMATIC_VERSION_FILE_ENABLE_TOOLS` (comma-separated)

Idiomatic version files are off by default. To turn them on for Node.js and Python:

sh

```
mise settings add idiomatic_version_file_enable_tools node
mise settings add idiomatic_version_file_enable_tools python
```

See [Idiomatic version files](https://mise.jdx.dev/dev-tools/versions.html#idiomatic-version-files).

### `ignored_config_paths`[Permalink to ignored\_config\_paths](#ignored_config_paths)miserc

Config files and directories mise does not load.

- Type: `path[]`
- Default: `[]`
- Environment variable: `MISE_IGNORED_CONFIG_PATHS` (separated by `:` ( `;` on Windows))
- Set in: a [`.miserc.toml`](https://mise.jdx.dev/configuration.html#miserc) file or `MISE_IGNORED_CONFIG_PATHS`. mise reads it before config files load, so `[settings]` in `mise.toml` has no effect. See [settings read before config files](https://mise.jdx.dev/configuration/settings.html#early-initialization).

Entries are paths or glob patterns, including `**`. A path without glob characters also matches everything under it. Relative paths in a `.miserc.toml` resolve from the directory that holds it, and relative paths in `MISE_IGNORED_CONFIG_PATHS` from the directory where you run mise.

Commands that act on every tracked config (`mise prune`, `mise ls --prunable` and the stale-version check in `mise upgrade`) honor only the environment variable and the global and system `miserc.toml` files, so a project's `.miserc.toml` cannot hide other projects' configs from them. The same applies to global and system config: a project's `.miserc.toml` cannot ignore it.

`mise settings` and `mise settings add` refuse this setting, because mise reads it before config files load; put it in a `.miserc.toml` file or the environment variable instead.

### `install_layout`[Permalink to install\_layout](#install_layout)experimental

Name installations by what they are (`identity`) instead of `installs/<tool>/<version>` (`legacy`).

- Type: `string`
- Default: `identity` on Windows with `experimental = true`, otherwise `legacy`
- Environment variable: `MISE_INSTALL_LAYOUT`
- Choices:
  - `legacy`
  - `identity`

With `identity`, each installation goes into an `installs/<label>-<hash>/` directory named by its identity (backend, version, platform and install-affecting options), and `installs/<tool>/<version>` stays as a link to it. Shorthands such as `age` and `aqua:FiloSottile/age` then share one install, and variants of one version that differ in an install-affecting option can coexist. Existing installs stay where they are and keep working.

It requires `experimental = true`. On Windows, `experimental = true` turns it on by itself; set `install_layout = "legacy"` to keep the legacy layout there. On other platforms, set both:

See [Identity install layout](https://mise.jdx.dev/dev-tools/install-layout.html) for what changes and what does not.

### `ipv4_only`[Permalink to ipv4\_only](#ipv4_only)

Connect to remote hosts over IPv4 only.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_IPV4_ONLY`

Use it when a host answers wrongly over IPv6 while IPv4 works, for example `dl.google.com` returning `404 Not Found` for a Go download, or the Sigstore trust root failing to load during attestation verification. Without it mise follows the system resolver, which usually prefers IPv6 on a dual-stack machine.

It applies to the connections mise makes with its own HTTP client: downloads, version lookups, GitHub API calls, attestation verification, vfox plugins and the GitHub relay. Programs mise runs, such as `git`, `npm` or a tool's installer, follow the system, and so does the package manager embedded in the `npm:` backend. For those, prefer IPv4 system-wide, for example with `precedence ::ffff:0:0/96 100` in `/etc/gai.conf`.

A host with no IPv4 address cannot be reached while this is on.

### `java`[Permalink to java](#java)

#### `java.shorthand_vendor`[Permalink to java.shorthand\_vendor](#java.shorthand_vendor)

Vendor used for a Java version without a vendor prefix, such as `java@21`.

- Type: `string`
- Default: `"temurin"`
- Environment variable: `MISE_JAVA_SHORTHAND_VENDOR`

Versions without a vendor prefix, such as `java@21`, `java@lts` or `java@latest`, install this vendor's builds. The default is `temurin` (Eclipse Temurin), which keeps publishing quarterly updates for long-term-support releases.

Earlier versions of mise defaulted to `openjdk`. jdk.java.net publishes OpenJDK builds only until the next feature release, so with that vendor `java@21` stays on 21.0.2. To keep the old behavior, set this to `openjdk`, or request `openjdk-21` explicitly. Temurin has no builds of Java 9, 10 or 12 through 15; request those with an `openjdk-` prefix.

Installed versions are not changed: a version such as `21.0.2` installed from OpenJDK keeps resolving to that OpenJDK build when the current vendor does not publish it. A lockfile entry records the vendor setting that resolved it, so an entry written with `openjdk` matches only while this setting is `openjdk`. With another vendor, `mise install --locked` reports that the tool is not in the lockfile. Plain `mise lock` locks the installed OpenJDK build again under the new setting; run `mise lock --bump java` to lock the new vendor's build instead.

### `jobs`[Permalink to jobs](#jobs)

How many jobs to run concurrently such as tool installs. Values below 1 are treated as 1.

- Type: `integer`
- Default: `8`
- Environment variable: `MISE_JOBS`

### `libc`[Permalink to libc](#libc)

Libc build to install on Linux when a tool publishes both glibc and musl builds.

- Type: `string`
- Default: unset
- Environment variable: `MISE_LIBC`
- Choices:
  - `glibc`
  - `gnu`
  - `musl`

When unset, mise uses musl builds on a detected musl host, and leaves a glibc host unqualified rather than pinned to glibc, so a backend that picks assets by name can still choose a musl build there. Set it to require the glibc build, for example when a package's musl build misbehaves:

`gnu` is accepted as an alias for `glibc`.

### `locked`[Permalink to locked](#locked)

Require tool versions and download URLs to come from the lockfile.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_LOCKED`

Resolves tools only from lockfile entries, instead of looking versions and URLs up through the GitHub API or a registry. A tool with no matching entry is an error, both for installs and for lookups such as `mise which`, even when the tool is already installed. Backends that lock download URLs must have one for the current platform, with a checksum (see [`locked_require_checksum`](#locked_require_checksum)), and recorded dependency graphs must be present and match their digest; backends that do not lock URLs, such as `cargo`, `go`, `gem` and `asdf`, need only the version. Installs in this mode do not update lockfiles. Same as passing `--locked`.

Use it in CI to avoid API rate limits and to catch an incomplete lockfile early; create the entries with `mise lock`. Locked mode reads existing lockfiles even when [`lockfile`](#lockfile) is unset, and fails with `lockfile = false`. It does not make installs offline: downloads, provenance checks and private-repository API downloads can still need network access and authentication. [`locked_scopes`](#locked_scopes) limits which config scopes it applies to. See [Strict lockfile mode](https://mise.jdx.dev/dev-tools/mise-lock.html#strict-lockfile-mode).

### `locked_require_checksum`[Permalink to locked\_require\_checksum](#locked_require_checksum)

Make locked mode fail when the lockfile has no checksum for a download.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_LOCKED_REQUIRE_CHECKSUM`

[`locked`](#locked) mode requires a checksum in the lockfile for every artifact it downloads for the current platform, including additional GitHub release assets, and fails before downloading when one is missing. Otherwise a lockfile entry with a URL but no checksum would install whatever the URL serves. Run `mise lock` to fill in missing checksums: it downloads and hashes any artifact whose checksum the release does not publish. Backends that do not lock download URLs, such as `cargo` and `go`, are not affected.

Unset, this is on, except with [`lockfile_mode = "merge"`](#lockfile_mode), which can lock a URL without a checksum. Set it to `false` to install such entries anyway, or to `true` to require checksums in merge mode too. The setting will be removed after merge mode is.

### `locked_scopes`[Permalink to locked\_scopes](#locked_scopes)global only

Config scopes where invocation-wide locked mode is enforced.

- Type: `string[]`
- Default: `["global", "project", "system"]`
- Environment variable: `MISE_LOCKED_SCOPES` (comma-separated)
- Set in: global or system config, or `MISE_LOCKED_SCOPES`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).
- Choices:
  - `project`
  - `global`
  - `system`

Limits invocation-wide locked mode (`--locked`, `MISE_LOCKED=1` or [`locked = true`](#locked)) to tools from these config scopes; `global` is your user config. For example, a distribution that declares rolling tools in system config can exclude them while keeping strict locking for project and user tools:

Tools given as command arguments or through environment variables have no config scope, so invocation-wide locked mode always applies to them. A config root's `[tool_config] locked = true` applies even when its scope is not listed. Excluding a scope relaxes locked mode there; mise still reads existing lockfiles. Project config cannot set this, so a repository cannot weaken your locked-mode policy.

### `locked_verify_provenance`[Permalink to locked\_verify\_provenance](#locked_verify_provenance)

Re-verify provenance at install time even when the lockfile already records it.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_LOCKED_VERIFY_PROVENANCE`

By default, when a lockfile entry has both a checksum and a provenance type, `mise install` trusts the lockfile and does not verify provenance (SLSA, cosign, minisign or GitHub artifact attestations) again, which saves API calls. With `true`, mise verifies every time, at the cost of more network requests. [`paranoid`](#paranoid) turns it on.

### `lockfile`[Permalink to lockfile](#lockfile)

Create, read and update `mise.lock` lockfiles for tool versions.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_LOCKFILE`

A lockfile records the exact version, download URLs and checksums resolved for each tool, so every machine installs the same build even when `mise.toml` asks for `node = "24"`.

- `true` in a config file: create, read and update lockfiles for project config.
- unset: read and update lockfiles that exist, but create none; run `mise lock` to create one. `MISE_LOCKFILE=1` behaves the same way.
- `false`: ignore lockfiles. [`locked`](#locked) mode fails with this value.

A lockfile normally sits next to its config file and is named `mise.lock`, or `mise.local.lock`, `mise.<env>.lock` or `mise.<env>.local.lock` for local and environment config; `.config/mise/config.toml` uses `.config/mise/mise.lock`, and fragments in `conf.d` share the lockfile of the directory that holds `conf.d`. Global lockfiles are created only by `mise lock --global`. See [Lockfile](https://mise.jdx.dev/dev-tools/mise-lock.html) and [which lockfile a tool uses](https://mise.jdx.dev/dev-tools/mise-lock.html#which-lockfile-a-tool-uses).

### `lockfile_auto_prune`[Permalink to lockfile\_auto\_prune](#lockfile_auto_prune)

Remove lockfile tools that are absent from the active configuration.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_LOCKFILE_AUTO_PRUNE`

By default, a full `mise lock` removes entries for tools that the active configuration does not declare, which keeps a lockfile in step with one configuration. Set this to `false` for a lockfile shared by [config environments](https://mise.jdx.dev/configuration/environments.html) that declare different tools:

It changes only which tools are kept: configured tools still refresh, and their stale versions, option variants and native dependency graphs are still removed. It applies in both [`lockfile_mode`](#lockfile_mode) values and to `mise lock --global`. A filtered command such as `mise lock node` keeps tools outside its filter anyway.

### `lockfile_mode`[Permalink to lockfile\_mode](#lockfile_mode)

How lockfiles are written: rebuilt from the current requests, or entry by entry.

- Type: `string`
- Default: `"generate"`
- Environment variable: `MISE_LOCKFILE_MODE`
- Choices:
  - `merge`: \[deprecated] update entries as tools are installed
  - `generate`: rebuild the whole lockfile from the current requests, reusing unchanged entries, and lock other platforms during installs

Neither mode creates lockfiles; set [`lockfile = true`](#lockfile) or run `mise lock`. See [Complete lockfile generation](https://mise.jdx.dev/dev-tools/mise-lock-reference.html#complete-lockfile-generation).

WARNING

`merge` is deprecated. mise will start warning about it in 2027.4.0 and remove it in 2027.10.0. Remove `lockfile_mode` to use the default `generate` mode.

### `lockfile_platforms`[Permalink to lockfile\_platforms](#lockfile_platforms)

Platforms to resolve lockfile URLs and checksums for.

- Type: `string[]`
- Default: unset
- Environment variable: `MISE_LOCKFILE_PLATFORMS` (comma-separated)

Applies to `mise lock` without `--platform` and to automatic locking during `mise install` and `mise use`. The current platform is always added. Listing only the platforms you use keeps lockfiles smaller and installs faster. An explicit `mise lock --platform` ignores this setting.

When unset, mise keeps the platforms already recorded in the lockfile, and automatic locking also adds the current platform. A new lockfile gets `linux-x64`, `linux-x64-musl`, `linux-arm64`, `linux-arm64-musl`, `macos-x64`, `macos-arm64` and `windows-x64`, plus the current platform. Write platforms as `os-arch` or `os-arch-qualifier`, such as `linux-x64-musl`.

### `minimum_release_age`[Permalink to minimum\_release\_age](#minimum_release_age)

Skip versions published more recently than this duration or date.

- Type: `string`
- Default: `"24h"`
- Environment variable: `MISE_MINIMUM_RELEASE_AGE`

Holds back newly published versions to limit supply-chain risk. Accepts a duration such as `7d`, `6mo` or `1y`, or a cutoff date such as `2024-06-01` or `2024-06-01T12:00:00Z`; `0s` turns the delay off.

The `24h` default applies to every backend, except for tools in [`minimum_release_age_excludes`](#minimum_release_age_excludes). A backend can only hold back versions it knows a release date for. When mise installs a version that has none, it warns that the cutoff could not be checked; see [Which backends have a default](https://mise.jdx.dev/security.html#which-backends-have-a-default). A value you set applies to every backend.

The cutoff applies when mise resolves a version request such as `node@22` or `latest`, for backends that report release dates. Exact versions such as `node@22.5.0`, and versions recorded in `mise.lock`, are not filtered. The `npm:` and `pypi:` backends also pass the cutoff to the package manager for unpinned transitive dependencies. Override it for one tool with the `minimum_release_age` tool option, or for one command with `--minimum-release-age`. See [Minimum release age](https://mise.jdx.dev/security.html#minimum-release-age).

### `minimum_release_age_excludes`[Permalink to minimum\_release\_age\_excludes](#minimum_release_age_excludes)

Tools and backends that the configured and default `minimum_release_age` do not apply to.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_MINIMUM_RELEASE_AGE_EXCLUDES` (comma-separated)

An entry matches a backend wildcard such as `npm:*`, a registry short name such as `trivy`, or a full backend ID such as `aqua:aquasecurity/trivy`:

A tool's own `minimum_release_age` option and `--minimum-release-age` still apply to excluded tools. Lists from several config files are combined, so a project can add exclusions without repeating global ones; `MISE_MINIMUM_RELEASE_AGE_EXCLUDES` replaces the combined list.

### `netrc`[Permalink to netrc](#netrc)

Use a netrc file for HTTP Basic authentication.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_NETRC`

mise reads credentials from the netrc file and sends them to matching hosts with HTTP Basic authentication, such as a private Artifactory or Nexus server. The file is `~/.netrc` on Unix and macOS. On Windows, mise looks for `%USERPROFILE%\_netrc`, then `%USERPROFILE%\.netrc`. Use [`netrc_file`](#netrc_file) for another location.

text

```
machine artifactory.example.com
  login myuser
  password mytoken
```

### `netrc_file`[Permalink to netrc\_file](#netrc_file)

Path of the netrc file to use instead of `~/.netrc` (`%USERPROFILE%\_netrc` on Windows).

- Type: `path`
- Default: unset
- Environment variable: `MISE_NETRC_FILE`

### `no_env`[Permalink to no\_env](#no_env)

Do not load environment variables from config files.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_NO_ENV`

Same as the global `--no-env` flag. See [Environment variables](https://mise.jdx.dev/environments/#no-env).

### `no_hooks`[Permalink to no\_hooks](#no_hooks)

Do not execute hooks from config files.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_NO_HOOKS`

Same as the global `--no-hooks` flag. See [Hooks](https://mise.jdx.dev/hooks.html#disable-hooks).

### `node`[Permalink to node](#node)

#### `node.apply_patches`[Permalink to node.apply\_patches](#node.apply_patches)

Patch files or URLs to apply to the Node.js source before building it.

- Type: `string`
- Default: unset
- Environment variable: `MISE_NODE_APPLY_PATCHES`

List one patch file or URL per line. mise applies them to the extracted source before `./configure` runs. They apply only to source builds: with [`node.compile = true`](#node.compile), or when no precompiled build exists for the version and mise compiles instead. When mise installs a precompiled build, it warns that the patches were skipped.

#### `node.cflags`[Permalink to node.cflags](#node.cflags)

Extra `CFLAGS` for compiling Node.js, for example to replace `-O3`.

- Type: `string`
- Default: unset
- Environment variable: `MISE_NODE_CFLAGS`

When unset, mise uses `NODE_CFLAGS` from the environment.

#### `node.compile`[Permalink to node.compile](#node.compile)

Compile Node.js from source instead of using a precompiled build.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_NODE_COMPILE`

`false` uses only precompiled builds and fails when none exists. When unset, [`all_compile`](https://mise.jdx.dev/configuration/settings.html#all_compile) decides; if that is unset too, mise uses a precompiled build when one exists and compiles otherwise. See [Building from source](https://mise.jdx.dev/lang/node.html#building-from-source).

#### `node.concurrency`[Permalink to node.concurrency](#node.concurrency)

Number of parallel jobs (`make -j`) when compiling Node.js.

- Type: `integer`
- Default: physical CPU cores, or unset when `node.ninja` is on
- Environment variable: `MISE_NODE_CONCURRENCY`

Values below 1 count as 1. With [`node.ninja`](#node.ninja), ninja picks the job count when this is unset.

#### `node.configure_opts`[Permalink to node.configure\_opts](#node.configure_opts)

Extra arguments for `./configure` when compiling Node.js.

- Type: `string`
- Default: unset
- Environment variable: `MISE_NODE_CONFIGURE_OPTS`

When unset, mise uses `NODE_CONFIGURE_OPTS` from the environment.

#### `node.corepack`[Permalink to node.corepack](#node.corepack)

Run `corepack enable` after installing a Node.js version, so `yarn` and `pnpm` go through Corepack.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_NODE_COREPACK`

Applies to Node.js versions that include Corepack. When [`node.npm_shim`](#node.npm_shim) is off, mise also runs `corepack enable npm`. mise installs package managers itself, so Corepack is optional; see [Corepack](https://mise.jdx.dev/lang/node.html#corepack).

#### `node.default_packages_file`[Permalink to node.default\_packages\_file](#node.default_packages_file)deprecated

File listing npm packages to install globally into each new Node.js version.

- Type: `path`
- Default: `"~/.default-npm-packages"`
- Environment variable: `MISE_NODE_DEFAULT_PACKAGES_FILE`
- Deprecated: Default npm package files are deprecated. Use tool-level postinstall hooks for packages that should be installed into every node version, or use the npm: backend for CLI tools.

One package per line, with an optional version:

text

```
typescript@latest
@types/node@^24
```

When this is unset, mise uses `NODE_DEFAULT_PACKAGES_FILE`, then `~/.default-nodejs-packages` or `~/.default-node-packages` if one exists, and otherwise `~/.default-npm-packages`. See [Default packages file](https://mise.jdx.dev/lang/node.html#default-node-packages).

#### `node.flavor`[Permalink to node.flavor](#node.flavor)

Linux build flavor of Node.js to download, such as `musl` or `glibc-217`.

- Type: `string`
- Default: unset
- Environment variable: `MISE_NODE_FLAVOR`

Flavors come from the [unofficial builds](https://unofficial-builds.nodejs.org/) project, so also set [`node.mirror_url`](#node.mirror_url) to `https://unofficial-builds.nodejs.org/download/release/`. `musl` is the exception: with the default mirror, mise downloads it from the unofficial builds by itself, and it selects `musl` automatically on musl hosts. See [Unofficial builds](https://mise.jdx.dev/lang/node.html#unofficial-builds).

#### `node.gpg_verify`[Permalink to node.gpg\_verify](#node.gpg_verify)

Check the OpenPGP signature of the `SHASUMS256.txt` file that `node.verify` checks downloads against.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_NODE_GPG_VERIFY`

mise checks the signature itself, so `gpg` does not need to be installed. It runs only when [`node.verify`](#node.verify) is on, for installs and for the checksums [`mise lock`](https://mise.jdx.dev/cli/lock.html) records. A missing `.sig` fails the check, except on unofficial-builds, which signs nothing. Set to `false` to skip the check, for example for a mirror that does not copy the signatures or for one of the few early releases (such as 4.0.0 and 5.12.0) that never published one. The global [`gpg_verify`](https://mise.jdx.dev/configuration/settings.html#gpg_verify) setting sets the default.

#### `node.make`[Permalink to node.make](#node.make)

Command to run instead of `make` when compiling Node.js, such as `gmake`.

- Type: `string`
- Default: unset
- Environment variable: `MISE_NODE_MAKE`

Used for both the build and `make install`.

#### `node.make_install_opts`[Permalink to node.make\_install\_opts](#node.make_install_opts)

Extra arguments for `make install` when compiling Node.js.

- Type: `string`
- Default: unset
- Environment variable: `MISE_NODE_MAKE_INSTALL_OPTS`

When unset, mise uses `NODE_MAKE_INSTALL_OPTS` from the environment.

#### `node.make_opts`[Permalink to node.make\_opts](#node.make_opts)

Extra arguments for `make` when compiling Node.js.

- Type: `string`
- Default: unset
- Environment variable: `MISE_NODE_MAKE_OPTS`

When unset, mise uses `NODE_MAKE_OPTS` from the environment.

#### `node.mirror_url`[Permalink to node.mirror\_url](#node.mirror_url)

Base URL that mise downloads Node.js archives and their `SHASUMS256.txt` checksums from.

- Type: `URL`
- Default: `"https://nodejs.org/dist/"`
- Environment variable: `MISE_NODE_MIRROR_URL`

When unset, mise uses `NODE_BUILD_MIRROR_URL` from the environment, then the default. With the default URL, musl builds come from the unofficial builds site instead. See [Unofficial builds](https://mise.jdx.dev/lang/node.html#unofficial-builds).

#### `node.ninja`[Permalink to node.ninja](#node.ninja)

Build Node.js with ninja instead of make when compiling from source.

- Type: `boolean`
- Default: `true` when `ninja` is on `PATH`
- Environment variable: `MISE_NODE_NINJA`

With ninja, and [`node.concurrency`](#node.concurrency) unset, ninja chooses how many jobs to run in parallel.

#### `node.nodenv_root`[Permalink to node.nodenv\_root](#node.nodenv_root)

Root directory of nodenv, which `mise sync node --nodenv` imports Node.js versions from.

- Type: `path`
- Default: `"~/.nodenv"`
- Environment variable: `NODENV_ROOT`

#### `node.npm_shim`[Permalink to node.npm\_shim](#node.npm_shim)

Wrap each Node.js version's `npm` so that `npm install -g` runs `mise reshim` afterwards.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_NODE_NPM_SHIM`

The wrapper reshims after global installs and uninstalls, so new global commands get [shims](https://mise.jdx.dev/dev-tools/shims.html). It is not installed on Windows. Turn it off to let Corepack or a global `npm install -g npm@...` manage `bin/npm` directly. See [Global npm packages](https://mise.jdx.dev/lang/node.html#global-npm-packages).

#### `node.nvm_dir`[Permalink to node.nvm\_dir](#node.nvm_dir)

Directory of nvm, which `mise sync node --nvm` imports Node.js versions from.

- Type: `path`
- Default: `"~/.nvm"`
- Environment variable: `NVM_DIR`

#### `node.verify`[Permalink to node.verify](#node.verify)

Verify downloaded Node.js archives against the SHA-256 checksums in `SHASUMS256.txt`.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_NODE_VERIFY`

When the lockfile has no checksum for an archive, mise downloads `SHASUMS256.txt`, checks its OpenPGP signature as [`node.gpg_verify`](#node.gpg_verify) allows, and records the archive's checksum. Setting this to `false` skips both the checksum file and its signature check. A checksum already in the lockfile is still verified.

### `not_found_auto_install`[Permalink to not\_found\_auto\_install](#not_found_auto_install)

Install the tool that provides a command when your shell cannot find that command.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_NOT_FOUND_AUTO_INSTALL`

When a command is not found in a shell where mise is activated, or a shim cannot resolve its tool, mise installs the tool that provides the command if a config file for the current directory requests that tool. It finds the tool from the registry's list of each tool's commands, so this also covers a configured tool that has never been installed. A tool configured with a backend identifier such as `github:owner/repo` or `cargo:name` has no such list, so mise finds it only once a version of it is installed. Turn this off if the handler interferes with your shell's own command-not-found behavior.

To also install tools that no config requests, enable [`not_found_auto_install_registry`](#not_found_auto_install_registry). Has no effect when [`auto_install`](#auto_install) is `false`. See [Typing a missing command does not install it](https://mise.jdx.dev/troubleshooting.html#auto-install-on-command-not-found-does-not-trigger).

### `not_found_auto_install_registry`[Permalink to not\_found\_auto\_install\_registry](#not_found_auto_install_registry)

Install an unconfigured tool when exactly one registry entry provides a command your shell cannot find.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_NOT_FOUND_AUTO_INSTALL_REGISTRY`

When [`not_found_auto_install`](#not_found_auto_install) finds no configured tool for the command, mise looks the command up in the registry. If exactly one enabled tool provides it, mise installs that tool's latest version and adds it to the global config, as `mise use --global <tool>` does. Ambiguous matches are skipped, and tools in [`auto_install_disable_tools`](#auto_install_disable_tools) are never installed this way.

### `not_found_system_fallback`[Permalink to not\_found\_system\_fallback](#not_found_system_fallback)

Let a shim run a same-named executable from elsewhere on `PATH` when mise cannot resolve the tool.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_NOT_FOUND_SYSTEM_FALLBACK`

This can make a configured but uninstalled tool silently run an unrelated system binary, which is common for commands the OS also ships, such as `python3`. Set both this and [`not_found_auto_install`](#not_found_auto_install) to `false` to make such a shim fail instead.

### `npm`[Permalink to npm](#npm)

#### `npm.package_manager`[Permalink to npm.package\_manager](#npm.package_manager)

Package manager that installs `npm:` tools.

- Type: `string`
- Default: `"auto"`
- Environment variable: `MISE_NPM_PACKAGE_MANAGER`
- Choices:
  - `auto`: the embedded aube, or npm when `npm.shell_out` is on
  - `npm`: `npm install -g`
  - `aube`: the embedded aube, even when `npm.shell_out` is on
  - `aube_cli`: a standalone `aube` executable
  - `bun`: bun
  - `pnpm`: pnpm

Every value except `auto` and `aube` needs that executable on `PATH`. Options such as `pnpm_args` apply only to the package manager they name. See [Choosing an installer](https://mise.jdx.dev/dev-tools/backends/npm.html#choosing-an-installer).

#### `npm.provenance`[Permalink to npm.provenance](#npm.provenance)

Verify the Sigstore provenance npm publishes for an `npm:` tool's own package.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_NPM_PROVENANCE`

On by default. `mise install` fetches the attestations the public npm registry holds for the tool's package and requires the SLSA provenance among them to verify against the integrity hash the install locks. A package that publishes no provenance, or a private registry that serves none, installs as it does with this off. Provenance must come from GitHub Actions or from CI on gitlab.com; provenance built anywhere else, such as on a self-hosted GitLab, is not accepted yet, so such a package fails to install while this is on.

The first install with this on records the repository the provenance names in `mise.lock` (lockfile revision 4). Later installs of that version must show the same repository, and a package that has stopped publishing provenance is refused.

Only the embedded aube package manager verifies provenance. An install through another [`npm.package_manager`](#npm.package_manager), and one of a local archive pinned only by a checksum, installs unchecked. The `provenance` category of [`paranoid`](https://mise.jdx.dev/configuration/settings.html#paranoid) turns verification on even when this is off, and makes those installs an error instead. If npm's attestation lookup fails, [`provenance_api_failures_fatal`](https://mise.jdx.dev/configuration/settings.html#provenance_api_failures_fatal) decides whether the install fails or goes on unchecked, unless `mise.lock` pins a repository. Set this to `false` to skip verification, for example behind a network that blocks Sigstore's trust roots.

#### `npm.shell_out`[Permalink to npm.shell\_out](#npm.shell_out)

Use the npm CLI instead of mise's built-in client for `npm:` version metadata and default installs.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_NPM_SHELL_OUT`

With this on, mise runs `npm view` for version metadata, and the default `auto` [`npm.package_manager`](#npm.package_manager) installs with `npm install -g` instead of the embedded aube. An explicit package manager, including `aube`, is used as chosen. npm must be installed.

Use it for npm behavior the built-in client does not replicate, or to debug a registry problem with npm itself. See [Private registries](https://mise.jdx.dev/dev-tools/backends/npm.html#registry-configuration) for the npm configuration the built-in client reads.

### `oci`[Permalink to oci](#oci)

#### `oci.default_from`[Permalink to oci.default\_from](#oci.default_from)experimental

Base image for `mise oci` builds when neither `[oci].from` nor `--from` sets one.

- Type: `string`
- Default: `"debian:bookworm-slim"`
- Environment variable: `MISE_OCI_DEFAULT_FROM`

Choose a base whose libc and shared libraries suit the tools you package. Most prebuilt tool binaries need glibc, which the default image and `ubuntu:24.04` provide. An Alpine or other musl base needs musl or static builds; changing the base does not rebuild installed tools for another libc. `scratch` builds without a base image. See [Base images](https://mise.jdx.dev/dev-tools/mise-oci.html#registry-base-image-support).

#### `oci.default_mount_point`[Permalink to oci.default\_mount\_point](#oci.default_mount_point)experimental

Directory inside `mise oci` images that holds tool installs, unless `[oci].mount_point` sets one.

- Type: `string`
- Default: `"/mise"`
- Environment variable: `MISE_OCI_DEFAULT_MOUNT_POINT`

It must be an absolute path. The image sets `MISE_DATA_DIR` to it, and tools install under its `installs` directory, with copied vfox plugins under `plugins`. The `--mount-point` flag overrides it for one command.

#### `oci.insecure_registries`[Permalink to oci.insecure\_registries](#oci.insecure_registries)

Registries, as `host` or `host:port`, that mise contacts over plain HTTP instead of HTTPS.

- Type: `string[]`
- Default: unset
- Environment variable: `MISE_OCI_INSECURE_REGISTRIES` (comma-separated)

Applies to `mise oci` commands and to `oci::` references in `include` and in task includes, like Docker's `insecure-registries` daemon option. Loopback registries such as `localhost:5000` always use plain HTTP and need no entry. An entry without a port matches the host on any port.

See [Plain-HTTP registries](https://mise.jdx.dev/dev-tools/mise-oci.html#plain-http-registries).

### `offline`[Permalink to offline](#offline)

Block every HTTP request, so mise uses only installed tools and cached data.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_OFFLINE`

Commands that need the network, such as installing a new tool version, fail. Use it on air-gapped machines or when the network is unreliable. Offline mode also turns on [`prefer_offline`](#prefer_offline), the softer mode that shims and shell activation use by themselves.

### `os`[Permalink to os](#os)

Operating system of the precompiled binaries mise downloads.

- Type: `string`
- Default: host OS
- Environment variable: `MISE_OS`

`darwin` is accepted for `macos`. Other values, such as `linux` or `windows`, are used as written.

### `otel`[Permalink to otel](#otel)

#### `otel.enabled`[Permalink to otel.enabled](#otel.enabled)experimental

Export OpenTelemetry traces of `mise run`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_OTEL_ENABLED`

mise exports traces only when a traces endpoint is also set, with `OTEL_EXPORTER_OTLP_ENDPOINT` or `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`, and reads other options, such as `OTEL_EXPORTER_OTLP_HEADERS` and `OTEL_SERVICE_NAME`, from the standard OpenTelemetry variables. Because export needs this setting too, an endpoint set for another tool does not make mise send data. Export also needs `experimental = true`, and offline mode turns it off.

Task output is exported separately, with [`otel.logs`](#otel.logs). See [OpenTelemetry](https://mise.jdx.dev/tasks/opentelemetry.html).

#### `otel.logs`[Permalink to otel.logs](#otel.logs)experimental

Export each line of task stdout and stderr as an OpenTelemetry log record.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_OTEL_LOGS`

mise exports logs only when a logs endpoint is also set, with `OTEL_EXPORTER_OTLP_ENDPOINT` or `OTEL_EXPORTER_OTLP_LOGS_ENDPOINT`. Each record is linked to its task's span, which [`otel.enabled`](#otel.enabled) exports. Log export also needs `experimental = true`.

This is separate from `otel.enabled` because it sends task output, including any secret a task prints, to the collector. With log export on, `interleave` tasks get a pipe instead of the terminal, which can change programs that check for a TTY, such as their colors, progress bars and prompts. See [Logs](https://mise.jdx.dev/tasks/opentelemetry.html#logs).

### `override_config_filenames`[Permalink to override\_config\_filenames](#override_config_filenames)miserc

Project config filenames that mise looks for instead of `mise.toml`, `.mise.toml` and the other standard names.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_OVERRIDE_CONFIG_FILENAMES` (colon-separated)
- Set in: a [`.miserc.toml`](https://mise.jdx.dev/configuration.html#miserc) file or `MISE_OVERRIDE_CONFIG_FILENAMES`. mise reads it before config files load, so `[settings]` in `mise.toml` has no effect. See [settings read before config files](https://mise.jdx.dev/configuration/settings.html#early-initialization).

In each directory, mise loads only files with these names, and environment-specific files such as `mise.development.toml` still load. The first name is also the name new config files get. For example, to load `mise.toml` but not `mise.local.toml` or `.mise.toml`:

sh

```
export MISE_OVERRIDE_CONFIG_FILENAMES=mise.toml
```

### `override_tool_versions_filenames`[Permalink to override\_tool\_versions\_filenames](#override_tool_versions_filenames)miserc

Filenames that mise reads as `.tool-versions` files instead of `.tool-versions`.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_OVERRIDE_TOOL_VERSIONS_FILENAMES` (colon-separated)
- Set in: a [`.miserc.toml`](https://mise.jdx.dev/configuration.html#miserc) file or `MISE_OVERRIDE_TOOL_VERSIONS_FILENAMES`. mise reads it before config files load, so `[settings]` in `mise.toml` has no effect. See [settings read before config files](https://mise.jdx.dev/configuration/settings.html#early-initialization).

Set it to `none` to stop reading `.tool-versions` files:

sh

```
export MISE_OVERRIDE_TOOL_VERSIONS_FILENAMES=none
```

### `packslip`[Permalink to packslip](#packslip)

#### `packslip.exec`[Permalink to packslip.exec](#packslip.exec)

Let packslip tools run their own executable at install time to generate resources, such as an agent skill.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_PACKSLIP_EXEC`

Some releases offer a resource only as a command, for example one that prints a skill's `SKILL.md`. With this on, mise runs the newly installed executable during installation to generate it. Resources published as files do not need this setting.

Shell completions are separate: mise may run a declared completion command when the shell asks for completions, or for `mise completion --tool`, whatever this is set to. See [When mise runs a publisher's command](https://mise.jdx.dev/dev-tools/packslip-resources.html#resource-selection-and-command-execution).

#### `packslip.stampers`[Permalink to packslip.stampers](#packslip.stampers)

Hosts that must approve a packslip release before mise lists or installs it, each as `host=PIN`.

- Type: `string[]`
- Default: unset
- Environment variable: `MISE_PACKSLIP_STAMPERS` (comma-separated)

A stamper is a registry, mirror or review service that publishes a signed list of the releases it approves. When this is set, a version needs an approval from at least one of these hosts. `PIN` is the minisign public key the host signs with, the path of its `.pub` file, or, for a host that signs keylessly from GitHub, an identity prefix:

Replace the example hosts and key path with a service and pin you trust. A stamp never replaces the vendor's signature, a vendor withdrawal still excludes the version, and a tool with the `trust = "vendor"` option needs no stamp. See [Stamps](https://mise.jdx.dev/dev-tools/packslip-verification.html#stamps).

### `paranoid`[Permalink to paranoid](#paranoid)global only

Turn on paranoid mode: `true` for every protection, or a list of categories (`config`, `env_files`, `plugins`, `provenance`) for only those.

- Type: `boolean | string[]`
- Default: `false`
- Environment variable: `MISE_PARANOID`
- Set in: global or system config, or `MISE_PARANOID`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).
- Choices:
  - `config`: content-bound trust for project config, no automatic or CI trust
  - `env_files`: content-bound trust for env files and sourced scripts
  - `plugins`: refuse unregistered community plugins
  - `provenance`: re-verify provenance on every install

`true` turns on every part of [paranoid mode](https://mise.jdx.dev/paranoid.html). A list turns on only the categories it names, and `false` turns them all off:

- `config`: every config file outside global and system config needs your approval, which is tied to the file's contents, so any edit needs approval again. `.miserc.toml` files and configs under [`trusted_config_paths`](#trusted_config_paths) still load without it. Automatic trust, CI trust and `--yes` for the trust prompt are turned off, and trust is not shared between worktrees or through a monorepo marker.
- `env_files`: files loaded by `env._.file`, `env._.source` and [`env_file`](#env_file) need approval tied to their contents.
- `plugins`: mise refuses to install an unregistered community plugin by short name.
- `provenance`: [`locked_verify_provenance`](#locked_verify_provenance) is turned on, and a "no attestations" answer from mise-versions is confirmed with GitHub.

From the environment, `MISE_PARANOID=1` selects every category and `MISE_PARANOID=config,env_files` selects those listed. An unknown category is an error. See [Paranoid mode](https://mise.jdx.dev/paranoid.html).

### `pin`[Permalink to pin](#pin)

Make `mise use` and `mise upgrade --bump` write the resolved version, such as `24.14.0` for `node@24`, instead of the requested one.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_PIN`

Same as passing `--pin` every time; `--fuzzy` writes the requested version for one command. `mise upgrade --bump` also writes resolved versions, replacing requests such as `latest` or `20` in `mise.toml`. To record exact versions without changing `mise.toml`, use a [lockfile](https://mise.jdx.dev/dev-tools/mise-lock.html).

### `prefer_offline`[Permalink to prefer\_offline](#prefer_offline)

Use cached version lists instead of fetching new ones, and go to the network only when nothing is cached.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_PREFER_OFFLINE`

Unlike [`offline`](#offline), network requests stay possible: when nothing cached answers a request, mise fetches once, with a short timeout and no retries. The [versions host](#use_versions_host) is skipped. mise turns this on by itself for shims and for `mise activate`, `mise hook-env`, `mise env`, `mise exec`, `mise direnv`, `mise ls`, `mise current`, `mise where` and `mise which`.

### `prereleases`[Permalink to prereleases](#prereleases)

Include prereleases in `mise ls-remote`, `latest` and prefix matching for every tool.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_PRERELEASES`

Same as setting the `prerelease = true` tool option on every tool. Every backend supports that option except the core `java` plugin, which selects early-access builds with `release_type = "ea"` instead. `mise ls-remote --prerelease` does this for one command.

Without it, mise skips prereleases when it resolves `latest` or a prefix such as `node@24`. A version counts as a prerelease when its source flags it, as GitHub and Forgejo releases, npm and .NET do, or when it looks like one, such as `1.0.0-rc1` or `2.0.0-beta.2`. `mise ls-remote` leaves out flagged versions; backends whose source has no flag, such as cargo, gem, go, http, s3, spm, vfox and asdf, flag versions that look like prereleases. Other sources, such as aqua's `github_tag` version source, list every tag there. Draft releases are always excluded.

### `provenance_api_failures_fatal`[Permalink to provenance\_api\_failures\_fatal](#provenance_api_failures_fatal)

Fail an install when a provenance API, such as GitHub's attestation API, cannot be reached or queried.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_PROVENANCE_API_FAILURES_FATAL`

Applies to GitHub artifact attestation lookups for aqua and github backend tools, to SLSA provenance lookups for github backend tools, and to the attestation lookups of [`npm.provenance`](#npm.provenance). Set to `false` to warn and install without that check, for example on a build network that cannot reach the GitHub API. Checksum failures, provenance that fails verification, and provenance that the lockfile already requires for the tool still fail the install.

### `prune`[Permalink to prune](#prune)

#### `prune.exclude`[Permalink to prune.exclude](#prune.exclude)

Tools that `mise prune` never removes, such as `node` or `aqua:BurntSushi/ripgrep`.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_PRUNE_EXCLUDE` (comma-separated)

`mise prune`, `mise ls --prunable` and the automatic removal after `mise upgrade` keep every version of these tools, even when no tracked config uses them and even when the tool is named on the command line. Name tools as elsewhere in mise: `node`, `core:node` and `aqua:BurntSushi/ripgrep` all work, and a short name matches the same tool as its full name. `mise uninstall` is unaffected.

Use it for a tool that something outside mise reaches through its install path, or whose old versions you keep on purpose:

### `pypi`[Permalink to pypi](#pypi)

#### `pypi.keyring_provider`[Permalink to pypi.keyring\_provider](#pypi.keyring_provider)

Read a `pypi:` index password from a `keyring` command when the index URL names a user but no password.

- Type: `string`
- Default: unset
- Environment variable: `MISE_PYPI_KEYRING_PROVIDER`
- Choices:
  - `disabled`
  - `subprocess`

Set to `subprocess` to have mise run `keyring get <index-url> <username>` when [`pypi.registry_url`](#pypi.registry_url) (or a tool's `registry_url`) contains a username but no password, such as an Azure Artifacts feed written as `https://VssSessionToken@pkgs.dev.azure.com/.../simple/{}/`. The printed password is sent as HTTP Basic credentials when mise lists versions and locks. This is the same value and lookup uv's `keyring-provider = "subprocess"` uses, so `mise lock` reaches the index the way `mise install` does through uv. mise also sets `UV_KEYRING_PROVIDER=subprocess` for the `uv lock` it runs, which ignores `uv.toml`. The `keyring` executable must be on `PATH`; if it is missing or has no entry, the request goes out without credentials. The password is never written to the lockfile or the version cache. See [Private indexes](https://mise.jdx.dev/dev-tools/backends/pypi.html#private-indexes).

#### `pypi.registry_url`[Permalink to pypi.registry\_url](#pypi.registry_url)

Package index URL that `pypi:` tools list versions from, with `{}` in place of the package name.

- Type: `string`
- Default: `"https://pypi.org/pypi/{}/json"`
- Environment variable: `MISE_PYPI_REGISTRY_URL`

Use a JSON API URL such as the default, or a simple index such as `https://pypi.org/simple/{}/`. A simple index is asked for its PEP 691 JSON form and falls back to PEP 503 HTML. [`minimum_release_age`](https://mise.jdx.dev/configuration/settings.html#minimum_release_age) works when the index supplies PEP 700 `upload-time`; HTML responses have no timestamps. The URL must start with `http://` or `https://` and contain `{}`. mise also derives the index's simple URL from it and passes that to installs, as `UV_INDEX` for uv or `PIP_INDEX_URL` for pipx. A tool's [`registry_url`](https://mise.jdx.dev/dev-tools/backends/pypi.html#registry-url) option overrides it for that tool's version list and dependency locking. `pipx.registry_url` is an older name for this setting. See [Private indexes](https://mise.jdx.dev/dev-tools/backends/pypi.html#private-indexes).

#### `pypi.uvx`[Permalink to pypi.uvx](#pypi.uvx)

Install `pypi:` tools with uv when uv is available, and with pipx otherwise.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_PYPI_UVX`

Set to `false` to install every `pypi:` tool with pipx. The uv-only tool options `with`, `expose` and `dependency_prereleases`, and dependency-graph locking, are then unavailable. One tool can opt out with the tool option `uvx = false`, but `uvx = true` cannot turn uv back on while this is `false`. `pipx.uvx` is an older name for this setting. See [Using pipx instead of uv](https://mise.jdx.dev/dev-tools/backends/pypi.html#using-pipx).

### `python`[Permalink to python](#python)

#### `python.compile`[Permalink to python.compile](#python.compile)

Compile Python from source with python-build instead of using a precompiled build.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_PYTHON_COMPILE`

`false` uses only precompiled builds: installs fail when none exists, and `mise ls-remote` lists only precompiled CPython builds and PyPy releases. When unset, [`all_compile`](https://mise.jdx.dev/configuration/settings.html#all_compile) decides; if that is unset too, mise uses a precompiled build when one exists and compiles otherwise. Has no effect on Windows, which always uses precompiled builds. See [How mise installs Python](https://mise.jdx.dev/lang/python.html#precompiled-python-binaries).

#### `python.default_packages_file`[Permalink to python.default\_packages\_file](#python.default_packages_file)deprecated

Requirements file that mise installs with pip into each new Python version.

- Type: `path`
- Default: `"~/.default-python-packages"`
- Environment variable: `MISE_PYTHON_DEFAULT_PACKAGES_FILE`
- Deprecated: Default python package files are deprecated. Use tool-level postinstall hooks for packages that should be installed into every python version, or use the pypi: backend for CLI tools.

mise runs `pip install --upgrade -r` with this file. See [Default packages file](https://mise.jdx.dev/lang/python.html#default-python-packages).

#### `python.github_attestations`[Permalink to python.github\_attestations](#python.github_attestations)

Verify GitHub artifact attestations of precompiled Python builds from python-build-standalone.

- Type: `boolean`
- Default: value of [`github_attestations`](https://mise.jdx.dev/configuration/settings.html#github_attestations)
- Environment variable: `MISE_PYTHON_GITHUB_ATTESTATIONS`

#### `python.patch_url`[Permalink to python.patch\_url](#python.patch_url)

URL of a patch that python-build applies to every Python version it compiles.

- Type: `URL`
- Default: unset
- Environment variable: `MISE_PYTHON_PATCH_URL`

Precompiled builds are not patched. To patch each version separately, use [`python.patches_directory`](#python.patches_directory).

#### `python.patches_directory`[Permalink to python.patches\_directory](#python.patches_directory)

Directory of per-version patches for the Python versions that python-build compiles.

- Type: `path`
- Default: unset
- Environment variable: `MISE_PYTHON_PATCHES_DIRECTORY`

When compiling Python, mise passes `<version>.patch` from this directory to python-build, for example `3.14.2.patch`. If that file does not exist, mise warns and builds without a patch.

#### `python.precompiled_arch`[Permalink to python.precompiled\_arch](#python.precompiled_arch)

Architecture of the python-build-standalone build that mise downloads, such as `x86_64_v3` or `aarch64`.

- Type: `string`
- Default: host architecture: `x86_64` or `aarch64`
- Environment variable: `MISE_PYTHON_PRECOMPILED_ARCH`

On x86-64 Linux, mise picks the build that matches the CPU features mise itself was compiled for, which for official mise releases is the baseline `x86_64` build. Set `x86_64_v3` to use the faster build on a CPU with AVX2, or `x86_64_v2` or `x86_64_v4` for other CPU levels. Give only the architecture; set the OS with [`python.precompiled_os`](#python.precompiled_os).

#### `python.precompiled_flavor`[Permalink to python.precompiled\_flavor](#python.precompiled_flavor)

Build flavor of python-build-standalone to download, such as `install_only` or `freethreaded-install_only_stripped`.

- Type: `string`
- Default: `"install_only_stripped"`
- Environment variable: `MISE_PYTHON_PRECOMPILED_FLAVOR`

Flavors are the last part of the asset names in the [python-build-standalone releases](https://github.com/astral-sh/python-build-standalone/releases). When unset, mise prefers `install_only_stripped`, then `install_only`, and never picks a free-threaded build. Only flavors of `.tar.gz` assets match, such as `install_only` and `freethreaded-install_only_stripped`; the `-full` flavors, published as `.tar.zst`, do not. See [Free-threaded Python](https://mise.jdx.dev/lang/python.html#free-threaded-python).

#### `python.precompiled_os`[Permalink to python.precompiled\_os](#python.precompiled_os)

OS part of the python-build-standalone build name that mise downloads, such as `unknown-linux-musl`.

- Type: `string`
- Default: host OS: `apple-darwin`, `pc-windows-msvc`, `unknown-linux-gnu` or `unknown-linux-musl`
- Environment variable: `MISE_PYTHON_PRECOMPILED_OS`

Set it, with [`python.precompiled_arch`](#python.precompiled_arch) if needed, to download a build for a platform other than the one mise detects, such as `unknown-linux-musl` on a glibc host.

#### `python.pyenv_repo`[Permalink to python.pyenv\_repo](#python.pyenv_repo)

Git repository that mise clones pyenv from to get python-build.

- Type: `string`
- Default: `"https://github.com/pyenv/pyenv.git"`
- Environment variable: `MISE_PYENV_REPO`

#### `python.uv_venv_auto`[Permalink to python.uv\_venv\_auto](#python.uv_venv_auto)

Activate, and optionally create, the virtualenv of the uv project around the current directory.

- Type: `boolean | string`
- Default: `false`
- Environment variable: `MISE_PYTHON_UV_VENV_AUTO`
- Choices:
  - `false`: off
  - `source`: activate an existing virtualenv
  - `create|source`: create the virtualenv with uv when it is missing, then activate it
  - `true`: deprecated; like `create|source`, and also exports `UV_PYTHON` set to the Python version

mise finds the project by walking up to the nearest `uv.lock`; without one, this does nothing. The environment is `.venv` next to `uv.lock`, or the path in `UV_PROJECT_ENVIRONMENT`. See [uv projects](https://mise.jdx.dev/lang/python.html#uv-projects).

`true` warns since mise 2026.7.0 and is removed in mise 2027.7.0. Use `"create|source"` or `"source"` instead.

#### `python.uv_venv_create_args`[Permalink to python.uv\_venv\_create\_args](#python.uv_venv_create_args)

Arguments for `uv venv` when mise creates a virtualenv with uv.

- Type: `string[]`
- Default: unset
- Environment variable: `MISE_PYTHON_UV_VENV_CREATE_ARGS` (colon-separated)

Used by [`_.python.venv`](https://mise.jdx.dev/lang/python.html#python-venv) when it sets no `uv_create_args`, and by [`python.uv_venv_auto`](#python.uv_venv_auto) when it creates a virtualenv.

#### `python.venv_create_args`[Permalink to python.venv\_create\_args](#python.venv_create_args)

Arguments for `python -m venv` when `_.python.venv` creates a virtualenv without uv.

- Type: `string[]`
- Default: unset
- Environment variable: `MISE_PYTHON_VENV_CREATE_ARGS` (colon-separated)

A [`_.python.venv`](https://mise.jdx.dev/lang/python.html#python-venv) directive's own `python_create_args` takes precedence.

#### `python.venv_stdlib`[Permalink to python.venv\_stdlib](#python.venv_stdlib)

Create `_.python.venv` virtualenvs with `python -m venv` even when uv is installed.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_VENV_STDLIB`

[`python.uv_venv_auto`](#python.uv_venv_auto) always uses uv.

### `quiet`[Permalink to quiet](#quiet)

Hide mise's own non-error messages, such as progress and task headers; same as `--quiet`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_QUIET`

Sets the log level to `error`. Output from the commands mise runs, such as tasks, still appears; [`silent`](#silent) hides that too. [`verbose`](#verbose) takes precedence over this setting.

### `raw`[Permalink to raw](#raw)

Connect tasks and install commands directly to the terminal's stdin, stdout and stderr.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_RAW`

Use it when a task or plugin script needs to prompt for input. mise does not read the output, so it cannot label or redact it. Only one such command runs at a time, so tasks run one after another whatever [`jobs`](#jobs) is set to. Same as `--raw`.

### `registry_cache_ttl`[Permalink to registry\_cache\_ttl](#registry_cache_ttl)

How long a downloaded mise registry stays fresh when `registry_floating` is on.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"1h"`
- Environment variable: `MISE_REGISTRY_CACHE_TTL`

Set to `0s` to download it again on every command that may use the network.

### `registry_floating`[Permalink to registry\_floating](#registry_floating)

Use the latest released mise registry and the current aqua registry, not only the copies built into mise.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_REGISTRY_FLOATING`

The built-in copies remain the fallback when a download fails. Each mise release is tested with its built-in registries, so this is off by default; turn it on when your mise package lags behind registry changes, for example on a distribution that updates mise slowly.

mise downloads the mise registry only for commands that may use the network, and keeps it for [`registry_cache_ttl`](#registry_cache_ttl). The aqua registry follows [`aqua.registry_cache_ttl`](https://mise.jdx.dev/configuration/settings.html#aqua.registry_cache_ttl).

### `ruby`[Permalink to ruby](#ruby)

#### `ruby.apply_patches`[Permalink to ruby.apply\_patches](#ruby.apply_patches)global only

Patch files or URLs to apply to the Ruby source before compiling it, one per line.

- Type: `string`
- Default: unset
- Environment variable: `MISE_RUBY_APPLY_PATCHES`
- Set in: global or system config, or `MISE_RUBY_APPLY_PATCHES`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

mise applies them through ruby-build, or ruby-install when [`ruby.ruby_install`](#ruby.ruby_install) is on. Precompiled builds are not patched.

#### `ruby.compile`[Permalink to ruby.compile](#ruby.compile)

Compile Ruby from source instead of using a precompiled build.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_RUBY_COMPILE`

`false` uses only precompiled builds: installs fail when none exists, and `mise ls-remote` and prefixes consider only versions that have one. When unset, [`all_compile`](https://mise.jdx.dev/configuration/settings.html#all_compile) decides; if that is unset too, mise uses a precompiled build when one exists and compiles otherwise. Has no effect on Windows, where mise installs RubyInstaller2 builds. See [Precompiled binaries](https://mise.jdx.dev/lang/ruby.html#precompiled-binaries).

#### `ruby.default_packages_file`[Permalink to ruby.default\_packages\_file](#ruby.default_packages_file)deprecated

File listing gems to install into each new Ruby version.

- Type: `string`
- Default: `"~/.default-gems"`
- Environment variable: `MISE_RUBY_DEFAULT_PACKAGES_FILE`
- Deprecated: Default ruby gem files are deprecated. Use tool-level postinstall hooks for gems that should be installed into every ruby version, or use the gem: backend for CLI tools.

Each line names a gem, optionally followed by a version constraint or `--pre`. See [Default gems file](https://mise.jdx.dev/lang/ruby.html#default-gems).

#### `ruby.github_attestations`[Permalink to ruby.github\_attestations](#ruby.github_attestations)

Verify GitHub artifact attestations of precompiled Ruby builds.

- Type: `boolean`
- Default: value of [`github_attestations`](https://mise.jdx.dev/configuration/settings.html#github_attestations)
- Environment variable: `MISE_RUBY_GITHUB_ATTESTATIONS`

Applies when [`ruby.precompiled_url`](#ruby.precompiled_url) names a GitHub repository, such as the default `jdx/ruby`. Builds from a URL template are not checked.

#### `ruby.precompiled_arch`[Permalink to ruby.precompiled\_arch](#ruby.precompiled_arch)

Architecture name of precompiled Ruby builds, such as `x86_64` or `arm64`, used with `ruby.precompiled_os`.

- Type: `string`
- Default: unset
- Environment variable: `MISE_RUBY_PRECOMPILED_ARCH`

mise uses it only when [`ruby.precompiled_os`](#ruby.precompiled_os) is also set, and then looks for `<arch>_<os>` builds instead of detecting the platform. Set both to use builds you host with [`ruby.precompiled_url`](#ruby.precompiled_url) for a platform jdx/ruby does not publish, such as musl Linux. See [Precompiled binaries](https://mise.jdx.dev/lang/ruby.html#precompiled-binaries).

#### `ruby.precompiled_os`[Permalink to ruby.precompiled\_os](#ruby.precompiled_os)

OS name of precompiled Ruby builds, such as `linux`, used with `ruby.precompiled_arch`.

- Type: `string`
- Default: unset
- Environment variable: `MISE_RUBY_PRECOMPILED_OS`

See [`ruby.precompiled_arch`](#ruby.precompiled_arch).

#### `ruby.precompiled_url`[Permalink to ruby.precompiled\_url](#ruby.precompiled_url)

GitHub repository, as `owner/repo`, or URL template that mise downloads precompiled Ruby builds from.

- Type: `string`
- Default: `"jdx/ruby"`
- Environment variable: `MISE_RUBY_PRECOMPILED_URL`

A repository must publish releases with assets named like jdx/ruby's. A URL template can use `{version}`, `{platform}`, `{arch}` and `{os}`. `{platform}` is `macos`, `arm64_linux` or `x86_64_linux`, or `<arch>_<os>` from [`ruby.precompiled_arch`](#ruby.precompiled_arch) and [`ruby.precompiled_os`](#ruby.precompiled_os). `{arch}` and `{os}` are the parts of `{platform}` before and after its last `_`, such as `x86_64` and `linux`; on macOS they are `arm64` and `macos`. When both of those settings are set, `{arch}` and `{os}` are their values as given.

With a URL template, mise cannot list which versions have builds, and it does not verify GitHub artifact attestations.

#### `ruby.ruby_build_cli_opts`[Permalink to ruby.ruby\_build\_cli\_opts](#ruby.ruby_build_cli_opts)

Options for ruby-build itself, such as `--keep`, passed before the version and install directory.

- Type: `string`
- Default: unset
- Environment variable: `MISE_RUBY_BUILD_CLI_OPTS`

#### `ruby.ruby_build_opts`[Permalink to ruby.ruby\_build\_opts](#ruby.ruby_build_opts)

Arguments for Ruby's `configure`, such as `--enable-yjit`, passed through ruby-build.

- Type: `string`
- Default: unset
- Environment variable: `MISE_RUBY_BUILD_OPTS`

mise passes them after ruby-build's `--` separator.

#### `ruby.ruby_build_repo`[Permalink to ruby.ruby\_build\_repo](#ruby.ruby_build_repo)

Git repository or ZIP archive URL that mise gets ruby-build from.

- Type: `string`
- Default: `"https://github.com/rbenv/ruby-build.git"`
- Environment variable: `MISE_RUBY_BUILD_REPO`

#### `ruby.ruby_install`[Permalink to ruby.ruby\_install](#ruby.ruby_install)

Compile Ruby with ruby-install instead of ruby-build.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_RUBY_INSTALL`

#### `ruby.ruby_install_opts`[Permalink to ruby.ruby\_install\_opts](#ruby.ruby_install_opts)

Arguments for Ruby's `configure`, passed through ruby-install.

- Type: `string`
- Default: unset
- Environment variable: `MISE_RUBY_INSTALL_OPTS`

mise passes them after ruby-install's `--` separator.

#### `ruby.ruby_install_repo`[Permalink to ruby.ruby\_install\_repo](#ruby.ruby_install_repo)

Git repository or ZIP archive URL that mise gets ruby-install from.

- Type: `string`
- Default: `"https://github.com/postmodern/ruby-install.git"`
- Environment variable: `MISE_RUBY_INSTALL_REPO`

#### `ruby.verbose_install`[Permalink to ruby.verbose\_install](#ruby.verbose_install)

Show ruby-build's full output while compiling Ruby.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_RUBY_VERBOSE_INSTALL`

When unset, it follows [`verbose`](https://mise.jdx.dev/configuration/settings.html#verbose).

### `rust`[Permalink to rust](#rust)

#### `rust.cargo_home`[Permalink to rust.cargo\_home](#rust.cargo_home)

Directory used as `CARGO_HOME` for the Rust toolchains that mise manages.

- Type: `path`
- Default: `$CARGO_HOME`, otherwise `~/.cargo`
- Environment variable: `MISE_CARGO_HOME`

Set it with [`rust.rustup_home`](#rust.rustup_home) to keep mise's toolchains apart from another rustup installation. It takes precedence over a `CARGO_HOME` in your environment, but a `CARGO_HOME` set in `[env]` takes precedence over it. See [Cargo, rustup and installed binaries](https://mise.jdx.dev/lang/rust.html#where-rustup-and-cargo-live).

#### `rust.default_host`[Permalink to rust.default\_host](#rust.default_host)

Host triple that mise passes to `rustup init` as `--default-host`.

- Type: `string`
- Default: unset
- Environment variable: `MISE_RUST_DEFAULT_HOST`

#### `rust.rustup_home`[Permalink to rust.rustup\_home](#rust.rustup_home)

Directory used as `RUSTUP_HOME` for the Rust toolchains that mise manages.

- Type: `path`
- Default: `$RUSTUP_HOME`, otherwise `~/.rustup`
- Environment variable: `MISE_RUSTUP_HOME`

Set it with [`rust.cargo_home`](#rust.cargo_home) to keep mise's toolchains apart from another rustup installation. It takes precedence over a `RUSTUP_HOME` in your environment, but a `RUSTUP_HOME` set in `[env]` takes precedence over it. See [Cargo, rustup and installed binaries](https://mise.jdx.dev/lang/rust.html#where-rustup-and-cargo-live).

### `safe`[Permalink to safe](#safe)global only

Load project config without letting it run code or change the environment, for automation that reads untrusted config.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SAFE`
- Set in: global or system config, or `MISE_SAFE`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Use it for bots and CI jobs that process config you did not write, such as a bot that runs `mise lock --bump` on pull request branches. Project config then loads without a trust prompt.

Safe mode refuses template `exec()`, `read_file()` and `get_env()`, tasks, the `postinstall`, `install_env` and spm `install_command` tool options, asdf plugin scripts, S3 requests, plugin installs, mise secrets and `mise daemons`, with an error. It skips hooks, as `--no-hooks` does, and ignores project `[env]`, `[shell_alias]`, `[settings]`, `[deps]`, lazy tools, `path:` versions, remote includes, `[bootstrap]` and `[dotfiles]`, so a repository cannot turn off verification, redirect a backend, run its dependency providers or write files into your home directory. Project templates see an empty `env`, the hosts a project `api_url` or download URL names get none of your forge tokens or netrc credentials, and a project `[tool_alias]` cannot redirect a tool global config declares. Global and system config still apply. Version resolution and `mise.lock` updates keep working for backends that query HTTP APIs, such as core tools, `aqua:`, `github:`, `pypi:` and `npm:`.

See [Safe mode](https://mise.jdx.dev/security.html#safe-mode) for the complete lists and what safe mode does not protect against.

### `sandbox`[Permalink to sandbox](#sandbox)

#### `sandbox.deny_all`[Permalink to sandbox.deny\_all](#sandbox.deny_all)

Deny filesystem access, network access and inherited environment variables by default for `mise run` and `mise exec`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SANDBOX_DENY_ALL`

Tasks and the `--allow-*` flags can still add exceptions. Windows does not enforce filesystem or network rules and filters the environment only partly; mise warns and runs the command anyway. See [Sandbox every command](https://mise.jdx.dev/sandboxing.html#default-restrictions).

#### `sandbox.deny_env`[Permalink to sandbox.deny\_env](#sandbox.deny_env)

Deny environment variable inheritance by default for `mise run` and `mise exec`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SANDBOX_DENY_ENV`

Tasks and the `--allow-env` flag can still let variables through. On Windows, the command still inherits mise's own environment. See [Sandbox every command](https://mise.jdx.dev/sandboxing.html#default-restrictions).

#### `sandbox.deny_net`[Permalink to sandbox.deny\_net](#sandbox.deny_net)

Deny network access by default for `mise run` and `mise exec`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SANDBOX_DENY_NET`

Tasks and flags cannot make exceptions for particular hosts; see [access to particular hosts](https://mise.jdx.dev/sandboxing.html#access-to-particular-hosts). Windows does not enforce it; mise warns and runs the command anyway. See [Sandbox every command](https://mise.jdx.dev/sandboxing.html#default-restrictions).

#### `sandbox.deny_read`[Permalink to sandbox.deny\_read](#sandbox.deny_read)

Deny filesystem reads by default for `mise run` and `mise exec`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SANDBOX_DENY_READ`

Tasks and the `--allow-read` flag can still add exceptions. Windows does not enforce it; mise warns and runs the command anyway. See [Sandbox every command](https://mise.jdx.dev/sandboxing.html#default-restrictions).

#### `sandbox.deny_write`[Permalink to sandbox.deny\_write](#sandbox.deny_write)

Deny filesystem writes by default for `mise run` and `mise exec`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SANDBOX_DENY_WRITE`

Tasks and the `--allow-write` flag can still add exceptions. Windows does not enforce it; mise warns and runs the command anyway. See [Sandbox every command](https://mise.jdx.dev/sandboxing.html#default-restrictions).

### `self_update`[Permalink to self\_update](#self_update)

#### `self_update.api_url`[Permalink to self\_update.api\_url](#self_update.api_url)global only

GitHub API base URL that `mise self-update` gets mise releases from.

- Type: `URL`
- Default: `"https://api.github.com"`
- Environment variable: `MISE_SELF_UPDATE_API_URL`
- Set in: global or system config, or `MISE_SELF_UPDATE_API_URL`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

It must use HTTPS. For GitHub Enterprise, include the API path, such as `https://github.example.com/api/v3`. Use it with [`self_update.repository`](#self_update.repository) to serve updates from a mirror. See [Updating mise](https://mise.jdx.dev/installing-mise.html#updating).

#### `self_update.auto`[Permalink to self\_update.auto](#self_update.auto)global only

Update mise automatically before running eligible commands.

- Type: `boolean | string`
- Default: `false`
- Environment variable: `MISE_SELF_UPDATE_AUTO`
- Set in: global or system config, or `MISE_SELF_UPDATE_AUTO`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

With `true`, mise checks for a newer release every [`self_update.check_duration`](#self_update.check_duration), before commands that may use the network; a duration such as `"1d"` sets the interval instead. When one is available, it installs it without updating plugins, then runs the original command again with the new binary. Automatic updates are skipped in CI, in offline and prefer-offline modes, in non-interactive sessions, for shell activation commands, and for installations whose packager disabled self-update.

See [Updating mise](https://mise.jdx.dev/installing-mise.html#updating).

#### `self_update.check_duration`[Permalink to self\_update.check\_duration](#self_update.check_duration)global only

How often automatic updates check for a new mise release when `self_update.auto = true`.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"7d"`
- Environment variable: `MISE_SELF_UPDATE_CHECK_DURATION`
- Set in: global or system config, or `MISE_SELF_UPDATE_CHECK_DURATION`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

#### `self_update.minimum_release_age`[Permalink to self\_update.minimum\_release\_age](#self_update.minimum_release_age)

Skip mise releases published more recently than this, for `mise self-update` and automatic updates.

- Type: `string`
- Default: value of [`minimum_release_age`](https://mise.jdx.dev/configuration/settings.html#minimum_release_age), else `24h`
- Environment variable: `MISE_SELF_UPDATE_MINIMUM_RELEASE_AGE`

Takes precedence over [`minimum_release_age`](https://mise.jdx.dev/configuration/settings.html#minimum_release_age) for `mise self-update`, automatic updates and update notifications, and accepts the same durations and cutoff dates. `0s` turns the delay off. A version named on the command line, such as `mise self-update 2026.10.4`, skips the delay; `--force` does not. `mise self-update --minimum-release-age` overrides this setting.

The install script reads `MISE_SELF_UPDATE_MINIMUM_RELEASE_AGE` instead of this setting; see [Installer options](https://mise.jdx.dev/installing-mise.html#installer-options).

#### `self_update.repository`[Permalink to self\_update.repository](#self_update.repository)global only

GitHub repository, as `owner/repo`, that mise gets its own releases from.

- Type: `string`
- Default: `"jdx/mise"`
- Environment variable: `MISE_SELF_UPDATE_REPOSITORY`
- Set in: global or system config, or `MISE_SELF_UPDATE_REPOSITORY`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Used by `mise self-update`, automatic updates and update notifications. Point it at a private mirror that holds only the releases your organization approves; the mirror must keep the official archive names and embedded signatures. mise authenticates with its usual [GitHub token](https://mise.jdx.dev/dev-tools/github-tokens.html) sources. See [Updating mise](https://mise.jdx.dev/installing-mise.html#updating).

### `shared_install_dirs`[Permalink to shared\_install\_dirs](#shared_install_dirs)

Extra read-only directories that mise searches for installed tool versions.

- Type: `path[]`
- Default: unset
- Environment variable: `MISE_SHARED_INSTALL_DIRS` (separated by `:` ( `;` on Windows))

mise searches these after its own installs directory and never installs into them on its own; `mise install --shared <DIR>` installs into one. Use them on shared machines, such as Docker images or bastion hosts, where a base set of tools is preinstalled and users add their own. The system installs directory, [`system_installs_dir`](#system_installs_dir), is always searched when it exists and does not need to be listed. See [Other shared directories](https://mise.jdx.dev/dev-tools/system-installs.html#other-shared-directories).

### `shims`[Permalink to shims](#shims)

#### `shims.exclude`[Permalink to shims.exclude](#shims.exclude)

Command names that mise never creates shims for, so they resolve to whatever else is on `PATH`.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_SHIMS_EXCLUDE` (comma-separated)

mise still installs and manages the tool, and the next `mise reshim` removes existing shims with these names. Version-qualified shims such as `python3.12` are still created:

Lazy-tool and plugin-provided shims skip these names too, but a [command wrapper](https://mise.jdx.dev/dev-tools/shims.html#command-wrappers) you declare with one of them still works. Only shims are affected: under `mise activate` without `--shims`, a tool's whole `bin` directory is on `PATH`, so excluded names are still found there. See [Exclude commands from shims](https://mise.jdx.dev/dev-tools/shims.html#excluding-command-names).

### `shims_dir`[Permalink to shims\_dir](#shims_dir)global only

Directory where mise writes user shims.

- Type: `path`
- Default: `$MISE_DATA_DIR/shims`
- Environment variable: `MISE_SHIMS_DIR`
- Set in: global or system config, or `MISE_SHIMS_DIR`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

The path must be absolute after `~` expansion. `mise reshim` can share a directory such as `~/.local/bin`, replacing or removing only files it recognizes as mise shims, but `mise activate` treats the whole directory as a shim directory, so use a dedicated one if you activate mise. Lazy tools declared in user config put their bootstrap shims here. See [Rebuild shims with mise reshim](https://mise.jdx.dev/dev-tools/shims.html#mise-reshim).

### `silent`[Permalink to silent](#silent)

Hide all output from `mise run` and `mise watch`, including what tasks print, except errors; same as `--silent`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SILENT`

### `skills`[Permalink to skills](#skills)

#### `skills.auto_sync`[Permalink to skills.auto\_sync](#skills.auto_sync)

Link the active tools' agent skills into the project after `mise install`, `mise use`, and `mise upgrade`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SKILLS_AUTO_SYNC`

mise runs the equivalent of `mise skills sync` after every install, after `mise use` changes a version, and after `mise upgrade`, so the links under [`skills.dir`](#skills.dir) point at the versions active in the project. Only projects with a mise config get links, and nothing is written outside a project root, except that global tools link into `skills.dir` under your home directory. A project's directory gets the tools its own config files declare and the tools `MISE_<TOOL>_VERSION` selects, but not the ones only the global config declares. When the project and home share one `skills.dir`, every active tool links there. mise touches only links it made, and removes stale ones only when [`skills.prune`](#skills.prune) is on. See [Keep project links up to date](https://mise.jdx.dev/dev-tools/packslip-resources.html#keep-project-links-up-to-date).

#### `skills.dir`[Permalink to skills.dir](#skills.dir)

Where `mise skills sync` links skills: a path under the project root, or under the home directory with `--global`.

- Type: `string`
- Default: unset
- Environment variable: `MISE_SKILLS_DIR`

Set it to the directory the agents you use read, such as `.claude/skills`. An absolute path is used as written, and `--dir` overrides it for one run.

Left unset, mise uses `.agents/skills` if it exists, then `.claude/skills` if it exists. When neither exists it creates `.agents/skills` and a `.claude/skills` symlink to it, so agents that read either path find the skills. See [Choose a skill directory](https://mise.jdx.dev/dev-tools/packslip-resources.html#choose-a-skill-directory).

#### `skills.fetch`[Permalink to skills.fetch](#skills.fetch)

Fetch the agent skills a tool's packslip declares when installing it.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_SKILLS_FETCH`

mise fetches them at install time, so each installed version carries its own copy. Turning this off skips fetching from then on; it does not delete skills already fetched or existing links. See [Skills](https://mise.jdx.dev/dev-tools/packslip-resources.html#skills).

#### `skills.prune`[Permalink to skills.prune](#skills.prune)

Remove links mise made for skills that are no longer active, on every `mise skills sync`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SKILLS_PRUNE`

Without it, `mise skills sync` keeps those links unless you pass `--prune`. With it, every sync prunes, including the automatic syncs from [`skills.auto_sync`](#skills.auto_sync). mise removes only links it made, which point into its installs directory.

### `slsa`[Permalink to slsa](#slsa)

Verify SLSA provenance of downloads whose publisher provides it.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_SLSA`

When `false`, the aqua and github backends skip SLSA checks whatever [`aqua.slsa`](https://mise.jdx.dev/configuration/settings.html#aqua.slsa) and [`github.slsa`](https://mise.jdx.dev/configuration/settings.html#github.slsa) say.

### `sops`[Permalink to sops](#sops)

#### `sops.age_key`[Permalink to sops.age\_key](#sops.age_key)

Private age key for decrypting SOPS files, used before any other key source.

- Type: `string`
- Default: unset
- Environment variable: `MISE_SOPS_AGE_KEY`

Prefer the `MISE_SOPS_AGE_KEY` environment variable to writing a private key into a config file. See [Where mise finds the age key](https://mise.jdx.dev/environments/secrets/sops.html#environment-variables).

#### `sops.age_key_file`[Permalink to sops.age\_key\_file](#sops.age_key_file)

Path of an age key file for decrypting SOPS files, used after `sops.age_key` and before `SOPS_AGE_KEY_FILE`.

- Type: `path`
- Default: unset
- Environment variable: `MISE_SOPS_AGE_KEY_FILE`

When neither this nor [`sops.age_key`](#sops.age_key) is set, mise tries `SOPS_AGE_KEY_FILE`, `SOPS_AGE_KEY` and then `~/.config/mise/age.txt`. See [Where mise finds the age key](https://mise.jdx.dev/environments/secrets/sops.html#environment-variables).

#### `sops.rops`[Permalink to sops.rops](#sops.rops)

Decrypt SOPS files with mise's built-in rops library instead of the `sops` CLI.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_SOPS_ROPS`

rops handles age keys. Set to `false` to run `sops decrypt` instead, for AWS KMS, GCP KMS, Azure Key Vault, HashiCorp Vault or PGP keys. The CLI is slower and cannot read TOML, so an encrypted `.env.toml` needs rops. See [Choose a decryption method](https://mise.jdx.dev/environments/secrets/sops.html#choose-a-decryption-method).

#### `sops.strict`[Permalink to sops.strict](#sops.strict)

Fail when a SOPS file cannot be decrypted, instead of skipping it.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_SOPS_STRICT`

Decryption fails when no age key is found, the key cannot decrypt the file, or the `sops` CLI is missing. Set to `false` to skip the file and continue.

### `spm`[Permalink to spm](#spm)

#### `spm.artifactbundle_only`[Permalink to spm.artifactbundle\_only](#spm.artifactbundle_only)

Install `spm:` tools only from prebuilt artifact bundles, and fail instead of building from source.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_SPM_ARTIFACTBUNDLE_ONLY`

A tool with the `artifactbundle = false` option, or a version that selects a Git revision, then fails to install. See [`artifactbundle`](https://mise.jdx.dev/dev-tools/backends/spm.html#artifactbundle).

### `status`[Permalink to status](#status)

#### `status.missing_tools`[Permalink to status.missing\_tools](#status.missing_tools)

When to warn that tools requested by config are not installed.

- Type: `string`
- Default: `"if_other_versions_installed"`
- Environment variable: `MISE_STATUS_MESSAGE_MISSING_TOOLS`
- Choices:
  - `never`: never warn
  - `if_other_versions_installed`: warn only when another version of the tool is installed
  - `always`: always warn

mise prints the warning, such as `missing: node@24.14.0`, from the shell hook of `mise activate` and from commands such as `mise exec` and `mise env`. Tools marked `lazy = true` are never reported.

#### `status.show_deps_stale`[Permalink to status.show\_deps\_stale](#status.show_deps_stale)

Warn in an activated shell when a `[deps]` provider with `auto = true` is out of date.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_STATUS_SHOW_DEPS_STALE`

The warning names each stale provider and the reason, such as `npm (package-lock.json changed)`. See [Staleness warnings](https://mise.jdx.dev/dev-tools/deps.html#staleness-warnings).

#### `status.show_env`[Permalink to status.show\_env](#status.show_env)

When you change directories in an activated shell, list the environment variables that mise adds, changes or removes.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_STATUS_MESSAGE_SHOW_ENV`

Names are shown with `+`, `~` or `-`, without values. Config files that start or stop applying are listed the same way.

#### `status.show_tools`[Permalink to status.show\_tools](#status.show_tools)

When you change directories in an activated shell, list the tool versions that become active or inactive.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_STATUS_MESSAGE_SHOW_TOOLS`

Versions are shown as `+node@24.14.0` or `-node@22.21.1`.

#### `status.truncate`[Permalink to status.truncate](#status.truncate)

Cut the lines from `status.show_env` and `status.show_tools` to the terminal width.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_STATUS_MESSAGE_TRUNCATE`

Setting [`truncate`](https://mise.jdx.dev/configuration/settings.html#truncate) to `false` turns this off too.

### `swift`[Permalink to swift](#swift)

#### `swift.gpg_verify`[Permalink to swift.gpg\_verify](#swift.gpg_verify)

Verify the OpenPGP signature of Swift downloads on Linux.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_SWIFT_GPG_VERIFY`

mise checks the signature itself, so `gpg` does not need to be installed. The global [`gpg_verify`](https://mise.jdx.dev/configuration/settings.html#gpg_verify) setting sets the default.

#### `swift.platform`[Permalink to swift.platform](#swift.platform)

Linux distribution build of Swift to download, such as `ubuntu24.04` or `fedora41`.

- Type: `string`
- Default: detected from your Linux distribution
- Environment variable: `MISE_SWIFT_PLATFORM`

By default, mise detects your distribution and picks the closest build that the Swift release publishes. Set this to force one build, or to install without fetching swift.org's release index. Ignored on macOS and Windows. See [Linux distributions](https://mise.jdx.dev/lang/swift.html#linux-distributions).

### `system_config_file`[Permalink to system\_config\_file](#system_config_file)env only

One file to use as the system config instead of the config files in `/etc/mise`.

- Type: `path`
- Default: unset
- Environment variable: `MISE_SYSTEM_CONFIG_FILE`
- Set in: `MISE_SYSTEM_CONFIG_FILE` only. mise reads it before config files load and ignores it in any config file. See [environment-only settings](https://mise.jdx.dev/configuration/settings.html#environment-only-settings).

When unset, mise reads `conf.d/*.toml`, `config.toml`, `mise.toml` and their environment and `.local` variants in `/etc/mise` (or `MISE_SYSTEM_CONFIG_DIR`), and writes such as `mise config set --system` go to `config.toml` there. When set, mise reads and writes only this file.

### `system_deps`[Permalink to system\_deps](#system_deps)

What mise does when a plugin's declared system dependencies are missing before it installs a tool.

- Type: `string`
- Default: `"prompt"`
- Environment variable: `MISE_SYSTEM_DEPS`
- Choices:
  - `prompt`: list what is missing and offer to install it with the first available package manager; without a terminal and without `--yes`, same as `warn`
  - `auto`: install what is missing without asking, using sudo as `system_packages.sudo` allows
  - `warn`: list what is missing and the commands to install it, then continue
  - `ignore`: skip the check

Dependencies that mise can find, however they were installed, are never reinstalled. Missing optional dependencies are reported in one line and never block an install, and the check never fails an install on its own. See [System dependencies](https://mise.jdx.dev/tool-plugin-development.html#system-dependencies).

### `system_installs_dir`[Permalink to system\_installs\_dir](#system_installs_dir)global only

Directory that `mise install --system` installs into.

- Type: `path`
- Default: `$MISE_SYSTEM_DATA_DIR/installs` ( `/usr/local/share/mise/installs`)
- Environment variable: `MISE_SYSTEM_INSTALLS_DIR`
- Set in: global or system config, or `MISE_SYSTEM_INSTALLS_DIR`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Lazy tools declared in the system config install here too. The path must be absolute after `~` expansion. See [System installs](https://mise.jdx.dev/dev-tools/system-installs.html#directories).

### `system_packages`[Permalink to system\_packages](#system_packages)

#### `system_packages.elevate_with`[Permalink to system\_packages.elevate\_with](#system_packages.elevate_with)global only

Command mise uses to get root instead of `sudo`, such as `run0` or `doas`.

- Type: `string`
- Default: unset
- Environment variable: `MISE_SYSTEM_PACKAGES_ELEVATE_WITH`
- Set in: global or system config, or `MISE_SYSTEM_PACKAGES_ELEVATE_WITH`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

When unset, mise uses the first of `sudo`, `run0` and `doas` found on `PATH`, so hosts without sudo, such as secureblue (`run0`) or OpenBSD-style setups (`doas`), work with no configuration. Set it to pin one:

The command is run as `<command> <program> <args>...`. Without a terminal, mise checks for passwordless sudo with `sudo -n true`, and runs the command with `doas -n` or `run0 --no-ask-password` so it fails instead of prompting; any other command is used only in an interactive terminal. A `doas` rule that restricts commands must also allow `env`, which mise uses to pass variables such as `DEBIAN_FRONTEND` to apt. [`system_packages.sudo`](#system_packages.sudo) still turns elevation off entirely.

Set it to an empty string to never elevate: mise runs package manager commands as your user, with no `sudo`, `run0` or `doas` in front. Use this where there is no root to get, such as Termux or a rootless container, and the package manager works without it. Unset still picks an elevator from `PATH`. See [Root privileges and sudo](https://mise.jdx.dev/bootstrap/packages/#sudo).

#### `system_packages.managers`[Permalink to system\_packages.managers](#system_packages.managers)

Package managers that mise may use for `[bootstrap.packages]` and missing plugin dependencies.

- Type: `string[]`
- Default: unset
- Environment variable: `MISE_SYSTEM_PACKAGES_MANAGERS` (comma-separated)

When unset, mise uses every manager in `[bootstrap.packages]` that is available on the machine. Names match built-in managers and package manager plugins alike:

Status lists packages of excluded managers as `skipped`, and naming an excluded manager with `--manager` is an error. For per-OS defaults, set it in a platform config file such as `mise.linux.toml`, which loads only when that environment is active, with `-E linux`, `MISE_ENV` or [`auto_env`](https://mise.jdx.dev/configuration/settings.html#auto_env). See [By package manager](https://mise.jdx.dev/bootstrap/packages/#choosing-which-managers-run).

#### `system_packages.ostree`[Permalink to system\_packages.ostree](#system_packages.ostree)

What mise does with `dnf:` packages on an image-based Fedora host such as Silverblue or Bazzite.

- Type: `string`
- Default: `"error"`
- Environment variable: `MISE_SYSTEM_PACKAGES_OSTREE`
- Choices:
  - `error`: stop and explain why the packages cannot be installed, and what to do instead
  - `layer`: layer missing packages with `rpm-ostree install`; they take effect after a reboot

On a host booted from an ostree image, `/usr` is read-only and `dnf install` fails. Packages are layered with `rpm-ostree install` instead and take effect on the next boot. Layering is discouraged on these systems, so mise does it only when you opt in. See [Fedora Atomic hosts](https://mise.jdx.dev/bootstrap/packages/dnf.html#fedora-atomic-hosts).

#### `system_packages.sudo`[Permalink to system\_packages.sudo](#system_packages.sudo)

Let mise run `sudo` for steps that need root, such as system packages and `mise install --system`.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_SYSTEM_PACKAGES_SUDO`

mise uses sudo only when it is not running as root, and logs each command it runs with sudo. An interactive session can prompt for your password; a noninteractive one needs sudo to work without a password.

In `mise bootstrap`, this covers every step that needs root: system packages, Linux accounts, system files and directories, system services, the firewall, and Compose projects with `sudo = true`. Some status checks also use sudo to read root-only state, such as `mise bootstrap firewall status`. For [`mise install --system`](https://mise.jdx.dev/dev-tools/system-installs.html#permissions-and-sudo) and `mise reshim --system`, mise prepares files as your user and uses sudo only to move the installation, its links and the system shims into protected system directories. Package manager plugins never use sudo.

Set to `false` to never run sudo. Steps that need root then fail unless mise already runs as root, and installs into directories you can write still work. See [Root privileges and sudo](https://mise.jdx.dev/bootstrap/packages/#sudo).

### `system_shims_dir`[Permalink to system\_shims\_dir](#system_shims_dir)global only

Directory where `mise reshim --system` writes shims for system installs.

- Type: `path`
- Default: `$MISE_SYSTEM_DATA_DIR/shims` ( `/usr/local/share/mise/shims`)
- Environment variable: `MISE_SYSTEM_SHIMS_DIR`
- Set in: global or system config, or `MISE_SYSTEM_SHIMS_DIR`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

It may be the same directory as [`shims_dir`](#shims_dir); mise then manages one combined set of shims. The path must be absolute after `~` expansion. See [Shims for system installs](https://mise.jdx.dev/dev-tools/system-installs.html#system-shims).

### `task`[Permalink to task](#task)

#### `task.auto_infer`[Permalink to task.auto\_infer](#task.auto_infer)

Workspace providers whose package scripts mise imports as tasks, such as `node`.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_TASK_AUTO_INFER` (comma-separated)

With `["node"]`, the `scripts` of every Node.js workspace package become tasks named like `node:@acme/web#build`. Requires a monorepo root. An explicit mise task at the same path replaces the imported one. See [Node.js package scripts](https://mise.jdx.dev/tasks/workspace-graph.html#node-package-scripts).

#### `task.cache.audit_report`[Permalink to task.cache.audit\_report](#task.cache.audit_report)experimental

File that receives every undeclared path found by the `cache.audit` task option.

- Type: `path`
- Default: unset
- Environment variable: `MISE_TASK_CACHE_AUDIT_REPORT`

mise writes one JSON object per line, with `task`, `kind` and `path` fields, while the console shows at most 20 paths per task. Each `mise` invocation that audits a task replaces the file with that run's findings; tasks restored from the cache do not run and add nothing. A relative path resolves against the directory mise runs in. See [Find undeclared inputs on Linux](https://mise.jdx.dev/tasks/caching.html#find-undeclared-inputs-on-linux).

#### `task.cache.remote_mode`[Permalink to task.cache.remote\_mode](#task.cache.remote_mode)experimental

How mise uses the remote task cache server.

- Type: `string`
- Default: `"read-write"`
- Environment variable: `MISE_TASK_CACHE_REMOTE_MODE`
- Choices:
  - `read-write`: read entries when the local cache misses, and upload new results
  - `read-only`: read entries without uploading
  - `write-only`: upload new results without reading entries

mise uploads results only from GitHub Actions and GitLab CI jobs that push to a protected branch. Other runs only read, whatever this is set to, and with `write-only` they do not use the server at all. `mise run --task-cache local-only` skips the server for one run. See [Who can write](https://mise.jdx.dev/tasks/remote-cache.html#who-can-write).

#### `task.cache.remote_namespace`[Permalink to task.cache.remote\_namespace](#task.cache.remote_namespace)experimental

Namespace that keeps this project's entries apart on the remote task cache server.

- Type: `string`
- Default: unset
- Environment variable: `MISE_TASK_CACHE_REMOTE_NAMESPACE`

Required when [`task.cache.remote_url`](#task.cache.remote_url) is set. The server also uses it to decide who may read and write. It is an identifier, such as a repository name, not a secret.

#### `task.cache.remote_oidc_audience`[Permalink to task.cache.remote\_oidc\_audience](#task.cache.remote_oidc_audience)experimentalglobal only

Audience of the GitHub Actions OIDC token that mise requests for the remote task cache.

- Type: `string`
- Default: unset
- Environment variable: `MISE_TASK_CACHE_REMOTE_OIDC_AUDIENCE`
- Set in: global or system config, or `MISE_TASK_CACHE_REMOTE_OIDC_AUDIENCE`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

The workflow needs the `id-token: write` permission. mise refreshes the token before it expires. [`task.cache.remote_token`](#task.cache.remote_token) and [`task.cache.remote_token_file`](#task.cache.remote_token_file) take precedence. See [GitHub Actions OIDC](https://mise.jdx.dev/tasks/remote-cache.html#github-actions-oidc).

#### `task.cache.remote_token`[Permalink to task.cache.remote\_token](#task.cache.remote_token)experimentalglobal only

Bearer token that mise sends to the remote task cache server.

- Type: `string`
- Default: unset
- Environment variable: `MISE_TASK_CACHE_REMOTE_TOKEN`
- Set in: global or system config, or `MISE_TASK_CACHE_REMOTE_TOKEN`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Prefer the `MISE_TASK_CACHE_REMOTE_TOKEN` environment variable to writing the token into a config file. It takes precedence over [`task.cache.remote_token_file`](#task.cache.remote_token_file) and OIDC. A request with a credential must use HTTPS, except to a loopback address. See [Credentials](https://mise.jdx.dev/tasks/remote-cache.html#credentials).

#### `task.cache.remote_token_file`[Permalink to task.cache.remote\_token\_file](#task.cache.remote_token_file)experimentalglobal only

File that mise reads the remote task cache token from before each request.

- Type: `path`
- Default: unset
- Environment variable: `MISE_TASK_CACHE_REMOTE_TOKEN_FILE`
- Set in: global or system config, or `MISE_TASK_CACHE_REMOTE_TOKEN_FILE`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Suits rotating credentials such as Kubernetes projected service account tokens. See [Credentials](https://mise.jdx.dev/tasks/remote-cache.html#credentials).

#### `task.cache.remote_url`[Permalink to task.cache.remote\_url](#task.cache.remote_url)experimental

Base URL of the remote task cache server.

- Type: `URL`
- Default: unset
- Environment variable: `MISE_TASK_CACHE_REMOTE_URL`

The server must implement the [remote cache protocol](https://mise.jdx.dev/tasks/remote-cache-protocol.html); mise appends `v1/...` to this URL. Remote access also needs [`task.cache.remote_namespace`](#task.cache.remote_namespace). When unset, mise uses only the local cache. See [Remote task cache](https://mise.jdx.dev/tasks/remote-cache.html).

#### `task.cache_dir`[Permalink to task.cache\_dir](#task.cache_dir)experimental

Directory for the task artifact cache.

- Type: `path`
- Default: `$MISE_CACHE_DIR/task-artifacts`
- Environment variable: `MISE_TASK_CACHE_DIR`

mise keeps entries in a versioned subdirectory, such as `v2`, and includes this directory in `mise cache clear` and cache pruning. See [Storage, retention, and output replay](https://mise.jdx.dev/tasks/caching.html#storage-retention-and-output-replay).

#### `task.cache_max_age`[Permalink to task.cache\_max\_age](#task.cache_max_age)experimental

Remove task artifact cache entries that have not been used for this long.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: unset
- Environment variable: `MISE_TASK_CACHE_MAX_AGE`

mise enforces it after storing a new entry, and treats an older entry as a miss. Unset or `0s` keeps entries regardless of age. It is separate from [`cache_prune_age`](https://mise.jdx.dev/configuration/settings.html#cache_prune_age).

#### `task.cache_max_size`[Permalink to task.cache\_max\_size](#task.cache_max_size)experimental

Maximum total size of the task artifact cache, such as `500MB` or `2GiB`.

- Type: `string`
- Default: unset
- Environment variable: `MISE_TASK_CACHE_MAX_SIZE`

After storing a new entry, mise removes the least recently used entries until the cache fits. Unset or `0` means no limit.

#### `task.disable_paths`[Permalink to task.disable\_paths](#task.disable_paths)

Paths that mise does not look for tasks in.

- Type: `path[]`
- Default: `[]`
- Environment variable: `MISE_TASK_DISABLE_PATHS` (separated by `:` ( `;` on Windows))

Relative paths in a config file resolve from that file's config root, and relative paths in the environment variable from the directory mise runs in. See [Exclude paths from discovery](https://mise.jdx.dev/tasks/task-discovery.html#exclude-paths-from-discovery).

#### `task.disable_spec_from_run_scripts`[Permalink to task.disable\_spec\_from\_run\_scripts](#task.disable_spec_from_run_scripts)

Build task usage specs only from the `usage` field, ignoring deprecated `arg()`, `option()` and `flag()` in run scripts.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_TASK_DISABLE_SPEC_FROM_RUN_SCRIPTS`

Defining task arguments with `arg()`, `option()` and `flag()` in run scripts is deprecated, and mise 2027.5.0 removes it. Enable this to adopt that behavior now. mise then skips the extra pass that renders run scripts to collect those calls, which also avoids the empty strings the functions return during that pass. Define arguments in the task's [`usage`](https://mise.jdx.dev/tasks/task-arguments.html#usage-field) field instead; see [Migrate from Tera argument functions](https://mise.jdx.dev/tasks/task-arguments.html#tera-templates).

#### `task.lock_timeout`[Permalink to task.lock\_timeout](#task.lock_timeout)

How long a task waits for a lock another run holds before it fails.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: unset
- Environment variable: `MISE_TASK_LOCK_TIMEOUT`

Applies to tasks that set [`lock`](https://mise.jdx.dev/tasks/task-configuration.html#lock). Unset, a task waits until the lock is free. `mise run --lock-timeout` overrides this setting. The task's own [`timeout`](https://mise.jdx.dev/tasks/task-configuration.html#timeout) starts only once it holds the lock.

#### `task.monorepo_depth`[Permalink to task.monorepo\_depth](#task.monorepo_depth)

Maximum directory depth for automatic monorepo discovery, which is deprecated.

- Type: `integer`
- Default: `5`
- Environment variable: `MISE_TASK_MONOREPO_DEPTH`

Only used by automatic monorepo discovery, which mise falls back to when the root config has no `[monorepo].config_roots`. That fallback is deprecated; list projects in [`config_roots`](https://mise.jdx.dev/tasks/monorepo.html#explicit-config-roots) instead.

`1` finds `projects/`, and `2` finds `projects/frontend/`. If the walk is slow and every project is at `projects/*`, set it to `2`:

#### `task.monorepo_exclude_dirs`[Permalink to task.monorepo\_exclude\_dirs](#task.monorepo_exclude_dirs)

Directory names that automatic monorepo discovery skips.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_TASK_MONOREPO_EXCLUDE_DIRS` (comma-separated)

Only used by automatic monorepo discovery, which mise falls back to when the root config has no `[monorepo].config_roots`. That fallback is deprecated; list projects in [`config_roots`](https://mise.jdx.dev/tasks/monorepo.html#explicit-config-roots) instead.

When empty, mise skips `node_modules`, `target`, `dist` and `build`. A list you set replaces those defaults, so include any of them you still want skipped, such as `["node_modules", "vendor"]`.

#### `task.monorepo_respect_gitignore`[Permalink to task.monorepo\_respect\_gitignore](#task.monorepo_respect_gitignore)

Skip directories that `.gitignore` files ignore during automatic monorepo discovery.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_TASK_MONOREPO_RESPECT_GITIGNORE`

Only used by automatic monorepo discovery, which mise falls back to when the root config has no `[monorepo].config_roots`. That fallback is deprecated; list projects in [`config_roots`](https://mise.jdx.dev/tasks/monorepo.html#explicit-config-roots) instead.

#### `task.output`[Permalink to task.output](#task.output)

Default output style for `mise run`.

- Type: `string`
- Default: `interleave` when tasks run one at a time, otherwise `prefix`
- Environment variable: `MISE_TASK_OUTPUT`
- Choices:
  - `prefix`: print each line labelled with the task name
  - `interleave`: print output as it arrives, without labels
  - `keep-order`: label lines, stream one task live and buffer the others, then print them in definition order as they finish
  - `replacing`: replace the previous stdout line with each new one, like `mise install` progress; stderr is printed as is
  - `summary`: hold each task's output until it ends, then print one line if it passed and everything it printed if it failed
  - `timed`: print only stdout lines that stay on screen for more than a second
  - `quiet`: deprecated; use `interleave` with `task.quiet`
  - `silent`: print nothing from tasks or mise except errors

How much mise prints is a separate choice: [`task.quiet`](#task.quiet), `--quiet` and `--silent`, or a task's own `quiet` and `silent` properties, combine with any style. For example, `task.output = "prefix"` with `task.quiet = true` keeps the task-name labels and hides mise's own messages. See [Output](https://mise.jdx.dev/tasks/running-tasks.html#output).

The `quiet` value is deprecated: it warns from mise 2026.9.3 and is removed in mise 2027.9.3. Use `task.output = "interleave"` with `task.quiet = true` instead, or `--output interleave --quiet` on the command line.

#### `task.quiet`[Permalink to task.quiet](#task.quiet)

Hide mise's own messages while tasks run, without changing other mise commands.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_TASK_QUIET`

Hidden messages include the command line mise prints before a task runs, such as `[build] $ cargo build`. Task stdout and stderr stay visible, and any [`task.output`](#task.output) style can be combined with this setting.

#### `task.remote_no_cache`[Permalink to task.remote\_no\_cache](#task.remote_no_cache)

Download remote task files and task includes on every run instead of using the cached copy.

- Type: `boolean`
- Default: unset
- Environment variable: `MISE_TASK_REMOTE_NO_CACHE`

Applies to tasks whose `file` is a URL, which mise otherwise keeps until `mise cache clear`, and to `git::` and `oci::` task includes. Same as `mise run --no-cache`. See [Remote tasks](https://mise.jdx.dev/tasks/toml-tasks.html#remote-tasks).

#### `task.run_auto_install`[Permalink to task.run\_auto\_install](#task.run_auto_install)

Install missing tools automatically before tasks run.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_TASK_RUN_AUTO_INSTALL`

Has no effect when [`auto_install`](https://mise.jdx.dev/configuration/settings.html#auto_install) is `false`.

#### `task.show_full_cmd`[Permalink to task.show\_full\_cmd](#task.show_full_cmd)

Print each task's full command line instead of truncating it to the terminal width.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_TASK_SHOW_FULL_CMD`

Setting [`truncate`](https://mise.jdx.dev/configuration/settings.html#truncate) to `false` does this for all output.

#### `task.skip`[Permalink to task.skip](#task.skip)

Names of tasks that `mise run` skips.

- Type: `string[]`
- Default: `[]`
- Environment variable: `MISE_TASK_SKIP` (comma-separated)

#### `task.skip_depends`[Permalink to task.skip\_depends](#task.skip_depends)

Run only the tasks named on the command line, skipping their dependencies; same as `mise run --skip-deps`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_TASK_SKIP_DEPENDS`

#### `task.source_freshness_equal_mtime_is_fresh`[Permalink to task.source\_freshness\_equal\_mtime\_is\_fresh](#task.source_freshness_equal_mtime_is_fresh)

Treat a task as up to date when its newest source and its outputs have the same modification time.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_TASK_SOURCE_FRESHNESS_EQUAL_MTIME_IS_FRESH`

By default, outputs must be strictly newer than every source. Enable this on filesystems with coarse timestamps, where a build can leave outputs with the same time as its inputs. See [Freshness settings](https://mise.jdx.dev/tasks/caching.html#freshness-settings).

#### `task.source_freshness_hash_contents`[Permalink to task.source\_freshness\_hash\_contents](#task.source_freshness_hash_contents)

Decide whether a task's sources changed by hashing their contents instead of comparing modification times.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_TASK_SOURCE_FRESHNESS_HASH_CONTENTS`

mise compares BLAKE3 hashes of the source contents with those recorded after the last successful run. This is slower, but checkouts and copies that change timestamps do not make a task stale. The first run after you enable it always runs the task. See [Freshness settings](https://mise.jdx.dev/tasks/caching.html#freshness-settings).

#### `task.timeout`[Permalink to task.timeout](#task.timeout)

Time limit for a whole `mise run` invocation, across every task it runs.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: unset
- Environment variable: `MISE_TASK_TIMEOUT`

A task's own [`timeout`](https://mise.jdx.dev/tasks/task-configuration.html#timeout) limits each of its commands; whichever limit is reached first stops the task. `mise run --timeout` overrides this setting.

#### `task.timings`[Permalink to task.timings](#task.timings)

Print each task's elapsed time when it finishes.

- Type: `boolean`
- Default: `true` for the `prefix`, `keep-order` and `timed` styles unless output is quiet, otherwise `false`
- Environment variable: `MISE_TASK_TIMINGS`

`mise run --no-timings` hides the elapsed time for one run.

### `tera_v1`[Permalink to tera\_v1](#tera_v1)deprecated

Render templates with Tera v1 instead of Tera v2.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_TERA_V1`
- Deprecated: tera\_v1 is a temporary compatibility escape hatch for templates that still need Tera v1 behavior. Update templates to Tera v2 syntax instead.

Use it only for templates that need Tera v1 macros or its lenient handling of undefined variables. In a shared `mise.toml`, set `MISE_TERA_V1 = true` under `[env]` instead: older mise releases treat it as an ordinary environment variable, while they reject an unknown setting. It does not apply to `.miserc.toml`. See [Migrating from Tera v1](https://mise.jdx.dev/templates.html#tera-v2-migration).

### `terminal_progress`[Permalink to terminal\_progress](#terminal_progress)

Show install progress in the terminal window's own progress indicator, with OSC 9;4 escape sequences.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_TERMINAL_PROGRESS`

mise sends the sequences only to terminals that support them, such as Ghostty, iTerm2, VS Code's terminal, Windows Terminal and VTE-based terminals such as GNOME Terminal and Ptyxis, and not to Alacritty, WezTerm or kitty.

### `tool_update`[Permalink to tool\_update](#tool_update)

#### `tool_update.check_duration`[Permalink to tool\_update.check\_duration](#tool_update.check_duration)global only

How often mise checks for a newer version of a global tool that sets `auto_update = true`, or of every global tool with `tool_update.global_auto = true`.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"24h"`
- Environment variable: `MISE_TOOL_UPDATE_CHECK_DURATION`
- Set in: global or system config, or `MISE_TOOL_UPDATE_CHECK_DURATION`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

A tool can set its own interval instead, such as `auto_update = "6h"`. Intervals under `1h` are raised to `1h`. See [Automatic tool updates](https://mise.jdx.dev/dev-tools/#automatic-tool-updates).

#### `tool_update.global_auto`[Permalink to tool\_update.global\_auto](#tool_update.global_auto)global only

Update every tool in global config automatically, as if each set `auto_update` to this value.

- Type: `boolean | string`
- Default: `false`
- Environment variable: `MISE_TOOL_UPDATE_GLOBAL_AUTO`
- Set in: global or system config, or `MISE_TOOL_UPDATE_GLOBAL_AUTO`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

`true` checks every [`tool_update.check_duration`](#tool_update.check_duration), and a duration such as `"6h"` sets the interval instead. A tool's own `auto_update` takes precedence, so `auto_update = false` opts one tool out. Tools in project config are never updated, and exact versions such as `node = "24.11.1"` stay as they are. See [Automatic tool updates](https://mise.jdx.dev/dev-tools/#automatic-tool-updates).

### `truncate`[Permalink to truncate](#truncate)

Truncate long terminal output to fit the available width.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_TRUNCATE`

Set to `false` to show complete values in output that fits itself to the terminal, such as table cells and task command lines. Limits on captured command output, HTTP response bodies and history data still apply. AI coding agents always get untruncated output. Commands that truncate also accept `--truncate` and `--no-truncate` for one run.

### `trusted_config_paths`[Permalink to trusted\_config\_paths](#trusted_config_paths)global only

Directories whose config files mise trusts without asking.

- Type: `path[]`
- Default: `[]`
- Environment variable: `MISE_TRUSTED_CONFIG_PATHS` (separated by `:` ( `;` on Windows))
- Set in: global or system config, or `MISE_TRUSTED_CONFIG_PATHS`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Covers config files at or below these paths, including projects you create there later. `["/"]` trusts every config file, which turns trust checks off. They override trust prompts you declined and `mise trust --ignore`, but [`ignored_config_paths`](#ignored_config_paths) still stops configs from loading. Paranoid mode still trusts these paths, without tying trust to file contents. See [Trust by path](https://mise.jdx.dev/security.html#trust-by-path).

mise first compares each entry as written with the config's canonical path, which needs no filesystem lookup. Only if none match does it resolve the entries, once per process. Entries on autofs or network mounts (such as `/home/...` on macOS, an autofs mount that holds no user homes) cost a lookup each, so use `~/...` or a path that exists locally instead.

### `unix_default_file_shell_args`[Permalink to unix\_default\_file\_shell\_args](#unix_default_file_shell_args)global only

Shell that runs a file task on Unix when the task sets no `shell` and the file has no shebang or known extension.

- Type: `string`
- Default: `"sh"`
- Environment variable: `MISE_UNIX_DEFAULT_FILE_SHELL_ARGS`
- Set in: global or system config, or `MISE_UNIX_DEFAULT_FILE_SHELL_ARGS`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Applies when mise runs the file through a shell instead of executing it: when the file is not executable, or when [`use_file_shell_for_executable_tasks`](#use_file_shell_for_executable_tasks) is on. mise appends the file path as the last argument.

### `unix_default_inline_shell_args`[Permalink to unix\_default\_inline\_shell\_args](#unix_default_inline_shell_args)global only

Shell and arguments that run inline scripts on Unix, such as a task's `run`, hooks and `postinstall`.

- Type: `string`
- Default: `"sh -o errexit -c"`
- Environment variable: `MISE_UNIX_DEFAULT_INLINE_SHELL_ARGS`
- Set in: global or system config, or `MISE_UNIX_DEFAULT_INLINE_SHELL_ARGS`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

mise appends the script as the last argument. A task's own `shell` takes precedence.

### `upgrade`[Permalink to upgrade](#upgrade)

#### `upgrade.auto_prune`[Permalink to upgrade.auto\_prune](#upgrade.auto_prune)

Remove the version that `mise upgrade` replaced once `upgrade.prune_after` has passed.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_UPGRADE_AUTO_PRUNE`

The old install stays during that grace period, so running processes can keep loading files from it. Set this to `false` when something outside mise uses that path for good, such as a virtualenv built from a mise-managed interpreter; the old version then stays without a scheduled removal.

Versions still used by another tracked config, a tool stub or a running process are kept, and may be removed up to a day after that use ends. A process runs from a version when its executable, or an absolute path in its command line, is inside that version's directory. When the path goes through a link such as `installs/<tool>/latest` that an upgrade has moved since the process started, the process's executable tells which version it runs; when only a script path goes through the link, every version of that tool counts as running until the process exits. `mise upgrade --prune` removes the replaced version at once, and `--no-prune` keeps it without scheduling removal. See [`upgrade.prune_after`](#upgrade.prune_after).

#### `upgrade.prune_after`[Permalink to upgrade.prune\_after](#upgrade.prune_after)

Grace period before mise removes a version that `mise upgrade` replaced.

- Type: `duration` (for example `30s`, `10m`, `1h` or `7d`)
- Default: `"24h"`
- Environment variable: `MISE_UPGRADE_PRUNE_AFTER`

The replaced version stays at its original path for this long, and a later mise command removes it. This protects long-running commands that still load files from the old install. Set to `0s` to remove scheduled versions on the next mise command. Run `mise upgrade --prune` or `mise prune` to remove them in the current one.

### `url_replacements`[Permalink to url\_replacements](#url_replacements)

URL patterns and replacements that mise's HTTP client applies before sending a request, for mirrors and proxies.

- Type: `table`
- Default: unset
- Environment variable: `MISE_URL_REPLACEMENTS` (a JSON object)

A plain key matches anywhere in the URL, and every occurrence is replaced. A key that starts with `regex:` is a Rust regular expression, and its first match is replaced. Rules apply in order, and the first one that changes the URL wins. `MISE_URL_REPLACEMENTS` takes the rules as a JSON object.

Rerouting `api.github.com`, or a repository's release and attestation API paths, also stops mise from using [mise-versions](#use_versions_host) for that GitHub metadata, so it all comes from the replacement. Rerouting only release downloads does not. See [URL replacements](https://mise.jdx.dev/url-replacements.html).

### `use_file_shell_for_executable_tasks`[Permalink to use\_file\_shell\_for\_executable\_tasks](#use_file_shell_for_executable_tasks)

Run executable file tasks through a shell instead of executing them directly.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_USE_FILE_SHELL_FOR_EXECUTABLE_TASKS`

By default, mise runs a file task that the OS can execute as a program. With this on, mise starts it with a shell, chosen in this order: the task's `shell`, the file's shebang, its extension (`.ps1` runs with `pwsh -File`), then [`unix_default_file_shell_args`](#unix_default_file_shell_args) or [`windows_default_file_shell_args`](#windows_default_file_shell_args).

### `use_versions_host`[Permalink to use\_versions\_host](#use_versions_host)

Get version lists, public GitHub release metadata and attestations from mise-versions before upstream APIs.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_USE_VERSIONS_HOST`

[mise-versions](https://mise-versions.jdx.dev) is a shared cache that refreshes version lists for registry tools and public GitHub release data on a schedule. Using it avoids GitHub's anonymous rate limit in normal installs, including Docker builds and CI jobs without a GitHub token.

When a command resolves several tools, such as `mise outdated` or `mise upgrade`, mise reads one index of every tool's newest versions instead of each tool's version list, and keeps it as long as a version list ([`fetch_remote_versions_cache`](#fetch_remote_versions_cache)). A request the index cannot answer exactly reads that tool's list as before.

When the host does not list a registry tool's versions, mise lists them from the tool's own source. GitHub release data, tags, repository contents and attestations for public repositories come from the host first. When the host errors (a timeout, a lost connection, or a `429` or `5xx` response), mise asks `api.github.com` for the same data instead, with your GitHub token if one is set. If GitHub fails too, release data and tag lists fall back to a copy mise cached earlier when it has one; otherwise the error names both failures. Anything the host answers is trusted, including a `404` for a missing release, tag or repository and an empty list, so those never reach GitHub's anonymous rate limit. Paranoid mode also asks GitHub when the host can't settle an attestation answer. Otherwise mise asks GitHub directly only for a repository the host does not know, which may be private, and only with GitHub credentials. Private repositories, GitHub Enterprise, custom GitHub API URLs, and installs with this setting off need normal [GitHub authentication](https://mise.jdx.dev/dev-tools/github-tokens.html) to stay within GitHub's rate limits.

Every backend's list may come from the host, including `npm:`, `pipx:`, `cargo:`, `gem:` and `go:` tools, when the tool is a registry tool on its registry backend. The host is skipped for a tool whose source you have redirected, because its list describes the public one: an npm registry from `.npmrc` or `NPM_CONFIG_REGISTRY` (or any registry under `npm.shell_out`), [`pypi.registry_url`](#pypi.registry_url), [`cargo.registry_name`](#cargo.registry_name), a `GOPROXY`, `GOPRIVATE` or `GONOPROXY` other than the default, a gem source or a primary `gem sources` entry other than rubygems.org, [`dotnet.registry_url`](#dotnet.registry_url), an `AWS_ENDPOINT_URL` for S3, or a local tool option that changes the listing. `conda:` tools always list from their channel, since which versions exist depends on the platform. The host's list can trail a brand-new release, which the default [`minimum_release_age`](#minimum_release_age) hides anyway. Set this to `false` to list straight from the source.

When [`url_replacements`](#url_replacements) reroutes `api.github.com`, or a repository's release or attestation API paths, mise does not use this host for that GitHub metadata; the replacement is the only source, because this host describes only github.com. Rules that reroute only release downloads do not change this, and version lists for registry tools still come from this host.

Every request to the host sends the mise version and invoking shell in the User-Agent. When mise runs in CI, these requests also send an `x-mise-ci: true` header and end the User-Agent with `ci`, for example `mise/2026.10.10 linux-x64 (2026-10-11) bash ci`.

The host's request logs record each request's IP address, path and query, User-Agent, and time. They are kept in private storage for up to 35 days and then deleted automatically. A nightly job turns them into usage counts, such as monthly active users and installs per tool, keyed by a salted hash of the IP address.

Set this to `false` to stop these requests. mise still downloads Python's list of precompiled builds from this host; see [How mise installs Python](https://mise.jdx.dev/lang/python.html#precompiled-python-binaries).

`mise ls-remote --no-versions-host` skips the host for one command. See [A new release is not listed](https://mise.jdx.dev/troubleshooting.html#new-version-of-a-tool-is-not-available).

### `use_versions_host_track`[Permalink to use\_versions\_host\_track](#use_versions_host_track)

Send download statistics to mise-versions after installing a tool.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_USE_VERSIONS_HOST_TRACK`

After a successful install of a core or registry tool, mise reports it to [mise-versions](https://mise-versions.jdx.dev), which shows tool popularity. Set this to `false` to opt out. Nothing is sent when [`use_versions_host`](#use_versions_host) is `false`.

The report is a `GET https://mise-versions.jdx.dev/t` request whose query parameters carry:

- the tool name, its full backend identifier such as `aqua:BurntSushi/ripgrep` (with any URL credentials, query string, and fragment removed, and left out when a URL in it cannot be parsed), and the version
- the OS and architecture

The request also sends the mise version and invoking shell, in the User-Agent, and whether it runs in CI.

The host's request logs store the report with your raw IP address, User-Agent, and time in private storage for up to 35 days, then delete it automatically. A nightly job counts installs from these logs, keyed by a salted hash of the IP address.

### `verbose`[Permalink to verbose](#verbose)

Show extra output, such as installation logs, and set the log level to debug; same as `--verbose`.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_VERBOSE`

### `windows_default_file_shell_args`[Permalink to windows\_default\_file\_shell\_args](#windows_default_file_shell_args)global only

Shell that runs a file task on Windows when the task sets no `shell` and the file has no shebang or known extension.

- Type: `string`
- Default: `"cmd /c"`
- Environment variable: `MISE_WINDOWS_DEFAULT_FILE_SHELL_ARGS`
- Set in: global or system config, or `MISE_WINDOWS_DEFAULT_FILE_SHELL_ARGS`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Applies when mise runs the file through a shell instead of executing it: when Windows cannot run the file by itself, or when [`use_file_shell_for_executable_tasks`](#use_file_shell_for_executable_tasks) is on. mise appends the file path as the last argument.

### `windows_default_inline_shell_args`[Permalink to windows\_default\_inline\_shell\_args](#windows_default_inline_shell_args)global only

Shell and arguments that run inline scripts on Windows, such as a task's `run`, hooks and `postinstall`.

- Type: `string`
- Default: `"cmd /c"`
- Environment variable: `MISE_WINDOWS_DEFAULT_INLINE_SHELL_ARGS`
- Set in: global or system config, or `MISE_WINDOWS_DEFAULT_INLINE_SHELL_ARGS`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

mise appends the script as the last argument. A task's own `shell` takes precedence.

### `windows_executable_extensions`[Permalink to windows\_executable\_extensions](#windows_executable_extensions)

File extensions, without the dot, that mise treats as executable on Windows.

- Type: `string[]`
- Default: `["exe", "bat", "cmd", "com", "ps1", "vbs"]`
- Environment variable: `MISE_WINDOWS_EXECUTABLE_EXTENSIONS` (comma-separated)

mise tries them in order when it looks up a command name that has no extension, and a file with one of them counts as a file task. Matching ignores case.

### `windows_powershell_no_profile`[Permalink to windows\_powershell\_no\_profile](#windows_powershell_no_profile)

Pass `-NoProfile` to the PowerShell (`pwsh` or `powershell`) that mise starts, so startup profiles are skipped.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_WINDOWS_POWERSHELL_NO_PROFILE`

This matches `sh -c` and `zsh -c` on Unix, which do not read interactive startup files. A profile that changes `PATH`, such as one that activates mise or adds the shims directory, can otherwise shadow a task's own tools and cause `cannot find binary path` errors.

It applies to tasks, hooks and `watch_files` commands that run through PowerShell, on any OS, and has no effect on `cmd`, `sh`, `bash` or other shells. Set it to `false` if your tasks rely on variables, functions or aliases from your PowerShell profile.

### `windows_shim_mode`[Permalink to windows\_shim\_mode](#windows_shim_mode)

How mise creates shims on Windows.

- Type: `string`
- Default: `"exe"`
- Environment variable: `MISE_WINDOWS_SHIM_MODE`
- Choices:
  - `exe`: copy `mise-shim.exe` as `<tool>.exe`; works in every shell, with package managers and with `where.exe`, and needs `mise-shim.exe` next to `mise.exe`
  - `file`: write a `.cmd` batch file, plus an extensionless script for Git Bash and Cygwin
  - `hardlink`: create NTFS hard links to `mise.exe`, which must be on the same volume as the shims
  - `symlink`: create symbolic links to `mise.exe`, which needs administrator rights or Developer Mode

Use `exe` unless a tool needs one of the others. When `mise-shim.exe` is missing, mise warns and uses `file`. After mise is upgraded, the next `mise reshim` rebuilds `exe`, `file` and `hardlink` shims.

### `write_targets`[Permalink to write\_targets](#write_targets)

#### `write_targets.dotfiles`[Permalink to write\_targets.dotfiles](#write_targets.dotfiles)global only

Global config file where `mise dot add` creates new `[dotfiles]` entries.

- Type: `path`
- Default: unset
- Set in: global or system config. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

`mise dot edit` uses it too when it adds an entry. `mise dot track` does not: it writes new track entries to the global `config.toml`, or to `conf.d/dotfiles-tracking.toml` when another entry already manages the target. Existing entries stay in the file that declares them. The path may be absolute, start with `~/`, or be relative to the `config_root` of the config file that sets it, and may use templates such as `{{ config_source | dirname }}`. It must name a file that mise loads from the global config directory, such as `~/.config/mise/conf.d/30-dotfiles.toml`. See [Send global writes to separate files](https://mise.jdx.dev/configuration.html#global-section-write-targets).

#### `write_targets.packages`[Permalink to write\_targets.packages](#write_targets.packages)global only

Global config file where `--global` package commands create new `[bootstrap.packages]` entries.

- Type: `path`
- Default: unset
- Set in: global or system config. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Used by `mise bootstrap packages use --global` and `mise bootstrap packages import --global`. Existing entries stay in the file that declares them. The path may be absolute, start with `~/`, or be relative to the `config_root` of the config file that sets it, and may use templates such as `{{ config_source | dirname }}`. It must name a file that mise loads from the global config directory, such as `~/.config/mise/conf.d/20-packages.toml`. See [Send global writes to separate files](https://mise.jdx.dev/configuration.html#global-section-write-targets).

#### `write_targets.tools`[Permalink to write\_targets.tools](#write_targets.tools)global only

Global config file where `mise use --global` creates new `[tools]` entries.

- Type: `path`
- Default: unset
- Set in: global or system config. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Existing entries stay in the file that declares them. The path may be absolute, start with `~/`, or be relative to the `config_root` of the config file that sets it, and may use templates such as `{{ config_source | dirname }}`. It must name a file that mise loads from the global config directory, such as `~/.config/mise/conf.d/10-tools.toml`. See [Send global writes to separate files](https://mise.jdx.dev/configuration.html#global-section-write-targets).

### `yes`[Permalink to yes](#yes)global only

Answer yes to every confirmation prompt, as `--yes` does.

- Type: `boolean`
- Default: `false`
- Environment variable: `MISE_YES`
- Set in: global or system config, or `MISE_YES`. Ignored, with a warning, in project config. See [global-only settings](https://mise.jdx.dev/configuration/settings.html#global-only-settings).

Use it in scripts and CI; mise also turns it on when the `CI` environment variable is set. It also answers the config trust prompt, which records trust, except in [paranoid mode](https://mise.jdx.dev/paranoid.html).

### `zig`[Permalink to zig](#zig)

#### `zig.use_community_mirrors`[Permalink to zig.use\_community\_mirrors](#zig.use_community_mirrors)

Download Zig from the community mirrors that ziglang.org recommends before trying ziglang.org itself.

- Type: `boolean`
- Default: `true`
- Environment variable: `MISE_ZIG_USE_COMMUNITY_MIRRORS`

ziglang.org offers no uptime or speed guarantees and asks automated downloads to use the [community mirrors](https://ziglang.org/download/community-mirrors/). Every download is verified against the Zig Software Foundation's signing key, so a mirror cannot alter it. When every mirror fails, mise downloads from ziglang.org.

mise refreshes its cached mirror list daily. Without network access to the list, write one mirror URL per line to `zig/community-mirrors.txt` in the directory that `mise cache path` prints.

[Edit this page](https://github.com/jdx/mise/edit/main/docs/configuration/settings.md)

Last updated:

Pager

[Previous pageConfig environments](https://mise.jdx.dev/configuration/environments.html)

[Next pageConfig variables](https://mise.jdx.dev/configuration/vars.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)