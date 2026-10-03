---
name: xgent-init
metadata:
  internal: true
description: 用于在 XGENT 出仓 App 仓库初始化或补齐 AGENTS.md、PRODUCT.md、DESIGN.md；已有文件不覆盖。保留 Portal 产品边界与设计规范。
---

# xgent-init · 出仓 App 的 Agent、产品与设计上下文

用在 App 自己的 repo 里。产出：

| 文件 | 内容 | `service` 型（无前端） |
| --- | --- | --- |
| `AGENTS.md` | 通用编码准则、onebox 联调、前端浏览器验收 | 生成，前端约定按适用条件执行 |
| `PRODUCT.md` | 产品上下文与 Portal 已提供的能力边界 | 生成 |
| `DESIGN.md` | Portal 色彩、字体、组件、主题与 iframe 设计规范 | 不新生成，已有文件保留 |

`AGENTS.md` 是独立的通用规范，直接使用 [references/external-app-AGENTS.template.md](references/external-app-AGENTS.template.md)，没有需要填写的 App 身份或启动命令占位，不生成或镜像 `CLAUDE.md`。

`PRODUCT.md` 与 `DESIGN.md` 分别使用 [references/external-app-PRODUCT.template.md](references/external-app-PRODUCT.template.md) 和 [references/external-app-DESIGN.template.md](references/external-app-DESIGN.template.md)。填槽位前读 [references/fill-guide.md](references/fill-guide.md)，保留模板里的 Portal 产品边界与设计规范，不替换成通用设计建议。

`install` 保留 hooks / impeccable 安装，可选步骤直接创建仓根 `AGENTS.md`，不安装本 skill，也不生成产品和设计文档。需要补齐这些文档时显式安装并使用本 skill；非交互 install 用 `--xgent-init` 创建 AGENTS.md，`--force` 也不覆盖已有文件。

## 约束

- 不编造用户、指标、竞品、身份色或项目事实。仓库读不到的必要信息合成一轮追问；可选信息缺失就删对应段落，不留占位。
- 已有文件一律跳过。用户明确要求更新时，只修改指定内容。
- 平台步骤以 `portal-external-app` / `portal-micro-app` / `portal-dev-setup` / `portal-app-exchange` / `xgent-app-release` / `xgent-image-push` 为准，不引用目标仓访问不到的门户源码路径。

## 流程

1. **判定目标与检查文件。** 确定出仓 App 仓根，读取 `app.manifest.json`（根目录或 `deploy/portal/`、`portal-app/`、`deploy/` 下）；用户明确说明是出仓 App 而缺清单时，追问必要事实。门户 monorepo 内的 App 使用 `portal-micro-app`。检查 AGENTS.md / PRODUCT.md / DESIGN.md，只补缺的；都齐了就报告并停止。
2. **采集产品与设计事实。** 从 manifest 的 name、type、color、tagline、desc、navItems、scopes、aclManifest，以及 README、页面、路由与组件中收集槽位候选和来源。name 为对象时取 zh-CN，为字符串时直接使用。type 决定是否生成 DESIGN.md。
3. **一轮追问。** 把仍缺的必要槽位一次问完。用户跳过的可选项按 fill-guide 删除；未确定的必需身份色或产品事实不得猜填。
4. **渲染。** AGENTS.md 原样复制通用模板。PRODUCT / DESIGN 按指南填槽位并删除填写指引；保留 PRODUCT 的七个二级标题、DESIGN 的六个二级标题与 YAML frontmatter。身份色使用 manifest 的 color，hover / dark 推导值注明来源。保留 `Page<T>` 与 `{colors.app-identity}` 等类型和 token 引用。service 型不新生成 DESIGN.md，PRODUCT 中的设计文档指引改为仅在有前端时适用。
5. **写入并校验。** 只创建缺的文件，不触碰已有文档；运行 `node <本 skill 目录>/scripts/check-docs.mjs <目标仓根>`。修复本次生成文件的错误后重跑；已有文件的问题仅报告，不自行覆盖。没有 Node 时按指南人工检查并说明未用脚本校验。
6. **报告。** 列出创建与跳过的文件、槽位值和来源、删除的可选段落，以及身份色推导值。说明尚未完成的校验或缺失信息。
