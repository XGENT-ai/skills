Configuration [​](#configuration)

Defaults work without a configuration file. Add only the values you want to change. mbx reads configuration from three places; the first value found wins:

1. Environment variables ( `MBX_*`).
2. `.mbx.toml` at the resolved Cargo workspace root, for the [supported workspace settings](#workspace-policy) only. `mbx exec` reads it from its [project root](https://mr-boxington.jdx.dev/standalone-builds#sharing-results-across-checkouts) instead: `--project-root` when given, otherwise the enclosing checkout root, or the working directory outside a checkout.
3. `mbx/config.toml` in the platform configuration directory:
   - Linux: `~/.config/mbx/config.toml`, honoring `$XDG_CONFIG_HOME`
   - macOS: `~/Library/Application Support/mbx/config.toml`
   - Windows: `%APPDATA%\mbx\config.toml`

Anything still unset takes its default. mbx rejects unknown TOML keys, so a misspelled setting is an error.

Shell examples on this page set a variable for one command with POSIX syntax, `NAME=value command`. In PowerShell, set `$env:NAME` before the command and remove it afterward.

## Change settings from the command line [​](#change-settings-from-the-command-line)

`mbx settings set` writes one setting to the global configuration file and creates the file if it does not exist yet:

sh

```
mbx settings set gc.max_size 20GiB
mbx settings set target.evict_first .claude/worktrees,tmp
mbx settings get gc.max_size
mbx settings unset gc.max_size
```

The value must match the setting's type and allowed values and load as that setting, or nothing is written. A problem already elsewhere in the file does not block the edit, so `set` can repair one setting at a time; mbx warns about what is still wrong. List settings take comma-separated items. Comments and formatting elsewhere in the file are kept, and a file that is a symlink stays one.

`unset` removes the key, so the setting falls back to its default. It also removes a key mbx does not recognize. Use it to clear a misspelled key that stops mbx from loading; the error for such a key names the command.

`mbx settings ls` prints every setting with its current value, and `mbx settings ls gc` prints one group. `ls` does not print the value of `remote.token`; `mbx settings get remote.token` does. `get` and `ls` read the environment, the global file, and defaults; they do not read `.mbx.toml`. Edit table settings such as `linker.profiles` in the file directly. Settings read only from the environment, such as `MBX_VERIFY`, cannot be written with `mbx settings set`.

## Sizes and durations [​](#sizes-and-durations)

Sizes accept SI and IEC units. `20GB` and `20GiB` are different values. Durations accept values such as `30s`, `15m`, `1h`, and `30d`.

## Common adjustments [​](#common-adjustments)

| Change | Command or guide |
| --- | --- |
| Leave capacity for your editor | `mbx settings set scheduler.reserve_cpus 2`; [parallel builds](https://mr-boxington.jdx.dev/scheduling) |
| Set one budget for cached build data | `mbx settings set gc.max_total_size 50GiB`; [single cache budget](#single-cache-budget) |
| Keep the action store under a fixed size | `mbx settings set gc.max_size 20GiB` |
| Drop cached results of Rust versions you stopped using | `mbx settings set gc.rustc_max_age 90d`; [old compiler versions](#old-compiler-versions) |
| Keep live targets longer | `mbx settings set target.max_age 60d`; [managed target directories](https://mr-boxington.jdx.dev/managed-targets#collection) |
| Collect agent worktrees' targets first | `mbx settings set target.evict_first .claude/worktrees`; [managed targets](https://mr-boxington.jdx.dev/managed-targets#keep-or-evict-specific-checkouts) |
| Use factual savings messages | `mbx settings set savings plain` |
| Print more cache detail | `mbx settings set summary full`; [cache results](https://mr-boxington.jdx.dev/cache-results) |
| Share results with CI | [Remote cache](https://mr-boxington.jdx.dev/remote-cache) |
| Pin a linker per profile or target | [Managed linkers](https://mr-boxington.jdx.dev/linkers) |
| Tune the shared compiler pool | [Parallel builds](https://mr-boxington.jdx.dev/scheduling) |
| Keep private incremental state from the first compilation | `mbx settings set eager_incremental true`; [incremental builds](https://mr-boxington.jdx.dev/incremental#eager-incremental-reuse) |
| Bound private incremental state across checkouts | `mbx settings set gc.incremental_max_size 30GiB`; [incremental builds](https://mr-boxington.jdx.dev/incremental#storage-and-collection) |
| Compare restored outputs with fresh compilations | `MBX_VERIFY=1`; [troubleshooting](https://mr-boxington.jdx.dev/troubleshooting#verify-mode) |

## Local build storage [​](#local-build-storage)

Keep the cache directory and build outputs on local storage. Remote caches are configured separately under `[remote]`.

By default, the cache directory (`cache_dir`, `MBX_CACHE_DIR`) is `mbx` inside the platform cache directory:

- Linux: `~/.cache/mbx`, honoring `$XDG_CACHE_HOME`
- macOS: `~/Library/Caches/mbx`
- Windows: `%LOCALAPPDATA%\mbx`

`mbx cache dir` prints the store's path, `<cache_dir>/actions`, not the cache directory itself.

On Linux and macOS, mbx refuses to start a build when the cache directory or Cargo output storage is on NFS; other platforms do not run this check. Set `cache_dir` (`MBX_CACHE_DIR`) and, when configured separately, `target.root` (`MBX_TARGET_ROOT`) to local storage. Cargo target directories and separate intermediate build directories that you choose must also be local: check `CARGO_TARGET_DIR` / `build.target-dir` and `CARGO_BUILD_BUILD_DIR` / `build.build-dir`.

The check follows symlinks and checks the destination filesystem even when the directory does not exist yet. An NFS source checkout is supported when its build outputs are local, including a `target` link into a local managed target directory. Remote cache URLs are unaffected; use a [remote cache](https://mr-boxington.jdx.dev/remote-cache) to share results across machines.

Help, cache inspection, and cleanup commands still run with the old configuration, so you can inspect or remove previous NFS storage. Changing the configuration does not copy the cache to the new disk; expect a cold cache. See [Change target placement](https://mr-boxington.jdx.dev/managed-targets#change-target-placement) before moving managed targets to another disk.

## Containers sharing a cache [​](#containers-sharing-a-cache)

Compiler shims are executable wrappers, not cached build artifacts, but by default both live under `cache_dir`. When containers share that directory but each has its own mbx installation, set `shims_dir` (`MBX_SHIMS_DIR`) in each container to a private, dedicated local directory:

sh

```
MBX_CACHE_DIR=/shared/mbx MBX_SHIMS_DIR=/var/lib/worker/mbx-shims mbx build
```

The shim directory must survive later builds, because CMake and other build systems can record absolute compiler or launcher paths. mbx resolves `shims_dir` as follows:

- Unset: `<cache_dir>/shims`
- Absolute path: used as given
- Relative path: resolved under `cache_dir`; rejected if `..` climbs out of it
- Empty, or a relative path that normalizes to empty (such as `.`, `./`, or `a/..`): rejected

The setting also covers `mbx exec` and CMake launchers; cached artifacts stay in the shared `cache_dir`. Set it in the global file or the environment; it is not a workspace policy.

Use a directory reserved for mbx shims, with no real compilers in it. mbx marks shim directories with `.mbx-shims` and skips them when searching for real compilers.

Changing `shims_dir` does not rewrite build configurations generated earlier. If one still records an old shim path, reconfigure that build with the new setting and its original configure options. For a CMake build, that is `mbx exec cmake --fresh -S . -B build` plus those options.

Keep shims while builds run

Do not remove a shim directory while a build uses it. Updating mbx during an active build retains the existing executable-replacement limitations.

## Single cache budget [​](#single-cache-budget)

To manage cached build data with one size setting, add this to your global configuration:

toml

```
[gc]
max_total_size = "50GiB"
```

Or run `mbx settings set gc.max_total_size 50GiB`. The environment equivalent is `MBX_GC_MAX_TOTAL_SIZE`.

The budget covers action-store objects and results, managed targets, learned incremental state, and [generated source trees](https://mr-boxington.jdx.dev/limits#out-dir-sharing). These components share the budget without the usual disk-scaled size caps; learned incremental state and generated source trees default to a cap of 10% of the total, at least 10 GiB but never above the total. Incremental units are collected individually by age and least recent use; a build in the same checkout does not refresh unused units. Explicit component limits still apply; remove those settings if you want mbx to manage the allocation. Age limits and the automatic disk-free-space safeguard remain enabled.

Collection reserves only the space the action store occupies, up to its limit, so an empty store does not force useful targets out. Learned incremental state and generated source trees use the remaining allowance up to their cap. Generated trees are collected first to the smaller of their current size or 10% of that shared allowance; incremental units get what remains after them. Managed targets use what remains after both. If protected state leaves less room, mbx collects the action store to fit the remaining budget. This order favors shared cached results and incremental state over older target directories; allocation is not based on measured rebuild cost.

The budget is a **logical-byte collection target**, not a physical disk quota. Hard-linked files count once within learned incremental state and within managed targets, but other shared blocks can make physical usage smaller, while metadata, session history, and temporary files add overhead outside the budget. Builds can exceed the budget between sweeps. Active builds, held incremental session locks, explicitly kept targets, and untracked state can also prevent collection from reaching it; mbx warns when the combined remainder exceeds the budget. If a component cannot be measured, mbx reports that the combined budget could not be verified and conservatively gives the action store no remaining allowance. Use `mbx gc --dry-run` to inspect collection, or `mbx gc --json` for each component's logical sizes.

Prediction manifests are outside the byte budget. `mbx gc` removes a manifest, and its lock file, once no live checkout claims its identity and it has gone a day untouched. The next build of that identity starts without prefetch hints and writes a new manifest. `mbx gc --dry-run` reports them as unclaimed task manifests, and `mbx gc --json` as `action_store.removed_task_manifests`.

One budget applies even when managed targets live on a different disk from the cache directory; free-space safeguards still operate per disk. Setting `gc.max_total_size = "none"` restores the [disk-scaled defaults](https://mr-boxington.jdx.dev/managed-targets#budgets-scale-with-the-disk) for any component without an explicit limit.

### Old compiler versions [​](#old-compiler-versions)

A new Rust release cannot reuse what an older one cached, so after an upgrade the previous version's results only take up space until the size budget evicts them. `gc.rustc_max_age` removes them sooner:

sh

```
mbx settings set gc.rustc_max_age 90d   # or none, the default
```

Each cached compilation is attributed to the exact compiler build it was made with, including nightlies. A version counts as in use while any of its results was restored or written within the window. The sweep, and `mbx gc`, remove the results of every other version together with the objects only those results need. The most recently used version is always kept, so a store left idle for longer than the window is not emptied. Results that are not Rust compilations, and results a pending grouped CI export still needs, are not affected.

Use time is the result file's access time, which mbx records itself when a build restores the result, at most once a day per file. It does not depend on the filesystem refreshing access times on reads, so it holds under `noatime`, on APFS, and on Windows volumes with last-access updates turned off. Choose a window longer than you go between building a project and rebuilding it. `mbx gc --dry-run` shows what a window would remove.

To drop a version right after you stop using it, name it instead of waiting out a window:

sh

```
mbx gc --rustc-version 1.97.1 --dry-run   # preview
mbx gc --rustc-version 1.97.1
```

The value is the release `rustc --version` prints, such as `1.97.1` or `1.99.0-nightly`. To pick one nightly, give the whole first line of `rustc -vV` without `rustc`, such as `1.99.0-nightly (abc123def 2026-09-01)`. This removes the version even if it was used a moment ago or is the newest in the store, and objects another version also uses stay. It applies to that run only; the automatic sweep never uses it.

## Example global configuration [​](#example)

This example shows several available controls, not a recommended configuration. Copy only the settings you need into your global configuration file. Write a dotted setting under its TOML section header: `gc.max_size` is `max_size` under `[gc]`. Remote settings and machine-specific paths do not belong in a checked-in `.mbx.toml`, which rejects them; see [Workspace policy](#workspace-policy).

<details>

<summary>Show the example</summary>



toml

```
# <config directory>/mbx/config.toml
cache_dir = "/var/cache/mbx"
incremental = false
eager_incremental = false  # opt-in state from the first compilation
seed_incremental = true  # reuse private state across checkouts
share_out_dir = true
share_workspace_root = false
build_script_execution = true
cc = true
summary = "auto"         # or "short", "ci", "full", "off"
savings = "quips"        # or "plain", "off"

[linker]
default = "system"

[linker.profiles.dev]
x86_64-unknown-linux-gnu = "mold@2.42.0"
aarch64-unknown-linux-gnu = "wild@0.10.0"
default = "rust-lld"

[linker.profiles.release]
default = "system"

[gc]
auto = true
max_size = "20GiB"       # default: 5% of the cache disk
incremental_max_size = "20GiB" # default: 5% of the cache disk
incremental_max_age = "30d"    # default
rustc_max_age = "90d"    # optional, off by default
max_total_size = "50GiB" # optional combined budget
min_free_size = "20GiB"  # default: 10% of each disk
interval = "1h"

[target]
views = true
max_size = "30GiB"       # default: 10% of the target disk
max_age = "30d"          # default
keep = ["~/src/app"]     # never collected for age or size
evict_first = [".claude/worktrees"]  # collected first when over budget

[remote]
url = "https://cache.example.com"  # or "s3://bucket/prefix"
namespace = "acme/backend"
mode = "read-write"
# s3_endpoint = "https://<account>.r2.cloudflarestorage.com"
# s3_region = "auto"

[http]
timeout = "30s"
download_timeout = "10m"
retries = 3

[scheduler]
enabled = true
cpus = 16                # default: logical CPUs
reserve_cpus = 2         # default: 0
memory = "24GiB"         # default: 85% of physical memory
priority = "normal"      # or "low"
```

</details>

## Workspace policy [​](#workspace-policy)

A repository may check in a `.mbx.toml` containing only these settings:

- `incremental`, `eager_incremental`, `share_out_dir`, `share_workspace_root`, `build_script_execution`, and `cc`
- `linker.default` and `linker.profiles`, with any [selector](https://mr-boxington.jdx.dev/linkers#selectors) except `path:`
- `scheduler.enabled`, `scheduler.cpus`, `scheduler.reserve_cpus`, `scheduler.memory`, `scheduler.priority`, `scheduler.pressure`, and `scheduler.tests`

For example:

toml

```
incremental = false
eager_incremental = false
share_out_dir = false
share_workspace_root = false
build_script_execution = true
cc = true

[linker.profiles.dev]
default = "rust-lld"

[scheduler]
reserve_cpus = 2
memory = "12GiB"
priority = "normal"
```

Environment variables still win. Machine paths, remote-cache configuration, credentials, diagnostics, target placement, and collection settings are not accepted from a repository-owned file. mbx reports an error for an unsupported or misspelled workspace setting.

See [`share_out_dir`](#share-out-dir) and [`build_script_execution`](#build-script-execution) in the settings reference, and [`OUT_DIR` sharing](https://mr-boxington.jdx.dev/limits#out-dir-sharing) for eligibility and compatibility details.

`share_workspace_root = false` is the global default. Setting it to true maps the workspace root to a placeholder wherever rustc records a source path. A crate rebuilt in a second checkout then comes out byte-identical, so the crates that depend on it still share cached results. The setting suits a machine that builds many checkouts of one repository. The cost is that debug information, `file!()`, and panic locations name the placeholder instead of the real source path. See [A rebuilt workspace crate records its checkout](https://mr-boxington.jdx.dev/limits#a-rebuilt-workspace-crate-records-its-checkout).

## Build-script C and C++ [​](#build-script-c-and-c)

`cc = true` (`MBX_CC`, on by default) caches host C and C++ compiled by Cargo build scripts, such as the native code built by `*-sys` crates. No project changes are required: for the duration of the mbx command, build scripts use mbx's compiler wrappers.

If mbx cannot safely model a compiler call, it runs the real compiler without caching that call. Run `mbx explain` to see why a build bypassed the cache, or read the [full C and C++ limits](https://mr-boxington.jdx.dev/limits#c-and-c-caching-covers-the-host-compiles-mbx-drives).

To cache C and C++ builds that run outside Cargo, put the build command after `mbx exec`. With `cc = false`, `mbx exec` warns and runs the command uncached. See [Cache C and C++ builds outside Cargo](https://mr-boxington.jdx.dev/standalone-builds).

## The savings line [​](#the-savings-line)

`savings` controls the one-line report of accumulated savings after a build (`MBX_SAVINGS` from the environment). `quips`, the default, draws the line from a pool of dry one-liners. `plain` reports the same figures without a quip. `off` keeps the totals without printing anything.

## Build summaries [​](#build-summaries)

`summary` (`MBX_SUMMARY` from the environment) controls the build summary mbx prints to stderr after a build:

| Value | Prints |
| --- | --- |
| `auto` (default) | `ci` in CI, `short` otherwise |
| `short` | One line; routine compiler probes (`compiler-query`, `standard-input`, and their `cc-` counterparts) are left out of its bypass count |
| `ci` | The `short` line plus session timing, estimated compiler time avoided, explanations for compilations not looked up, and bypass reasons |
| `full` | Detailed timing, compiler, bypass, transfer, and output-restoration figures |
| `off` | No build summary; `MBX_STATS_REPORT` is still written when configured |

`auto` treats a build as CI when `CI` or `GITHUB_ACTIONS` is `1`, `true`, or `yes` (case-insensitive). Set `short`, `ci`, `full`, or `off` to use that style everywhere. Cargo's `-q` and `--quiet` also suppress the summary for that invocation.

The `ci` style's `object cache:` counts and transfers exclude artifacts Cargo reused directly and archives that a CI cache step restored or saved. Compiler time avoided is summed across compilations, not elapsed job time saved. In CI, mbx also skips the first-build notice about local cache management.

## Settings [​](#settings)

The complete setting reference is generated from the setting declarations and doc comments in `crates/mbx/src/config.rs`. Environment-only settings are labeled in the entries below.

### `ar_determinism` [​](#ar-determinism)

- **Type:** `string`
- **Default:** `auto`
- **Set with:** `MBX_AR_DETERMINISM`

Set `ZERO_AR_DATE` for build scripts so native archives omit a timestamp.

Apple's `ar` and `ranlib` read the variable; other archivers ignore it. Without it, those tools stamp the time into every archive they write, such as the ones CMake-based dependencies build with `/usr/bin/ar` on macOS. An archive rebuilt from unchanged objects then gets a new digest, and every cached action downstream of it misses.

`auto` normalizes every build whose Cargo `PROFILE` is not `release`. Cargo reports `release` for `--release` builds and for every profile that inherits from `release`, such as `bench`, so `auto` leaves their archives byte-for-byte as the host toolchain made them. `always` covers those too, and `off` leaves the toolchain alone. A `ZERO_AR_DATE` you set yourself always wins.

mbx sets the variable through the build-script wrapper that `build_script_execution` installs, so this has no effect on build scripts compiled while that setting is off.

**Choices:**

- `auto`
- `always`
- `off`

### `blaze` [​](#blaze)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_BLAZE`

Send dev builds typed as plain `cargo` through blaze.

When the Cargo shim receives `cargo build`, `run`, `test`, or `check` without `--release` or `--profile`, and `blaze` is on `PATH`, it runs `blaze` with the same arguments instead of Cargo. blaze rebuilds an edited function in milliseconds, runs no Cargo, and compiles workspace crates itself, so mbx caches none of its builds. The shim runs `blaze test` with `BLAZE_TESTS=all` unless `BLAZE_TESTS` is set, so a routed `cargo test` runs every test, as Cargo does; a `blaze test` run directly defaults to the tests an edit affects. Releases of blaze from before its rename ship `ra-jit` and no `blaze`; with only `ra-jit` on `PATH` the shim runs `ra-jit cargo` instead, which runs Cargo when it has to, through this shim, so those builds still use the cache. That fallback goes away in mbx 2.0. Experimental: Linux and macOS on Apple silicon only, where blaze builds.

### `build_script_execution` [​](#build-script-execution)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_BUILD_SCRIPT_EXECUTION`

Cache executions of build scripts using Cargo's freshness inputs.

This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `bypass_log` [​](#bypass-log)

- **Type:** `option<path>`
- **Optional:** true
- **Scope:** only from the environment or the command line
- **Set with:** `MBX_BYPASS_LOG`

Append the full reason for every bypassed compilation to this path.

### `cache_dir` [​](#cache-dir)

- **Type:** `option<path>`
- **Optional:** true
- **Default:** an `mbx` directory in the platform cache directory, such as `~/.cache/mbx`
- **Set with:** `MBX_CACHE_DIR`

Cache directory, which holds the store and private incremental state.

Managed targets (`target.root`) and compiler shims (`shims_dir`) live inside it unless configured elsewhere. NFS is unsupported for local build storage.

### `cache_links` [​](#cache-links)

- **Type:** `bool`
- **Default:** `true`
- **Scope:** only from the environment or the command line
- **Set with:** `MBX_CACHE_LINKS`

Cache natively linked test binaries, executables, and proc macros.

On Linux, `cdylib`s are cached too. Only links for the host qualify: a binary, test, or `cdylib` built with an explicit `--target`, including one set by `build.target`, still links normally even when it names the host triple. The built-in WebAssembly targets are cached regardless of this setting. On macOS this also passes ld64 `-oso_prefix` so a debug-info link's debug map stops naming this checkout, which is what lets the link cache. Supported on Linux, macOS, and Windows; a link mbx cannot describe exactly still links normally.

### `cc` [​](#cc)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_CC`

Cache C and C++ compilations run by build scripts and by `mbx exec`.

With this off, `mbx exec` runs its command uncached. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `cc_store_path_specific` [​](#cc-store-path-specific)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_CC_STORE_PATH_SPECIFIC`

Store C objects that embed absolute paths under checkout-specific keys.

Disable for disposable worktrees to avoid storing objects that cannot be reused at another path. Existing entries may still be restored.

### `display` [​](#display)

- **Type:** `string`
- **Default:** `auto`
- **Set with:** `MBX_DISPLAY`

How mbx displays Cargo's output.

`auto` allows animated output in a terminal. `plain` disables it even there, including the inline build view and its warning browser, so `pretty_inspect` has no effect.

**Choices:**

- `auto`
- `plain`

### `eager_incremental` [​](#eager-incremental)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_EAGER_INCREMENTAL`

Keep private workspace incremental state from the first compilation.

It applies in CI too, and it overrides `incremental` and learned incremental reuse. Workspace crates then compile with that state instead of restoring or publishing shared results, and crates that link them stay private as well. It follows the same LTO and optimization limits as `learned_incremental`. The first build can be slower. Ignored for compilations selected for verification by `MBX_VERIFY` or `verify_sample_rate`. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `events` [​](#events)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_EVENTS`

Record a per-compilation event stream for `mbx tui` to watch.

`mbx explain --last` and `mbx analyze` also read it back. A build run with this off is not recorded, so those commands report the newest build that was, or find none. Cache entries and other build state are still stored.

### `events_max_size` [​](#events-max-size)

- **Type:** `string`
- **Default:** `16MiB`
- **Set with:** `MBX_EVENTS_MAX_SIZE`

Cap on the per-compilation history one build records, or `"none"`.

Past the cap, the totals the build reports stay complete but its per-compilation rows stop, and `mbx explain` says so.

### `forward_compiler_notifications` [​](#forward-compiler-notifications)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_FORWARD_COMPILER_NOTIFICATIONS`

Forward rustc's diagnostics and artifact notifications to Cargo as they arrive.

Cargo can then start a dependent against a crate's metadata while that crate's code generation continues. Disable to hold the compiler's output until mbx has stored the result, which is useful when diagnosing the compiler shim itself.

### `gc.auto` [​](#gc-auto)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_GC_AUTO`

Sweep after a build when collection is due.

### `gc.incremental_max_age` [​](#gc-incremental-max-age)

- **Type:** `duration`
- **Default:** `30d`
- **Set with:** `MBX_GC_INCREMENTAL_MAX_AGE`

Collect learned incremental units unused this long, or `"none"`.

### `gc.incremental_max_size` [​](#gc-incremental-max-size)

- **Type:** `option<string>`
- **Optional:** true
- **Default:** 5% of the cache disk, from 10 GiB to 100 GiB; with `gc.max_total_size`, 10% of the total, at least 10 GiB but never above the total
- **Set with:** `MBX_GC_INCREMENTAL_MAX_SIZE`

Shared budget for learned incremental state and generated source trees, or `"none"`.

Over budget, mbx collects inactive incremental units least recently used first, using rustc session directory modification times. Generated source trees are collected first, capped at their current size or 10% of this shared budget, whichever is smaller. Incremental units get the remaining budget. With `gc.max_total_size`, defaults to 10% of that total, at least 10 GiB but never above the total.

### `gc.interval` [​](#gc-interval)

- **Type:** `duration`
- **Default:** `1h`
- **Set with:** `MBX_GC_INTERVAL`

Minimum interval between automatic sweeps while the disks have room.

While a disk is short of `gc.min_free_size`, the interval is capped at five minutes, so sweeps can run that often even when this is longer.

### `gc.max_size` [​](#gc-max-size)

- **Type:** `option<string>`
- **Optional:** true
- **Default:** 5% of the cache disk, from 5 GiB to 500 GiB; `gc.max_total_size` when set
- **Set with:** `MBX_GC_MAX_SIZE`

Size budget for the action store.

It also caps how much one build downloads from the remote cache. Unlike the other size budgets, it cannot be `"none"`.

### `gc.max_total_size` [​](#gc-max-total-size)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_GC_MAX_TOTAL_SIZE`

Combined size target for the action store, managed targets, and incremental state, or `"none"`.

A size such as `50GiB` counts logical bytes across the action store, managed targets, learned incremental state, and generated source trees. When set, it replaces disk-scaled defaults and caps incremental state and generated source trees at 10% of the total, at least 10 GiB but never above the total, unless `gc.incremental_max_size` is explicit. Explicit component limits still apply. Active and protected state may exceed this target.

### `gc.min_free_size` [​](#gc-min-free-size)

- **Type:** `option<string>`
- **Optional:** true
- **Default:** 10% of each disk, from 5 GiB to 50 GiB
- **Set with:** `MBX_GC_MIN_FREE_SIZE`

Free space to keep on the disks that hold the cache directory and managed targets, or `"none"`.

Below it, sweeps run more often and collect private state, generated source trees, managed targets, and shared store objects past their budgets to free the shortfall. Active state and explicitly kept targets stay, so the disk can remain short.

### `gc.rustc_max_age` [​](#gc-rustc-max-age)

- **Type:** `duration`
- **Default:** `none`
- **Set with:** `MBX_GC_RUSTC_MAX_AGE`

Collect cached Rust compilations for compiler versions unused this long, or `"none"`.

A new compiler cannot reuse what an older one cached. A version counts as in use while any of its cached results was used within this window, and the sweep removes the results of the others along with the objects only they need. The most recently used version is always kept. Off by default; `"90d"` suits a machine that upgrades Rust now and then.

### `http.download_timeout` [​](#http-download-timeout)

- **Type:** `duration`
- **Default:** `10m`
- **Set with:** `MBX_HTTP_DOWNLOAD_TIMEOUT`

Deadline for one blob download, retries and backoff included.

It also caps each whole request mbx makes to download a managed linker.

### `http.read_stall_budget` [​](#http-read-stall-budget)

- **Type:** `duration`
- **Default:** `90s`
- **Set with:** `MBX_HTTP_READ_STALL_BUDGET`

Wall-clock time a build may lose to failed reads from a cache server.

Once it is spent, the build stops reading from the remote and compiles instead. `0` keeps reading however long it takes. Has no effect on an `s3://` remote.

### `http.retries` [​](#http-retries)

- **Type:** `int`
- **Default:** `3`
- **Set with:** `MBX_HTTP_RETRIES`

How many times to retry a remote cache request after a transient failure.

### `http.timeout` [​](#http-timeout)

- **Type:** `duration`
- **Default:** `30s`
- **Set with:** `MBX_HTTP_TIMEOUT`

Connect timeout, and how long a remote cache request may wait for data.

A response must begin within this time of its request starting, so an upload's whole body has to be sent within it. After that, the clock restarts whenever response bytes arrive: a long download keeps going while data flows and fails only after this long with none. `http.download_timeout` caps a whole blob download, retries included. Managed linker downloads use this setting only as their connect timeout.

### `incremental` [​](#incremental)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_INCREMENTAL`

Let Cargo compile local workspace members incrementally.

mbx stops forcing `CARGO_INCREMENTAL=0`, so Cargo's profiles decide: by default, dev builds compile incrementally and release builds do not. Those compilations bypass the shared cache, crates that depend on them may miss, and learned incremental reuse turns off. Ignored when `CI` is `1`, `true`, or `yes`, and while `eager_incremental` is in effect. When it applies, Cargo still honors a `CARGO_INCREMENTAL` already set in the environment. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `learned_incremental` [​](#learned-incremental)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_LEARNED_INCREMENTAL`

Use private incremental state for crates whose sources keep changing.

A workspace crate qualifies on its first source edit; any other crate after three consecutive misses with changed sources. A qualifying crate's outputs, and those of crates that link them, stay out of the shared cache. Off in CI (`CI` set to `1`, `true`, or `yes`), while `incremental` or `eager_incremental` is in effect, and for compilations selected for verification by `MBX_VERIFY` or `verify_sample_rate`. Not used for compiles that use LTO or are optimized at opt-level 2 or higher (including `s` and `z`), where rustc's incremental local ThinLTO can produce rlibs that do not link. A profile with `lto = "off"` or `codegen-units = 1` stays eligible; Cargo's default `lto = false` does not.

### `linker.default` [​](#linker-default)

- **Type:** `string`
- **Default:** `system`

Linker used when the active Cargo profile has no matching selection.

This may also be set in workspace `.mbx.toml`, but not to a `path:` selector; `MBX_LINKER` wins.

### `linker.profiles` [​](#linker-profiles)

- **Type:** `option<map<string, map<string, string>>>`
- **Optional:** true

Linkers selected by Cargo profile and target triple.

This may also be set in workspace `.mbx.toml`, where no entry may be a `path:` selector. Workspace entries replace the global file's entries for the same profile and target, and `MBX_LINKER` wins over both.

### `linker.selection` [​](#linker-selection)

- **Type:** `option<string>`
- **Optional:** true
- **Scope:** only from the environment or the command line
- **Set with:** `MBX_LINKER`

Override the configured linker for this invocation.

### `log` [​](#log)

- **Type:** `string`
- **Default:** `info,portable_pty=off`
- **Scope:** only from the environment or the command line
- **Set with:** `MBX_LOG`

Log filter, such as `debug` or `mbx=trace`.

It covers every log the mbx process emits: its own and those of the libraries it builds on. The default keeps the pty library behind the inline build view quiet, because mbx falls back to plain Cargo when that view cannot start.

### `pretty_inspect` [​](#pretty-inspect)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_PRETTY_INSPECT`

Open the terminal warning browser after a successful Cargo build.

Has no effect when `display` is `plain`.

### `remote.mode` [​](#remote-mode)

- **Type:** `string`
- **Default:** `read-write`
- **Set with:** `MBX_REMOTE_MODE`

Remote access mode.

Only trusted CI (a push to a protected branch on GitHub Actions or GitLab CI) may write, unless `remote.trust_writes` is set. Everywhere else, including a local shell, `read-write` acts as `read-only` and `write-only` disables the remote.

**Choices:**

- `read-write`
- `read-only`
- `write-only`

### `remote.namespace` [​](#remote-namespace)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_NAMESPACE`

Namespace that isolates one project's cache, such as `acme/backend`.

Required when `remote.url` is set.

### `remote.oidc_audience` [​](#remote-oidc-audience)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_OIDC_AUDIENCE`

GitHub Actions OIDC token audience for authenticating to a cache server.

GitHub Actions only; the job needs `id-token: write`. An `s3://` remote refuses it.

### `remote.s3_conditional_writes` [​](#remote-s3-conditional-writes)

- **Type:** `string`
- **Default:** `auto`
- **Set with:** `MBX_REMOTE_S3_CONDITIONAL_WRITES`

How to handle a bucket that does not implement S3 conditional writes.

Conditional writes keep concurrent updates to the action manifest, which prefetch reads, from overwriting each other. `auto` uses them until the bucket rejects them and then writes without them; `required` fails against such a bucket; and `off` never sends conditional headers.

**Choices:**

- `auto`
- `required`
- `off`

### `remote.s3_endpoint` [​](#remote-s3-endpoint)

- **Type:** `option<url>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_S3_ENDPOINT`

S3 endpoint for a service other than AWS, such as MinIO or R2.

### `remote.s3_force_path_style` [​](#remote-s3-force-path-style)

- **Type:** `option<bool>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_S3_FORCE_PATH_STYLE`

Address S3 buckets in the path rather than the host.

### `remote.s3_region` [​](#remote-s3-region)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_S3_REGION`

S3 region; Cloudflare R2 uses `auto`.

### `remote.token` [​](#remote-token)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_TOKEN`

Bearer token for a cache server.

An `s3://` remote refuses it. Takes precedence over `remote.token_file` and `remote.oidc_audience`.

### `remote.token_file` [​](#remote-token-file)

- **Type:** `option<path>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_TOKEN_FILE`

File containing a bearer token for a cache server.

An `s3://` remote refuses it. `remote.token` takes precedence over it, and it takes precedence over `remote.oidc_audience`.

### `remote.trust_writes` [​](#remote-trust-writes)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_REMOTE_TRUST_WRITES`

Skip the CI write policy and use `remote.mode` as configured.

For CI systems mbx does not detect, such as Buildkite or Jenkins. Set it only from the agent or pipeline environment, only for builds you trust, and never from a file that a pull request can change. Workspace `.mbx.toml` refuses it. The write credential should still be limited to those builds.

### `remote.url` [​](#remote-url)

- **Type:** `option<url>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_URL`

Remote cache URL; its scheme selects the backend.

Use `https://` for an mbx cache server or `s3://bucket[/prefix]` for an S3-compatible bucket. mbx refuses the `remote.s3_*` settings with any other scheme.

### `restore_hardlink` [​](#restore-hardlink)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_RESTORE_HARDLINK`

Restore cached outputs by hard link when the filesystem cannot clone them.

Filesystems with clone support (APFS, Btrfs, XFS with reflink, ZFS) are unaffected: they clone either way. On other Unix filesystems, ext4 above all, this is the difference between a restore that writes nothing and one that copies every cached byte. On Windows, mbx copies whatever it cannot clone. A hard-linked output is the store's object, so it is read-only. mbx unlinks it before a compiler rewrites it, but Cargo run without mbx in the same target directory can fail with `output file ... is not writeable`. Disable to give every restored output a file of its own.

### `savings` [​](#savings)

- **Type:** `string`
- **Default:** `quips`
- **Set with:** `MBX_SAVINGS`

Style of the savings line printed after a build.

`quips` draws the line from a pool of dry one-liners, `plain` reports the same figures without a quip, and `off` prints nothing but keeps the totals.

**Choices:**

- `quips`
- `plain`
- `off`

### `scheduler.cgroup_root` [​](#scheduler-cgroup-root)

- **Type:** `option<path>`
- **Optional:** true
- **Set with:** `MBX_SCHEDULER_CGROUP_ROOT`

Absolute path to a delegated cgroup v2 directory for `scheduler.suspend`.

mbx must be able to write to it. mbx uses it only on Linux, and only while `scheduler.suspend` is in effect.

### `scheduler.cpus` [​](#scheduler-cpus)

- **Type:** `option<int>`
- **Optional:** true
- **Default:** logical CPUs
- **Set with:** `MBX_SCHEDULER_CPUS`

CPU core budget in the machine-wide pool before `scheduler.reserve_cpus`.

Compilers charge their thread counts; test binaries can use learned core counts. Memory has its own independent budget. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `scheduler.enabled` [​](#scheduler-enabled)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_SCHEDULER`

Coordinate real compilations machine-wide through a permit pool.

Cache hits never take a permit. A compilation that runs the compiler takes one first, so concurrent mbx builds on one machine share one pool. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `scheduler.memory` [​](#scheduler-memory)

- **Type:** `option<string>`
- **Optional:** true
- **Default:** 85% of physical memory, or of the Linux cgroup memory limit if lower
- **Set with:** `MBX_SCHEDULER_MEMORY`

Independent memory budget, such as `24GiB`, or `"none"`.

Requests charge predicted memory bytes separately from CPU cores. `"none"` schedules CPU only and turns off `scheduler.pressure` and `scheduler.suspend`. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `scheduler.pressure` [​](#scheduler-pressure)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_SCHEDULER_PRESSURE`

Delay additional compilations while memory is under pressure.

Works on Linux and macOS. Has no effect when `scheduler.memory` is `"none"`. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `scheduler.priority` [​](#scheduler-priority)

- **Type:** `string`
- **Default:** `normal`
- **Set with:** `MBX_SCHEDULER_PRIORITY`

Priority of this build's compilations in the machine-wide permit pool.

While a normal-priority build is waiting, a `low` build leaves about a quarter of the pool free for it. This may also be set in workspace `.mbx.toml`; the environment variable wins.

**Choices:**

- `normal`
- `low`

### `scheduler.reserve_cpus` [​](#scheduler-reserve-cpus)

- **Type:** `int`
- **Default:** `0`
- **Set with:** `MBX_SCHEDULER_RESERVE_CPUS`

CPU cores withheld from `scheduler.cpus` to leave CPUs for other work.

At least one core always remains; the memory budget is unaffected. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `scheduler.suspend` [​](#scheduler-suspend)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_SCHEDULER_SUSPEND`

Experimentally suspend Linux compiler process trees under memory pressure.

Takes effect only with `scheduler.cgroup_root` set, `scheduler.pressure` on, and `scheduler.memory` not `"none"`. When suspension cannot start, mbx warns once per build and only delays new compilations.

### `scheduler.tests` [​](#scheduler-tests)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_SCHEDULER_TESTS`

Run `cargo test` and `cargo nextest` binaries under the same permit pool.

nextest admits each test separately; `cargo test` admits each binary.

This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `seed_incremental` [​](#seed-incremental)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_SEED_INCREMENTAL`

Seed private incremental state from another checkout on rustc 1.98 or later.

Uses `.` as the working directory in debug info and backtrace frames. On stable and beta, enables compiler bootstrap only for private-state compilations, while still rejecting `#![feature]`. An existing `RUSTC_BOOTSTRAP` disables automatic flag injection. A user-supplied `-Zremap-cwd-prefix` still enables seeding when this setting is off.

### `share_out_dir` [​](#share-out-dir)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_SHARE_OUT_DIR`

Reuse Rust compilations across checkouts with matching build-script output.

mbx gives rustc a shared, content-addressed `OUT_DIR` in the cache directory and remaps generated source paths in Rust and C/C++ debug information. Disable to preserve Cargo's original `OUT_DIR` and paths. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `share_workspace_root` [​](#share-workspace-root)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_SHARE_WORKSPACE_ROOT`

Remap the workspace root so compilations do not record their checkout.

A crate rebuilt in a second checkout then comes out byte-identical, so its dependents still share. Source paths in debug information and panic messages name a placeholder instead. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `shims_dir` [​](#shims-dir)

- **Type:** `option<path>`
- **Optional:** true
- **Default:** `<cache_dir>/shims`
- **Set with:** `MBX_SHIMS_DIR`

Directory for the persistent compiler shims.

Containers that share a cache directory should each use a private, dedicated local directory that survives builds and contains no real compilers. A relative path resolves under `cache_dir` and cannot climb above it with `..` or normalize to an empty path.

### `stats_report` [​](#stats-report)

- **Type:** `option<path>`
- **Optional:** true
- **Set with:** `MBX_STATS_REPORT`

Write a JSON build report to this path.

### `summary` [​](#summary)

- **Type:** `string`
- **Default:** `auto`
- **Set with:** `MBX_SUMMARY`

Detail of the build summary printed after a build.

`auto` prints the explanatory `ci` report in CI and the one-line `short` summary locally; `short`, `ci`, `full`, and `off` select a fixed style.

**Choices:**

- `auto`
- `off`
- `short`
- `ci`
- `full`

### `target.evict_first` [​](#target-evict-first)

- **Type:** `option<list<string>>`
- **Optional:** true
- **Set with:** `MBX_TARGET_EVICT_FIRST`

Checkouts whose managed targets are collected first when over budget.

Entries, such as `.claude/worktrees`, are matched like `target.keep`.

### `target.keep` [​](#target-keep)

- **Type:** `option<list<string>>`
- **Optional:** true
- **Set with:** `MBX_TARGET_KEEP`

Checkouts whose managed targets are never collected for age or size.

An absolute path covers the checkouts under it; a relative one matches wherever it appears in a checkout's path. A leading `~` is your home directory. When `target.evict_first` also matches a checkout, the entry that reaches deeper into its path wins, and a tie keeps it. A kept target is still collected once its checkout is deleted.

### `target.lanes` [​](#target-lanes)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_TARGET_LANES`

Give `cargo check` and `cargo clippy` their own directory in the managed target.

They then run beside a build instead of waiting for Cargo's target lock.

### `target.max_age` [​](#target-max-age)

- **Type:** `duration`
- **Default:** `30d`
- **Set with:** `MBX_TARGET_MAX_AGE`

Collect managed targets, and build units inside them, unused this long, or `"none"`.

### `target.max_size` [​](#target-max-size)

- **Type:** `option<string>`
- **Optional:** true
- **Default:** 10% of the target disk, from 10 GiB to 100 GiB; shared budget when `gc.max_total_size` is set
- **Set with:** `MBX_TARGET_MAX_SIZE`

Size budget for managed targets, or `"none"`.

Over budget, mbx collects inactive managed targets least recently used first. Use `target.keep` to protect specific checkouts.

### `target.root` [​](#target-root)

- **Type:** `option<path>`
- **Optional:** true
- **Default:** `<cache_dir>/targets`
- **Set with:** `MBX_TARGET_ROOT`

Managed target root: the directory that holds managed targets.

A relative path resolves under `cache_dir`. Changing it moves each checkout's managed target on its next build; across filesystems, mbx removes the old outputs rather than copying them. NFS is unsupported for build outputs.

### `target.seed` [​](#target-seed)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_TARGET_SEED`

Copy registry build units from another checkout's managed target into a profile this checkout has not built yet (Cargo 1.100 or later).

### `target.views` [​](#target-views)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_TARGET_VIEWS`

Let mbx place eligible target directories under the managed target root.

### `verify` [​](#verify)

- **Type:** `bool`
- **Default:** `false`
- **Scope:** only from the environment or the command line
- **Set with:** `MBX_VERIFY`

Compile and consult the cache, then compare outputs.

### `verify_sample_rate` [​](#verify-sample-rate)

- **Type:** `int`
- **Default:** `0`
- **Set with:** `MBX_VERIFY_SAMPLE_RATE`

Percentage of compilation identities to verify (0–100), selected deterministically.

[Edit this page on GitHub](https://github.com/jdx/mr-boxington/edit/main/docs/configuration.md)

Last updated:

Pager

[Previous pageThe song](https://mr-boxington.jdx.dev/song)

[Next pageCLI commands](https://mr-boxington.jdx.dev/cli/)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)