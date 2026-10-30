# Crates and Dependencies

Read when changing Cargo manifests, dependencies, features, public crate boundaries or
`no_std` support.

## Toolchain and layout

- Inspect `rust-version`, edition, toolchain pins, lockfile and CI before choosing new
  syntax or dependencies. Preserve the supported toolchain; do not upgrade it to make
  a suggested idiom compile. For new crates, choose an edition compatible with the MSRV.
- Keep crates focused, but preserve the existing layout during ordinary changes.
  Split a crate only for a real responsibility or dependency boundary, not because a
  directory exists or a size threshold was reached. Account for version coordination,
  build overhead and cross-crate optimization before splitting.
- Reuse `[workspace.dependencies]` and workspace lint settings when already available;
  each package must opt into inherited settings. Cargo `[lints]` requires 1.74+; older
  toolchains need compatible configuration. See [Clippy and linting](clippy-and-linting.md).
- Prefer explicit startup over hidden pre-`main` constructors. Preserve required
  platform/framework initialization during ordinary changes.

## Dependency and feature contracts

- Add a dependency only when it earns its compile time, binary size and maintenance
  cost. Include build scripts and proc macros in that assessment. Follow project
  version policy; avoid wildcard versions or exact pins without a concrete reason.
- Preserve and commit application lockfiles. A `--locked` failure is a dependency-state
  question, not permission to run a broad `cargo update`.
- Keep features additive: enabling one should not remove or incompatibly change an
  existing API. Prefer capability names such as `serde`, `std` or `test-util`.
- Avoid feature-dependent public fields or required trait methods that make combinations
  incompatible. Respect existing mutually exclusive features and test supported sets
  separately; `--all-features` does not resolve conflicts.
- Feature-gate optional integrations when useful, and account for third-party types
  exposed in signatures. Use `cargo tree --duplicates` to inspect dependency overlap;
  inspect feature activation and build/proc-macro dependencies too. Do not assume all
  versions can be unified. Run project audit/license tools when dependency changes
  call for them; a style guide does not authorize changing the project's license.

## no_std and constrained targets

- Add `no_std` support for a concrete target or caller need. `core` supplies fundamental
  types; `alloc` adds heap-backed collections and pointers where allocator/target support
  exists. Check target support for required atomics rather than assuming `Arc` everywhere.
- Gate std integration additively, commonly with
  `#![cfg_attr(not(feature = "std"), no_std)]`; use capability names such as `std`/`alloc`.
  Hashing does not fundamentally require an OS: choose compatible collections, hashers
  and randomness policy. Use `try_reserve` when allocation failure must be recoverable.
- Build for the intended target and supported allocator configuration. Document native
  prerequisites; prefer normal Cargo builds and ship generated files when practical.
  Binding generation and drift checks belong at the [FFI boundary](unsafe-and-ffi.md).

## Compatibility and validation

- Removing or renaming public items, adding required bounds/methods, adding variants to
  exhaustive enums, changing constructible public structs, or losing dyn compatibility
  may break downstream callers. Assess before editing; deprecate and migrate according
  to the crate's version policy.
- Check default, no-default and relevant feature combinations when affected. Include
  supported targets and MSRV checks for changes that can alter compatibility.
- Prefer the project's existing CI commands over introducing another tool suite.
  Keep them reproducible locally with supported toolchain pins, detect generated-code
  drift, and investigate flaky tests rather than deleting their coverage. Report
  unsupported or untested combinations rather than implying universal coverage.
- Keep package description, license, repository, authors, keywords and categories
  accurate; avoid redundant metadata URLs. Document breaking changes and follow the
  project's release/tag policy. These checks do not authorize tagging or publishing.
