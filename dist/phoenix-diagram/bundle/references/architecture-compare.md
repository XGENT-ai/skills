# 架构比较

仅在用户明确要求比较两份架构时使用。两份输入都必须是 `architecture`；其他图型继续使用各自的单图流程。比较只陈述输入中可追溯的变化，不推断运行影响、因果、风险或可合并性。

## 输入与生成

保留原 JSON。组件通过 `components[].id` 对应，连接通过作者明确写入的 `connections[].id` 对应，边界通过 `kind + label` 对应。边界键须唯一，两份输入至少共享一个组件 ID；两侧都写仓库时必须属于同一仓库。连接缺稳定 ID 时，普通单图仍可使用，比较会拒绝；由作者确认对应关系后再补 ID，不能按数组位置、标签相似度或端点猜同一关系。

```sh
"$PD" compare architecture base.json head.json comparison.html --json
"$PD" compare architecture base.json head.json comparison.html \
  --quality showcase --repo-root "<仓库根>" --brand-cache "<本地品牌缓存>" --json
```

`--quality` 与输入已有质量标记冲突时会失败；不要为换取比较通过而降低质量。仓库来源和品牌继续使用已固定的本地身份，普通比较不联网采集资源。

compare 写出 HTML 和 HTML 旁的比较 summary，默认 `comparison.html.compare.json`；`--receipt PATH` 可在同一目录另取文件名。HTML、summary 与两份输入应是不同文件；失败时不写任何文件。

比较 summary 包含双侧原始/语义摘要、编译配置、引擎、来源、品牌资源、比较 IR、HTML 身份与 artifact 检查。`status: pass` 只表示比较与写出成功，`browserStatus: not-run`、`visualReview.status: required-pending`、`deliveryStatus: incomplete` 仍须如实保留。它不等于交付完成。

## 阅读三个视图

Before、Delta、After 分别展示原版、变化和新版。Delta 保留完整的新版事实，旧位置或已移除内容以幽灵层作对照；这些辅助图元不成为新版 Reader 的真实拓扑。点击变化列表定位相应对象，保留未变事实和每侧完整标签、来源及 Reader 控件。无变化时仍可读取原版和新版。

字段变化分类为 semantic、evidence、geometry、topology、scope。来源变化与移动、改线等几何变化分开；数组重排、对象键顺序、来源条目顺序或 `meta.output` 改动不自行成为业务变化。仅在两侧来源已验证、仓库相同且提交已固定时为 `proofLevel: revision-pinned`，其他情况为 `authored`。

## 自动检查与实际看图

```sh
"$PD" check comparison.html --json
"$PD" visual-check comparison.html --out-dir compare-evidence-01 --json
```

`check` 重跑比较 artifact 检查，只输出 stdout。`visual-check` 对三个视图分别测量四个桌面窗口及窄屏、暗亮主题，写出截图，并把浏览器回执、`browserStatus` 和 `screenshots` 写回比较 summary（返回的 `summary` 为其路径）。HTML 与 summary 记录不一致时会要求重新 compare。实际看图使用 summary `screenshots` 的 12 张图：每个视图的 1440×900、2048×1320 各 light/dark。逐张打开，核对主图、标签、卡片、关系遮挡、层级和控件；Delta 还须核旧位置、删除标记和变化定位与输入相符。DOM 数值、截图存在或自动测试不能替代观察。

visual-check 通过后视觉仍是 `required-pending`、交付仍为 `incomplete`。缺工具、浏览器失败、某视图缺测或尚未实际看图分别报告，不沿用普通单图或另一次比较的审核。

## 审核记录

按[交付合同的审核记录](delivery-contract.md#审核记录)写同样格式的 JSON，`screenshots` 逐项列出比较 summary 中的 12 个路径，每张写实际观察。没有缺陷才能用 `verdict: passed`、`findings: []`；有缺陷用 `failed` 并写具体 findings；无法看图用 `unavailable` 并写实际限制。

```sh
"$PD" review-record comparison.html --record comparison-review-01.json --json
```

review-record 要求 visual-check 已通过且 HTML 与比较 summary 一致，把审核写回比较 summary。再读该 summary：`browserStatus: pass`、`visualReview.status: passed`、`deliveryStatus: complete` 才能报告完成；`failed` 或 `unavailable` 保持 `incomplete`。输入改变需要重新 compare、visual-check 和看图审核；重新 visual-check 会清空已有审核。
