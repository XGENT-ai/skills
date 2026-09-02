---
name: xgent-init
description: 在一个 XGENT.ai Portal 出仓 App 自己的仓库里生成配套的 CLAUDE.md / PRODUCT.md / DESIGN.md —— 读 app.manifest.json 与代码事实、一次性把缺的问清楚、按模板填出可直接用的三份文档，不留任何待填占位，已存在的文件不覆盖。凡任务是「初始化/接入一个新的出仓 App 仓库」「给这个 App 仓补上 CLAUDE.md / PRODUCT.md / DESIGN.md」「补一份 impeccable 能读的设计文档」，或用户刚拿到一个空的/只有代码没有规范的 App 仓时使用；门户 monorepo 内的 App、非 XGENT 项目不用本 skill。Use in an external XGENT portal app's own repo to scaffold its CLAUDE.md / PRODUCT.md / DESIGN.md from the app manifest and repo facts — one round of questions, no leftover placeholders, never overwrites existing files.
---

# xgent-init · 出仓 App 仓库的三份文档

**用在 App 自己的 repo 里**（门户代码不在你手上，也不需要在）。产出：

| 文件 | 谁读它 | `service` 型（无前端） |
| --- | --- | --- |
| `CLAUDE.md` | 每次开工的 agent | 生成，删掉前端四节 |
| `PRODUCT.md` | `impeccable` skill 每条子命令开工前 | 生成 |
| `DESIGN.md` | `impeccable` skill、任何 UI 工作 | **不生成** |

模板在 `references/`，槽位规则在 [references/fill-guide.md](references/fill-guide.md)，结构检查在 `scripts/check-docs.mjs`。
**平台契约不在本 skill 里**：口径以目标仓已装的 `portal-external-app` / `portal-micro-app` / `xgent-app-release` / `xgent-image-push` / `portal-dev-setup` / `portal-app-exchange` 为准，模板正文已经引用它们，不要在这里或生成的文件里复述一遍。

## 红线

1. **不编造。** 用户、指标、竞品、启动命令、端口 —— 从 manifest 或仓库里读不出来的，只能问，不能猜。
2. **不覆盖。** 已存在的文件一律跳过。只有用户明确要求更新时才动，且只改指定小节。
3. **不留占位。** 生成完的文件里不能有 `[方括号]` 槽位、`<!-- 填写指引 -->`、`<APP_KEY>` 这类令牌，也不能有「请根据实际情况补充」这种话。
4. **不引门户仓路径。** 目标仓访问不到 `apps/…` `packages/…` `docs/…`，要指路就指 skill 名。
5. **只问一轮。** 把所有缺的合成一次问完，别来回打断。
6. **脚本不绿不算完成。**

## 流程

### 1. 前置判定

目标仓有 `app.manifest.json`（根目录，或 `deploy/portal/` `portal-app/` `deploy/` 下），或用户明确说这是一个出仓 App → 继续。
在门户 monorepo 里 → 停下，指向 `portal-micro-app`；这份 skill 只服务代码不在门户仓的 App。

### 2. 存在性检查

逐份看 `CLAUDE.md` / `PRODUCT.md` / `DESIGN.md`。

- 该有的都有（`micro` 三份 / `service` 两份）→ **报告「已齐，未写任何文件」并停止**。
- 部分存在 → 只补缺的那几份，已存在的原样不动。
- 用户明确要求更新某份已存在的文件 → 只改他指名的小节：先把改动后的小节全文贴出来，确认了再写，不整份重写。

### 3. 事实采集

按这个顺序，**不许跳级**：

1. **`app.manifest.json`** —— `listingKey`、`name`、`type`、`color`、`tagline`、`desc`、`navItems`、`helpEntry`、`scopes`、`aclManifest`。
   `name` 两种形状都接受：对象取 `zh-CN`，字符串直接用。
2. **仓库信号** —— `package.json` 的 scripts 与依赖、`compose*.y*ml`、`Dockerfile`、README、目录结构与路由 / 页面 / 组件命名。
3. **仍然缺的** —— 进第 4 步问。

边读边填一张「槽位 → 候选值 → 来源」表（槽位清单见 fill-guide.md §1–§2）。`type` 决定 DESIGN.md 生不生成、CLAUDE.md 怎么裁剪。

### 4. 一次批量追问

把仍为空且推不出来的槽位合成**一轮**问题，通常 3–6 个：分条编号、每条给一个默认建议，让用户可以整体回「都按建议」。
用户跳过的槽位按 fill-guide.md §5 的「删法」处理 —— **删掉那段，不要留空位，也不要编一个**。

### 5. 渲染

以 `references/` 里的模板为骨架，逐槽位填：

- 替换 `<APP_KEY>` / `<APP_NAME>` / `<PREFIX>`（PREFIX = key 大写、`-` 换 `_`，不用问）与 `type: [micro|service]`。
- 删掉全部 `<!-- 填写指引 -->` 注释，以及 `## Design Context` 里那条讲链接路径的注释。
- 按 fill-guide.md §5 裁剪可选小节；保留的小节要把标题里的括号条件去掉。
- `service` 型：删 CLAUDE.md 的前端四节，并把 `## Design Context` 收敛成只指向 PRODUCT.md（不再提 DESIGN.md、impeccable、浏览器验证）。
- DESIGN.md frontmatter 的 `name` / `description` 用双引号，值里的 `"` 与 `\` 要转义；`app-identity` 用 manifest 的 `color`，hover / dark 按 fill-guide.md §3 推导并在 frontmatter 上方留一行 YAML 注释说明是推导值。
- **不要动** `Page<T>`、`{colors.x}` 引用、PRODUCT 的七个标题、DESIGN 的六个标题。

### 6. 写入

只写本次缺的文件。

### 7. 校验

```bash
node <本 skill 目录>/scripts/check-docs.mjs <目标仓根>
```

有 `✗` 就改了重跑，直到退出 0。`!` 是提醒，自己判断要不要处理。
环境里没有 Node → 照着 fill-guide.md §0 与 §7 逐条人工过一遍，并在报告里写明「未用脚本校验」。

### 8. 报告

一张表列出每个槽位的值与来源（manifest / 仓库信号 / 用户回答 / 模型建议），外加：

- 生成了哪几份文件、跳过了哪几份；
- 删掉了哪些可选段落（让用户知道少了什么，而不是以为漏了）；
- `app-identity-hover` / `-dark` 的推导值，一句「可按需调整」；
- 一句提示：这三份文件的结构是给 `impeccable` 读的，装了 impeccable 的话建议让它读一次确认。

标为「模型建议」的行是给用户纠正用的 —— 用户改口后走第 2 步的小节级更新，不要重跑整个流程。
