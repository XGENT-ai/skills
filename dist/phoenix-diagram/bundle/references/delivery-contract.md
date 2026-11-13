# 交付与实际审核

读本文件处理 finalize、static、诊断、实际图像审核、review-record 或环境失败。用户明确请求双输入架构比较时，使用[架构比较合同](architecture-compare.md)的三视图与六图审核；本文件下方的单图三门与两图记录保持不变。命令使用 `SKILL.md` 中已解析的 `PD`，从用户工作目录执行。

## 三门与 summary

```sh
"$PD" finalize <type> candidate.json diagram.html --json
"$PD" finalize <type> candidate.json diagram.html --repo-root "<仓库根>" --json
```

带 `sources` 或 `meta.repository` 的图用第二种，单图的 `--repo-root` 接受 `validate`、`deliver`、`finalize`；双输入架构比较另由 `compare --repo-root` 处理。来源核对独立于自动门：回执与 summary 的 `evidence.status` 为 `verified`、`partial`、`unverified` 或 `none`，校验与交付回执另列 `evidence.unverified`（pointer、元素、原因）与 `evidence.unsourced`。结构错误（凭据 URL、路径逃逸、非法行区间、provider 与主机不符、网页链接用了不支持的主机）在 input 阶段失败且不生成 HTML。

固定顺序为 `validate → deliver → browser-check`。首个非 pass 阶段后其余阶段保持 `not-run`。`validate` 做输入/布局与 artifact 检查；`deliver` 写出本次 HTML；`browser-check` 检查浏览器实测并采集截图。读取每个阶段真实 status，不能补造通过。

stdout 是单一 JSON，同时写入 HTML 旁唯一的 summary `<名称>.finalize-summary.json`（路径见 `evidence.summaryPath`）。summary 记录三门回执、`candidate` / `artifact` 的 `path` 与 `identity:{sha256,bytes}`、`configSha256`、`engine`、`screenshots` 与审核状态；`locale` 给出请求、解析后的界面语言与是否回退英文。自动三门通过后，`visualReview.status` 仍为 `required-pending`，`deliveryStatus` 为 `incomplete`。

失败时使用 summary 的 diagnostics。validate/deliver 未过时，输出路径可能仍是旧文件，不对它做视觉验收。修复 candidate 后重新 finalize；重跑会覆盖同名 HTML、截图与 summary，旧审核随之作废。仅为取完整状态或登记看图结果不得再次 finalize。

## 浏览器与实际看图

自动检查仍覆盖 1440×900、1600×1000、1920×1080、2048×1320 桌面与窄屏；实际审核只使用 summary `screenshots` 列出的两张 1440×900 截图，light/dark 各一张，默认在 HTML 旁，`--out-dir` 可指定目录。打开每张图片，检查主图、全部标签/notes、卡片、关系遮挡、层级和控件。截图存在、DOM 数值或测试输出都不代表已经观察。

每个桌面尺寸必须无水平溢出。纵向滚动仅按 Reader 声明和测量接受：`intrinsic-height` / `width-first` 的自动画布，或 Architecture 显式 viewBox 的 `authored-height`，需完整内容留在 panel 内，无内部滚动或裁剪，页面允许纵向滚动，且回执确认可读并给出 `verticalScrollAccepted: true` / `overflowDisposition: "readable-vertical-scroll"`。未知声明、水平溢出、不可读文字和 chrome 遮挡仍失败。不能手工改 DOM 属性伪造该例外。

定向诊断可用下列公开命令；它们只输出 stdout 回执，不改 summary，也不替代 finalize：

```sh
"$PD" validate <type> candidate.json --quality showcase --layout-json --json
"$PD" check diagram.html --json
"$PD" browser-check diagram.html --json
"$PD" visual-check diagram.html --out-dir evidence-2 --json
```

环境 `unavailable`、执行或断言 `failed`、后续 `not-run` 与未实际看图必须分别报告。自动门失败先修该门；没有图像能力时不得从测量推测 `passed`。不自动安装引擎或浏览器，也不打开用户现有浏览器、启动 preview。

## 审核记录

实际打开两张图片后写审核记录（JSON），`screenshots` 逐项复制 summary `screenshots` 中的路径，每张一次：

```json
{
  "reviewer": "<真实审核者>",
  "verdict": "passed",
  "screenshots": [
    { "path": "<summary.screenshots[0]>", "observations": "<这张图里实际看到的主图、标签、卡片、关系遮挡、层级与控件>" }
  ],
  "findings": []
}
```

无缺陷才能用 `verdict: "passed"`，且每张图都要有非空 `observations`；有缺陷用 `failed` 并在 `findings` 列具体问题；确实没有图像工具或审核者时用 `unavailable`，在 `findings` 写实际限制，保持未完成。不要把观察复制到未打开的截图，也不要加未知字段。

```sh
"$PD" review-record diagram.html --record review-01.json --json
```

review-record 只接受自动三门已通过、产物仍与 summary 记录一致、且记录逐一列出当前 summary 全部截图的交付，把审核写回同一 summary（返回的 `summary` 为其路径）。少列、多列或使用另一份 summary 的截图路径都不能登记。已有 summary 不迁移：旧 summary 若列四张截图，仍须实际查看并逐一登记这四张；新生成的 Reader summary 列两张。再读该 summary：三门全部 pass、`automatedStatus: "pass"`、`visualReview.status: "passed"`、`deliveryStatus: "complete"` 才算完成。仅凭 review-record 退出码或截图存在不能称完成。记录有误时改正后重跑 review-record；用户要求或实际缺陷导致改 candidate 时，重新执行 summary `command` 对应的 `finalize` 或 `static` 并重新看图审核。

## 架构比较的收口

仅明确的架构比较请求使用以下顺序，完整约束见[架构比较](architecture-compare.md)：

```sh
"$PD" compare architecture base.json head.json comparison.html --json
"$PD" visual-check comparison.html --out-dir compare-evidence-01 --json
# 实际观察比较 summary 中 screenshots 列出的六张图：三个视图各一张 1440×900 light、一张 dark，写审核记录。
"$PD" review-record comparison.html --record comparison-review-01.json --json
```

比较 summary 是 compare 写在 HTML 旁的 JSON（默认 `comparison.html.compare.json`），visual-check 写入浏览器结果与截图，review-record 写入审核。记录必须逐一列出比较 summary 的六张截图并分别填写实际观察，缺任一张不能通过。只有 `browserStatus: pass`、`visualReview.status: passed`、`deliveryStatus: complete` 才完成。不要用单图记录或重新生成成果代替。

## 静态图

明确要求可内联的静态 SVG 时使用：

```sh
"$PD" static <type> CANDIDATE OUTPUT.svg --json
"$PD" static <type> CANDIDATE OUTPUT.svg --font-policy system --json
```

`--font-policy embedded|system` 默认为 `embedded`。`embedded` 内嵌 JetBrains Mono；`system` 用于调用方明确要求系统字体的场景，不含嵌入字体、JetBrains Mono 或 `@font-face`，只使用 `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`。两种策略都要在实际输出字体下检查文字边界并看图；静态 SVG 的字节数须不超过同一 candidate Reader HTML 的 50%。

`static` 的顺序为 `validate → static → browser-check`，首个非 pass 阶段后的阶段保持 `not-run`。校验或静态渲染未过时不写或覆盖 SVG；浏览器不可用时已写出的 SVG 保留并如实记录 `unavailable`。stdout 是 `command: "static"` 的 JSON，同时写出：

- `OUTPUT.svg`：单个无脚本、无外部资源的静态 SVG，包含主图、关系、标签与图内图例，不包含 Reader 页面标题、节点索引或交互控件；卡片以 `<metadata data-phoenix-cards="">` JSON（每项 `dot`、`title`、`items`，顺序同 Reader）随 SVG 交付，不画进图面，由宿主显示。
- SVG 旁的 `NAME.static-summary.json`。
- `NAME.static.1440x900.light.png` 与 `NAME.static.1440x900.dark.png` 两张截图；`--out-dir` 可指定截图目录。

宿主 `<html data-theme="light">` 或 `data-theme="dark"` 优先决定主题；没有有效属性时跟随 `prefers-color-scheme`，打印为亮色。取静态图不读写同名 Reader HTML、`.finalize-summary.json` 或其截图。

实际打开 static summary 列出的两张 1440×900 亮、暗截图，逐张填写非空 `observations`，审核记录格式与单图 Reader 相同，随后运行：

```sh
"$PD" review-record OUTPUT.svg --record review.json --json
```

`review-record` 按 `.svg` 找到 `NAME.static-summary.json`，仍要求自动门已通过、SVG 身份未变、记录中的截图与 summary 一一对应。自动门未过时按诊断重跑 `static` 直到通过；SVG 已变化时重跑 `static`，再实际看图并登记。Reader HTML 的对应恢复命令仍是 `finalize`。只有三门 pass、`automatedStatus: "pass"`、`visualReview.status: "passed"`、`deliveryStatus: "complete"` 才完成。`static` 不存在或失败时如实报告真实阶段和保留文件，不能用 Reader 的审核记录代替静态图审核。

完整取图流程、字体策略选择与失败恢复见[静态图](static-figure.md)。
