# 模板报告数据

写 `report.json` 时读取。页面版式固定，这里只决定填什么；项目差异都放进 `sections`，不改模板。

```json
{
  "schemaVersion": 1,
  "batchId": "AI-ASSISTANT-FAB",
  "title": "常驻助手",
  "summary": "每个页面右下角都能打开助手。",
  "meta": {
    "source": "docs/plan/AI-ASSISTANT-FAB.md",
    "baseline": "f9422e52",
    "environment": "本地 one-box，本地测试数据，只读"
  },
  "overviewChart": "overview.html",
  "items": [
    {
      "id": "assistant-entry",
      "name": "常驻入口与浮窗",
      "summary": "原来只有部分页面能打开，现在每个页面都有入口。",
      "howTo": ["点右下角 `AI 助手`。", "按 Esc 收起。"],
      "limits": ["应用区过矮时面板铺满。"],
      "note": "数据是本地测试数据。",
      "video": "assistant-entry.webm",
      "chart": "assistant-entry.flow.html"
    }
  ],
  "sections": [
    { "title": "没有录制", "items": ["**发送问题**：未授权写入。未实跑。"] }
  ]
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `schemaVersion` | 是 | 固定为 1 |
| `batchId` | 是 | 批次标识，项目内稳定 |
| `title` / `summary` | 标题必填 | 报告标题、一句话摘要 |
| `meta.source` | 是 | 计划路径或 git 范围 |
| `meta.baseline` | 是 | commit 或版本 |
| `meta.environment` | 是 | 入口、数据来源；用了替身或验收 harness 时写明 |
| `overviewChart` | 否 | 整批总览图 |
| `items[].id` / `name` / `summary` | 是 | 目录定位标识（字母、数字、`-`、`_`）、名称、解决什么 |
| `items[].howTo` / `limits` | 否 | 用法步骤、限制，都是文字列表 |
| `items[].note` | 否 | 这一项用了替身或测试数据时的说明 |
| `items[].video` | 否 | `.webm` 或 `.mp4` |
| `items[].chart` | 否 | 独立 HTML 的图，`.html` 或 `.htm` |
| `sections[].title` / `items` | 选了就必填 | 节标题和条目，按顺序显示 |

路径相对 `report.json`，必须在批次目录内，可以以 `./` 开头，不能含 `.`、`..` 段或反斜杠。文字支持行内代码、粗体和 http(s) 链接，其他写法按原文显示。

上限与 SPMS 发布一致，`render-report.mjs` 生成时就会拦下：

- `title` 不超过 200 个字符；`report.json` 的字段数据不超过 2MiB。
- 视频和图各最多 20 处引用（图含 `overviewChart`，同一文件被多处引用按次数算）。
- 视频单文件不超过 100MiB，图单文件不超过 25MiB。
- 字段数据加全部引用文件合计不超过 500MiB，同一文件只计一次。

发布到 SPMS 时，`report.json` 的内容就是报告字段，本身不作为文件上传；只上传它引用的视频和图，`index.html` 与 `*.mjs` 不上传。同一 `batchId` 再发布会原位替换同项目里的那份报告。

## 常用的可选节

没有内容的节不写。按需要另起标题，不用都写：

- `没有录制`：不可录的功能、原因、用法，标明是否实跑过
- `计划有但没实现或走不通`：计划项、实际情况、依据
- `改动里有但计划没写`：改动、影响、用法
- `细小修复`：一行一项
- `重新录制`：启动方式和就绪判据、录制命令、需要的环境变量名
