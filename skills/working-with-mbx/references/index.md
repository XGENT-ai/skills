[▶New: **Pray Silence for Mister Boxington**, the theme song and music videothe theme song→](https://mr-boxington.jdx.dev/song)

# mr boxingtonA shared cache for Cargo builds

Reuse compiler work across projects, worktrees, and CI. Keep running Cargo: mbx restores matching outputs, compiles the rest, and keeps build storage within a disk budget.

[Get started](https://mr-boxington.jdx.dev/getting-started)

`mise use --global --tool-option mr_boxington=true rust mr-boxington` Copy

mise 2026.9.2+ · Linux · macOS (Apple Silicon) · Windows [More install options →](https://mr-boxington.jdx.dev/installation)

![Mr Boxington, a cardboard cache box with a monocle and a handlebar mustache](https://mr-boxington.jdx.dev/logo.svg)

1. Fold: Two pens draw a flat cardboard net. It folds up into a box, the lid slams shut, and a strip of tape runs down the front.
2. Mr Boxington: The box leaps, lands, and comes to life as Mr Boxington: a skeptical eye, a monocle on a brass chain, a handlebar mustache, and rosy cheeks. His name card reads mr boxington, a shared cache for Cargo builds.
3. What mbx does: The camera dives into his monocle, and the line mbx reuses Cargo compiler work across projects, worktrees, and CI slams into place.
4. Every checkout compiles again: Without a shared cache, a project, a worktree, and a CI runner each run cargo build and compile the same crates again, and old target/ directories pile up.
5. Under cargo build: You keep typing cargo build. Cargo plans the build, and mbx wraps each rustc call it makes, with no daemon to manage.
6. First build fills the store: The first build compiles as usual, and mbx stores each output under a key of its inputs, with the checkout's paths replaced by placeholders. Nothing was cached yet, so Mr Boxington finishes taped, without a strawberry.
7. Same checkout, empty target/: Measured in the hk benchmark: the same checkout with an empty target/ restores 354 of 354 compilations from the store in 1.3 seconds. Mr Boxington blushes, is taped, and gets a strawberry.
8. Another worktree: A new worktree at another path computes the same keys, so its matching crates are restored and only the edited crate compiles.
9. Six builds at once: Six builds running at once share one pool of compiler permits. Cache hits never wait, and a compilation two of the builds need runs once. In the hk benchmark, six CI jobs on one runner peaked at 32 compilers running at once with the scheduler on, against 162 with it off.
10. CI: In CI, pushes to main publish compiled outputs to a remote cache, such as the GitHub Actions cache, a cache server, or an S3 bucket, and pull requests only restore from it.
11. Next push, measured: A bar chart of the hk benchmark's next push in CI, with the cache holding the previous commit: Cargo alone takes 18.9 seconds and Cargo with mbx takes 9.2 seconds, 9.6 seconds less, with 353 of 354 compilations restored and 1 compiled.
12. target/ pruned: target/ lives in the cache. Cartons of old target directories pile up, and after a build mbx removes those whose checkout is gone or that went unused for 30 days, then the least recently used while the pile is over its disk budget.
13. Morph: The cartons that were kept melt together into one liquid shape, which sets into Mr Boxington's outline.
14. End card: Mr Boxington returns in full with a strawberry beside his tape, above the name mr boxington, the line A shared cache for Cargo builds, the command cargo install mbx --locked && mbx setup, and the address mr-boxington.jdx.dev.

*Build times come from the [benchmarks](https://mr-boxington.jdx.dev/benchmarks).*

## Reuse builds across worktrees

When you build in a new worktree, mbx checks the shared store for matching compiler outputs before compiling. The GitHub Action carries cached work between CI runs. Connect a remote cache to reuse CI's compiler outputs on your machines too.

[How cache matching works →](https://mr-boxington.jdx.dev/how-it-works#portable-keys)

● ● ●two worktrees / one storeillustration

First build New worktree

\~/project$ mbx build

Compilinglibcstored for later

Compilingserdestored for later

Compilingyour-appstored for later

Compiler outputs stored for reuse.

## Follow build progress

In an interactive terminal, `build`, `check`, `clippy`, `run`, and `test` show each compiling crate with a timer, label finished crates with their cache outcome, and color the progress bar by outcome. This applies to `mbx build` and to `cargo build` set up to use mbx. In CI, or with `display = "plain"` (`MBX_DISPLAY=plain`), mbx shows Cargo's own output instead.

![mbx build output with live crates and timers, finished crates marked hit or miss, a progress bar split into cache hits and misses, and estimated compiler time saved, beside the Mr Boxington mascot](https://mr-boxington.jdx.dev/screenshots/cargo-pretty.gif)

*`mbx build` during a partly cached rebuild. Pause animation*

## Inspect builds and cache usage

LiveStoreInsightsExample build data

[![mbx Live dashboard with build activity, hit and miss graphs, store capacity, and compilation savings](https://mr-boxington.jdx.dev/screenshots/tui-live.png)](https://mr-boxington.jdx.dev/screenshots/tui-live.png)

*Watch each compilation's cache outcome as builds run, with hit and miss graphs, store capacity, and compiler time saved. [ View full size ↗](https://mr-boxington.jdx.dev/screenshots/tui-live.png)*

[Watch builds with `mbx tui` →](https://mr-boxington.jdx.dev/tui)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)