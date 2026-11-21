# 按任务查命令

以下给出 `agent-browser` 后的参数。尖括号表示需要替换的值；示例中的 ref 要从本次快照取得。要查当前版本某个命令的全部参数，在本机运行 `agent-browser <子命令> --help`，例如 `agent-browser wait --help`。

```formal
RULE @result_condition:
  WHEN: waiting_after_action
  PREFER: condition_distinguishing_before_and_after
RULE @artifact_delivery:
  WHEN: local_file_requested
  MUST: confirm_written_path_before_closing_session
RULE @batch_boundary:
  WHEN: next_action_depends_on_intermediate_output
  MUST: inspect_output_before_selecting_next_action
```

## 定位、输入与读值

| 目的 | 子命令示例 | 如何确认 |
| --- | --- | --- |
| 打开或返回页面 | `open <url>`、`back`、`forward`、`reload` | `get url` 和 `get title`；需要核对正文时取完整快照 |
| 从页面找控件 | `snapshot -i`、`find role button click --name "预览"` | 第二条会直接点击，仅在角色和名称已能唯一确定目标时使用 |
| 更改表单 | `fill @e3 "杭州"`、`select @e4 "CN"`、`check @e5`、`uncheck @e5` | `get value @e3`、`is checked @e5`；按钮是否可用看 `is enabled @e6` |
| 测键盘交互 | `focus @e3`、`press Tab`、`press Enter` | 对可能提交表单的按键，先明确预期结果，再检查结果区 |
| 提取页面数据 | `get text @e3`、`get attr @e4 href`、`get count ".result"` | 读正文与字段值分别用 `get text` 和 `get value`；脚本取数可加 `--json` |

## 等待条件

用能区分动作前后的条件。比如检索词会先出现在输入框里，等待页面出现检索词可能过早结束；可以等结果区的第一条记录，再读取记录内容。

| 预期变化 | 子命令示例 |
| --- | --- |
| 跳转到结果页 | `wait --url "**/results"`，再用 `get url` 核对实际地址 |
| 显示回执 | `wait --text "草稿已保存"`，再取快照核对草稿名称 |
| 加载占位消失 | `wait ".loading" --state hidden`，再读结果区或空状态 |
| 指定元素出现 | `wait "#results li"`；如果无结果也是合法结局，应同时考虑页面的空状态 |

等待超时后读取当前 URL 和快照：错误提示、登录重定向或「无结果」都比再点一次按钮更能说明原因。

## 标签页与内嵌页面

用 `tab list` 读取标签标识，接着用 `tab <标识>` 切换。照抄实际返回的标识，例如 `t2`。切换后重新取快照。需要自己开页时用 `tab new <url>`，需要结束单个标签时用 `tab close <标识>`。跨 frame 的选择见[快照与 ref](snapshots.md)。

## 文件和证据

若交付物是导入回执、下载文件、截图、PDF 或粘贴结果，按[文件与剪贴板](files.md)选择命令和验收条件。

## 批处理

若步骤之间只依赖命令成功，可以用 `&&`。已知选择器时，也可把多个子命令交给 `batch --bail`，遇错即停。例如 `agent-browser batch --bail "get title" "get url"` 一次取两个值。若下一步要根据快照选 ref，先读取快照再组织下一批。部分命令已执行时，从页面现状续做，尤其要先查清提交结果。

## 调试与其他入口

| 场景 | 本地可用的入口 |
| --- | --- |
| 页面没按预期更新 | `errors` 看页面异常，`console` 看控制台，`network requests` 查看请求记录 |
| 需要复现尺寸或外观 | `set viewport 1280 800`、`set media dark`；比较状态前后可用 `diff snapshot` |
| 要保留回放或分析耗时 | 录制画面用 `record start flow.webm` 与 `record stop`；需要定位浏览器线程和事件耗时时，按[性能分析](profiler.md)录制并解读 trace |
| 原生对话框挡住操作 | `dialog status` 确认内容，再按目标选择 `dialog accept` 或 `dialog dismiss` |
| 按任务找专门功能 | 组件、交互性能与 SPA 导航见[React 与 Web Vitals](react.md)。可访问性用 `a11y --help`，浏览器存储用 `storage --help`，配置 MCP 用 `mcp --help`。顶层 `--help` 可列出本机版本的其余命令组 |

来源：[官方命令参考](https://agent-browser.dev/commands)。
