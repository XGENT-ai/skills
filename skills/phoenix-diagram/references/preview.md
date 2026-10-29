# 显式本地预览

只有用户明确要求本地预览才启动前台服务。普通作图不启动 preview；启动预览也不自动授权打开用户浏览器窗口。只有用户希望打开窗口时才加 `--open`。预览不充当正式交付或实际图像审核。

沿主 SKILL 设置 `PD`，使用事实齐备的现有 candidate，不为预览重新猜测或改写输入。首次阅读仍为 Still，Live 由用户主动选择；不采用旧入口的默认 Live。

## 启动与事件

```sh
"$PD" preview <type> candidate.json diagram.html --port 0 --json
# 用户同时明确希望打开窗口时：
"$PD" preview <type> candidate.json diagram.html --port 0 --open --json
```

支持现有十类。可加 `--quality standard|showcase`、`--input-contract legacy-2.17|phoenix-v1`、`--repo-root DIR`、`--brand-cache DIR`；合同适用类型和声明冲突规则沿普通编译。repo-root 须为现有目录，来源是否 verified 仍单独核对。preview 不接受 `--out-dir` 或 `--layout-json`。品牌只消费本地固定资源，服务不隐式联网补品牌。

固定监听127.0.0.1，默认 port=0由系统分配；`--port` 范围0–65535。未加 `--open` 时从 started 读取 URL。stdout 是前台 NDJSON，每行一个事件，不能把多行整体当单个 JSON，也不能在第一条 updated 后把仍运行的进程当已停止。

| event | 含义 |
| --- | --- |
| `started` | 会话/监听已建立；额外给 `url`、`port`，不表示已有成功图。 |
| `checking` | 正在检查当前 generation。 |
| `updated` | 当前结果已正式发布并回读，HTTP artifact与正式文件同字节。 |
| `failed` | 本次尝试失败；可能继续等待修改，不能把该事件直接当进程已退出。 |
| `stopped` | 停止结果；核 `shutdownComplete` 与 `exitCode`，不只看事件名。 |

每条事件有公共头 `schemaVersion/tool/toolVersion/command/status/diagnostics`，command 为 preview；另有 `event/sessionId/generation/lastSuccessfulGeneration/lastAttemptStatus/artifact/artifactAvailable/locale/browserStatus/visualReviewStatus`。event 与 status 不同：status可能为 waiting/checking/ready/error/stopped；lastAttemptStatus可能为 not-run/checking/pass/failed/cancelled。failed与stopped事件另给真实 exitCode；shutdownComplete仅 stopped 给出。

首次 generation/lastSuccessfulGeneration为0、artifact=null、artifactAvailable=false。artifact可用时给 `generation/inputSha256/sha256/bytes`；后两个是正式 HTML 的摘要和字节数。browserStatus与visualReviewStatus保持 not-run，不能由updated推出浏览器或看图通过。

## 修改、last-good与冲突

输入按800ms轮询、400ms稳定窗口处理；内容摘要或不可用/删除状态变化产生新 generation，同字节不重复推进。单worker和最新待处理槽保证旧结果不能倒退覆盖新结果。

半写、非法或删除输入时保留最后成功文件。shell在画布外显示最新失败、诊断和仍显示的成功版本；首个成功之前为waiting/error，即使输出路径原本有文件，也不冒称本会话已有成功图。修复后只有最新且仍绑定原输入/资源的结果才发布。

输出被后来替换时停止发布并报告冲突，不能拿内存旧图冒充正式路径的当前文件；不能覆盖外来文件来继续演示成功。需要输出恢复时按 diagnostics 指明的精确事务和现有 recover-output 合同处理，不自行选记录、删除锁或重建恢复材料。

服务仅提供该随机会话路径下 GET/HEAD 的shell、status、artifact；URL直接取started，不猜路径。Host/Origin受检查，无CORS或任意文件服务，响应no-store/no-referrer。正式HTML/导出不混入会话端口、token、输入绝对路径或错误栈；连接断开不等于视觉通过。

## 停止与正式交付

用Ctrl+C或SIGTERM停止前台会话，只结束自己的监听、连接与worker；等停止事件并确认shutdownComplete=true。最后成功正式文件保留。SIGKILL没有正常收尾回执，不能补写stopped或关闭通过；留下的发布事务仍按原恢复入口检查，不杀未知PID。

停止后按普通 finalize、实际查看当前四张截图、review-record 和同一 summary 收口。已有HTML、HTTP可打开或updated都不能代替四门与 mandatory visual。预览期间的编辑不重置同一失败门的历史最佳和停滞计数；达到2后停止继续修 candidate，按主 SKILL 交接未解决项。服务不授权补造事实、删除语义或改语言原则。
