# ELINTP-VISUALS-P3 代码评审 · R2（修复效果复审）

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R2 | [ELINTP-VISUALS-P3 · 静态图与看图审核减量](ELINTP-VISUALS-P3.md) | 复审：R1 目标 `bfd626b55499f96a7a489cb85d1d0717dc692eed` → 修复提交 `cdc6ea52222d767ca68fcccaa66f2c3475535454`（HEAD `29d692703cfca0e81aa9f7cd5d171ec517f10926` 为记录提交）；工作树未提交项仅台账与 R1 报告工件（见覆盖） | [ELINTP-VISUALS-P3.code-review.md](ELINTP-VISUALS-P3.code-review.md)（R1，SHA-256 `a01359c2d756ed34632ac5a4290a2bacc7aa87ea5a48ea39a4ae4ebc31f6aeac`） |

**复审记录**：上轮报告为 R1（初审），所审对象为 `63ec4961^1..bfd626b5` 净差异、当时工作树 clean。本轮为**修复效果复审**：修复差异取 `bfd626b5..cdc6ea52`（代码：`skills/elintp/scripts/draft.mjs`、`test/elintp/render.test.js`、`skills/phoenix-diagram/SKILL.md`、`dist/phoenix-diagram/bundle/{SKILL.md,manifest.json}`；其余为计划与记录回写），逐项复核 R1 全部问题并检查修复引入的新缺陷。已有处置记录：[M4 评审修复](ELINTP-VISUALS-P3.records/M4.md#评审修复)（F-01/F-02/窄屏）、[M3 评审修复](ELINTP-VISUALS-P3.records/M3.md#评审修复)（F-03）、[M1 计划符合性处置](ELINTP-VISUALS-P3.records/M1.md#评审处置计划符合性)、[M2 恢复信息补证](ELINTP-VISUALS-P3.records/M2.md#评审处置恢复信息补证)、[M5 修复后收口](ELINTP-VISUALS-P3.records/M5.md#评审修复后的收口证据)。

## 上轮问题复审

| 来源报告 / F-ID或稳定章节 | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R1 / F-01 [P1] | elintp `static:` 拒收 architecture、erd、lifecycle、statechart 四类图的真实引擎产物（属性白名单缺 `pointer-events`；兄弟组合器一律拒绝） | [M4 评审修复](ELINTP-VISUALS-P3.records/M4.md#评审修复)；`cdc6ea52` | **已解决** | 代码核对：`STATIC_SVG_ATTRIBUTES` 补入 `pointer-events` 且取值仅 `none`；`selectorChain` 改为收集 `+`/`~`，`validateSelectors` 仅在根复合选择器及其之前（`combinators.slice(0, rootIndex + 1)`）拒绝兄弟组合器，根内部绘图层级放行。本轮在当前版本实测：`local/phoenix-diagram/elintp-p3-final-out/` 全部 22 份真实产物（11 类 × 两策略）逐件经公开入口 `render.mjs` 内联，22/22 退出 0、内联标记存在、无 iframe、system 无字体资源；根上/根前 `+`、`~` 拒绝用例仍在（18/18 通过） | 无 |
| R1 / F-02 [P2] | 静态 CSS 经双重编码字符引用绕过作用域与离线检查（校验后原样拼入页面，HTML 再解码一次） | [M4 评审修复](ELINTP-VISUALS-P3.records/M4.md#评审修复)；`cdc6ea52` | **已解决** | 代码核对：`parseStaticCSS` 先剥离注释（未闭合注释拒绝、引号内不误剥），再对 `cleaned` 全量拒绝 `&`；输出规则全部切片自同一 `cleaned`，属性与文本另走 `escapeHTML`，`<`/`\`/NUL 早已拒绝。本轮实测：R1 原始攻击载荷退出 2、旧页字节不变；三重编码变体同样退出 2；良性 `&amp;` 注释产物退出 0（注释剥离后不残留）；新增双重编码攻击用例在套件中通过 | 无 |
| R1 / F-03 [P3] | phoenix-diagram SKILL.md 正文重复 static-figure.md 完整流程 | [M3 评审修复](ELINTP-VISUALS-P3.records/M3.md#评审修复)；`cdc6ea52` | **已解决** | 「可内联的静态图」一节已从 `skills/phoenix-diagram/SKILL.md` 与 bundle 副本删除；「按请求启用」入口（读 `references/static-figure.md`）保留；bundle `manifest.json` 中 SKILL.md 的 SHA-256/size 与实际文件一致（本轮计算核对）；`test/phoenix-diagram/skill.test.js` 6/6 通过 | 无 |
| R1 计划符合性 / §5.3 路线 | 实现为标记级转换（`SvgConversion` 后处理 Reader SVG），计划正文仍写生成点引入 `RenderContext`，偏差未就地修订 | 计划已修订；M1 记录登记处置；`cdc6ea52` | **已解决** | 计划 ADR-1、D5、§1.2、§2、§5.2、§5.3、§5.6 已改为现行路线并保留全部验收门槛（本轮读 diff 核对）；[M1 记录](ELINTP-VISUALS-P3.records/M1.md#评审处置计划符合性)如实登记代价（Reader 新增交互属性须维护转换清单） | 无 |
| R1 计划符合性 / V-4 引擎部分 | 计划要求 `tests/delivery.rs` 的静态 CLI 用例未实现，M1 记录却写「V-4 引擎测试通过」 | M1 记录已更正为如实表述，并以公开 CLI 证据补足 | **已解决**（记录失实已更正） | [M1 记录](ELINTP-VISUALS-P3.records/M1.md#评审处置计划符合性)现写明该结果不含 `static` CLI 用例；错误字体策略退出 2、22 件真实产物内联等命令级证据补足。Rust 自动用例仍未新增，R1 已判定命令级证据充分、不构成缺陷，维持该判断并列为后续范围 | 无 |
| R1 计划符合性 / V-3 窄屏证据 | M4 记录的 360px 截图实际视口为 500px，「正文裁切」属测量假象 | M4 记录更正并以真实 360px 重测 | **已解决** | [M4 记录](ELINTP-VISUALS-P3.records/M4.md#v-3-看图验收2026-11-12t2208-0800-补齐)：CDP 实设 360×800，`innerWidth=scrollWidth=360`，正文无横向溢出、图在自身区域滚动；R1 评审时亦以真实 360px 视口实测一致 | 无 |
| R1 计划符合性 / `delivery/review.rs` HTML 提示 | 计划要求「原 finalize 的 HTML 提示保持不变」，实际措辞改为 artifact | 计划 §5.4 已按实际行为修订（保留 finalize 恢复语义） | **不再适用** | 恢复命令仍为 `finalize`，行为无变化；[M2 记录](ELINTP-VISUALS-P3.records/M2.md#评审处置恢复信息补证)以真实 `review-record` CLI 四路径验证 `static`/`finalize` 分派与诊断码；R1 已判不构成缺陷 | 无 |

旧问题解决情况：**3/3 已解决，1 项不再适用（计划措辞对齐，非缺陷项）**。

## 确认问题

未发现可确认的问题。修复增量（`bfd626b5..cdc6ea52` 代码部分）经静态审查与实测未发现新引入或新激活的缺陷：选择器链索引语义正确（`combinators[i]` 连接 `parts[i]` 与 `parts[i+1]`，根锚点前的兄弟组合器仍拒、根内放行）；`&` 拒绝在注释剥离之后执行且输出全部来自同一 `cleaned` 字符串，引号内、注释内均无残留通道；`pointer-events` 取值门限与 CSS 侧约束一致；`render.test.js` 新增用例断言可观察行为（退出码、旧页字节、产物标记）。

## 待核实/待决策项

无。本轮结论所依赖的证据均在评审边界内自行取得（命令与结果见下节）。

## 覆盖与验证

- 仓库：`/Users/rockie/Documents/gh-xgent/skills`，评审日期 2026-11-13。原输入 `docs/plan/ELINTP-VISUALS-P3.md`（当前 SHA-256 `2fdb14f89ba17d8a4cdd16dfb235476c8b0fd8500f903f2d398adb85d50f9101`），plan 模式触发复审；上轮报告取轮次最大者 R1。
- 范围解析：登记提交 `63ec4961`/`bfd626b5` 与修复 `cdc6ea52` 均在当前分支（完整 SHA 见上表）；`2fb94fb6`/`6ca033cd`/`f4e83113`/`29d69270` 只改计划与记录，为进度回写，不作为代码对象。`bfd626b5..HEAD` 期间 `tools/` 无任何改动，引擎结论沿用 R1 对同一引擎版本的实测。工作树未提交项：`.xgent-ai/sdlc/status/plan/fe520dbf…`（M）、`status/review/index.md`（M）、`status/review/cc272fa5…`（新增，R1 台账页）与未跟踪的 R1 报告——均为台账/报告工件，非代码 findings 对象；`tools/ skills/ test/` 无未提交改动，index 为空。
- 引擎对应性（复核）：复用产物由开发引擎 `local/phoenix-diagram/elintp-p3-final` 生成，其 `source-inputs.json` 191 个输入本轮逐项重算 SHA-256 与 size，全部与当前树一致。
- 本轮实际执行（全部在当前版本 HEAD `29d69270` + 干净的代码路径上）：

  | 检查 | 命令或步骤 | 结果 |
  | --- | --- | --- |
  | elintp 回归 | `mise exec --no-deps -- node --test test/elintp/render.test.js test/elintp/review.test.js` | 18/18 通过 |
  | skill 元数据 | `mise exec --no-deps -- node --test test/phoenix-diagram/skill.test.js` | 6/6 通过（0 失败） |
  | F-01 主路径 | 22 份真实产物（`elintp-p3-final-out/*.svg`）逐件写最小 general 内容稿，`static:` 绝对路径引用，运行 `node skills/elintp/scripts/render.mjs` | 22/22 退出 0；内联标记、无 iframe、system 无 `@font-face`/`data:font`/JetBrains 全部满足 |
  | F-02 攻击复现 | R1 原始双重编码载荷与三重编码变体经 `render.mjs`；良性 `&amp;` 注释产物同入口 | 攻击退出 2（`CSS 不允许残留字符引用或 &`）且旧页字节不变；三重编码退出 2；良性注释退出 0 |
  | F-03 与 C-7 | 计算实际 SKILL.md SHA-256/size 对照 bundle `manifest.json`；`diff -rq skills/phoenix-diagram dist/phoenix-diagram/bundle` | 一致；bundle 差异仅 gitignore 的 `.DS_Store`、`ENGINE.json` 与 bundle 侧 `manifest.json`，符合既有约定 |
  | 修复 diff 审查 | 通读 `bfd626b5..cdc6ea52` 全部代码 diff 与 `draft.mjs` 相关函数全文 | 无新缺陷（见「确认问题」） |

- 引用的现有证据（本轮未重跑，条件未变）：R1 对同一引擎版本的 `static` 命令、截图减量、审核登记、宿主主题/打印与多图隔离实测；M5 记录的全量回归、十一类 Reader SHA 比对、44 张看图与旧四张登记、V-6 安装验收。引擎源码本轮零改动，上述证据适用性不变。
- 未覆盖（沿用 R1，均不阻塞本轮结论）：`viewer.css` 的 signal-flow、blueprint、editorial 三种预设未逐一看静态图（计划 §11 已列开放风险）；SPMS 门户阅读面（A-1，范围外）。本轮由当前 agent 独立完成，无并行 reviewer。

## 计划符合性

计划版本：`docs/plan/ELINTP-VISUALS-P3.md`（SHA-256 见覆盖节），5/5 里程碑完成，修复已回写并提交。

| 需求/检查 | 实现证据 | 状态 |
| --- | --- | --- |
| WORK-F5/AC-2、C-1（elintp 内联） | `static:` 来源 + 本轮 22/22 真实产物经公开入口内联 | 已核实（R1 缺口 F-01/F-02 关闭） |
| WORK-F9、WORK-F10、WORK-N3/AC-3 | R1 已实测引擎侧；本轮确认引擎零改动、191 输入与树一致 | 已核实（证据沿用） |
| C-4、C-7（复述与 bundle） | F-03 修复后 bundle 与 manifest 一致；skill.test.js 通过 | 已核实 |
| V-4 elintp 部分 | `render.test.js` 18/18（含修复新增攻击与放行用例） | 已核实 |
| V-4 引擎部分（`tests/delivery.rs` 静态 CLI 用例） | 未新增 Rust 自动用例；以公开 CLI 命令级证据补足并如实登记 | 后续范围（R1 已判不构成缺陷，维持） |

## 总体结论与修复顺序

**验收判定：通过验收**

修复结论：**已解决**（F-01、F-02、F-03 全部关闭，均符合原问题关闭判据；两项计划符合性缺口同步解决、一项不再适用）。修复增量结论：`patch is correct`（`bfd626b5..cdc6ea52` 代码部分未发现新缺陷；该结论只覆盖此增量，旧问题关闭由复审表独立支撑）。

R1 的三项问题在当前版本全部经真实入口复验：十一类 × 两策略的引擎产物可被 elintp 公开渲染入口完整内联，双重编码攻击被拒绝且失败不覆盖旧页，重复文档已删除并同步 bundle。无未决 P0–P3 问题。

覆盖限制：三种预设样式的静态图与门户阅读面（A-1）仍按计划 §11 保持开放，不改变本期 AC 判定。本结论只覆盖上述提交与工作树，不是合并、发布或需求验收授权。
