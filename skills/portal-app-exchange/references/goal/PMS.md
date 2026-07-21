# XGENT Track · AI Agent 赋能的研发项目管理系统

XGENT Track 是一个面向软件研发团队的、由 **AI Agent 深度参与**的项目管理系统（类
Linear 体验）。除了人类成员，系统内置四个可被指派 Issue 的 AI Agent：

| Agent | 角色 | 职责 |
| --- | --- | --- |
| **Atlas** · 规划 | 需求拆解 / PRD | 拆解需求、维护 PRD，依据讨论自动更新 |
| **Forge** · 编码 | 实现 / 重构 | 实现功能、重构代码、提交草稿 PR |
| **Sentry** · 测试 | 用例 / 回归 | 自动生成回归用例、发现缺陷自动建 Issue |
| **Scribe** · 文档 | 文档 / 变更日志 | 生成文档与变更日志 |

界面与交互按 `prototype/` 中的高保真原型 1:1 实现。

## 技术栈

**后端** — Bun · Elysia · Drizzle ORM · PostgreSQL
**前端** — Vite · React · TypeScript · shadcn/ui · Tailwind CSS · TanStack Query

```
xgent-ai-pms/
├── prototype/          # 原始高保真原型（React UMD + Babel）
├── backend/            # Bun + Elysia + Drizzle REST API
│   ├── src/
│   │   ├── db/         # schema / 连接 / seed
│   │   ├── routes/     # meta / issues / notifications
│   │   ├── lib/        # 序列化助手
│   │   └── index.ts    # Elysia 服务入口
│   └── drizzle.config.ts
└── frontend/           # Vite + React + shadcn + Tailwind 单页应用
    └── src/
        ├── components/ # 视图与 UI 组件
        ├── store/      # AppData context + 数据 hooks
        ├── lib/        # api 客户端 / 类型 / 常量
        └── hooks/      # 设置（主题/强调色/密度）
```

## 功能

- **Issues**：列表 / 看板双视图，按 状态 / 优先级 / 负责人 分组，看板支持拖拽改状态
- **行内编辑**：状态、优先级、负责人均可在列表/看板/详情中通过气泡菜单即时修改
- **Issue 详情**：滑出面板，含描述、子任务进度、活动流、评论、**AI Agent 工作区卡片**（实时步骤）
- **AI 指派**：把 Issue 指派给 Agent 时自动打上 `AI 生成` 标签并标记 `aiAssigned`
- **项目 / 周期 / 路线图**：项目看板（进度环）、当前周期燃尽与 Agent 贡献、甘特式路线图
- **产品生命周期 (PLC)**：项目卡片展示生命周期阶段（构思 → 开发 → 发布 → 维护 → 退役）阶梯
- **敏捷 Scrum**：
  - **产品待办列表**：按 backlog 排序，拖拽 Issue 到迭代进行 **Sprint 规划**
  - **迭代 Sprint**：Sprint 看板（4 列 + 各列故事点）、**燃尽图**（理想线 vs 实际）、**速度图**（承诺/完成 + 平均速度）
  - **故事点**：Issue 详情展示故事点；迭代统计按故事点汇总
- **收件箱**：Agent 与团队动态通知
- **命令面板**（⌘K）：搜索 Issue、跳转视图、新建
- **新建 Issue**（按 `c`）：标题 / 描述 / 状态 / 优先级 / 负责人
- **外观设置**：浅色 / 深色、蓝 / 橙强调色、紧凑 / 常规密度（持久化到 localStorage）

## 本地运行

### 前置条件
- [Bun](https://bun.sh) ≥ 1.1
- PostgreSQL ≥ 14（本地或容器）

### 1. 准备数据库
```bash
# 例：本地已有 postgres，创建数据库
createdb xgent
```

### 2. 启动后端
```bash
cd backend
cp .env.example .env          # 按需修改 DATABASE_URL
bun install
bun run db:push               # 建表（drizzle-kit push）
bun run db:seed               # 写入原型示例数据
bun run dev                   # http://localhost:3001  （/docs 有 Swagger）
```

### 3. 启动前端
```bash
cd frontend
bun install
bun run dev                   # http://localhost:5173
```

前端通过 Vite 代理把 `/api/*` 转发到后端（见 `frontend/vite.config.ts`）。

## API 速览

所有接口以 `/api` 为前缀，详见 `http://localhost:3001/docs`。

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/bootstrap` | 一次性返回 members / teams / labels / cycles / projects |
| GET | `/api/issues?team=&assignee=&project=&cycle=` | Issue 列表（含标签、子任务计数） |
| GET | `/api/issues/:id` | Issue 详情（子任务 + 活动流） |
| POST | `/api/issues` | 新建（自动生成 `TEAM-N` 编号；指派 Agent 自动置 aiAssigned） |
| PATCH | `/api/issues/:id` | 局部更新（状态/优先级/负责人/标签…） |
| DELETE | `/api/issues/:id` | 删除 |
| POST | `/api/issues/:id/comments` | 新增评论（Agent 评论记为 AI 动态） |
| PATCH | `/api/issues/:id/sub/:subId` | 切换子任务完成态 |
| GET | `/api/notifications` · POST `/read-all` | 收件箱通知 |

## 测试

后端使用 **`bun test`**（`backend/test/`）：

- `serialize.test.ts` — 序列化助手的单元测试（纯函数）
- `api.test.ts` — 集成测试，通过 `app.handle(Request)` 在进程内直接驱动 Elysia
  应用，覆盖 bootstrap / issues CRUD / 评论 / 子任务 / 通知，以及 AI 指派自动置位、
  自动编号、404、校验等行为

测试前会通过 `test/setup.ts`（在 `bunfig.toml` 中 preload）自动建表 + 写入示例数据。

```bash
cd backend
createdb xgent_test                 # 一次性
DATABASE_URL=postgres://postgres:postgres@localhost:5432/xgent_test bun test
```

> 集成测试会创建临时 Issue 并在结束时清理；建议使用独立的 `xgent_test` 库，
> 因为 `setup.ts` 会重置数据到已知种子状态。

## CI

`.github/workflows/ci.yml` 在 `push` 到 `main` 和针对 `main` 的 `pull_request` 时运行：

- **Backend** — `bun install --frozen-lockfile` + `tsc --noEmit` + `bun test`
  （CI 内置 `postgres:16` service，库名 `xgent_test`）
- **Frontend** — `bun install --frozen-lockfile` + `bun run build`（即 `tsc --noEmit` + `vite build`）

## 数据模型

`members`（人类 + Agent 同表，便于负责人外键）、`teams`、`labels`、`cycles`、
`projects`（含 PLC `phase`）、`sprints`、`sprint_snapshots`（燃尽快照）、
`issues`（含 `sprint_id` / `story_points` / `backlog_rank`）、`issue_labels`、
`sub_issues`、`activities`、`notifications`。详见 `backend/src/db/schema.ts`。

### Scrum API（`/api/sprints`）

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/sprints?team=` | 迭代列表 |
| GET | `/api/sprints/backlog?team=` | 产品待办（无迭代的 Issue，按 backlogRank 排序） |
| GET | `/api/sprints/:id` | 迭代详情 + 承诺 Issue + 统计（承诺/完成故事点） |
| GET | `/api/sprints/:id/burndown` | 燃尽数据（理想线 + 每日实际剩余点） |
| GET | `/api/sprints/velocity?team=` | 各迭代承诺/完成点 + 平均速度 |
| PATCH | `/api/sprints/:id/issues/:issueId` | 把 Issue 移入迭代（`_backlog` 表示移出） |
