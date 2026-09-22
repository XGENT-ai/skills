A shared cache for Cargo builds

# mr boxingtonA shared cache.<br>A tidier `target/`.

Reuse Cargo builds across worktrees and CI. Keep the workflow you already have.

[Get started](https://mr-boxington.jdx.dev/getting-started)

`mise use --global --tool-option mr_boxington=true rust mr-boxington` Copy

Linux · macOS · Windows [More install options ↗](https://mr-boxington.jdx.dev/installation)

![Mr Boxington, a cardboard cache box wearing a monocle and bow tie](https://mr-boxington.jdx.dev/logo.svg)

Build here. Reuse there.

## A new worktree.<br>A head start.

Build once, then restore matching work from another checkout or CI.

[How the cache travels →](https://mr-boxington.jdx.dev/how-it-works#portable-keys)

● ● ●two worktrees / one storeillustration

First build New worktree

\~/project$ mbx build

Compilinglibcstored for later

Compilingserdestored for later

Compilingyour-appstored for later

Freshly compiled. Safely tucked away.

While Cargo does its work

## A little company for the compile.

![Animated Boxington mascot beside mbx build output showing live and completed crates](https://mr-boxington.jdx.dev/screenshots/cargo-pretty.gif)

*Build output during a rebuild. Pause animation*

Your cache, in plain sight

## Watch the work you don’t have to do.

LiveInsightsStoreExample build data

[![mbx Live dashboard with build activity, hit and miss graphs, store capacity, and compilation savings](https://mr-boxington.jdx.dev/screenshots/tui-live.png)](https://mr-boxington.jdx.dev/screenshots/tui-live.png)

*Follow every build, watch cache traffic, and spot the biggest wins. [ View full size ↗](https://mr-boxington.jdx.dev/screenshots/tui-live.png)*

[Explore the dashboard →](https://mr-boxington.jdx.dev/tui)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)