# 创作默认值

新稿写 candidate 前读一次；已有候选仅在修复需要改变创作决策时读取。以用户材料和匹配 schema 为准，示例不是可复制的私人事实。

## 事实与修复

保留所有必要责任、实体、关系方向、协议、同步/异步行为、条件、waiting 与恢复路径。关键审批、授权或状态条件放在受影响节点或边上，卡片补充解释。边界必须有真实隔离、所有权、运行或持久化依据。没有节点、边、来源、卡片或边界配额；需要更多内容时增加有意义的行和页面高度。

关系标签携带动作、协议、数据和方向含义。碰撞时先调整间距、自动路线或标签位置，再在保留意义的前提下改写措辞。schema 允许空标签时，仅可省略两端已完整表达且无新增语义的文字，并解释这一创作选择；不能为修几何删除必要标签。Dataflow 的每条 flow 必须有非空 `label`。

用诊断的 `subject` 定位对象、`evidence` 核对实测、`supportedFixes` 选择允许修复。保持语义清单、用户 pins 与已确认拓扑，只修改相关布局邻域。按实际失败门记录 error 数、历史最佳和停滞计数：首次失败计数0，修复后严格低于该门最佳才更新并清零，否则加1；到2立即停止，不因错误种类变化重置或再试。重建交付或返回已记录门时，沿用该门历史，不因新回执重新初始化；环境失败也不允许无限重试。例如同一门 `6 → 4 → 5 → 4` 的后两轮都未优于历史最佳4，应停止。前门通过后首次进入后门是实际进展，按新门初始化并保留全部门状态，不混算跨门的顶层 error 数；也不额外添加强制摆放或重试轮次。

## 新旧合同

新稿明确写 `meta.input_contract: "phoenix-v1"`，默认 showcase。Workflow 新稿使用 `schema_version: 2`，Lifecycle 新稿使用 `schema_version: 3`；其余三类为 1。已有未标记 JSON 保留原字节，按 `legacy-2.17` 验证；不要仅为套用新默认值迁移合同。CLI 的显式 `--input-contract` 覆盖必须与用户已授权的导入意图一致。

读取 `compatibility.coreCompatibility` 与 `compatibility.deferredFields`。品牌、repository sources、旧 guided views 等延期字段逐项报告处理结果及期次，并保留原 JSON；不能将有延期字段的整份输入称为全量兼容，也不能借延期删除核心事实。P1 不执行品牌查询/采集、仓库来源自动核对或旧安装认领。

## 布局

- Architecture：先按真实连接组织主路径与子系统，支线靠近负责节点；保留真实边界及显式坐标。新稿可用 schema 的 grid 字段组织有意义的行，省略无依据的固定 viewBox，先用自动路线。
- Workflow：保留泳道、条件、分支、回路及 mainPath。跨阶段许可按已加载 SKILL 的创作步骤逐状态核对，不能将先补交、重提等业务动作变为前提；保留等价表达，不要求逐节点直接边。新稿省略固定 viewBox 和手工宽度，让编译器测量标签；已有 v1 固定几何不切成 v2。若实测自动路线无解或仍有共用通道，优先调整相关未固定节点的 `col` 或间距，给整条路线留出空间；随后再按诊断决定端口或路线 pins。
- Sequence：保留参与者顺序、消息方向、`y`、同步/异步、return 与完整 notes。新稿省略 `meta.column_fit` 和 viewBox 使用新合同默认布局；旧合同缺省 fixed 保持原样。参与者内部动作放在其真实跨参与者消息的 note 中，不造 self-message。给 note 的实际换行高度留空间，28px 基础消息间距不代表 note 容量；修复时协调后续消息、segment 和 activation，不能删 note 含义。
- Dataflow：明确每个输入由哪个 producer 实际输出，组合转换应保留独立标注的各路输入。真实存储端使用 `database`，内存计算通过生成它的 `backend` 表达。以 stage/row 网格开始；普通数据用 default，主数据 emphasis，确有策略/PII 含义才用 security，异步批处理才用 dashed。
- Lifecycle：`mainPath` 列出真实主路径，并显式写出每条 transition；保留退出边的 label、note、variant。等待用 waiting，可恢复失败必须存在真实返回边。不得把终止或无法恢复伪装成 recovery。

只有实测缺陷或用户固定几何要求才添加 route/label pins。自动端口方向是离开/进入边界的方向合同；不得接受穿过无关不透明节点、含混共用通道或标签盖住其他关系的路线。

## 文本与展示

沿用户选择或对话主语言创作，保留原样标识。`meta.locale` 仅选择 Viewer 固定界面，P1 使用 `en` / `zh-CN`；其他语言需说明固定界面和 `<html lang>` 英文 fallback，不翻译作者内容，也不声称完整五语已验收。

省略默认 `meta.visual_preset` 得到 classic；省略 subtitle、legend 得到标题与 truthful auto。其他视觉风格、额外副标题须有用户请求。`engineering_profile` 仅在明确请求且事实已知的部署所有权场景启用，启用后不为过检而删除。

桌面图保持完整可读且无水平溢出；按真实内容增加纵向节奏。只接受 Reader 已声明、浏览器已测量的页面纵向滚动，具体见[交付合同](delivery-contract.md)。禁止裁剪、内部图解滚动或缩小字体来伪造通过。
