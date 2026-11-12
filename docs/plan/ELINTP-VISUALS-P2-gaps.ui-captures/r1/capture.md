# R1 · 采集清单

- 目标包：[冻结目标](../../ELINTP-VISUALS-P2-gaps.ui-target.md)；目标指纹 `06c637df09c2a8979c811753f0537f22004713dfbfaee0e5423c57957cbba160`。计算输入：[target-fingerprint.txt](target-fingerprint.txt)，无目标图附件；目标文件 SHA-256 `ab9400900ff46a4470e381cddd3a173cb7321348dd5e5888b2e21bcae6e3cc11`。
- 被测 HEAD `3e098c0ceeb71bfc77d4e9726267117d8d85888d`；tracked dirty SHA-256 `72d2a3b4ce22ef484dd389c57e0fe4cc434eaae892d73fe9b77ffbdb9c14609b`。`skills/elintp` 受测源码无未提交修改，采集前后 hash 一致；详见 [version.json](logs/version.json)。
- 当前页面全部由公开 CLI 从代表性内容稿重新生成。旧 P2 `0d097e94` 仅用于建立真实旧结果和同轮浏览器草稿，后续续接、提交与终稿均运行当前版本。
- Playwright 1.63.0 / 系统 Chrome 157.0.8103.78；DPR 1；桌面 1440×900、窄屏 360×800；等待 networkidle 与 fonts.ready，截图关闭动画。终端均为真实产品入口，无账号、无业务数据、无接口数据拦截。
- 终稿从不含原图的空 cwd 以当前审阅页作 `--assets-from` 生成；浏览器 `offline: true` 后以 file URL 打开。打印由 Chrome 生成 A4 PDF，再以 pdftoppm 26.10.0 渲染实际 PDF 页面。
- 实际操作：[observed-interactions.md](observed-interactions.md)。新图均为 R1 当前版本真实页面或真实打印输出；主 agent 已逐张打开核对入口、视口、主题与画面。没有历史复用图。
- 两张 U2 亮色图是在系统暗色下手动切成亮色后拍摄，出处分别标明；真正系统暗色另拍，不以文件名代替主题核对。

## 新图出处

统一工具为 `Playwright 1.63.0 / Chrome 157.0.8103.78`，打印代表页另标 `Chrome PDF / pdftoppm 26.10.0`。file 入口均为 `docs/plan/ELINTP-VISUALS-P2-gaps.ui-captures/r1/logs/empty-cwd/final.html` 的绝对 file URL。

| 图片 | 单元 · 状态 | 视口 / DPR | 实际主题 | 时间 UTC | 实际入口 / 工具 |
| --- | --- | --- | --- | --- | --- |
| [U1-continued-1440-light.png](U1-continued-1440-light.png) | U1 · 旧结果续接 | 1440×900 / 1 | light | 2026-11-12T02:29:06.361Z | `http://127.0.0.1:61021/`；统一工具 |
| [U1-default-1440-light.png](U1-default-1440-light.png) | U1 · 默认收起 | 1440×900 / 1 | light | 2026-11-12T02:29:08.618Z | `http://127.0.0.1:61027/`；统一工具 |
| [U1-missing-edit-360-dark.png](U1-missing-edit-360-dark.png) | U1 · 长引用编辑、键盘焦点 | 360×800 / 1 | dark | 2026-11-12T02:29:10.726Z | `http://127.0.0.1:61027/`；统一工具 |
| [U1-nojs-1440-light.png](U1-nojs-1440-light.png) | U1 · 无脚本 | 1440×900 / 1 | light | 2026-11-12T02:29:11.420Z | `http://127.0.0.1:61027/`；统一工具，JavaScript 禁用 |
| [U1-annotated-1440-light.png](U1-annotated-1440-light.png) | U1 · 三标注刷新恢复 | 1440×900 / 1 | light | 2026-11-12T02:29:13.440Z | `http://127.0.0.1:61027/`；统一工具 |
| [U1-submitted-1440-light.png](U1-submitted-1440-light.png) | U1 · 主动提交成功 | 1440×900 / 1 | light | 2026-11-12T02:29:13.539Z | `http://127.0.0.1:61027/`；统一工具 |
| [U1-submit-failure-1440-light.png](U1-submit-failure-1440-light.png) | U1 · 服务不可达、标注保留 | 1440×900 / 1 | light | 2026-11-12T02:39:48.540Z | `http://127.0.0.1:50609/`；统一工具；提交前停止本机服务 |
| [U1-print-1.png](U1-print-1.png) | U1 · 打印第 1 页 | A4 / 120dpi | 固定 light | 2026-11-12T02:29:12Z | `http://127.0.0.1:61027/`；Chrome PDF / pdftoppm；原 PDF 在 logs/review-print.pdf |
| [U2-default-1440-light.png](U2-default-1440-light.png) | U2 · 离线默认 | 1440×900 / 1 | light | 2026-11-12T02:31:36.236Z | final.html file URL；统一工具，offline |
| [U2-outline-1440-light.png](U2-outline-1440-light.png) | U2 · 手动亮色、大纲末项 | 1440×900 / 1 | 系统 dark → 手动 light | 2026-11-12T02:31:38.446Z | final.html#s3 file URL；统一工具，offline |
| [U2-missing-360-light.png](U2-missing-360-light.png) | U2 · 手动亮色、缺失引用 | 360×800 / 1 | 系统 dark → 手动 light | 2026-11-12T02:31:40.865Z | final.html file URL；统一工具，offline |
| [U2-outline-1440-dark.png](U2-outline-1440-dark.png) | U2 · 系统暗色、大纲末项 | 1440×900 / 1 | dark | 2026-11-12T02:34:15.689Z | final.html#s3 file URL；统一工具，offline |
| [U2-missing-360-dark.png](U2-missing-360-dark.png) | U2 · 系统暗色、缺失引用 | 360×800 / 1 | dark | 2026-11-12T02:34:16.393Z | final.html file URL；统一工具，offline |
| [U2-print-1.png](U2-print-1.png) | U2 · 打印第 1 页 | A4 / 120dpi | 固定 light | 2026-11-12T02:31:42Z | final.html file URL，offline；Chrome PDF / pdftoppm；原 PDF 在 logs/final-print.pdf |

## 验收条件覆盖

“实测”指本轮真实浏览器操作；CLI 产物与页面度量仅由主 agent 用于机器检查，不作为视觉评审结论。

| 目标条件 | 单元 | 本轮实拍 | 本轮实测 / 出处 | 覆盖 |
| --- | --- | --- | --- | --- |
| C1 固定模板、亮暗与资源 | U1、U2 | default、annotated、missing-edit、outline light/dark、missing light/dark | 操作 4–7、13–20；CLI 产物由主 agent 核对 | 已覆盖 |
| C2 窄屏、长引用与宽图 | U1、U2 | missing-edit、missing light/dark | 操作 4–7、13–16、19–20；页面度量由主 agent 核对 | 已覆盖 |
| C3 键盘、焦点、文字与 alert | U1、U2 | missing-edit、submit-failure、outline light/dark | 操作 4–7、13–16、19–21 | 已覆盖 |
| C4 默认、已有标注、成功与失败 | U1 | default、annotated、submitted、submit-failure | 操作 4–7、10–11、21 | 已覆盖 |
| C5 无脚本、打印与离线 | U1、U2 | U1-nojs、两张 print-1、U2 全组 | 操作 8–9、13–20；完整四页 PDF 由主 agent 逐页看图 | 已覆盖 |
| U1-1 两图标注、类型、编辑删除 | U1 | annotated、missing-edit、submit-failure | 操作 4–7、10–11、21 | 已覆盖 |
| U1-2 完整原引用 | U1 | missing-edit | 操作 4–7、11；长引用与双空格逐字记录 | 已覆盖 |
| U1-3 旧结果与同轮草稿续接 | U1 | continued、annotated | 操作 1–3、10–11 | 已覆盖 |
| U1-4 刷新、主动提交与失败保留 | U1 | annotated、submitted、submit-failure | 操作 10–11、21 | 已覆盖 |
| U2-1 终稿正文、图片与无审阅界面 | U2 | U2 全组 | 操作 13–20；DOM 产物由主 agent 核对 | 已覆盖 |
| U2-2 空 cwd 资源恢复与精确内容稿 | U2 | default、missing light/dark、print-1 | 操作 13–20；CLI 产物由主 agent 核对 | 已覆盖 |
| U2-3 新终稿来源拒绝与旧来源 | U2 | 无需独立视觉帧 | CLI 实跑由主 agent 核对，不要求视觉评审推断 | 已覆盖 |
| U2-4 主题、大纲与体积 | U2 | default、outline light/dark | 操作 13–16、19；CLI 体积由主 agent 核对 | 已覆盖 |

## 待评单元与材料边界

- U1、U2 都交本轮独立评审；没有继承评分或上一轮差距。
- 评审只需打开目标包、本清单、上述 PNG 与 [observed-interactions.md](observed-interactions.md)。同一状态可共用图片，不需机械打开每张打印页。
- 日志、CLI 检查、页面度量、临时脚本、旧版源码、计划与代码均不属于评审材料。
- 没有必需条件缺证；未对触屏、真人速度、真实 Agent 后续改写或跨期图型验收作推断。
