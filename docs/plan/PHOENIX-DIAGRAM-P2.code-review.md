# Phoenix Diagram P2 · R1 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1，开发计划模式初审 | [PHOENIX-DIAGRAM-P2](PHOENIX-DIAGRAM-P2.md) | 累计净差异 `e29d4cafcada8809335e0a3dfaa939be95bb9643..8539d18c18ef2055f29aca3e866015b1bedc0b74`；登记的四个提交见下文 | 无；同目录 `.review.md` 是开发计划评审 |

计划采用观察 HEAD `0b5b37d9179682842e6c35b6cfc77f27d62c7d6d` 中的版本，SHA-256 为 `fe36f9831a7fd3b7c92d701c7cfd396e890e3efbf8f59b0de4e1222a7bf27c85`。以下代码定位均为 target 侧；这些代码在观察 HEAD 中未再改变。

## 确认问题

### F-01 · [P2] 在非法仓库 URL 的诊断中也脱敏凭据

- 定位：[src/evidence/location.rs](../../tools/phoenix-diagram/src/evidence/location.rs):225–227，target 新增代码。
- 问题：当 `meta.repository.url` 是带前导空格的凭据 URL，例如 ` https://user:example-token@github.com/a/b`，输入门会按语法错误拒绝，但诊断脱敏只识别从第一个字符开始的协议名，因此完整用户名和 token 被原样插入 JSON 诊断。宿主收到失败回执或保存输出时就会获得这些秘密；输入被拒绝并没有消除泄露。本期新增的仓库来源校验激活了这条路径，违反计划对诊断原文也不得包含凭据的要求。
- 证据：[parse_remote](../../tools/phoenix-diagram/src/evidence/location.rs):85–87 先拒绝空白字符；[错误构造](../../tools/phoenix-diagram/src/evidence.rs):254–265 随后将 `redact(url)` 插入消息；[计划 §7](PHOENIX-DIAGRAM-P2.md#7-nfr安全与运行保障):505–506 明确禁止凭据进入诊断原文。以随包 `minimal.tree.json` 为基础，添加上述 `meta.repository.url` 与 40 个 `a` 的 `revision`，实际执行最终候选的 `validate tree INPUT.json --json`：退出 1，`repository/url-invalid`，`reason=syntax`，消息仍含 `user:example-token`。去掉前导空格的对照输入会显示 `REDACTED`。实验只使用虚构凭据。
- 修复与验证：保留非法输入的拒绝行为，但在所有错误输出前对原始非法 URL 也完成 userinfo 脱敏；增加正常凭据 URL、前导空白及控制字符变体的回归，断言 stdout、stderr 和回执均不含虚构密码或 token，不能仅靠把非法输入 trim 成合法输入来消除失败。
- 置信度：高；实际原生入口已复现，正常 URL 对照排除了输入构造或普通脱敏失败。

### F-02 · [P2] 为跨年时间线保留可读年份

- 定位：[src/layout/timeline.rs](../../tools/phoenix-diagram/src/layout/timeline.rs):104–106，target 新增代码。
- 问题：普通时间戳事件跨日时，日期标签只保留月日，轴标题又省略完整日期，焦点详情也复用同一格式。输入 `2020-01-01T00:00Z` 与 `2026-01-01T00:00Z` 后，两个事件都显示 `01-01 00:00`，详情上下文也完全相同；读者无法从可见图或详情确定它们各自属于哪一年。完整时间戳虽然留在机器属性和 metadata 中，却没有可读入口，图内 SVG 的可见内容也没有年份。本期新增 timeline 在合法输入上生成并标记通过，未满足时间语义的阅读要求。
- 证据：[event_label](../../tools/phoenix-diagram/src/layout/timeline.rs):134–149 使用该标签；[轴标题](../../tools/phoenix-diagram/src/layout/timeline.rs):793–804 在 `multi_day` 时移除完整日期；[节点上下文](../../tools/phoenix-diagram/src/render/timeline.rs):88–97 继续使用年缺失的 `clock.label`；[Reader 详情](../../tools/phoenix-diagram/viewer/focus.js):398–406 直接展示这些属性，不读取原始时间戳。[PRD WORK-F1](../PRD-PHOENIX-DIAGRAM.md#work-f1-生成十种语义完整的图解):115、127、133、136 要求保留时间语义，并可从可见图与阅读详情获得。最小输入为 `schema_version=1`、`diagram_type=timeline`、`meta={title:"Release history",evidence:"observed"}`，事件为 `{id:"a",at:"2020-01-01T00:00Z",title:"Start"}` 与 `{id:"b",at:"2026-01-01T00:00Z",title:"Today"}`；最终候选 `deliver timeline INPUT.json OUTPUT.html --json` 和 `visual-check` 均退出 0、`status=pass`。提取 SVG 所有文本后没有 `2020` 或 `2026`，两节点上下文均为 `01-01 00:00:00 UTC+00:00`；已实际查看 1440×900 亮色截图确认。
- 修复与验证：跨年时在轴或事件标签中表达明确年份，并让焦点详情给出含年份的完整日期；补两个相同月日、不同年份的事件用例，检查可见 SVG、详情与导出能区分年份。参考 oracle 的格式一致性不能替代上述可读语义要求。
- 置信度：高；合法输入、产物文本、详情属性与实际截图相互印证。

### F-03 · [P2] 在 ERD 阅读界面保留列级外键目标

- 定位：[src/layout/erd.rs](../../tools/phoenix-diagram/src/layout/erd.rs):1270–1272，target 新增代码。
- 问题：ERD 列行只生成键角色、列名和类型，未把 `attributes[].references` 放入可见行或阅读详情；表级关系线也不能补出目标列。对于同一目标表同时有主键和唯一键的合法模型，将外键从主键改指唯一键后，用户看到的 SVG 和焦点信息完全不变，无法判断该列实际引用哪一个键。本期新增 ERD 因而没有完成 PRD 明确要求的“外键目标”阅读语义，原始 JSON 中保留该字段不足以满足可见图与详情验收。
- 证据：[行文本构造](../../tools/phoenix-diagram/src/layout/erd.rs):1251–1311 仅读取键角色、`name`、`type`；[ERD 节点详情属性](../../tools/phoenix-diagram/src/render/erd.rs):341–358 只提供通用 context、sublabel、tag；[Reader 详情](../../tools/phoenix-diagram/viewer/focus.js):398–406 没有读取属性列的 `references`。[PRD WORK-F1](../PRD-PHOENIX-DIAGRAM.md#work-f1-生成十种语义完整的图解):124、133、136 要求保留全部键角色与外键目标并供阅读获取。实际构造 `customer`（`id bigint pk`、`external_id bigint uk`）和 `orders`（`customer_ref bigint fk`）两个实体，使用相同的 orders→customer 多对一关系，只把 `customer_ref.references` 在 `customer.id` 与 `customer.external_id` 之间切换；两次 `deliver erd` 都退出 0、`status=pass`，提取的完整 SVG 字节完全相同，SHA-256 均为 `1941027654129afa92f2b2807f413ea0aa71336d90e1e2ea54f3503875f8e1d6`，且 SVG 中不含两个完整目标引用。
- 修复与验证：在列行或可发现的列级详情中展示已有 `references`，不要要求作者在自由文案中重复字段事实；添加上述 PK/UK 两种目标的回归，断言可见图或详情能明确辨别引用目标，且两者仍可正常检查与交付。
- 置信度：高；仅改变外键目标的对照实验及完整 SVG 比较已确认语义缺失。

## 待核实与验证缺口

1. Phoenix UI 全量回归本轮未通过。配置固定工具后，安装测试 7 项中 6 项通过，`a user-modified old skill blocks Phoenix before any project write, including the XGENT part` 在 `test/phoenix-ui/install.test.js:165` 失败：引擎探测 5 秒超时，实际退出 4，预期为冲突退出 3。后续 Rust/WASM/browser 等步骤因此未运行。该测试、`bin/xgent-skills.js`、`bin/phoenix-ui.js` 和 UI 测试脚本均不在本次 diff；尚未做基线复跑，不能归因为 P2 引入，也不能记作通过。后续需在引擎可稳定探测的环境复跑失败项及未执行部分，补齐当前版本 V-14 证据。
2. 本轮未重新 dispatch 四平台 CI、调用实时 Actions API 或开启 Claude Code/Codex 验收会话；最终包的跨平台、完整导出与宿主结果仍依赖 [M8 记录](PHOENIX-DIAGRAM-P2.records/M8.md) 及本机既有聚合结果。本轮读取了这些记录与聚合结果，但没有独立重做其全部原始证据校验。它们不能覆盖 F-01–F-03 的新增反例。

## 覆盖与验证

本次意图是补齐五种原生图型，并将十类阅读探索、固定提交来源核对、多语言与同包候选交付连通。评审日期为 2026-10-22（Asia/Shanghai），仓库根为 `/Users/rockie/Documents/GitHub/xgent/skills`，由当前单个 reviewer 执行。

### 范围与版本

- 原输入：`$$review-code docs/plan/PHOENIX-DIAGRAM-P2.md`；按计划完成记录取累计净差异，不自动扩成整仓检查。
- base：`e29d4cafcada8809335e0a3dfaa939be95bb9643`。
- target：`8539d18c18ef2055f29aca3e866015b1bedc0b74`。
- 计划登记提交：`711a5b7b44ba7b1c76dfe1c3ca2e2a3418b43ca2`（M1–M6）、`e09bc820e0c7c412ad34124cbae0ce17d9bf3805`（M6 修复与 M7）、`a8f3ec40d39f2d90217d08151b037baec7f0b328`（候选测试引擎复制修复）、`8539d18c18ef2055f29aca3e866015b1bedc0b74`（M8 收口记录）。四个提交均在观察 HEAD 可达。
- 穿插的 `0a8c6a88b62485d83a5ec45f876e4170f1f5cbc6`、`5c0528d6f7473d958560866a65c69731a89dc424`，以及 target 之后的观察 HEAD，仅修改计划 SHA 回填，不作为额外代码提交；没有发现未登记的代码增量。
- 开始时 tracked/index/untracked 均干净；观察 HEAD tree 为 `ad1c2713c5e727e6c2d69d65697c47b810cd4bd5`。落报告前对开始时全部 4,831 个 tracked 文件重新计算 SHA-256，无变化；计划 hash 同报告头。报告及随后的 SDLC 登记是本轮产物，不纳入被审 diff。
- 净差异共 217 个文件。覆盖 `tools/phoenix-diagram`、对应 skill/schema/示例、diagram 专用 scripts/workflow/测试/dist 及计划相关文档；无额外文件过滤。P3 的旧入口替换、品牌/预览/比较、安装迁移/回滚、公开发布和 Windows 均按计划排除，不记为本期缺陷。

### 静态与产物检查

审阅了输入与语义校验、五类布局/渲染/检查、共享路由及 workflow 修复、时间与数字格式、来源 URL/Git/工作树核对、交付身份/事务/回执、Reader 属性与详情/探索、装配、多语言、候选构建/消费/聚合的主要行为链和相关测试。运行测试与反例时未修改源码或计划。

生成的巨量 oracle、catalog 全部翻译条目与全部新增测试没有逐行人工核查；这部分依赖回归、装配检查与结构/身份对照，不宣称完整人工覆盖。对源 skill 与 dist bundle 做了文件比较（排除身份清单 `ENGINE.json`），没有差异；五份新 schema 与冻结参考在扣除计划允许的 `input_contract`/precision 增量后结构一致。

反例使用最终候选的 darwin-arm64 原生引擎，位置为 `local/phoenix-diagram/p2-m8-ci/39689275122/artifacts/12389786526/payload/candidate/release-dir/phoenix-diagram-darwin-arm64`；`engine-probe` 为 `0.2.0 input=1 receipt=1 viewer=phoenix-v2`，二进制 SHA-256 为 `a8a8725d5a8db3625279e969d80e527b307a854e50188737c8a56904d6cbd2a9`。重新计算构建清单的 170 个源码输入，逐项 size/hash 与当前源码相符，`sourceInputSha256=7fcabd8897c0b3b563d5b5db74b0eaeaed2b61b3c715ff75305830c7077cb20e`；反例引擎与被审源码身份一致。

### 实际执行

| 命令/检查 | 结果与适用范围 |
| --- | --- |
| `MISE_ENV_CACHE=0 mise exec --no-deps -- npm test` | 退出 0；Node 顶层 34 个测试文件通过，Python 四组分别 22/24/2/13 项通过。npm 子进程实际落到本机 Node 19，不能据此声称固定 Node 25 的维护检查通过。 |
| `MISE_ENV_CACHE=0 mise exec --no-deps -- node scripts/test-phoenix-diagram-native.mjs` | 固定 Node 25.2.1，退出 0；包含装配检查、Rust fmt/clippy、Rust 341 passed / 2 ignored、Node 209 passed / 5 skipped、Reader/上游 124 passed。脚本明确不覆盖真实浏览器与 provider 会话。此前经 `npm run test:diagram` 的调用因 npm 子进程 Node 版本不符在预检失败，改用此等价入口后完成。 |
| `WASM_OPT=local/phoenix-ui/build-tools/darwin-arm64/binaryen-version_117/bin/wasm-opt MISE_ENV_CACHE=0 mise exec --no-deps -- node scripts/test-phoenix-ui.mjs`（实际传入 wasm-opt 绝对路径） | 退出 1；构建和 critique 碰撞用例通过，安装用例 6/7 通过，失败与后续未执行范围见待核实第 1 项。第一次调用还因未提供 WASM_OPT 在预检停止；配置后才得到上述测试结果。 |
| 最终候选 `validate tree INPUT.json --json`，凭据 URL 三组对照 | 三组均按预期拒绝；带前导空格的一组凭据未脱敏，确认 F-01。 |
| 最终候选 `deliver timeline INPUT.json OUTPUT.html --json`；`visual-check OUTPUT.html --json` | 均通过；Chrome 155.0.8059.85 自动检查四个桌面尺寸与 390×844、暗亮主题共 10 组，捕获 4 张截图。实际人工查看其中 1440×900 亮色截图，结合文本/属性确认 F-02；没有把自动 pass 记为语义审核通过，也未提交 review-record。 |
| 最终候选两次 `deliver erd INPUT.json OUTPUT.html --json` | 均通过；PK/UK 外键目标不同而完整 SVG 相同，确认 F-03。 |
| tree、单事件 timeline、单实体 ERD、单跨度 waterfall 边界 smoke | 合法样本均能交付；只证明这些局部边界，不代表五类完整视觉验收。 |

临时输入、产物与运行日志位于本机 `/var/folders/_v/szrkbcq52n363ks1n4jpdwb00000gn/T/phoenix-p2-review-4zmng4ar/`（`npm-test-pinned.log`、`diagram-test-direct.log`、`phoenix-test-configured.log`、`url-*.stdout.json`、`integrity-and-timeline.json`、`foreign-key-results.json` 等）。这些是未提交的临时复现材料，可能被系统清理；上述输入条件、代码定位、命令与结果已写入报告，其他机器应按描述重建，不依赖该目录长期存在。

## 计划符合性

| 里程碑/承诺 | 本轮核查结果 |
| --- | --- |
| M1：十类输入合同、身份与共享基础；V-1/V-2 | 类型、schema、共享场景/i18n、版本/构建输入均有实现，相关回归通过。 |
| M2：ERD/class；V-3 | 原生布局、渲染与检查已接入；ERD 外键目标阅读语义缺口为 F-03，对应 WORK-F1/AC-2。 |
| M3：tree/timeline/waterfall；V-4 | 三类原生链路及格式化已实现；timeline 跨年事实不可读为 F-02，对应 WORK-F1/AC-2。 |
| M4：仓库来源核对与显示；V-5、§7 | 固定提交、Git 禁传输、工作树比较和三态表达已实现；非法凭据 URL 诊断泄露为 F-01，违反 §7 的明确安全要求。 |
| M5：十类 Reader、范围卡片、语言；V-6 | 探索属性、详情来源、目录与单测已接入；F-02/F-03 仍不能通过 Reader 详情补回语义事实。 |
| M6：阶段 A 收口；V-7/V-8/V-9 | runner 增量与既有收口记录可追溯，本轮维护回归通过；没有独立重做完整 corpus/导出/语言看图验收。新增反例说明原通过集未覆盖所有承诺事实。 |
| M7：skill、说明、包脚本与宿主；V-10/V-11 | 审阅了同步的接口文案、workflow 修复及候选/宿主证据绑定；本轮未重新执行真实宿主会话。 |
| M8：四平台候选与收口；V-12/V-13/V-14 | 最终候选身份与当前源码匹配，已读取 run 39689275122 的本地聚合通过结果；未实时复核远端全部证据。Diagram 全量回归通过，Phoenix UI 本轮回归存在探测超时与未执行部分，不能声称当前 V-14 全通过。 |

本报告不改写原计划的完成记录，也不把开发完成或既有 handoff 等同于本轮代码验收通过。WORK-N2/N3 等后续终验仍按计划留到 P3；未要求本期完成被排除的公开发布或迁移。

## 总体结论与修复顺序

验收判定：**需要修改**。

正确性结论：`patch is incorrect`。确认 3 项 P2：非法输入诊断泄露凭据（F-01），以及新图型在可见图和详情中丢失年份、外键目标两类事实（F-02/F-03）。现有维护回归和最终候选的自动检查通过均未排除这些可复现问题；另保留 Phoenix UI 回归及独立环境验收的证据限制。

建议先修 F-01，再修 F-02/F-03；分别达到凭据不出现在错误输出、跨年事件可区分年份、ERD 引用目标可从阅读界面辨别的关闭条件，补针对性回归，并核对变更影响到的引擎/Reader/候选证据。随后按本报告逐项复审。

下一步：`$apply-code-review docs/plan/PHOENIX-DIAGRAM-P2.code-review.md`。
