# 交付与实际审核

读本文件处理 finalize、诊断、实际图像审核、review-record 或环境失败。用户明确请求双输入架构比较时，使用[架构比较合同](architecture-compare.md)的三视图与 12 图审核；本文件下方的单图三门与四图记录保持不变。命令使用 `SKILL.md` 中已解析的 `PD`，从用户工作目录执行。

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

自动检查覆盖 1440×900、1600×1000、1920×1080、2048×1320 桌面与窄屏；实际审核使用 summary `screenshots` 列出的四张截图：1440×900 与 2048×1320 各 light/dark，默认在 HTML 旁，`--out-dir` 可指定目录。打开每张图片，检查主图、全部标签/notes、卡片、关系遮挡、层级和控件。截图存在、DOM 数值或测试输出都不代表已经观察。

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

实际打开四张图片后写审核记录（JSON），`screenshots` 逐项复制 summary `screenshots` 中的路径，每张一次：

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

review-record 只接受自动三门已通过、且 HTML 仍与 summary 记录一致的交付，把审核写回同一 summary（返回的 `summary` 为其路径）。再读该 summary：三门全部 pass、`automatedStatus: "pass"`、`visualReview.status: "passed"`、`deliveryStatus: "complete"` 才算完成。仅凭 review-record 退出码或截图存在不能称完成。记录有误时改正后重跑 review-record；用户要求或实际缺陷导致改 candidate 时，重新 finalize 并重新看图审核。

## 架构比较的收口

仅明确的架构比较请求使用以下顺序，完整约束见[架构比较](architecture-compare.md)：

```sh
"$PD" compare architecture base.json head.json comparison.html --json
"$PD" visual-check comparison.html --out-dir compare-evidence-01 --json
# 实际观察比较 summary 中 screenshots 列出的 12 张图，写审核记录。
"$PD" review-record comparison.html --record comparison-review-01.json --json
```

比较 summary 是 compare 写在 HTML 旁的 JSON（默认 `comparison.html.compare.json`），visual-check 写入浏览器结果与截图，review-record 写入审核。只有 `browserStatus: pass`、`visualReview.status: passed`、`deliveryStatus: complete` 才完成。不要用单图记录或重新生成成果代替。
