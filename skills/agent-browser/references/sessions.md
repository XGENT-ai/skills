# 会话隔离与持久化

```formal
RULE @account_isolation:
  WHEN: independent_accounts_required
  MUST: [separate_browser_sessions, separate_persistent_state,
         confirm_visible_account_in_each_session]
RULE @login_reuse:
  WHEN: continuing_authenticated_task
  MUST: [reuse_selected_profile_or_restore_key, verify_visible_identity]
RULE @shared_browser:
  WHEN: sessions_share_cdp_browser
  MUST: [verify_tab_binding, account_for_shared_site_storage]
```

## 先决定哪些状态应该共享

| 实际任务 | 选法 | 如何核对 |
| --- | --- | --- |
| 同一次表单操作分几次执行 | 各条命令带同一个 `--session <name>`。后台浏览器会保留页面和已填内容 | `session list` 列出活动会话，`get url` 核对当前页 |
| 同时验证买家、卖家两个账号 | 各用独立命名会话，各自启动浏览器并登录。两个账号分别使用自己的 profile 或恢复键 | 在两边分别读账号标识。仅看到两个标签页不足以证明账号隔离 |
| 要在下次运行继续同一个测试账号 | 用专用目录 `--profile <目录>` 保留浏览器数据，或用稳定的会话名加 `--restore` 保存和恢复站点状态 | 重启后访问需要登录的页面，读取该账号可见的内容 |
| 用户要求操作已经打开的页 | 沿用用户指定的连接和标签。在同一 Chrome 上用 CDP 建多个会话时，它们仍共享该浏览器里的站点数据 | 先列出标签再核对页面身份，连接选择和标签丢失的恢复见 [CDP 模式](cdp-mode.md) |

## 一个双角色用例

以下以 POSIX shell 为例。将两个 URL 变量设为实际入口。分别登录后，从各自快照辨认当前账号，再在对应会话中执行后续步骤。

```sh
buyer="buyer-$$"
seller="seller-$$"
agent-browser --session "$buyer" open "$BUYER_URL"
agent-browser --session "$seller" open "$SELLER_URL"
agent-browser --session "$buyer" snapshot
agent-browser --session "$seller" snapshot
```

两边的元素编号可能相同。买家页得到的 ref 只用于买家页。收工时分别对自己创建的两个会话执行 `close`。

## 恢复以后如何判断

- **需要保留哪一类数据？** 专用 profile 目录适合复用整个浏览器配置。若只关心指定站点登录状态，可用 `--session catalog-qa --restore`；每次复用同一个恢复键。状态文件可能包含有效会话凭据，放在用户选定的私有目录，避免进入仓库或报告。
- **为什么打开了登录页？** 先用 `session info --json` 核对会话和恢复信息，再核对域名及页面上实际显示的账号。状态文件存在也可能遇到服务端登录过期，由用户在该任务会话中重新登录即可。
- **为什么操作跑到另一个页面？** 先看 `tab list`。共享 CDP 环境可用 `--pin-tab` 绑定选中的标签；若收到 `tab_gone`，从当前列表重新确认目标。遇到用户已有会话，结束时保留现场。
- **为什么中途丢了页面？** 改动 profile、代理或浏览器可执行路径等启动参数可能引起浏览器重启。需要不同设置时，先选好新会话，再导航和取快照。

来源：[官方会话文档](https://agent-browser.dev/sessions)。
