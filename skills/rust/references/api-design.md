# Public API Design

Read when adding or reviewing public types, constructors, traits or signatures. Apply
these choices to new APIs; changing an existing API requires a compatibility decision.
Application-local code should stay concrete unless reuse or a real boundary needs more.

## Names and signatures

- Follow std naming: `UpperCamelCase` types/traits/variants, `snake_case` functions and
  modules, `SCREAMING_SNAKE_CASE` constants. Treat acronyms as words (`Uuid`, `HttpClient`)
  and prefer specific domain names over vague suffixes or redundant module prefixes.
  Name errors by the failed operation, such as `ParseAddrError`.
- Use a method for receiver behavior; otherwise choose a free or associated function
  by domain meaning. Keep analogous parameter ordering consistent, normally with
  closures last. Essential operations should be discoverable without obscure trait imports.
- Getters normally omit `get_`: use `field`, `field_mut`, `set_field`, `take_field` or
  `replace_field` as appropriate. Pair `len` with `is_empty`; do not add unchecked access
  just to complete a pair. Closure accessors must define borrowing/reentrancy behavior.
- Return values in tuples or structs instead of out-parameters unless an in-place API
  or reusable caller buffer justifies them. Return useful work already computed, such
  as an insertion index or replaced value, when it saves callers repeating the operation.

## Domain types and construction

- Use newtypes to distinguish IDs and units. A type alias does not prevent mixing them.
  Keep invariant-bearing fields private and validate through `TryFrom`, `FromStr` or a
  fallible constructor; an infallible `From` must not accept invalid raw values.
  A newtype also permits local trait implementations around a foreign type. Forward
  only meaningful traits; use `repr(transparent)` when layout equivalence is contractual.
- Use enums for mutually exclusive modes or states, `Option` for normal absence, and
  named fields for compound data. Replace ambiguous boolean choices in new APIs with
  named modes; a clear boolean setter need not become an enum. Independent flags may
  use a bitflag type; a single choice remains an enum.
- Prefer `Path`/`PathBuf` for paths and `Duration` for elapsed time. Do not weaken a
  useful domain type merely to expose a primitive at a boundary.
- Prefer `new`, domain verbs such as `open`/`connect`, and conventional conversion names:
  `as_` borrows cheaply, `to_` performs conversion work, `into_` consumes the receiver.
  Use `as_mut_slice`, not `as_slice_mut`; place conversions on the more specific type.
- Make `new()` and `Default` agree when both represent the same default construction.
  Do not add `Default` to a type that has no meaningful default or requires external I/O.
- Introduce a builder when related options or cross-field validation justify it.
  Validate the complete configuration at `build`; setters may reject individual invalid
  values. Consuming setters move inputs; borrowing setters support staging and reuse.
  Prefer defaults and struct-update syntax for simple options, and do not introduce a
  builder just because a private function has several arguments.

## Ownership and standard traits

- Follow the [ownership choices](coding-idioms.md#11-borrowing-over-cloning) for inputs.
  Generalize with `AsRef<Path>`, `IntoIterator` or `RangeBounds` when callers benefit;
  weigh inference, compile-time and code-size costs. Returning an iterator avoids forced
  allocation; return an owned collection when callers need its ownership/indexing.
- Generic `Read`/`Write` parameters may be taken by value: callers can pass `&mut reader`
  to retain ownership. Document consumption and buffering; accept caller-provided I/O
  using the project's synchronous or async traits.
- Keep smart-pointer and interior-mutability details private unless ownership or sharing
  is itself the contract. `Box<dyn Trait>` and shared handles are legitimate boundaries;
  do not mechanically remove them from existing signatures.
- Implement applicable `Debug`, `Clone`, `Copy`, equality, ordering, `Hash` and `Default`.
  Give even empty values recognizable `Debug` output. Redact secrets in diagnostic and
  user formatting; test sensitive formatting. Do not derive `Clone` for a unique resource
  or `Copy` for a large value just because its fields allow it.
- If ordering ignores fields, equality must agree and hashing must hash equal values
  identically. Derive together only when field-wise semantics are correct. Test these
  laws when implementations are custom; do not blindly derive equality beside custom
  order or force a partial order into `Ord` when the domain has no total order.
- Implement `From`/`TryFrom`, not `Into`/`TryInto` directly. Use `FromStr` for parsing,
  `Display` for user-facing text, and `FromIterator`/`Extend` for collection construction.
  Do not hide a lossy conversion in `From` or an unchecked `as` cast. Use operator traits
  for natural domain operations, and numeric formatting traits only when meaningful.
- Expose useful owned, borrowed and mutable iteration, with conventional iterator names
  such as `Iter`, `Keys`, `IntoIter`. Keep `size_hint` truthful; implement
  `DoubleEndedIterator`/`ExactSizeIterator` only when their contracts hold.
- Use `#[must_use]` when ignoring a result, lazy value or pure transformation is likely
  a bug. Optional Serde integration must validate invariants on deserialization rather
  than bypassing construction through an indiscriminate derive.
- Put trait bounds on the implementations that need them unless the type's definition
  itself requires a bound. Use `Deref` for smart-pointer behavior, not fake inheritance.

## Visibility and evolution

- Start private; widen to `pub(crate)` or `pub(super)` before committing to `pub`.
  Keep representation details out of the public contract.
- Consider `#[non_exhaustive]` for new extensible public enums/structs and sealing for
  traits downstream must not implement. Retrofitting either can break existing users.
- Treat exposed dependency types as part of the API. If callers need to name one,
  provide an intentional public path and account for its version compatibility. Make
  the canonical path clear while preserving alternate paths needed by existing users.
- Treat public `const fn` capability as a compatibility commitment. Hide unstable
  implementations behind wrappers or `impl Trait` while retaining promised traits.
- Preserve public signatures, derives, error variants and observable output during a
  behavior-preserving refactor. Report required before/after changes and their migration
  impact instead of quietly applying style rules to an established contract.

For dispatch tradeoffs read [Generics and dispatch](generics-and-dispatch.md); for
compile-time transitions read [Type state](type-state.md). Error contracts are covered
in [Error handling](error-handling.md).
