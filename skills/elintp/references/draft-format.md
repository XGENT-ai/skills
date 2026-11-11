# 内容稿格式

写内容稿前读本文件。内容稿是受限 Markdown；只负责内容、数据和本地文件引用。页面的样式与布局由脚本固定，不写 CSS、布局参数或图形坐标。自绘降级图另存为 SVG。

运行 `node <本 skill 目录>/scripts/render.mjs <内容稿.md> <输出.html>`。相对路径按运行命令的当前目录解析，跨目录时可用绝对路径。成功打印 `PAGE:`；退出码 2 时按 `L行号: 问题。改法：…` 修改内容稿再跑，旧页面不会被覆盖。页面不携带内容稿，请在当前任务需要修改时保留临时稿件。

## 页头与小节

开头用两行 `---` 包住页头，每行一个 `键: 值`；值是文字，不是 YAML 对象，不加引号、不写行尾注释。

| 键 | 写法 |
| --- | --- |
| `template` | 必填：`general`、`prd`、`crd`、`plan` |
| `title` | 必填：页面标题 |
| `lang` | 必填：内容语言，如 `zh-CN`、`zh-TW`、`en`、`ja` |
| `subtitle` | 可选：一行说明 |
| `source` | 可重复：出处与读到的版本，在页末呈现 |
| `progress` | 只在 `plan` 中必填：`todo`、`partial`、`done` |

其他键不支持。简体中文标签（`zh`、`zh-CN`、`zh-SG`、`zh-Hans…`）使用简体界面；`zh-TW`、`zh-HK`、`zh-MO`、`zh-Hant…` 使用繁体界面，其余用英文界面。`<html lang>` 保留填写的语言标签。

页头后、第一个 `##` 前写至少一段主题结论，只能写段落。小节写 `## slot | 这一节的结论`，里面必须有内容。按下表顺序排列，★ 必填；除 `more` 外不可重复。

| 模板 | 小节顺序 |
| --- | --- |
| `general` | `why`★ → `scale` → `risks` → `more` |
| `prd` | `gain`★ → `delta`★ → `rollout`★ → `more` |
| `crd` | `ask`★ → `stance`★ → `agreed`★ → `open`★ → `more` |
| `plan` / `todo` | `build`★ → `route`★ → `more` |
| `plan` / `partial` | `built`★ → `verify`★ → `build`★ → `route`★ → `gaps` → `records` → `more` |
| `plan` / `done` | `built`★ → `verify`★ → `gaps`★ → `records` → `more` |

`more` 可重复，放重心之外但不能丢的警告、数字和前提，不显示固定标签。通用模板的 `scale`、`risks` 只有材料确实没有时才省略；计划的 `records` 在没有相关过程记录时可省略。其他必填小节没有内容时如实写「原文没有说」，不编造。计划的完成状态依据原文记录，不把转述写成亲自验证。

## 正文与组件

- 连续非空行组成段落，空行分段。小节内可用 `### 小标题`；不支持一级或四级以上标题。
- 行内支持反引号代码、`**强调**`、`[文字](链接)`。链接只接受 `http://`、`https://`、`mailto:` 和 `#` 开头的页内锚点。内部小节锚点依次为 `#s1`、`#s2`。
- `<script>`、HTML 标签之类的文字会原样显示，不执行；代码块内容逐字显示。不要用原样 HTML 代替组件。
- 无序列表用 `- 项`，子项缩进两个空格，最多两层。步骤用 `1. 项`、`2. 项` 连续递增，每段从 1 开始。
- 代码块用单独一行的三个反引号闭合，起始行可带语言名。
- 不支持 `>` 引用块，改为提醒块或普通段落。

### 表格

```text
| 结果 :: 当前检查的结果 | 说明 :: 未完成时还需要什么 |
| --- | --- |
| [ok] 已生成 | 可打开页面 |
| [warn] 未验证 | 仍须实际看图 |
| [no] 无法读取 | 核对文件路径 |
```

每行列数与表头一致；表头 `::` 后必须有释义。单元格开头的 `[ok]`、`[no]`、`[warn]` 后必须有文字，分别显示 ✓、✗、!；其他单元格正常显示。单元格内不要放管道字符，拆句或用普通段落解释。宽表格在自身区域滚动；表头释义可用键盘展开，禁用脚本或打印时直接显示。

### 结论、提醒与指标

```text
:::conclusion 请求先进入队列
队列将接收请求与执行任务分开。
- 接收方先确认收到。
- 工作者稍后处理。
:::

:::reminder 容量有限
队列满时接收方必须明确返回失败。
:::

:::metrics
200ms | 平均等待 | 从收到到开始处理
:::
```

结论与提醒的标题可省略，内部只放段落或列表。指标每行恰好三段：数值、名称、含义，三段都不能为空。每个容器用单独一行 `:::` 闭合，不嵌套容器。

### 图片与图

图片单独占一行，说明同时用作图注与替代文字：

```text
![记录中的部署结果](docs/screenshots/deploy.png)
```

只用本地 `png`、`jpg`、`jpeg`、`gif`、`webp`、`avif`、`svg` 文件；远程地址需先存成本地文件。脚本嵌入 data URI。图片缺失或读不了会显示「图片缺失」及原路径，stderr 输出 `WARN`，仍可生成；不得据此声称截图完整。

```text
:::figure 请求从作者进入处理流程
reader: /absolute/path/diagram.html
阅读器图注。图例与标签在图内。
:::

:::figure 队列连接接收方和工作者
svg: /absolute/path/queue.svg
箭头表示任务流向。
:::
```

`:::figure` 后的标题必填；首个非空行只能是一个 `reader:` 或 `svg:` 来源，其后是图注。`reader:` 只引用 phoenix-diagram 已交付的自包含 HTML；脚本嵌入完整阅读器，并加入无脚本时隐藏主题按钮的规则，不修改源图文件、不替图工具做审核。图文件不存在或读不了是格式错误。启用脚本时，阅读器自己的主题控件独立于宿主页面。

自绘 SVG 根元素须有 `viewBox`。文字与线条用 `currentColor`，或使用模板类名 `f-fg`、`f-muted`、`f-accent`、`f-surface`、`f-ok`、`f-warn`、`f-no` 表示填充；对应 `s-` 前缀表示描边。不用固定黑白色，否则暗色下可能不可读。

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 80">
  <rect x="1" y="1" width="318" height="78" rx="8" class="f-surface s-muted"/>
  <text x="160" y="48" text-anchor="middle" font-size="24" class="f-fg">队列</text>
</svg>
```

允许元素：`svg g defs title desc path rect circle ellipse line polyline polygon text tspan marker linearGradient radialGradient stop clipPath use`。属性限于这些元素的几何、文字、颜色、渐变、标记、裁切、可访问标签与本地引用；所有属性值加引号。不允许 `style`、`on…` 事件属性、脚本、`foreignObject`、DOCTYPE、CDATA 或外部资源。`href` / `xlink:href` 只接受 `#片段`，`url(...)` 只接受 `url(#片段)`。报错会同时指向内容稿行号和 SVG 内的行号、标签；修正源 SVG，不靠静默删除规避。

## 四种模板最小例子

### 通用主题

```markdown
---
template: general
title: 队列把接收与处理分开
lang: zh-CN
---
请求先进入队列，工作者准备好后再处理。

## why | 两端不必同时就绪
队列保存已接收的任务，等待工作者读取。
```

### PRD

```markdown
---
template: prd
title: 导出时可以选择日期范围
lang: zh-CN
source: docs/PRD-export.md · v1
---
导出只包含指定日期范围内的记录。

## gain | 新增日期筛选
作者可以选择起止日期。

## delta | 全量导出变成按范围导出
日期范围之外的记录不进入结果。

## rollout | 先明确日期边界
原文没有说具体排期。
```

### CRD

```markdown
---
template: crd
title: 接收方需要提供匯出狀態
lang: zh-TW
source: CR-export.md · RESP-01
---
提出方要求查詢任務狀態，接收方尚未答覆。

## ask | 提出方向接收方要求查詢狀態
提出方需要區分進行中與已完成的匯出。

## stance | 尚未取得接收方答覆
原文沒有說接收方的立場。

## agreed | 尚無雙方確認的約定
目前只有提出方的請求。

## open | 接收方下一步評估請求
交付日期尚未談定。
```

### 已完成计划

```markdown
---
template: plan
progress: done
title: Export plan records a completed implementation
lang: en
source: docs/plan/export.md · completion record
---
The plan records the export implementation as complete; this session has not verified it.

## built | The plan records date filtering as delivered
Exports accept a start and an end date.

## verify | Check records at both date boundaries
Run the verification command listed in the original plan.

## gaps | The plan does not state follow-up work
No further completion claim can be made from this source.
```

## 常见报错

| 提示 | 修改方法 |
| --- | --- |
| 缺少引言 | 在第一个 `##` 之前写结论段落 |
| slot 未知、重复、缺失或顺序不对 | 对照本模板的顺序表，调整小节，不改模板 |
| 表格列数不对 | 按表头补齐或删去多余列 |
| 容器或代码块未闭合 | 加单独一行 `:::` 或三个反引号 |
| 图来源不止一行 | 每张图单独一个 `:::figure`；每个只有一个来源 |
| SVG 白名单失败 | 按错误中的 SVG 行号改标签或属性，外部引用改成本地片段 |
