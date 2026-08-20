# SPMS 需求落库速查

本文所有字段/枚举/写面缺口均核自 SPMS 平台实现,当**平台契约**用。文中偶尔出现的 `apps/spms-server/...` 这类源码路径是**门户仓内的核实出处记录**——装到外部 repo 后本 repo 里没有这些文件,不要去读、不要去找;拿不准的以运行中的 SPMS MCP 面实际行为为准。

## 1. PRD 各部分落到哪

| PRD 章节 | SPMS 落点 | 怎么写 |
| --- | --- | --- |
| §0 一句话 | `projects.summary`（概述） | `project_update`(FR-236);**整段覆盖,先 `project_get` 读现状** |
| §1.1 背景 | `projects.background`（背景） | 同上 |
| §2 用户与场景 | `projects.personas`（用户与场景） | 同上 |
| §1.2 目标 | `projects.goal`（目标） | 同上;**Web 端是列表编辑器,一行一条** |
| §1.3 非目标 | `projects.nonGoals`（非目标） | 同上;**一行一条**(去向写在同一行) |
| §4 约束与前提 | `projects.constraints`（约束与前提） | 同上 |
| §5 开放问题 | `projects.openQuestions`（开放问题） | 同上;**一行一条** |
| §3 FR 条目 | `requirements`（`FR-N`) | `requirement_create` |
| §3 NFR 条目 | `requirements`（`NFR-N`,带 `category`) | `requirement_create` |
| 条目正文 | `requirements.description`（UI 标签「PRD 描述」) | 完整 markdown |
| 验收标准 | `requirements.acceptanceCriteria`（UI 标签「验收标准」) | **纯文本,一行一条** |
| TC 种子 | `test_cases`（`TC-N`) | `testcase_create(requirementKey=...)` |
| 开发计划(下游) | `plans`（`PLAN-N`)+ `plan_requirements` | dev-plan 阶段 `plan_create(requirementKeys)` / `plan_update({content})`。**PRD : 计划多对多**——可拆可合,关联键是 FR/NFR key;⚠️ **只能挂同项目的需求**,跨项目 key 报 `LIFECYCLE_MISMATCH`(平台契约) |

## 2. 枚举真值

| 字段 | 取值 | 中文标签 |
| --- | --- | --- |
| `type` | `functional` / `non_functional` | 功能性 / 非功能性 |
| `category`（**仅 NFR**,FR 留 null） | `performance` / `security` / `usability` / `reliability` / `compatibility` / `maintainability` | 性能 / 安全 / 易用性 / 可靠性 / 兼容性 / 可维护性 |
| `status` | `draft` → `reviewing` → `approved` → `in_dev` → `shipped`,或 `rejected` | 草稿 / 评审中 / 已批准 / 开发中 / 已交付 / 已拒绝 |
| `priority`（紧急度） | `urgent` / `high` / `medium` / `low` / `none` | 紧急 / 高 / 中 / 低 / 无 |
| `importance`（重要度,**与 priority 正交**） | `critical` / `high` / `medium` / `low` / `none` | 关键 / 高 / 中 / 低 / 未评估 |
| TC `status` | `draft` / `active` / `deprecated` | 草稿 / 可用 / 废弃 |
| TC `result` | `untested` / `passed` / `failed` / `blocked` | 未测 / 通过 / 失败 / 阻塞 |

**PRD 阶段一律建 `status=draft`**;`reviewing`/`approved` 是人的动作。

## 3. MCP 写面缺口(必须人工在 Web 补)

`requirement_create` / `requirement_update` 的 inputSchema 只有:
`projectId` · `title` · `type` · `category` · `priority` · `status` · `description` · `acceptanceCriteria`(update 用 `key` 寻址)。

**写不到的字段**——交付时单列一张「待人工补」清单:

| 字段 | UI 位置 | 备注 |
| --- | --- | --- |
| `importance` 重要度 | 需求抽屉 | 表里有列、Web 能写,MCP 没开 |
| `ownerId` 负责人 | 需求抽屉 | 与「执行人」(由关联 Issue 派生)不是一回事 |
| `dueDate` 截止日期 | 需求抽屉 | 需求池按日期范围筛选用 |
| `releaseId` 版本 | 需求抽屉 | 应与项目的 release 一致,否则 UI 出告警 |
| 附件 | 需求抽屉 | MCP 只能读(`attachment_read`),不能传 |
| 排期/点数 | Sprint 规划页 | 属规划期(`sprint_plan_items`),**不在 PRD 阶段做** |
| 项目治理字段(名称/状态/负责人/团队/版本) | 项目抽屉 | `project_update` 只写基本信息七段;**无 `project_create`** |

## 4. key 分配规则(建之前必须知道)

- `FR-N` / `NFR-N` / `TC-N` 都是**租户级**序列(不是项目级)——编号跨项目连续,别指望 `FR-1` 是本项目第一条。
- 前缀在**创建时**按 `type` 决定;**之后改 `type`,key 不会重写**(与 issue key 契约一致)。→ **建之前把 FR/NFR 判定定死**,建完再改类型就是永久错配。
- key 创建后才知道 → PRD 草稿用 `R1..Rn` 临时编号,**建完立刻回填**并全篇替换。
- `*_create` **不幂等**:重复调用 = 重复需求 + 烧掉 key。建之前 `pms_search` + `requirement_list` 查重,写库前把清单摊给用户确认。

## 5. 正文与验收标准的物理格式

**`description`(PRD 描述)** —— markdown 全量渲染(平台统一的 markdown 渲染组件),标题/列表/**表格**/代码块都可用。
图片只认 `![name](xgent-attachment:<id>)` 稳定引用,而 MCP 没有上传面 → **正文里不要放外链图**。

**`acceptanceCriteria`(验收标准)** —— **不是 markdown**。前端按 `\n` 切行、trim、丢空行,渲染成圆点列表,并剥掉行首的 `\d+\. `:

```
✅ 管理员在「应用市场」点击安装后，应用 3 秒内出现在左侧导航
✅ 非管理员访问该入口返回 403，且导航不显示该项
✅ 已安装应用重复安装时提示「已安装」，不产生第二条记录

❌ - 管理员可以安装应用        → 行首 "-" 会原样显示成「• - 管理员…」
❌ **重要**：安装要快          → 加粗语法原样显示，且「快」不可断言
❌ （空行分段）                → 空行被丢弃，分段无效
```

一行一条、无前缀符号、无空行、无 markdown 语法、每条可断言。

## 6. 常用调用序列

```
project_list()                              → 拿 projectId（令牌白名单内）
project_get(projectId)                      → 成员/迭代/计数/基本信息七段(summary,background,personas,
                                              goal,nonGoals,constraints,openQuestions)
project_update({projectId, summary, …})     → 回写基本信息七段(整段覆盖，先读后写；null 清空)
requirement_list(projectId)                 → 看已有需求，校准粒度 + 查重
pms_search(keyword)                         → 跨需求/Issue/TC 查重（上限 50 条，用具体词）
requirement_get('FR-18')                    → 读某条全量（含验收标准/关联 Issue/附件）

--- 用户确认后 ---
requirement_create({projectId, title, type, category?, priority, status:'draft',
                    description, acceptanceCriteria})   → 返回体里拿真实 key
testcase_create({projectId, title, requirementKey:'FR-37', preconditions?, steps, expected,
                 priority?, status:'draft'})            → TC-N，result 默认 untested
```

## 7. 错误码与闸(照实报,别绕道)

| 现象 | 含义 | 怎么办 |
| --- | --- | --- |
| `CAPABILITY_REQUIRED` | 令牌没有 `write` 能力 | 让用户在 SPMS「设置 → Agent 接入」重签带 write 的令牌(能力是签发时定死的,改不了) |
| `PROJECT_NOT_ALLOWED` | 目标项目不在令牌白名单 | 白名单**可就地改签**(令牌列表页「项目」单元格),不必吊销重签 |
| `PROJECT_NOT_FOUND` / `REQUIREMENT_NOT_FOUND` | 不存在,或跨租户(不泄露存在性) | 核对 projectId / key |
| 403（需求写闸） | 需要 `spms:action:requirement.manage` **或**本项目 Lead | 报出缺什么,请用户授权;不要改去建 Issue 绕行 |
| `VALIDATION_FAILED` | 标题为空等 | 修入参 |
| `429` / `503` | 令牌 rpm 超限 / 门户不可达(fail-closed) | 退避重试;503 时门户侧先恢复 |

> 需求状态变更会通知需求作者 —— 别为了「整理」批量翻状态,会刷屏。
