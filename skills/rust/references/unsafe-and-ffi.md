# Unsafe and FFI

Read when a change touches unsafe blocks/impls, raw pointers or a foreign interface.

## Safety boundary

- Prefer safe std or maintained crate abstractions. Unsafe needs a concrete reason:
  an abstraction safe Rust cannot express, a measured performance requirement or FFI.
- Keep the unsafe implementation in the smallest module that can maintain its
  invariants. Safe callers must not be able to cause undefined behavior through safe
  operations, including malformed input, cancellation or unwinding.
- Each unsafe block or impl needs a `// SAFETY:` comment that explains why its actual
  obligations hold: validity, alignment, initialization, bounds, aliasing and lifetime
  as applicable. Merely saying a pointer is non-null is insufficient.
- Every `unsafe fn` documents caller obligations in `# Safety`. Enable
  `unsafe_op_in_unsafe_fn` so unsafe operations remain explicit inside unsafe functions.
- Do not use `transmute` to evade lifetimes or cast enums, or `unsafe impl Send/Sync`
  merely to silence the compiler. A manual impl requires a soundness argument.
- Mark a function unsafe for caller obligations whose violation can cause undefined
  behavior, not merely because the operation is destructive.
- Validate safety-critical conditions in release builds too; `debug_assert!` cannot
  replace checks required to keep a safe interface sound.

## Foreign interfaces

- Match the foreign ABI and layout deliberately: `#[repr(C)]` for shared structs,
  integer types matching the actual C declaration, and `CString`/`CStr` for C strings.
  `repr(C)` alone does not make references, ownership or nested Rust types FFI-safe.
- Document allocation/free pairs and ownership transfer. Wrap owned foreign resources
  in RAII types that call the matching foreign destructor exactly once. Offer explicit
  fallible cleanup if errors must reach the caller; keep `Drop` from panicking.
- Native-handle escape hatches are optional. Distinguish a borrowed handle from ownership
  transfer, and document validity, lifetime and unsafe reconstruction obligations. Never
  return pointers to stack storage or free a resource still borrowed by a caller.
- Validate null pointers, lengths and return codes, then establish the remaining
  lifetime/aliasing obligations before creating references or slices.
- Keep business logic in safe Rust and concentrate translation in the binding layer.
  Use the project's bindings or generator; adding a separate `-sys` crate is a design
  choice, not a prerequisite for a small fix. Prefer maintained/generated bindings to
  hand-copying a large native API. Pin headers and generation tools, record upstream
  source/version/hash, and check generated drift. Make optional build tools optional.
- Choose static/dynamic linkage for deployment requirements. Across dynamic libraries,
  share only data with an explicit ABI/lifetime contract; do not assume Rust layout,
  statics or `TypeId` identity is shared. Export attributes must fit the edition/toolchain.
- Specify the unwind policy. Do not let Rust panics escape a non-unwinding ABI.
  `catch_unwind` can translate unwinding Rust panics to an FFI error at an appropriate
  Rust boundary; it cannot catch `panic = "abort"` and is not a general mechanism for
  catching foreign exceptions. Do not introduce panic-based control flow.
  See the [Rustonomicon's FFI and unwinding rules](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding).

## Verification

Exercise safe wrappers with adversarial inputs and ownership/drop sequences. Run
applicable tests under Miri when available; unsupported foreign calls need native
integration tests on supported platforms. Miri passing is evidence, not a proof of
soundness. Report tool/platform gaps explicitly.
