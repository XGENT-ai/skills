# Phoenix third-party notices

Phoenix incorporates the Apache-2.0 Impeccable runtime by Paul Bakaus, imported
from https://github.com/pbakaus/impeccable at
`508d7e8955de3b3caf2d8676e85206723d41a887`. The original license and NOTICE
are preserved in the accompanying `THIRD-PARTY-LICENSES/` texts. The root
XGENT npm package's MIT license does not replace these licenses. Phoenix
modifications must carry source and provenance notices.

`THIRD-PARTY.json` identifies each locked Rust/Node dependency, its license,
version, original source location, and preserved license/notice text by SHA-256.
It also identifies embedded JavaScript, UI code and fonts. License expressions
with `OR` offer the stated alternatives; expressions with `AND` retain all
required terms. Source files retain their original copyright notices.

The MPL-2.0 covered components are cssparser, cssparser-macros, dtoa-short and
selectors. Their unmodified corresponding source is freely available from the
exact versioned `sourceUrl` archives in `THIRD-PARTY.json`; each archive's
SHA-256 is its `checksum`. Their source form remains under MPL-2.0. The full
MPL-2.0 text is included in the manifest's `licenses` entries. Recipients may
obtain, modify and redistribute that covered source under those terms;
Phoenix's surrounding license does not restrict those rights. If any covered
source is changed, the distribution must provide the changed corresponding
source and retain its notices before it can pass the release gate.

The two webpki-roots versions incorporate root-certificate data under
CDLA-Permissive-2.0. The complete agreement accompanies them in the manifest's
`licenses` entries, as required when sharing that data.

The bundled `modern-screenshot.umd.js` (editable source:
`skills/phoenix-ui/src/scripts/modern-screenshot.umd.js`) is the unchanged
`package/dist/index.js` from modern-screenshot 4.7.0, under MIT, Copyright
2021-present wxm. Its original MIT text, npm archive integrity, archive hash
and file hash are recorded separately; this library is outside bun.lock.

Albert Sans, Alumni Sans and JetBrains Mono remain under SIL OFL-1.1. Their
original project copyright notices and full OFL texts accompany the unchanged
font files. The fonts are bundled with Phoenix; their original license and
reserved-name conditions continue to apply.

The CSS/JS kit in `ui/component-review/vendor/` is redistributed in the fixed
Apache-2.0 Impeccable repository. Its sync script identifies the original
private impeccable-site repository; that separate private tree was not
available for comparison. The fixed public copies and their original comments
are the source evidence used here.

The upstream NOTICE identifies the iOS/Android reference files as derived from
ehmo's https://github.com/ehmo/platform-design-skills under MIT; its author and
attribution notice are preserved verbatim. The exact pre-distillation source
revision is not supplied by that NOTICE.

is-reference 3.0.3 and locate-character 3.0.0 declare MIT in their fixed upstream
README/package.json but supply no separate LICENSE/copyright text in those
source trees or npm packages. The manifest preserves the declarations and
links to the exact upstream revisions rather than inventing a copyright
statement. Other original notices, including Playwright's bundled third-party
notices, are retained as supplied by their packages.

`crates/cli/src/capture/` and the `phoenix-ui-ux-walker` subagent adapt parts
of oil-ui 0.33.2 by oil-oil (screenshot steps, locating and failure hints,
settling, mask/mark/sheet/stitch/frames methods, and the UX walkthrough
handoff and report format), rewritten for Phoenix. `phoenix-ui media`
(`crates/cli/src/media/`) adapts its media source mappings, license grading,
relevance filtering, stable references, and visual analysis algorithms.
`phoenix-ui prep-image` (`crates/cli/src/prep_image.rs`) adapts its processing
order and image snippet format. Both are rewritten in Rust. oil-ui is used under the
MIT license, reproduced in full:

```text
MIT License

Copyright (c) 2026 oil-oil

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

