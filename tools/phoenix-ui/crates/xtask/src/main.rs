//! Workspace tasks that need no Node.
//!
//! `cargo xtask bundle` builds the in-page detector bundle:
//!   1. locked/offline Cargo via the prepared mise wrapper, followed by the
//!      pinned wasm-bindgen and Binaryen, into `target/wasm-bundle/`;
//!   2. concatenates the page JS (`browser-bundle/*.js`, embedded in
//!      `bundle.rs`) in a fixed order with the wasm-bindgen glue and the
//!      .wasm embedded as base64;
//!   3. writes the tracked `crates/live/assets/detect-antipatterns-browser.js`
//!      (deterministic: same sources, same bytes), which live mode embeds and
//!      serves as `/detect.js`.
//!
//! Run this after touching `crates/core`, `crates/foundation`,
//! `crates/wasm`, or `browser-bundle/`, and commit the refreshed asset.
//!
//! `cargo xtask bundle --check` verifies logical input and output hashes,
//! without rebuilding WASM.

mod bundle;
mod freshness;

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    if args.first().map(String::as_str) != Some("bundle")
        || args.iter().skip(1).any(|a| a != "--check")
    {
        eprintln!("usage: cargo xtask bundle [--check]");
        std::process::exit(2);
    }
    let root = root();
    if check {
        freshness::check(&root).unwrap_or_else(|e| die(e));
        println!("Browser bundle inputs and artifact hash are current");
        return;
    }
    let (glue, wasm) =
        bundle::wasm_pack_build(&root.join("crates/wasm"), &root.join("target/wasm-bundle"))
            .unwrap_or_else(|e| die(e));
    if let Err(mismatch) = bundle::check_capture_contract() {
        die(mismatch);
    }
    let out = bundle::in_page_bundle(&glue, &wasm);
    std::fs::write(root.join(freshness::ASSET), &out).expect("write bundle");
    freshness::record(&root).unwrap_or_else(|e| die(e));
    let b64_len = wasm.len().div_ceil(3) * 4;
    println!(
        "{}: {} KB (wasm {} KB, base64 {} KB, js {} KB)",
        freshness::ASSET,
        out.len() / 1024,
        wasm.len() / 1024,
        b64_len / 1024,
        (out.len() - b64_len) / 1024
    );
}

fn die(message: String) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
