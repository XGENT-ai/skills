# Dependency and distribution checks

The checker is `node scripts/check-phoenix-deps.mjs`, run from the repository
root. It requires the already prepared tools in `build-tools.lock.json`,
including cargo-deny **0.20.2**. It does not install tools or dependencies.
The official release is
https://github.com/EmbarkStudios/cargo-deny/releases/tag/0.20.2.

Explicit network preparation:

```sh
node scripts/check-phoenix-deps.mjs --prepare
```

This fetches the official RustSec database into ignored
`local/phoenix-ui/dependency-audit/advisory-dbs/`, then selects commit
`b8a1a33e246a0a9a3b5f377248c41a503defec74` (tree
`7310ad041f7925accfc258bec7fb4f76fb7ff4a9`). It records the acquisition time
in `prepared.json`. It also retrieves the pinned modern-screenshot archive
and missing Puppeteer license, verifies the source pins, runs Bun 1.3.14's
`bun audit --json`, and sends the complete locked/bundled package-name/version
request to npm's official bulk advisory endpoint. The request contains
dependency names and versions, without project files or credentials.

The checker reuses an existing cargo-deny executable or the explicitly copied
`local/phoenix-ui/dependency-audit/bin/cargo-deny`; an override may be supplied
with `PHOENIX_CARGO_DENY`. Every executable must report exactly 0.20.2.
Missing Cargo archives must be prepared separately with the pinned
`cargo fetch --locked`. Neither operation is performed by ordinary checks.

Generate/review the license inventory and actual audit record after a lock or
resource change:

```sh
node scripts/check-phoenix-deps.mjs --write
node scripts/check-phoenix-deps.mjs
```

Both commands are offline. Cargo metadata uses `--locked --offline
--all-features`; cargo-deny uses `--all-features --frozen check advisories
licenses`, including private workspace and development dependencies. Coverage
is compared to every entry in Cargo.lock. Original cached `.crate` archives
must match Cargo.lock's SHA-256, and their expanded files must match the
archive; changed or extra registry source is refused. License files and
AUTHORS/NOTICE files are preserved verbatim, deduplicated by hash. Dependencies
without a standalone license also retain precise declaration/source locations.

Resource `path` values are current repository-relative locations.
`importedPath` values retain the original `UPSTREAM.json` keys for the import's
source URLs and hashes. The material scripts use the single editable
`skills/phoenix-ui/src/scripts` root; the original `tools/phoenix-ui/skill/scripts`
root is read only during the import-to-single-source transition. Moving a file
requires an explicit inventory refresh even when its content hash is unchanged.

Ordinary checks refuse a changed/dirty RustSec tree, missing preparation,
another Bun version or bun.lock hash, omitted npm request entries, malformed
responses, registry skips, and any reported advisory. RustSec content and the
npm snapshot expire after 30 days. Fetching an old database pin again does not
refresh the content's age: maintainers must review and explicitly update its
revision/tree pin. cargo-deny has an additional 90-day offline fetch-age gate.
The policy has no advisory ignores and no crate license exceptions. MPL-2.0
and CDLA-Permissive-2.0 are allowed with the corresponding source and notice
requirements described in `THIRD-PARTY-NOTICES.md`; this is not a vulnerability
or license exception.

The first check of the imported lock was blocked by rustls 0.23.43's
[RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285) /
[GHSA-2mjx-qc3c-rqvc](https://github.com/rustls/rustls/security/advisories/GHSA-2mjx-qc3c-rqvc).
Its upstream advisory identifies 0.23.45 as the patched version. The shared
Cargo.lock was minimally updated by the main implementer; this checker did
not waive the finding or edit that lock. The current actual result and input
hashes are in `dependency-audit.json`.

Each engine/resource archive and npm package must carry these sidecars together:

- `THIRD-PARTY.json`
- `THIRD-PARTY-NOTICES.md`
- every referenced `THIRD-PARTY-LICENSES/<sha256>.txt`

Check an expanded distribution candidate before release:

```sh
node scripts/check-phoenix-deps.mjs --distribution-dir /absolute/path/to/candidate
```

This refuses missing or changed sidecars and MPL entries without exact
unmodified corresponding source evidence. It does not prove the final binary
was built from that source, the generated assets are fresh, external source
URLs remain available, or all native platforms passed; those remain the build,
freshness, source-availability and platform release checks. A normal dependency
audit explicitly reports distribution verification as `not-run`. Shipping the
sidecars through the binary/npm generators is an integration task; the checker
does not automatically copy them into candidates.

Lightweight tests:

```sh
node --test test/phoenix-ui/dependency-audit.test.js
npm test
```

These exercise parsing, integrity and refusal paths with small temporary data.
They never run native checkers, download data, or start providers. A passing
lightweight suite is separate from the actual offline cargo-deny/Bun snapshot
check and final distribution acceptance.
