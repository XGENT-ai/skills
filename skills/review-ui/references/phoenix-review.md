# Phoenix managed 评审协议

仅在识别或接入 Phoenix managed 循环时读取。纯 Markdown 循环继续按技能正文和四份合同人工执行；本参考不新增评分或处置规则。规则投影的唯一事实源为 [rules.v1.json](rules.v1.json)，绑定 [报告](report-contract.md)、[独立评审](judge.md)、[目标与采集](target-and-capture.md) 和 [修复记录](../../apply-ui-review/references/record-contract.md) 合同。

## 先发现身份与能力

1. 从本循环的安装回执或固定版本包清单读取期望的 package/runtime/review schema；不能拿 PATH 中任意同名命令、上游 `impeccable` 或旧 CLI 当成匹配 engine。
2. 用实际 `phoenix-ui engine-probe` 核对身份，再执行 `phoenix-ui review --help` 与 `phoenix-ui review schema --json`；可用 `--kind <kind>` 只取该操作的输入形状。核对返回的 `reviewSchema` 与所需类型、变体后才构造机器输入。发现结果是当前 serde 类型校验的完整样例与枚举变体，`format: typed-examples`，不是完整 JSON Schema。样例不是证据：其中 `EXAMPLE_ONLY` 文本、数字和布尔值均须换成实际操作或取证值；cycle、round、cell、packet、revision 取实际返回，hash、被测版本和取证声明取真实材料。帮助/查询不读写循环，退出 0 只证明发现查询成功；入口不存在、身份/版本或协议不匹配时保持只读，不猜字段。
3. 读已有索引与 `review status`，确认前缀、当前轮、baseline、revision、状态和原记录位置。未来 schema、损坏或不能识别的索引不能按「无索引」处理。
4. 已有 managed 索引而 engine 缺失或不匹配：只读历史，可采集未登记素材；不计分、不裁定、不改代码、目标、处置、计数或生成区。给出按实际固定包版本恢复 engine 的命令，例如 `npm exec --package=@xgent-ai/skills@<packageVersion> -- phoenix-ui engine install --project <项目路径>`，替换为回执中的真实值并正确引用路径；没有期望版本时先说明所缺的回执，不能填一个猜测版本。恢复后续接原轮次，不重置、不静默降级、不自动下载或升级。

循环索引的实际路径是 `<前缀>.ui-review.json`；当前识别口径为 `schemaVersion: 1`、`mode: managed`。缺 CLI 时直接读原文件识别既有状态；字段缺失、未知 schema 或损坏时保持只读，不能视作不存在。固定包清单声明的 `reviewSchema` 与 engine 身份都须核对；仅收到文本身份 probe 不能推断支持 review verbs。

没有索引时还要检查旧 Markdown 轮次和修复历史：已有人工历史就继续人工或显式 import，不能 begin 清零。全新无旧历史且用户已要求验收时，匹配 engine 与实际 verbs 后可 begin managed；能力不可用则按人工流程。不能把「无索引」等同「永远人工」，也不能把旧历史自动接管。

## 固定 verbs 与调用时机

以下是操作形状，不是可直接写入的 JSON schema。路径、round、revision 和输入字段取自本项目的真实材料及匹配 engine 返回；工作流操作和 schema 查询带 `--json`，帮助也可用 `--json`。执行时保存实际 argv、输入、输出与退出码。

| 操作 | 参数 | 用途与边界 |
| --- | --- | --- |
| `phoenix-ui review --help` | 也支持 `help`、`-h`；可选 `--json` | 只读列出现有 verbs、flags 和协议发现入口，不读写循环 |
| `phoenix-ui review schema` | `--json`；可选 `--kind <kind>` | 只读输出 `reviewSchema: 1`、`discoveryVersion: 1` 的 typed examples；kind 为 `tested-version`、`capture`、`judge`、`mappings`、`adjudication`、`runtime-check`、`applied`、`import`。覆盖两种 import 形状与嵌套字段/变体；未知 kind 返回 2，样例不能直接充当真实输入或证据 |
| `phoenix-ui review begin` | `--prefix <前缀> --target <目标包>`；可选 `--plan <计划> --scope <范围> --threshold <门槛> --tested-version <被测版本.json>` | 分配轮次、冻结目标、矩阵与被测版本。被测版本写 HEAD 加工作树指纹或可核对的构建 ID，本轮采集必须与它一致；省略时为未知版本，该轮不能判通过。已有未结束轮次就续接，不再 begin 一轮绕过它 |
| `phoenix-ui review capture add` | `--round <round> --input <capture.json> --expect-revision <revision>` | 登记真实采集或明确 missing；补采追加 attempt，不覆盖历史图片 |
| `phoenix-ui review judge-pack` | `--round <round>`；可选 `--units <单元>` | 生成不可覆盖的白名单材料包及 packet hash；只能选该范围内材料完整的单元 |
| `phoenix-ui review judge add` | `--round <round> --input <judge.json> --raw <评审原文> --packet <packet> --expect-revision <revision>`；可选 `--mappings <mappings.json>` | 登记评审本人输出，或保留原文的忠实转换；不替评审补分或删差距。评审的新临时编号与同单元已有正式编号是同一问题时，用 `--mappings` 登记数组 `[{key, id, basis}]`：key 为该评审的临时编号，id 为已分配的正式编号，basis 为判为同一问题的依据；未映射的临时编号分配从未用过的新编号 |
| `phoenix-ui review finalize` | `--round <round> --input <adjudication.json> --expect-revision <revision>`；给已封存的最新轮追加整体豁免时改用 `--expect-cycle-revision <cycle revision>` | 登记机器检查、主 agent 复核与裁定；需要确认时留在草稿，不能封存或宣称通过。追加豁免只比较 cycle revision，带 round 的 `--expect-revision` 会返回 2；豁免作为生成块写在该轮报告的验收判定之后 |
| `phoenix-ui review apply record` | `--round <round> --input <dispositions.json> --expect-revision <applied revision>` | 更新当前轮修复处置和原记录位置的所属区块，不改历史评审 |
| `phoenix-ui review status` | `--prefix <前缀>` | 只读状态；明确区分草稿、封存、缺失、损坏和未知 schema。新前缀的父目录不存在且无历史时返回 `state: absent`，不创建目录或循环。索引损坏、schema 来自未来版本或证据漂移时仍输出 `state: unknown` 的对象，但 `ok:false` 并退出 4 |
| `phoenix-ui review import` | `--prefix <前缀> --input <legacy-map.json>` | 唯一的 Markdown 接入入口（`legacyMarkdown`），先核对映射与源 hash；生成区被手改时用 `projectionRebase` 按权威事实恢复，不导入手改内容 |

退出 0 表示本次操作成功且返回的状态有效，不表示 UI 通过；`status` 读到损坏、未来 schema 或漂移的状态时退出 4。输入/schema 错误（2）、revision/锁/历史冲突（3）、文件或环境错误（4）都先保留实际输出并处理原因；不通过改计数、删索引或换前缀绕开。

## 一轮内的证据和隔离

- `begin` 的目标快照、矩阵、scope、threshold 与被测版本在本轮固定。报告沿用 Markdown 循环的文件名：首轮 `<前缀>.ui-review.md`，之后 `<前缀>.ui-review.r<n>.md`；最新轮次以索引和 `status` 为准，不另写「最新」区块。断点恢复读原 round/revision；本轮已封存后，新的采集或评审进入下一轮。
- 采集先确认真实入口、当前构建、操作轨迹和图像内容，再登记 hash。文件存在、mtime 新或 hash 一致都不能单独证明真实性。目标图、旧截图、模拟页、合成图不能登记为当前采集。
- 同一格子的首次记录是 attempt 1，包含 missing；补采递增 attempt。相同 cell/attempt/hash 重复提交才幂等，冲突不覆写。视口 ID 同时绑定宽、高、DPR，文件名避免同宽不同高或 DPR 冲突。
- `judge-pack` 只含期望节、目标附件、当前已登记采集及交互、每单元最近一次有评审结论的覆盖轮的截图与评审自述（待复核旧差距取最新一轮仍未关闭的清单）、可选差分；目标指纹变了的单元附目标包完整的「变更记录」节，作为目标事实，条目不标单元或轮次，不做筛选。不能跟随材料链接打开代码、完整报告、修复记录、机器检查、门槛或主 agent 的看法。若旧差距材料缺失，先补材料，不让评审凭空复核。
- 评审用全新只读上下文，继承宿主模型与 effort，不再派生；主 agent 保留实际派发与开图轨迹。工具白名单不等同权限沙箱，不能只凭 packet 已生成声称信息隔离已验证。
- 评分与理由来自评审本人；转换时保留原文、hash 与转换依据，未评单元不填零分或假结果。差距的可选 `summary` 只照抄评审原文给该差距的标题；原文没有就省略，报告改用「现状」首句作标题，主 agent 不代拟。旧差距逐条复核，新临时编号显式映射至正式编号，不凭相似描述自动关闭或合并。
- 确认或补采重评按合同选择某位评审的一整组较低分及理由，差距取并集。确认评审使用同一批材料；返回 `confirmation_required` 仍是草稿，确认结果登记后再 finalize。
- `capture.md`、合并 `judge.md`、报告与处置 Markdown 是 JSON 状态的投影。每位评审原文另存且不覆盖；主 agent 不改原文、结构化评分或历史轮结果。

## 旧历史与生成区

导入前逐份读取目标、各轮报告、评审原文、截图和修复记录；明确目标指纹、差距 ID、处置、真实尝试与各范围重构的映射。保留 legacy 文件及源 hash。不能从文件名、修改时间或一次摘要猜完整历史，不能把旧图当本轮采集。

未知尝试次数、目标 hash 或重构次数不能填零，也不能领取一次新的重构机会。最后一轮旧报告所在基线中有全量轮分数未知时，历史最佳不确定，`import` 返回 3 且循环保持人工。无法唯一映射时保持人工循环，或请用户确认新基线及旧差距如何保留；不自动覆盖历史。

发现 generated 区被手改时，先另存用户原文与当前 source hash，停止覆盖；再用 `review import` 的 `projectionRebase`（另存副本、当前文件 hash 与 cycle revision 均须匹配）按权威事实恢复生成区。手改内容不经 import 导入，要保留的采集或处置以当前 revision 用 `capture add` 或 `apply record` 重新提交。封存报告正文与评审原文保持不可变，可恢复的只是其后追加的豁免生成块。CAS 冲突时回读最新 revision 和差异，不强写。

修复记录仍在首次选定的原里程碑或 applied 路径。投影只更新本循环、本轮所属区块，保留人工文字和其他轮记录；不创建「UI 整改」里程碑。用户豁免须有真实原话、日期、版本与剩余影响，追加在该轮报告验收判定之后的生成块里，不能从勾选框或修复者自述推断豁免或已解决。
