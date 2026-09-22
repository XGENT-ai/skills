Benchmarks

Benchmarks [​](#benchmarks)

mbx is measured against plain Cargo and [kache](https://github.com/kunobi-ninja/kache) on [jdx/hk](https://github.com/jdx/hk), a mid-size Rust CLI with C dependencies, pinned to one commit and built with `cargo build --locked`. The scenarios cover work a developer or CI runner may repeat. Published results come from GitHub Actions. The page labels a tool fastest only when its lead exceeds the observed variation between trials.

[warm8.1s1.25× faster than kache](#warm) [commit26.2s1.66× faster than Cargo](#commit) [edit6.0slevel with Cargo](#edit) [contention1m 9s23.7s sooner than scheduler off](#contention)

### warm CI rebuilds a commit it has already built

The store is warm from an earlier build of the same commit and target/ is empty. Cargo is left out: with nothing to reuse it would repeat that earlier build.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| mbxfastest |  | 8.1s 7.9–8.6s |
| kache |  | 10.1s 9.8–10.3s |

mbx fastest, 2.0s ahead of kache. That lead is wider than either tool's own range across 3 runs.

mbx restored 1,560 output files on 722 cache hits.

### commit CI builds the next push

The caches were warmed at the parent commit and build the child. Cargo has no cache to restore, so its row is the uncached build.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| mbxfastest1.66× faster than Cargo |  | 26.2s 25.7–26.5s |
| kache1.53× faster than Cargo |  | 28.4s 28.3–28.8s |
| Cargouncached |  | 43.5s 42.5–44.3s |

mbx fastest, 2.2s ahead of kache. That lead is wider than either tool's own range across 3 runs.

mbx restored 1,558 output files on 721 cache hits.

### edit One line changed, rebuilt in place

The local edit loop with incremental compilation on. Cargo's own incremental rebuild is the thing to beat here, not a control.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| Cargoincremental rebuild |  | 5.9s 5.8–5.9s |
| mbxlevel with Cargo |  | 6.0s 5.9–6.1s |
| kachelevel with Cargo |  | 6.1s 5.9–6.1s |

Too close to call: Cargo and mbx finished 0.09s apart, inside the 0.2s one of them moved across its own runs.

mbx restored 2 output files on 1 cache hits.

First edit after a build, before the loop settles: Cargo 6.3s, mbx 22.6s, kache 24.8s.

### contention Six CI jobs on one runner

Overlapping check, Clippy, and test-compilation jobs. Run one after another, then all at once with and without mbx's machine-wide compiler limit.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| sequential31 compilers at peak, 63.2 GB free at the low |  | 1m 49s 107.7–109.6s |
| parallelscheduler off170 compilers at peak, 54.1 GB free at the low |  | 1m 33s 90.7–93.7s |
| parallelmbx schedulerfastest32 of 32 permits used, 61.7 GB free at the low |  | 1m 9s 60.4–72.3s |

With the scheduler on, the parallel batch finished 23.7s sooner than unscheduled and 39.5s sooner than running the jobs in turn, peaking at 32 compilers instead of 170.

Cache hits over the batch: sequential 8; parallel, scheduler off 12; parallel, mbx scheduler 3,390. The scheduler holds identical compilations until the first finishes, so the other jobs hit the store instead of repeating the work.

Measured on Linux-7.1.4-x86\_64-with-glibc2.39 (nsc-runner-g7llknk0pjtlu) with Rust 1.97.1, building `hk` at `fc29ead` under mbx 1.11.1, cargo 1.97.1 (c980f4866 2026-06-30), rustc 1.97.1 (8bab26f4f 2026-07-14), kache 0.21.0. [The run that produced them](https://github.com/jdx/mr-boxington/actions/runs/34930250982) has the per-build logs.

## Reading the results [​](#reading-the-cards)

Each scenario has three independent trials per tool. A trial starts with a fresh clone and empty store, then performs the scenario's warm-up and measured builds. The bar shows the median; the whisker spans the fastest and slowest trial. A tool is marked fastest only when its lead exceeds both tools' trial ranges. Otherwise, the card reports no clear winner.

The Cargo row means something different in each scenario, so the card tags it. In the commit scenario it is the uncached build CI does without a cache. In the edit scenario it is the incremental rebuild the caches have to keep up with. The warm scenario has no Cargo row, because with an empty `target/` Cargo would repeat the build that warmed the store.

## The scenarios [​](#the-scenarios)

### Warm build [​](#warm-build)

A first build warms the store, then `target/` is wiped and the same commit builds again. This is a runner restoring its cache and building something it has already seen.

### Next commit [​](#next-commit)

The store is warmed at one commit and the build runs at the next. Most of the dependency graph is unchanged and a few crates are not. Cargo's row is a cold build, since with an empty `target/` that is all it can do.

### Local edit [​](#local-edit)

A full build, then one line of hk's own source changed and rebuilt in the same `target/` with incremental compilation on. This measures the edit/build loop, including cache bookkeeping and incremental compilation. Two details make the comparison useful:

- `CI` is unset for every tool. mbx switches [learned incremental reuse](https://mr-boxington.jdx.dev/incremental#learned-incremental-reuse) off in CI because fresh runners have no earlier edit state to reuse.
- The first edit establishes incremental state; the second supplies the headline timing. Cargo's own build already wrote its incremental state, while mbx builds an edited crate's [private state](https://mr-boxington.jdx.dev/incremental#learned-incremental-reuse) on the first edit and reuses it afterwards. The card shows what that first edit cost, since a developer waits for it once per fresh build.

### Six parallel jobs [​](#six-parallel-jobs)

Six overlapping Rust CI jobs from an empty store: default and all-targets/all-features variants of `cargo check`, Clippy, and test compilation. The sequential row runs them in turn in one `target/`. The two parallel rows give each job its own `target/`, as separate CI steps would, and differ only in whether the [machine-wide scheduler](https://mr-boxington.jdx.dev/scheduling#machine-wide-compile-scheduling) is on. Cargo bounds the compilers it starts itself and knows nothing about the Cargo process beside it; the scheduler gives every process one pool of permits and holds identical compilations until the first finishes. Peak compilers and lowest free memory show whether a faster batch shared the machine or oversubscribed it.

## Keeping it fair [​](#keeping-it-fair)

- The registry is fetched once, before any timed build. No cell is timed while it downloads crates.
- Every trial starts from a fresh clone, an empty store, and a new `target/`. Nothing carries over between tools or between runs.
- The toolchain is the runner's, the same as any user's build, and the `toolchain` field in `results.json` records which release produced the numbers. Every trial in a run uses that one compiler, so the tools are compared fairly against each other. A Rust bump between runs changes every cache key at once, which reads as a cache that stopped working, so check that field before treating a drop across runs as a regression.
- `CARGO_INCREMENTAL=0` matches CI everywhere except the edit scenario. Any inherited `RUSTC_WRAPPER` is cleared, and the run fails if the Cargo baseline turns out to be an mbx shim.
- Both caches run local-only. A remote would measure the network.
- Validity checks reject runs that do not exercise the intended cache behavior. That is any run where a warm build restored nothing or was no faster than the build that seeded it, where the edit rebuild compiled nothing, or where the scheduled contention batch went past its permits or the unscheduled one never did.

## Running it yourself [​](#running-it-yourself)

sh

```
mise run bench
```

That builds mbx, clones hk, and runs the warm, commit, and edit scenarios once each. kache is included when it is on `PATH` and noted as skipped otherwise. `mise run bench:refresh` is what CI runs: every scenario, three trials each, written to `benchmarks/results.json`.

The [bench-refresh workflow](https://github.com/jdx/mr-boxington/actions/workflows/bench-refresh.yml) checks weekly and reruns when the recorded mbx version differs from the latest release. It measures that released version and opens a pull request with the results. On `main`, a manual run with `force` also opens or updates that PR, even when the recorded version already matches. Set `dry_run` to measure without proposing an update, or `source` to measure the selected source ref instead of the latest release; `source` implies `dry_run`.

## What this does not measure [​](#what-this-does-not-measure)

These results describe the pinned hk workload. A project with a very different dependency shape, such as heavy proc macros, a large C component, or many small leaf crates, will see different ratios. Three trials expose some variation; their ranges are not confidence intervals, and a “fastest” label is a display heuristic, not a statistical significance test. The benchmark is Linux-only, and [limits](https://mr-boxington.jdx.dev/limits) covers what changes on macOS and Windows.

Instruction-counted measurements of mbx's own startup path, and cold and warm correctness runs against this workspace, live in [`benchmarks/`](https://github.com/jdx/mr-boxington/tree/main/benchmarks).

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/docs/benchmarks.md)

Last updated:

Pager

[Previous pageHow mbx compares](https://mr-boxington.jdx.dev/compared)

[Next pageFAQ](https://mr-boxington.jdx.dev/faq)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)