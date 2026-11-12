# PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps · R1 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1，初次完整评审 | [capture 步骤失败提示补齐](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md) | `0509ba6c9012f5eb5b6a51507a28d10ad7a66601..c84f6a4d348be97caf5f61bb20ea972a6c882467` 的实现净差异；正式冻结时工作树/index 均 clean，无未提交代码 | 无；原 P1 的 R1–R4 属于另一计划，不作为本计划上轮 |

- 日期：2026-11-11，Asia/Shanghai；仓库：`/Users/rockie/Documents/GitHub/xgent/skills`，分支 `main`。
- 原始输入：`$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md`；按计划模式解析 M1 登记提交 `c84f6a4`，完整 SHA 如上，已验证在当前分支可达。
- 实际验证 checkout：`45e384462842d19ce59cf7e363cf9a8f139b3f1c`。`git diff c84f6a4 HEAD -- tools/phoenix-ui` 为空，本轮运行的工具源码、夹具和依赖与所审目标一致。
- 计划版本：checkout `45e3844` 的计划，SHA-256 `02cfedb01e16e8ac69d7cc69ee8fa48018dcaaf253799f9a4fa6c42fbbd42cfd`；[M1 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.records/M1.md) SHA-256 `7851eaba9071cff64deda4a0663826d593c72084f28567b0d6a432c0bda9b036`。相对实现提交，计划仅回填进度和提交 SHA，契约未变。

## 1. 确认问题

未发现可确认的问题。新增 findings：0；无须执行代码修复。

## 2. 待核实项与范围限制

没有阻塞本次代码验收的待核实项。以下不属于本轮结论：

- 候选定位按计划 D2/D3 只提供写法，不保证唯一，也不替调用方判断哪个候选符合任务意图；不把这一明确边界作为缺陷。
- 没有重跑四平台 CI、发布流程或真实宿主整组验收。本改动未触及平台、分发和宿主接入，这些由计划 §0.5 明确排除；本轮不宣称 npm/R2 已发布。
- 本轮核对了取证产物是否呈现目标状态，没有进行独立 UI 视觉评分，也没有重审原 P1 全部需求。
- 整包格式与 lint 的存量情况沿用 [M1 验收记录](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.records/M1.md#验收)：本期 Rust 文件无格式差异、无新增 clippy 诊断，记录同时披露 15 个无关文件的格式差异和 4 条存量诊断。本轮未重跑这两个包级命令，不将整包 fmt 描述为全绿。

## 3. 覆盖与验证

### 3.1 变更意图与范围固定

为隐藏、遮挡和禁用目标补充状态文件格式的替代定位提示，并在 `waitFor` 超时中保留主文档定位限制，不改变步骤判定或报告结构。

已完整读取本次五个实现/测试文件的 diff，再沿直接调用关系核对：

| 文件 | 已审行为 |
| --- | --- |
| [page_js.rs](../../tools/phoenix-ui/crates/cli/src/capture/page_js.rs) | `disabledBy`、`cssOf`、`suggest`；相关文字优先、同类补足、最多三项、排除自身与三类禁用状态；短 `innerText` 输出 text，其他输出转义 CSS；missing 分支保留原有筛选/顺序/输出；隐藏、离屏及警告路径接线 |
| [steps.rs](../../tools/phoenix-ui/crates/cli/src/capture/steps.rs) | `LocateError.scope_note`；隐藏/missing 的 `page_hints` 与既有 `role_hints`；`waitFor` 的轮询、出现/消失判定和超时拼接 |
| [capture.rs](../../tools/phoenix-ui/crates/cli/tests/capture.rs) | 扩展已有真实 Chrome/CLI 场景；断言报告和终端输出，保留 missing、消失超时、日期填写等原断言；未发现绑定内部实现的新增测试负担 |
| [failures.html](../../tools/phoenix-ui/tests/fixtures/capture/pages/failures.html)、[failures.json](../../tools/phoenix-ui/tests/fixtures/capture/pages/failures.json) | 保留原元素顺序，增加可见替代按钮、开放 Shadow Root 及两种等待超时状态 |

直接上下游另读 `capture/plan.rs` 的输入校验与定位序列化、`capture/mod.rs` 的步骤/元素图/mark 调用、`capture/report.rs` 的失败 hints 输出、Cargo 配置、mise 固定工具链及相关 CI 命令。定位输入已有非空校验；Rust 返回值处理未新增 unwrap/panic；schema 与终端行结构均未改动。

提交与工作树取证：

- `git rev-list --reverse c84f6a4^..HEAD` 得到 `c84f6a4d348be97caf5f61bb20ea972a6c882467`、`470d55fd8e6e518ee0b803047cd040118f5cbfa6`、`45e384462842d19ce59cf7e363cf9a8f139b3f1c`。
- `470d55f` 仅回填本计划及 M1 的提交记录，不作为代码 findings 对象。
- 开始时的未提交内容均为台账、PRD、原 P1 计划/记录与既有报告；评审期间它们由外部提交 `45e3844`（`docs: update docs and logs`，32 个文档文件）收录。已询问是否扩大范围；截至交付未收到纳入指示，本轮仅将必要 PRD/计划材料作为契约上下文，不评审该文档提交，也不扩大到原 P1。
- 正式冻结时 `git status --porcelain=v1` 为空；无 staged/unstaged/untracked 实现内容。空 index diff 的 SHA-256 为 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。空 dirty 文件 hash 映射按 UTF-8、排序 JSON、紧凑分隔符计算的 SHA-256 为 `44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a`。
- 对五个改动文件、计划/M1 记录及已读直接依赖保存 SHA-256，在裁定前复核均未变化。报告及台账回写是本次输出，不重新纳入输入。
- 主 agent 单人评审，未委派 reviewer。没有修改代码、夹具、原计划或历史评审报告。

### 3.2 实际执行与用户入口证据

环境：darwin-arm64；mise 固定 Rust 1.99.1；显式指定本机 Chrome，实际报告为 `Chrome/157.0.8103.40`。浏览器运行未使用 `sandbox-exec`。以下均在所审代码对应的 checkout 执行；构建复用已有缓存，未安装/升级依赖。

| 层级与检查 | 实际命令/步骤 | 实际结果 |
| --- | --- | --- |
| 现有场景测试，V-2 | 在 `tools/phoenix-ui` 下运行 `MISE_AUTO_INSTALL=false MISE_EXEC_AUTO_INSTALL=false PHOENIX_UI_BROWSER='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' mise exec --no-deps -- cargo test -p phoenix-ui --test capture --locked --offline step_failures_and_warnings -- --test-threads=1 --nocapture` | 退出 0；1 passed，7 filtered out，测试阶段 30.83 秒。浏览器已显式存在，实际启动真实 CLI/Chrome，非无浏览器跳过。隐藏 hints/终端、遮挡/禁用警告、iframe 与 Shadow DOM 等待超时、missing 既有候选和消失超时均通过 |
| 真实入口，V-1① | 读取 `target/debug/phoenix-ui capture --help`；从公开状态示例取登录三步与 `waitFor 任务`，第五步设为 `click #apply-filter`；运行 `DEMO_PASSWORD=<本地夹具测试值> target/debug/phoenix-ui capture tests/fixtures/capture/app --viewport desktop --states "$TMP/early.json" --out "$TMP/early"` | 按预期退出 2；第五步隐藏失败；终端依次给出 `可改为：{"text":"筛选"}`、`可改为：{"text":"新建"}`，原因保留“先执行让它出现的那一步” |
| 真实入口与亲自开图，V-1② | 按上一条提示，在第五步前插入 `{"do":"click","target":{"text":"筛选"}}`，其余不改，使用 `fixed.json` 和新输出目录重跑 | 退出 0，2 shots、0 invalid、0 failed，六个步骤均为 ok。已亲自打开 `desktop-apply-early.png`，确认“状态”“负责人”“应用筛选”面板展开，页面呈现任务列表；未只据退出码认定成功 |
| 额外有界浏览器实验，§5.1/D3 | 在隔离临时目录构造隐藏按钮、native disabled、`aria-disabled="TRUE"`、`pointer-events:none`、仅 aria-label 的按钮、长文字按钮与普通按钮，使用同一 CLI capture 入口分别触发隐藏/禁用 | 隐藏按预期失败、禁用仍 warning 并继续；两条新增路径均只列三个可用候选：转义 CSS、长文字 CSS、普通 text，三种禁用按钮均未进入候选，目标自身未入选 |
| 候选回填验证，§5.1/D3 | 将上一条生成的第一条 JSON CSS 定位原样解析并写回 `click.target`，只运行 reuse 状态 | 退出 0，1 shot、0 invalid、0 failed；包含冒号的 id 与斜杠的 class 经转义后能重新定位，无文字按钮未被错误输出成 text |

测试构建产生的 debug 二进制 SHA-256：`dbbb4bba041443d59967108291bb315138dcbb4245165f8f23314e2e96aefbb6`。上述手工场景 `$TMP` 实际为本轮隔离临时目录 `phoenix-gaps-review-ni81azre`；输入、日志和截图仅作有界验证，查看后清理，不新增仓内测试或证据框架。V-1③ 的当前版本 iframe 等待说明由上表现有 CLI/Chrome 场景覆盖，另与 M1 既有实测记录核对；没有声称重复执行一份独立的第三次手工验收。

### 3.3 已核对的反证与边界

- missing 文字为空时新 helper 跳过相关性遍历，但公开状态解析已拒绝空白文字定位；没有形成原可达输入的候选回归。
- 新增禁用过滤仅用于 `like` 非空的路径；missing 沿用旧过滤，角色定位按计划沿用 `role_hints`。不以新增文字/CSS候选规则反向要求改动既有角色算法。
- `waitFor until:hidden` 可见目标超时时 `probe` 为成功结果，不追加 scope note；missing/hidden 的 `not_visible` 判定没有变化，未把限制说明变成新的步骤失败条件。
- 动态实验覆盖生成的候选能否被实际入口再次消费；未把生成 JSON 或测试通过本身当作筛选任务完成的证明，后者另经真实重跑和开图确认。

## 4. 计划符合性

核对本计划 M1、§5.1–§5.4、V-1/V-2，不将原 P1 其他交付或后续期作为当前承诺。上游为 [PRD WORK-F1](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md#work-f1-按步骤取得多状态界面证据可供其他-skill-调用)，当前 PRD SHA-256 `09b19f65376834f46480a3dec8394e8ab1a18ba2707f0399da0e9be0caad1e25`。

| 承诺 | 实现与当前证据 | 状态 |
| --- | --- | --- |
| WORK-F1/AC-3 后半句、G-01：隐藏给出写法，遮挡/禁用警告追加写法 | `LOCATE.suggest` 与三个调用路径、`locate` 隐藏 hints；V-2 当前运行及 V-1 真实修正任务 | 已核实 |
| §5.1/D2/D3：三项上限、排除自身、三种禁用过滤、text/CSS 序列化与转义 | 静态核对两阶段遍历；当前有界浏览器实验与生成候选回填成功 | 已核实 |
| WORK-F1 边界、AC-7 等待超时、G-02 | missing 保存 scope note，超时传播；V-2 的 wait-framed/wait-shadow 均实际验证完整主文档限制说明 | 已核实 |
| C-1/C-2：判定、schema、终端行格式不变 | 隐藏仍失败；遮挡/禁用仍 warning 并执行；消失超时与 missing 原断言通过；`report.rs`/`mod.rs` 序列化消费链核对 | 已核实 |
| C-3/C-5：脚本留 Rust 常量、外部返回值缺字段按空处理 | 改动均在登记路径；未新增 crates 下 JS；新增字段使用 Option/默认值，无新增 unwrap | 已核实 |
| C-4、§0.5：工具链/分支/不发布边界 | mise、locked/offline、当前 main；未改变分支，未推送或发布，未启动四平台回归 | 已核实 |

未发现本期明确需求遗漏或越界实现；这一结论仅针对本计划所承担的 G-01/G-02，不代替整份 PRD 覆盖核查。

## 5. 总体结论与下一步

验收判定：**通过验收**。

正确性结论：`patch is correct`（限声明的实现净差异与当前已核实调用链）。未发现 P0–P3 问题；用户主路径有当前版本的真实命令、修正重跑及亲自开图证据，提示与等待限制均符合本期契约。无代码修复顺序或必需补证动作；本判定不代表合并、发布或跨期需求终验授权。

本计划代码验收完成。可选继续核对来源 PRD 的既有差异是否收口：

```text
$review-prd-dev-gaps docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md
```

该调用由用户另行发起，本轮不自动执行。
