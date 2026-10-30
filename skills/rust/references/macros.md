# Macros and Generated APIs

Read when writing or reviewing declarative macros, proc macros or generated Rust APIs.

## Choose generation for a concrete need

Prefer functions, types, traits and ordinary control flow when they express the operation.
Use `macro_rules!` for useful repetition or declarative syntax, such as keeping one table
of definitions synchronized. Choose a proc macro when syntax analysis/transformation
earns its build cost, IDE limitations and diagnostic complexity. A normal example may
be clearer than a new macro abstraction.

## Preserve the caller's contract

- Make syntax resemble the Rust it produces and expose control-flow effects. Document
  generated public items and signature transformations; do not hide surprising APIs.
- Generated items are private unless callers request visibility. Forward attributes
  such as `cfg`, docs and derives to the intended items, and document any restrictions.
- Accept ordinary type syntax, including primitive, generic and qualified types. Do not
  move caller types into an inaccessible generated module or require accidental imports.
- Keep name resolution hygienic across supported invocation scopes and dependency
  renaming. In declarative macros, use `$crate` for defining-crate paths where appropriate.
- Evaluate expression arguments the documented number of times, normally once. Test
  side effects and borrowing rather than assuming a textual expansion behaves like a call.

## Keep proc macros maintainable

- Keep entrypoints thin and substantial transformation logic in normally testable code.
  Add a helper crate only when it earns its maintenance cost; tiny macros do not need
  a prescribed multi-crate architecture.
- Generated code may need runtime helpers through a hidden re-export. That path remains
  externally reachable even with `#[doc(hidden)]`; account for visibility, compatibility
  and renamed dependencies.
- Exact companion versions and coordinated releases may be necessary when generated
  code tightly couples to runtime helpers. Do not apply that pinning rule to unrelated
  dependencies; follow [crate compatibility](crates-and-dependencies.md).

## Verify expansion and diagnostics

Use the project's compile-pass/compile-fail tests (for example, an existing `trybuild`
suite) to cover accepted syntax and rejected input. Include visibility and attribute
forwarding, generic/qualified types, supported module/function/nested invocation sites,
name collisions and renamed dependencies. Verify argument evaluation behavior and
actionable diagnostics. Inspect expansions when useful; expansion snapshots alone do
not prove generated code compiles or behaves correctly.
