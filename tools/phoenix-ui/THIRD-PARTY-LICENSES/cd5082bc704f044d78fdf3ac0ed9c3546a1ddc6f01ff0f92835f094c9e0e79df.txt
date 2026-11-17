# Third-Party Notices

This project includes content derived from third-party work, used under the terms of its original license.

## Phoenix UI fork

XGENT maintains Phoenix UI as a modified Apache-2.0 fork of Impeccable skill 4.5.0 (commit `508d7e8955de3b3caf2d8676e85206723d41a887`) and engine 0.1.11 (commit `86be0f02dcdaaeb3f9b6780711dbdc03a14ce222`). The original license and notices remain in this distribution.

Phoenix UI has since diverged from upstream and is maintained independently; local changes include public identity and state paths, offline distribution, provider hooks, local catalog candidates, screenshot resources, and removed or simplified commands and internals. `UPSTREAM.json` preserves the original file hashes at the fork point. Skill sources now live under `skills/phoenix-ui/src/`.

## Platform Design Skills

The `skill/reference/ios.md` and `skill/reference/android.md` platform reference files are distilled from ehmo's `platform-design-skills` (Apple Human Interface Guidelines and Material Design 3 rules), rewritten in Impeccable's voice.

**Original work:** https://github.com/ehmo/platform-design-skills
**Original license:** MIT
**Author:** ehmo

## oil-ui

The `phoenix-ui capture` command (`crates/cli/src/capture/`) adapts oil-ui's screenshot workflow: the step model, text locating and failure hints, animation settling, and the mask, mark, sheet, stitch, and sequence-frame methods, rewritten in Rust over Phoenix's CDP layer with JSON state files. The `phoenix-ui-ux-walker` subagent (`skills/phoenix-ui/src/subagents/phoenix-ui-ux-walker.md`) adapts oil-ui's lightweight UX walkthrough handoff and report format.

The `phoenix-ui media` command (`crates/cli/src/media/`) adapts its media source mappings, license grading, relevance filtering, stable references, and visual analysis algorithms. The `phoenix-ui prep-image` command (`crates/cli/src/prep_image.rs`) adapts its processing order and image snippet format. Both are rewritten in Rust.

The `phoenix-ui compare` command (`crates/context/src/compare/`) adapts its comparison page's proposal fields, baseline marking, and ways of viewing (side by side or single, viewport switch, actual size), rewritten as a Rust loopback server with its own page. The phoenix-ui skill's `animate`, `craft-floor`, `routing`, and `vague-feedback` references (`skills/phoenix-ui/src/reference/`) adapt its methods for motion choreography and feel, demonstration data and icons, next steps after a delivery, and translating vague words into variables; all are rewritten.

**Original work:** oil-ui 0.33.2 (local copy; upstream repository address not verified from that copy)
**Original license:** MIT
**Author:** oil-oil

