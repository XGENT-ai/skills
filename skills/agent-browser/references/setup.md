# 安装与诊断

```formal
WORKFLOW @check_installation:
  DO: [locate_project_or_path_executable, run_version_check,
       probe_blank_page_in_fresh_session, verify_returned_url]
RULE @missing_cli:
  WHEN: cli_absent_and_installation_requested
  MUST: select_install_scope_from_project_or_user_context
RULE @launch_failure:
  WHEN: cli_runs_but_browser_launch_fails
  MUST: distinguish_missing_browser_from_missing_system_libraries
RULE @site_failure:
  WHEN: blank_page_succeeds_but_target_fails
  MUST: inspect_target_url_service_or_login
```

## 检查当前环境

在实际执行任务的终端运行 `agent-browser --version`。如果命令查找失败，macOS / Linux 用 `command -v agent-browser` 查看 PATH，PowerShell 用 `Get-Command agent-browser -ErrorAction SilentlyContinue`。如果找到了路径但执行报错，保留错误信息，检查它指向的包管理器或版本管理器。若项目已声明该依赖，先在项目环境核对本地命令。

## 选择安装方式

根据用途选一行。安装 CLI 后重新运行该行的版本检查。表中的 `npx` 可能在首次运行时联网下载。

| 用途 | 安装或试用 | 版本检查与后续调用 |
| --- | --- | --- |
| 在多个项目长期使用，已有 npm | `npm install -g agent-browser` | `agent-browser --version`，后续用 `agent-browser` |
| macOS，平时由 Homebrew 管理 CLI | `brew install agent-browser` | `agent-browser --version`，后续用 `agent-browser` |
| 团队项目或 CI，需要可复现的版本 | 有现成依赖就按仓库锁文件安装。新接入的 npm 项目若仅用于开发验证，可用 `npm install --save-dev agent-browser` 并纳入锁文件 | 在项目目录执行 `npx agent-browser --version`，后续用 `npx agent-browser` |
| 只想试一次，已有 npx | `npx agent-browser --version` 即可试用 | 本次每条命令都用 `npx agent-browser`，包括安装浏览器和关闭会话 |

## 浏览器启动与故障定位

本地 Chrome 模式还需要一个可启动的浏览器。可直接打开任务目标；单独验收安装时，在一个新命名会话中打开 `about:blank` 并读取当前 URL。例如在 POSIX shell 中运行：

```sh
ab_probe="ab-check-$$"
agent-browser --session "$ab_probe" open about:blank
agent-browser --session "$ab_probe" get url
agent-browser --session "$ab_probe" close
```

确认 `open` 成功且 `get url` 返回 `about:blank`。PowerShell 可用 `$ab_probe = "ab-check-$PID"` 创建会话名，后三条命令相同。若选择了 npx，给每条命令加上 `npx` 前缀。

| 观察到的现象 | 下一步 |
| --- | --- |
| CLI 可以运行，启动时报缺少 Chrome | 执行 `agent-browser install` 下载 Chrome for Testing，或按前述调用方式执行 `npx agent-browser install`。如果已有适用的 Chrome，可用 `agent-browser --session "$ab_probe" --executable-path "/实际路径/chrome" open about:blank` 指定可执行文件。再用同一会话检查 URL 并关闭。 |
| Linux 启动时报缺少系统共享库 | 在允许安装系统依赖的机器或镜像中使用 `agent-browser install --with-deps`。这一步会调用系统包管理器，非 root 用户需要可用的 sudo。 |
| 空白页能打开，目标站点失败 | 浏览器已能启动。根据具体错误核对目标 URL、服务是否已启动、代理或登录会话。保留失败的 URL 与错误信息即可定位这一层。 |
| 启动或升级后仍报错 | 若本机支持，运行 `agent-browser doctor --offline`，查看 Chrome、配置、守护进程和启动检查。加 `--quick` 会跳过真实启动；普通诊断可能清理过期的守护进程文件。`--fix` 还可能关闭会话或删除旧状态，应按报告选择需要的修复。 |

安装 CLI 后再下载浏览器时，较老的 macOS 可能无法使用 Chrome for Testing。此时可指定本机兼容的 Chrome。Windows 如需观察操作过程，可用 `--headed` 启动可见窗口。

来源：[官方安装说明](https://agent-browser.dev/installation)。
