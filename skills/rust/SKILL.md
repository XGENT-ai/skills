---
name: rust
description: Use when writing, reviewing, refactoring, testing or debugging Rust code. Covers ownership, errors, performance and Tokio async patterns.
when_to_use: 'Requests like "why does the borrow checker reject this", "should this be Arc or a reference", or "write this Tokio task".'
license: MIT
compatibility: Rust 1.70+ (1.75+ for async fn in traits), Cargo
allowed-tools: Bash(cargo:*) Bash(rustc:*) Bash(rustfmt:*) Bash(clippy:*) Read Write Edit Glob Grep
---

# Rust Best Practices

Apply these guidelines when writing, reviewing, refactoring, testing or debugging Rust code.

## Working Approach

- Read the affected code, tests, `Cargo.toml`, toolchain pin and CI commands first.
  Respect the project's edition, MSRV, feature combinations and conventions.
- If the affected crate uses Bevy, read [Bevy practices](references/bevy.md) before
  applying generic borrowing, import or lint advice to system functions.
- Keep application code direct: concrete types, simple control flow and no speculative
  traits or builders. Libraries with external callers need deliberate public contracts.
- Model the domain with Rust's ownership, enums, traits and `Result`; do not translate
  inheritance or exception patterns mechanically from another language.
- During refactoring, preserve public signatures, error variants, output ordering and
  behavior unless changing them is in scope. Explain necessary contract changes; do not
  rename getters or replace existing parameters merely to satisfy a style preference.
- Read-only reviews produce findings, not edits. Stay within the requested change;
  installing tools, upgrading dependencies or publishing is not implied by this skill.

## References by Task

Read the relevant references before making decisions. Each topic owns its detailed rules
and examples; follow cross-links only when the task crosses that boundary.

| Task | Reference |
|---|---|
| Borrow-checker friction, iterators, imports or function extraction | [Coding idioms](references/coding-idioms.md) |
| Clippy warnings, suppressions or workspace lint configuration | [Clippy and linting](references/clippy-and-linting.md) |
| Profiling, allocations, cloning or hot paths | [Performance](references/performance.md) |
| Fallible operations, library error types or resource cleanup | [Error handling](references/error-handling.md) |
| Regression, property, integration or snapshot tests; controllable I/O | [Testing](references/testing.md) |
| Closure bounds, trait contracts, static dispatch or trait objects | [Generics and dispatch](references/generics-and-dispatch.md) |
| Compile-time state transitions | [Type state](references/type-state.md) |
| Comments, public docs or executable examples | [Documentation](references/documentation.md) |
| Lifetimes, smart pointers, interior mutability or shared ownership | [Pointers and ownership](references/pointers-and-ownership.md) |
| Tokio tasks, channels, cancellation or async shared state | [Async with Tokio](references/async-tokio.md) |
| Public types, constructors, standard traits or API evolution | [API design](references/api-design.md) |
| Unsafe blocks, raw pointers, native handles or foreign interfaces | [Unsafe and FFI](references/unsafe-and-ffi.md) |
| Cargo manifests, dependencies, features, MSRV or no_std | [Crates and dependencies](references/crates-and-dependencies.md) |
| Declarative/procedural macros, generated APIs or expansion diagnostics | [Macros](references/macros.md) |
| Queued actions, configurable algorithms, AST traversal or tree transformation | [Behavioral patterns](references/behavioral-patterns.md) |
| Bevy systems, queries, scheduling, deferred changes or ECS tests | [Bevy](references/bevy.md) |

## Core Rules

- Make ownership intentional: borrow to read, move to consume, and share only when the
  design needs multiple owners. Resolve borrow scopes before adding deep copies.
- Use types to express domain invariants and keep public contracts deliberate. Prefer
  concrete code; introduce traits, builders, macros or type states for an actual need.
- Return `Result` for fallible operations and `Option` for normal absence. Never use
  `unwrap()`/`expect()` outside tests. Use `thiserror` for library errors and `anyhow`
  for binaries; preserve causes and add useful context. Do not use panics or
  `catch_unwind` as ordinary error handling. Keep `Drop` non-panicking.
- Measure performance in release builds before changing ownership, allocation, dispatch
  or synchronization. Keep loops when they express control flow more clearly.
- Keep blocking I/O and heavy CPU work off async workers. End synchronous lock guards
  before `.await`; choose async synchronization only when the operation requires it.
  Bound concurrency, handle cancellation and give outbound calls timeouts.
- Keep unsafe code behind a small sound boundary, document each unsafe operation's
  invariants, and document caller obligations in `# Safety` for every `unsafe fn`.
- Keep formatting and lint policy compatible with the project and MSRV. Use narrow,
  justified suppressions; enforce warnings-as-errors through CI rather than blanket
  `#![deny(warnings)]` in source.
- Test observable behavior, including failures and boundaries. Reproduce bugs before
  fixing them; use unit, integration and doc tests for their respective contracts.
- Use `//` for rationale and `///`/`//!` for API usage and contracts. Link TODOs to issues,
  document errors/panics/safety where relevant, and enforce library documentation with
  `#![deny(missing_docs)]`. Keep fallible doc examples executable with `?`.

## Verification Before Handoff

Use the project's commands and supported feature/target matrix. For code changes, run
format checks, Clippy and relevant tests; include doc tests and `cargo doc --no-deps`
for public API/doc changes. Check new syntax and dependencies against the MSRV. Unsafe,
FFI or performance changes also need applicable Miri, platform tests or release benchmarks.
Report what actually ran and any limitations; do not claim unrun checks passed.
