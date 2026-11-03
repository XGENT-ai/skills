# Live browser assets

`detect-antipatterns-browser.js` is generated and tracked; `cargo xtask bundle`
rewrites it. Do not hand-edit it.

It is the in-page detector bundle: the rule core (`crates/core`) compiled to
WebAssembly by `crates/wasm`, concatenated with the page JS in
`browser-bundle/` and the module embedded as base64.
`crates/live/src/browser_assets.rs` embeds it with `include_str!` and the
live server hands it to the browser as `/detect.js`, so the binary has to
carry it. `bundle-inputs.json` records the input and output hashes.

```bash
cargo xtask bundle          # rewrites it
cargo xtask bundle --check  # fails when it is stale
```

The other browser scripts the live server serves (`live-browser*.js`,
`modern-screenshot.umd.js`) are not copied here: they are embedded straight
from `skills/phoenix-ui/src/scripts/`, the one copy the build also ships to every provider.
