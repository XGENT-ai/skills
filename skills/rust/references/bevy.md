# Bevy — framework-specific Rust practices

Adapted from `local/rust-best-practices/references/bevy.md`. Read only when the affected
crate uses `bevy`/`bevy_ecs`, or the requested code is explicitly a Bevy system/plugin.
Check the versions in `Cargo.toml` and `Cargo.lock`, then consult matching
[API docs](https://docs.rs/bevy/0.20.0/bevy/) and
[migration guides](https://bevy.org/learn/migration-guides/). The local digest targets
0.20; do not migrate a project or assume its API names match that version.

This is guidance, not a claim that the project's systems or every example have been
executed. Framework exceptions below apply to Bevy APIs; general error, safety and scope
rules still apply to helpers and application logic.

## Keep Bevy's conventions

- **System parameters by value:** Bevy constructs `Res`, `ResMut`, `Query`, `Single`,
  `Commands`, `Local` and message-reader/writer parameters. Do not convert them to
  `&Res` or `&Query` to satisfy ordinary borrowing advice. Helpers should accept the
  narrow domain data they need rather than an entire query.
- **Framework prelude:** `use bevy::prelude::*` is an intentional Bevy convention.
  Do not remove it merely for being a glob or create a project prelude as a consequence.
- **Plain-data components/resources:** public fields are reasonable for passive ECS
  data. Protect coupled invariants where necessary; avoid getters, builders or panicking
  constructors that add no useful contract. Apply gameplay caps at the mutation boundary.
- **Query complexity:** multiple system parameters and complex query types can be normal.
  Use a named alias or reusable `SystemParam` when it clarifies a shared concept, not
  merely to satisfy `too_many_arguments` or `type_complexity`.
- **Lint adaptation:** inspect the project's existing configuration and toolchain. A
  justified system-specific or crate-scoped exception may be appropriate for
  `needless_pass_by_value`, `type_complexity`, `too_many_arguments` or
  `elided_lifetimes_in_paths`. Do not disable these globally for unrelated crates or
  install an external lint preset. Keep the Rust 1.81 `expect` boundary in mind.

## Correctness checks

### Conflicting access

Queries that might access the same component mutably can be rejected at runtime during
system initialization. `With<Player>` and `With<Enemy>` do not prove disjointness because
an entity can carry both markers. Use domain-correct `Without` filters for genuinely
disjoint sets, or a `ParamSet` when overlapping accesses must happen sequentially.
Check resource borrow conflicts too. Do not change gameplay membership just to silence
the diagnostic; exercise the combined system/plugin in a test.

### Deferred commands and system ordering

`Commands` queue structural changes. A consumer must run after both the producer and
the relevant deferred buffer application. Explicit ordering (`chain`, `before`, `after`
or system sets) communicates dependencies; a tuple of systems alone does not promise order.
Bevy 0.20 inserts `ApplyDeferred` along appropriate dependency edges by default, but
custom schedule settings can change this. Verify the schedule actually used by the project.
See [ApplyDeferred](https://docs.rs/bevy/0.20.0/bevy/ecs/schedule/struct.ApplyDeferred.html).

Check same-frame assumptions for resources, components, messages, spawns and despawns.
For example, checking whether a wave is empty before queued spawns become visible can
start a duplicate wave. Order the entire observation/update sequence deliberately.

### Cardinality, system errors and vanished entities

Handle failed entity lookup and single-entity cardinality explicitly; a dead player or
second camera should not turn a recoverable situation into an unwrap panic. Depending
on the locked version and semantics, use a fallible query with `let Ok(...) = ... else`
or an appropriate system parameter. In Bevy 0.20, `Single` validation skips a system
when exactly-one cardinality is not met; use it only when skipping is intended.
See [Single](https://docs.rs/bevy/0.20.0/bevy/ecs/system/struct.Single.html).

Returning `Result` from a system does not alone establish graceful recovery: inspect
the configured error handler and its default behavior. Select logging, skipping or
escalation intentionally; do not globally silence all system errors to fix one case.

Queued commands can encounter entities despawned earlier in the frame. Avoid duplicate
despawn logic, or use the locked version's fallible/tolerant entity-command APIs such
as `try_insert`/`try_despawn` where their semantics fit. Verify command-application errors.

### Messages and event APIs

Each reader maintains its own cursor; an active reader does not reread already consumed
messages. A reader scheduled before its writer may observe data on a later update.
Order them when same-frame handling is required and test retention when readers are
disabled for multiple updates. Use the project's actual message/event API names and
observer signatures; they vary between Bevy versions.

## Testing systems

- Use a headless `App` with only the required resources and plugins; do not pull in
  rendering/window plugins for a pure gameplay test.
- Control time explicitly using the chosen time resource or update strategy. Avoid
  wall-clock sleeps, and check that a time plugin is not overwriting manually advanced time.
- Test zero, one and multiple matching entities; overlapping markers; disappearing
  entities; duplicate commands; message ordering; and bounds such as health caps when relevant.
- Test deferred changes at the point the consumer observes them, not just command enqueueing.
- Add a smoke test for the whole affected plugin and run at least two updates. Isolated
  system tests can miss query conflicts, missing resources and cross-system ordering.
- Record the Bevy version and which headless/native scenarios actually ran. Compile
  success alone does not establish that systems initialize and execute correctly.
