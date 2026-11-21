# 浏览器性能分析：从一次慢操作到一份可读的记录

`profiler` 在 Chromium 浏览器中记录 Chrome Trace Event。它能把输入、JavaScript、样式布局和绘制放回同一条时间线。开始前先写出要解释的区间，例如「从点击保存到订单 B 的回执出现」。这样能排除打开页面、登录和查看报告本身的耗时。

```formal
WORKFLOW @profile_interaction:
  TRIGGER: browser_performance_investigation
  DO: [define_user_visible_start_and_end, select_observation_tool,
       prepare_reproducible_page_state, start_recording,
       exercise_target_path_once, stop_and_save_trace,
       inspect_matching_interval, report_evidence_and_uncertainty]
RULE @capture_boundary:
  WHEN: interaction_failed_or_timed_out
  MUST: [stop_active_recording, preserve_partial_observation]
RULE @attribution:
  WHEN: proposing_performance_cause
  MUST: locate_supporting_events_in_relevant_thread_and_interval
  MUST_NOT: infer_causation_from_total_event_count
```

## 先选证据

| 想回答的问题 | 工具 | 用什么作判断 |
| --- | --- | --- |
| 这次加载或交互的体验指标是多少？ | `vitals --json` | 读取本次测量可用的 LCP、CLS、INP 等；缺值和零分开记录 |
| 是哪些 React 组件提交了渲染？ | `react renders start` / `react renders stop --json` | 按组件看提交次数、状态变化和耗时。需要在页面启动前开启 hook，见[React 运行时诊断](react.md) |
| 慢在脚本、布局、绘制，还是等待？ | `profiler start` / `profiler stop <path>` | 在原始浏览器时间线定位任务、线程和事件。要分析 JS 调用栈，再检查记录中是否含 CPU 采样 |
| 只是要证明操作成功 | [页面快照](snapshots.md)或[文件验收](files.md) | 订单状态或实际交付物即可。录到一段 trace 不能证明文件可读或订单已保存 |

## 例：保存草稿时出现卡顿

下面是 POSIX shell 示例，`$APP_URL` 为任务页面。若任务针对指定的现有浏览器，按[CDP 连接](cdp-mode.md)选择会话。以下 ref 和回执文字要换成目标页的实际值。

```sh
ab_perf="draft-perf-$$"
agent-browser --session "$ab_perf" open "$APP_URL"
agent-browser --session "$ab_perf" snapshot -i
```

导航和准备做完后再开始录制。有现成的 `performance.mark()` 或 `performance.measure()` 时，把相应标记纳入记录。

```sh
agent-browser --session "$ab_perf" profiler start --categories "devtools.timeline,v8.execute,blink.user_timing"
agent-browser --session "$ab_perf" click @e4
agent-browser --session "$ab_perf" wait --text "草稿已保存"
agent-browser --session "$ab_perf" profiler stop ./save-draft.json
```

把 `stop` 当作需要单独执行的收尾步骤：中间的点击或等待失败时也要执行，避免用整条 `&&` 链把收尾跳过。要测首次加载则改变起点，先在空白测试页启动记录，再导航到目标 URL。涉及用户已有表单时不要为了录制重载该页。

## 收窄采集范围

| 要观察什么 | 适合的类别 | 解读注意点 |
| --- | --- | --- |
| 尚未定位耗时来源 | 先用默认 `profiler start` | 自定义类别会改变采集内容；过早收窄可能漏掉真正的瓶颈 |
| 脚本或长任务 | `devtools.timeline,v8.execute`；需要采样时加入 `disabled-by-default-v8.cpu_profiler` | JS 执行区间表明线程在运行脚本；定位到函数还需要可读的采样或调用栈 |
| 样式、布局、绘制 | `devtools.timeline,blink` | 看同一帧中更新的先后，单个布局事件无法证明发生了反复强制布局 |
| 输入到显示的等待 | `latencyInfo,devtools.timeline` | 结合主线程与渲染事件，区分输入排队和处理后迟迟未呈现 |
| 应用已经有业务标记 | `blink.user_timing` 加上与假设相关的类别 | 按具体业务标记查找操作区间；没有标记时回到输入事件与目标帧 |

## 验收记录，形成结论

1. **文件可解析**：检查指定路径确实写出了 JSON，`traceEvents` 中有事件。可在 Chrome DevTools 的 Performance 面板选择 Load profile，或用 Perfetto 打开本地文件。
2. **目标操作在里面**：按应用标记或输入事件缩小时间窗，再按 `pid`、`tid` 找相关线程。`ph` 表示事件类型，完整事件常见 `ph: "X"`，其 `ts` 与 `dur` 以微秒记录。嵌套区间会重叠，直接累加所有 `dur` 会重复计算。
3. **区分测到的现象与推断**：例如本地演练可以从记录中找到应用产生的 `save-draft` 事件，并从页面读到成功回执。这能确认覆盖了目标操作。若要归因为脚本或布局，还需指出该区间内哪条线程的哪些事件占用了时间。
4. **比较有共同条件**：复用相同输入、浏览器模式、前后台状态、缓存条件及采集类别；标明比较的是首次加载、热加载还是一次交互。单次 trace 用于定位，再用同条件的观测核对改动效果。

`profiler` 依赖 Chromium 的追踪能力。记录期间事件积存在内存中，时间过长可能触及事件上限；浏览器失去响应时，停止阶段也可能超时。将这类结果标为不完整，先保留实际生成的文件。有可用记录后再关闭本次创建的临时会话。

来源：[agent-browser Profiler](https://agent-browser.dev/profiler) · [Perfetto 对 Chrome JSON 事件的说明](https://perfetto.dev/docs/getting-started/other-formats)。
