# The engine: the Rust runtime behind every skill verb

Every command the skill text runs is `phoenix-ui <verb>`. The binary is built
from this Cargo workspace; there is no Node at runtime. Observable CLI
behavior is pinned by the golden corpus in `tests/oracle/`.

## Layout

```
Cargo.toml              the workspace (crates/*), release profile
ENGINE_VERSION          the engine version the launcher reports
.cargo/config.toml      the `cargo xtask` alias
browser-bundle/         the page JS the in-page detector bundle is built from
crates/
  cli          the `phoenix-ui` binary: verb router, exit codes, native capture
  common       Io handle (stdout/stderr/stdin/env/cwd), path + process helpers
  context      context, doctor, staleness, signals, concept-seed, critique, ...
  hook         the design hook (hook, hook-before-edit, hook-admin)
  live         live mode: helper server, wrap, accept, manual edits, Svelte preview
  skills       install / update / check / link
  install      the npm-side installer entry
  comp         comp-fidelity pure libs (raster, png, metrics, fonts)
  comp-verbs   build-phase, comp-diff, comp-spec, font-match
  detect       `phoenix-ui detect`: file walk, config, ignores, output, regex engine
  html         the static HTML engine: parser, cascade, static DOM, rule adapters
  browser      the URL engine: Chrome discovery, CDP, snapshot, visual pass
  foundation   JS-semantics helpers, color, findings, the rule registry, inline
               ignores, the Dom trait, SnapshotDom, plain-data types
  core         the rule logic: every `check_*` / `scan_*`, the browser rule
               adapters, the visual-contrast decisions
  rules        the `xgent/*` rule pack the CLI and hooks install
  wasm         wasm-bindgen exports over `core` for the in-page bundle
  xtask        `cargo xtask bundle`: builds the in-page bundle
```

`crates/core` re-exports the foundation modules, so consumers name one crate:
`impeccable_core::js`, `impeccable_core::color`,
`impeccable_core::browser::driver::collect_browser_findings`.

Build and test:

```bash
cargo build -p phoenix-ui                                  # target/debug/phoenix-ui
cargo test -p <crate>
node tests/oracle/run.mjs --bin target/debug/phoenix-ui    # the CLI behavior gate
```

## The browser bundle

The rules also run in the page, compiled to WebAssembly. `cargo xtask bundle`
builds `crates/wasm` with the pinned wasm-bindgen and Binaryen, concatenates
the page JS in `browser-bundle/*.js` with the glue and the base64 module, and
writes the tracked `crates/live/assets/detect-antipatterns-browser.js`, which
`crates/live/src/browser_assets.rs` embeds and the live server serves as
`/detect.js`. The page JS only implements the `Dom` probe, marshals JSON and
draws the overlay; no rule logic lives there.

`cargo xtask bundle --check` fails when the recorded inputs or the tracked
bundle are stale (`crates/live/assets/bundle-inputs.json`). Run
`cargo xtask bundle` after touching `crates/core`, `crates/foundation`,
`crates/wasm`, `crates/xtask` or `browser-bundle/`.

URL scans do not run WebAssembly next to the page: `crates/browser` injects
only `15-snapshot.js`, captures the page as JSON and runs the same rule core
natively over `SnapshotDom`, so strict-CSP pages are scanned passively.

## Rule packs

The built-in rules always run. `impeccable_core::rule_pack::RulePack` adds
rows to the registry (`rule_pack::install`) and hooks for the text engine
(`check_text`) and the browser driver (`check_element_dom`, `check_page_dom`).
The CLI and hooks install `phoenix_ui_rules::PACK`; the pack reference travels
in `ScanOptions.rule_pack`, `TextOptions.rule_pack`,
`DetectHtmlOptions.rule_pack` and `BrowserConfig.rule_pack`. Pack findings are
appended after the built-in passes and before inline ignores, so
`phoenix-ui-disable` waives them like built-in rules. The in-page WASM
detector runs the built-in rules only.
