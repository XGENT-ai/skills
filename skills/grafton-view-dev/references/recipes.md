# Recipes：可复用 worked 示例

可直接抄改的完整模式。每个示例只给 schema/代码 + 一句上下文；语义细节去对应主题文件查（[bindings](bindings.md) / [schema](schema.md) / [stores](stores.md) / [components](components.md)）。

## 1. 一个完整 view（store + schema + createView）

```js
// stores/tasks.js —— 导出工厂函数
import { Runtime, createStore } from '@xgent-ai/grafton';

const tasksStore = () =>
  createStore('tasks')({ tasks: [], filter: 'all', loading: false, loadError: null })
    .extendSelectors(() => ({
      total: (s) => s.tasks.length,
      remaining: (s) => s.tasks.filter((t) => !t.done).length,
      visibleTasks: (s, filter) =>
        filter === 'active' ? s.tasks.filter((t) => !t.done)
        : filter === 'done' ? s.tasks.filter((t) => t.done)
        : s.tasks,
    }))
    .extendActions((api) => ({
      addTask: (title) => api.set.state((d) => { d.tasks.push({ $key: 'id', id: Date.now(), title, done: false }); }),
      toggleTask: (id) => api.set.state((d) => { const t = d.tasks.find((x) => x.id === id); if (t) t.done = !t.done; }),
      clearDone: () => api.set.state((d) => { d.tasks = d.tasks.filter((t) => !t.done); }),
    }));
export default tasksStore;
```

```jsx
// pages/home.jsx
import { createView } from '@xgent-ai/grafton';
import { Button } from '@xgent-ai/ui-shadcn';            // 非内置组件必须注册
import schema from '../views/home.view.json';
import tasksStore from '../stores/tasks';

export default createView({
  displayName: 'HomePage',
  ns: 'taskboard',
  stores: { tasks: tasksStore },                       // view store 自动附加
  registry: { Button },
  schema,
});
```

> 视图必须渲染在 `RouterProvider` 内（内置导航动作依赖 router）。

## 2. 数据驱动列表 + 空状态

推荐写法:单个 `List`,网格类挂 `className`(真实容器),空态用 `emptyState` slot —— 一个节点搞定,不用对偶 `$when`:

```jsonc
{
  "$type": "List",
  "variant": "none",
  "className": "grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3",
  "data": "@store::tasks.tasks",
  "itemTemplate": {
    "$type": "HBox", "$style": "items-center gap-3 rounded-lg border p-3",
    "$content": [
      {
        "$type": "Button", "variant": "ghost", "size": "icon-sm",
        "$content": { "$if": "@local::data.done", "$then": "✓", "$else": "○" },
        "onClick": { "$action": "@action::tasks.toggleTask", "$args": ["@local::data.id"], "$withEventArgs": false }
      },
      { "$type": "Text", "text": "@local::data.title", "noTranslate": true,
        "$style": { "$if": "@local::data.done", "$then": "line-through text-muted-foreground", "$else": "" } }
    ]
  },
  "emptyState": {
    "$type": "VBox",
    "$style": "items-center gap-2 rounded-lg border border-dashed p-8 text-muted-foreground",
    "$content": [{ "$type": "Text", "text": "app.empty" }, { "$type": "Text", "text": "app.emptyHint" }]
  }
}
```

要点：列表 key 由**数据**携带（store 里 `$key: 'id'`）；空态优先用 `emptyState` slot(`List`/`PlainList`/`DataList` 都支持);per-item action 用 `$args: ["@local::data.id"]` + `$withEventArgs: false`。

> **网格布局**：网格类挂在 `List` 的 `className` 上（`List` 是真实 `ul`/`ol` 容器）。**别**把网格容器放进 `before`/`after` slot —— 那是夹在列表项前后的兄弟节点、不包裹卡片,网格不生效。`PlainList` 容器是 `Fragment`、做不了网格,要网格卡片就用 `List`。
>
> **空状态**:首选 `emptyState` slot(如上,data 空/null 时渲染,进入该分支时 before/after/列表项都不渲染)。另一种是不挂 slot、在列表外用 `$when:_isEmpty` 的对偶节点控制 —— 两者皆可,slot 更简洁。详见 [components.md](components.md)。

## 3. 双向绑定过滤器（带参 selector + `<>`）

```jsonc
// 过滤控件:必须 <> 绑定(bindable 受控组件传静态值只读)
{ "$type": "FilterTabs", "value": "<>@store::tasks.filter",
  "options": [ { "value": "all", "label": "All" }, { "value": "active", "label": "Active" }, { "value": "done", "label": "Done" } ] }

// 列表把过滤状态接进带参 selector,$ifNull 兜底
{ "$type": "List", "data": { "$selector": "@store::tasks.visibleTasks", "$args": [{ "$ifNull": "@store::tasks.filter", "$then": "all" }] } }
```

## 4. 异步加载 + loading / 错误态

```js
// store action(放在 extendActions 内)
loadTasks: async () => {
  api.set.loading(true); api.set.loadError(null);
  try {
    const data = await Runtime.$api.get_('/tasks');
    api.set.tasks(data.map((t) => ({ ...t, $key: 'id' })));   // 顺手带列表 key
  } catch (err) {
    api.set.loadError(err);                                    // 存 Error,ErrorPanel 读 error.message
  } finally {
    api.set.loading(false);
  }
},
```

```jsonc
[
  { "$type": "Button", "variant": "outline", "$content": "Reload",
    "disabled": "@store::tasks.loading", "onClick": "@action::tasks.loadTasks" },
  { "$type": "Text", "text": "Loading...", "noTranslate": true, "$when": "@store::tasks.loading" },
  { "$type": "ErrorPanel", "error": "@store::tasks.loadError",
    "$when": { "$selector": "!@view::_isNull", "$args": ["@store::tasks.loadError"] } }
]
```

> 进场自动加载用节点 `$effects` + store `extendSideEffect`（见 [stores.md](stores.md)）；或惰性用 `createApiStore`。

## 5. 自定义 widget：slots + bindable(signal)

```jsx
// slots widget —— 插槽值是 schema,grafton 渲染成 ReactNode 再传入
function TaskItem({ title, done, statusIcon, actions, className }) {
  return (<div className={cn('flex items-center gap-3 rounded-lg border p-3', className)}>
    {statusIcon}<span className={cn(done && 'line-through text-muted-foreground')}>{title}</span>
    <div className="ml-auto flex gap-1">{actions}</div></div>);
}
TaskItem.grafton = { leaf: true, slots: ['statusIcon', 'actions'] };

// bindable widget —— signal() 解包;传静态值时 setCurrent 是 no-op(退化只读)
import { signal } from '@xgent-ai/grafton';
function FilterTabs({ value, options = [] }) {
  const [current, setCurrent] = signal(value ?? 'all');
  return (<div className="inline-flex rounded-lg border p-0.5">{options.map((o) => (
    <button key={o.value} type="button" onClick={() => setCurrent(o.value)}
      className={cn('rounded-md px-3 py-1 text-sm', current === o.value && 'bg-secondary font-medium')}>{o.label}</button>
  ))}</div>);
}
FilterTabs.grafton = { leaf: true, bindable: ['value'] };
```

```jsonc
// TaskItem 的插槽 schema 里照常能用 @local::(列表项 payload 可见)
{ "$type": "TaskItem", "$mapping": { "title": "title", "done": "done" },
  "statusIcon": { "$type": "Button", "variant": "ghost", "size": "icon-sm",
    "$content": { "$if": "@local::data.done", "$then": "✓", "$else": "○" },
    "onClick": { "$action": "@action::tasks.toggleTask", "$args": ["@local::data.id"], "$withEventArgs": false } } }
```

## 6. 表单（Form + 字段 + onSubmit + 实时预览）

```jsx
import { Form, FormInput, FormSelect, ButtonWidget } from '@xgent-ai/ui-shadcn/widgets';
// createView registry 里加: { Form, FormInput, FormSelect, ButtonWidget }
```

```jsonc
{
  "$type": "Form", "className": "flex items-end gap-2",
  "defaultValues": { "title": "", "priority": "normal" },
  "onSubmit": "@action::tasks.addFromForm",
  "$content": [
    { "$type": "FormInput", "name": "title", "label": "New task", "rules": { "required": true } },
    { "$type": "FormSelect", "name": "priority", "label": "Priority",
      "items": [ { "value": "normal", "label": "Normal" }, { "value": "high", "label": "High" } ] },
    { "$type": "ButtonWidget", "type": "submit", "text": "Add" },
    // 实时预览:Form 子树内 @form::watch 读字段
    { "$type": "Text", "className": "text-xs text-muted-foreground", "noTranslate": true,
      "text": { "$selector": "@form::watch", "$args": ["title"] },
      "$when": { "$selector": "!@view::_isEmpty", "$args": [{ "$selector": "@form::watch", "$args": ["title"] }] } }
  ]
}
```

```js
// action 收到表单值对象
addFromForm: (values) => api.set.state((d) => {
  d.tasks.push({ $key: 'id', id: Date.now(), title: values.title, done: false, priority: values.priority });
}),
```

## 7. i18n 语言切换（逻辑放 store action）

```js
switchLanguage: () => {
  const i18n = Runtime.i18n;
  i18n.changeLanguage(i18n.language?.startsWith('zh') ? 'en' : 'zh');
},
```
```json
{ "$type": "Button", "variant": "ghost", "size": "sm", "$content": "中 / EN", "onClick": "@action::tasks.switchLanguage" }
```

## 8. 多页应用：页面级树按路由 key 重挂载（必做）

不同 schema = 不同 hooks 序列，React 同位复用会崩（"change in the order of Hooks called by Grafton"）。路由出口给页面包带 key 的容器：

```jsx
import { useLocation } from '@xgent-ai/router-lite';
function PageOutletWrapper({ children }) {
  const { pathname } = useLocation();
  return <Fragment key={pathname}>{children}</Fragment>;
}
```
