Get started [​](#get-started)

mbx reuses compiler work across Rust workspaces, checkouts, and CI runs. Cargo still resolves dependencies and decides what needs building. mbx restores matching compilations and runs the compiler for everything else.

You need a Cargo project. The mise command in [Install](#install) also installs Rust; with the other methods, install a Rust toolchain first. No cache server or configuration file is required.

## Set up with your AI agent [​](#set-up-with-your-ai-agent)

Want your coding agent to handle setup? Open it in your project and paste the prompt below. It asks the agent to install mbx, adapt it to your existing setup, and verify that a build reuses cached work.

<details>

<summary>Copy a setup prompt</summary>



text

```
Set up mbx (mr-boxington) for this project and verify that it works.

Read the current installation and setup guides:
https://mr-boxington.jdx.dev/installation
https://mr-boxington.jdx.dev/setup
https://mr-boxington.jdx.dev/cache-results#measure-cache-reuse

Inspect my OS, shell, Rust toolchain, build commands, and existing mise,
Cargo, and compiler-cache configuration. Use my existing tool manager where
appropriate, and preserve Rust version pins and unrelated settings. Install
mbx if needed; do not upgrade Rust just to enable it.

Prefer project-scoped setup. With a compatible mise installation, enable
native mise integration (the Rust tool's mr_boxington option) while preserving
the existing Rust tool options.
Otherwise, get explicit mbx commands working first. Ask before changing my
global shell/editor configuration or replacing another compiler wrapper.
Do not delete existing build outputs or caches, or change CI configuration.

Run mbx doctor and a representative build for this project. With native mise
integration, doctor can warn that the standalone Cargo shim is missing or
inactive even when wrapping works. Verify activation through actual builds
and cache hits; do not change PATH solely to clear that setup warning.
Use your own command environment, since an agent may use a different PATH
than my terminal. Installing mbx alone is not proof that cargo uses it.

Demonstrate reuse with the same source, toolchain, profile, and features in
two fresh temporary target directories, using the same mbx cache. If you
configured plain cargo commands to use mbx, run this check through that route.
Check the second build for real cache hits. Do not count Cargo skipping
up-to-date outputs as a cache hit. If there are no hits, use mbx explain --last to
investigate and report any remaining blocker. Remove only the temporary
target directories you created for this check when finished.

Summarize what you changed, the verification results, how I should run builds
and tests from this agent and my terminal, and how to undo the setup. Explain
which benefits are enabled by default. If I run multiple agents or test suites,
mention the optional scheduler.tests setting and ask whether I want it enabled.
```

</details>

To set up mbx yourself, follow the steps below.

## Install [​](#install)

With [mise](https://mise.jdx.dev):

sh

```
mise use --global --tool-option mr_boxington=true rust mr-boxington
```

This requires mise 2026.9.2 or newer. With an older mise, see [Older mise versions](https://mr-boxington.jdx.dev/installation#older-mise-versions).

Drop `--global` for project-scoped wrapping. To keep an existing Rust version pin, add `mr_boxington = true` to that `rust` entry instead; see [Share setup with a project](https://mr-boxington.jdx.dev/setup#share-setup-with-a-project).

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

mbx preserves Cargo aliases, installed subcommands, and toolchain selection. Use the same toolchain, features, and profile across builds to reuse the same cached work.

## Keep using plain Cargo [​](#keep-using-plain-cargo)

The mise command in [Install](#install) makes mise run Cargo through mbx, so you do not need `mbx setup`. Use `mise exec -- cargo build`, `mise run` tasks, or plain `cargo` with mise activation or shims on `PATH`.

`mbx doctor` does not recognize native mise integration, so its `setup` check can warn that the Cargo shim is missing or inactive even when Cargo runs through mbx. To confirm wrapping, follow [Verify plain Cargo](https://mr-boxington.jdx.dev/setup#verify-plain-cargo).

For a standalone installation, run `mbx setup` to install a Cargo shim and configure rust-analyzer, then `mbx setup --status` to check that both are current:

sh

```
mbx setup
mbx setup --status
```

See [Cargo and editor setup](https://mr-boxington.jdx.dev/setup) to verify Cargo's path, share project configuration, configure rust-analyzer, and use mbx from desktop applications.

## The first build [​](#the-first-build)

A new local store has no compilations to restore. The first build fills it; later builds and equivalent worktrees can reuse that work. If Cargo already has up-to-date outputs in `target/`, it skips those compilations entirely. That is normal, and those compilations do not appear as mbx hits.

mbx creates a managed target directory and leaves a `target` symlink in the workspace. It moves an existing `target/` into the managed target and keeps its outputs; CI builds leave `target/` in place. See [Managed target directories](https://mr-boxington.jdx.dev/managed-targets) for placement and cleanup.

Outside CI, the first build on a machine also prints the cache location and the disk budgets chosen for it. Automatic [collection](https://mr-boxington.jdx.dev/managed-targets#collection) runs after builds, at most once an hour.

## Read the result [​](#read-the-result)

An illustrative build summary looks like this:

text

```
mbx[cache]: 139 hits, 8 misses, 4 not looked up, 7 bypassed; 0 B downloaded, 0 B uploaded, 41.2 MiB stored locally
```

| Result | What it means |
| --- | --- |
| [Hit](https://mr-boxington.jdx.dev/cache-results#hit) | mbx restores a matching compilation |
| [Miss](https://mr-boxington.jdx.dev/cache-results#miss) | No stored result matches the cache key; mbx stores the result if compilation succeeds |
| [Not looked up](https://mr-boxington.jdx.dev/cache-results#could-not-look-up) | mbx cannot compute the key yet, so it compiles, then stores a successful result and records its inputs |
| [Bypass](https://mr-boxington.jdx.dev/cache-results#bypass) | mbx runs the invocation without shared caching |

mbx does not store a compilation that keeps private incremental state, such as a workspace crate you just edited. The build summary counts those as `incremental`; see [Incremental builds](https://mr-boxington.jdx.dev/incremental).

`stored locally` is the size of the new data this build added to the local store. With a [remote cache](https://mr-boxington.jdx.dev/remote-cache), `downloaded` and `uploaded` report remote traffic, and a `prefetched` count shows results that mbx fetched from the remote before the build asked for them.

To check reuse, build into two fresh target directories with the same command and options. [Measure cache reuse](https://mr-boxington.jdx.dev/cache-results#measure-cache-reuse) shows the commands and explains why rerunning an up-to-date build shows no mbx hits.

Use `mbx explain --last` to inspect the last recorded build. To follow builds as they run, open `mbx tui`; see [Watching builds](https://mr-boxington.jdx.dev/tui).

## Inspect the cache directory [​](#inspect-the-store)

sh

```
mbx cache stats     # size and contents
mbx gc --dry-run    # preview collection
```

A cache can always be rebuilt. See [Inspect and clean up](https://mr-boxington.jdx.dev/managed-targets#inspect-and-clean-up) for what each cleanup command removes.

## Next steps [​](#next-steps)

| I want to… | Read |
| --- | --- |
| Set up an editor or watch loop | [Local development](https://mr-boxington.jdx.dev/cookbook/local-development) |
| Cache a GitHub Actions job | [GitHub Action](https://mr-boxington.jdx.dev/github-action) |
| Run independent builds together | [Parallel builds](https://mr-boxington.jdx.dev/scheduling) |
| Tune disk, output, or build policy | `mbx settings ls` and [Configuration](https://mr-boxington.jdx.dev/configuration) |
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