Documentation

Documentation [​](#documentation)

mbx adds a shared compiler cache and automatic storage management to Cargo. Start with a local build, then add the integrations your workflow needs.

## Start here [​](#start-here)

[Get started](https://mr-boxington.jdx.dev/getting-started) takes you from installation to your first cached build. For platform-specific downloads, see [Installation](https://mr-boxington.jdx.dev/installation). For automatic Cargo wrapping and rust-analyzer, see [Cargo and editor setup](https://mr-boxington.jdx.dev/setup).

## Work locally [​](#work-locally)

| Task | Guide |
| --- | --- |
| Use editors, watch loops, or a debugger | [Local development](https://mr-boxington.jdx.dev/cookbook/local-development) |
| Run tests and Clippy at the same time | [Parallel builds](https://mr-boxington.jdx.dev/scheduling) |
| Understand what is stored or deleted | [Managed target directories](https://mr-boxington.jdx.dev/managed-targets) |
| Tune repeated source edits | [Incremental builds](https://mr-boxington.jdx.dev/incremental) |
| Select a linker by profile and target | [Managed linkers](https://mr-boxington.jdx.dev/linkers) |
| Watch compiler and cache activity | [Watching builds](https://mr-boxington.jdx.dev/tui) |
| Cache make or CMake compiler calls | [Standalone C and C++](https://mr-boxington.jdx.dev/standalone-builds) |

## Set up CI and sharing [​](#set-up-ci-and-sharing)

| Task | Guide |
| --- | --- |
| Add caching to a GitHub Actions job | [GitHub Action](https://mr-boxington.jdx.dev/github-action) |
| Replace an existing caching step | [Migrate from rust-cache or sccache](https://mr-boxington.jdx.dev/cookbook/migrate) |
| Support pull requests from forks | [CI with fork pull requests](https://mr-boxington.jdx.dev/cookbook/fork-prs) |
| Connect a server or S3-compatible bucket | [Remote cache](https://mr-boxington.jdx.dev/remote-cache) |
| Operate the reference server | [Cache server](https://mr-boxington.jdx.dev/cache-server) |

## Understand a result [​](#understand-a-result)

Start with [Troubleshooting](https://mr-boxington.jdx.dev/troubleshooting) for an unexpected build. Read [Cache results](https://mr-boxington.jdx.dev/cache-results) to interpret the counters, [How it works](https://mr-boxington.jdx.dev/how-it-works) for the architecture, and [Caching limits](https://mr-boxington.jdx.dev/limits) for invocations mbx bypasses.

Use [Savings and statistics](https://mr-boxington.jdx.dev/stats) for lifetime totals and workspace sharing, or [Watching builds](https://mr-boxington.jdx.dev/tui) to explore live activity and per-build insights.

[Benchmarks](https://mr-boxington.jdx.dev/benchmarks) includes measured results and the method behind them. [How mbx compares](https://mr-boxington.jdx.dev/compared) explains the tradeoffs with other caches and Cargo's incremental compilation.

## Look something up [​](#look-something-up)

- [Configuration](https://mr-boxington.jdx.dev/configuration): file locations, precedence, and every setting.
- [CLI reference](https://mr-boxington.jdx.dev/cli/): command syntax, flags, and arguments.
- [Stability](https://mr-boxington.jdx.dev/stability): upgrade behavior and supported output formats.
- [Protocol compatibility](https://mr-boxington.jdx.dev/protocol-compatibility): local, remote, and Rust API contracts.
- [FAQ](https://mr-boxington.jdx.dev/faq): common questions and the story behind the name.

To improve these docs or contribute code, read [Contributing](https://github.com/jdx/mr-boxington/blob/main/CONTRIBUTING.md). The [acknowledgements](https://mr-boxington.jdx.dev/acknowledgements) recognize the projects mbx builds on.

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/docs/guide.md)

Last updated:

Pager

[Next pageGet started](https://mr-boxington.jdx.dev/getting-started)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)