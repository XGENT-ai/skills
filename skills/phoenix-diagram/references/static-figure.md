# 静态图（可内联 SVG）

仅在用户或调用方明确要求可内联的静态图时使用本流程；普通交付默认是独立 HTML Reader，不因普通作图自动另取静态图。

## 何时取静态图

- 页面要求零网络、无脚本的阅读面，或宿主要以 SVG 内联（例如 elintp 的 `static:` 来源）。
- 需要与 Reader HTML 并存的轻量图版本；两者相互独立，取静态图不读写同名 Reader HTML、其 finalize summary 或截图。

## 命令

```sh
"$PD" static <type> CANDIDATE OUTPUT.svg --json
"$PD" static <type> CANDIDATE OUTPUT.svg --font-policy system --json
```

- `--font-policy embedded|system` 默认 `embedded`（内嵌 JetBrains Mono WOFF2）。
- 调用方或作者明确要求系统字体时必须选 `system`：产物不含任何字体资源、JetBrains Mono 或 `@font-face`，只使用 `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`。不能取 `embedded` 产物后另行删字体，也不能复用另一策略的审核记录。
- 输出路径必须以 `.svg` 结尾，否则退出 2 且不写任何文件。
- 两种策略都要在对应实际输出字体下通过浏览器检查与实际看图；同一 candidate 的静态 SVG 字节数不得超过其 Reader HTML 的 50%。

## 产物

`static` 依次执行 validate → static → browser-check，写出三类文件：

- `OUTPUT.svg`：单个无脚本、无外部资源的静态 SVG，含主图、关系、标签与图内图例；不含 Reader 页面标题、节点索引或交互控件；卡片以 `<metadata data-phoenix-cards="">` JSON（每项 `dot`、`title`、`items`，顺序同 Reader）随 SVG 交付，不画进图面，由宿主显示。
- `NAME.static-summary.json`：固定字段的交付 summary。
- `NAME.static.1440x900.light.png` 与 `NAME.static.1440x900.dark.png` 两张截图；`--out-dir` 可指定截图目录。

宿主约定：`<html data-theme="light|dark">` 优先控制亮暗；没有有效属性时跟随系统偏好；打印为亮色。静态 SVG 自带候选与策略派生的 ID 前缀和本地引用，可与其他图同页共存。要显示卡片的宿主读取 `<metadata data-phoenix-cards>`。

## 看图与登记

实际打开 summary 列出的两张 1440×900 亮、暗截图，逐张写非空 `observations`，再执行：

```sh
"$PD" review-record OUTPUT.svg --record review.json --json
```

审核记录格式与单图 Reader 相同，必须与 summary 截图一一对应。自动三门 pass、`automatedStatus: "pass"`、`visualReview.status: "passed"`、`deliveryStatus: "complete"` 才算完成。

## 失败与恢复

- 自动门未过：按诊断修正后重跑 `static`（不是 `finalize`），再重新看图登记。
- SVG 已变化：重跑 `static`，重新看图并登记。
- `static` 不存在（旧引擎）或执行失败：如实报告真实阶段、退出码与保留文件，改用自绘 SVG 或 Reader 交付，不冒称静态交付完成。
