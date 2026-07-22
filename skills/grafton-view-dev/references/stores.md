# Stores / Runtime / HttpClient

状态层（zustood = zustand 封装）与数据获取。读本文当：写 `createStore` 及 `extendSelectors`/`extendActions`/`extendMemoSelectors`/`extendSideEffect`、用 `createApiStore`/`createEntityStore`/`createListStore`、在组件代码里用 store hooks、在 store action 里发 HTTP 请求。

## 工厂约定

store 总是**工厂函数**，交给 `createView`（或手挂 `StoresProvider`）在渲染时实例化：

```js
const myStore = (props, parentStores, getStores) =>
  createStore('my')({ /* initialState 必填 */ }, { /* options 可选 */ })
    .extendSelectors(/* ... */)
    .extendActions(/* ... */);
```

| 参数 | 说明 |
|------|------|
| `props` | StoresProvider 收到的 props（经 createView 即视图组件的"其余 props"，如 `data`/`selectors`）—— 初始状态可由运行时 props 决定 |
| `parentStores` | 父级 StoresProvider 的 stores（顶层为 undefined）；内置 view selector `_parent` 即由此实现 |
| `getStores` | 返回"正在构建中的当前 store 集合"，允许同级 store 互相引用 |

工厂上可附加 **`canOverride = true`**：嵌套 Provider 遇父级同名 store 时，默认**沿用外层不重建**；只有 `canOverride` 的工厂才在内层重建（内置 view store 即如此）。

## 自动生成的接口

`createStore(name)(initialState)` 返回：

| 成员 | 说明 |
|------|------|
| `name` | store 名 |
| `store` | 原始 zustand store（`getState`/`setState`/`subscribe`） |
| `get.<key>()` | 每个状态 key 的同步读 |
| `get.state()` | 整个 state 快照 |
| `set.<key>(v)` | 每个状态 key 的写；**新旧值 `===` 时跳过**（不触发更新） |
| `set.state(objOrFn)` | 整包写；传函数时为 **immer draft**，可原地修改 |
| `use.<key>()` | 每个状态 key 的 React hook 订阅（浅比较去重） |
| `useState(selector)` | 任意 selector 的 hook 订阅（浅比较） |
| `extendSelectors` / `extendMemoSelectors` / `extendActions` / `extendSideEffect` | 链式扩展，均返回 store 自身，可任意顺序续链 |

schema 侧：`@store::my.x` 读 → `use.x()`；`@action::my.x` 写 → `set.x`。

## 内置 set 动作

除 per-key setter 外，`set` 上始终可用：

| 动作 | 说明 |
|------|------|
| `set.state(objOrFn)` | 整包写（immer） |
| `set.toggleState(path)` | 翻转布尔字段；支持深路径 `"ui.sidebarOpen"` |
| `set.enableState(path)` / `set.disableState(path)` | 置 true / false（深路径） |
| `set.useSideEffect(key, ...args)` | 调用 extendSideEffect 定义的副作用 hook |

schema 中即 `@action::my.toggleState` 配 `$args: ["open"]`。

## extendSelectors

```js
.extendSelectors((api) => ({
  total: (state) => state.items.length,
  hasItem: (state, id) => state.items.some((i) => i.id === id),   // 带参
  title: (state, prefix) => prefix + api.get.total(),             // 引用其他 selector
}))
```

- 回调收到 **store api 本身**，selector 间可 `api.get.xxx()` 复用；
- 签名 `(state, ...args) => value`；
- 每个 selector 暴露为 `get.<name>(...args)`（同步）与 `use.<name>(...args)`（hook，浅比较）；schema 中无参 `@store::s.total`，带参用 `{ "$selector": "@store::s.hasItem", "$args": [42] }`；
- ⚠️ selector 每次调用都执行；hook 侧靠**浅比较**去重，返回"每次新建的深对象"仍被视为变化 → 用 `extendMemoSelectors`。

## extendMemoSelectors

带缓存：deps 浅比较未变则返回缓存，不重算。

```js
.extendMemoSelectors((api) => ({
  siteMenu: {
    deps: (state) => ({ sitemap: state.sitemap }),     // 轻量依赖提取
    selector: ({ sitemap }) => sitemap.map(toMenu),    // 昂贵计算，入参是 deps 返回值
  },
  // 二元组写法(兼容): siteMenu: [ (state)=>({...}), ({sitemap})=>... ]
}))
```

- `deps` 也可写**字符串**引用已存在的 selector 名；
- 调用传入的 `...args` 转交给 deps（`deps(state, ...args)`）；
- ⚠️ 每个 memo selector **只有一个缓存槽**：交替用不同参数调用会互相挤掉缓存，参数高频交替场景不适用。

## extendActions

```js
.extendActions((api) => ({
  addItem: (item) => api.set.state((draft) => { draft.items.push(item); }),
  reset: () => { api.set.addItem({ id: 0 }); api.set.count(0); },   // action 互调
  load: async () => {                                              // 可 async
    const data = await Runtime.$api.get_('/items');
    api.set.items(data);
  },
}))
```

- 闭包式普通函数，挂到 `set`；通过闭包里的 `api` 读写；
- 与同名条目（自动 setter 或先前 action）重名时**覆盖**，旧实现保留在 `新函数.origin`，可在新实现内调用；
- schema 中 `@action::store.x` 匹配顺序：action → 自动 setter → 内置 set 动作。

## extendSideEffect 与 $effects

由 React 渲染驱动的副作用（内部包 `useEffect`）：

```js
.extendSideEffect((api) => ({
  syncTitle: {
    deps: (state, suffix) => [state.title, suffix],   // 必须返回数组(useEffect deps)
    effect: (title, suffix) => {                      // 入参为 deps 数组展开
      document.title = title + suffix;
      return () => { /* 可返回清理函数 */ };
    },
  },
}))
```

消费两条途径（本质都是渲染期调 `store.set.useSideEffect(key, ...args)`）：
1. **schema 节点 `$effects`**：项为 `"store.effectName"` 或 `{ "$effect": "store.effectName", "$args": [...] }`；
2. **值级 `$effect` 对象**（见 [bindings.md](bindings.md)）。

⚠️ 副作用 hook 参与组件 hooks 序列 —— 同一位置节点增删 `$effects` 需重挂载（见 SKILL.md 坑 #3）。

## 配置项（createStore options）

```js
createStore('my')(initialState, {
  isGlobal: true,
  middlewares: [myMiddleware],
  devtools: { enabled: true },
  persist: { enabled: true, name: 'my-storage' },
  persistX: { enabled: true, storage: customStorage },   // 任意 persist* 前缀 = 一条独立通道
})
```

| 选项 | 说明 |
|------|------|
| `isGlobal` | 注册进 `Runtime.stores`（见下） |
| `middlewares` | 自定义 zustand 中间件；immer 始终内置（`set.state(fn)` 一定是 draft 语义） |
| `devtools` | Redux DevTools。⚠️ **必须先在 `Runtime.init` 的 `addons` 注入 `storeDevTools`**，否则 `enabled: true` 直接抛错；`name` 自动填 store 名 |
| `persist` | 持久化（默认 localStorage）。键：`enabled`、`name`、`storage`（实现 `getItem/setItem/removeItem`）、`version`+`migrate`、`partialize`（白名单）、`merge`、`priority` |
| `persist*`（任意前缀） | 除 `persist` 外任何以 `persist` 开头的键 = 一条额外持久化通道（`persistX`、`persistSession`），配置同上；控制接口挂在 `store.<键名>` |

持久化通道运行时接口（`store.persist.*` / `store.<通道名>.*`）：`hasHydrated()`、`onHydrate(cb)`、`onFinishHydration(cb)`、`setOptions()`、`clearStorage()`。**水合是异步的**，初始渲染可能在水合完成前发生。辅助：`getAppPersistName(module)` → `` `${Runtime.config.app}-${module}` ``。

## 全局 store

`isGlobal: true` 创建时注册进 `Runtime.stores[name]`（重名抛错），生命周期脱离 React 树；**顶层** StoresProvider 解析 store 时先复用全局注册表中同名实例。适用跨视图共享、与组件生命周期无关的状态（会话、主题）。

## StoresProvider

`createView` 内部用它创建 stores，一般不需手挂。纯 React 代码中提供 store：

```jsx
import { StoresProvider } from '@xgent-ai/grafton';

<StoresProvider creators={{ tasks: tasksStore }} {...props}>
  {children}
</StoresProvider>
```

- `creators` 是工厂映射；除 `children`/`creators` 等保留 prop 外，**其余 props 都传给每个工厂**作第一参数；
- 嵌套中同名 store 默认沿用外层，工厂带 `canOverride = true` 才内层重建；
- Provider 的 props **深比较**变化时重建可重建的 store（给视图传新 `data` 重建 view store 即此机制 —— 及其局限见 SKILL.md 坑 #4）。

## React hooks（组件代码访问 store，均从 `@xgent-ai/grafton` 导出）

| hook | 签名 | 返回 |
|------|------|------|
| `useStores()` | — | 全部 stores 映射 |
| `useStore(name)` | string | 单个 store（不存在抛错） |
| `useActions(name)` | — | 该 store 的 `set`（全部动作） |
| `useStoreState(name, selector)` | `(state)=>v` | 自定义 selector 订阅 |
| `useStateActions(name, key)` | 点分路径 / path array / selector | `[值, store.set]` 元组 |
| `useSelector(pathKey, ...args)` | `"store.xxx.yyy"` | 见下 |
| `useSignal(pathKey, storesMapping?)` | `"store.key"` / `"store.a.b"` | `[value, setter]` |
| `signal(propValue)` | 组件作者用 | `[value, setter]`（见 [bindings.md](bindings.md)） |

`useSelector` 路径语义：
- **不带 args**：`"tasks.filter"` —— 首段 store 名，第二段按 selector/状态 key 取值，后续段逐级取；段支持 `.$` 操作符（`"tasks.items.$size"`）；首段以 `$` 开头时对全量 state 走路径；
- **带 args**：`useSelector('tasks.hasItem', 42)` —— 路径只允许 `store.selector` 两段。

`useSignal` setter：单段 key 走 `store.set[key]`（自动 setter **或同名 action**），多段走 `set.state` + 深路径写入。

## 内置 view store

每个视图自带（工厂 `canOverride = true`）：

- 渲染时 **`data` prop 成初始状态**（每 key 有自动 setter → `@view::xxx` 支持 `<>` / `@action::view.xxx` 可写）；
- **`selectors` prop** 追加自定义 selector（签名同 extendSelectors）；
- 内置工具 selector（`@view::_xxx`，均可配 `$args`）：

| selector | 作用 |
|----------|------|
| `_toPath(...parts)` | `Runtime.basePath` + 片段拼路径 |
| `_toHref(url, options, local)` | URL 更新（query 等） |
| `_urlJoin(...args)` | 纯拼接（不含 basePath） |
| `_isNull(v)` | `v == null` |
| `_isEmpty(v)` | 空数组/对象/字符串/null |
| `_isStrictlyEqual(a, b)` | 双非空且 `===` |
| `_isEqual(a, b)` | 深比较 |
| `_and(...args)` / `_or(...args)` | 布尔组合 |
| `_template(text, data)` | 字符串模板插值 |
| `_omitNull(obj)` | 去掉空值字段 |
| `_filterOut(data, key, value)` | 排除 `item[key]` 等于 value（或 value 数组中任一）的项；key 支持深路径 |
| `_parent` | 父视图的 view store（嵌套取上层） |

## createApiStore

普通 store + "API 获取 + 缓存 + Suspense"：每个 API 一个**惰性 selector**，首次读取才发请求，结果缓存。

```js
const productStore = () =>
  createApiStore('product')(
    { selectedId: null, total: 0 },
    {
      productList: {
        fetcher: async (api) => Runtime.$api.get_('/products'),
        transformer: (result) => { const { products, ...payload } = result; return [products, payload]; },
        onSuccess: (api, _products, payload) => api.set.total(payload.total),
      },
      selectedProduct: {
        deps: (state) => (state.selectedId ? [state.selectedId] : null),
        fetcher: async (api, id) => Runtime.$api.get_(`/products/${id}`),
      },
    },
    { /* 常规 options 可选 */ },
  );
```

| 键 | 默认 | 说明 |
|----|------|------|
| `fetcher` | 必填 | `async (storeApi, ...args) => raw` |
| `deps` | — | `(state, ...args) => argsArray \| null`：派生 fetcher 参数；返回 `null` = 依赖未就绪不发请求。⚠️ 不配 `deps` 时该 API 的 selector **不允许带参** |
| `preload` | — | `true` 或参数数组：创建 store 时立即调用一次 |
| `suspenseOnPending` | `true` | 首次加载抛 Promise 走 Suspense |
| `suspenseOnReloading` | `false` | 重载也走 Suspense |
| `minSuspensePeriod` | `2000` | Suspense 最短挂起（毫秒，防闪烁） |
| `consumeDataOnly` | `true` | selector 只返回 `data`；false 时返回 `{pending, reloading, fetched, data, error}` |
| `transformer` | 取 `result.data` | `(raw, api) => [data, payload]`。**默认解构 `{data, ...payload}`** —— ⚠️ 响应不是这形状必须自定义，否则存入 `undefined` |
| `onSuccess` | — | `(storeApi, data, payload) => {}` |
| `onError` | — | `(storeApi, error) => {}` |

自动生成：selector `use.<apiName>(...args)` / `get.<apiName>(...args)`；actions `set.apiMutate(name, data, payload)`（改写缓存）、`set.apiDrop(name)`（清缓存）、`set.apiReload(name)`（重请求，新数据到达前保留旧数据）。`deps` 返回值（深比较）变化时下次读取自动重载。

## createEntityStore

按 **id 缓存实体**（详情页/逐项加载）：

```js
const userEntities = () =>
  createEntityStore('users')({}, {
    loadById: async (id, api) => Runtime.$api.get_(`/users/${id}`),
    // 可选: suspenseOnPending / minSuspensePeriod / transformer / onSuccess / onError
  });
```

| 接口 | 说明 |
|------|------|
| `use.byId(id)` | hook；缓存未命中触发加载并抛 Promise（Suspense） |
| `use.entities` | hook；返回缓存 **Proxy**，`entities[id]` 取值即触发惰性加载 |
| `get.byId(id)` / `get.byIdNow(id)` | 同步读缓存，未命中返 null，**不触发**加载 |
| `get.entitiesCache()` | 原始缓存映射 |
| `set.mutateById(id, data, payload?)` | 直接写缓存（不请求） |
| `set.dropById(id)` | 删除缓存项 |
| `set.reloadById(id)` | 重新加载单个实体 |
| `set.prefetchById(id)` | 预取：发起加载但**不抛** Suspense |

内部键 `__entityCache` / `__entityStatus` 为实现细节，勿直接读写。

## createListStore

列表/表格数据（快照含 schema 元数据与分页游标）：

```js
const ordersList = () =>
  createListStore('orders')({}, {
    load: async (api, cursor) => Runtime.$api.get_('/orders', { cursor }),
    // load 必须 resolve 为 { schema, rows, totalRows?, cursor? }
    // 可选: transformer / suspenseOnPending / suspenseOnReloading / minSuspensePeriod / onSuccess / onError
  });
```

| 接口 | 说明 |
|------|------|
| `use.list()` | hook；首次读取触发加载（Suspense），返回 `{schema, rows, totalRows, cursor, pending, reloading, error}` |
| `get.list()` | 同步读当前快照，不触发加载 |
| `set.reload()` | 重新加载并**替换** rows |
| `set.loadMore()` | 以当前 cursor 调 `load`，结果 rows **追加** |
| `set.setCursor(c)` | 显式设置下次 loadMore 的游标 |

内部键 `__list` 为实现细节，勿直接读写。

## Runtime & HttpClient（数据获取所需）

`Runtime` 全局单例，`Runtime.init(config, mode)` 应用入口调一次（先于任何视图渲染）。`mode` 以 `'production'` 开头视为生产，反映在 `Runtime.isDevMode`。完整 app 配置（grafton-app.json / services / i18n / addons）属 `web-app-dev` skill 范畴；view 开发只需知道：

- `services` 每个条目实例化为 `HttpClient`，挂在 **`Runtime.$<服务名>`**。`Runtime.createServiceClient(nameOrOptions)` 手建。
- `Runtime.$<name>` 的方法（带下划线后缀）：`get_(path, query?, options?)`、`delete_(path, query?, options?)`、`post_(path, body?, query?, options?)`、`put_(...)`、`patch_(...)`，返回解析后数据的 Promise。
- options 常用键：`endpoint`（覆盖端点）、`headers`（鉴权 token）、`timeout`、`withCredentials`、`onSending`、`onResponse`/`onResponseError`/`onOtherError`、`formData`/`fileField`/`fileName`/`onProgress`（上传）、`usePipe`（流式）。

```js
const list = await Runtime.$api.get_('/tasks', { page: 1 }, { timeout: 5000 });
const created = await Runtime.$api.post_('/tasks', { title: 'x' }, null, { headers: { Authorization: `Bearer ${token}` } });
```

其他静态成员：`Runtime.config`、`Runtime.i18n`（`changeLanguage`/`language`）、`Runtime.logger`、`Runtime.stores`、`Runtime.toLocalPath(...parts)`、`Runtime.registerGlobalStore(store)`、结构化日志 `Runtime.log`/`logEvent`/`logError`/`logApi`/`logAction`。
