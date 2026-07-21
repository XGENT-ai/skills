# CLAUDE.md

<!-- gomad:start -->
Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.
<!-- gomad:end -->

## Project Conventions

### 本地开发: 一次起齐用 `bun run dev:all`

本地起开发栈,**只用 `bun run dev:all`** —— 它用 concurrently 一把起齐 api(:3000)+ web(:5300)+ 6 个独立后端(files/spms/sms/qbank/lms/llm-gateway,:4100–4600)+ 8 个 micro-app 前端(:5301–5308)。

- **不要**把单个 `dev:spms` / `dev:sms` 等和 `dev:all` 叠着跑,也不要重复跑。Vite 端口被占时会**自增**抢相邻端口(如第二个 spms-app 抢掉 sms-app 的 :5305),导致某个 App 的 iframe 加载到**另一个** App。要重起先把旧的全停掉(清掉 :3000/:5300/:5301–5308/:4100–4600 的监听)再 `dev:all`。
- 某个 App「打不开 / 服务正在部署 / 未配置嵌入地址」,先查**它的 server+app 进程在不在**(`curl /svc/<key>/health`),多半是没起,而不是代码问题。
- 按需部署(deploy-controller)是**生产**组件,本地不跑;本地服务常驻。**部署门现在在非生产(`!isProd()`,即本地 dev:all / 一盒)自动把可部署 App 视作 `ready`**(`apps/api/src/modules/deploy/service.ts` 的 `getAppRuntime`),所以本地**不会**再卡「服务正在部署」门、也无需手动把 `app_service_deployments.status` 置 `ready`;生产(`NODE_ENV=production`)仍走真实门(controller 拉起后端再翻 `ready`)。
- `db:seed`(仅 dev/本地,**破坏性**;生产走 `bootstrap:prod`)现已串起完整链:先**重建门户**(`apps/api` 的 portal-only seed),再依次跑 `qbank` → `lms` → `llm-gateway` 三个独立非破坏性种子(顺序固定:`lms` 要给 `sms`/`qbank` 清单补 `lms.read` 并接 `qbank→lms` 兑换,故 `qbank` 必须先于 `lms`)。所以单跑 `bun run db:seed` 就能让这三个 App 回到应用市场,一般无需再单独补种(`db:seed:qbank` / `db:seed:lms` / `db:seed:llm-gateway` 仍保留,可单独跑;这仨 root 脚本现以 `bun --filter @xgent/api db:seed:<x>` 调命名脚本 —— 旧的 `--filter … run <file>` 形式在 bun 1.3.x 下会报「No packages matched the filter」)。
- 想连各**独立后端的「每租户演示数据」**(per-tenant bootstraps)一起重建,跑 **`bun run db:reseed`**(`apps/api/scripts/reseed-all.ts`):= 链式 `db:seed` + 解析新租户 UUID + 按依赖序对 晨光(sunrise)/星网(starnet) 跑 `lms`→`sms`/`qbank`→`spms`→`llm-gateway` bootstrap(DB 直写,只需 Postgres)+ `files:bootstrap`(走 HTTP,需 `dev:all` 在跑,否则**自动跳过**并提示)。完事跑 `bun run verify:all` 冒烟(7 suites,含 sms/qbank 经 token-exchange 读 LMS)。

### API: HTTP status code 只反映传输/路由层, 不反映业务状态

不用 HTTP status code 表达业务状态。

- 404 = API endpoint 不存在 (路由层), 不表示"数据不存在"。数据不存在用 200 + 业务字段 (例 `data: null`)。
- 所有业务成功统一返 200, 包括新建、更新、删除、覆写。不用 201 / 204。
- 业务失败 (校验不过 / 状态非法 / 资源冲突等) 也走 200 + 响应体里的错误结构, 不用 4xx。4xx/5xx 只留给参数不正确、未登录、未授权、真传输/服务层故障 (路由不存在、网关挂、超时)。

### 数据库: 字典表统一带 `sort` 排序字段

所有**字典表**(租户级可维护的枚举/分类列表 —— 学段/年级/科目/学期/出版社/册次/课标/教材版本、题库标签、班级类型/学年 等)**必须**带一个排序字段,规范固定为:

- 列名固定 `sort`,类型 `integer NOT NULL DEFAULT 0`(drizzle: `sort: integer('sort').notNull().default(0)`)。**不要**再用 `seq` 或别的名字。
- 列表查询统一 `orderBy(asc(t.sort), asc(t.name))` —— `sort` 升序为主、`name` 升序兜底(同 `sort` 值时输出稳定)。
- 创建/更新接口都接 `sort: t.Optional(t.Number())`;创建缺省 `sort: body.sort ?? 0`,更新走 `if (body.sort !== undefined) patch.sort = body.sort`。
- 前端类型/接口/表单字段一律叫 `sort`(别再出现 `seq`),展示标签可复用 i18n `dict.field.sort` / `common.sort`(「排序」)。
- **这条对以后所有新建字典表强制生效。**
- **例外 —— 树/兄弟次序表不算字典表,保留 `seq`**:课标条目树(`curriculum_items`)、教材章节树(`textbook_chapters`)、子母题(`questions.seq`)用 `seq` 表「同一父节点下的兄弟排序」,语义是序号不是字典排序,**不要**把它们改成 `sort`。

现状(全部已对齐):LMS `stages/grades/subjects/terms/publishers/curriculum_standards/textbook_editions/volumes`、题库 `tags`、学校管理 `class_types/school_years` 均带 `sort`。原先 LMS `terms`、学校管理 `school_years` 用的 `seq` 已迁移为 `sort`(lms-server `0003_dict_sort`、sms-server `0006_dict_sort`)。

### 前端 UI 开发: 设计用 impeccable, 验证用 Chrome extension

涉及前端 UI (页面/组件/布局/交互/视觉) 的开发, 强制两步:

- **设计阶段**用 `impeccable` skill 提升设计质量 (视觉层级、信息架构、间距对齐、配色、动效、可访问性、空状态/错误态等)。不要靠"凭感觉写 Tailwind"出 UI。
- **验证阶段**用 Chrome extension (`mcp__claude-in-chrome__*`) 在真实浏览器里跑一遍, 看到改动生效、走通主路径和关键边界, 再报告完成。类型检查/单测只验证代码正确, 不验证功能正确; UI 改动只看 diff 不算验证完。
- 如果环境跑不起来 (dev server 起不来、没法连浏览器), 显式说明"未在浏览器中验证", 不要默认声称已完成。

## Design Context

This project carries dedicated design docs. Read both before any frontend UI work (the `impeccable` skill loads them automatically):

- **[PRODUCT.md](PRODUCT.md)** — strategic context. Register: **product** (authenticated app shell). XGENT.ai Portal is a general-purpose SaaS platform framework (底座) for hosting micro-apps, role-based for both end-users and platform admins. Domain-neutral: the education sample data in `prototype/` is demo flavoring only.
- **[DESIGN.md](DESIGN.md)** — the visual system (colors, typography, elevation, components) extracted from `prototype/`. Tokens live in `prototype/assets/colors_and_type.css`; primitives in `prototype/assets/atoms.jsx` and `shell.jsx`.
