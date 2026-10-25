# Rustify Docs 作者契约

创建或配置站点前读取本文件。此处是可跨项目携带的最小作者契约，不依赖本机 Rustify 源码目录。

核对依据：Rustify UI 源码 `032756c041615dca075a49685fbb458ce7cc833f` 的 [作者指南](https://github.com/rockie/rustify-ui/blob/032756c041615dca075a49685fbb458ce7cc833f/docs/docs-ssg.md)、[CLI README](https://github.com/rockie/rustify-ui/blob/032756c041615dca075a49685fbb458ce7cc833f/crates/rustify-docs-cli/README.md) 与 CLI 参数定义。基线版本为 0.2.1，不表示永远是最新版；目标项目版本不同时先读其匹配文档和实际帮助，不混用契约。

## CLI 与项目

先探测已安装的 CLI 和 Rust，不因 PATH 缺入口就覆盖安装。新项目没有版本约束且需要安装时，已核对的 registry 命令是：

```sh
cargo install rustify-docs-cli --version 0.2.1 --locked
rustify-docs --version
rustify-docs --build-info
rustify-docs --help
```

registry 安装使用标准 release profile，不加 `--profile docs-cli`。此基线使用稳定 Rust 1.99，build-info 应有 `panic: unwind` 和 `distribution: registry`。不要为创建站点擅自升级已有项目工具链、CLI 或依赖。

纯内容站：

```sh
rustify-docs new my-docs
cd my-docs
rustify-docs check --deny-warnings
rustify-docs dev
```

`new` 只接受新目录或空目录，默认 content 模板。需要真实 Rust DOM 组件 / 交互 demo 时，使用 `rustify-docs new my-components --template components`；需要稳定 Rust 和 `wasm32-unknown-unknown` target。保留生成的 Cargo.toml、带 checksum 的 Cargo.lock 以及 host SSR / browser hydrate 结构；不得混合 SSR 与 CSR/hydrate 特性。

组件项目严格配对 CLI build-info 中的 rustify-docs、rustify-dom、rustify-components 精确版本和来源。遇到 `rust-framework-pair` 修复整套配对，不用 path、file/link 或随意换 Git revision 绕过。首次组件 check/build 可能获取锁定依赖；纯内容站在 CLI 安装后不需要站点编译、Node、外部 Tailwind CLI 或 GPU 工具链。

编辑完成后停止本任务的 dev，再验生产输出：

```sh
rustify-docs check --deny-warnings
rustify-docs build
rustify-docs preview
```

使用打印出的实际 URL，勿硬编码端口。`preview` 只服务既有输出，不会重建。`dev`、`check`、`build`、`preview` 可用 `--config` 指定配置；配置内路径相对配置目录，命令行 `--out` / `--dir` 相对工作目录。静态产物通过 HTTP 访问，不用 file URL。

## 配置、资源与外观

生成的 `rustify-docs.toml` 是起点。必要字段与常用路径如下，按实际目录和部署目标修改，不复制示例品牌：

```toml
schema_version = 1

[site]
name = "Product Docs"
lang = "zh"
base = "/"

[content]
dir = "content"

[assets]
dir = "static"

[build]
out = "dist"

[[nav.groups]]
label = "入门"
dir = "guide"
```

- `content/index.md` 对应 `/`，`content/guide/start.md` 对应 `/guide/start/`。普通页写 title、description 和有序标题；只保留一个 H1，hero 与正文不能重复制造 H1。
- `site.url` 可选；已知发布地址时设置，其路径必须等于 `site.base`。子路径构建 / 预览必须使用同一 base。不要在未知域名时把示例域名当作 canonical 发布。
- `content.include` 是相对 content.dir 的精确 `.md` 列表，不支持 glob；缺省递归发现，空数组是选择零页。它控制发布源；`nav.groups` 控制导航，不能代替源选择。
- 本地图片、下载和 HTML 资源链接进入资源图并按内容发布；`static/` 按整体目录发布，不放密钥或不应公开的资料。普通页面不可直接加载远程图片，先取得可用的本地素材。
- 外观使用主题与内容中的 utility classes；CLI 在进程内从实际页面 class 编译样式。核对实际输出，不添加不存在的 custom CSS 配置键，不为静态样式引入运行时 utility Worker。
- `theme.primary` 可分别设 light / dark；完整主题从生成的 `theme.example.json` 复制并用 `[theme] document = "theme.json"` 激活。document 与 primary 不能同时设置。
- 默认系统字体。自有字体以 `[[theme.fonts]]` 声明本地 family / src / weight / style，同时在主题两种模式的 font token 中选择该 family，保留许可；不引用远程字体 URL。
- Markdown 原生支持标题、表格、代码组、alerts 和源码 include。内置 Tabs、CardGrid、Steps 不需要作者 Rust 项目；仅使用静态内置组件不是选择 components 模板的理由。

更复杂的 i18n、demo 注册、布局 slot 或主题格式，按需读匹配版本作者指南的相应小节。不要从其他框架类推配置。

## Landing Page 的最小有效形态

在已有 content 项目添加页面或替换对应首页，不运行不存在的 landing 模板：

```markdown
---
title: 将产品价值写成客户能理解的结果
description: 清楚说明目标客户、问题与产品价值。
layout: landing
tagline: 用具体事实支持主张，减少客户理解和行动的成本。
actions:
  - text: 查看解决方案
    link: "#solution"
---

## 解决方案 {#solution}

在这里展示经过核对的产品能力与证据。
```

这是语法示例，交付时替换为目标产品内容。landing 共用 home hero 的 title、tagline、actions、features；正文接在 hero 后。品牌头部没有文档导航、搜索、语言或主题开关，主题跟随系统或站点已有选择；有 profile footer 或 footer slot 才显示相应页脚。默认 `index` / `search` / `ai` 全为 false，三者独立按目标显式设置。

没有 profile 时仍是严格 CSP。只有需要第三方请求或表单时才加入配置；例如以下只是已存在表单服务的接线形状，域名必须换成实际获准的接收端：

```toml
[landing.profiles.campaign]
carry = ["utm_source", "utm_campaign"]

[landing.profiles.campaign.sources]
form = ["https://leads.example.com"]
```

页面 frontmatter 添加 `landing: campaign`，正文可放：

```html
<form id="lead" method="post" action="https://leads.example.com/lead">
  <label for="email">工作邮箱</label>
  <input id="email" name="email" type="email" autocomplete="email" required>
  <input type="hidden" name="utm_source" data-rd-carry>
  <button type="submit">预约演示</button>
</form>
```

- 命名 profile 必须存在且只供 `layout: landing` 使用。`sources` 分 script、connect、img、frame、form；来源填写 origin，不是带路径的 URL。真实厂商域名清单按厂商当前文档核实，勿照猜。
- profile 的 `scripts` 列表只接受外部 src 或项目内 `.js` 文件；本地脚本宜放 `landing/` 等 static 之外的目录。load 是 async 或 defer，默认 defer；外部脚本 origin 自动加入 script-src。
- 不允许内联 script/style、style 属性、事件处理属性、eval 或通过 nonce/hash 绕过；Landing Page 不放宽 style-src。把厂商内联初始化放进已声明的本地脚本，不去修改生成器策略。
- iframe 的来源需在 sources.frame，提供 title、禁止 srcdoc；iframe 服务本身也须允许被嵌入。需内联样式的第三方嵌入不能借此绕过约束。
- form 的 action / 按钮 formaction 必须匹配获准来源或可解析的站内页面。没有 sources.form 不允许提交；生成器不提供接收、存储或邮件后端。站内静态感谢页本身不是线索接收端。
- 原生 POST 接收端接受表单编码，保存后用 303 跳向感谢页。GET 会用字段替换 action 的 query，需要固定参数时用 hidden input。实际提交测试须避免未授权的真实客户数据或通知。
- `carry` 只透传明确列出的参数：hidden input 以 name 匹配，链接 / iframe 用 `data-rd-carry`；hero actions 自动标记。它不使用 cookie/localStorage；无脚本时页面和原生表单仍可用，只缺参数透传。
- 统计、同意管理与隐私说明按站点已有要求及实际接入处理，不因创建落地页自动新增追踪器。缺厂商 ID、表单服务或授权时完成独立页面并保留接入缺项。

## 托管与验收

输出目录含 `rustify-docs-manifest.json`、HTML、指纹 CSS/JS、引用资源与 404，直接交给静态主机。主机需按目录 index.html 回应，并将不存在的路由以真实 HTTP 404 返回；正确发送 CSS/JS/wasm 等 MIME。部署和回滚替换整份产物，不混用各次 build 的 HTML 与资源。

每页都有 meta CSP。若主机还发送 CSP 响应头，Landing Page 必须按 manifest 的 `page_policies` 使用该页策略，或不在这些页发送 CSP 头；全站强加严格头会继续拦截脚本和表单。GitHub Pages 这类无法设置头的主机使用 meta，无需模拟按页响应头。

生产 preview 里核对资源、深链、404 与实际需要的表单、脚本、iframe、透传。本地替身只证明接线契约，不能记为第三方真实集成或转化统计通过。上线另按目标项目已有授权与流程执行。
