# R1 apply · 核实与自检材料

- 来源：[R1 报告](../../ELINTP-VISUALS-P2-gaps.ui-review.md)；[冻结目标](../../ELINTP-VISUALS-P2-gaps.ui-target.md)，目标指纹 `06c637df09c2a8979c811753f0537f22004713dfbfaee0e5423c57957cbba160`，目标文件 SHA-256 `ab9400900ff46a4470e381cddd3a173cb7321348dd5e5888b2e21bcae6e3cc11`。本轮未改目标。
- 修前版本 `3e098c0ceeb71bfc77d4e9726267117d8d85888d`；修后为 dirty@`3e098c0c`，产品差异仅 `skills/elintp/scripts/templates.mjs` 的 zh-CN `review.sent`。正式修复 SHA 见 [M1 完成记录](../../ELINTP-VISUALS-P2-gaps.md#完成记录) 的 UI修复项；源码指纹见 [version.json](logs/version.json)。
- 环境：2026-11-12，Playwright 1.63.0 / 系统 Chrome 157.0.8103.78，headless，DPR 1；真实公开 `review.mjs` CLI、本机 loopback HTTP 与真实 `/submit`，无接口替身、账号或业务数据。输入复用 [R1 内容稿](../r1/logs/inputs/plan.md)，包含四张现有图片和一条刻意缺图的长引用；预期缺图警告保留在服务日志。
- 每次均新建浏览器上下文，等待 networkidle、fonts.ready；实拍关闭动画。三个本轮新 PNG 均已打开看图。原 R1 截图、报告与 judge 未覆盖；截图和 logs 按仓内既有规则忽略，仅原工作树可完整复现。

## 操作与实际结果

1. 以当前公开 CLI 启动审阅页，开启“标注内容”。分别选择普通段落、无图注现有图与无图注缺失图，选择疑问 / 没看懂 / 建议并填写一句说明，保存。
2. 刷新，实测恢复“3 条标注”，再开启标注并真实点击“提交给 Agent”。HTTP 200、服务写出完整 result.json 后正常退出 0；三个块为 `s1:b1`、`s1:b4`、`s2:b6`，原话、完整引用与连续双空格均保留。
3. 修前成功反馈为“已提交给 Agent”，对 C4 期望“已提交给当前 Agent”的运行时断言失败，退出 1；其余结果字段及页面宽度断言通过。修前完整结果已经含顶层 `title: 空图注图片的审阅与离线交付`，每项含 `section`、`quote`、`kind`、`note`，因此 UI-01 不采纳。目标 U1-4 未要求每项重复 `pageTitle`；结果契约无改动。
4. 只将 zh-CN `review.sent` 改为“已提交给当前 Agent”，在桌面亮色、窄屏暗色各重走一次以上路径。两次实际 `.review-status` 均为新文案，`aria-live=polite`，提交后工具条按钮禁用；页面 scrollWidth 分别为 1440、360，与视口相等，状态文字完整可见；无 pageerror。自检不等于独立验收。

## 共用截图与结果

同一状态图片共用于 UI-01（顶部页面标题及对应真实提交的完整 JSON）与 UI-02（顶部工具条成功文字），没有按问题复制截图。JSON 中的标题和意见字段以实际结果文件为准，不从截图推断。

| 新图 | 条件 / UTC 时间 | 入口 | 对应完整结果 |
| --- | --- | --- | --- |
| [修前提交](U1-submitted-before-1440-light.png) | 1440×900 / light / 03:10:21.167Z | `http://127.0.0.1:54393/` | [before result](logs/before-1440-light.result.json) |
| [修后提交](U1-submitted-after-1440-light.png) | 1440×900 / light / 03:11:23.264Z | `http://127.0.0.1:55336/` | [desktop result](logs/after-1440-light.result.json) |
| [修后窄屏提交](U1-submitted-after-360-dark.png) | 360×800 / dark / 03:11:24.398Z | `http://127.0.0.1:55356/` | [narrow result](logs/after-360-dark.result.json) |

- 实测原始记录：[before.checks.json](logs/before.checks.json)、[after.checks.json](logs/after.checks.json)；复现命令：`node docs/plan/ELINTP-VISUALS-P2-gaps.ui-captures/r1-apply/logs/check.cjs before`（修前预期失败）、`node docs/plan/ELINTP-VISUALS-P2-gaps.ui-captures/r1-apply/logs/check.cjs after`（修后通过）。脚本使用一次性结果路径，复跑须另选路径，不能覆盖本次证据。
- 清理：三次服务 PID 77369、79448、79471 均在提交后退出 0，浏览器已关闭；无新增 worktree、分支、容器或常驻服务。证据保留供复审，不作为临时垃圾删除。

## 影响范围与复用

- UI-02 只影响简体中文审阅页的成功状态，仍由同一 `.review-status[aria-live=polite]` 输出。两种代表条件已补采；没有缺证或外部阻塞，也没有重构。
- UI-01 的顶层标题、小节、块引文、类型和原话均以本轮三份真实完整结果复核。处置为“不采纳”，交独立复审裁定。
- 原 R1 的 [capture.md](../r1/capture.md)、[实际操作](../r1/observed-interactions.md) 与截图仍是历史证据。对照 R1 `version.json`，其余八个受测产品文件指纹未变；U2 终稿不消费 `review.sent`，阅读/离线/打印、资源与块身份、失败保留路径均未改变，可沿用原材料。本轮只新验上述成功文案与真实提交，不冒充重跑全部 R1 条件或独立评分。
