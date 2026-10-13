# 全新上下文验证

此文件用于维护两项 UI skill，普通任务不加载。下列是自然请求和评测侧判据，尚未执行；不能作为 installed-provider、真实 native CLI 或具体产品 UI 的通过记录。

## 源码 description 路由

给候选会话两份当前源码 description 与原请求，请它只返回 `review-ui`、`apply-ui-review` 或 `none`。不加载正文，不给期望答案或失败信号；这只验证这两个候选间的选择，`none` 不证明其他 skill 被正确执行。全文与前 100 字符分别检查，若内容相同则记录这个事实。

| ID | 原请求 | 评测侧期望 |
| --- | --- | --- |
| T1 | 订单页已经实现了，我想按目标图验收并列出下一轮能修的差距。 | review-ui |
| T2 | 刚按上一轮意见修完登录页，请对当前页面和目标图做一次复审。 | review-ui |
| T3 | 按这份 UI 验收报告处理 UI-04 和 UI-07，修完留证据交回复审。 | apply-ui-review |
| T4 | 这份 UI 报告要求重构 U3，请核实后修这一单元，其他部分保持范围。 | apply-ui-review |
| T5 | 帮我评审这次代码 diff 的并发和错误处理问题。 | none |
| T6 | 我想给新产品找几种视觉方向和参考，现在还没有界面实现。 | none |
| T7 | 从现有页面提炼颜色和字体规范，更新 DESIGN.md。 | none |
| T8 | 没有 UI 评审报告，直接帮我实现一个新的订单列表页。 | none |

## 正文执行

每题显式要求读取待测源码 `SKILL.md` 及适用引用；只提供原请求与隔离项目材料。材料可包含真实产品/文档，或明确标记的协议合同 fixture；后者只能验证指令边界，不能冒充真实 CLI 返回或 native acceptance。不要向候选提供本表判据。

| ID | 原请求与材料 | 评测侧观察 |
| --- | --- | --- |
| E1 | 使用 review-ui 核查订单页是否具备验收材料；目前应用无法启动，没有 Phoenix CLI，先读计划、partial DESIGN.md 与 PRD，暂不保存。无 managed 索引。 | 读取原文，区分已知与 partial；不凭代码/建议计分，列真实采集缺口 |
| E2 | 使用 apply-ui-review，先核实这份人工 UI 报告，不改文件、不提交。报告、目标、接口和 CSS 可读，页面无法运行。 | 直接读原文；CSS 不能证明不采纳或已解决；AND 接口不被发明成 OR |
| E3 | 使用 review-ui 续接这个循环，先补上 r3 未采到的格子。材料载明 managed 索引已存在，固定 engine 不可用。 | 停在只读与未登记素材；不重新 begin、人工写分或猜版本 |
| E4 | 使用 apply-ui-review 按本轮报告修 UI-04。材料载明 managed 索引存在，固定 engine 不可用。 | 先读状态/恢复依赖，不改产品、目标或处置 |
| E5 | 请按 review-ui 合同核查这次评审材料与下一步；两位评审对同一单元给出交叉维度分，旧轮未覆盖该单元，材料有完整报告链接。只读。 | 保留整组低分与全部差距；最近覆盖轮而非最新轮；不把完整报告/修复信息传给评审；不把 fixture 分数宣称为产品验收 |
| E6 | 使用 apply-ui-review 接续旧循环。旧报告发过重构预警，一份修复记录缺失；这次先核实范围，暂不实现。 | 历史不明不当零；不领取新重构次数，不改目标或旧评审 |

## 可复现执行与留证

先保存实际 `codex exec --help` 和 `codex --version`，确认当前 runner 支持以下形状；参数变化时按实际帮助调整。每题新启动，不使用 resume/fork，不设置 model、effort、provider、HOME、CODEX_HOME 或个人配置覆盖。

```sh
codex exec --ephemeral --sandbox read-only --skip-git-repo-check \
  --cd /绝对路径/隔离题目 --json --color never \
  --output-last-message /绝对路径/证据/final.txt - \
  < /绝对路径/证据/input.txt \
  > /绝对路径/证据/trace.jsonl \
  2> /绝对路径/证据/stderr.log
```

只读题用 `read-only`。确需实现/保存的题改用 `workspace-write`，只让新建 case fixture 可写；保存前后路径、hash 与 diff，检查未写共享源码或其他题。设置评测侧合理超时并保存首次退出码；超时、认证或 runner 故障保留完整首跑，不重试刷分，确认子进程退出与文件静止后才结束该题。

每题保存源码路径/hash、原输入、runner/provider 版本、实际模型与 effort、完整 JSONL 工具轨迹、原始输出、退出码及耗时。预期答案在评测侧逐题核对，分别记 pass、fail、unrun、incomplete 或 open-materials；明确执行了原文阅读、真实开图、子评审、CLI、写入中的哪些。没有最终回复或未读到所需材料不能算通过。

源码与引用的哈希在整轮前后核对不变。受维护者已读判据影响的开放材料结果单独标注；协议 fixture、description 路由、正文解释、真实 CLI 与安装后 provider 会话分别记录，不互相替代。

源码修订后保存旧源快照与所有首跑，再为受影响场景建立带新版本/hash 的独立变体；不覆盖原题、首跑或结果。全新无旧历史且匹配 engine 的实际 begin，以及真实 capture/packet/judge/finalize/apply/import 恢复，须等可用的真实 CLI 后另验，不能用合同 fixture 或 stub 代替。
