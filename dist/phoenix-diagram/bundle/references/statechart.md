# 状态机模型检查（statechart）

写 statechart 前读一次，与 [schema](../schemas/statechart.schema.json)、[common schema](../schemas/common.schema.json) 和[最小示例](../examples/minimal.statechart.json)一起用。

## 何时用

- **statechart**：要判断一个流程会不会违反某条规则、会不会卡住，并找出最短的出错路径。作者写有限模型，引擎穷举所有可达状态、逐条判定规则，再把结果画成状态图。
- **lifecycle**：要画一条已知的状态流（主路径、等待、终止与恢复），不需要判定规则。Mermaid `stateDiagram` 仍转为 lifecycle。

statechart 没有 `sources`/`meta.repository`、品牌和架构比较；图面状态由模型推导，没有作者给定的元素可以挂来源。

## 模型

- `model.variables`：每个变量用 `values`（同类型的字符串、布尔或整数，互不相同）或 `range: [lo, hi]`（整数）。域保持小，状态数是各域的乘积。
- `model.init`：键恰好是全部变量；值可以是数组，表示可选的初值，初始状态是它们的笛卡尔积。
- `model.actions`：`when` 是守卫（缺省为真），`set` 同时赋值（右侧读步前值，缺省原地不动），`pick` 让一个动作对每个参数值各走一步。
- `model.rules`：`text` 写成读者能认同的一句话，`holds` 写它在每个状态都应成立的条件。不为了让检查通过而放宽规则。id 不能是 `deadlock`；它会作为「违反 {id}」直接写在被标记的状态上，取读者看得懂的短名。
- `model.deadlock`：缺省检查死锁。用 `ends` 声明可以停下的终态；`check: false` 关闭检查。

表达式是单键 JSON 对象：

| 形状 | 含义 |
| --- | --- |
| `"s"`、`3`、`true` | 字面量 |
| `{"var": "x"}` / `{"next": "x"}` | 当前值 / 步后值（`next` 只用在 `view.steps[].when`） |
| `{"param": "p"}` | 所属动作的 `pick` 参数 |
| `not`、`and`、`or`、`implies` | 布尔运算，`and`/`or` 取数组 |
| `eq`、`ne`、`lt`、`le`、`gt`、`ge` | 比较，两侧同类型；大小比较只用于整数 |
| `{"in": [e, [值, …]]}` | 属于列表 |
| `add`、`sub` | 64 位整数加减，溢出报错 |
| `{"if": [c, a, b]}` | 条件取值 |

与变量或参数比较的字面量必须在其域内，拼错的值会报 `statechart/value-not-in-domain`，不会静默永不成立。

探索最多 100,000 个状态。到达上限即停，`complete` 为 false，未见违反的规则与死锁记为 `not-checked`，并发出 `statechart/state-limit` 警告。

## view

- `view.state`：标签规则按顺序取首个命中，最后一条通常不写 `when`。`label` 是字符串，或字符串数组（父状态路径，如 `["结账", "支付中"]`）。同名标签的模型状态合并为一个图状态。
- `view.regions`：两个以上并行区域 `{name, state}`。引擎会在每条箭头上自动写出它对其他区域的条件，如 `[除非 已锁定]`。区域图不能如实表达模型时，自动改画父状态，并在概况卡片写明原因。
- `view.steps`：给动作写读者经历的标签；`label: null` 隐藏该步；没有命中的步骤用动作 id。模板 `{name}` 取同名参数或变量（状态标签取该状态的值，步骤标签取步前值）。
- `view.groups` + `view.colors`：阶段着色，颜色取 `cyan、emerald、violet、amber、orange、slate`；`rose` 留给违反规则的标记。每个 group 都要有颜色。

写 view 的守则：

- 状态标签写读者看到的，步骤标签写用户经历的；箭头控制在约 20 条以内。
- 一步只在某些条件下发生时，在标签末尾用方括号写条件，如 `重试 [网关恢复后]`。区域图里不要给其他区域的条件写括号，引擎会自动写；作者的括号会替换它。
- 隐藏步骤时：每个图状态仍要能从起点沿箭头到达；模型有出步的图状态至少保留一条出箭头；一条箭头背后的步骤要么全隐藏、要么全保留。出错路径上的步骤不能隐藏（`statechart/hidden-bug-step`）。
- 合并标签时，不要把违反不同规则的状态合成一个；合并后只部分成员违反的状态会标成「可能违反」。决定下一步的变量没写进标签时，出错路径会看似停在同一个状态里又走了一步；把这个变量写进标签。
- 每个图状态的标签和标记文字各不超过 212px（约 16 个汉字），超出报 `statechart/state-text-too-long`；把细节移到父状态路径或卡片。
- 图超过 60 个状态或 40 条箭头时报 `statechart/chart-too-large`。整图默认从左到右；太宽时自动折行或改为自上而下，仍超出阅读宽度报 `statechart/chart-too-wide`，高于宽度 1.6 倍（手机上放不下）报 `statechart/chart-too-tall`。这些都靠合并标签、隐藏无关步骤或缩小域解决，不能删规则。
- statechart 没有路线、端口或布局字段。看图发现走线难以分辨时，candidate 侧能做的同样是合并标签、隐藏不影响结论的步骤或缩小域；做不到时如实报告图像审核未通过。

## 诊断码

| 代码 | 含义 |
| --- | --- |
| `statechart/unknown-variable`、`unknown-param`、`unknown-placeholder` | 名字不存在 |
| `statechart/next-outside-step` | `next` 用在了 `view.steps[].when` 之外 |
| `statechart/expression-type` | 表达式类型不符，evidence 给期望与实际 |
| `statechart/value-not-in-domain` | 字面量不在变量或参数的域内 |
| `statechart/init-incomplete`、`reserved-rule-id`、`group-color` | 初值缺变量、规则用了 `deadlock`、group 没有颜色 |
| `statechart/value-out-of-domain`、`arithmetic-overflow` | 运行中赋值越界或溢出，evidence 带动作、参数与到达路径 |
| `statechart/unlabeled-state`、`empty-label`、`leaf-is-parent` | 没有命中的标签规则、标签为空、一个路径既是状态又是父状态 |
| `statechart/hidden-bug-step` | 出错路径上的步骤被隐藏 |
| `statechart/state-text-too-long`、`chart-too-large`、`chart-too-wide`、`chart-too-tall` | 图状态文字或整图超出可读范围 |
| `statechart/state-limit`（警告） | 到达探索上限 |
| `statechart/artifact-mismatch` | HTML 中的标记或箭头与推导结果不符 |

## 回执与卡片

validate、deliver 回执和 finalize 摘要带 `model`：

- `states`、`steps`、`complete`、`stateLimit`
- `chart`：`states`、`arrows`、`regions`、`regionsFallback`（回退原因，未回退为 null）、`inside`（停在同一图状态内、不画的步数）、`hidden`
- `rules[]`：`id`、`text`、`outcome`（`breaks`、`holds`、`not-checked`）；`breaks` 带最短 `path`，首项为起点 `{state}`，其后每项为 `{step, state}`

图下的卡片依次为概况、每条违反规则一张、成立的规则、未检查的规则，之后才是作者 `cards`。规则判定不影响退出码：画出出错路径本身就是一次成功的交付。

## 交接

- 逐条列出 `outcome: breaks` 的规则：作者写的那句话、id 与最短路径的每一步。
- 说明 `not-checked` 的规则及 `complete: false`、隐藏了哪些步骤、模型省略了什么（例如只建模了一次重试）。
- 模型描述的是代码时，按路径逐步在代码中复现每条出错路径；复现之前，先假定是模型写错了。
