Configuration

Configuration [​](#configuration)

Defaults work without a configuration file. Add only the values you want to change. mbx reads configuration from three places; the first value found wins:

1. Environment variables ( `MBX_*`).
2. `.mbx.toml` at the resolved Cargo workspace root, for the [supported workspace settings](#workspace-policy) only.
3. `mbx/config.toml` in the platform configuration directory:
   - Linux: `~/.config/mbx/config.toml`, honoring `$XDG_CONFIG_HOME`
   - macOS: `~/Library/Application Support/mbx/config.toml`
   - Windows: `%APPDATA%\mbx\config.toml`

Anything still unset takes its default. Unknown TOML keys are rejected, so a misspelled setting is an error.

## Common adjustments [​](#common-adjustments)

| Change | Setting or guide |
| --- | --- |
| Leave capacity for your editor | `scheduler.reserve_cpus = 2`; [parallel builds](https://mr-boxington.jdx.dev/scheduling) |
| Keep the action store under a fixed size | `gc.max_size = "20GiB"` |
| Keep live targets longer | `target.max_age = "60d"`; [managed targets](https://mr-boxington.jdx.dev/managed-targets) |
| Use factual savings messages | `savings = "plain"` |
| Print more cache detail | `summary = "full"`; [cache results](https://mr-boxington.jdx.dev/cache-results) |
| Share results with CI | [Remote cache](https://mr-boxington.jdx.dev/remote-cache) |

Use TOML section headers for dotted settings, as shown in the example below. Shell examples that set `NAME=value command` use POSIX syntax; PowerShell users can set `$env:NAME` before the command and remove it afterward.

## Local build storage [​](#local-build-storage)

Keep the working cache and build outputs on local storage. Remote caches are configured separately under `[remote]`.

On Linux and macOS, mbx rejects NFS-backed working caches and Cargo output storage before starting a build. Set `cache_dir` (`MBX_CACHE_DIR`) and, when configured separately, `target.root` (`MBX_TARGET_ROOT`) to local storage. User-selected Cargo target directories and separate intermediate build directories must also be local: check `CARGO_TARGET_DIR` / `build.target-dir` and `CARGO_BUILD_BUILD_DIR` / `build.build-dir`.

The check follows symlinks and checks the destination filesystem even when the directory has not been created yet. An NFS source checkout is supported when its build outputs are local, including a `target` link into a local managed target directory. Remote cache URLs are unaffected; use a [remote cache server](https://mr-boxington.jdx.dev/remote-cache) to share results across machines. Other platforms do not currently enforce this filesystem check.

Help, cache inspection, and cleanup commands remain available with the old configuration so you can inspect or remove previous NFS storage. Changing the configuration does not copy the cache to the new disk; expect a cold cache. See [changing target placement](https://mr-boxington.jdx.dev/managed-targets#change-target-placement) before moving managed targets to another disk.

## Disk-scaled defaults [​](#disk-scaled-defaults)

Three size budgets default to a share of the disk holding their data: 5% for the action store (`gc.max_size`), 10% for managed target directories (`target.max_size`), and 5% for learned incremental state (`gc.incremental_max_size`), each bounded at both ends. Managed targets and learned incremental state are also collected after 30 days unused. The table in [managed target directories](https://mr-boxington.jdx.dev/managed-targets#budgets-scale-with-the-disk) lists the bounds and what collection removes.

Setting an explicit budget overrides the scaling; `"none"` disables `target.max_size`, `target.max_age`, `gc.incremental_max_size`, `gc.incremental_max_age`, and `gc.max_total_size`.

## Example [​](#example)

This example shows several available controls, not a recommended configuration. Copy only the settings you need into your global configuration file. Remote settings and machine-specific paths do not belong in a checked-in `.mbx.toml`.

<details>

<summary>Example global configuration</summary>



toml

```
# <config directory>/mbx/config.toml
cache_dir = "/var/cache/mbx"
incremental = false
learned_incremental_max_size = "8GiB"  # or "none"
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
max_total_size = "50GiB" # optional action + target + incremental budget
interval = "1h"

[target]
views = true
max_size = "30GiB"       # default: 10% of the cache disk
max_age = "30d"          # default

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

## Managed linkers [​](#managed-linkers)

Select a linker for each Cargo profile and target, or override it for one build with `MBX_LINKER`. See [Managed linkers](https://mr-boxington.jdx.dev/linkers) for selectors, prerequisites, and complete examples.

## Workspace policy [​](#workspace-policy)

A repository may check in a `.mbx.toml` containing the build-policy switches and scheduler policy below:

toml

```
incremental = false
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

Environment variables still win. Machine paths, remote-cache configuration, credentials, diagnostics, target placement, and garbage collection are not accepted from a repository-owned file. mbx reports an error for an unsupported or misspelled workspace setting.

`share_out_dir = true` is the global default. It lets Rust compilations reuse cached artifacts across checkouts with matching build-script output by giving rustc a shared copy of that output as `OUT_DIR`. It also remaps generated source paths in Rust and C/C++ debug information. Set it to false when a build needs Cargo's original `OUT_DIR` or literal generated source paths; Rust compilations that read `OUT_DIR` then remain checkout-specific. See [`OUT_DIR` sharing](https://mr-boxington.jdx.dev/limits#out-dir-sharing) for eligibility and compatibility details.

`share_workspace_root = false` is the global default. Setting it to true maps the workspace root to a placeholder wherever rustc records a source path, so a crate rebuilt in a second checkout comes out byte-identical and the crates above it still share. It is worth turning on for a machine that builds many checkouts of one repository, and costs literal source paths in debug information, `file!()` and panic locations. See [A rebuilt workspace crate records its checkout](https://mr-boxington.jdx.dev/limits).

`build_script_execution = true` (`MBX_BUILD_SCRIPT_EXECUTION`) caches eligible `build.rs` executions. Set it to false to keep compilation caching while every build script runs normally.

## Build-script C and C++ [​](#build-script-c-and-c)

`cc = true` (`MBX_CC`, on by default) caches host C and C++ compiled by Cargo build scripts, such as the native code built by `*-sys` crates. No project changes are required: for the duration of the mbx command, build scripts use mbx's compiler wrappers.

mbx preserves a host compiler selected with `CC`, `CXX`, `HOST_CC`, or `HOST_CXX`, and does not cache those compiles. For a cross-compile, mbx does not guess the target toolchain. It caches only when the build names a compiler with `CC_<target>`, `CXX_<target>`, `TARGET_CC`, or `TARGET_CXX`; mbx wraps that compiler without replacing the build's choice.

If mbx cannot safely model a compiler call, it runs the real compiler without caching that call. Use `mbx explain` to see why a build bypassed the cache, or read the [full C and C++ limits](https://mr-boxington.jdx.dev/limits#c-and-c-caching-covers-the-host-compiles-mbx-drives).

To cache C and C++ builds that run outside Cargo, put the build command after `mbx exec`. See [cache C and C++ builds outside Cargo](https://mr-boxington.jdx.dev/standalone-builds).

## Machine-wide compile scheduling [​](#machine-wide-compile-scheduling)

Simultaneous mbx builds share CPU and memory permits. Set `scheduler.cpus`, `scheduler.reserve_cpus`, `scheduler.memory`, and `scheduler.priority` to tune that pool. See [Parallel builds](https://mr-boxington.jdx.dev/scheduling) for examples and the difference between a shared budget and Cargo's per-build `-j` limit.

## Verify mode [​](#verify-mode)

`MBX_VERIFY=1` compiles and consults the cache side by side and compares the results. It is expensive; use it to investigate correctness, not for everyday builds.

For routine checks, set `MBX_VERIFY_SAMPLE_RATE=5` (or `verify_sample_rate = 5`) to verify approximately 5% of compilation identities. The range is 0–100; 0 disables sampling. Selection is stable across wrapper processes and build order, so rerunning the same invocation selects the same sample. This samples units, not elapsed compiler time. `MBX_VERIFY=1` takes precedence and verifies all eligible units. Selected units rehash inputs and disable learned incremental compilation, just like full verification.

The build reports what it found:

text

```
mbx[cache]: qualification: 24 verified, 0 diverged
```

The verified count includes divergent compilations. Each compilation contributes at most one divergence, reporting its first mismatch. Warnings identify the adapter, unit and action; stdout/stderr differences include the first differing byte offset, line number and bounded, escaped excerpts of both results. Cached diagnostics are rewritten into this checkout's paths before comparison.

Cargo must actually invoke the compiler to verify anything. Run in the checkout that filled the cache with a fresh target directory; an unchanged build in an existing target can be a Cargo no-op. Keep the original target and shared store. `MBX_BYPASS_LOG` and `mbx explain` show what was left out.

For audits across worktrees, populate and verify using the same virtual source root. For example, run this from each checkout's workspace root, first with `MBX_VERIFY=0` to populate, then with `MBX_VERIFY=1` in the other checkout:

sh

```
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--remap-path-prefix=$PWD=/workspace" \
  CARGO_TARGET_DIR="$PWD/target-audit" MBX_VERIFY=1 mbx build --all-targets --locked
```

Use a fresh `target-audit` directory each time and the same toolchain, profile, and other compiler flags. The source side of `--remap-path-prefix` is keyed portably; its virtual destination must agree between checkouts. If using `CARGO_ENCODED_RUSTFLAGS`, add the remap there instead: Cargo gives it precedence over `RUSTFLAGS`.

Remapping reduces embedded-source-path differences; it does not guarantee byte-identical outputs, especially for native links or paths outside the mapped root. See [artifact equivalence](https://mr-boxington.jdx.dev/limits#restored-artifacts-are-equivalent-not-always-identical). Investigate remaining divergences rather than treating every cross-worktree mismatch as harmless. Please report unexplained differences, including the identified unit and action.

Use verification to check a caching feature against your own workload, including [native link caching](https://mr-boxington.jdx.dev/limits#native-linking-is-cached-only-where-the-linker-can-be-described).

## The savings line [​](#the-savings-line)

`savings` controls the one-line report of accumulated savings after a build (`MBX_SAVINGS` from the environment). `quips`, the default, draws the line from a pool of dry one-liners. `plain` reports the same figures without a quip. `off` keeps the totals without printing anything.

## Build summaries [​](#build-summaries)

`summary` controls the cache report printed to stderr after a build (`MBX_SUMMARY` from the environment). `auto`, the default, selects `ci` when `CI` or `GITHUB_ACTIONS` is `1`, `true`, or `yes` (case-insensitive), and `short` otherwise. `short` prints one line and leaves routine `compiler-query` and `standard-input` probes out of its bypass count. `ci` adds session timing, estimated compiler time avoided, explanations for compilations that could not be looked up, and bypass reasons. Its object-cache counts and transfers exclude artifacts Cargo reused directly and archives restored or saved by a CI action. Compiler time avoided is summed across compilations, not elapsed job time saved. CI also skips the first-build notice about local cache management.

Set a fixed style to override automatic selection. `full` prints detailed timing, compiler, bypass, transfer, and output-restoration figures. `off` prints no cache summary, while still writing `MBX_STATS_REPORT` when configured. Cargo's `-q` and `--quiet` also suppress the summary for that invocation.

## Incremental builds [​](#incremental-builds)

Leave `MBX_INCREMENTAL` unset for mbx's default combination of shared caching and private incremental state. `MBX_INCREMENTAL=1` hands control to Cargo and reduces reuse across checkouts. See [Incremental builds](https://mr-boxington.jdx.dev/incremental).

## Learned incremental reuse [​](#learned-incremental-reuse)

mbx recognizes source edits and retains private state for the affected crates. `learned_incremental_max_size` bounds that state per crate; its default is `8GiB`. See [Learned incremental reuse](https://mr-boxington.jdx.dev/incremental#learned-incremental-reuse) for triggers, cleanup, and overrides.

## Sizes and durations [​](#sizes-and-durations)

Sizes accept SI and IEC units. `20GB` and `20GiB` are different values. Durations accept values such as `30s`, `15m`, and `1h`.

## Settings [​](#settings)

The complete setting reference is generated from mbx's runtime declarations. Environment-only settings are labeled in the entries below.

### `ar_determinism` [​](#ar-determinism)

- **Type:** `string`
- **Default:** `auto`
- **Set with:** `MBX_AR_DETERMINISM`

Set `ZERO_AR_DATE` for build scripts so native archives stop embedding a timestamp. Without it, tools like CMake's `ar` rewrite an archive's header on every build, moving its digest and missing every cached action downstream even when no member changed. Auto normalizes every profile except `release`, leaving published artifacts byte-for-byte as the host toolchain made them; always covers `release` too; off leaves the toolchain alone. A `ZERO_AR_DATE` you set yourself always wins.

**Choices:**

- `auto`
- `always`
- `off`

### `build_script_execution` [​](#build-script-execution)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_BUILD_SCRIPT_EXECUTION`

Cache executions of build scripts using Cargo's freshness inputs. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `bypass_log` [​](#bypass-log)

- **Type:** `option<path>`
- **Optional:** true
- **Scope:** only from the environment or the command line
- **Set with:** `MBX_BYPASS_LOG`

Append the full reason for every bypassed compilation to this path.

### `cache_dir` [​](#cache-dir)

- **Type:** `option<path>`
- **Optional:** true
- **Default:** platform cache directory
- **Set with:** `MBX_CACHE_DIR`

Cache root. NFS is unsupported for local build storage.

### `cache_links` [​](#cache-links)

- **Type:** `bool`
- **Default:** `true`
- **Scope:** only from the environment or the command line
- **Set with:** `MBX_CACHE_LINKS`

Cache natively linked test binaries, executables, and proc macros. On macOS this also passes ld64 `-oso_prefix` so a debug-info link's debug map stops naming this checkout, which is what lets it cache. Supported on Linux, macOS, and Windows; a link mbx cannot describe exactly still links normally.

### `cc` [​](#cc)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_CC`

Cache C and C++ compilations run by build scripts.

### `cc_store_path_specific` [​](#cc-store-path-specific)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_CC_STORE_PATH_SPECIFIC`

Store C objects that embed absolute paths under checkout-specific keys. Disable for disposable worktrees to avoid storing objects that cannot be reused at another path. Existing entries may still be restored.

### `display` [​](#display)

- **Type:** `string`
- **Default:** `auto`
- **Set with:** `MBX_DISPLAY`

Cargo display mode. Plain disables animated output even in a terminal.

**Choices:**

- `auto`
- `plain`

### `events` [​](#events)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_EVENTS`

Record a per-compilation event stream for `mbx tui` to watch.

### `events_max_size` [​](#events-max-size)

- **Type:** `string`
- **Default:** `16MiB`
- **Set with:** `MBX_EVENTS_MAX_SIZE`

How much per-compilation history one build may record, or "none" for no limit. Past this the counters carry on but the rows stop, and `mbx explain` says so.

### `forward_compiler_notifications` [​](#forward-compiler-notifications)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_FORWARD_COMPILER_NOTIFICATIONS`

Forward rustc's diagnostics and artifact notifications to Cargo as the compiler prints them, so Cargo can start a dependent against this crate's metadata while its code generation continues. Turn it off to hold the compiler's output until mbx has stored the result, which is useful when diagnosing the shim itself.

### `gc.auto` [​](#gc-auto)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_GC_AUTO`

Sweep after a build when collection is due.

### `gc.incremental_max_age` [​](#gc-incremental-max-age)

- **Type:** `duration`
- **Default:** `30d`
- **Set with:** `MBX_GC_INCREMENTAL_MAX_AGE`

Collect learned incremental state unused this long, or "none".

### `gc.incremental_max_size` [​](#gc-incremental-max-size)

- **Type:** `option<string>`
- **Optional:** true
- **Default:** 5% of the cache disk, from 10GiB to 100GiB
- **Set with:** `MBX_GC_INCREMENTAL_MAX_SIZE`

Aggregate learned-incremental budget, or "none". Inactive checkouts are collected oldest-first while the most recently used checkout is kept.

### `gc.interval` [​](#gc-interval)

- **Type:** `duration`
- **Default:** `1h`
- **Set with:** `MBX_GC_INTERVAL`

Minimum interval between automatic sweeps.

### `gc.max_size` [​](#gc-max-size)

- **Type:** `option<string>`
- **Optional:** true
- **Default:** 5% of the cache disk, from 5GiB to 500GiB
- **Set with:** `MBX_GC_MAX_SIZE`

Action-store and per-session remote-download budget.

### `gc.max_total_size` [​](#gc-max-total-size)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_GC_MAX_TOTAL_SIZE`

Combined action-store, managed-target, and learned-incremental budget, or "none".

### `http.download_timeout` [​](#http-download-timeout)

- **Type:** `duration`
- **Default:** `10m`
- **Set with:** `MBX_HTTP_DOWNLOAD_TIMEOUT`

Deadline for one blob download, retries and backoff included.

### `http.read_stall_budget` [​](#http-read-stall-budget)

- **Type:** `duration`
- **Default:** `90s`
- **Set with:** `MBX_HTTP_READ_STALL_BUDGET`

Wall clock a build may lose to failed remote reads before it stops reading and just compiles. "0" keeps reading however long it takes.

### `http.retries` [​](#http-retries)

- **Type:** `int`
- **Default:** `3`
- **Set with:** `MBX_HTTP_RETRIES`

Request retries.

### `http.timeout` [​](#http-timeout)

- **Type:** `duration`
- **Default:** `30s`
- **Set with:** `MBX_HTTP_TIMEOUT`

Connect and request timeout.

### `incremental` [​](#incremental)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_INCREMENTAL`

Let local workspace members compile incrementally.

### `learned_incremental` [​](#learned-incremental)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_LEARNED_INCREMENTAL`

Compile crates that keep missing the cache with changed content incrementally, keeping their outputs out of the shared cache.

### `learned_incremental_max_size` [​](#learned-incremental-max-size)

- **Type:** `string`
- **Default:** `8GiB`
- **Set with:** `MBX_LEARNED_INCREMENTAL_MAX_SIZE`

How much learned incremental state one crate may keep, or "none". State past this is discarded before the crate compiles again.

### `linker.default` [​](#linker-default)

- **Type:** `string`
- **Default:** `system`

Linker used when the active Cargo profile has no matching selection.

### `linker.profiles` [​](#linker-profiles)

- **Type:** `option<map<string, map<string, string>>>`
- **Optional:** true

Linkers selected by Cargo profile and target triple.

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

Log filter such as `debug` or `mbx=trace`. It covers every log the mbx process emits, its own and those of the libraries it builds on. The default keeps the pty library behind the inline build view quiet, because mbx falls back to plain Cargo when that view cannot start.

### `pretty_inspect` [​](#pretty-inspect)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_PRETTY_INSPECT`

Open the terminal warning browser after a successful Cargo build.

### `remote.mode` [​](#remote-mode)

- **Type:** `string`
- **Default:** `read-write`
- **Set with:** `MBX_REMOTE_MODE`

Remote access mode.

**Choices:**

- `read-write`
- `read-only`
- `write-only`

### `remote.namespace` [​](#remote-namespace)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_NAMESPACE`

Remote namespace; required when a URL is configured.

### `remote.oidc_audience` [​](#remote-oidc-audience)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_OIDC_AUDIENCE`

CI OIDC audience.

### `remote.s3_conditional_writes` [​](#remote-s3-conditional-writes)

- **Type:** `string`
- **Default:** `auto`
- **Set with:** `MBX_REMOTE_S3_CONDITIONAL_WRITES`

How to treat an S3 store that does not implement conditional writes.

**Choices:**

- `auto`
- `required`
- `off`

### `remote.s3_endpoint` [​](#remote-s3-endpoint)

- **Type:** `option<url>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_S3_ENDPOINT`

S3 endpoint for a store that is not AWS, such as MinIO or R2.

### `remote.s3_force_path_style` [​](#remote-s3-force-path-style)

- **Type:** `option<bool>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_S3_FORCE_PATH_STYLE`

Address S3 buckets in the path rather than the host.

### `remote.s3_region` [​](#remote-s3-region)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_S3_REGION`

S3 region; Cloudflare R2 uses "auto".

### `remote.token` [​](#remote-token)

- **Type:** `option<string>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_TOKEN`

Bearer token for the remote cache.

### `remote.token_file` [​](#remote-token-file)

- **Type:** `option<path>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_TOKEN_FILE`

File containing a bearer token.

### `remote.url` [​](#remote-url)

- **Type:** `option<url>`
- **Optional:** true
- **Set with:** `MBX_REMOTE_URL`

Remote cache URL.

### `savings` [​](#savings)

- **Type:** `string`
- **Default:** `quips`
- **Set with:** `MBX_SAVINGS`

How the savings line after a build reads.

**Choices:**

- `quips`
- `plain`
- `off`

### `scheduler.cpus` [​](#scheduler-cpus)

- **Type:** `option<int>`
- **Optional:** true
- **Default:** logical CPUs
- **Set with:** `MBX_SCHEDULER_CPUS`

Machine-wide concurrent compile permits.

### `scheduler.enabled` [​](#scheduler-enabled)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_SCHEDULER`

Coordinate real compilations machine-wide through a permit pool.

### `scheduler.memory` [​](#scheduler-memory)

- **Type:** `option<string>`
- **Optional:** true
- **Default:** 85% of physical memory
- **Set with:** `MBX_SCHEDULER_MEMORY`

Memory budget the permits divide, or "none" for plain CPU permits.

### `scheduler.priority` [​](#scheduler-priority)

- **Type:** `string`
- **Default:** `normal`
- **Set with:** `MBX_SCHEDULER_PRIORITY`

Permit priority of this build's compilations.

**Choices:**

- `normal`
- `low`

### `scheduler.reserve_cpus` [​](#scheduler-reserve-cpus)

- **Type:** `int`
- **Default:** `0`
- **Set with:** `MBX_SCHEDULER_RESERVE_CPUS`

Logical CPUs to leave free for the rest of the machine.

### `share_out_dir` [​](#share-out-dir)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_SHARE_OUT_DIR`

Reuse Rust compilations across checkouts with matching build-script output by giving rustc a shared, content-addressed `OUT_DIR` under the cache. Also remap generated source paths in Rust and C/C++ debug information. Disable to preserve Cargo's original `OUT_DIR` and paths.

### `share_workspace_root` [​](#share-workspace-root)

- **Type:** `bool`
- **Default:** `false`
- **Set with:** `MBX_SHARE_WORKSPACE_ROOT`

Remap the workspace root so rustc does not record which checkout a compilation ran in, which lets a crate rebuilt in a second checkout come out byte-identical so its dependents still share. Source paths in debug information and panic messages then name a placeholder. This may also be set in workspace `.mbx.toml`; the environment variable wins.

### `stats_report` [​](#stats-report)

- **Type:** `option<path>`
- **Optional:** true
- **Set with:** `MBX_STATS_REPORT`

Write a JSON build report to this path.

### `summary` [​](#summary)

- **Type:** `string`
- **Default:** `auto`
- **Set with:** `MBX_SUMMARY`

Detail printed after a build. Auto uses an explanatory CI report in CI and one line locally; short, ci, full, and off select a fixed style.

**Choices:**

- `auto`
- `off`
- `short`
- `ci`
- `full`

### `target.max_age` [​](#target-max-age)

- **Type:** `duration`
- **Default:** `30d`
- **Set with:** `MBX_TARGET_MAX_AGE`

Collect live managed targets unused this long, or "none".

### `target.max_size` [​](#target-max-size)

- **Type:** `option<string>`
- **Optional:** true
- **Default:** 10% of the cache disk, from 10GiB to 100GiB
- **Set with:** `MBX_TARGET_MAX_SIZE`

Managed-target budget, or "none". Live views are collected oldest-first.

### `target.root` [​](#target-root)

- **Type:** `option<path>`
- **Optional:** true
- **Default:** \<cache\_dir>/targets
- **Set with:** `MBX_TARGET_ROOT`

Managed target root. NFS is unsupported for build outputs.

### `target.views` [​](#target-views)

- **Type:** `bool`
- **Default:** `true`
- **Set with:** `MBX_TARGET_VIEWS`

Let mbx place eligible target directories under the managed root.

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

[Previous pageFAQ](https://mr-boxington.jdx.dev/faq)

[Next pageCLI commands](https://mr-boxington.jdx.dev/cli/)

MIT License·Copyright © 2026· [jjdx.dev](https://jdx.dev)