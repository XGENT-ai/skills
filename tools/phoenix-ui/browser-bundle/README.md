# browser-bundle: the page-side JavaScript of the detector

Plain JavaScript that runs inside a page: the DOM probe the wasm rule core
calls back into, the page snapshot producer, the visual-contrast sampling IO,
the overlay UI and the scan API. Measurement and presentation only; every
rule decision is a call into the wasm rule core built from `crates/core`
(`docs/ENGINE.md`).

Two consumers:

- `crates/browser` embeds `15-snapshot.js` (the snapshot producer the URL
  engine injects; no WebAssembly runs in the page).
- `cargo xtask bundle` (`crates/xtask/src/bundle.rs`) embeds every file here
  with `include_str!` and concatenates them, in filename order, with the wasm
  core into the in-page bundle, written to the tracked
  `crates/live/assets/detect-antipatterns-browser.js` the engine embeds.

Because the files are embedded, a new one here has to be added to
`PAGE_JS` in `crates/xtask/src/bundle.rs` (and to the order it is concatenated
in); a test fails when the two lists disagree.

`15-snapshot.js` lists the computed-style properties the rules read; the
bundle build checks that list against the core's and fails when they drift.
