---
name: grafton-view-dev
description: '用 @xgent-ai/grafton 做 schema 驱动 UI 开发的工作流与速查。当需要：(1) 用 JSON schema 写/改一个 grafton view，(2) 写绑定表达式（@store:: / @view:: / @local:: / @action:: / @form::），(3) 写 zustood store（createStore / extendSelectors / extendActions / createApiStore 等）并在 schema 中读写，(4) 写列表模板、条件渲染、双向绑定、表单，(5) 写带 Component.grafton 元数据的自定义 widget，(6) 排查 grafton 渲染报错（hooks 顺序崩溃、bindable 组件点不动、Fragment className、$type 解析失败）时，使用此 Skill。凡是涉及 createView / grafton schema / @store:: 表达式 / grafton widget 的活，都先用它，别凭记忆硬写。注意：配置 grafton-app.json 用 web-app-dev，创建模块脚手架用 web-module-dev。'
---

# Grafton View Dev

`@xgent-ai/grafton` 把 **JSON schema 解析为 React 组件树**：schema 声明组件类型、props、插槽、列表模板、条件渲染，以及对 store / 表单 / 本地上下文的数据绑定与动作绑定。

本文是**开发时的高频速查 + 标准工作流 + 必避的坑**；详尽细节按主题拆进 `references/`，**写到拿不准时按需读对应文件**(不要一次性全读)：

| 要做的事 | 读 |
|---|---|
| 写绑定 / 值级条件对象 / `.$` 操作符 / `<>` 双向绑定 / 导航动作 | [references/bindings.md](references/bindings.md) |
| 节点特殊键全集 / `$type` 解析 / createView · createLazyView 参数 | [references/schema.md](references/schema.md) |
| store / selector / action / 持久化 / hooks / 三件套 / HTTP 获取 | [references/stores.md](references/stores.md) |
| 内置组件清单 / 自定义 widget(grafton 元数据) / slots / renderers / 表单 / i18n | [references/components.md](references/components.md) |
| 可直接抄改的完整示例(列表 / 异步加载 / 双绑过滤器 / widget / 表单 / 路由重挂载) | [references/recipes.md](references/recipes.md) |

> 本 skill 不覆盖 WiseView 扩展与流式视图(`createStreamingView`)——那两块直接看源码。

## 心智模型

```
schema (JSON)  +  registry (组件表)  +  stores (zustood 状态)
        │
        ▼
   createView()  ──►  React 组件
```

- **schema**：嵌套 JSON 节点，`$type` 指组件，其余键为 props，`$content` 为子节点。
- **registry**：`$type` 名 → React 组件。grafton 内置组件(`graftons`)自动并入，不用注册。
- **stores**：zustood store，schema 里用 `@store::` 等前缀读写，**响应式**。
- **payload**：沿组件树向下传的本地上下文(列表项数据 `{data, index}`、`$payload` 变量、局部 registry)，用 `@local::` 读。

数据流：`store → UI`(绑定)、`事件 → @action → store`(动作)、`<>` 双向绑定让一个 prop 同时读写。

## 标准工作流：写一个 view

1. **定义 store**(工厂函数,见下) → verify：`bun test` 或在视图里读得到值。
2. **`createView({ schema, registry, stores, ns })`** → verify：页面渲染无红色 ErrorPanel。
3. **schema 里用绑定接数据**(`@store::`/`@view::`/`@local::`) → verify：改 store 数据 UI 跟着变。
4. **接动作**(`onClick: '@action::...'` 或动作对象) → verify：点击触发、状态变化。
5. **列表/条件/表单** 按需加(见下)。
6. **UI 改动必须在浏览器里验证**(项目约定)：用 Chrome extension 跑通主路径，别只看 diff/类型检查。设计阶段先过 `impeccable`。

注册组件：内置组件自动可用；`Button` / `NavSelect` / `Form` 等来自 `@xgent-ai/ui-shadcn`，**必须** 传进 `createView` 的 `registry`。

## 绑定表达式速查（最高频）

字符串值形如 `[!][<>]@source::path` 时按引用解析，否则按字面量。

### 数据源前缀

| 前缀 | 解析自 | 双向 | 例 |
|------|--------|------|----|
| `@store::` | StoresProvider 的 store | ✓ | `@store::tasks.remaining`、`@store::test.formData.tags` |
| `@view::` | 内置 view store(视图局部态 + 工具 selector) | ✓ | `@view::subtitle`、`@view::_isEmpty` |
| `@local::` / `@payload::` | 当前 payload | ✗ | `@local::data.title`、`@local::index` |
| `@action::` | 内置动作 或 store 动作 | ✗ | `@action::navPush`、`@action::tasks.addTask` |
| `@form::` | react-hook-form 上下文(需在 `Form` 内) | ✗ | `{ "$selector": "@form::watch", "$args": ["title"] }` |
| `@fieldArray::` / `@fieldArrayItem::` / `@fieldArrayValues::` | 数组字段 / 当前项 / 当前项值 | ✗ | `@fieldArray::append` |
| `@hook::` / `@context::` / `@registry::` | 注册的 hook / 渲染上下文 / factory registry | ✗ | `@hook::useSidebar` |

### 修饰符

- **`!` 取反**：`"!@store::app.loading"`(`@action::` 不支持)。⚠️ 取反**只有 `!` 前缀**,没有 `$not`/`$negate` 字段——写了会被无视、逻辑反掉。
- **`<>` 双向绑定**：`"<>@store::tasks.filter"`。**只支持 `store`/`view` 源**，且目标 prop 必须在组件 `grafton.bindable` 中声明。
- **动态键 `.[...]`**：`"@store::perms.[@local::data.roleId].name"`,方括号内引用先求值再拼路径。
- **`.$` 操作符**(对取到的值加工)：`$size` `$keys` `$values` `$pick:a,b` `$slice:0,3` `$join:,` `$split:,` `$remap:to=from` `$mapGet:path` `$cast:type` 等。例 `"@store::tasks.tasks.$size"`。

### 值级表达式对象（任意 prop 值都能用，可嵌套）

```jsonc
{ "$selector": "@store::tasks.visibleTasks", "$args": ["active"] }   // 带参 selector，! 可取反
{ "$action": "@action::tasks.addTask", "$args": ["x"], "$withEventArgs": false }  // 动作对象
{ "$if": "@local::data.done", "$then": "line-through", "$else": "" }  // 条件取值
{ "$and": ["@store::a.ready", "!@store::a.error"] }
{ "$or": ["@local::data.urgent", "@local::data.pinned"] }
{ "$ifNull": "@store::tasks.filter", "$then": "all" }    // 空值兜底
{ "$extends": ["@store::app.baseProps"], "size": "sm" }  // 对象浅合并(后者覆盖)
{ "$concats": ["@store::a.list1", "@store::a.list2"] }   // 数组拼接
{ "$effect": "report.track", "$args": ["click"] }        // store side effect
```

**动作对象字段**：`$action`(必填) · `$args`(预置参数,走绑定解析) · `$withEventArgs`(默认 `true`,把事件参数附在 `$args` 后 —— 给 action 传精确参数时**关掉**) · `$eventArgsFirst` · `$preventDefault` / `$stopPropagation` · `$debounced: {throttled, interval, leading, tailing}`。

## Schema 节点特殊键速查

| 键 | 语义 |
|----|------|
| `$type` | 组件类型(必填)。解析顺序：HTML 白名单 → 当前组件局部 registry → 祖先 payload registry → view registry+内置。全未命中→红色 ErrorPanel |
| `$content` | 子内容→children。只渲染 truthy 值；数组渲染为列表；`leaf` 组件禁用 |
| `$when` | 求值**严格 === `false`** 才隐藏节点。⚠️ `0`/`""`/`null` 不隐藏 —— 要布尔语义配 `_isEmpty`/`_isNull` 或 `!` |
| `$payload` | 声明 payload 变量(支持绑定),与父级合并下发,子树 `@local::` 读 |
| `$clearParentPayload` | 切断父级 payload 继承(含累积 registry) |
| `$style` | 追加 className(与 `className` 用 `cn()` 合并)。惯例：基础类写 `className`,动态/附加写 `$style` |
| `$tooltip` | 包一层 Tooltip,字符串等价 `{title}` |
| `$action` | 快捷链接：字符串→href(`"_target:href"` 指定 target),函数→`<a onClick>` |
| `$breakpointProps` | 响应式 props `{sm:{...}, md:{...}}`,按断点级联合并 |
| `$mapping` | 列表模板里把数据项字段直通 props：`{propName: fieldName}` |
| `$key` | 渲染时剥离。列表项 key 只在**数据项自身**含 `$key` 时由 `ListRenderer` 用 |
| `$effects` | 副作用列表,项为 `"store.effectName"` 或 `{$effect, $args}` |
| `$extendSchema` / `$comment` / `$debug` | 运行时扩展 / 注释 / 调试输出 |

## Store 速查

store 永远是**工厂函数**,交给 `createView`(或 `StoresProvider`)在渲染时实例化：

```js
const myStore = (props, parentStores, getStores) =>
  createStore('my')({ count: 0 /* initialState 必填 */ }, { /* options 可选 */ })
    .extendSelectors((api) => ({
      doubled: (state) => state.count * 2,            // (state, ...args) => value
      hasItem: (state, id) => state.items.some(i => i.id === id),
    }))
    .extendActions((api) => ({
      inc: () => api.set.count(api.get.count() + 1),
      addItem: (it) => api.set.state(draft => { draft.items.push(it); }),  // immer draft
      load: async () => { api.set.items(await Runtime.$api.get_('/items')); },
    }));
```

**每个状态 key 自动生成**：`get.x()`(同步读) · `set.x(v)`(写,**新旧 `===` 时跳过**) · `use.x()`(hook 订阅)。schema 侧：`@store::my.x` 读 → `use.x()`；`@action::my.x` 写 → `set.x`。

**`set` 上始终可用**：`set.state(objOrFn)`(immer) · `set.toggleState(path)`(翻转布尔,支持深路径) · `set.enableState/disableState(path)` · `set.useSideEffect(key, ...args)`。

**链式扩展**：
- `extendSelectors` —— 派生值;selector 之间可 `api.get.x()` 复用。⚠️ 返回**每次新建的深对象**会绕过浅比较致多余重渲染。
- `extendMemoSelectors` —— 带缓存：`{ deps: (state)=>({...}), selector: (depsResult)=>... }`,deps 浅比较未变则不重算。⚠️ **只有一个缓存槽**,不适合参数高频交替。
- `extendActions` —— 闭包函数挂到 `set`;重名覆盖,旧实现存于 `新函数.origin`;可 async。
- `extendSideEffect` —— `useEffect` 驱动的副作用:`{ deps: (state,...a)=>[...], effect: (...deps)=>cleanup? }`,经 `$effects` 或 `set.useSideEffect` 消费。

**配置项**(`createStore(name)(init, options)`)：`isGlobal`(注册进 `Runtime.stores`) · `middlewares`(immer 始终内置) · `devtools`(⚠️ 须先在 `Runtime.init` 的 `addons` 注入 `storeDevTools`) · `persist` / 任意 `persist*` 前缀(各为一条持久化通道)。

**内置 view store**：每个视图自带,`@view::` 读。渲染时 `data` prop 成其初始状态(每 key 有自动 setter,支持 `<>` / `@action::view.x`);`selectors` prop 追加自定义 selector。工具 selector：`_isEmpty` `_isNull` `_isEqual` `_and` `_or` `_template(text,data)` `_toPath(...parts)` `_filterOut(data,key,value)` `_parent` 等。

**专用 store 三件套**(完整配置见 [references/stores.md](references/stores.md),用到再查)：
- `createApiStore` —— API 获取+缓存+Suspense,每个 API 一个惰性 selector。⚠️ 默认 `transformer` 假定响应形如 `{data, ...}`,不是这形状必须自定义。
- `createEntityStore` —— 按 id 缓存实体,`use.byId(id)` / `use.entities` Proxy。
- `createListStore` —— 列表/表格,`load` 须 resolve `{schema, rows, totalRows?, cursor?}`,`set.loadMore()` 追加。

## 列表 / 条件 / 双向绑定 / 表单（高频模式）

**列表**：`List` 接 `data` + `itemTemplate`;每项往 payload 注入 `{data, index}`,模板内 `@local::data.x` 读。`$mapping` 字段直通。**列表 key 必须由数据携带**(`{ "$key": "id", "id": 1, ... }`),写在 itemTemplate 上的 `$key` 不参与外层 key,`$mapping` 也不能稳定 key。⚠️ 空状态用 `emptyState` slot(`List`/`PlainList`/`DataList` 都支持,data 空/null 时渲染)。想要网格/容器样式 → 挂在 `List` 的 `className`(真实 `ul`/`ol` 容器);`PlainList` 容器是 `Fragment`、挂 className 无效;`before`/`after` 是夹在列表项前后的**兄弟节点不是包裹层**(网格容器别放 before),且空分支下 before/after 不渲染。详见 [references/components.md](references/components.md)。

**条件**：节点级用 `$when`(严格 `=== false` 隐藏);prop 级用 `$if`/`$and`/`$or`/`$ifNull`;整块 UI 二选一用 `IfElse`/`Switch` 组件。

**双向绑定**：`"value": "<>@store::tasks.filter"`。组件侧用 `signal()` 解包成 `[value, setter]`。

**表单**(`Form` 等来自 `@xgent-ai/ui-shadcn/widgets`,grafton 本体不含)：`Form` 接 `defaultValues`/`onSubmit`/`resolver`,通过局部 registry 注入 rhf 上下文,字段组件**必须**在 `Form` 子树内;`onSubmit` 的 action 收到表单值对象。

## 自定义 widget（Component.grafton 元数据）

任何 React 组件注册进 registry 即可用;带 `Component.grafton` 静态元数据的才是 **grafton widget**：

```jsx
function TaskItem({ title, done, statusIcon, actions }) { /* ... */ }
TaskItem.grafton = {
  leaf: true,                  // 禁止 $content
  bindable: ['value'],         // 允许 "<>" 双向绑定的 prop
  slots: ['statusIcon', 'actions'],  // 值是 schema，grafton 先渲染成 ReactNode 再传入
  registry: { Title: SheetTitle },   // 局部注册表，只在该组件子树内解析 $type
};
```

元数据字段：`leaf` · `t`(注入 i18n `t`) · `bindable` · `slots`(`'name'` 或 `{name, required?, template?}`) · `renderers`(schema prop → `(item,index)=>El`,`List` 的 `itemTemplate` 即此) · `registry` · `factory` / `payload` / `schema`(元组件用)。

**bindable + signal()**：`signal(value)` 在绑定时返回真 setter,**传普通值时 setter 是 no-op**(`isNotSetter` 标志)——见下方坑 #1。

## ⚠️ 必避的坑（LLM 最容易写错的）

1. **bindable 受控组件传静态值是只读且无报错的**。`NavSelect`/`SelectWidget`/`SliderWidget`/`ToggleGroupWidget`/`FilterTabs` 等,`value` **必须** `<>@store::`/`<>@view::` 绑定才能交互。"组件渲染正常但点不动/拖不动"先查这里。
2. **`Text` 的 `variant: "plain"` 渲染为 Fragment,不能带 className**(报 "Invalid prop className supplied to React.Fragment")。要样式包一层 `div`/`span`。
3. **SPA 路由切换,不同 schema 的页面级树必须按路由 key 重挂载**:`<StoresProvider key={pathname}>` 或路由出口包 `<Fragment key={pathname}>`。否则 React 同位复用实例、hooks 数量不一致直接崩("change in the order of Hooks called by Grafton")。
4. **响应式状态用"已存在 store 的 setter + selector"**,不要靠重建 view store(给视图传新 `data`)刷新已挂载节点 —— 已渲染节点的订阅不跟随 store 实例重建。
5. **内置 `Link` 不转发 ref**:放进 Radix `asChild`/Slot 位置时改用 `@xgent-ai/router-lite` 的 `Link`(forwardRef,`to` 属性)。
6. **局部注册表只在子树内有效**:`Title`/`Close` 这类名字必须出现在声明它们的组件(如 `DialogWidget`)的 slots/子树内,校验/预览解析 `$type` 时才找得到。
7. **`devtools: {enabled:true}` 须先注入 addon**(`Runtime.init` 的 `addons.storeDevTools`),否则创建 store 直接抛错。
8. **selector 返回新建深对象**绕过浅比较致多余重渲染 → 用 `extendMemoSelectors`(但注意单缓存槽)。
9. **来自数据的字符串都应 `noTranslate: true`**,否则碰巧与翻译 key 同形会被误翻。
10. **Tailwind v4 不扫 node_modules**:grafton/ui-shadcn 的类要靠 `@source` 显式纳入,否则组件"没样式"(项目搭建/配置属 `web-app-dev` skill 范畴)。

## 自检清单

写完一个 view,逐项核对：

- [ ] 自定义组件(Button/Form/...)都进了 `registry`？内置组件不用注册。
- [ ] 交互型 bindable 组件都用了 `<>` 绑定,不是静态值？(坑 #1)
- [ ] 列表数据项都带了 `$key` 字段稳定 key？(不是写在 itemTemplate 上)
- [ ] `$when` 想要布尔语义时配了 `_isEmpty`/`_isNull`/`!`,没指望 falsy 隐藏？(坑 #2 of `$when`)
- [ ] 给 action 传精确参数时 `$withEventArgs: false`？
- [ ] 数据来源字符串都 `noTranslate`,文案用了翻译 key？
- [ ] 多页应用页面级树按路由 key 重挂载？(坑 #3)
- [ ] **在浏览器里真的跑通了**(不是只看 diff/类型检查)？
