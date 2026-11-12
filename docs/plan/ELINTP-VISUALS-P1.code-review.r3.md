# ELINTP-VISUALS-P1 · R3 修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R3 | [ELINTP-VISUALS-P1](ELINTP-VISUALS-P1.md)，SHA-256 `d8037d7ef55bc984b8c6a81fdc0a1724c106cc5831e71cd3f18bb99b4f2d9cce` | `fd55e6485f2407ba8eec464a32fca601dfc0a879..43b934f7f18cbae5415cc6e015bacc1c09917de1`；另核对未提交快照，指纹 `f070b9f132f17bf9bb311c441dc269278958655f5e1479108707b609f5489213` | [R2](ELINTP-VISUALS-P1.code-review.r2.md)，SHA-256 `0eabc3d6778d9bae659f169e02210353783161d32d31ab219fdd6f9fe4689761` |

模式：**修复效果复审**。上轮代码目标为 `fd55e648`，本轮当前 HEAD 为 `43b934f7`，最后一个产品代码提交为 `b25842ce28f70a8be17aff59a6ae230f26e3ba4f`。本轮核对 R2 的全部旧项，并审查其后 UI 修复 `873ecb3bec405a1dcb50a821b4a639cde4d4fc00`、`256c43a3030024a3a3388b2d00c974702151736c`、`b25842ce28f70a8be17aff59a6ae230f26e3ba4f`；不宣称重新完整审查最初实现。

已有处置与证据：[M4 修复记录](ELINTP-VISUALS-P1.records/M4.md#ui-验收修复)、[R3 UI 报告](ELINTP-VISUALS-P1.ui-review.r3.md)、[R3 直接修复取证](ELINTP-VISUALS-P1.ui-captures/r3-fix/capture.md)。历史报告及其结论保持不变。

## 1. 确认问题

未发现可确认的新增问题。

### 上轮问题复审

| 来源报告 / F-ID或稳定章节 | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R2 / 上轮问题复审（R1 / F-01；review `88e75755-1262-4e9c-8692-6680cefa9cda`） | 多张内联 SVG 的同名 ID 串引，P2；R2 已解决 | `b9d0807d`；本轮核对当前 `b25842ce` | **已解决** | 当前 [draft.mjs](../../skills/elintp/scripts/draft.mjs) 仍按图给 ID、href/xlink:href、URL 与 ARIA 引用加同一前缀；当前公开 CLI 的 5 项测试全部通过，含双图同名 ID 与确定性用例。后续 draft.mjs 增量只触及图片/阅读器分支，没有改写 `inlineSVG` | 无 |
| UI R3 / UI-03 | 同一正文位置的当前大纲项受点击/滚动路径影响，P2；部分解决 | R3 直接修复；`b25842ce` | **已解决** | 当前 [page.js](../../skills/elintp/assets/page.js) 只按 160px 参考线与实际滚动位置标记。本轮 Chrome 中 U3 在 1440/360 下点击末项再离开并回到相同 `scrollY`，当前项均为 `#s2`；U2-done 的同类路径均为 `#s1`，实际截图与 DOM 结果一致 | 独立 UI 复审可由用户另行发起 |
| UI R3 / UI-05 | 禁用脚本时嵌入阅读器仍显示主题按钮，P2；仍存在 | R3 直接修复；`b25842ce` | **已解决** | 当前 [draft.mjs](../../skills/elintp/scripts/draft.mjs) 在嵌入副本的 head 中加入 `noscript` 规则，源阅读器不变。本轮真实 file 页面在禁用脚本时宿主与阅读器按钮均隐藏；启用脚本时两者可见，阅读器主题实际从 light 切到 dark，图与说明仍显示 | 独立 UI 复审可由用户另行发起 |

R2 没有其他确认问题或阻塞性待核实项。其“未重新执行完整 UI、V-1/V-2/V-4/V-5”的文字是上轮覆盖边界，不是额外缺陷；本轮只补查本次改变及直接依赖，不据此声称上述整套验收已重新运行。

## 2. 待核实 / 待决策项

无影响本轮代码结论的待核实或待决策项。R3 直接修复后的独立 UI 复审尚未进行，本报告不代替 UI 评分或产品终验；当前代码主路径已有本轮真实入口证据，因此不构成代码验收缺证。

## 3. 覆盖与验证

一句话意图：把固定模板页面的打印、宽截图、大纲和无脚本嵌入行为修到计划约定，同时保持 SVG 隔离、离线单文件与确定性输出。用户显式输入仅为本计划路径；行为合同来自计划、R2 代码报告及已有 UI 处置记录。

### 冻结范围与文件

- 仓库根：`/Users/rockie/Documents/gh-xgent/skills`；评审日期：2026-11-11；主 Agent 独立评审，未委派 reviewer。
- 原输入：`$review-code docs/plan/ELINTP-VISUALS-P1.md`；按最高代码报告 R2 进入修复效果复审。
- 完整读取 `fd55e648..43b934f7` 的产品 diff，并追踪当前 `page.css`、`page.js`、`draft.mjs`、`draft-format.md`、公开 CLI 测试及实际阅读器模板。审查文件为 `skills/elintp/assets/page.css`、`assets/page.js`、`scripts/draft.mjs`、`references/draft-format.md`。
- `b90edbb7`、`c1271161`、`43b934f7` 只回写计划/记录，不作为代码 finding 对象；没有未登记的实现提交。
- index 为空。未提交快照枚举到 8 个 tracked 修改与 52 个 untracked 文件，共 60 个；没有 `skills/elintp/` 或 `test/elintp/` 的未提交改动。其他既有文档、经验、状态页、历史评审与 UI 取证未作为本轮产品改动审查，也未把本轮新报告计入输入指纹。
- 指纹算法：对 `git ls-files -m -o --exclude-standard -z` 的路径去重排序，逐文件计算内容 SHA-256，再对 `[路径, hash]` 数组的紧凑 JSON 计算 SHA-256。未提交内容只在原工作树可取回，hash 不代替内容快照。
- 关键当前内容 SHA-256：`page.css` `624e0955b456a4065ace3a9663325a62df31855a085b5b4b3ba045605ade5091`，`page.js` `0289ad221e209938e8751e0a119b842bc5f5c71413ecc8214e32e0da825e3ae9`，`draft.mjs` `a23c24d3217f81bc0a4a0a3834d34aa1e0be8ab0b92a39ead8fbd1223ac95bef`，`render.test.js` `4bdfe7c488be7a8907c9a8651024973b149781969e6cbe80ae372a100f831aed`。HEAD 与计划、R2、M4 指纹在落盘前复核一致。

### 行为链与反证核对

- **SVG**：源 SVG 白名单/实体解码 → 每图 ID 与本地引用前缀 → 整页 HTML。不同图前缀不共享，同一图内映射保持一一对应；当前测试证明双图的图形、渐变、标记、裁切和 ARIA 引用留在本图。
- **大纲**：原生锚点跳转/滚轮 → scroll 事件 → 各节相对 160px 参考线的位置 → 唯一 `aria-current`。重点检查页尾短节、同位置不同路径、直接锚点及页面横向溢出；未把“点击目标”当作当前位置的替代证据。
- **无脚本阅读器**：读取已交付 HTML → 只修改嵌入副本 → 转义进 `iframe srcdoc` → 禁用/启用脚本两种浏览器上下文。实际 Phoenix 阅读器模板存在明确 `</head>`，注入点可达；源文件与 Phoenix 工具未改。
- **打印与图片**：`beforeprint` 测量阅读器 → 打印 CSS 以 840px 排版并缩放 → 实际 PDF；图片则由 `image-scroll` 与窄屏/打印 CSS 控制。另以尚未滚动到阅读器的冷启动页面补查 lazy iframe 与打印测量，没有发现空文档或内容裁切。

### 实际验证

| 类型 | 实际命令 / 操作 | 当前版本、结果与限制 |
| --- | --- | --- |
| CLI 回归 | `node --test test/elintp/render.test.js` | HEAD `43b934f7`，Node `v25.2.1`；5 pass、0 fail。覆盖 F-01、格式/安全、旧页保护与确定性。 |
| 公开渲染入口 | 从仓库根用当前 `skills/elintp/scripts/render.mjs` 将既有 U1、U2、U2-done、U3 内容稿生成到 `/tmp/elintp-review-r3-9ZJKwv/` | 四次均 exit 0、输出 `PAGE:`、无 WARN；产物为真实本地 file 页面，没有请求拦截或 DOM 补丁。 |
| Chrome 大纲 | Playwright 驱动 Chrome `157.0.8103.39`，四页 × 1440×900/360×800；末项键盘激活后上滚再回滚，记录 `scrollY`、当前项与页面宽度，并实际查看 U3 截图 | U3 同位置均为 `#s2`，U2-done 均为 `#s1`；U1/U2 的长页邻近路径无异常。8 条件均 `scrollWidth === innerWidth`，0 pageerror。桌面截图实际可见第二项高亮；窄屏以 DOM 度量为准。 |
| Chrome 无脚本/主题 | 新上下文分别以 `javaScriptEnabled=false/true` 打开当前 U1，滚动到真实阅读器并查看/操作 | 禁用时两个主题按钮均隐藏；启用时均可见，阅读器实际 light→dark，0 pageerror。截图实际确认图、图例和说明保留。 |
| 冷启动打印 | 当前 U1 打开后保持 `scrollY=0`，不先滚动到阅读器，直接 Chrome `page.pdf`；`pdfinfo`、`pdftoppm` 后查看末页 | 无控制台/page error；打印前 iframe document 可访问，测得阅读器内容高 843、打印变量 788。PDF 共 4 页，末页完整显示六节点、图例、四张说明卡、图注与出处。该次为默认 Letter；A4 前后一致性另有 R3 修复记录。 |
| 既有独立 UI 证据 | [R3 UI 报告](ELINTP-VISUALS-P1.ui-review.r3.md) 与 [R3 修复取证](ELINTP-VISUALS-P1.ui-captures/r3-fix/capture.md) | 前者覆盖至 `256c43a3` 的独立 UI 结论；后者是当前 `b25842ce` 的修复方证据。本轮只复用未受影响行为，不把修复方记录称作独立 UI 复审。 |

未重跑完整 V-1/V-2/V-4/V-5、`npm pack`、全仓测试、Phoenix 运行时或完整 WCAG 检查；它们不直接覆盖本轮新增行为，或已有未受影响版本的记录。未修改实现、测试、计划或历史报告，未提交、发布或访问真实业务数据。

## 4. 计划符合性

| 承诺 | 本轮核对 | 判定 |
| --- | --- | --- |
| §5.4 / M2：多图 SVG 的 ID 与本地引用隔离 | 当前实现与 R2 关闭根因一致；5 项公开 CLI 回归通过 | 已核实 |
| §6.1–§6.2 / UI-01、UI-02：阅读器完整打印、打印固定亮色 | 当前冷启动实际 PDF 完整；暗色媒体仍限定 `screen`，打印变量为亮色。暗色 A4 的独立历史证据未被本轮改动失效 | 已核实到声明范围 |
| §6.1 / UI-04：窄屏宽截图保持原尺寸并在自身区域滚动 | 当前 wrapper、焦点属性与 CSS 保持该实现；后续未改图片分支或相关 CSS | 已核实，复用未受影响证据 |
| §6.2 / UI-03：当前项只随实际滚动位置判定 | 特殊页尾/哈希覆盖已删除；本轮桌面与窄屏同位置路径结果一致 | 已核实 |
| §6.2 / UI-05：无脚本控件不干扰静态阅读 | 嵌入副本以 `noscript` 隐藏阅读器主题按钮；启用脚本时独立主题交互保留 | 已核实 |
| §6.3 / 格式参考同步 | `draft-format.md` 已说明嵌入副本的无脚本规则及启用脚本后的独立主题；SKILL.md/frontmatter 未改 | 已核实 |
| P2–P5、跨期需求终验、发布 | 不属于本轮修复范围 | 后续范围 |

## 5. 总体结论与修复顺序

**验收判定：通过验收。**

修复结论：**已解决**。R2 覆盖的旧代码问题在当前版本保持已解决；UI-03、UI-05 均达到各自关闭判据。本轮选定旧项实际解决/保持解决 3 项，不再适用 0 项，未关闭 0 项。修复增量 `patch is correct`，未发现新增缺陷；公开 CLI、当前真实 file 页面、Chrome 操作和实际 PDF 支撑该结论。

本结论仅覆盖声明的复审范围，不是合并、发布、完整 UI 评分或 PRD 终验授权。无需再执行 apply。可选下一步：先由用户发起独立 UI 复审；计划来自 PRD，也可另行核查 PRD 与开发交付差异。

```text
$review-ui docs/plan/ELINTP-VISUALS-P1.md
$review-prd-dev-gaps docs/plan/ELINTP-VISUALS-P1.md
```
