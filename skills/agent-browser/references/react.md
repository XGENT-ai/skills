# React 运行时诊断与 Web Vitals

遇到「点了保存，界面没变」时，先在目标记录的状态区取完整快照。输入值、按钮状态和保存回执分别证明不同的事。[快照与 ref](snapshots.md) 能回答用户看到了什么；需要解释这次更新在哪个组件发生时，再采集 React 证据。

```formal
WORKFLOW @diagnose_react:
  TRIGGER: react_runtime_diagnosis_requested
  DO: [define_visible_symptom, enable_hook_before_app_boot,
       confirm_mounted_component, isolate_one_interaction,
       correlate_component_observation_with_page_result]
RULE @component_identity:
  WHEN: inspecting_component
  MUST: use_id_column_from_current_component_tree
RULE @render_attribution:
  WHEN: interpreting_render_profile
  MUST: separate_commit_count_from_user_visible_delay
RULE @metrics:
  WHEN: timing_or_interaction_metric_unavailable
  MUST: report_missing_measurement
  MUST_NOT: interpret_missing_as_zero
```

## 先决定采集什么

| 现场问题 | 采集路径 | 可以得到的结论 |
| --- | --- | --- |
| 点击后业务状态正确吗？ | 页面快照 → 点击 → 目标记录的回执 | 用户可见的结果；通常无需开启 React hook |
| 哪个组件接收了什么数据？ | `react tree --json` → `react inspect <fiberId> --json` | 该组件实例当前可见的 props、hooks、state 和来源线索 |
| 一次点击涉及哪些渲染？ | `react renders start` → 一次交互 → 等结果 → `react renders stop --json` | 该时间段有提交的组件、次数及可用的耗时或变化原因 |
| 加载占位为什么还在？ | 同时采集区域快照和 `react suspense --json` | fallback 是否可见、工具识别了哪些边界、是否有可定位的阻塞线索 |
| 是首屏慢、布局移动，还是交互慢？ | `vitals --json`，按指标和用户操作解读 | 本次页面访问的浏览器测量；此命令也可用于非 React 页面 |

## 打开带 hook 的会话，找到正确的实例

React DevTools hook 要在页面脚本执行前装入。对已经启动且没有 hook 的浏览器追加该选项可能会重启会话。如果当前页面里还有未提交的表单，在新命名会话中复现，保留原现场。下例使用 POSIX shell；后续各命令沿用同一个会话。

```sh
ab_react="react-check-$$"
agent-browser --session "$ab_react" open --enable react-devtools "$APP_URL"
agent-browser --session "$ab_react" react tree --json
```

等待目标页面挂载后，从 `data.tree` 读取表头和行。在本地示例中，表头给出 `depth id parent name`，一行是 `3 4 3 DraftCounter`：组件名是 `DraftCounter`，要传给 `react inspect` 的是第二列 `4`。若页面同时有多个同名实例，结合父组件、key 以及 inspect 中的记录标识选中目标。页面导航后重新取树。页面里的 `@e4` 和这里的 `4` 属于不同的编号系统。

若纯文本只显示完成提示，先读 JSON 中的树；若树里只有表头，则检查是否在正确的页面、目标 React 根是否已挂载、是否在启动时启用了 hook。inspect 的 `source` 可能为空，这时保留组件名称和已经观测到的 props/state。

## 把一次点击和一次状态变化对上

例：初始页面显示 `Draft 0`，点击 `Save draft` 后应显示 `Draft 1`。先取交互快照，再按当前输出替换示例中的 ref。把录制区间收窄到这一次操作。

```sh
agent-browser --session "$ab_react" snapshot -i
agent-browser --session "$ab_react" react renders start
agent-browser --session "$ab_react" click @e3
agent-browser --session "$ab_react" wait --text "Draft 1"
agent-browser --session "$ab_react" react renders stop --json
agent-browser --session "$ab_react" snapshot
```

本地 React 18 演练中，报告列出了父组件和 `DraftCounter`，其中子组件记录的是第一个 state hook 从 0 变为 1；页面状态区同时出现 `Draft 1`。这两条证据分别说明内部状态发生了变化、这次变化也呈现在页面上。

如果 state 已变而页面结果错误，继续检查消费该值的子组件和对应区域。如果报告里出现多次渲染，先看涉及多少实例、是首次挂载还是重新渲染、耗时集中在谁身上，再结合可见延迟定位。同一录制区间内发生的后台刷新也会进入报告。若等待结果超时，仍执行 `react renders stop --json` 收回报告，保留失败时的快照。

## 区分 Suspense 报告与当前画面

在占位仍可见时取一次 `react suspense --json` 和区域快照；数据完成后，再观察同一区域是否已经显示内容。`--only-dynamic` 适合从大量边界中筛选候选。筛选后没有条目时，去掉过滤参数看完整报告。

在本地延迟加载示例中，内容已经从 `Report pending` 切换到 `Report ready`，分析器仍将该边界归为 dynamic，并把 blocker 报为 unknown。因此判断「用户还在等吗」要读取当前区域；判断「是什么让它挂起」要用报告实际给出的线索。仅有 dynamic 分类时，具体阻塞源仍待确认。

## 读性能指标并验证客户端导航

组件报告能说明谁重新渲染。若还要解释输入为何排队、布局或绘制是否耗时，按[性能分析](profiler.md)录下同一操作的浏览器时间线。

- `vitals --json` 读取当前页面；`vitals <url> --json` 指定测量地址。TTFB、FCP 和 LCP 分别帮助定位首字节、首次内容绘制和最大内容绘制；CLS 衡量布局位移，INP 反映可测交互的响应延迟。CLS 是分值，其余这些时间指标使用毫秒。比较两次运行时记录页面、加载方式和执行过的交互。
- React profiling 构建被识别后，报告可包含 hydration 阶段。普通客户端挂载页面可能得到 `hydration: null`，一次短会话也可能得到 `inp: null`。交付时列出实际测得的字段，这类缺值不能证明耗时为零。
- 要验证真实菜单或链接，点击该控件，再核对 URL 和目的页内容。专门验证客户端路由时可用 `pushstate /reports`、`wait --url "**/reports"`，随后取完整快照确认报表标题或主要内容。该命令在支持的 Next.js 页面会尝试路由器导航，其他环境可能依赖 History API 与导航事件；某些应用的 URL 会变而视图不会更新。要测直接访问或完整加载，另用 `open`。
- 结束时关闭本次创建的诊断会话：`agent-browser --session "$ab_react" close`。

功能与命令来源见[官方 React 与 Web Vitals 参考](https://agent-browser.dev/react)。
