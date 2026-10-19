---
name: phoenix-diagram
description: "用于架构、工作流、时序、数据流、生命周期及对应 Mermaid 语义转换；不用于数值图表、仪表盘或其他 infographic。生成经 Rust 检查、浏览器验证和实际图像审核的独立交互 HTML。"
license: MIT
metadata:
  version: "0.1.0"
  based_on: "tt-a1i/Archify; Cocoon-AI/architecture-diagram-generator (MIT)"
---

# Phoenix Diagram

把授权材料整理成完整的五类 JSON 图解，由原生引擎生成独立 HTML。保留每个必要实体、关系、方向、条件和标签，不设节点、边或卡片配额。示例只说明字段结构，不能成为用户的私有事实。缺少影响含义的参与者、条件、日期或金额时先询问，收到答复前不写 candidate 或生成图。

普通依据材料作图时，原材料已明确指出业务条件、日期或金额缺失，就直接走澄清分支；不能以主干仍可画或已有结果标签为由继续作图。其他影响含义的必需事实缺失时，同样先澄清：

1. 只提取阻止按原材料作图的必需缺口，不发散到材料未提及的可选分支、异常路径或设计选择。已给出的角色、关系和标签直接按原文使用，不以细化称谓、层级或内部细节为由追加问题。
2. 把每个缺口写成询问其实际内容的开放式问题。不列候选答案，不用疑问句、选项或括号夹带假设的日期、金额、阈值、分支、触发条件、次数上限或终止规则；需要字段格式时只用未赋值占位符。
3. 发送前逐条删去候选答案、假设例子、扩展问题与流程复述，**只输出剩下的简短问题列表，随后立即结束**。不说明后续创作步骤。

本轮不写 candidate 或生成图，不提供流程草稿，不用缺失规则卡片代替询问。把具体值称为“草拟”“默认”或“待确认”，也不能代替用户提供事实。

只有用户明确要求拟定这些值或规则时，才给清楚标为建议的方案；普通作图请求不包含这项授权。

## 选择图型

| 类型 | 适用问题 | 示例 |
| --- | --- | --- |
| `architecture` | 组件职责、服务连接、真实系统边界 | [最小架构](examples/minimal.architecture.json) |
| `workflow` | 审批、操作流程、条件分支和回路 | [最小工作流](examples/minimal.workflow.json) |
| `sequence` | 参与者之间的调用、返回和异步消息 | [最小时序](examples/minimal.sequence.json) |
| `dataflow` | 数据来源、加工、存储和消费关系 | [最小数据流](examples/minimal.dataflow.json) |
| `lifecycle` | 状态变化、等待、终止和恢复 | [最小生命周期](examples/minimal.lifecycle.json) |

不明确时运行 `guide`，按返回的包内 `schema`、`commonSchema`、`example` 路径选择。P1 不支持 ERD、class、tree、timeline、waterfall 或其他 infographic；不能将数值图表或仪表盘改称这五类图来绕过范围。

Mermaid 作为语义材料处理：`flowchart` / `graph` 转为 workflow，组件连接图可选 architecture；`sequenceDiagram` 保留参与者、消息方向、同步/异步与 return；`stateDiagram` 保留主路径、waiting、终止及真实 recovery 边。由宿主写新的 JSON，不运行 Mermaid 或 Node 图解 renderer，不照搬 Mermaid 样式。

## 最短创作路径

以下创作步骤仅适用于事实已齐备的材料。

命令从用户工作目录执行。将下方 `PD` 替换为**当前加载的本 skill 目录**中的绝对路径，候选和输出路径使用用户项目的实际路径：

```sh
PD="<当前加载的skill目录>/scripts/phoenix-diagram"
"$PD" guide "API 调用链" --json
```

1. 新稿只读所选 `schemas/<type>.schema.json`、[common schema](schemas/common.schema.json)、一个匹配示例及[创作默认值](references/authoring-defaults.md)，随即写完整 candidate。新稿设置 `meta.input_contract: "phoenix-v1"`、`meta.quality_profile: "showcase"`；从自动路线和自动标签开始。保留用户提供的固定几何，不在首稿预先添加 `via`、`channelX`、`channelY` 或 `labelAt`。

   写出 candidate 或完成涉及语义的修复后，按原材料逐分支走读实际可走路径，核对条件、动作、执行者与先后关系；进入处理步骤不等于已经通过或获准。仅在某条件成立后允许的动作，必须留在该条件控制的分支上，不能前移为其他分支的共同前置动作，也不能因合并节点或回路改变原有顺序。跨阶段适用的动作，先按原文枚举许可覆盖的全部非终态，再逐项核对主图路径；例如“任一下单前可撤回”也覆盖提交、补充材料等适用阶段。每个适用阶段都应能行使该许可并抵达相应结果，无需先满足额外条件或完成审批、提交、补交等其他业务动作。无条件且不执行业务动作的瞬时起始跳转可与后续阶段等价，显式覆盖全部适用阶段的结构也可等价表达，不要求逐节点各画一条直接边。卡片或“各阶段可……”文案不能代替主图中这项许可的完整范围。线型、颜色与图例也必须对应材料给出的关系含义；未说明的同步/异步、批处理等执行方式，使用不承诺该含义的普通关系表达，不能凭画图习惯补成事实。节点、标签齐全或卡片复述不代表路径等价；只据已给事实修正，必需事实缺失时沿用前述澄清规则，不自行补规则。
2. 已有完整 candidate 直接进入 `finalize`。不先读 renderer、扫描全仓或重新改写输入。未标记的旧 JSON 默认按 `legacy-2.17` 处理，不静默切换新合同。

   ```sh
   "$PD" finalize <type> candidate.json diagram.html --json
   ```

3. 读取真实阶段及 diagnostic 的 `subject`、`evidence`、`supportedFixes`，只修相关邻域。修复后重新 finalize；必要时用 `validate <type> candidate.json --layout-json --json` 定向诊断。首次失败时按实际失败门记录客观 error 数、历史最佳值和停滞计数0；同一门每次修复后，只有 error 低于该门历史最佳才更新最佳并清零，否则停滞计数加1。计数达到2立即停止并列出未解决项；错误种类变化不能重置计数，也不允许再试一次。重建交付或返回已记录门时，沿用该门历史，不因新回执重新初始化；环境失败也不允许无限重试。已通过前门而首次进入后门属于实际进展，按新门初始化记录；保留每轮全部门的真实状态，不把不同门的顶层 error 数拼成同一停滞序列。不能删语义、降低质量、缩字或隐藏溢出来换通过。
4. 按[交付合同](references/delivery-contract.md)实际打开当前四张暗/亮截图，检查主图、标签、卡片、关系遮挡、层级和控件。默认必须做实际图像审核；DOM 数值、截图存在或测试通过不能替代看图。随后 `review-record` 登记，再回读**同一交付**的当前 summary。只为登记审核或取得完整状态不得再次 finalize。

自动四门通过后，视觉状态仍可能是 `required-pending`；只有四门 `pass`、`automatedStatus: "pass"`、`visualReview.status: "passed"`、`deliveryStatus: "complete"` 且身份仍绑定当前输入/HTML/配置/引擎/截图时，才能报告完整交付。修改 candidate 会使旧证据失效，需要新交付及对应的新审核。

## 环境与停止

普通 launcher 和安装脚本需要 Node.js 18+；图解核心为 Rust，消费环境不需要 Rust、Cargo 或检出源码。普通命令只验证已存在的引擎，不下载、不更新、不安装，不主动打开用户浏览器页面或启动 preview 服务。`finalize` 的隔离浏览器验收属于默认交付路径。

工具或图像能力不可用时如实保留 `unavailable` / `not-run` / `required-pending` 和未完成门，交付可用 candidate 或已有文件。无 shell 时不能手写 SVG 冒充 Rust 校验通过。出现 `recovery-required` 时按交付合同识别确切事务，不自动选择恢复记录。

缺引擎时给出错误回执的 `setupCommand`；只有用户已明确请求引擎安装才执行，并按当前 `ENGINE.json` 的 `distribution` 处理：

- `release`：使用回执给出的固定发行版本命令。
- `candidate` / `development`：先核对用户或本题已提供材料是否明确给出了配套完整 release-dir 的实际绝对路径。
  - **未提供路径：只询问该绝对路径并停止安装，等待答复。** 安装授权不包含寻找或推定候选资源来源；不搜索项目外目录，不猜路径，不省略 `--release-dir` 试装，也不执行含 `MATCHING_RELEASE_DIR` 占位符的命令。
  - **已提供路径：** 将回执命令中的 `MATCHING_RELEASE_DIR` 替换为该实际绝对路径后再执行；不改用 registry 获取同名同版本的另一份包。

首次阅读为 Still，用户主动选择 Live。保留用户主要语言和原样的产品、代码、协议与 API 标识。只在用户请求 Reader 功能、导出或阅读交互时读[阅读器运行约定](references/viewer-runtime.md)。

## 交付答复

简短返回 HTML 和 candidate 路径、图型、原 `deliveryReceiptId`、当前四门结果、图像审核结果及 summary 路径；有失败时说明真实阶段、保留文件和具体缺口。退出码非零、旧输出存在或自动检查通过都不能单独描述为完整成功。
