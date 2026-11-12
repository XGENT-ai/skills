# Phoenix UI 中文排版检测与口径 P2 实施

- 任务 ID：ddfc2db0-1081-4c9e-8cee-8aecf3179c7e
- 计划：[PHOENIX-UI-EVIDENCE-CRAFT-P2](../../docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md)

## E001 — 写文件工具把 `\uXXXX` 转义落成了字面字符
- 状态：pending
- 线索：用 Write 写含 JS 正则 `[  …]` 的 Rust 原始字符串，落盘后变成真实的不可见字符，U+2028 让页内脚本报 `Invalid regular expression: missing /`；改用脚本显式写反斜杠序列后正常。
- 定位：`tools/phoenix-ui/crates/browser/src/typography.rs` 的 `SPACE_ALL`；Claude Code 会话 7a00223f（2026-11-11）

## E002 — 渲染 skill 还会改受版本控制的 `dist/phoenix-ui/bundle/`
- 状态：pending
- 线索：计划只写了 `skills/phoenix-ui/` 与 `GENERATED.json`，按文件清单暂存后才发现 `bun scripts/render.mjs` 同时重写了 75 个 bundle blob 和 `manifest.json`，补进了 M3 提交。
- 定位：`docs/phoenix-ui.md`「维护者」节；提交 `76814c1`

## E003 — 单独跑 `cargo xtask bundle` 缺构建前置环境
- 状态：pending
- 线索：直接运行报 `Missing PHOENIX_UI_CARGO_WRAPPER`；用 `scripts/phoenix-build-tools.mjs` 的 `preflight()`+`buildEnvironment()`，并导出 `local/phoenix-ui/ci-prepare/<平台>.env.json` 里的工具路径后可运行。重复步骤，可考虑给一个只做 bundle 的入口。
- 定位：`scripts/build-phoenix-ui.mjs`:80；`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M1.md`

## E004 — 新增夹具让浏览器测试耗时翻倍
- 状态：pending
- 线索：`xgent_rules` 的 URL 表每页每视口单独起一次扫描（约 6 秒），一类一页时 446 秒，合并同类夹具后 199 秒。
- 定位：`tools/phoenix-ui/crates/cli/tests/xgent_rules.rs` 的 `URL_EXPECTATIONS`

## E005 — CDP 读字体不强制排版，只有真实会话的状态取证才暴露
- 状态：pending
- 线索：`CSS.getPlatformFontsForNode` 对刚插入、未排版的节点返回空。夹具表与单测全过，直到 V-7 的会话在点击后带过渡的状态里取证，才出现整页“字体未核实”的虚假备注（6 次重拍 3 次）；插入后读尺寸强制排版并在读空时重读后 12 次为 0。
- 定位：`tools/phoenix-ui/crates/browser/src/typography.rs`（`LAYOUT_JS`）；提交 `532f8d8`；`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md`「缺陷与修复」
