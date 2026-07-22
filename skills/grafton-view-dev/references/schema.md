# Schema 节点 / createView / createLazyView

schema 节点的结构与视图工厂。读本文当：拿不准某个 `$xxx` 节点键的语义、`$type` 为什么解析不到、createView 接什么参数、视图组件渲染时还能传什么 props。

## $type 解析顺序

1. **HTML 标签白名单**（小写）：`div` `span` `input` `a` `p` `nav` `section` `article` `aside` `header` `footer` `img` `video` `audio`
2. **当前组件的局部注册表**（`Component.grafton.registry`，见 [components.md](components.md)）
3. **祖先累积的 payload registry**（局部注册表沿子树向下继承）
4. **view registry**（`createView` 传入 + grafton 内置组件）

全部未命中则抛错，由 `ErrorBoundary` 捕获渲染为红色 `ErrorPanel`。

## 节点级特殊键

| 键 | 类型 | 语义 |
|----|------|------|
| `$type` | string | 组件类型（必填） |
| `$content` | string \| object \| array | 子内容 → React children。只渲染 truthy 值；数组渲染为元素列表；`leaf` 组件禁止使用 |
| `$when` | 表达式 | 求值**严格等于 `false`** 时整个节点不渲染。⚠️ `0`、`""`、`null` 等 falsy 值**不会**隐藏节点；要布尔语义配 `_isEmpty`/`_isNull` 或 `!` 取反 |
| `$payload` | object | 声明 payload 变量，与父级 payload 合并后传给子树（子树用 `@local::` 读）；值支持绑定表达式 |
| `$clearParentPayload` | boolean | 为真时子树不继承父级 payload（含累积的 registry） |
| `$style` | string | 追加 className（与已有 `className` 用 `cn()` 合并）。惯例：基础类写 `className`，动态/附加类写 `$style` |
| `$tooltip` | string \| object | 给节点包一层 Tooltip；字符串等价 `{title: ...}` |
| `$action` | string \| function | 快捷链接：字符串作 href（`"_target:href"` 可指定 target），函数包成 `<a onClick>` |
| `$breakpointProps` | object | 响应式 props：`{sm: {...}, md: {...}}`，按当前断点级联合并 |
| `$extendSchema` | 引用 | 运行时扩展：引用求值出的对象浅合并进当前节点 props |
| `$effects` | array | 副作用列表，项为 `"store.effectName"` 或 `{$effect, $args}`（见 [stores.md](stores.md) 的 extendSideEffect） |
| `$key` | string | 渲染时剥离，不传给组件。列表外层 key 只在**数据项自身**含 `$key` 时由 `ListRenderer` 使用（见 [components.md](components.md) 的 renderers） |
| `$mapping` | object | 列表模板中的字段映射 `{propName: fieldName}` |
| `$debug` | boolean | 控制台输出该节点的 getters / hooks 信息 |
| `$comment` | any | 注释，渲染时忽略 |

```jsonc
{
  "$type": "div",
  "$when": { "$selector": "!@view::_isEmpty", "$args": ["@store::app.items"] },
  "$style": "p-4 border rounded",
  "$tooltip": "common.hint",
  "$content": [{ "$type": "Text", "text": "hello" }]
}
```

## createView 参数

| 参数 | 说明 |
|------|------|
| `schema` | 视图 schema（必填） |
| `registry` | `$type` → 组件映射，自动并入 grafton 内置组件；`icons` / `schemas` / `hooks` 子对象自动展开为命名空间条目（见 [components.md](components.md) 的局部注册表） |
| `stores` | store 工厂映射 `{name: creator}`，自动附加内置 `view` store |
| `actions` | 全局动作映射 |
| `ns` | i18n 默认命名空间 |
| `rootPayload` | 根节点初始 payload |
| `noTranslate` | 不包 `withTranslation()` HOC |
| `displayName` | 组件名（调试用） |

## 视图组件的渲染期 props

`createView` 返回的组件在渲染时还接受：

| prop | 说明 |
|------|------|
| `debug` | 输出渲染日志 |
| `ns` | 覆盖默认命名空间 |
| `registry` / `actions` | 追加/覆盖 view model 中的同名配置 |
| `storesMapping` | store 别名映射 `{schema中的名: 实际store名}`，绑定解析时换名 |
| `tooltipProps` | 传给内置 TooltipProvider |
| **其余 props** | 全部传给内部 StoresProvider，作为 **store 工厂的入参**（如内置 view store 消费的 `data` / `selectors`，见 [stores.md](stores.md)） |

## createLazyView

从 URL 异步加载 schema，带缓存与 Suspense：

```js
const Page = createLazyView({
  schemaUrl: '/schemas/page.view.json',   // 必填
  loadingFallback: () => <Spinner />,     // 可选
  // 其余 createView 参数(registry / stores / actions / ns ...)照常可用
});
```

schema 首次拉取后缓存；同一 `schemaUrl` 不重复请求。

## schema 外置

schema 是纯 JSON，可从 `.jsx` 搬到 `.json` 文件，由 `createView({ schema })` 引入；放在 `src/` 内 Tailwind v4 能自动扫到类名。这是 schema 驱动的核心收益（独立存储、下发、非前端角色维护）。更进一步用 `createLazyView` 从 URL 动态下发。
