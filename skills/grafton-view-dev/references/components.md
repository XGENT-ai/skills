# 内置组件 / Widget 元数据 / 表单 / i18n

读本文当：选用内置组件、写带 `Component.grafton` 元数据的自定义 widget、用 slots/renderers/局部注册表、接表单、配 i18n。

## 内置组件（graftons，自动并入 registry，无需注册）

| 组件 | 要点 |
|------|------|
| `Box` / `VBox` / `HBox` / `Container` | 布局容器；`as` 换标签；V/H 为 flex 列/行 |
| `ResponsiveContainer` | 监听容器宽度，经 `ResponsiveContext` 向子树提供宽度；宽度初始化后才渲染 children |
| `Wrap` | 有 `before`/`after` slot 时才包一层 `with` 指定容器，否则裸渲染 children |
| `Text` | `text` + `variant`（`default`/`heading`/`blockquote`/`code`/`keyboard`/`emphasis`/`strong`/`quote`/`span`/`plain`）。⚠️ **`plain` 渲染为 Fragment，不能带 className**；`noTranslate` 跳过翻译；`values` 模板插值 |
| `Icon` | `variant: "registry"`（查 `icon::name`）/ `"asset"`（SVG sprite）/ `"url"`（mask-image） |
| `List` / `PlainList` | 数据列表；`data` + `itemTemplate`；都支持 `emptyState` slot。⚠️ `List` 有真实容器(`ul`/`ol`)→ 布局/网格类挂 `List` 的 `className` 生效;`PlainList` 容器是 `Fragment` → className 无效、做不了容器样式。详见下「列表容器、布局与空状态」 |
| `DataList` | key-value 展示（Radix DataList）；`icons`/`itemProps`/`i18nPrefix` |
| `ListRenderer` | 底层列表渲染器（`as`/`itemContainerAs`/`keyExtractor`…），组件作者用 |
| `Link` | `link` 为绝对 URL 渲染 `<a>`，否则 RouterLink(SPA)。⚠️ **不转发 ref**，配 `asChild`/Slot 时换 `@xgent-ai/router-lite` 的 `Link` |
| `Action` | `action` prop：字符串(URL) / 函数 / `{type:'link'\|'action', value}`，渲染为链接或可点击元素 |
| `RegistryView` | 按 `name` 渲染 registry 中的命名 schema（`schema::name`） |
| `DynamicView` | 渲染任意 `schema` prop，自带 ErrorBoundary（用于渲染来自数据的任意 schema） |
| `IfElse` | `value` ? 渲染 `then` schema : `else` schema；分支为 `"/path"` 字符串时跳转 |
| `Switch` | 按 `value` 在 `cases` 映射中选 schema，`default` 兜底 |
| `Tooltip` | `title`（经 i18n）+ children；`TooltipProvider` 已由 createView 提供 |
| `Image` | `<img>` + object-contain |
| `ErrorPanel` / `ErrorBoundary` | 错误面板（红底 `role="alert"`，prop 为 `error` 对象，显示 `error.message`）与边界 |
| `SlotsContainer` / `Slot` / `SlotContent` / `Fragment` | 显式插槽组合原语 |
| `Outlet` | `@xgent-ai/router-lite` 的子路由出口 |
| `Delegate` | `@radix-ui/react-slot` 的 `Slot` 别名，把 props 委托给子元素 |

> `Form` 及字段组件**不在**内置中，由组件库（`@xgent-ai/ui-shadcn`）提供；grafton 只提供 hook 注入与 `@form::` 解析机制（见末尾「表单」）。

## 组件 grafton 元数据

组件用静态属性 `Component.grafton = {...}` 声明能力，grafton 渲染时按此注入：

| 字段 | 说明 |
|------|------|
| `leaf` | 叶子组件，禁止 `$content`（传了报错） |
| `t` | 注入 i18n 翻译函数为 `t` prop（配合节点 `ns`） |
| `factory` | 注入 view factory（组件内手动渲染 schema / 取 registry 组件） |
| `payload` | 注入当前 payload 为 prop |
| `schema` | 注入当前节点 schema 为 prop（元组件/编辑器用） |
| `dir` | 布局方向标注（`'v'`/`'h'`，文档性质） |
| `bindable` | 允许 `<>` 双向绑定的 prop 名数组（见 [bindings.md](bindings.md)） |
| `slots` | 具名插槽，项为 `'name'` 或 `{name, required?, template?}` |
| `renderers` | 模板渲染器，项为 `'name'` 或 `{name, fromProp, template}` |
| `registry` | 局部注册表：`$type` 名 → 组件，只在该组件子树内可解析，沿子树累积 |

```jsx
import { cn } from '@xgent-ai/grafton/utils';

function TaskItem({ title, done, statusIcon, actions, className }) {
  return (
    <div className={cn('flex items-center gap-3 rounded-lg border p-3', className)}>
      {statusIcon}
      <span className={cn(done && 'line-through text-muted-foreground')}>{title}</span>
      <div className="ml-auto flex gap-1">{actions}</div>
    </div>
  );
}
TaskItem.grafton = { leaf: true, slots: ['statusIcon', 'actions'] };
```

## slots：插槽

slot 类 prop 的值是一段 schema（单个或数组），grafton 在组件渲染前先渲染成 React 元素再传入（组件作者拿到的是普通 `ReactNode`）：

- `required: true` 且未提供时：有 `template` 用模板兜底，否则报错；
- 提供的 schema 会与 `template` 合并（数组逐项合并）；
- 插槽 schema 里照常可用 `@local::`（在当前节点 payload 上下文渲染）。

```js
SideSheet.grafton = { slots: ['trigger', 'header', 'footer'], registry: { Title: SheetTitle, Close: SheetClose } };
```
```json
{ "$type": "SideSheet", "trigger": { "$type": "Button", "$content": "Open" }, "header": [{ "$type": "Title", "$content": "标题" }] }
```

## renderers：列表与模板

`renderers` 把一个 schema prop 转成 `(itemData, index) => ReactElement` 渲染函数。约定：渲染函数名 `renderItem`，来源 prop 名 `itemTemplate`（`List` 即此）。

模板渲染每项时，payload 注入 `{ data: item, index }`，故模板内：
- `@local::data.xxx` 读当前项字段；
- `$mapping: { "toProp": "fromField" }` 把项字段直接映射成模板 props；
- **外层列表项 React key**：数据项自身有 `$key` 时取 `item[item.$key]`；否则用组件 `keyExtractor(item, index)`；再否则用 index。⚠️ `itemTemplate.$key` **不**参与外层 list item key，`$mapping` 也不能稳定外层 key。

## 列表容器、布局与空状态

底层 `ListRenderer` 的渲染分两支:

```jsx
data?.length
  ? <ListContainer {...props}>{before}{items}{after}</ListContainer>   // 非空
  : children                                                          // 空/null:兜底分支
```

由此推出三条必须记牢的事实:

- **要网格/容器样式 → 挂在 `List` 的 `className`(它是真实 `ul`/`ol`,className 落在容器上)。** `PlainList` 的容器是 `Fragment`,挂 className 完全无效,做不了网格;它用于"无容器样式的裸列表"。`variant: "column"` 是 `List` 的内置网格预设。
- **`before`/`after` 不是包裹层**:它们是容器内夹在列表项前后的**兄弟节点**。把网格容器放进 `before` 不会套住卡片(网格不生效)——这是常见错误。
- **空状态**:`List` / `PlainList` / `DataList` **都有 `emptyState` slot**(与组件库的 `Grid`/`Pane`/`Table` 一致),`data` 为 null 或空数组时渲染它。⚠️ 进入 emptyState 分支时,`before`/`after`/列表项都**不渲染**(整个容器被 emptyState 取代)。也可不用 slot,在列表外面用 `$when:_isEmpty` 的对偶节点控制(见 [recipes.md](recipes.md) #2),但 `emptyState` slot 更简洁、是推荐写法。

```json
{
  "$type": "List", "variant": "column",
  "data": "@store::test.actions",
  "itemTemplate": { "$type": "MenuItem", "$mapping": { "label": "label" }, "$content": "@local::data.label" }
}
```

要用数据稳定 key，让每项携带 key 字段与 `$key` 指示：
```json
{ "data": [ { "$key": "value", "value": "open", "label": "Open" }, { "$key": "value", "value": "close", "label": "Close" } ] }
```

## registry：局部注册表

局部注册表中的 `$type` 只在该组件 slots/子树内可解析，并向更深子树累积（`$clearParentPayload` 切断）。除组件外支持命名空间条目：

- `hook::xxx` —— 注册 hook，供 `@hook::xxx`（内置已注册 `useRef`、`useId` 等）；
- `icon::xxx` —— 注册图标，`Icon` 的 `variant:"registry"` 按 name 查；
- `schema::xxx` —— 注册命名 schema，`RegistryView` 按 name 查；
- `style::xxx` —— 注册样式串。

`createView` 的 registry 里传 `icons: {...}` / `schemas: {...}` / `hooks: {...}` 对象，自动展开成上述命名空间条目：

```jsx
import { Plus } from 'lucide-react';
createView({
  registry: {
    TaskItem,
    icons: { Plus },                                                          // → icon::Plus
    schemas: { taskCard: { $type: 'TaskItem', $mapping: { title: 'title' } } }, // → schema::taskCard
  },
});
```
```json
{ "$type": "Icon", "variant": "registry", "name": "Plus", "className": "size-4" }
{ "$type": "RegistryView", "name": "taskCard" }
```

> ⚠️ 局部注册表只在子树内有效：校验/预览解析 `$type` 时，`Title`/`Close` 这类名字必须出现在声明它们的组件（如 `DialogWidget`）的 slots/子树内。

## 表单

`Form`（组件库提供，如 `@xgent-ai/ui-shadcn/widgets`）通过 `registry: {'hook::useFormContext':..., 'hook::useWatch':...}` 向子树暴露 react-hook-form 上下文；字段组件（`FormInput` 等）在 `Form` 内**自动注册**，因此**必须**位于 `Form` 子树内。

- `Form` 接 `defaultValues` / `onSubmit` / `onError` / `mode`（默认 `onTouched`）/ `resolver`（可挂 zod）；
- 字段组件用 `name` 注册字段，`rules` 写 rhf 校验规则；
- `onSubmit` 是普通动作绑定，action 收到表单值对象；
- `ButtonWidget` 是 `leaf`，文案用 `text` prop（不能 `$content`）。

schema 侧前缀（详见 [bindings.md](bindings.md)）：`@form::xxx`（值/方法，如 `@form::watch` 带 `$args`）、`@fieldArray::append`/`remove`、`@fieldArrayItem::index`/`namePrefix`、`@fieldArrayValues::xxx`。`FormFieldArray` 的 `itemTemplate` 内子字段 `name` 自动按 `namePrefix.index.name` 嵌套。

## i18n

- `Runtime.init` 的 `i18n` 配置初始化 i18next（http-backend `/locales/{lng}/{ns}.json` + 浏览器语言探测），**`ns` 命名空间数组必填**；
- 声明 `grafton.t` 的组件（内置 `Text` 即此）收到注入的 `t`，文案 props（`text`、`label`、`title` 等）**默认翻译**；
- 节点级键：`ns`（单节点换命名空间）、`noTranslate: true`（跳过翻译）、`i18nPrefix`（给子树 key 加前缀）、`values`（`Text` 模板插值）；
- 缺失 key 原样显示（开发期"写原文 + `noTranslate`"与"写 key"可共存）；
- ⚠️ **来自数据的字符串都应 `noTranslate`**，否则碰巧与 key 同形会被误翻；
- 语言切换：`Runtime.i18n.changeLanguage(lng)`（配 `detection: {order: ['querystring', ...]}` 时也可 URL `?lng=` 切换）。

```json
{ "$type": "Text", "variant": "heading", "text": "app.title" }
{ "$type": "Text", "text": "app.remaining", "values": { "count": "@store::tasks.remaining", "total": "@store::tasks.total" } }
```
