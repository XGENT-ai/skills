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

## E006 — 评审修复先用评审原样输入复跑
- 状态：pending
- 线索：apply-code-review P2 R1 的 F-02 首版修法（按元素 getClientRects 分片盒分栏）在自造夹具（段落位于分栏容器内）上通过，按评审原样输入（段落自身是分栏容器）复跑才发现仍漏检，改为按文档顺序判栏。
- 定位：`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M1.md` 评审修复；提交 `aa1d065`；Claude Code 会话 47d42dec-5e76-41a8-a36d-d7a1071233fb

## E007 — 造修前二进制时用了 git stash
- 状态：pending
- 线索：为对比修前/修后，临时 git stash 再构建，违反 apply-code-review “不 stash” 的约定（已原样还原，无损失）；可改为动手前先留存修前二进制或用 git worktree。
- 定位：Claude Code 会话 47d42dec-5e76-41a8-a36d-d7a1071233fb：关键词 phoenix-ui-before；证据待核对

## E008 — 修法里记成“已知边界”的情形被复审打回
- 状态：pending
- 线索：P2 R1 的 F-02 修法把“每栏只有一行的分栏”写进已知边界未处理，R2 复审判为计划已承诺的多栏行为并给出真实反例；按字符位置推断换栏在同行带上无解，改读分栏容器的栏位栅格后修掉。另：两张自造对照页前提不成立（直接文本不足 10 字、栏宽放得下整段），截图和反向去掉判断后才发现对照是空的。
- 定位：`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M1.md` 评审修复；提交 `f273d66`；Claude Code 会话 b41b09f1-403f-4dc9-9a9e-f33beac4e1bc

## E009 — 宿主会话被 API 前置网络限制拒绝
- 状态：pending
- 线索：P2 R3/R5 宿主曾报 network_edge_blocked 403；用户提醒先查 HTTPS_PROXY。执行子进程未继承代理，而 .zshrc/系统均已配置；显式传入既有 HTTP(S)_PROXY 后 Claude 正常调用 Skill/工具。先查环境继承，别直接归因外部阻塞；宿主产物另验。
- 定位：`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md`「R3 修复后的适用性」；Claude Code 2026-11-11。
