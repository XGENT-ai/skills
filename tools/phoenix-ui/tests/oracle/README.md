# Oracle: end-to-end goldens for every `phoenix-ui` verb

Each case runs one verb (or a sequence of steps) of the `phoenix-ui` binary in
a temporary copy of a workspace and records stdout, stderr, exit code and the
named files, with machine-specific paths and timestamps normalized. The goldens
are the behavior contract: any byte difference fails.

Run from the repository root:

```sh
node tools/phoenix-ui/tests/oracle/run.mjs --bin <phoenix-ui> [id-prefix]
node tools/phoenix-ui/tests/oracle/record.mjs --bin <phoenix-ui> <id-prefix>
```

`--bin` defaults to `$PHOENIX_UI_BIN`. `run.mjs` replays the cases (all, or
those whose id starts with the prefix) and exits 1 on any difference or missing
golden; `npm run test:phoenix` runs it against the freshly built engine.
`record.mjs` overwrites goldens. Record only cases you added or whose behavior
changed on purpose, and review every golden diff by hand: the binary under
test is also the recorder, so a bug would be frozen too.

The corpus runs on macOS and Linux.

## Layout

- `cases/*.mjs`: the corpus (default export: an array, or an async function
  returning one). The verb is the binary's subcommand; `cli-help` and
  `cli-version` run `--help` / `--version`. `detect.mjs`: `detect`.
  `hooks.mjs`: `hook`, `hook-before-edit`, `hook-admin`. `context.mjs`:
  `context`, `doctor`, `pin`, `surface-brief`, `critique-storage`,
  `embed-prompt`, `context-signals`, `detect-csp`, `concept-seed`,
  `generate-image`, `serve-question` (offline paths only: the shipped
  catalog, image-generation argument errors, question modes that never open a
  browser).
  `comp.mjs`: comp and component-review verbs. `skills.mjs`: `install`,
  `link`, `update`, `check`, `skills`. `live-*.mjs`: the `live*` verbs.
- `workspaces/`: project fixtures copied to a temp dir per run, so cases can
  write freely. `fixtures/external-project/` is a second git project staged
  beside the workspace by the two cross-project `detect` cases.
- `golden/<id>.json`: the recorded results.
- `lib.mjs`: staging, execution, normalization and diffing.
  `live-helpers.mjs`: staged buffers and wrapped sources for the live cases.

A case is `{ id, verb, args, cwd?, stdin?, env?, files?, workspace?,
setup?, steps?, normalize?, platforms?, offline? }`; `lib.mjs`
documents each field. `steps[]` entries may carry their own `setup(ws)` and
`daemon: true` with `readyFile`: the verb is spawned detached, later steps run
against it and its output lands in the golden as `daemon`. `offline: true`
runs the verb under `sandbox-exec` without network on macOS, so static
framework fixtures cannot discover a dev server left listening on the host.

## Normalizations

Beyond the workspace (`<WS>`), `tools/phoenix-ui` (`<REPO>`), the skill source
(`<PHOENIX_SKILL>`), home (`<HOME>`) and binary (`<PHOENIX_UI>`,
`<HOOK_ADMIN_CMD>`) paths, `normalize()` masks ISO timestamps (`<ISO>`),
epoch stamps (`<EPOCH>`), critique snapshot stamps (`<STAMP>`), pids, UUIDs,
hook durations, the image-converter probe (`<IMAGE_TOOLS_PROBE>`), the
dev-server port probe (`<DEV_SERVER_PROBE>`) and the relative climb to an
outside target (`<UP_TO_ROOT>`). A case's own `normalize: [[regex, flags,
replacement]]` masks values only that flow produces (a dynamic port,
timestamps).

Not covered here: real image generation, `serve-question --start` (browser and port), and the
browser-driven live flows (see `tests/live-e2e`). `live-poll-*-connection-refused`
assumes nothing listens on 127.0.0.1:65531.
