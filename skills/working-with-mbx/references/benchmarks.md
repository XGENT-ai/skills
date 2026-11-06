Benchmarks [​](#benchmarks)

These benchmarks compare mbx with plain Cargo and [kache](https://github.com/kunobi-ninja/kache) on two pinned Rust projects, built with `cargo build --locked`. hk shows a change to one application crate; aube shows changes propagating through a workspace. Published results come from GitHub Actions. A tool is labeled fastest only when its lead exceeds the observed variation between trials.

## hk: one application crate changes [​](#hk-one-application-crate-changes)

[jdx/hk](https://github.com/jdx/hk) is a mid-size Rust CLI with C dependencies. Its next-commit build recompiles one crate while restoring its dependencies. This is the workload featured in the landing-page showreel.

[warm1.3slevel with kache](#warm) [commit9.2s2.05× faster than Cargo](#commit) [edit2.4slevel with Cargo](#edit) [contention39.2s9.0s sooner than scheduler off](#contention)

### warm CI rebuilds a commit it has already built

The store is warm from an earlier build of the same commit and target/ is empty. Cargo is left out: with nothing to reuse it would repeat that earlier build.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| kache |  | 1.2s 1.1–1.2s |
| mbx |  | 1.3s 1.3–1.3s |

Too close to call: kache and mbx finished 0.06s apart, inside the 0.2s one of them moved across its own runs.

mbx restored 1,665 output files on 354 cache hits.

### commit CI builds the next push

The caches were warmed at the parent commit and build the child. Cargo has no cache to restore, so its row is the uncached build.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| mbx2.05× faster than Cargo |  | 9.2s 8.5–9.3s |
| kache1.97× faster than Cargo |  | 9.6s 8.6–9.9s |
| Cargouncached |  | 18.9s 18.3–19.1s |

Too close to call: mbx and kache finished 0.3s apart, inside the 1.3s one of them moved across its own runs.

mbx restored 1,663 output files on 353 cache hits.

### edit One line changed, rebuilt in place

The local edit loop with incremental compilation on. Cargo's own incremental rebuild is the thing to beat here, not a control.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| kachelevel with Cargo |  | 2.3s 2.2–2.7s |
| mbxlevel with Cargo |  | 2.4s 2.2–2.6s |
| Cargoincremental rebuild |  | 2.4s 2.3–2.5s |

Too close to call: kache and mbx finished 0.08s apart, inside the 0.5s one of them moved across its own runs.

mbx restored 7 output files on 2 cache hits.

First edit after a build, before the loop settles: kache 9.3s, mbx 9.1s, Cargo 2.2s.

### contention Six CI jobs on one runner

Overlapping check, Clippy, and test-compilation jobs. Run one after another, then all at once with and without mbx's machine-wide compiler limit.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| sequential32 compilers at peak, 62.5 GB free at the low |  | 50.7s 46.9–51.3s |
| parallelscheduler off162 compilers at peak, 53.3 GB free at the low |  | 48.2s 46.1–50.2s |
| parallelmbx schedulerfastest32 of 32 permits used, 61.6 GB free at the low |  | 39.2s 37.0–39.3s |

With the scheduler on, the parallel batch finished 9.0s sooner than unscheduled and 11.5s sooner than running the jobs in turn, peaking at 32 compilers instead of 162.

Cache hits over the batch: sequential 49; parallel, scheduler off 12; parallel, mbx scheduler 3,402. The scheduler holds identical compilations until the first finishes, so the other jobs hit the store instead of repeating the work.

Measured on Linux-7.1.4-x86\_64-with-glibc2.39 (nsc-runner-da58b12ru280a) with Rust 1.94.0, building `hk` at `fc29ead` under mbx 1.17.0, cargo 1.94.0 (85eff7c80 2026-01-15), rustc 1.94.0 (4a4ef493e 2026-03-02), kache 0.26.3. [The run that produced them](https://github.com/jdx/mr-boxington/actions/runs/35910301201) has the per-build logs.

## aube: changes propagate through a workspace [​](#aube-changes-propagate-through-a-workspace)

[jdx/aube](https://github.com/jdx/aube) is a 15-crate Rust workspace with C dependencies. Its next commit changes two libraries and rebuilds their dependents, exposing the work that whole-compilation caching cannot reuse.

[warm3.3slevel with kache](#aube-warm) [commit23.2s1.43× faster than Cargo](#aube-commit) [edit8.2s0.3s behind Cargo](#aube-edit) [contention54.7s12.3s sooner than scheduler off](#aube-contention)

### warm CI rebuilds a commit it has already built

The store is warm from an earlier build of the same commit and target/ is empty. Cargo is left out: with nothing to reuse it would repeat that earlier build.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| mbx |  | 3.3s 2.9–4.1s |
| kache |  | 3.6s 3.5–4.1s |

Too close to call: mbx and kache finished 0.2s apart, inside the 1.2s one of them moved across its own runs.

mbx restored 2,350 output files on 577 cache hits.

### commit CI builds the next push

The caches were warmed at the parent commit and build the child. Cargo has no cache to restore, so its row is the uncached build.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| mbxfastest1.43× faster than Cargo |  | 23.2s 22.7–23.4s |
| kache1.30× faster than Cargo |  | 25.5s 25.5–26.4s |
| Cargouncached |  | 33.2s 33.1–33.4s |

mbx fastest, 2.4s ahead of kache. That lead is wider than either tool's own range across 3 runs.

mbx restored 2,328 output files on 568 cache hits.

### edit One line changed, rebuilt in place

The local edit loop with incremental compilation on. Cargo's own incremental rebuild is the thing to beat here, not a control.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| Cargoincremental rebuildfastest |  | 7.9s 7.8–8.1s |
| mbx0.3s behind Cargo |  | 8.2s 8.2–8.3s |
| kache0.4s behind Cargo |  | 8.3s 8.3–8.5s |

Cargo fastest, 0.3s ahead of mbx. That lead is wider than either tool's own range across 3 runs.

First edit after a build, before the loop settles: Cargo 8.0s, mbx 24.9s, kache 27.5s.

### contention Six CI jobs on one runner

Overlapping check, Clippy, and test-compilation jobs. Run one after another, then all at once with and without mbx's machine-wide compiler limit.

wall clock · lower is better

| Tool | Relative wall clock | Median and range |
| --- | --- | --- |
| sequential32 compilers at peak, 60.1 GB free at the low |  | 2m 12s 132.1–133.1s |
| parallelscheduler off172 compilers at peak, 50.6 GB free at the low |  | 1m 7s 66.4–68.1s |
| parallelmbx schedulerfastest31 of 32 permits used, 54.9 GB free at the low |  | 54.7s 52.5–54.8s |

With the scheduler on, the parallel batch finished 12.3s sooner than unscheduled and 77.4s sooner than running the jobs in turn, peaking at 31 compilers instead of 172.

Cache hits over the batch: sequential 3; parallel, scheduler off 23; parallel, mbx scheduler 3,550. The scheduler holds identical compilations until the first finishes, so the other jobs hit the store instead of repeating the work.

Measured on Linux-7.1.4-x86\_64-with-glibc2.39 (nsc-runner-kv0r2drcad4f6) with Rust 1.94.0, building `aube` at `e9a36eb` under mbx 1.19.0, cargo 1.94.0 (85eff7c80 2026-01-15), rustc 1.94.0 (4a4ef493e 2026-03-02), kache 0.26.3. [The run that produced them](https://github.com/jdx/mr-boxington/actions/runs/36334170965) has the per-build logs.

Below each project's charts, a line names the mbx version, Rust version, and CI run that produced its results. Historical runs can use different mbx releases, so compare tools within one project's run. A refresh measures both projects with the same mbx release.

## Read the results [​](#reading-the-cards)

Each scenario has three independent trials per tool. A trial starts with a fresh clone and empty store, then performs the scenario's warm-up and measured builds. The bar shows the median; the whisker spans the fastest and slowest trial. A tool is marked fastest only when its lead exceeds both tools' trial ranges. Otherwise, the card reports no clear winner.

The Cargo row means different things in different scenarios, so the card tags it. In the new-worktree and commit scenarios it is the uncached baseline: the cold build a fresh checkout or CI runner does without a cache. In the edit scenario it is the incremental rebuild the caches have to keep up with. The warm scenario has no Cargo row, because with an empty `target/` Cargo would repeat the build that warmed the store. The six-parallel-jobs scenario has no Cargo row either, because every row there runs mbx.

## The scenarios [​](#the-scenarios)

### Warm build [​](#warm-build)

A first build warms the store, then `target/` is wiped and the same commit builds again. This is a runner restoring its cache and building something it has already seen.

### New worktree [​](#new-worktree)

A build at one commit warms the local store. Then the same commit builds in a new Git worktree with no target directory of its own. Each tool uses its default target placement, and the benchmark harness does not copy the first worktree's target directory. The scenario tests reuse across checkout paths, including dependency target seeding when the tool and Cargo support it. Cargo's cold build provides the uncached baseline. [Start new checkouts from existing units](https://mr-boxington.jdx.dev/managed-targets#start-new-checkouts-from-existing-units) describes how mbx seeds a new checkout.

Results published before this scenario was added do not include it, so a project whose current run predates it has no worktree card. The line under each project's charts names the run that produced its cards.

### Next commit [​](#next-commit)

The store is warmed at one commit and the build runs at the next. hk's next commit changes its application crate. For aube, the next commit changes two library crates, so they and the workspace crates that depend on them compile again while the rest of the graph is restored. Cargo's row is a cold build, since with an empty `target/` that is all it can do.

### Local edit [​](#local-edit)

A full build runs first. Then one line changes in hk's `src/main.rs` or in aube's `aube-util`, which most of aube's workspace depends on, and the project rebuilds in the same `target/` with `CARGO_INCREMENTAL=1`. mbx overrides that, as it does by default, and uses learned incremental reuse instead. This measures the edit/build loop, including cache bookkeeping and incremental compilation. Two details make the comparison useful:

- The harness unsets `CI` for every tool. mbx switches [learned incremental reuse](https://mr-boxington.jdx.dev/incremental#learned-incremental-reuse) off in CI because fresh runners have no earlier edit state to reuse.
- The first edit establishes incremental state; the second supplies the headline timing. Cargo's full build already wrote its incremental state, while mbx builds an edited crate's learned incremental state on the first edit and reuses it afterwards. The card also shows what that first edit cost, since you wait for it once per fresh build.

### Six parallel jobs [​](#six-parallel-jobs)

Six overlapping Rust CI jobs from an empty store: default and all-targets/all-features variants of `cargo check`, Clippy, and test compilation. Every row runs mbx; this scenario compares mbx with itself and has no Cargo or kache row. The sequential row runs the jobs in turn in one `target/` with the scheduler on. The two parallel rows give each job its own `target/`, as separate CI steps would, and differ only in whether the [machine-wide scheduler](https://mr-boxington.jdx.dev/scheduling#machine-wide-compile-scheduling) is on.

Cargo bounds the compilers it starts itself and knows nothing about the Cargo process beside it; the scheduler gives every process one pool of permits and holds identical compilations until the first finishes. Peak compilers and lowest free memory show whether a faster batch shared the machine or oversubscribed it.

## Fairness controls [​](#keeping-it-fair)

- The registry is fetched once, before any timed build, so no timed build includes crate downloads.
- Every trial starts from a fresh clone, an empty store, and a new `target/`. Nothing carries over between tools or between runs.
- Like your own build, the harness uses whichever Rust toolchain the runner provides. The `toolchain` field in each results file records which release produced the numbers. Every trial in a run uses that one compiler, so the tools are compared fairly against each other. A Rust upgrade between runs changes every cache key at once, which reads as a cache that stopped working. Check that field before treating a drop across runs as a regression.
- The harness sets `CARGO_INCREMENTAL=0`, as CI does, in every scenario except the edit scenario. It clears any inherited `RUSTC_WRAPPER` or `RUSTC_WORKSPACE_WRAPPER`, and the run fails if the Cargo baseline turns out to run under mbx, for example through the Cargo shim `mbx setup` installs.
- Both caches run local-only, since a remote would measure the network. The harness runs kache with `KACHE_LOCAL_ONLY=1`. It does not clear an mbx remote cache; published results are local-only because the CI runner that produces them configures none.
- The harness rejects a run that does not exercise the intended cache behavior. A run fails when:
  - a warm build restored nothing, or ran no faster than the build that seeded it
  - mbx wrote no build report ( `MBX_STATS_REPORT`) for a new-worktree build
  - an edit rebuild compiled nothing
  - the scheduled contention batch went past its permits, or the unscheduled one never did

## Run the benchmarks yourself [​](#running-it-yourself)

sh

```
mise run bench
```

That builds mbx, clones aube, and runs the warm, new-worktree, commit, and edit scenarios once each. kache is included when it is on `PATH` and noted as skipped otherwise.

The harness does not turn off an mbx remote cache, so with one configured the mbx rows include network restores. Set `MBX_REMOTE_URL` to an empty value for the run; it overrides `remote.url` in your [configuration file](https://mr-boxington.jdx.dev/configuration):

sh

```
MBX_REMOTE_URL= mise run bench
```

`mise run bench:refresh` runs CI's full measurement against the mbx built from your checkout: both projects across every scenario, three trials each, written to `benchmarks/results.json` (hk) and `benchmarks/results-aube.json` (aube).

CI runs the same harness against the latest release instead. The [bench-refresh workflow](https://github.com/jdx/mr-boxington/actions/workflows/bench-refresh.yml) checks weekly and reruns when the recorded mbx version differs from the latest release. It measures that released version and opens a pull request with the results. On `main`, a manual run with `force` also opens or updates that PR, even when the recorded version already matches. Set `dry_run` to measure without proposing an update, or `source` to measure the selected source ref instead of the latest release; `source` implies `dry_run`.

## What this does not measure [​](#what-this-does-not-measure)

These results describe the two pinned workloads. A project with a different dependency shape, such as heavy proc macros, a large C component, or many small leaf crates, will see different ratios. Three trials expose some variation; their ranges are not confidence intervals, and a “fastest” label is a display heuristic, not a statistical significance test. The benchmark is Linux-only, and [Caching limits](https://mr-boxington.jdx.dev/limits) covers what changes on macOS and Windows.

Instruction-counted measurements of mbx's own startup path, and cold and warm correctness runs against this workspace, live in [`benchmarks/`](https://github.com/jdx/mr-boxington/tree/main/benchmarks).

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/docs/benchmarks.md)

Last updated:

Pager

[Previous pageHow mbx compares](https://mr-boxington.jdx.dev/compared)

[Next pageAcknowledgements](https://mr-boxington.jdx.dev/acknowledgements)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)