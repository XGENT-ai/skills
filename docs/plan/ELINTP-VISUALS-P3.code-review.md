# ELINTP-VISUALS-P3 代码评审 · R1

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [ELINTP-VISUALS-P3 · 静态图与看图审核减量](ELINTP-VISUALS-P3.md) | `63ec49616e703a9fb8bd870bde740c32a6f7cb0d^1..bfd626b55499f96a7a489cb85d1d0717dc692eed` 净差异；工作树 clean，无未提交变更 | 无 |

意图：phoenix-diagram 新增 `static` 命令输出可内联的静态 SVG（两种字体策略、亮暗两张审核截图），单图/比较审核截图减为 2/6 张；elintp 新增 `:::figure` 的 `static:` 来源，经安全解析后内联静态图。

## 确认问题

### F-01 · [P1] elintp `static:` 拒收四类图的真实引擎产物

- 定位：[skills/elintp/scripts/draft.mjs:10](../../skills/elintp/scripts/draft.mjs#L10)（`STATIC_SVG_ATTRIBUTES` 缺 `pointer-events`）；[skills/elintp/scripts/draft.mjs:134](../../skills/elintp/scripts/draft.mjs#L134)（凡遇 `+`/`~` 一律拒绝），target `bfd626b5`
- 问题：phoenix-diagram `static` 对十一类图都能成功出图，但其中 architecture、lifecycle、statechart 的产物在自动交叉线底衬 `<path … pointer-events="none"/>` 上带有 `pointer-events` 属性，erd 产物的类型 CSS 含选择器 `svg[data-phoenix-static][data-erd-ui="cards"] .er-key-badge + text`。elintp 的静态属性白名单只在 CSS 属性表里收了 `pointer-events`，SVG 属性表里没有；`selectorChain` 又把任何兄弟组合器都当作"跳出静态 SVG"拒绝，即便它位于根锚点之后的绘图层级、根本跳不出 SVG。于是作者按 elintp SKILL.md 要求内联图（WORK-F5/AC-2，也是 summarize 的阅读面）时，只要页面里有架构图、ER 图、生命周期图或状态机图，`render.mjs` 就以退出码 2 报格式错误，只能退回自绘 `svg:`。架构图正是本场景最常见的一类。计划 §6.1 要求白名单"按实际引擎产物补"，且只拒绝"跳出 SVG 的组合器"；现有 V-3/V-6 只用 workflow 和 sequence 走过 elintp，V-4 用的是手写 fixture，所以没有发现。
- 证据：用被审版本的开发引擎（`local/phoenix-diagram/elintp-p3-final`，191 个源输入 SHA-256 与当前树逐一相同）对 architecture/erd/lifecycle/statechart/workflow 的最小示例重新运行 `static --font-policy system`，五个都退出 0；随后作为 `static:` 交给 `node skills/elintp/scripts/render.mjs`：architecture、lifecycle、statechart 退出 2，报「SVG L68/L43 path：属性 pointer-events 不允许」；erd 退出 2，报「CSS 选择器不能用兄弟组合器跳出静态 SVG」；workflow 退出 0。对 `local/phoenix-diagram/elintp-p3-final-out/` 里全部 22 份产物（11 类 × 2 策略）逐份执行，有 8 份失败，涉及的正是这四类。在临时副本里只补 `pointer-events` 属性并放行兄弟组合器后，22 份全部通过，说明没有第三个阻断点。
- 修复与验证：把 `pointer-events` 加入 `STATIC_SVG_ATTRIBUTES`（取值仍只接受 `none`，与 CSS 侧一致）；`selectorChain` 只在根复合选择器及其之前拒绝 `+`/`~`，根锚点之后的绘图复合选择器之间允许使用。验证方式是扩展场景 E2E：用 V-5 第 3 步的 22 份真实 `static` 产物逐份经 `render.mjs` 内联并断言退出 0，同时保留现有"根上 `svg[data-phoenix-static] + svg` 被拒"的用例。
- 置信度：高，已在被审版本上实际复现。

### F-02 · [P2] `static:` 的 CSS 原样输出，HTML 字符引用可绕过作用域与离线检查

- 定位：[skills/elintp/scripts/draft.mjs:333](../../skills/elintp/scripts/draft.mjs#L333)（`parseStaticCSS` 的结果未转义就拼进页面），配合 [draft.mjs:232](../../skills/elintp/scripts/draft.mjs#L232)（只拒绝 `\`、NUL 和 `<`），target `bfd626b5`
- 问题：`<style>` 文本先做一次 XML 实体解码，再交给 `parseStaticCSS` 校验；校验通过后，CSS 原样写进 HTML 页面里的内联 `<svg><style>`。HTML 解析器在 SVG 外来内容中会再解码一次字符引用，而且接受不带分号的数字引用。于是源文件写 `&amp;#125`，校验器看到的是无害的 `&#125`，浏览器看到的却是 `}`。伪造标记的 SVG 就能在一条通过校验的声明里写出 `fill:red&#125 * &#123 display&#58 none&#125 html &#123 background&#58 url&#40 https&#58//…&#41`：整页被隐藏，并发出外部网络请求。这违反计划 §6.1「实体与 CSS 转义须在 URL/属性判断前解码或明确拒绝，不能只扫描危险字符串」「不允许匹配宿主元素」，也违反 C-1 引用的 WORK-N2 零网络要求。不能执行脚本，但安全边界承诺的样式隔离与离线性质被突破。审阅页走同一个 `parseDraft`，同样受影响。
- 证据：复现材料在临时目录 `/private/tmp/claude-501/-Users-rockie-Documents-gh-xgent-skills/01641c38-078c-4da0-9944-3e2a3615e8af/scratchpad/f01/`（内容稿 + 伪造 `figure.svg`）。`render.mjs` 退出 0，产物 style 原样含 `&#125 * &#123 …`；用 Chrome headless 打开后实测得到 `getComputedStyle(body).display === "none"`、`html` 的 `background-image` 为 `url("https://example.com/leak.png")`，style 文本被解析为 `…{fill:red} * { display: none} html { background: url( https://example.com/leak.png)}`。引擎自己的 `static_style.rs` `validate_value` 拒绝 `&`，修正后的产物 CSS 也不含 `&`。
- 修复与验证：去掉注释后，若 CSS 中仍有 `&` 就拒绝（与引擎侧一致，最简单）；或者在输出时把 `&` 转义为 `&amp;`。在 `test/elintp/render.test.js` 已有的攻击表里加一条双重编码的字符引用用例，断言退出 2、旧页字节不变。
- 置信度：高，已实际复现。

### F-03 · [P3] phoenix-diagram SKILL.md 正文重复了 static-figure.md 的完整流程

- 定位：[skills/phoenix-diagram/SKILL.md:92–106](../../skills/phoenix-diagram/SKILL.md#L92-L106)「可内联的静态图」一节（bundle 副本同样存在），target `bfd626b5`
- 问题：计划 §5.7 对 SKILL.md 只要求改描述、在「按请求启用」列表加一个指向 `references/static-figure.md` 的入口（L49 已加）、以及改第 4 步的截图数。实际又在正文新增一节，把命令、字体策略、产物、宿主约定和登记规则完整重写了一遍，与 `static-figure.md` 内容重复。SKILL.md 每次触发都会加载，这一节违背了"能力按请求启用"的设计，让不需要静态图的普通作图也背上这段上下文；两处规则以后也须同步维护，违反 AGENTS.md `@documentation` 的 `duplicate_docs`。
- 证据：SKILL.md L49 的入口与 L92–L106 的正文，对照 `references/static-figure.md`「命令」「产物」「看图与登记」三节。
- 修复与验证：删除 L92–L106，保留 L49 的入口；改完运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle`，再跑 `node --test test/phoenix-diagram/skill.test.js`。
- 置信度：高。

## 待核实/待决策项

无。下文提到的证据缺口都已在本轮自行补证，或改用本轮实测证据裁定。

## 覆盖与验证

- 仓库：`/Users/rockie/Documents/gh-xgent/skills`，评审日期 2026-11-13。原输入为 `docs/plan/ELINTP-VISUALS-P3.md`（plan 模式，初审）。
- 范围解析：完成记录登记了 `63ec4961`（M1–M3）、`bfd626b5`（M4）、`2fb94fb6`（M5），三者都在当前分支上。`2fb94fb6`、`6ca033cd`、`f4e83113` 只改本计划文件和 `.records/`，属于进度回写，排除在审查对象之外。`63ec4961` 与 `bfd626b5` 相连，按 `63ec4961^1..bfd626b5` 净差异审查（计划目录外 44 个文件，含 bundle 副本）。HEAD `f4e83113` 与 `bfd626b5` 之间 `tools/ skills/ test/` 没有差异；工作树 clean。
- 已审行为链：
  - CLI：`static` 命令、`--font-policy`、`.svg` 后缀校验 → `delivery::static_figure`（validate → static → browser-check，失败语义，summary 路径）→ `render_static` / `static_style.rs`（Reader SVG 加命名空间后做标记转换；viewer.css、类型 CSS 与字体块的解析与主题/打印展开；属性清理；背景矩形与根属性）。
  - 浏览器检查：`browser/static_figure.rs` + `static-probe.js`（宿主页、两主题、四项检查、截图、清理）；`evidence.rs` 文件名主干；`browser_check` 对 `.svg` 的分流。
  - 截图减量：`mod.rs` / `comparison.rs` / `evidence.rs` 及四个测试文件的数量断言；`observation::summarize_named`。
  - 审核登记：`delivery/review.rs` 按 `summary.command` 分派恢复提示；`support::summary_path`。
  - elintp：`draft.mjs` 静态白名单、CSS 解析、选择器作用域、URL/MIME 位置、重序列化；`page.css` 窄屏与打印规则。
  - 文档：phoenix-diagram SKILL.md、五个 reference、`docs/phoenix-diagram.md`、README、documentation-writer，elintp 的 SKILL.md 与 draft-format.md，以及 bundle 同步。
- 实际运行（引擎均为被审版本的开发构建 `local/phoenix-diagram/elintp-p3-final/phoenix-diagram-darwin-arm64`；其 `source-inputs.json` 列出的 191 个文件的 SHA-256 与当前树逐一相同）：

  | 检查 | 命令或步骤 | 结果 |
  | --- | --- | --- |
  | 主路径：取静态图 | `static workflow minimal.workflow.json flow.{embedded,system}.svg --font-policy …` | 两份都退出 0；三阶段全部 pass，`required-pending` / `incomplete`；各两张 `NAME.static.1440x900.{light,dark}.png`；136,280 / 46,160 字节，对应 Reader 971,381 字节（14.0% / 4.8%） |
  | 输入错误 | 输出 `.png`；`--font-policy serif` | 都退出 2，不写文件 |
  | 看图 | 打开 system dark 与 embedded light 两张截图 | 节点、关系、标签与图例完整，亮暗可辨，没有裁切 |
  | 宿主主题、多图隔离 | 同一页放两种策略的静态图和一个普通 SVG，分别设 `data-theme` 为空 / dark / light，各在 `--force-dark-mode` 开与关时测计算样式 | 无属性时跟随系统偏好；显式属性优先于系统偏好；system 图用 D4 字体栈，embedded 图用 JetBrains；普通 SVG 不受影响 |
  | 显式暗色打印 | `--print-to-pdf` 后转 PNG 查看 | 两图都按亮色完整打印 |
  | 审核登记 | `review-record` 只列一张截图；改动 SVG 后登记；完整登记 | 前两者被拒（`review/record`；`review/artifact-changed`「…run static again」）；完整登记后 `passed` / `complete` |
  | elintp 兼容 | 22 份真实产物 + 5 类重新生成的产物，逐份经 `render.mjs` | 见 F-01 |
  | elintp 安全 | 伪造 SVG 加双重编码字符引用，经 `render.mjs`，再用 Chrome 测计算样式 | 见 F-02 |
  | elintp 窄屏 | 用 CDP `Emulation.setDeviceMetricsOverride` 设 360×800，打开内联 architecture 静态图的页面（该图在已补 F-01 两项的临时解析器副本下生成；page.css 未改） | `innerWidth=scrollWidth=360`，页面没有横向滚动；`.image-scroll` 320/360 在区域内滚动；截图里正文没有裁切 |
  | 局部测试 | `mise exec --no-deps -- node --test test/elintp/render.test.js` | 13/13 通过（与 F-01、F-02 同时成立：用例是手写 fixture，没有覆盖这两条路径） |
  | 文档复述 | V-5 第 4 步的 `git grep`；`diff -rq skills/phoenix-diagram dist/phoenix-diagram/bundle` | 剩下的旧数量只出现在有意保留的旧 summary 兼容句里；bundle 与源一致（只差 gitignore 的 ENGINE.json 和 manifest） |

- 引用的现有证据（本轮未重跑）：`npm run test:diagram` 全量 Rust/Node/Reader 通过，以及十一类 Reader HTML 与基线 SHA-256 逐一相同，均见 [M5 记录](ELINTP-VISUALS-P3.records/M5.md#v-5-全量回归)。本轮的改动判定没有依赖这两项。
- 未覆盖：`viewer.css` 的 signal-flow、blueprint、editorial 三种预设没有逐一看静态图（计划 §11 已列）；SPMS 门户阅读面（A-1，范围外）。审核由当前 agent 独立完成，没有并行 reviewer。

## 计划符合性

计划版本：`docs/plan/ELINTP-VISUALS-P3.md`，当前 SHA-256 `4cceadcdc16ba1cb106cd212dc8e561aa404f284019fc67e4d8de95cafd676f9`；范围 M1–M5。

| 需求/检查 | 实现证据 | 状态 |
| --- | --- | --- |
| WORK-F9/AC-1–AC-4 | `static` 命令、两种策略、体积、校验失败不写 SVG、Reader 与 summary 互不影响 | 已核实（本轮实测 workflow；十一类见 M5 记录） |
| WORK-F10/AC-1–AC-5 | 截图 2/6、10 条覆盖不变、登记规则复用 | 已核实（本轮实测单图登记；比较部分见 M2/M3 记录） |
| WORK-F5/AC-2、C-1 | elintp `static:` | **存在缺陷**：F-01（四类图不能内联）、F-02（安全边界可被绕过） |
| WORK-N3/AC-3 | Reader 字节不变 | 已核实（引用 M5 记录；Reader 分支零改动，本轮读 diff 确认） |
| C-4、C-7 | 复述与 bundle | 已核实；另见 F-03 |
| §5.3 路线 | 计划要求在 TypeRenderer 生成点引入 `RenderContext`，「不在输出后删属性」；实现改为把 Reader SVG 后处理成静态 SVG（标记级解析加属性/元素清理），六类渲染器没有改动 | 偏差未就地修订正文（违反 `@implementation_route`）。本轮实测产物合约成立，没有据此另报缺陷；后果是 Reader 新增交互属性后要同步维护 `static_style.rs` 的清单，建议在计划正文登记现行路线 |
| V-4 引擎部分、V-2 第 6 步 | 计划要求在 `tests/delivery.rs` 用 CLI 观察样式作用域、策略互不覆盖、错误策略退出 2，并断言两种恢复信息 | **未实施**：cargo 测试里没有任何一条调用 `static`，M1 记录仍写「V-4 引擎测试通过」。命令级证据存在，所以不单列缺陷；修 F-01 时补的 22 份产物 E2E 可以承担其中大部分 |
| V-3 窄屏证据 | M4 记录的 360px 截图由 headless `--window-size` 生成，实际视口是 500px，记录里的"窄屏正文裁切"属于测量假象 | 本轮用真实 360px 视口重测：正文没有裁切，图在自身区域滚动 |
| `delivery/review.rs` HTML 提示 | 计划要求「原 finalize 的 HTML 提示保持不变」；实际改成了 "The artifact differs from the finalized one" | 轻微措辞偏差，恢复命令仍是 finalize，不构成缺陷 |

## 总体结论与修复顺序

**验收判定：需要修改**

`patch is incorrect`：引擎侧的 `static` 命令、截图减量与审核登记在被审版本上实测正确；但 elintp `static:` 拒收 architecture、erd、lifecycle、statechart 四类图的真实引擎产物（F-01），使本期 WORK-F5/AC-2 对这四类图不成立。同时，静态 CSS 的安全边界能被双重编码的字符引用绕过（F-02）。

修复顺序：

1. F-01：按实际引擎产物补齐 elintp 白名单，并加入 22 份真实产物经 `render.mjs` 的 E2E。
2. F-02：拒绝或转义 CSS 中的 `&`，并在攻击表补一条用例。
3. F-03：删除 SKILL.md 中重复的那一节，然后同步 bundle。

解除阻塞的判据：十一类 × 两种策略的 `static` 产物全部能被 `render.mjs` 内联；F-02 的复现输入被拒绝（退出 2、旧页字节不变）。本结论只覆盖上述提交，不是合并或发布授权。
