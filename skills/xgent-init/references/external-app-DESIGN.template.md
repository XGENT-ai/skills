---
name: "[你的 App 中文名]"
description: "[一句话：这块工作区是什么，给谁用。]"
colors:
  app-identity: "#2563EB"
  app-identity-hover: "#1D4ED8"
  app-identity-dark: "#5B8DEF"
  brand-blue: "#0063D3"
  brand-blue-hover: "#0054B3"
  brand-blue-press: "#00408A"
  brand-orange: "#FF6B02"
  ink: "#0B1220"
  ink-2: "#1A2233"
  ink-3: "#2A3447"
  paper: "#FAFAFC"
  white: "#FFFFFF"
  slate-50: "#F2F4F9"
  slate-100: "#E7EBF2"
  slate-200: "#D5DBE6"
  slate-300: "#B6BECF"
  slate-400: "#8E99B0"
  slate-500: "#6E7A94"
  slate-600: "#54607A"
  slate-700: "#3E4A60"
  success-500: "#1F9D55"
  warning-500: "#D89400"
  danger-500: "#D6293E"
typography:
  title:
    fontFamily: "Space Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "-0.01em"
  body:
    fontFamily: "Space Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0"
  label:
    fontFamily: "Space Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 500
    lineHeight: 1.3
    letterSpacing: "0"
  mono:
    fontFamily: "JetBrains Mono, ui-monospace, SF Mono, Menlo, monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0"
rounded:
  sm: "4px"
  md: "8px"
  lg: "12px"
  xl: "16px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
  "2xl": "48px"
components:
  button-primary:
    backgroundColor: "{colors.app-identity}"
    textColor: "{colors.white}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  button-secondary:
    backgroundColor: "{colors.white}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  button-danger:
    backgroundColor: "{colors.danger-500}"
    textColor: "{colors.white}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  input:
    backgroundColor: "{colors.white}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 12px"
  select:
    backgroundColor: "{colors.white}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 32px 0 12px"
  badge:
    backgroundColor: "{colors.slate-100}"
    textColor: "{colors.slate-700}"
    rounded: "{rounded.pill}"
    padding: "3px 9px"
  card:
    backgroundColor: "{colors.white}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "20px"
  tab-active:
    backgroundColor: "transparent"
    textColor: "{colors.app-identity}"
    rounded: "{rounded.md}"
    height: "34px"
    padding: "6px 10px"
---

# Design System: [你的 App 中文名]

## 1. Overview

**Creative North Star: "[一句话北极星，例如「像一张随时可核对的账」]"**

<!-- 填写指引：2–3 段。第一段说这块工作区在做什么、密度取向；第二段说它与底座的关系（下面那段可直接留用）；
     第三段说明确拒绝什么（可从 PRODUCT.md 的 Anti-references 里提炼一句）。 -->

[第一段：这块工作区的性格。它是密集的数据面还是引导式的流程？用户一屏之内在找什么？]

**你画的是内容区，不是一个站。** 版头（应用图标 + 应用名 + 面包屑 + 帮助 / 全屏 / 刷新）与一级侧栏归门户，你的界面从版头之下开始。所以本系统不定义 logo 区、不定义全局导航、不定义登录与设置入口——那些在壳里已经有了一份。你要做的是让内容区**看起来像同一栋楼里的一间房**：同一套中性色、同一套圆角与投影、同一套状态语言，只有身份色和领域语汇是你自己的。

主题不是你的选择：宿主经 `sdk.onTheme` 推 `light` / `dark`，你把它落到自己根节点的 `data-theme` 上，两套都必须过 AA。**iframe 是独立文档**——宿主的 CSS 变量、字体、Tailwind 配置一律不会继承过来，token 与字体都要在你自己的 `index.css` / `index.html` 里再声明一份（本模板的 frontmatter 就是那份的权威值）。

> 本文件只管**长什么样**。SDK 行为（主题 / 语言 / 面包屑 / 弹层 / 路由）以 `portal-micro-app` skill 为准；
> 设计推进用 `impeccable` skill（它开工前会读本文件与 `PRODUCT.md`）。

**关键特征：**
- 冷调 slate 中性色，浅色底是 `paper` (#FAFAFC)，深色底是 `ink` (#0B1220)；两套都是一等公民。
- 一个身份色（`app-identity`）承担 App 内的强调与激活；橙色（#FF6B02）是门户的信号色，**App 内不用**。
- 几何无衬线 Space Grotesk 撑住整个界面；等宽 JetBrains Mono 只给机器值。
- 控件 8px 圆角、发丝边框、hover / focus 是感觉得到而不是看得见（120ms ease-out）。

## 2. Colors

### Primary

**你的身份色**（`app-identity`，模板里给的 #2563EB 是占位值，**必须换掉**）。它是这个 App 在门户里的名字之外的第二个识别物：门户会用它画应用市场 / 侧栏 / 通知里的 App Glyph（那枚圆角方块底色，由 manifest 的 `color` 字段给），你在 App 内用它画激活态、选中态、焦点环和图表的种子色。

三条硬约束：
1. **不要和已有 App 撞色**，也不要撞 XGENT 蓝 #0063D3（撞了就等于没有身份）。
2. **manifest 的 `color` 与这里的 `app-identity` 必须是同一个值**——一个在壳里、一个在房里，色不一样就是同一个 App 说两种话。
3. **深色主题下要单独给一个更亮的值**（`app-identity-dark`）：饱和蓝紫在 `ink` 上普遍掉到 AA 以下。

**「主按钮归谁」是你要做的唯一一次取舍**，全 App 只选一次、一致到底（两种在门户里都有先例）：

| 取舍 | 主按钮 / 焦点环 | 什么时候选它 |
| --- | --- | --- |
| **A. 继承门户蓝** | `brand-blue` #0063D3 | App 的交互与门户高度同构（表单、审批、配置类），你希望它读起来就是底座的一部分 |
| **B. 用身份色** | `app-identity` | App 有强烈的领域感（内容创作、数据分析），你希望进来之后是「另一间房」 |

选了 B，`brand-blue` 仍然保留在调色板里，只用于**指回门户**的东西（例如指向平台配置页的链接）。

### Secondary

本系统**没有**第二个强调色。门户的橙色 #FF6B02 是平台信号（未读 / 新增），App 内复用它会让「平台在说话」和「App 在说话」混在一起。需要第二层强调时，用身份色的 8–16% 淡色底，不要引入新色相。

### Neutral

- **Ink** (#0B1220)：浅色态的正文色；深色态的页面底色。整套投影都用它调（`rgba(11,18,32,…)`）。
- **Slate 600 / 500 / 400** (#54607A / #6E7A94 / #8E99B0)：次级与三级文字、meta、图标缺省色。
- **Slate 300 / 200** (#B6BECF / #D5DBE6)：控件强边框与发丝分隔线。
- **Slate 100 / 50** (#E7EBF2 / #F2F4F9)：下沉面与第二层面板（搜索框、分段控件轨道、骨架屏）。
- **Paper / White** (#FAFAFC / #FFFFFF)：浅色态的页面底与抬升面。
- **Ink 2 / Ink 3** (#1A2233 / #2A3447)：深色态的抬升面与第二层面。

### Status

**Success** (#1F9D55) · **Warning** (#D89400) · **Danger** (#D6293E)，各配一个 50 淡色底（浅色态 #E7F6EE / #FBF3DD / #FCE6E9；深色态用同色 16% 透明）。Info 复用门户蓝。

### Named Rules

**The Tint, Not Fill Rule.** 选中与 hover 用主色的 8–16% 淡色叠在中性色上，**不用**实心。实心色只留给「你该点的那一个」和「你该读的那个状态」。

**The Borrowed-Color Rule（反向版）.** 门户把中性留给你，好让你的身份色读得出来；作为交换，**App 内不使用门户的橙色**，也不要在内容区里重现门户的品牌元素。

**The Status-Never-Alone Rule.** 状态色永远和图标或文字一起出现。色盲用户、灰度打印、以及 `forced-colors` 模式下，色相是第一个消失的东西。

## 3. Typography

**正文 / UI 字体：** Space Grotesk（回退 `ui-sans-serif, system-ui, -apple-system, sans-serif`）
**等宽字体：** JetBrains Mono（回退 `ui-monospace, SF Mono, Menlo, Consolas, monospace`）

**字体要你自己引。** iframe 不继承宿主的字体加载，在你的 `index.html` 里加：

```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet" />
```

`/apps/<key>/` 的 CSP 已放行 `style-src … https://fonts.googleapis.com` 与 `font-src 'self' https://fonts.gstatic.com data: blob:`——**其它字体 CDN 不在放行名单里，会静默不加载**（症状：本地好看，线上变系统字）。要用别的来源就自托管进 `dist`（`'self'`）。

### Hierarchy

嵌进壳里的界面比门户自己的页面**小一档**：页面标题那一级已经由版头的应用名占掉了，你的最大标题是区块标题。

- **Title / h1**（Space Grotesk, 20px, 600, 行高 1.3）：视图标题。整屏最大的字，不要再往上加。
- **Section / h2**（18px → 16px, 600）：卡片与分组标题。
- **Body**（14px, 400, 行高 1.5）：默认阅读尺寸。长文本裁到 65–75ch；密集表格可以更宽。
- **Label**（13px, 500）：表单标签、按钮文字、标签页、chip。
- **Meta / caption**（12px, 400, `slate-500`）：时间、计数、辅助说明。**不要再往下**——12px 已经是 CJK 的可读下限。
- **Eyebrow**（12px, 600, 大写, tracking 0.08em）：只给分组分隔线与下拉分区标题，**不是**每个区块头上都要戴一顶。
- **Mono**（JetBrains Mono, 13px）：ID、token、scope、cron、IP、金额与数量的对齐列、行内代码。

### Named Rules

**The Mono-Means-Machine Rule.** 等宽字表示「这是一个字面量机器值」。不要拿它做强调，不要用它排人话。

**No-Display-Face-Inside-The-Shell.** 门户有一款展示字（Iceland），只给登录页那种 hero 时刻。嵌入式 App 里**没有** hero 时刻，一个都不要用。

## 4. Elevation

默认是平的。分隔靠发丝边框（`--border`）和第二层中性面，不靠投影；只有**离开平面**的东西才有阴影（弹层、模态、抽屉、分段控件被抬起的滑块）。浅色态阴影用 ink 调色，深色态用纯黑加大不透明度——深色下 4% 的黑等于没有。

### Shadow Vocabulary

- **Shadow 1** `0 1px 2px rgba(11,18,32,.04), 0 1px 1px rgba(11,18,32,.06)`：小元素的接触阴影（图标块、分段滑块、开关旋钮）。
- **Shadow 2** `0 4px 12px rgba(11,18,32,.06), 0 2px 4px rgba(11,18,32,.04)`：确实需要浮起来的卡片。
- **Shadow 3** `0 12px 28px rgba(11,18,32,.10), 0 4px 8px rgba(11,18,32,.06)`：Popover 与锚定下拉。
- **Shadow 4** `0 24px 56px rgba(11,18,32,.18), 0 8px 16px rgba(11,18,32,.10)`：模态与抽屉。
- **Focus ring** `0 0 0 2px var(--bg), 0 0 0 4px var(--ring)`：键盘焦点唯一处理；输入框另外叠一圈 3px 的主色 16% 辉光。

深色态整体替换为 `rgba(0,0,0,.4 → .6)`。

### Named Rules

**The Flat-At-Rest Rule.** 静止的面是平的。阴影是「离开了平面」的回应，不是卡片的默认装饰。静止卡片上出现 Shadow 3 就是错的。

**The Popover-Stays-Inside Rule（iframe 专属）.** 你的弹层**出不了 iframe 的边界**——超出的部分直接被裁掉，而在本地全屏调试时你看不出来。锚定弹层必须做边界翻转（flip / shift），长内容自己滚动；确实需要更高的可视区就调 `sdk.resize`，不要指望溢出到宿主页面上。

## 5. Components

每个可交互组件都要有 default / hover / focus / active / disabled，内容可能缺失的地方还要有 loading（骨架屏）与 empty / error（状态块）。手感是精确克制：8px 圆角、发丝边框、120ms ease-out。

### Buttons
- **形状**：8px 圆角，三档高度（sm 30 / md 36 / lg 40px），字重 600，单行。
- **Primary**：实心（身份色或门户蓝，见 §2 的取舍），白字。一屏之内**只有一个**。
- **Secondary**：白底 / 深色态 `ink-2` 底，正文色，`slate-300` 强边框。hover 转 `slate-50`。
- **Ghost**：透明、无边框，给密集工具条里的低强调动作。
- **Danger**：实心 #D6293E，只给破坏性动作，且破坏性动作必须二次确认（**用 App 内的 DOM 模态框**，见 §6）。
- **Disabled**：40% 不透明 + `not-allowed`。**禁用必须能解释原因**（tooltip 或旁注），不然用户只能猜。
- **Icon button**：36px 见方，8px 圆角，`slate-500` 图标；激活态用主色 8% 淡底 + 主色图标。

### Inputs / Fields
- **样式**：36px 高、白底、`slate-300` 边框、8px 圆角、13–14px 字，可带前导图标与 `*` 必填标记。
- **Focus**：边框转主色 + 3px 主色 16% 辉光。
- **Error**：边框与帮助文字转 danger，帮助行说清**怎么改**，不是只说「格式错误」。
- **中文输入法**：输入框回车提交前必须挡掉输入法候选确认的那一次 Enter（`keyCode === 229`），否则用户在选词时就被提交了。用 `@xgent/portal-ui` 的 `./keyboard` / `./input`，不要自己写一份。

### Select（硬规则，不是风格偏好）
原生 `<select>` 直接套输入框的类，留下的是**浏览器原生箭头**——位置 / 大小 / 颜色全归浏览器，`pr-*` 推不动它（padding 只影响文字），也不跟主题走。这个错误反复出现，因为类型检查、lint、单测**全绿**，只有人眼看得出来。

```
appearance-none                       ← 唯一硬要求：关掉原生箭头
pr-8                                  ← 给自绘箭头留位（按图标宽 + 右内缩算）
<ChevronDown size={14} className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-fg-3" />
```

收进你自己的 `Select` 原语里，视图代码里**不写裸 `<select>`**。宽度写在哪取决于 `className` 转发给谁：转发给 `<select>`（内部已有 `w-full`）就得在外面套一层定宽 `div`；转发给外层 wrapper 就直接写在组件上。

### Cards / Containers
- 12px 圆角（模态 / 抽屉 16px），白底 / `ink-2` 底，1px `slate-200` 发丝边框，内边距 20–24px。
- 静止不带阴影。**卡片里不套卡片**——需要第二层就用 `slate-50` 下沉面或一条分隔线。
- 卡片是偷懒的答案：先问一句这些内容是不是本来就该是一张表或一个列表。

### Tables / Lists
- 服务端分页，复用平台的 `Page<T>` 契约；不要一次拉全量再前端分页。
- 表头吸顶、数字右对齐且用等宽、行 hover 用主色 8% 淡底、选中行同色更深一档。
- 列宽稳定：异步加载不要让列宽跳动（骨架屏用等宽占位）。

### Feedback States
- **Loading**：骨架屏（按内容形状），不是转圈圈铺满整块。
- **Empty**：一句说明 + 一个动作。区分「还没有数据」和「筛选后没有命中」——后者要给「清除筛选」。
- **Error**：说清发生了什么、用户能做什么。业务失败是 HTTP 200 + 响应体错误结构，**别把它当网络故障渲染**。

### [Signature Component]

<!-- 填写指引：如果本 App 有一个撑起它身份的自定义组件（一个编辑器、一块画布、一种卡片、一张时间线），
     在这里写清它的结构、状态与由来。没有就整段删掉。 -->

[名字 + 一段：它长什么样、有哪些状态、为什么是它而不是通用组件。]

## 6. Do's and Don'ts

### Do:
- **Do** 把身份色用在激活 / 选中 / 焦点上，并让它与 manifest 的 `color` 逐字相同。
- **Do** 用主色 8–16% 淡底表达选中与 hover，实心色只给「该点的那一个」。
- **Do** 每个状态色都配图标或文字。
- **Do** 两套主题都过 AA，并在**深色态下单独校一遍身份色**。
- **Do** 用 `sdk.setBreadcrumbs()` 表达页面层级（首页推 `[]`，每次全量覆盖，label 用当前语言解析好）。
- **Do** 用 `@xgent/portal-ui` 已有的组件（`UserPicker`、IME 安全输入）再造新轮子。
- **Do** 机器值用等宽字，加载用骨架屏，空 / 错用状态块。
- **Do** 每个动效都给 `prefers-reduced-motion` 替代；过渡落在 120–280ms ease-out。

### Don't:
- **Don't** 在 App 内自绘应用图标 / 应用名 / tagline / 侧栏 / 租户切换 / 用户菜单 / 主题切换 / 通知铃铛——版头和壳已经有一份。
- **Don't** 自建「设置」页与它的侧栏入口（租户配置走平台应用配置页），也不自建审计页与通知中心。
- **Don't** 用 `window.alert` / `confirm` / `prompt`——跨源沙箱 iframe 里**被静默忽略**，用户什么都不会看到，代码却以为用户确认了。一律用 App 内的 DOM 模态框。
- **Don't** 给卡片 / 列表项 / 提示框加左侧色条（`border-left`）。
- **Don't** 在 var() 颜色上用 Tailwind 的 `/alpha` 修饰符（`bg-primary/50`）——**静默失效**，用 `opacity-NN` 或预先算好的色值。
- **Don't** 用门户的橙色，不用展示字体 Iceland，不用第三个字体家族。
- **Don't** 让静止卡片带阴影，不要卡片套卡片。
- **Don't** 写裸 `<select>`（见 §5），不要拿 `pr-*` 去推原生箭头。
- **Don't** 先想模态框：能内联、能抽屉、能就地展开的，都不要打断用户。
- **Don't** 只看 diff 就说 UI 改完了——类型检查和单测只证明代码对。改动要从宿主进入、在真浏览器里走通主路径与关键边界；环境起不来就**显式说明「未在浏览器中验证」**。
