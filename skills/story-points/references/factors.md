# 因子表与档位刻度

> 这份文件是本 skill 的**判定标准**。所有规则的唯一设计目标是:**三个人拿同一个输入包,数出同一个因子向量。**
>
> 因此规则一律写成「命中当且仅当清单里出现 X」的形式。凡是需要"感觉一下体量"的措辞,都是缺陷,发现即修。

## 1. 输入包(判定的全部依据)

| 输入 | 必需 | 来源 |
| --- | --- | --- |
| 需求 key + 正文 + 验收标准 | ✅ | `requirement_get` / `issue_get` |
| **影响文件/工作区清单**(已批准的开发计划里的「增量清单」,或调用者显式给出) | ✅ | `goal/<代号>.md` §1 |
| 仓库 commit | ✅ | `git rev-parse HEAD` |

**判定只看这三样,而且只看字面(§2.0)。** 需求正文(`description`)**整段都不是判定依据**——"这个改动挺大的""要重构一遍"这类形容词固然不算,连正文里写着"幂等""并发"也不算。文本侧唯一被用到的是**验收标准里逐字出现的关键词**(F7 的关键词表)。

缺影响文件清单 → **不估点**,报「输入不足」。(纯文档型交付走 §5,不需要清单。)

## 2. 因子(命中即 +1)

### 2.0 判定的字面性原则(先读这条,它管着下面九条)

**只看字面,不做推断。** 一条因子命中,必须能指着输入包里的**一处原文**说"就是它":

- **清单侧** = 影响文件清单里的**路径**或**它自带的说明文字**;
- **文本侧** = 需求的**验收标准**(`acceptanceCriteria` 那一段,逐行的那些)。

三条硬规则:

1. **PRD 正文(`description`)不是判定依据。** 正文是背景叙述,里面出现的词(哪怕写着"幂等""并发")**一律不计命中**——判据只认验收标准。
2. **语义等同不算命中。** 「不重复发送」「只触发一次」「同时点两下」都**不等于**关键词出现。要么逐字命中关键词表,要么不命中。
3. **推断不算命中。** 「这种改动大概要开事务」「应该会加索引」——**不计**;清单粒度不足以判定时走 §3 的 `unknown`。

这三条是复现性的地基:同一份输入,三个人**指着同一处原文**才会数出同一个向量。

| 因子 | 命中当且仅当(对照影响文件清单) | 明确**不**命中的情形 |
| --- | --- | --- |
| **F1** 后端逻辑改动 | 清单含任一 `apps/*-server/src/routes/**` 或 `apps/*-server/src/lib/**` 的新增/修改 | 只改 `src/db/schema.ts`;只改前端;只改脚本 |
| **F2** DB 迁移 | 清单含任一 `drizzle/**.sql` **新迁移文件** | 只改 `schema.ts` 而无 `.sql`;往既有 jsonb 列里加键(**不是迁移**) |
| **F3** 新页面或新可复用组件 | 清单含 `apps/*-app/src/pages/**` **新文件**,或 `src/components/**` / `packages/portal-ui/**` **新增导出组件** | 修改既有页面/组件(哪怕改很多);新增的是 `lib/**` / `store/**` 文件(那是工具不是组件) |
| **F4** 三语 i18n > 40 键 | 清单里 i18n 文件的**新增键数 > 40**(三语算**一**键) | 清单没给键数 → 见 §3 `unknown`;≤40 键 |
| **F5** 跨 ≥2 个工作区 | 清单涉及 ≥2 个 `apps/*` 或 `packages/*` 目录 | 全部改动都在同一个 `apps/<单个包>` 内(前后端同包也算一个) |
| **F6** 触及平台底座或跨 App 契约 | 清单含 `apps/api/**` 或 `packages/*`,**或**改动 scope / 应用清单(listing) / `exchangeTargets` / 服务账号(SA)声明 | 只改某个 `*-server` 自己的接口;只改 `docs/**` |
| **F7** 并发·幂等·事务·安全不变量 | **二选一,均以字面为准**:<br>**(a) 清单侧**——清单的路径或说明文字里出现 `tx` / 事务 / `for('update')` / advisory lock / 唯一索引 / 鉴权闸(`requirePerm` / gate / 权限闸)的**新增或修改**;<br>**(b) 验收标准侧**——`acceptanceCriteria` 逐行文本里**逐字出现**下列关键词之一:**并发、幂等、竞态、原子、fail-closed、越权、403** | **正文(`description`)里出现这些词不算**(§2.0-1);**语义等同不算**——「不重复发送」「只触发一次」「恰好一次」「去重」都**不命中**,除非同一段文字里逐字出现了关键词表里的词(§2.0-2);「大概要开事务」这类推断不算(§2.0-3) |
| **F8** 存量数据迁移 / 回填 | 清单含**回填脚本**,或迁移文件里有 `UPDATE` / `INSERT ... SELECT` | 迁移只有 `ALTER TABLE ADD COLUMN`;运行期的 reconcile 逻辑(不是一次性回填) |
| **F9** 新增或改写 ≥2 个 verify 套件 | 清单含 ≥2 个 `apps/*/scripts/verify-*.ts` 的新增/修改 | 只动 1 个套件;只跑既有套件不改 |

### 命中数 → 点数

```
0 → 1    1 → 2    2 → 3    3–4 → 5    5–6 → 8    ≥7 → 13
```

档位是斐波那契,**没有 4 分、没有 6 分**,也不允许"5 到 8 之间取个 6.5"。

## 3. `unknown` 处理(强制)

某条因子按 §2 **无法判定**(清单粒度不够、文件名给不出结论、i18n 没给键数)时:

1. **不计命中**(不许用"大概会改"补);
2. 把该因子号写进输出的 `unknown[]`;
3. **`unknown` 非空时,点数标注为「下限」**,并在汇报里写清楚缺什么信息才能定。

`unknown` 里的因子**不出现在 `evidencePaths`**——没有证据才叫 unknown。

## 4. 输出结构(逐字段比对用)

```json
{
  "points": 5,
  "matchedFactors": ["F1", "F5", "F7", "F9"],
  "unknown": [],
  "evidencePaths": {
    "F1": ["apps/spms-server/src/routes/requirements.ts"],
    "F5": ["apps/spms-server/**", "apps/spms-app/**"],
    "F7": ["apps/spms-server/src/lib/entities/sprints.ts (FOR UPDATE 固定锁序)"],
    "F9": ["apps/spms-server/scripts/verify-requirements.ts", "apps/spms-server/scripts/verify-scrum.ts"]
  },
  "anchorCompared": "FR-142=5"
}
```

`matchedFactors` 与 `unknown` **升序**排列(否则"集合相同"要靠人眼比对)。

**`evidencePaths` 的写法**:路径一律**完整写全**(`apps/spms-app/src/components/X.tsx`,不是清单里的简写 `apps/spms-app/components/X.tsx`)。计划里标 ★新 的文件**估点时可能还不存在**——照写并标注 `(★新)`;其余路径必须在仓库中真实存在,写错就是虚构证据。

## 5. 纯文档 / skill 类交付物(没有影响文件清单)

不走 §2,走三因子——**每条都在估点当时可判定**(不含"几轮实测才稳定"这类只有做完才知道的未来信息):

| 因子 | 命中当且仅当 |
| --- | --- |
| **D1** | 需要**新定义**一套可复现的判定标准(而非复述既有约定) |
| **D2** | 需要通过 MCP **写回并复核闭环**(不只是产出文本) |
| **D3** | 验收含「多次 / 多会话运行结果一致」这类**须实测复现**的断言 |

```
0 → 1    1 → 2    2 → 3    3 → 5
```

## 6. 判定溢出规则

因子表用精度换复现性(见 `goal/SDLC-SKILLS.md` 取舍 a):一条"改动很绕但只碰一个文件"的需求会被低估,这是**已承认的代价**。

**缓解**:因子给出的档位与你的直觉差 **≥2 档**时(如因子说 2 分、直觉说 8 分),**必须报出分歧**,写清"因子命中 N 条 → X 分;直觉 Y 分,理由是……",让人裁决。

⛔ **不允许**自己偷偷调档。调了档,复现性当场归零。

## 7. 锚定方法(换项目必做)

故事点是**相对值**。本项目的刻度**跨项目不可搬**——同租户另一个项目同期的一周吞吐是 100/174/125,与下面这一组差了一倍。

**怎么给一个新项目重新锚定**:

1. 取该项目 **3–5 条已排期、且点数经人工确认**的需求(优先取 `shipped` 的——点数已被交付结果验证过);
2. 对每条**回填输入包**(找到它的开发计划 §1 增量清单),按 §2 数出因子向量;
3. 比对因子档位与人工点数:**多数吻合**则刻度成立;**系统性偏高/偏低**则调整档位映射的边界(而不是调整某一条的结果),并把新映射写进这一节;
4. 把这组锚点连同它们的因子向量记录下来,后续估算的 `anchorCompared` 指向它们。

### 本项目现有锚点(项目「XGENT.ai 平台基座」`cbbaff8ef15d5d289bf7f7ad`)

| 需求 | 人工点数 | 状态 | 因子向量(按 §2 复算) | 因子档位 |
| --- | --- | --- | --- | --- |
| FR-161 需求池加 Sprint 筛选 | **1** | shipped | (无命中) | 1 ✅ |
| FR-142 需求拆解 + 排期互斥 | **5** | shipped | F1 F5 F7 F9 = 4 | 5 ✅ |
| FR-143 Issue 提出人 reporterId | **5** | shipped | F1 F2 F5 F8 F9 = 5 | 8 ⚠️ |
| FR-141 SPMS 角色与职能(ACL 收口) | **8** | reviewing | F1 F5 F6 F7 F8 F9 = 6 | 8 ✅ |
| FR-208 研发生命周期指引页 | **3** | draft(已排期) | F3 F4 = 2 | 3 ✅ |

⚠️ **已知偏置(如实记录,不掩饰)**:FR-143 因子给 8、人工给 5,**差 1 档**——低于溢出规则的 ≥2 档阈值,所以不触发分歧上报。成因是「新迁移 + 迁移内回填」会同时命中 F2 与 F8(双计)。**本期不改规则**:改成"F8 只在独立回填脚本时计"会让另一类需求失真,而 1 档偏差在斐波那契刻度里属于可接受噪声。**下次重新锚定时(§7 第 3 步)优先复查这一条。**

注:FR-141 当前 `reviewing`、FR-208 当前 `draft`,两条的点数都还没被交付结果验证过,用作锚点时心里有数。**五条锚点里四条吻合**,刻度成立;唯一偏差 FR-143 见上。

## 8. 吞吐锚点的适用边界

一周迭代 committed **70 / 39 / 55**(该项目 Sprint 1/2/3)——**只属于这一个项目/团队**,且同项目内也会漂移(当前 Sprint 4 committed 149)。

所以:**历史吞吐只用于校准点数刻度,永远不用于推断容量。容量口径以该 Sprint 自己的 `capacity` 为准**(`capacity` 为 `null` 时容量闸不成立,见 SKILL.md 第 4 步)。

## 9. Golden fixtures(可直接复跑的判定样例)

每条给出**输入包 → 因子向量 → 点数**。复现性校验时直接拿这几条断言,不依赖"跑三次碰巧一致"。

⚠️ **本节是校准样例,不是输入源。** 两条硬约束(实测踩出来的,见下):

1. **估点时不得从这里抄影响文件清单。** 清单只能来自本次调用者给的,或本次真去读的开发计划文件。fixture 里存的是**历史快照**——commit 不同、代码早变了;更要命的是,拿它补齐输入等于绕过了「输入不足就停」那道闸。**同一条需求出现在本节里,也照样要重新取输入包。**
2. **做「三会话复现性」复测时,输入必须选一条不在本节里的需求**——本节每条都自带答案,拿它去测就是让被测者读答案,测出来的一致是假的。

> 约束 1 是实测发现的:一次「只给需求名、不给清单」的负例测试里,被测会话没有按规则停手,而是从 FIXTURE-5 里取出 FR-215 的存档清单估出了 8 分。它如实说明了来源,但闸已经被绕过——**规则随即改成现在这样**。

### FIXTURE-1 · FR-161(1 点)

输入包:`apps/spms-app/src/components/RequirementsView.tsx`(改筛选行)、`apps/spms-app/src/store/sprints.ts`(改 `useProjectSprints`)、i18n 新增 2 键;服务端零改动。

```
matchedFactors: []        F1 ✗ 无 server;F3 ✗ 只改既有组件;F4 ✗ 2 键;F5 ✗ 只有 spms-app
unknown: []               points: 1        anchorCompared: FR-161=1
```

### FIXTURE-2 · FR-142(5 点)

输入包:`goal/PMS-6.md` §4 —— `apps/spms-server/src/routes/requirements.ts`(decompose 新端点)、`src/lib/entities/{requirements,sprints,issues}.ts`(排期互斥 helper + FOR UPDATE 固定锁序 + 前缀 advisory lock)、`apps/spms-app/{RequirementsView,scrum/_shared,scrum/BacklogView}.tsx` + i18n、`scripts/verify-requirements.ts` + `verify-scrum.ts`;无迁移。

```
matchedFactors: [F1, F5, F7, F9]        F2 ✗ 无迁移;F3 ✗ 无新组件文件;F6 ✗ 未出 spms;F8 ✗
unknown: []                             points: 5        anchorCompared: FR-142=5
```

### FIXTURE-3 · FR-141(8 点)

输入包:`goal/PMS-6.md` §3 —— `apps/api/src/modules/acl/manifests.ts`(+5 action +4 角色模板)、`apps/api/src/db/provisioning.ts`(`refreshSpmsAclManifest` + 逐租户 `reconcileTenantRoles` 存量刷新)、`apps/spms-server/src/{routes/catalog,routes/projects,lib/gate,lib/assignments,lib/portal-baseline}.ts`(requirePerm 收口)、`apps/spms-app` 按钮门控 + `lib/perm.ts`★新、`verify-{acl,catalog,assignments,mcp}.ts` 四套件;零迁移。

```
matchedFactors: [F1, F5, F6, F7, F8, F9]
  F6 ← apps/api/** + ACL 清单;F7 ← 鉴权闸新增 + 验收标准出现 403/越权;F8 ← 存量角色快照回填
  F2 ✗ 零迁移;F3 ✗ perm.ts 是 lib 不是组件
unknown: []                             points: 8        anchorCompared: FR-141=8
```

### FIXTURE-4 · FR-214(8 点 · 见 §10 的实证)

输入包:`goal/ACL-2.md` §1 —— `packages/shared/src/acl.ts` + `packages/portal-sdk/src/index.ts`、`apps/api/src/modules/acl/service.ts` + `apps/api/src/modules/token/index.ts` + `market/*`、`apps/web/src/pages/admin/RoleMatrix.tsx`、`apps/spms-server/src/lib/gate.ts` + `lib/relations.ts`(★新) + 10 个写闸、`verify-acl` + `verify-mcp`;**零 DB 迁移**。

```
matchedFactors: [F1, F5, F6, F7, F9]
  F2 ✗ 零迁移 —— role_grants.condition 是既有 jsonb 列(apps/api/src/db/schema.ts:801)
  F3 ✗ relations.ts 是 lib;RoleMatrix.tsx 是改既有组件      F8 ✗ reconcile 是运行期逻辑,非一次性回填
unknown: []                             points: 8        anchorCompared: FR-141=8
```

### FIXTURE-5 · FR-215(8 点 · **F7 边界的判例**)

输入包:`goal/ACCEPT-1.md` §1 —— `apps/spms-server/drizzle/0019_verify_env.sql`(★新)、`db/schema.ts`(5 列 + 1 索引)、`lib/entities/requirement-all-done.ts`(★新)+ `entities/{issues,requirements,sprints}.ts` + `lib/change-notify.ts` + `lib/serialize.ts` + `routes/{issues,requirements}.ts`、`apps/spms-app/src/components/VerifyEnvHint.tsx`(★新,4 入口共用)+ 既有组件改动 + i18n 7 键 ×3、`verify-{notify,issues,requirements}.ts` 三支;`apps/api` / `apps/web` 零改动。

```
matchedFactors: [F1, F2, F3, F5, F9]
  F3 ← VerifyEnvHint.tsx 是 src/components/** 下的新增导出组件(★新文件,不是改既有组件)
  F4 ✗ 7 键 ≤ 40      F6 ✗ apps/api 零改动、无 packages/*、明写零 MCP 面变化      F8 ✗ 迁移只加列与索引
unknown: []                             points: 8        anchorCompared: FR-141=8
```

⚠️ **F7 在这一条上不命中,这正是 §2.0 存在的原因**:该需求的 **PRD 正文**写了「幂等:持续处于全完成状态时不重复发送」,但**验收标准**逐行文本里**没有**「幂等/并发/竞态/原子/fail-closed/越权/403」中的任何一个词,清单侧也没有事务/锁/唯一索引/鉴权闸。
按 §2.0 的三条硬规则:正文不算(规则 1)、「不重复发送」这类语义等同不算(规则 2)→ **F7 不命中**。
**这个边界是被实测逼出来的**:F7 早期措辞含糊时,三个独立会话跑同一份输入,两个数出 5 条因子、一个数出 6 条(多记了 F7)——点数碰巧都落在 8,但因子向量不一致,**逐字段断言不通过**。收紧措辞后复测一致。

### FIXTURE-6 · FR-208(3 点 · **三会话实测一致的那一条**)

输入包:`goal/SDLC-UI.md` §1 中属于 FR-208 的部分 —— `apps/spms-app/src/components/GuideView.tsx`(★新指引页)、`components/Sidebar.tsx`(导航项)、`lib/types.ts` / `lib/route.ts` / `App.tsx`(视图分发各一支)、`lib/i18n.ts`(指引页 **50 键 ×3**)、`scripts/verify-routes.ts`(扩表);`apps/spms-server` / `apps/api` / `apps/web` 零改动。

```
matchedFactors: [F3, F4]
  F3 ← GuideView.tsx 是 src/components/** 下的★新文件      F4 ← 50 键 > 40(三语算一键)
  F1 ✗ 无 server 改动      F5 ✗ 只有 apps/spms-app 一个工作区      F9 ✗ 只动 1 支 verify 套件
unknown: []                             points: 3        anchorCompared: FR-161=1
```

✅ **这条是被三个互相独立的会话各估一次、逐字段比对通过的**(`points` / `matchedFactors` / `unknown` 三项全同),且与人工历史点数 **3** 吻合。

### FIXTURE-7 · 文档型:本 skill 自身(5 点)

输入包:`goal/SDLC-SKILLS.md` —— 交付 `.claude/skills/story-points/` 与 `test-plan/`,`apps/**` 零改动。走 §5 三因子。

```
matchedFactors: [D1, D2, D3]
  D1 ← 新定义因子表;D2 ← 经 sprint_plan_items 写回并复核;D3 ← 验收含"三会话结果一致"
unknown: []                             points: 5
```

## 10. 实证:客观因子的价值在于**误判可被事实推翻**

用这张表估 `docs/PRD-SDLC.md` 的 9 条需求时,**FR-214 当场出过一次误判**:

1. 初判凭「ACL 改动总得改表」的直觉,把 **F2(DB 迁移)记为命中**,档位偏高;
2. 核实:`goal/ACL-2.md` §1 明写「**零 DB 迁移**」,`apps/api/src/db/schema.ts:801` 显示 `role_grants.condition` 本来就是 `jsonb` 列——**往既有 jsonb 列里加一个键不是迁移**;
3. 撤销 F2 命中,落回 **8 点**。

**这就是因子表相对主观刻度的全部价值**:"体量中等"没法被推翻,"清单里有没有 `drizzle/**.sql`"可以——而且是任何人都能在 10 秒内复核的那种可以。
