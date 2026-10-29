# Actions, Algorithms and Tree Traversal

Read when actions need queuing/replay, callers select an algorithm, or heterogeneous
trees need traversal/transformation. Start with direct code; use a pattern only for an
actual requirement, not to introduce a framework around a single operation.

## Queue, replay or undo actions: Command

Use an enum or closure for simple actions. A command trait can serve multiple operations
that share an execution contract. Choose storage and dispatch according to ownership
and runtime selection; a function pointer is still an indirect call.

Define ordering, failure and rollback behavior. Store the data needed for undo rather
than assuming a command makes multiple external effects transactional. Test partial
failure, replay and rollback when those capabilities are promised.

## Select an algorithm: Strategy

A closure often suffices for one configurable operation; use a trait when the contract
has richer behavior. Choose closure bounds and static/dynamic dispatch through
[generics and dispatch](generics-and-dispatch.md). Do not add a strategy layer when a
plain branch already expresses the choice. Test each algorithm against the shared contract.

## Inspect heterogeneous nodes: Visitor

Separate `visit_*` hooks from shared traversal for structures such as an AST. Specify
traversal order, whether hooks recurse by default, early exits and error propagation.
Verify coverage of relevant node kinds. Use ordinary iteration for homogeneous data.

## Transform a tree: Fold

Default recursive `fold_*` methods can rebuild a tree while overrides transform selected
nodes. Define ownership, failure propagation and preservation of untouched nodes; test
those properties. This structural transformation differs from `Iterator::fold`.

## Evaluate a language: Interpreter

Introduce an interpreter only for a real recurring language with a grammar and an error
contract. Ordinary configuration does not need a DSL. For compile-time Rust-like syntax,
consider [macros](macros.md). Conversion/variant concepts such as Iso or Prism may help
reasoning, but `match`, field access and normal conversions usually suffice without an
optics abstraction.

## Related construction and resource scenarios

Use [API design](api-design.md) for newtypes and builders,
[type state](type-state.md) for compile-time protocols, and
[error handling](error-handling.md#cleanup-and-panic-boundaries) for RAII and fallible
teardown. These topics own their rules; do not duplicate them in a pattern layer.
