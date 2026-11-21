# CDP：接手指定浏览器，保留用户现场

要继续操作已经打开的标签，先分清三个对象。**端点**决定连上哪个浏览器，**session 名**让后续命令沿用这条连接，**标签标识**决定操作落在哪个页面。两个 session 连到同一个 Chrome，站点数据仍可能共享。需要隔离账号时参见[会话选择](sessions.md)。

```formal
WORKFLOW @attach:
  TRIGGER: existing_browser_or_cdp_endpoint_requested
  DO: [select_authorized_endpoint, attach_named_session, list_targets,
       identify_requested_tab, verify_url_and_page_identity]
  OUTPUT: confirmed_browser_and_tab
RULE @attachment_failure:
  WHEN: target_browser_unavailable
  MUST: report_connection_failure
  MUST_NOT: substitute_unrelated_browser
RULE @lost_binding:
  WHEN: pinned_tab_closed
  MUST: reidentify_intended_target_before_action
RULE @release:
  WHEN: owned_attachment_finished
  MUST: [detach_named_session, preserve_user_tabs_and_browser]
```

## 选连接方式

| 已知条件 | 做法 | 怎样确认 |
| --- | --- | --- |
| 有已开启调试的本地 Chrome 或 Chromium 应用端口 | `agent-browser --session review connect 9222`；将端口换成实际值 | 接下来带同一 session 执行 `tab list`。即使省略 `--cdp`，会话仍应连着该浏览器 |
| 用户指定当前 Chrome，已开启远程调试，端口未知 | 在新命名会话中用 `--auto-connect tab list`。若 Chrome 弹出连接许可，等用户在浏览器中批准后再读标签 | 核对目标 URL、页标题或业务对象。自动发现只说明连上了一个可调试实例 |
| 已有远程服务提供的 CDP 地址 | 使用 `--cdp "$CDP_ENDPOINT"`。受保护的握手支持 `--cdp-headers`，也可由服务配置提供配套的 `AGENT_BROWSER_CDP` 与 `AGENT_BROWSER_CDP_HEADERS` | 握手认证和页面登录分别核对。页面请求的 `--headers` 不能代替握手凭据；报告中省略带密钥的地址和请求头 |
| 服务商负责新建浏览器 | 已配置 provider 时用 `-p <provider>`；已给出浏览器端点时直接连接即可 | 先确认服务返回浏览器还是单页连接。单页连接的追踪和性能记录以该页面为范围 |

Chrome、Electron、WebView2 都需要暴露可用的 CDP 端点。为本地测试**自行启动** Chrome 时，可在启动参数中给出 `--remote-debugging-port=<端口>` 和专用 `--user-data-dir=<测试目录>`。Chrome 136 起，使用端口开关调试默认数据目录会受限。接手用户已有标签时，优先沿用已配置的端点。[Chrome 调试配置说明](https://developer.chrome.com/blog/remote-debugging-port)

远程服务一般给出完整的 `wss://` 浏览器地址。若平台将端点和握手头配套注入为上述环境变量，可在新会话中执行 `agent-browser --session "$ab_session" tab list`。若只拿到 URL，则用 `agent-browser --session "$ab_session" --cdp "$CDP_ENDPOINT" tab list`。收到握手认证错误时核对端点及其配套凭据；页面已经加载后出现登录提示，则按网站登录流程处理。

## 接手已有标签

假设目标是浏览器中已打开的订单 B。先读取标签，不要用 `open` 重新导航，以免覆盖未提交内容。下例为 POSIX shell，`t2` 仅代表这次列表返回的目标标识。

```sh
ab_session="review-$$"
agent-browser --session "$ab_session" connect "$CDP_PORT"
agent-browser --session "$ab_session" tab list
agent-browser --session "$ab_session" tab t2
agent-browser --session "$ab_session" get url
agent-browser --session "$ab_session" snapshot
```

找到订单 B 的业务标识后再操作。多个标签使用相同 URL 时，用标题、订单号或账号消除歧义。接手的是用户已在使用的 agent-browser 会话，就按用户要求保留连接；若此次只创建了一个用于连接的临时会话，用 `agent-browser --session "$ab_session" close` 释放它。外部 Chrome 由启动方管理。`tab close` 会实际关闭标签，含义不同。

## 共用 Chrome 时固定标签

若任务允许在新标签开始，支持固定标签的版本可在**首次连接**时运行：

```sh
agent-browser --session "$ab_session" --cdp "$CDP_PORT" --pin-tab open "$TASK_URL"
```

尚无绑定时会创建新标签。已有标签任务应先按上节识别目标；不要把第一次固定时新开的空页当成用户的原页。固定以 CDP target ID 绑定，其他人切到前台的标签不会接走当前操作。支持时用 `--no-pin-tab` 显式解除。可从本机 `--help` 确认版本支持。

| 现象 | 下一步 |
| --- | --- |
| `tab_locked` | 目标正由另一个活跃会话固定。核对该任务应使用的 session；确需交接时由持有方释放 |
| `tab_gone` | 原绑定已消失。执行 `tab list` 核对剩余页面；只有任务允许重开才用 `tab new <url>`。重新取快照，勿复用旧页 ref |
| 多次操作读到不同页面 | 核对每条命令的 session、URL 和目标标签。曾用 `connect` 的会话会记住连接；改环境变量不能证明已经切换端点 |
| 后台页动画或计时变慢 | Chrome 会节流后台标签。受版本支持时可用 `tab keep live` 维持工作；比较性能时先让各次测试采用相同的前后台条件 |
| 连接成功，下载却没有出现在本机 | 附着浏览器管理下载位置。按[下载交付](files.md#下载只触发一次)核对浏览器侧路径及文件可达性 |

若要复现深色模式等显示问题，可在该连接上指定 `--color-scheme dark`，并记录实际显示条件。它会影响该会话之后的页面观察。

来源：[CDP 模式](https://agent-browser.dev/cdp-mode)。
