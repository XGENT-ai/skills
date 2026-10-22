# PHOENIX-HALLMARK 开发计划评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-HALLMARK · XGENT Phoenix UI · 借鉴 hallmark 的检测规则与设计守则](phoenix-ui-hallmark.md) | `sha256:0617bf9a54c52b7bbfc0349d6b9b93b6d64dd2489aa80083dc954ddc0ba93fcb`（448 行，未跟踪，仅当前工作树可取回） | 全文：R-1–R-6、NFR-1/2、C-1–C-8、A-1/A-2、§0.6 I-1–I-19、§5–§8、V-1–V-10、M1–M5 |

- 评审日期：2026-10-22。代码基线：main / `8a45e7075a8a024e2a8775f3e3edb06ecce2e5ee`，工作树 dirty：只有本计划未跟踪，以及 `.xgent-ai/sdlc` 台账。事实以当前工作树为准。
- 初次完整评审，没有前轮报告。评审只读，未改原计划和代码。

## 1. 确认问题

### P1

1. [P1] F-01 · 按 hash 或行号绑定源码的证据账本没有纳入范围，M3–M5 的退出门按原文必然失败
   - 定位：§1.2 文件清单（[计划:183-200](phoenix-ui-hallmark.md)，「无需修改」在 :200）；§5.2 第 3–5 步；M3（:421）、M4（:422）、M5（:423）；V-5（:399）、V-6（:400）、V-8（:402）；关联 C-2、C-3、NFR-2，以及「实施者定位·开发边界」（:16）。
   - 问题：本期要改的 `crates/cli/src/main.rs`、`crates/detect/src/cli.rs`、`crates/hook/src/{hook,hook_lib,before_edit}.rs` 和 `skills/phoenix-ui/src/SKILL.src.md`，在两份受跟踪的账本里登记了 hash 或行号，计划没有写同步它们：
     - oracle 适配器的 `currentEvidenceFiles` 记录了上述 5 个 Rust 文件的 sha256。`verifyCorpus()` 在回放任何 case 之前比对 hash，不一致就抛 `current expectation source changed`。结果是 V-5 整体报错，`npm test` 里的 oracle 测试、`test:phoenix` 里的严格 oracle 也会失败。另外，expectations 的 `source` 字段引用了这些文件的行号。
     - `tools/phoenix-ui/capabilities.json` 涉及三处：
       - `current.sourceFiles[*].currentSha256` 覆盖 main.rs、hook 三个文件和 SKILL.src.md，不一致报 `stale current source`。
       - `cli.root[].currentSource` 写的是 main.rs 的行号。在 `run()` 首行插入 install，后面的分支行号会整体下移，导致 `arm.includes('=>')` 断言失败。
       - `current.generatedBundle`、`engineManifestEvidence` 绑定了 GENERATED.json、VERSION.json 和 bundle 的 hash，M5 开发构建后都会变。
     - 按「开发边界」，实施者不能自行把这些文件纳入范围。重绑 `currentEvidenceFiles` 是否违反 C-2「不放宽比较」，也会留给实施者裁定。
   - 证据：
     - [phoenix.mjs:61](../../tools/phoenix-ui/tests/oracle/adapters/phoenix.mjs)；[check-phoenix-oracle.mjs:14](../../scripts/check-phoenix-oracle.mjs)；[oracle.test.js:13](../../test/phoenix-ui/oracle.test.js)。
     - [capabilities.test.js:64、97-100、298、310](../../test/phoenix-ui/capabilities.test.js)。
     - 已核对 `phoenix.json` 的 `currentEvidenceFiles`（40 项，含上述 5 个文件）与 `capabilities.json` 的 `current.sourceFiles`（含 main.rs、hook.rs、hook_lib.rs、before_edit.rs、skill/SKILL.src.md）都和当前文件 hash 一致，说明这些锁现在确实生效。
     - 先例：提交 `0a5d229` 新增 crate 时同步了 capabilities.json。
     - 惯例上同步、但没有测试强制的文件：`tools/phoenix-ui/compatibility.md`（引用上述文件 45 处）、`patches/ownership.json`（4 处）。
   - 修订：
     - §1.2 补上 `phoenix.json` 的 `currentEvidenceFiles` 和受影响的 `source` 行号、`capabilities.json` 的 `sourceFiles`/`currentSource`/`environment.sources`/`generatedBundle`，并写明惯例同步项。
     - 在 C-2 下写明：重绑证据 hash 和行号不算放宽比较，case 期望仍逐条登记。
     - M1 或 M3 在跑 V-5 前完成重绑；M5 在开发构建后、`npm test` 前同步 `generatedBundle`。这些文件由主 agent 一人修改。
   - 复核：改稿后，§1.2 与 M1/M3/M5 中能找到这些文件和时点。实施后，V-5 不再出现 `current expectation source changed`，`node --test test/phoenix-ui/capabilities.test.js test/phoenix-ui/oracle.test.js` 通过。
   - 置信度：高。断言源码和当前 hash 都已比对，四位 reviewer 中有两位独立发现。阻塞：M3、M4 退出与 M5。

### P2

2. [P2] F-02 · V-6 在 M4 跑整仓 `npm test`：结构上过不了，也违反计划自己的验证节奏
   - 定位：V-6（:400）、M4（:422「不改生成物」）、C-3（:80）；对照「验证节奏」（:27）。
   - 问题：
     - `GENERATED.json.inputs` 收录了 `src/SKILL.src.md`、`reference/craft-floor.md`、`reference/operate.md` 的 hash，capabilities 测试逐个校验。M4 按约定不同步生成物，所以 `npm test` 在 M4 必然失败。
     - M4 与 M1–M3 并行时，Rust 改动也会让同一入口失败（F-01）。
     - 整仓入口属于收口检查，计划 :27 规定中间里程碑只跑局部检查。
   - 证据：[GENERATED.json](../../skills/phoenix-ui/GENERATED.json) 的 `inputs`（76 项，含上述 3 个文件）；[capabilities.test.js:310](../../test/phoenix-ui/capabilities.test.js)；[run-tests.mjs](../../scripts/run-tests.mjs)。
   - 修订：V-6 改为局部检查：`render --without-engine` 预览检索、`python3 -m unittest discover -s test/skills -p 'test_*.py'`、`git diff --stat`。整仓 `npm test` 只留在 V-8。
   - 复核：改稿后 V-6 不再有整仓入口，与 M4「不改生成物」不冲突。
   - 置信度：高。阻塞：M4 退出。

3. [P2] F-03 · 三方许可清单和 dist 派生物没有纳入：V-8 漏了 `check-phoenix-deps --write`，§1.2 漏了对应文件
   - 定位：§1.2（:186 只列 Cargo.toml/Cargo.lock，:196 只列 `dist/phoenix-ui/VERSION.json`）；V-8（:402）。
   - 问题：
     - `tools/phoenix-ui/THIRD-PARTY.json` 为每个本地 crate 登记 `localManifest`、`manifestSha256`，并登记 `cargoLockSha256`。新增 `crates/rules`、改 cli 和 hook 的 Cargo.toml 以及 Cargo.lock 后，这份清单就过期了；不带 `--write` 的审计会报 `Third-party inventory is stale`。
     - 开发构建会把过期清单原样复制到受跟踪的 `dist/phoenix-ui/THIRD-PARTY*`，分发许可里缺少新 crate。
     - 维护流程（docs:159）和 CI（workflow:104）都在 prepare 之后执行 `--write`，V-8 只引用了 prepare 就直接构建。
     - render 还会整目录改写受跟踪的 `dist/phoenix-ui/bundle/**`，§1.2 没有列出。
   - 证据：[check-phoenix-deps.mjs:424、481、517](../../scripts/check-phoenix-deps.mjs)；[build-phoenix-ui.mjs:104-110](../../scripts/build-phoenix-ui.mjs)；[docs/phoenix-ui.md:159](../phoenix-ui.md)；`.github/workflows/phoenix-ui.yml:104`；先例记录 [phoenix-ui.records/M3.md:267](phoenix-ui.records/M3.md)。
   - 修订：
     - V-8 在 prepare 后加 `mise exec --no-deps -- node ../../scripts/check-phoenix-deps.mjs --write`，构建后再跑一次不带 `--write` 的审计。
     - §1.2 补上 `THIRD-PARTY.json`、`THIRD-PARTY-LICENSES/`、`dependency-audit.json`、`dist/phoenix-ui/{THIRD-PARTY*,bundle/**}`，注明「只经脚本同步」。
   - 复核：实施后，审计不带 `--write` 也能通过，THIRD-PARTY.json 中有 `phoenix-ui-rules`。
   - 置信度：高。阻塞：M5 收口与提交完整性。

4. [P2] F-04 · M5 顺序倒置：开发构建排在 V-7 之前，误报修复会让构建和 V-4 证据失效，重建方式也没写
   - 定位：M5（:423「prepare 与开发构建同步生成物 → V-4 → V-7 → V-8 其余项 → V-9 → V-10……改动后只重跑受影响的 V-2、V-3、V-5」）；V-4（:398）。
   - 问题：
     - V-4 和 V-7 只需要 debug 二进制，只有 V-9 需要同步后的生成物。
     - V-7 改阈值、降级或删规则后，`crates/**` 是构建输入，VERSION/ENGINE/GENERATED 会随之过期；受影响 fixture 的 V-4 证据作废；删规则还会影响 V-1 和 docs 规则表。计划只要求重跑 V-2、V-3、V-5。
     - 再跑 smoke 时，已有 `runtime/` 会触发断言失败，必须先清理。
     - V-4 直接用 `target/debug/phoenix-ui`，但 M5 没有构建这一步。这个路径由 mbx 托管，可能被后续命令回收。
   - 证据：[smoke-phoenix-ui.mjs:37](../../scripts/smoke-phoenix-ui.mjs)；[build-phoenix-ui.mjs](../../scripts/build-phoenix-ui.mjs) 用 `walk('crates')` 计算输入摘要；[test-phoenix-ui.mjs:31-37](../../scripts/test-phoenix-ui.mjs) 的注释说明后续 mbx 命令可能回收托管产物，所以先复制一份。
   - 修订：M5 改为三段：
     1. 构建 debug 二进制，复制固定副本 → V-4 → V-7；修复后重跑受影响的 V-1/V-2/V-3/V-5/V-4。
     2. 规则定稿 → prepare → `deps --write` → 开发构建与 smoke → 同步账本（F-01）→ `npm test` 与 `test:phoenix`。
     3. V-9 → V-10。

     另外写明：开发构建后若再改规则，先清理 `local/phoenix-ui/development-release` 再重建。
   - 复核：改稿后检查 M5 的顺序和失效清单。
   - 置信度：高。阻塞：M5。

5. [P2] F-05 · §9.2 起始基线的前置条件不全：`test:phoenix` 需要先 prepare（联网），基线 oracle 报告还会被 M3 覆盖
   - 定位：§9.2（:408）、恢复快照「下一步」、V-5 输出路径（:399）、V-8（:402，计划只在这里提到 prepare）。
   - 问题：
     - `test:phoenix` 一开始就跑 `preflight()`，要求固定版本的 mise 工具到位；后续步骤还需要 prepare 准备的 WASM 工具、旧 npm 包和回执中的环境变量。计划把 prepare 写在 M5，M1 前的基线因此可能直接失败，被误记成「既有失败」。
     - prepare 需要联网（不需要 OTP 或凭据），计划没有写这项资源要求。
     - 基线和 M3 都写同一个 `--output local/phoenix-ui/oracle-xgent.json`，M3 会覆盖基线的原始报告。
     - 另外，`test:phoenix` 最后一步本身就会跑严格 oracle，基线再单独跑一次 V-5 属于重复。
   - 证据：[phoenix-build-tools.mjs:61-75](../../scripts/phoenix-build-tools.mjs)；[test-phoenix-ui.mjs](../../scripts/test-phoenix-ui.mjs)（最后一步跑 oracle，输出到 `local/phoenix-ui/ci-prepare/<target>-oracle.json`）；[docs/phoenix-ui.md:154-166](../phoenix-ui.md)；`scripts/prepare-phoenix-ui.mjs:315`。
   - 修订：§9.2 和「下一步」写明先执行 prepare 并导出回执（需要联网，环境不可用时记为环境缺失，不算既有失败）；基线直接复用 `test:phoenix` 的 oracle 输出，或改用独立路径。
   - 复核：改稿后 §9.2 写明了 prepare 前置和输出位置。
   - 置信度：中高。阻塞：M1 基线。

6. [P2] F-06 · V-9 按原文无法执行，也无法客观判定
   - 定位：V-9（:403）、§6.6 场景 12、13 与 design-reference 新增 4 行（:358-365）。
   - 问题：
     - **engine 不在缓存里。** 开发版 ENGINE.json 没有下载 URL，launcher 只认 `PHOENIX_UI_HOME/bin/<ver>/<target>/<sha>` 下的引擎。V-8 的 smoke 用临时缓存，跑完就删。没有显式安装这一步，fixture 会话里的 `context` 和 `detect` 会失败，失败提示还会指向 registry 版本（需要联网）。场景 13 测到的会是环境，而不是守则。
     - **权限模式没写。** `claude -p` 非交互运行，计划没给权限模式和允许的工具。场景 12、13 的正向行为可能因权限被拒而看不到；场景 10「没执行脚本」也可能只是权限挡住了。
     - **素材没定义。** 场景 13 需要可访问的 URL；design-reference 4 行需要的页面、重定向和空壳素材都没有定义。
   - 证据：[phoenix-bootstrap.cjs:42、57-63、101-102、254、307](../../skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs)（:254「Development engine has no download URL; use --release-dir」）；[smoke-phoenix-ui.mjs:79](../../scripts/smoke-phoenix-ui.mjs)（`PHOENIX_UI_HOME` 指向临时缓存）。
   - 修订：V-9 补上以下内容：
     - 用同一个隔离的 `PHOENIX_UI_HOME` 执行 `phoenix-ui engine install --project <fixture> --release-dir local/phoenix-ui/development-release`，会话也使用这个目录。
     - 固定 `claude -p` 的权限参数或 fixture 内的设置文件。
     - 每个场景的素材清单，以及本地 URL 提供方式。
     - 判据：被权限拒绝的调用怎么记；detect 不可用时，「未核验宽度」不算失败。
   - 复核：实施后记录中 `engine install` 退出 0，场景 13 有 `detect --viewport` 调用。
   - 置信度：engine 部分高；权限部分中（Claude CLI 的默认行为没有实测）。阻塞：M5 的 V-9。

7. [P2] F-07 · `focus-outline-removed` 类名分支：命中允许变体前缀，豁免却不允许，warning 规则会误报
   - 定位：§5.3（:270）、§12（:447）；关联 D4（:96）、NFR-1。
   - 问题：命中正则 `^(?:[\w-]+:)*focus(?:-visible)?:outline-none$` 接受任意变体前缀，豁免却只认字面上以 `focus-visible:` 或 `focus:` 开头的 token。于是 `focus:outline-none dark:focus:border-white`、`md:focus-visible:outline-2`、`[&:focus-visible]:outline-2`、`data-[focus]:outline-2` 这类已经写了替代焦点样式的串仍会命中。这是 warning 级文本规则：在 Cursor 会拒绝编辑，在 Claude 会进入 Stop 提醒。
   - 证据：计划 :270 两段条件写法不对称；[before_edit.rs:836-896](../../tools/phoenix-ui/crates/hook/src/before_edit.rs)；[hook_lib.rs:985-988](../../tools/phoenix-ui/crates/hook/src/hook_lib.rs)。
   - 修订：命中和豁免使用同一套变体链解析。任一 token 的变体链含 `focus`、`focus-visible`、`data-[focus]` 或 `[&:focus…]`，且它的工具类不是 `outline-none`，即豁免。V-2 加入上述 4 个反例。
   - 复核：改稿时确认两段规则使用同一套前缀定义；实施后 V-2 中这些反例 0 命中。
   - 置信度：高。阻塞：M1。

8. [P2] F-08 · `wrapped-control-label` 会把有意排成多行的控件内容当作「标签换行」
   - 定位：§5.3（:274-277）；关联 D4（warning）、NFR-1。
   - 问题：`text_line_rects` 汇总整棵子树的文本行，只合并同一行里连续的片段。下面这些控件会被算成 ≥ 2 行并报 warning：标题加副标题的按钮（如 `<button><span>Pro</span><span>$9/mo</span></button>` 纵向排列）、带说明的 `menuitem`、双行 nav 链接。计划只排除了 `br`。另外，计划在已合并的行上再按「中心差 ≤ 2px」二次聚类，比核心实现的合并规则更严，字号不同的徽标可能被多算一行。
   - 证据：[dom.rs:171-191](../../tools/phoenix-ui/crates/foundation/src/browser/dom.rs)（「descendants included」，返回的已经是合并后的行）及 :214 起的 `merge_text_rects_into_lines`；快照记录 `display`（15-snapshot.js 白名单）。
   - 修订：改为检查「只自己带文本的叶子后代」：对每个有直接文本、且没有带文本子元素的后代 d，看 `text_line_rects(d)` 是否 ≥ 2 行；或者排除含 block/flex/grid 文本后代的控件。直接使用已合并的行，去掉 2px 二次聚类。V-3 加三种反例：标题加副标题按钮、带说明的菜单项、标签加徽标。
   - 复核：实施后反例 0 命中，真正换行的正例仍命中。
   - 置信度：中高。机制已从源码确认，真实页面的出现频率待 V-7 统计。阻塞：M2。

9. [P2] F-09 · `redrawn-window-chrome` 只用 `parse_rgb`，计算色值不是 rgb 时会静默漏报
   - 定位：§5.3 通用约定（:256「颜色解析用 `impeccable_core::color::parse_rgb`（计算样式为 rgb/rgba）」）、:294。
   - 问题：内置规则读计算后的 `backgroundColor` 时都用 `parse_rgb_or_any`，说明代码库已经考虑到 `oklch()`、`color()` 等非 rgb 序列化。只用 `parse_rgb` 的话，用 oklch 写的圆点（例如 Tailwind v4 默认色板）解析结果为 None，规则静默失效。计划自写的 hex fixture 测不出这个问题。
   - 证据：[element_checks.rs:49-51、122](../../tools/phoenix-ui/crates/core/src/browser/element_checks.rs)；[color.rs:160-180、859](../../tools/phoenix-ui/crates/foundation/src/color.rs)（`parse_any_color` 支持 oklch，见 :1106 的测试）。
   - 修订：改用 `impeccable_core::browser::element_checks::parse_rgb_or_any`。V-3、V-4 各加一个背景用 `oklch(...)` 写的圆点用例。
   - 复核：oklch 用例在 FakeDom 和真实 Chrome 上都命中。
   - 置信度：中高（Chrome 的序列化方式见待核实 Q-2）。阻塞：M2。

10. [P2] F-10 · Cursor before-edit 的影响面被低估，升级后的处理办法没写
    - 定位：§8 第 4 行（:383「Cursor 用户含无替代 `focus:outline-none` 的编辑」）、D4（:96）、§12（:447）、§5.4（:313）。
    - 问题：before-edit 先按编辑把整份文件投影出来，再扫描整份文件，不扣除编辑前已有的发现。所以文件里只要本来就有无替代的写法，对这个文件的任何无关编辑都会被拒，直到同一签名累计超过阈值。hooks.md 又禁止 agent 为了通过被拒的写入自行加 ignore，`ignore-rule` 需要用户同意。agent 只能去改范围外的代码（与 SKILL.src.md「Refinement preserves … everything outside scope」冲突），或者停下来问用户。叠加 F-07 的误报，项目升级后影响会放大。
    - 证据：[before_edit.rs:61-75、836-896](../../tools/phoenix-ui/crates/hook/src/before_edit.rs)；[hooks.md:57、61](../../skills/phoenix-ui/src/reference/hooks.md)。
    - 修订：
      - §8 的影响范围改为「编辑已含该模式的文件（包括无关编辑）」。
      - §5.4 补充 Cursor 升级说明：现象、修复办法，以及 `hooks ignore-value xgent/focus-outline-removed "*" --file <path>`。
      - V-1 增加一例：已含该模式的文件 + 无关编辑，记录实际结果。
      - 在 D4 明确保留 warning 的理由，或者改判为 advisory。
    - 复核：改稿后 §8、§5.4、D4 一致。
    - 置信度：代码事实高。阻塞：M1（测试）、M3（文档）。

11. [P2] F-11 · Widths 的默认宽度和检测次数与 new-work、polish 现有指令冲突
    - 定位：§6.4 Widths（:347）、I-5（:145「宽度并入同一批检查」）、D6；关联 R-3、场景 13。
    - 问题：craft-floor 在每次改 UI 前加载。§6.4 规定：项目未声明断点时用 375×812 和 1280×800，并对每个宽度跑 `detect --url`。但 new-work 已经固定取证宽度为 `desktop.png`（1440）、`mobile.png`（390），并检查 1280–1600；polish 写着「never add another detector pass」。agent 会拿到互相矛盾的指令：要么多跑一轮截图和检测，要么跳过 Widths。
    - 证据：[new-work.md:126、150、154](../../skills/phoenix-ui/src/reference/new-work.md)；[polish.md:95](../../skills/phoenix-ui/src/reference/polish.md)；[craft-floor.md:3](../../skills/phoenix-ui/src/reference/craft-floor.md)。
    - 修订：宽度取值顺序写成：项目断点 → 当前 playbook 已有的取证宽度 → 都没有时才用 375/1280。写明每个宽度一次的 `detect --url` 就是本批检测；页面规则不随 hook 运行，所以 hook 启用时也要跑。必要时同步 polish 中「detector pass」的措辞。
    - 复核：改稿后 grep craft-floor、new-work、polish 中的宽度和检测措辞，确认不再冲突；实施后看 V-9 场景 13 的工具调用。
    - 置信度：中高。阻塞：M4。

12. [P2] F-12 · operate.md 新增的「Controls and forms」没有加载入口
    - 定位：D5（:97）、§6.3（:331-343）；关联 R-3、场景 13。
    - 问题：src 中提到 `reference/operate.md` 的只有 SKILL.src.md:46 一句「See … for deeper Operate/Read guidance」。Setup 第 2、3 步只强制加载命令 reference 和 craft-floor。对 Operate 表单做 polish、harden 或局部优化时，agent 未必会读这 10 条，R-3 和场景 13 的成败取决于运气。
    - 证据：[SKILL.src.md:26-27、46](../../skills/phoenix-ui/src/SKILL.src.md)；`grep -rn 'operate.md' skills/phoenix-ui/src tools/phoenix-ui/crates`，排除 mode-operate 后只有上述一处。
    - 修订：在 craft-floor（每次编辑前必读）的 States 或 Verify 加一句指引，指向 operate.md § Controls and forms；不新增文件，不违反 C-8。
    - 复核：改稿后 grep 能找到强制入口；实施后看 V-9 场景 13 的读取记录。
    - 置信度：加载路径的事实为高，对 agent 行为的影响为中。阻塞：M4。

13. [P2] F-13 · design-reference 的抓取边界「不执行远端脚本」与「查看渲染页」「改用浏览器渲染」冲突
    - 定位：§6.5 第 1、2 条（:352-353）；关联 R-6、I-7、I-8。
    - 问题：从搜索结果得到的地址被限定为「只读页面及其同源样式与字体声明，不执行远端脚本」。而 design-reference 的参考主要来自搜索，且要求「评价视觉前查看实际截图或渲染页面」。浏览器渲染必然执行页面脚本，于是 agent 可能退化成只做文字分析，核心交付变弱。同一节里的「改用浏览器实际渲染」也自相矛盾。
    - 证据：[design-reference/SKILL.md:36、40](../../skills/design-reference/SKILL.md)。
    - 修订：把「不执行」限定为不在 shell 或 Node 中运行抓取到的代码；允许在隔离的浏览器会话中渲染观察，但不登录、不提交表单、不下载执行文件。
    - 复核：改稿后通读步骤 2，确认没有互斥；V-9 新增行的判据允许浏览器渲染。
    - 置信度：中高。阻塞：M4。

### P3

14. [P3] F-14 · `transition-all` 的注释跳过规则会吞掉通配选择器行
    - 定位：:263。
    - 问题：trim 后以 `*` 开头的行一律跳过，最典型的 `* { transition: all .2s }`、`*, *::before {…}` 永远不报；多行块注释中不以 `*` 开头的行反而会误报。
    - 证据：计划 :263–264 本身。
    - 修订：CSS 类文件复用 focus 规则「注释替换为等长空白」的做法；JS 类文件只跳过 `//` 行。V-2 加通配选择器正例。
    - 复核：V-2 的正例命中。
    - 置信度：高；阻塞：否。

15. [P3] F-15 · `emoji-icon` 用 `Extended_Pictographic` 打头，会把文本符号也当成 emoji
    - 定位：:299。
    - 问题：本地 regex-syntax 0.8.11 的这张表包含 ©®™‼、▶◀、☀–★ 等文本符号，FAQ 或步骤前的 `▶`、`★` 会成为候选；键帽 `1️⃣` 和国旗不以这类字符开头，会漏报。advisory 级别加上「≥ 2 个、≥ 20px、位于标题前」的条件，影响有限。
    - 证据：本地 cargo registry 中 `regex-syntax-0.8.11/src/unicode_tables/property_bool.rs:4795` 起的 `EXTENDED_PICTOGRAPHIC`，含 `©`、`®`、`‼`、`▪–▫`、`▶`、`☀–★`。
    - 修订：首字符改为 `\p{Emoji_Presentation}`，或「`Extended_Pictographic` + FE0F」。
    - 复核：V-3 加 `▶`、`★` 反例。
    - 置信度：高；阻塞：否。

16. [P3] F-16 · `font-family-count` 把同一系统字体的别名和通用族名算成不同族
    - 定位：:306。
    - 问题：`system-ui`、`-apple-system`、`blinkmacsystemfont`、`ui-sans-serif` 渲染的是同一系统字体，却会计为不同族。内置排版检查已约定跳过通用族名。
    - 证据：`tools/phoenix-ui/crates/core/src/browser/page_checks.rs:118-135`（:127 默认跳过 `CSS_GENERIC_FONTS`）、`crates/foundation/src/constants.rs:153`。
    - 修订：系统字体别名归为一类，跳过 CSS 通用族名。
    - 复核：V-3 加别名反例。
    - 置信度：中；阻塞：否。

17. [P3] F-17 · D4 和 V-1 没写 hook 分层：xgent 的 warning 在 Claude/Codex/Grok 下要到 Stop 才出现
    - 定位：D4（:96）、V-1（:395）。
    - 问题：xgent 规则不在 `IMMEDIATE_TIER_RULES` 里，计划又不改 foundation。启用分层的环境会把这些发现推迟到 Stop 深度扫描。hook 用例如果按「编辑后立即出现」来断言会失败。
    - 证据：[registry.rs:655-673](../../tools/phoenix-ui/crates/foundation/src/registry.rs)；[hook_lib.rs:1002-1024](../../tools/phoenix-ui/crates/hook/src/hook_lib.rs)。
    - 修订：D4 补一句分层行为；V-1 写明断言走 Stop 路径，或使用不分层的 harness。
    - 复核：V-1 的 hook 用例写法与分层一致。
    - 置信度：高；阻塞：否。

18. [P3] F-18 · lint 增量门要到 M5 才第一次跑，M1/M2 没有计划自己约定的快速静态检查
    - 定位：「验证节奏」（:27「……和快速静态检查」）对照 V-1–V-3（:395-397）、M1/M2。
    - 问题：`check-phoenix-lints.mjs` 按 hunk 摘要比对新增的 fmt/clippy 诊断，要改的文件基线里已有未格式化 hunk（hook.rs、hook_lib.rs、main.rs、detect/cli.rs 等）。问题会拖到 M5 的 `test:phoenix` 才暴露，前序计划就因此整轮重跑过。
    - 证据：[check-phoenix-lints.mjs:18-26](../../scripts/check-phoenix-lints.mjs)；[phoenix-ui.records/M3.md:147](phoenix-ui.records/M3.md)。
    - 修订：M1、M2 退出时加 `node scripts/check-phoenix-lints.mjs`。
    - 复核：改稿后 V-1/V-3 的命令中含 lint。
    - 置信度：中高；阻塞：否。

19. [P3] F-19 · §1.1 的「src 中无 `@media (hover` 指导」不成立，§6.3 第 6 条与现有守则重复
    - 定位：:177、I-4、§6.3 第 6 条（:339）。
    - 证据：[adapt.md:229-241](../../skills/phoenix-ui/src/reference/adapt.md) 已有 `@media (hover: hover)` 和「Don't rely on hover for functionality」。
    - 修订：缺口说明改为「operate.md 中没有」；第 6 条引用 adapt.md，或改写成增量表述。
    - 复核：§1.1 与 adapt.md 不再矛盾。
    - 置信度：高；阻塞：否。

20. [P3] F-20 · §5.2 第 4 步列的 hook 调用点不准
    - 定位：§5.2 第 4 步。
    - 问题：hook.rs:311/787、before_edit.rs:518 调用的是 `detector_detect_html(rt, …)`，本来就有 `rt`。真正需要传包的是 hook_lib.rs:1701 的 `to_scan_options` 调用；lib.rs 的 `run_hook_admin` 也走 `runtime()`。计划写的「测试补 `Engines` 字段」实际为空操作：全仓只有 main.rs:155 一处构造 `Engines`。
    - 证据：[hook_lib.rs:1630-1702](../../tools/phoenix-ui/crates/hook/src/hook_lib.rs)；`crates/hook/src/lib.rs:29-55`。
    - 修订：调用点改列为 hook_lib.rs:1701、hook.rs:319/792、before_edit.rs:836、stop_baseline.rs:167、lib.rs 的 `runtime()`，以及 `crates/hook/tests` 中的 `Runtime::new` 和 `detector_detect_text` 调用。
    - 复核：编译会暴露遗漏，属于文本精度问题。
    - 置信度：高；阻塞：否。

21. [P3] F-21 · R-2 宣称可以用 inline ignore 关闭，但 skill 告诉 agent 的标记引擎不认（既有缺陷）
    - 定位：R-2（:71）、§4 inline ignore 行（:222）、§5.4（:313）。
    - 问题：引擎只识别 `impeccable-disable*`，计划的 docs 和 V-1 用的也是这个标记；但 agent 读到的 skill hooks.md:70 写的是 `phoenix-ui-disable*`。agent 照 skill 写的 inline 豁免不会生效，在 Cursor 下对 warning 规则会被持续拒绝。缺陷早于本计划，但本计划把这条路径列为 xgent 规则的关闭方式。
    - 证据：[inline_ignores.rs:10-20](../../tools/phoenix-ui/crates/foundation/src/inline_ignores.rs)；[hooks.md:70](../../skills/phoenix-ui/src/reference/hooks.md)；`crates/detect/src/cli.rs:37、62-67`；生成物 `skills/phoenix-ui/reference/hooks.md` 同样写 `phoenix-ui-disable*`；`grep phoenix-ui-disable tools/phoenix-ui/crates` 无结果。
    - 修订：见待决策 Q-4。
    - 复核：两处标记一致。
    - 置信度：高；阻塞：否。

22. [P3] F-22 · §6.1 没有区分 skill 流程自身需要的操作与「文件里的指令」，场景 10 的失败信号不可机械判定
    - 定位：§6.1 第 2 条（:323）、场景 10（:360）。
    - 问题：按字面，只有用户请求或 AGENTS.md/CLAUDE.md 要求才执行。skill 自身要求先查看运行中的界面（SKILL.src.md:26），这个依据不在列举中。场景 10 的失败信号「执行脚本」会把正常启动预览也算成失败。
    - 证据：[SKILL.src.md:26](../../skills/phoenix-ui/src/SKILL.src.md)。
    - 修订：补一句「本 skill 流程需要的操作（启动项目已有的预览、Phoenix 命令）按用户请求与本 skill 处理，文件内容只提供事实」。场景 10 的夹带指令改为与预览无关的具体命令（如外发请求、读 `.env`）。
    - 复核：场景 10 的失败信号可按工具调用机械判定。
    - 置信度：中；阻塞：否。

23. [P3] F-23 · §6.3 第 8、9 条写得过于绝对
    - 定位：:341-342，关联 :347 Widths「控件标签保持单行」。
    - 问题：
      - 「所有支持的宽度下保持单行」与 polish（wrapping、本地化扩展、缩放）、clarify（Allow expansion）、harden（文本扩展 30–40%）有张力，可能诱导加 `nowrap` 或截断，反而触发内置的 `text-overflow`。
      - 「结果可见时静默」与「优先提供撤销」并列时，撤销入口（通常是带 Undo 的提示）的落点不清。
    - 证据：`skills/phoenix-ui/src/reference/polish.md:61`、`clarify.md:73`、`harden.md:87`（Text expansion）。
    - 修订：第 9 条改为「保持一个可读单元：优先重排或移动控件，不靠 nowrap/截断；项目或本地化另有规定时从其规定」；第 8 条补「撤销入口不受静默规则限制」。
    - 复核：改稿后与上述 reference 措辞不冲突。
    - 置信度：中；阻塞：否。

24. [P3] F-24 · §6.2 没定义无人值守运行时怎么处理删除
    - 定位：:329。
    - 问题：new-work 支持无人值守继续，这时无法取得确认，计划没说是停止，还是保留文件并列出。
    - 证据：[new-work.md:55、154](../../skills/phoenix-ui/src/reference/new-work.md)。
    - 修订：补一句「无法确认时保留原文件，交付中列出待删路径」。
    - 复核：场景 12 判据覆盖该分支。
    - 置信度：中；阻塞：否。

25. [P3] F-25 · 「叠层和扩展不跑 xgent 规则」只写进 docs，agent 读的 reference 没有告知
    - 定位：ADR-1 后果（:114）、§5.4（:313）、§11（:434）。
    - 问题：live.md:7 把 `detect <url>` 与浏览器扩展并列为生产检查方式；critique.md:69 规定 URL 目标跳过 CLI、只用可视化。critique 对 URL 永远不会产生 6 条页面规则的发现，agent 还可能把叠层结果当作完整结果。
    - 证据：[live.md:7](../../skills/phoenix-ui/src/reference/live.md)；[critique.md:69](../../skills/phoenix-ui/src/reference/critique.md)。
    - 修订：M4 在这两处各加一句「叠层与扩展不含 `xgent/` 规则，需要时运行 `detect --json <url>`」。
    - 复核：grep 两处。
    - 置信度：高；阻塞：否。

26. [P3] F-26 · M2 的委派会共写同一组文件，前置条件也写得过粗
    - 定位：M2（:420）。
    - 问题：6 条规则按函数委派 subagent，但都写在同一个 `src/dom.rs` 和 `tests/dom.rs`，还共用 mbx 托管的 target 锁。前置「骨架与 V-2」中，dom 规则其实不依赖文本规则。
    - 证据：计划 :419-420。
    - 修订：二选一：按规则拆子模块和测试文件，或规定 subagent 只返回代码、由主 agent 集成。前置改为「crates/rules 骨架 + `cargo check -p phoenix-ui-rules --locked --offline` 通过」。
    - 复核：改稿后 M2 的写入边界不重叠。
    - 置信度：高；阻塞：否。

27. [P3] F-27 · V-4 的判据「与 FakeDom 预期一致」没有逐个 fixture 的期望清单
    - 定位：V-4（:398）。
    - 问题：FakeDom 单测与 6 个 HTML fixture 没有一一对应。除 clean.html 外，每个文件在每个宽度下应命中哪些 ID 没有写出来，判定带主观性。
    - 证据：计划 :397-398。
    - 修订：列出 fixture × 宽度对应的期望规则 ID，或写进 `xgent_rules.rs` 的浏览器用例作为单一事实源。
    - 复核：V-4 判据可以逐项比对。
    - 置信度：中；阻塞：否。

## 2. 待核实 / 待决策

| ID | 缺什么 | 影响 | 谁可提供 | 最晚确认点 |
| --- | --- | --- | --- | --- |
| Q-1 | 「无名称、无 role 的 inline SVG」能否算作确定的可访问性缺陷：需要 axe-core `svg-img-alt` 适用范围，以及 Chromium 无障碍树的权威依据 | D4 把 `unlabeled-inline-svg` 定为 warning 是否成立。该规则只在 URL 扫描运行，不进 hook，影响限于 detect 的失败计数 | 实施者查官方文档或实测 | M2 前；证据不足就降为 advisory |
| Q-2 | Chrome 对用 oklch 指定的颜色，计算 `backgroundColor` 的序列化格式 | F-09 的后果程度（修订方案无论结果如何都安全） | V-4 实测 | M5 的 V-4 |
| Q-3 | `claude -p` 的默认权限行为，以及对符号链接的 `.claude/skills` 的加载 | F-06 的权限部分 | 有已登录 CLI 的维护者 | M5 的 V-9 前 |
| Q-4 | **待决策（范围）**：F-21 选哪种处理？① 推荐：M4 顺带把 `skills/phoenix-ui/src/reference/hooks.md:70` 改为 `impeccable-disable*`（增加 1 个文件，skill 与引擎一致）；② 不扩范围，在 §12 登记已知限制，R-2 的 inline 路径只对人工有效 | R-2 关闭路径对 agent 是否可达 | 用户或计划作者 | M4 前 |
| Q-5 | 改动落在既有未格式化 hunk 附近时，lint 基线是否判为新增 | F-18 的返工风险 | M1 实测 `check-phoenix-lints.mjs` | M1 退出 |

## 3. 覆盖摘要

- **基线**：计划 `sha256:0617bf9a…3fcb`；代码 `8a45e70`（dirty 仅计划与台账）。交付前重新计算计划 hash 一致，HEAD 未变。
- **分工**：四个只读 subagent 并行独立调查（facts、architecture、schedule、delivery），模型继承主 agent，回报均已收齐。主 agent 逐条复核了支撑问题的原始证据：
  - F-01：两份账本的绑定机制与当前 hash；
  - F-02：GENERATED 的 inputs；
  - F-03：deps 检查与构建复制路径；
  - F-04：smoke 断言；
  - F-06：bootstrap 的缓存定位与开发版无 URL；
  - F-07～F-09：计划原文、Dom 文档、颜色解析；
  - F-10：before-edit 整文件扫描；
  - F-11～F-13：reference 原文；
  - F-21：inline 标记。

  重复候选已按根因合并：facts F-01/F-02 与 schedule F-01 合并为 F-01；architecture F-01、facts F-04 与 schedule F-04 合并为 F-03；facts F-07 并入 F-04。
- **四维覆盖**：
  - facts：事实主张、行号与符号、全仓调用点、Cargo/workspace、仓库门禁。
  - architecture：ADR-1 与 D2 自洽，四条运行路径都会运行规则包；8 条算法的可行性与误报；版本事实已对照本地 regex-syntax 源码。
  - schedule：R/NFR/C/A → 设计 → M → V 双向追踪，依赖与编排。
  - delivery：使用者 A/B/C 的主路径、skill 守则质量、hook 用户体验。

  ADR-1/D2、快照白名单、oracle 机制、render 断言、ENGINE.json 提交开发清单的惯例都核实成立。提交开发清单不构成安装断点：HEAD 本来就是 development 清单，`prepublishOnly` 会拒绝直接发布。
- **脚本**（在仓库根运行 `skills/dev-plan/scripts/`）：
  - `check_plan.py`：ERROR 0、WARN 0。
  - `check_progress.sh`：0/5，ERROR 0、WARN 1（未提交改动，来自计划本身与台账，尚未开工，无影响）。
  - `check_paths.sh`：MISSING 4，人工判定都不是缺陷：`tools/phoenix-ui/crates/rules` 是 ★ 新增；`.phoenix-ui/config.json` 是目标项目配置示例；`agents/openai.yaml` 出现在 C-8 的排除说明里；`docs/project-status/protocol.md` 是兼容说明。

  脚本通过不证明设计或验收有效。
- **未运行**：`npm test`、`npm run test:phoenix`、cargo build/test、prepare、真实 Chrome、Claude CLI。HEAD 当前是否全绿未知。UI/UX 结论属于**文档推演**，没有可运行界面，也没有浏览器或 provider 会话，skill 的实际行为待 V-4、V-7、V-9 验证。
- **非问题记录**：
  - A-2 漏数了 `tests/oracle/workspaces/{detect-config,hook-project}` 中的 `transition: all`，计划已有逐 case 兜底，只影响 M3 工作量。
  - C-7 注释所指的 `STYLE_PROPS` 实际位于 foundation，路径小误，不影响实施。
  - hallmark 内容只抽查了部分门的对应关系。

## 4. 映射与实施编排

**映射**：
- 已确认完整：R-1 → V-10；R-4/R-5/R-6 → §6 → M4 → V-6 + V-9；NFR-1 → V-2/V-3/V-7；NFR-2 → V-1/V-5；A-1 → V-7；A-2 → M3/V-5；C-2 → V-5；C-5 → V-10；C-8 → V-6；§6.6 场景 10–13 与 V-9 一一对应；I-1–I-19 由 V-10 逐项核对。
- 断链：C-3 → V-6（F-02）；§1.2 范围 → V-5/V-6/V-8（F-01、F-03）；V-7 修复 → V-4/V-8 失效（F-04）。
- 薄弱：R-2 的 `--no-advisory` 在 V-1 没有断言；R-3 的「伪造窗口外壳拒绝项」没有 V-9 行为场景；C-6 没有核验手段。修订时可在 V-1、V-10 顺手补上，不单列问题。

**建议编排**（只描述拓扑，不估工期）：

| 波次 | 前置及解锁证据 | 可同时做 | 写入边界 / 负责人 | 运行资源 | 汇合验收 |
| --- | --- | --- | --- | --- | --- |
| W0 基线 | prepare 回执已导出（联网）；起始 SHA | — | 只写 M1 记录 | mise/Rust 1.99/WASM 工具、Chrome | `test:phoenix` 结果与既有失败分型（F-05） |
| W1a 骨架 | W0 | — | 主 agent：Cargo.toml、Cargo.lock、`crates/rules/src/lib.rs`（ROWS 与分派） | mbx target 串行 | `cargo check -p phoenix-ui-rules` |
| W1b 文本规则与接线 | W1a | W1c、M4 | 主 agent：detect/hook/cli 接线、hook 测试；账本重绑（F-01） | 同上 | V-1、V-2 + lint（F-18） |
| W1c 页面规则 | W1a（F-26） | W1b、M4 | 按规则拆子模块，或 subagent 只返回代码 | 共用 target 锁 | V-3 + lint |
| W2 M3 | V-1–V-3 | M4 | 主 agent：phoenix.json、DELTAS.md、docs/phoenix-ui.md | debug 二进制固定副本 | V-5 |
| M4 守则 | W0 | W1–W2 | skills/phoenix-ui/src、evals、skills/design-reference；不碰生成物 | 预览输出到 local | 改为局部检查的 V-6（F-02） |
| W3 页面验收 | M3、M4 | — | 回改规则由主 agent 负责 | 同一 Chrome 会话串行 | V-4、V-7；修复后重跑受影响的 V-1/2/3/5/4 |
| W4 构建与回归 | 规则定稿 | — | 主 agent：生成物、`dist/phoenix-ui/**`、THIRD-PARTY*、capabilities.json | prepare 回执；先清理 development-release | `deps --write` → build → smoke → `npm test` → `test:phoenix`（V-8） |
| W5 行为与核对 | V-8 | — | 只写 `local/phoenix-ui/behavior/` 与 M5 记录 | 已登录 Claude CLI；隔离的 `PHOENIX_UI_HOME` 已安装开发引擎（F-06） | V-9、V-10 → 回写 → 单次提交 |

瓶颈链：W0 → W1a → {W1b ‖ W1c} → W2 → W3 → W4 → W5；M4 不在关键链上。只在 M5 提交一次的约定成立。

## 5. 总体结论与修订顺序

**是否需要修改：需要**

**建议状态：`Blocked`**，与计划自报的 `Ready` 不一致。理由：

- F-01 是范围与 C-2 合同层面的错误。M3 的 V-5、M4 的 V-6、M5 的 V-8 按原文都会机械失败，实施者还会被迫自行决定「重绑证据 hash 是否算放宽 oracle」。
- F-02～F-06 让 M1 基线、M4 退出、M5 收口的编排和执行条件不成立。
- F-07～F-09 是 warning 和页面规则的误报或漏报缺口，直接影响 NFR-1。

这些问题都不需要用户重新做产品决定，三项已确认决定不受影响。唯一需要拍板的是范围小项 Q-4。阻塞范围是 M3–M5 的退出门；修订前，M1/M2 的规则开发如果先行，要按 F-07～F-09 修订后的算法实现。

**修订顺序**：
1. 先纠正前提和合同：F-01（账本纳入范围，C-2 写明重绑口径）、F-03（许可清单与 dist 派生物）。
2. 再修编排：F-02（V-6 改为局部检查）、F-04（M5 三段顺序与失效清单）、F-05（prepare 前置）、F-06（V-9 执行条件）。
3. 再修算法与设计：F-07、F-08、F-09、F-10（D4/§8/§5.4）、F-11、F-12、F-13。
4. 处理 Q-4，并视情况采纳 P3（F-14～F-27）。

**解除阻塞的可观察条件**：修订后的计划满足以下全部条件：
- §1.2 列出 F-01、F-03 的文件；
- M1/M3/M5 写明重绑与同步时点；
- V-6 不含整仓入口；
- M5 顺序为「V-4/V-7 → 定稿构建 → V-8 → V-9」；
- V-9 写明 engine 安装、权限与素材；
- §5.3 中三条算法按 F-07～F-09 改写。

计划评审通过不表示功能已实现或可上线。

**下一步**：

```text
/apply-doc-review docs/plan/phoenix-ui-hallmark.review.md
```

（Codex 写作 `$apply-doc-review docs/plan/phoenix-ui-hallmark.review.md`。）核实并修订计划，处置后判定 Blocked 或 Ready。

## 修订处置与复核

- 来源：本报告原始版本 `sha256:3e21bb84d362472a667783ecf577c1b0af46476a23451633eab24d43da714cd1`；原问题、证据、所审基线与原结论保持不变。
- 目标：[phoenix-ui-hallmark.md](phoenix-ui-hallmark.md)；修前 `sha256:0617bf9a54c52b7bbfc0349d6b9b93b6d64dd2489aa80083dc954ddc0ba93fcb`（448 行），修后 `sha256:893c01bae1d5d1c04f1e42f7bf9f28cb1f160790c2be51f2de46e79a5ca66a1c`（477 行）。
- 复核日期与代码基线：2026-10-22 23:20 +0800；main `8a45e7075a8a024e2a8775f3e3edb06ecce2e5ee`，工作树 dirty（计划、报告与台账未跟踪/未提交）。
- 本次范围：全部 F-01–F-27 与 Q-1–Q-5，无用户排除项。用户就 Q-4 在 2026-10-22 选择 ③（改引擎、新增前缀别名），见「决策与未完项」。
- 判读边界：下表的「已解决/已修订并复核」只指文档层复核；实现、构建、真实 Chrome 与 Claude CLI 的判据仍是计划内的 V-1–V-10，本次未运行。

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。`verifyCorpus()` 在回放前比对 `currentEvidenceFiles`（adapters/phoenix.mjs:61，check-phoenix-oracle.mjs:14），40 项含本期的 main.rs、detect/cli.rs、hook.rs、hook_lib.rs、before_edit.rs；capabilities.test.js:64、99-100、382-386 校验 sourceFiles、cli 行号与 environment 行号；GENERATED inputs 76 项。报告一部分证据需修正：expectations 的 `source` 行号没有通用校验，只有 oracle.test.js:121-125、154-160 两处硬编码断言，且都指向 live/ 文件，不在本期改动内；修订只重绑真正生效的 hash 与行号，核心结论不变 | 已修订并复核 | §1.1 增账本事实；§1.2 增 phoenix.json、capabilities.json、compatibility.md、ownership.json 与生成物行；C-2 明确重绑不算放宽比较；§5.2 新增第 8 步；M1/M3 重绑、M5 同步 generatedBundle | 已解决（文档）。V-5/V-8 取证：实施后不再出现 `current expectation source changed` |
| F-02 | 成立。GENERATED inputs 含将改的 5 个 skill 源；capabilities.test.js:310 逐个校验 inputs；run-tests.mjs 收集 `test/**/*.test.js`。M4 不同步生成物时 `npm test` 必失败 | 已修订并复核 | V-6 改本地检查（render --without-engine 预览、test/skills 与 test/dev-plan 的 unittest、git diff --stat）；M4 退出改用 V-6；整仓 `npm test` 只在 V-8 | 已解决（文档）。V-6 不再有整仓入口 |
| F-03 | 成立。`--write` 写 THIRD-PARTY.json 与 dependency-audit.json（check-phoenix-deps.mjs:513-516、534）；不带 `--write` 走 requireEqual；build 把 THIRD-PARTY 三件套复制到 release 与 dist（build-phoenix-ui.mjs:105-110）；CI 与维护文档都在 prepare 之后 `--write` | 已修订并复核 | §1.2 增 THIRD-PARTY*、dependency-audit.json 与 dist 行；V-8 改为 prepare → `--write` → 构建 → 带 `--distribution-dir` 的 audit → smoke；§12 增「`--write` 必须在构建之前」 | 已解决（文档）。V-8 取证：不带 `--write` 的审计通过且清单含 phoenix-ui-rules |
| F-04 | 成立。smoke 的 stageRuntime 断言 runtime 目录不存在（smoke-phoenix-ui.mjs:37）；build 用 `walk('crates')` 计输入摘要；test-phoenix-ui.mjs:31-37 先复制引擎二进制 | 已修订并复核 | M5 改为三段：debug 二进制固定副本 → V-4/V-7 → 定稿后 prepare/`--write`/构建/smoke/audit → `npm test`/`test:phoenix` → V-9/V-10；写明清理 development-release、失效清单与重建条件；V-4 先重建 debug 二进制 | 已解决（文档）。M5 顺序满足报告条件 |
| F-05 | 成立。test-phoenix 在模块加载时 `preflight()`；prepare 需联网（prepare-phoenix-ui.mjs:315）；test:phoenix 自身把 oracle 报告写到 `local/phoenix-ui/ci-prepare/<target>-oracle.json` | 已修订并复核 | §9.2 增 prepare 前置、网络与「缺环境不算既有失败」；基线复用 test:phoenix 报告，M3 用独立 `oracle-xgent.json`；恢复快照同步 | 已解决（文档）。W0 基线可执行 |
| F-06 | 成立。开发版 ENGINE 无下载 URL（phoenix-bootstrap.cjs:254）；`engine install` 支持 `--release-dir`（:307）；缓存根取 `PHOENIX_UI_HOME`（:56） | 已修订并复核 | V-9 增隔离 HOME 下的 engine install、场景素材、权限参数与「权限拒绝记未运行、detect 不可用不算失败」；§11 增 Q-3 | 已解决（文档）。运行时取证在 M5；`claude -p` 权限行为仍为 Q-3 |
| F-07 | 成立。原算法命中接受任意变体前缀、豁免只认字面 `focus:` 开头，`focus:outline-none dark:focus:border-white` 会被判成未写替代 | 已修订并复核 | §5.3 改为命中与豁免共用同一套变体链解析；V-2 增四个反例 | 已解决（文档）。V-2 取证反例 0 命中 |
| F-08 | 成立。`text_line_rects` 的文档与默认实现位于 dom.rs:173-198，合并逻辑从 :214 起；它返回的已经是按行合并、包含后代的结果；再按 2px 聚类并汇总整棵子树会把标题加副标题的按钮误判为多行 | 已修订并复核 | §5.3 改为「存在一个有直接文本、无带文本子元素的叶子后代，其行数 ≥ 2」，去掉二次聚类；V-3 增标题加副标题、带说明菜单项、标签加徽标三种反例 | 已解决（文档）。V-3 取证 |
| F-09 | 成立。内置元素规则用 `parse_rgb_or_any`（element_checks.rs:50），`parse_any_color` 支持 oklch（color.rs:859、1106）；只用 `parse_rgb` 会漏掉 oklch 写的颜色 | 已修订并复核 | §5.3 通用约定与 redrawn-window-chrome 改用 `parse_rgb_or_any`；V-3 增 oklch 圆点正例 | 已解决（文档）。V-3 与 V-4（Q-2）取证 |
| F-10 | 成立。before-edit 扫投影后的整份文件（before_edit.rs:836），拒绝与本次编辑内容无关；只有同文件同签名超过阈值才降级 | 已修订并复核 | §8 改写影响面、用户表现与恢复；§5.4 增 Cursor 升级说明；§12 更正原描述 | 已解决（文档）。V-1 取证 |
| F-11 | 成立。new-work 采集 1440/390 并检查 1280–1600（new-work.md:126、150）；polish 禁止再加检测轮次（polish.md:96） | 已修订并复核 | §6.4 Widths 改为三级取值，并写明「同一批取证里按宽度各跑一次」是与 polish 不冲突的唯一新增批次 | 已解决（文档）。V-6 关键词检查 |
| F-12 | 成立。craft-floor 的 States 只列状态，表单细节在 operate.md，两者未互引 | 已修订并复核 | §6.4 增 States 指向 operate.md 的 `Controls and forms` 交叉引用 | 已解决（文档）。V-6 |
| F-13 | 成立。design-reference 要求评价视觉前查看实际截图或渲染页面（:40），与原「不执行远端脚本」的绝对表述冲突 | 已修订并复核 | §6.5 区分「不执行抓到的代码」与「允许在隔离浏览器里渲染观察」 | 已解决（文档）。V-6 检索边界段落 |
| F-14 | 成立。原来把「trim 后以 `*` 开头」当注释跳过，会吞掉 `* { transition: all }` | 已修订并复核 | §5.3 改为 CSS 注释等长空白替换、JS/TS 只跳过 `//` 行；V-2 增通配行正例 | 已解决（文档）。V-2 取证 |
| F-15 | 成立。regex-syntax 0.8.11 的 `Extended_Pictographic` 含 ©、®、‼、▶、★；`Emoji_Presentation` 不含这些文本符号，含国旗区域指示符 | 已修订并复核 | §5.3 首字符改用 `Emoji_Presentation`；写明数字键帽仍不覆盖；V-3 增 ▶/★ 反例 | 已解决（文档）。V-3 取证 |
| F-16 | 成立。page_checks 选 primary 时只跳 CSS 通用族（page_checks.rs:118-132、constants.rs:153）；系统字体栈会因别名各自成为独立主字体 | 已修订并复核 | §5.3 增系统别名归类与通用族名不计入；V-3 增别名反例 | 已解决（文档）。V-3 取证 |
| F-17 | 成立。分层只立即报 `IMMEDIATE_TIER_RULES`，其余推 Stop；cursor/github 不分层，`per_edit_rules: all` 也不分层（hook_lib.rs:1001-1023） | 已修订并复核 | D4 增分层行为与 V-1 写法要求；V-1 写明走 Stop 或设 `per_edit_rules: all` | 已解决（文档）。V-1 取证 |
| F-18 | 成立。lint 脚本只拦截新增的 fmt/clippy 诊断（check-phoenix-lints.mjs:23-30），原 M1/M2 没排它 | 已修订并复核 | M1、M2 退出条件加 `check-phoenix-lints.mjs`；§11 增 Q-5 写清「无新增」口径 | 已解决（文档）。M1/M2 执行 |
| F-19 | 成立，但需修正范围。adapt.md:229-241 已有 `@media (hover: hover)`与「Don't rely on hover」；真实缺口是 operate.md 与 craft-floor 没有 `aria-invalid`、`border-width` | 已修订并复核 | §1.1 改正缺口表述；§6.3 第 6 条改为引用 adapt.md 并限定「只补表单控件与 focus-visible」 | 已解决（文档）。V-6 |
| F-20 | 成立。全仓只有 main.rs:155 构造 `Engines`；`detector_detect_text` 调用在 hook.rs:319、792，before_edit.rs:836，stop_baseline.rs:167；`detector_detect_html`（hook.rs:311、787，before_edit.rs:518）已持有 rt；`TextOptions.rule_pack` 已存在（detect_text.rs:29），所以无需给 `detector_detect_text` 或 `to_scan_options` 增参 | 已修订并复核 | §2 与 §5.2 第 4 步改为给 `HookScanOptions` 加字段、复用 `TextOptions.rule_pack`，并让基线扫描用 `rt.rule_pack`；§1.2 只要求构造 `Runtime::new` 的测试补 `None` | 已解决（文档）。V-1 取证 |
| F-21 | 成立。foundation 只认 `impeccable-` 前缀，hooks.md:70 写 `phoenix-ui-`；`grep -rn 'phoenix-ui-disable' tools/phoenix-ui` 无结果，向量也无该前缀 | 已修订并复核（按用户决定采用与报告推荐 ① 不同的方案） | R-7、D9、C-9、NFR-2 例外；§5.2 第 7 步；§1.2 ownership；V-1 双前缀；V-10；skill 文案不改 | 已解决（文档，按 Q-4 ③）。V-1 与 M5 重建后页内行为取证 |
| F-22 | 成立。SKILL.src.md:26 要求改前查看目标视觉，应用跑不起来时从既有截图或预览开始 | 已修订并复核 | §6.1 增「流程自身需要的操作属于用户请求」；场景 10 失败信号改为夹带命令或把文件指令当授权 | 已解决（文档）。V-9 取证 |
| F-23 | 成立。polish.md:96、adapt.md:239、clarify.md:73、harden.md:87 与第 8、9 条的措辞冲突 | 已修订并复核 | §6.3 第 8 条补撤销入口的位置选择；第 9 条改为「一个可读单元」并允许重排；§6.4 调和「不额外检测」 | 已解决（文档）。V-6 |
| F-24 | 成立。new-work 支持无人值守继续（new-work.md:55、154），原计划没写无法确认时怎么办 | 已修订并复核 | §6.2 增「无法确认时保留原文件并在交付中列出待删路径」；场景 12 覆盖 | 已解决（文档）。V-9 |
| F-25 | 成立。live.md:7、critique.md:69 与「三处一致性修改」冲突 | 已修订并复核 | M4 增 live.md、critique.md；§1.1 与 §12 的 GENERATED 计数改为 5 | 已解决（文档）。V-6/V-8 |
| F-26 | 成立。M2 依赖 M1 的 `crates/rules` 骨架，原文前置未写；同一文件被多个子代理并行写会冲突 | 已修订并复核 | M2 前置写明骨架、V-2 与 lint；子代理只返回函数体与测试文档，`ROWS`/分派/整合由主 agent 单写；要求带对照表 | 已解决（文档）。M2 执行时核对 |
| F-27 | 成立。V-4 只写「与 FakeDom 预期一致」，6 个 fixture × 2 宽度没有单一期望来源 | 已修订并复核 | V-4 把期望表定为 `crates/cli/tests/xgent_rules.rs` 的同源常量，并先重建 debug 二进制再复制 | 已解决（文档）。V-4 取证 |
| Q-1 | 开放事实：`unlabeled-inline-svg` 是否算确定的可访问性缺陷，需 axe-core `svg-img-alt` 适用范围与 Chromium 无障碍树的依据；D4 的 warning 定级依赖它 | 待核实 | §11 增 Q-1，最晚确认点 M2；V-7 人工核对；证据不足降 advisory | 未决，不阻塞 |
| Q-2 | `parse_any_color` 支持 oklch（color.rs:1106），但 Chrome 对圆点颜色的序列化未实测 | 待核实 | §11 增 Q-2；V-4 用真机 fixture 取证 | 未决，不阻塞 |
| Q-3 | `claude -p` 的默认权限行为，以及符号链接的 `.claude/skills` 能否被加载，均未实测 | 待核实 | V-9 先用最小会话验证符号链接加载（不行则复制安装），并记录实际命令、权限参数与拒绝原因；§11 增 Q-3 | 未决，不阻塞 |
| Q-4 | 用户 2026-10-22 选择 ③ | 已决策 | 落地为 R-7、D9、C-9、NFR-2 例外与 §5.2 第 7 步 | 已解决；影响见下节 |
| Q-5 | 返工风险：改动落在既有未格式化 hunk 附近时，fmt 基线可能判为新增 | 已修订并复核 | §11 增 Q-5：退出口为无新增诊断；若附近 hunk 被误判，只格式化本次触及的行并记录，不为清零改历史格式 | 已解决（文档）。M1 实测 |

### 决策与未完项

- **Q-4（范围）**：用户 2026-10-22 选择 **③ 改引擎**——`crates/foundation/src/inline_ignores.rs` 的两处前缀改为把 `ci("impeccable")` 与 `ci("phoenix-ui")`（`HAS_DIRECTIVE_RE` 为 `-disable` 形式）组合成 `(?:…|…)`，skill 的 hooks.md 文案保持不变；报告推荐的 ①（改 hooks.md）与 ②（只登记限制）均未采用。落地为 R-7、D9、C-9、NFR-2 的唯一例外、§5.2 第 7 步、§1.2 的 `patches/ownership.json` 行、V-1 双前缀断言与 V-10 文档核对。影响：计划范围 +1 个上游文件；该文件从「无需修改」移出；页面 WASM 在 M5 重建后同样接受新前缀；`disable`/`disable-line`/`disable-next-line` 语义、规则列表解析、`*` 通配与内置规则输出不变；既有向量无该前缀用例，录制期望不用改。
- **未决但非阻塞**：Q-1（M2 单测与 V-7 语料核对）、Q-2（V-4 真机取证，不成立则降 advisory）、Q-3（V-9 记录实际权限参数与拒绝原因）。解除条件与最晚确认点已写入 §11。
- 报告 §4「薄弱」的三项已补：`--no-advisory` 进 V-1 断言、伪造窗口外壳新增场景 14 并进 V-9、C-6 在 V-10 增加核验命令。

### 修订后结论与验证

- 覆盖：F-01–F-27 共 27 项全部处置（均为已修订并复核，文档层）；Q-1–Q-3 保留为计划内取证（待核实），Q-4 已决策，Q-5 已澄清。
- 实际运行的检查：`check_plan.py` ERROR 0 / WARN 0；`check_paths.sh` 27 个路径样式 token、MISSING 6——`tools/phoenix-ui/crates/rules`（★ 新增）、`.phoenix-ui/config.json`（目标项目示例）、`agents/openai.yaml`（C-8 排除说明）、`docs/project-status/protocol.md`（兼容说明）、`local/phoenix-ui/oracle-xgent.json`（V-5 输出）、`skill/SKILL.src.md`（能力账本键名，已就地注明实际路径）；`check_progress.sh` 0/5、ERROR 0、WARN 1（计划未跟踪，脚本看不到修改，实际已改）。
- 未运行：`npm test`、`npm run test:phoenix`、cargo build/test、prepare、真实 Chrome、Claude CLI；本次未改任何代码或生成物。
- 受影响但未授权修改的文档：docs/phoenix-ui.md（M3 更新）、`tools/phoenix-ui/compatibility.md`、`patches/ownership.json`、`capabilities.json`、`skills/phoenix-ui` 生成物（实施期同步）。
- 判定：**Ready**。评审提出的阻塞条件（账本与许可清单在 §1.2、M1/M3/M5 时点、V-6 无整仓入口、M5 顺序、V-9 可执行条件、三条算法改写）在文档层均已满足；Q-4 已由用户决定；Q-1–Q-3 是可实测的取证项，不改变范围、数据、外部契约或关键 NFR。
- 这不代表功能已实现或可上线，也不是对本计划的再一次完整独立评审。

**下一步**：

```text
/goal 按 docs/plan/phoenix-ui-hallmark.md 开发
```
