Get started

Get started [​](#get-started)

mbx reuses compiler work across Rust workspaces, checkouts, and CI runs. Cargo still resolves dependencies and decides what needs building. mbx restores matching compilations and runs the compiler for everything else.

You need an existing Rust toolchain and a Cargo project. No cache server or configuration file is required.

## Install [​](#install)

With [mise](https://mise.jdx.dev):

sh

```
mise use --global --tool-option mr_boxington=true rust mr-boxington
```

This requires mise 2026.9.2 or newer. Drop `--global` for project-scoped wrapping. See [Installation](https://mr-boxington.jdx.dev/installation#mise) for older mise versions.

Or install from crates.io:

sh

```
cargo install mbx --locked
```

Linux, macOS, and Windows binaries are also available. See [Installation](https://mr-boxington.jdx.dev/installation) for verified downloads and platform requirements.

## Run a build [​](#run-a-build)

From your Rust workspace:

sh

```
mbx doctor
mbx build
```

Pass Cargo arguments as usual:

sh

```
mbx test --workspace --all-features
mbx clippy --workspace --all-targets -- -D warnings
mbx +stable check --workspace
```

Cargo aliases, installed subcommands, and toolchain selection are preserved. Use the same toolchain, features, and profile across builds to reuse the same cached work.

## Keep using plain Cargo [​](#keep-using-plain-cargo)

The mise command above enables wrapping without running `mbx setup`. Use `mise exec -- cargo build`, `mise run` tasks, or plain `cargo` with mise activation or shims on `PATH`.

For standalone installations, run:

sh

```
mbx setup
mbx setup --status
```

See [Cargo and editor setup](https://mr-boxington.jdx.dev/setup) to verify Cargo's path, share project configuration, configure rust-analyzer, and use mbx from desktop applications.

## The first build [​](#the-first-build)

A new local store has no compilations to restore. The first build fills it; later builds and equivalent worktrees can reuse that work. If Cargo already has up-to-date outputs in `target/`, it skips those compilations entirely. That is normal and will not appear as mbx cache hits.

For a checkout without an existing `target/`, mbx creates a managed target and leaves a `target` symlink in the workspace. An existing directory is replaced only after you accept the interactive prompt. See [Managed target directories](https://mr-boxington.jdx.dev/managed-targets) for placement and cleanup.

The first build also prints the cache location and disk budgets chosen for your machine. Automatic collection runs after builds, at most once an hour.

## Read the result [​](#read-the-result)

An illustrative summary looks like this:

text

```
mbx[cache]: 139 hits, 8 misses, 4 not looked up, 147 prefetched, 7 bypassed; 312.4 MiB downloaded, 0 B uploaded, 280.1 MiB stored locally
```

| Result | What happened |
| --- | --- |
| Hit | mbx restored a matching compilation |
| Miss | No result matched the key; successful compilation can fill the cache |
| Not looked up | mbx lacked a usable input prediction and had to compile first |
| Bypassed | The invocation ran without shared caching |

To check reuse, build into two fresh target directories with the same command and options. The [cache reuse walkthrough](https://mr-boxington.jdx.dev/cache-results#measure-cache-reuse) shows the commands and explains why simply rerunning an up-to-date Cargo build may produce no mbx hits. Use `mbx explain --last` to inspect the last recorded build or `mbx tui` to [watch builds live](https://mr-boxington.jdx.dev/tui).

## Inspect the store [​](#inspect-the-store)

sh

```
mbx cache stats     # size and contents
mbx gc --dry-run    # preview cleanup
```

A cache can always be rebuilt. Use the [management guide](https://mr-boxington.jdx.dev/managed-targets) to understand what each cleanup command removes.

## Next steps [​](#next-steps)

| I want to… | Read |
| --- | --- |
| Set up an editor or watch loop | [Local development](https://mr-boxington.jdx.dev/cookbook/local-development) |
| Cache a GitHub Actions job | [GitHub Action](https://mr-boxington.jdx.dev/github-action) |
| Run independent builds together | [Parallel builds](https://mr-boxington.jdx.dev/scheduling) |
| Tune disk, output, or build policy | [Configuration](https://mr-boxington.jdx.dev/configuration) |
| Understand an unexpected result | [Troubleshooting](https://mr-boxington.jdx.dev/troubleshooting) |

<details>

<summary>Looking for a section that moved?</summary>



<a href="https://mr-boxington.jdx.dev/setup#verify-plain-cargo">Verify plain Cargo</a> now lives in the setup guide.

<a href="https://mr-boxington.jdx.dev/installation#supported-platforms">Supported platforms</a> now lives in Installation.

<a href="https://mr-boxington.jdx.dev/installation">Installation methods and Windows notes</a> have moved there too.

<a href="https://mr-boxington.jdx.dev/linkers">Choose a linker</a> has its own guide.

<a href="https://mr-boxington.jdx.dev/scheduling">Parallel builds</a> covers separate targets and shared budgets.

<a href="https://mr-boxington.jdx.dev/troubleshooting#diagnose-the-installation">Installation checks</a> and <a href="https://mr-boxington.jdx.dev/troubleshooting#reporting-a-problem">Reporting a problem</a> now live in Troubleshooting.

</details>

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/docs/getting-started.md)

Last updated:

Pager

[Previous pageDocumentation](https://mr-boxington.jdx.dev/guide)

[Next pageInstallation](https://mr-boxington.jdx.dev/installation)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)