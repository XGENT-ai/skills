# 绑定表达式 / 值级对象 / 双向绑定 / 内置动作

schema 里如何读写数据。读本文当：写 `@store::`/`@view::`/`@local::`/`@action::`/`@form::`、值级条件对象、`<>` 双向绑定、导航动作。

## 引用 vs 字面量

字符串值形如 `[!][<>]@source::path` 时按**引用**解析，否则按**字面量**。

## 数据源前缀

| 前缀 | 解析自 | 双向 | 说明 / 例 |
|------|--------|------|----------|
| `@store::` / `@stores::` | StoresProvider 里的 store | ✓ | 路径点号取值；命中 selector 可配 `$args`。`@store::test.formData.tags` |
| `@view::` | 内置 view store | ✓ | 视图局部态 + 工具 selector。`@view::subtitle`、`@view::_isEmpty` |
| `@local::` / `@payload::` | 当前 payload | ✗ | 列表项 `@local::data.label`、`@local::index`、`$payload` 声明的变量 |
| `@action::` / `@actions::` | 内置动作 或 store 动作 | ✗ | 无点号→内置动作；`store.x`→store 的 action/setter |
| `@form::` | react-hook-form 上下文(需在 `Form` 内) | ✗ | `@form::watch` 带 `$args`；自动注入 `hook::useFormContext` |
| `@fieldArray::` | useFieldArray 上下文 | ✗ | 数组字段操作 `append`/`remove` 等 |
| `@fieldArrayItem::` | 当前数组项上下文 | ✗ | `index`、`namePrefix` 等 |
| `@fieldArrayValues::` | 当前数组项的表单值 | ✗ | watch + getValues 合并 |
| `@hook::` | registry 中注册的 hook | ✗ | `@hook::useSidebar`(注册名 `hook::useSidebar`) |
| `@context::` | 渲染上下文对象 | ✗ | 取 context 上的值 |
| `@registry::` | view factory registry | ✗ | 读 registry 条目(等价 `context.factory.registry`) |

## 修饰符

- **`!` 取反**：`"!@store::app.loading"` → 布尔取反(字符串前缀,或 `$selector` 字符串前缀)。`@action::` **不支持**。⚠️ 取反**只有 `!` 前缀这一种**——`$selector`/值级对象上**没有 `$not`/`$negate` 之类的键**(见 `translateSelector`),写了会被无视、逻辑悄悄反掉。
- **`<>` 双向绑定**：`"<>@store::test.sortedBy"`。**只支持 `store`/`view` 源**，且目标 prop 必须在组件 `grafton.bindable` 中声明。组件侧收到 `[value, setter]` 元组，用 `signal()` 解包（见末尾）。
- **动态键 `.[...]`**：`"@store::permissions.[@local::data.roleId].name"` —— 方括号内是另一个引用，运行时求值为字符串后拼进路径。
- **路径 `.$` 操作符**（对取到的值再加工）：

| 操作符 | 作用 |
|--------|------|
| `$get` | 取整体 |
| `$keys` / `$values` | 对象键 / 值数组 |
| `$size` | 长度 / 元素数。`"@store::test.tags.$size"` |
| `$pick:a,b` | 取指定字段子对象 |
| `$remap:to=from` | 字段改名 |
| `$mapGet:path` | 对数组每项取 path |
| `$slice:0,3` | 切片 |
| `$split:,` / `$join:,` | 字符串拆 / 数组合 |
| `$padLeft:x` / `$padRight:x` | 补齐 |
| `$box:key` | 包成 `{key: value}` |
| `$cast:type` | 类型转换 |

## 值级表达式对象

任意 prop 的值都可以是下列对象（可嵌套，`$args` 的每一项也走同样解析）。

### 选择器调用

```json
{ "$selector": "@store::library.isEntryActive", "$args": ["widget", "@local::data.name"] }
```

`$selector` 指向 store selector（`extendSelectors`，签名 `(state, ...args)`）或内置 view selector，`$args` 为参数。前缀 `!` 可取反。

### 动作对象

```json
{ "$action": "@action::test.toggleState", "$args": ["showAlert"], "$withEventArgs": false }
```

| 字段 | 默认 | 说明 |
|------|------|------|
| `$action` | 必填 | 动作引用；无 `@` 前缀时自动按 `@action::` 解析 |
| `$args` | `[]` | 预置参数（支持绑定表达式） |
| `$withEventArgs` | `true` | 是否把事件参数附加在 `$args` **之后**。给 action 传精确参数时**关掉** |
| `$eventArgsFirst` | `false` | 事件参数放在 `$args` **之前** |
| `$preventDefault` / `$stopPropagation` | — | 调用对应事件方法 |
| `$debounced` | — | `{throttled, interval=2000, leading=true, tailing}` 防抖/节流 |

### 条件与组合（值级，可出现在任何 prop 值）

```jsonc
{ "$if": "@store::app.loading", "$then": "Loading...", "$else": "Submit" }
{ "$and": ["@store::a.ready", "!@store::a.error"] }
{ "$or": ["@local::data.urgent", "@local::data.pinned"] }
{ "$ifNull": "@store::user.nickname", "$then": "Anonymous" }   // 空值兜底
{ "$extends": ["@store::app.baseProps"], "size": "sm" }         // 对象浅合并(后者覆盖)
{ "$concats": ["@store::a.list1", "@store::a.list2"] }          // 数组拼接
{ "$effect": "report.track", "$args": ["click"] }               // store side effect
```

> schema 级（整块 UI 二选一）的分支用 `IfElse` / `Switch` 组件，见 [components.md](components.md)。prop 级条件优先 `$if`。

## 内置动作

`@action::xxx`（无点号）解析为内置动作：

| 动作 | 参数 | 行为 |
|------|------|------|
| `alert` | `(message)` | 浏览器 alert |
| `navPush` | `(href)` | SPA 路由 push |
| `navReplace` | `(href)` | SPA 路由 replace |
| `navBack` / `navForward` | — | 历史前进/后退 |
| `navReload` | — | 整页刷新 |
| `navRedirect` | `(href)` | `window.location` 硬跳转 |
| `navUpdate` | `(query, reload?)` | 更新当前 URL query |

> 导航动作依赖 router 上下文 —— 这就是视图必须渲染在 `RouterProvider` 之内的原因。

`@action::store.x`（有点号）按顺序匹配：store 的 **action → 自动 setter → 内置 set 动作**（`toggleState`/`enableState`/`disableState`，见 [stores.md](stores.md)）。

## 双向绑定与 signal

声明了 `bindable: ['value']` 的组件，prop 以 `"<>@store::x.y"` 绑定时，组件实际收到 `[value, setter]` 元组。组件内用 `signal()` 解包：

```js
import { signal } from '@xgent-ai/grafton';

function SliderWidget({ value, ... }) {
  const [current, setCurrent] = signal(value ?? min);
  // 绑定时:setCurrent 写回 store;传普通值时:setCurrent 是 no-op!
}
```

⚠️ **`signal(普通值)` 返回的 setter 是 no-op**（函数带 `isNotSetter = true` 标志）。受控型 bindable 组件（SliderWidget、RangeWidget、SelectWidget、ToggleGroupWidget、NavSelect 等）**传静态值时只读** —— 滑杆拖不动、选择不生效且无报错。要交互必须 `<>@store::` / `<>@view::` 绑定。

写回落点：绑定路径单段 → `store.set.<key>`（自动 setter **或同名 action**）；多段 → `set.state` 深路径写入（`useSignal` 语义）。
