# R2 · 实际操作记录

来源：本轮当前源码经公开 render.mjs 生成的 file 页面；Playwright 1.59.1 / Chrome 157.0.8103.39；DPR 1。以下仅记录执行动作及观察，不包括运行检查结论。

## 四模板 · 1440×900 / 360×800 × 系统 light/dark

| 单元 | 视口 | 系统 | 动作 | 观察 |
| --- | --- | --- | --- | --- |
| U1 | 1440 | light | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切换亮暗", "toggled": "dark", "persisted": "dark"} |
| U1 | 1440 | light | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 55.96875} |
| U1 | 1440 | light | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s3", "last": "s3"} |
| U1 | 1440 | light | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "表头释义", "expanded": "true", "notesVisible": true} |
| U1 | 1440 | dark | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切换亮暗", "toggled": "light", "persisted": "light"} |
| U1 | 1440 | dark | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 55.96875} |
| U1 | 1440 | dark | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s3", "last": "s3"} |
| U1 | 1440 | dark | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "表头释义", "expanded": "true", "notesVisible": true} |
| U1 | 360 | light | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切换亮暗", "toggled": "dark", "persisted": "dark"} |
| U1 | 360 | light | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 56} |
| U1 | 360 | light | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s3", "last": "s3"} |
| U1 | 360 | light | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 40, "focus": "表头释义", "expanded": "true", "notesVisible": true} |
| U1 | 360 | dark | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切换亮暗", "toggled": "light", "persisted": "light"} |
| U1 | 360 | dark | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 56} |
| U1 | 360 | dark | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s3", "last": "s3"} |
| U1 | 360 | dark | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 40, "focus": "表头释义", "expanded": "true", "notesVisible": true} |
| U2 | 1440 | light | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切换亮暗", "toggled": "dark", "persisted": "dark"} |
| U2 | 1440 | light | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 55.5625} |
| U2 | 1440 | light | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s6", "last": "s6"} |
| U2 | 1440 | light | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "表头释义", "expanded": "true", "notesVisible": true} |
| U2 | 1440 | dark | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切换亮暗", "toggled": "light", "persisted": "light"} |
| U2 | 1440 | dark | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 55.5625} |
| U2 | 1440 | dark | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s6", "last": "s6"} |
| U2 | 1440 | dark | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "表头释义", "expanded": "true", "notesVisible": true} |
| U2 | 360 | light | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切换亮暗", "toggled": "dark", "persisted": "dark"} |
| U2 | 360 | light | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 56.15625} |
| U2 | 360 | light | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s6", "last": "s6"} |
| U2 | 360 | light | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 40, "focus": "表头释义", "expanded": "true", "notesVisible": true} |
| U2 | 360 | dark | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切换亮暗", "toggled": "light", "persisted": "light"} |
| U2 | 360 | dark | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 56.15625} |
| U2 | 360 | dark | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s6", "last": "s6"} |
| U2 | 360 | dark | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 40, "focus": "表头释义", "expanded": "true", "notesVisible": true} |
| U3 | 1440 | light | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切換亮暗", "toggled": "dark", "persisted": "dark"} |
| U3 | 1440 | light | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 109.5625} |
| U3 | 1440 | light | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s2", "last": "s4"} |
| U3 | 1440 | light | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "表頭釋義", "expanded": "true", "notesVisible": true} |
| U3 | 1440 | dark | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切換亮暗", "toggled": "light", "persisted": "light"} |
| U3 | 1440 | dark | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 109.5625} |
| U3 | 1440 | dark | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s2", "last": "s4"} |
| U3 | 1440 | dark | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "表頭釋義", "expanded": "true", "notesVisible": true} |
| U3 | 360 | light | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切換亮暗", "toggled": "dark", "persisted": "dark"} |
| U3 | 360 | light | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 55.984375} |
| U3 | 360 | light | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s2", "last": "s4"} |
| U3 | 360 | light | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 40, "focus": "表頭釋義", "expanded": "true", "notesVisible": true} |
| U3 | 360 | dark | Tab skip; Tab theme; Enter; reload | {"skip": "跳到正文", "focused": "切換亮暗", "toggled": "light", "persisted": "light"} |
| U3 | 360 | dark | keyboard Enter second outline item | {"hash": "#s2", "current": "#s2", "targetTop": 55.984375} |
| U3 | 360 | dark | 调用最后小节 scrollIntoViewIfNeeded（已可见时不会移动；不能据此断言实际滚动到底） | {"current": "#s2", "last": "s4"} |
| U3 | 360 | dark | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 40, "focus": "表頭釋義", "expanded": "true", "notesVisible": true} |
| U4 | 1440 | light | Tab skip; Tab theme; Enter; reload | {"skip": "Skip to content", "focused": "Switch theme", "toggled": "dark", "persisted": "dark"} |
| U4 | 1440 | light | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "Column notes", "expanded": "true", "notesVisible": true} |
| U4 | 1440 | dark | Tab skip; Tab theme; Enter; reload | {"skip": "Skip to content", "focused": "Switch theme", "toggled": "light", "persisted": "light"} |
| U4 | 1440 | dark | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "Column notes", "expanded": "true", "notesVisible": true} |
| U4 | 360 | light | Tab skip; Tab theme; Enter; reload | {"skip": "Skip to content", "focused": "Switch theme", "toggled": "dark", "persisted": "dark"} |
| U4 | 360 | light | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "Column notes", "expanded": "true", "notesVisible": true} |
| U4 | 360 | dark | Tab skip; Tab theme; Enter; reload | {"skip": "Skip to content", "focused": "Switch theme", "toggled": "light", "persisted": "light"} |
| U4 | 360 | dark | table focus; ArrowRight; Tab; Enter notes | {"before": 0, "after": 0, "focus": "Column notes", "expanded": "true", "notesVisible": true} |

## 补充操作

### 操作 59 · U1

- 条件：1440；见截图出处
- 动作：reader click zoom; click independent theme; drag 45/25px
- 观察：{"initial": "阅读100%", "zoomed": "阅读125%", "theme": "dark", "before": {"scrollLeft": 0, "svg": "transform: translate(-93px, -43px) scale(1.25); clip-path: inset(33.206px 71.818px 33.206px 71.819px);"}, "after": {"scrollLeft": 0, "svg": "transform: translate(-46.4685px, -17.2919px) scale(1.24588); clip-path: inset(13.879px 109.534px 54.011px 37.298px);"}}

### 操作 60 · U1

- 条件：1440；见截图出处
- 动作：scroll reader document to bottom
- 观察：{"scrollY": 107, "innerHeight": 736, "scrollHeight": 843, "text": "撤回 · 未定 · 无效\n冻结候选\n基线 / 最佳 / 已确认\n连续停滞先诊断\n任务 · 指导 · 执行 · 评测\n在未见任务上独立确认\n用于下一轮即不再未见\n修评测 / 查指导 / 换结构分叉\n连续尝试停滞\n达到预设条件\n进入依赖它的下一轮\n图例\n方法步骤\n1\n版本与候选\n1\n边界与确认\n2\n冻结与恢复\n1\n证据台账\n1\n实验执行\n1\n停滞诊断\n1\n\n路径\n地图\n透镜\n100%\n达到什么才算一轮有效\n• 先用初始对象测出基线，再比较候选\n• 收益要超过测量噪声，并有实际价值\n• 同时检查质量底线与相邻场景有没有回归\n• 记录这轮花了多少实验、验证与运行成本\n重放的三个证据层级\n• 历史效果复核：重读原始产物，只说明当时发生了什么\n• 代理验证：给候选当时的输入，只支持局部机制假设\n• 实际效果验证：让候选参与新的可比执行，才谈真实效果\n• 历史材料不能直接给新候选打效果分\n两类记录缺一不可\n• 工作样本：真实任务的输入、条件、产物与效果\n• 进化尝试：假设、父版本、候选差异、验证与去留理由\n• 只有工作样本，得到的是案例库\n• 两类关联起来，才能评价经验有没有被用好\n容易走偏的地方\n• 规则数量、修改次数、自我评价都不能当能力证明\n• 不能同时改对象和评分口径，再把分数变化算成改进\n• 确认集一旦用于选择，就不再是未见数据\n• 确认失败就恢复仍有效的已确认版本，不要反复试到出赢家"}

### 操作 61 · U1

- 条件：360；见截图出处
- 动作：reader click zoom; click independent theme; drag 45/25px
- 观察：{"initial": "阅读100%", "zoomed": "阅读125%", "theme": "dark", "before": {"scrollLeft": 0, "svg": "transform: translate(-90px, -41.625px) scale(1.25); clip-path: inset(29.245px 63.231px 29.245px 63.233px);"}, "after": {"scrollLeft": 0, "svg": "transform: translate(-90px, -41.625px) scale(1.25); clip-path: inset(33.3px 72px);"}}

### 操作 62 · U1

- 条件：360；见截图出处
- 动作：scroll reader document to bottom
- 观察：{"scrollY": 762, "innerHeight": 654, "scrollHeight": 1416, "text": "撤回 · 未定 · 无效\n冻结候选\n基线 / 最佳 / 已确认\n连续停滞先诊断\n任务 · 指导 · 执行 · 评测\n在未见任务上独立确认\n用于下一轮即不再未见\n修评测 / 查指导 / 换结构分叉\n连续尝试停滞\n达到预设条件\n进入依赖它的下一轮\n图例\n方法步骤\n1\n版本与候选\n1\n边界与确认\n2\n冻结与恢复\n1\n证据台账\n1\n实验执行\n1\n停滞诊断\n1\n\n路径\n地图\n透镜\n100%\n达到什么才算一轮有效\n• 先用初始对象测出基线，再比较候选\n• 收益要超过测量噪声，并有实际价值\n• 同时检查质量底线与相邻场景有没有回归\n• 记录这轮花了多少实验、验证与运行成本\n重放的三个证据层级\n• 历史效果复核：重读原始产物，只说明当时发生了什么\n• 代理验证：给候选当时的输入，只支持局部机制假设\n• 实际效果验证：让候选参与新的可比执行，才谈真实效果\n• 历史材料不能直接给新候选打效果分\n两类记录缺一不可\n• 工作样本：真实任务的输入、条件、产物与效果\n• 进化尝试：假设、父版本、候选差异、验证与去留理由\n• 只有工作样本，得到的是案例库\n• 两类关联起来，才能评价经验有没有被用好\n容易走偏的地方\n• 规则数量、修改次数、自我评价都不能当能力证明\n• 不能同时改对象和评分口径，再把分数变化算成改进\n• 确认集一旦用于选择，就不再是未见数据\n• 确认失败就恢复仍有效的已确认版本，不要反复试到出赢家"}

### 操作 63 · U3

- 条件：1440；见截图出处
- 动作：activate second outline link
- 观察：{"hash": "#s2", "current": "#s2", "top": 109.5625}

### 操作 64 · U3

- 条件：360；见截图出处
- 动作：activate second outline link
- 观察：{"hash": "#s2", "current": "#s2", "top": 55.984375}

### 操作 65 · U1

- 条件：代表视口；light
- 动作：image focus; ArrowRight; horizontal wheel 1000px
- 观察：{"before": {"scroll": 0, "width": 320, "content": 1440, "imageWidth": 1440, "naturalWidth": 1440}, "after": 40, "last": 1040, "pageScrollWidth": 360}

### 操作 66 · U1

- 条件：代表视口；dark
- 动作：image focus; ArrowRight; horizontal wheel 1000px
- 观察：{"before": {"scroll": 0, "width": 320, "content": 1440, "imageWidth": 1440, "naturalWidth": 1440}, "after": 39, "last": 1040, "pageScrollWidth": 360}

### 操作 67 · U1

- 条件：代表视口；见截图出处
- 动作：open with scripts disabled; scroll to reader
- 观察：{"hostThemeVisible": false, "readerThemeVisible": true, "text": "深色\n风格\n演示\n导出\nagi-mode 有边界的进化循环\n01 / 单轮进化（有边界）\n02 / 停滞诊断\n03 / 独立确认\n固定边界\n单轮机制实验\n冻结与确认\n固定目标与边界\n目标 · 指标 · 底线 · 预算\n提出机制假设\n会推翻它的观察\n出自明确父版本的候选\n不堆叠无法归因的猜想\n固定协议下比较\n先判有效性，再比收益\n记录去留与理由\n保留 · 撤回 · 未定 · 无效\n冻结候选\n基线 / 最佳 / 已确认\n连续停滞先诊断\n任务 · 指导 · 执行 · 评测\n在未见任务上独立确认\n用于下一轮即不再未见\n修评测 / 查指导 / 换结构分叉\n连续尝试停滞\n达到预设条件\n进入依赖它的下一轮\n图例\n方法步骤\n版本与候选\n边界与确认\n冻结与恢复\n证据台账\n实验执行\n停滞诊断\n\n路径\n地图\n透镜\n100%\n达到什么才算一轮有效\n• 先用初始对象测出基线，再比较候选\n• 收益要超过测量噪声，并有实际价值\n• 同时检查质量底线与相邻场景有没有回归\n• 记录这轮花了多少实验、验证与运行成本\n重放的三个证据层级\n• 历史效果复核：重读原始产物，只说明当时发生了什么\n• 代理验证：给候选当时的输入，只支持局部机制假设\n• 实际效果验证：让候选参与新的可比执行，才谈真实效果\n• 历史材料不能直接给新候选打效果分\n两类记录缺一不可\n• 工作样本：真实任务的输入、条件、产物与效果\n• 进化尝试：假设、父版本、候选差异、验证与去留理由\n• 只有工作样本，得到的是案例库\n• 两类关联起来，才能评价经验有没有被用好\n容易走偏的地方\n• 规则数量、修改次数、自我评价都不能当能力证明\n• 不能同时改对象和评分口径，再把分数变化算成改进\n• 确认集一旦用于选择，就不再是未见数据\n• 确认失败就恢复仍有效的已确认版本，不要反复试到出赢家"}

### 操作 68 · U1

- 条件：代表视口；见截图出处
- 动作：scripts disabled; focus image; ArrowRight
- 观察：{"left": 40, "width": 320, "scroll": 1440}

### 操作 69 · U1

- 条件：代表视口；见截图出处
- 动作：deny storage; reload; Tab Tab Enter theme
- 观察：{"theme": "light", "bg": "rgb(250, 250, 247)"}

### 操作 70 · U3

- 条件：1440；见截图出处
- 动作：second outline click; real wheel up 400; real wheel down 900
- 观察：{"before": {"scrollY": 1449, "hash": "#s2", "current": "#s2", "sectionTops": [-1113.765625, 109.5625, 346.34375, 583.125]}, "up": {"scrollY": 1049, "hash": "#s2", "current": "#s1", "sectionTops": [-713.765625, 509.5625, 746.34375, 983.125]}, "bottom": {"scrollY": 1449, "hash": "#s2", "current": "#s4", "sectionTops": [-1113.765625, 109.5625, 346.34375, 583.125]}}

### 操作 71 · U3

- 条件：360；见截图出处
- 动作：second outline click; real wheel up 400; real wheel down 900
- 观察：{"before": {"scrollY": 1977, "hash": "#s2", "current": "#s2", "sectionTops": [-1325.53125, 55.984375, 285.953125, 487.125]}, "up": {"scrollY": 1577, "hash": "#s2", "current": "#s1", "sectionTops": [-925.53125, 455.984375, 685.953125, 887.125]}, "bottom": {"scrollY": 1998, "hash": "#s2", "current": "#s4", "sectionTops": [-1346.53125, 34.984375, 264.953125, 466.125]}}

### 操作 72 · U1

- 条件：360；见截图出处
- 动作：reader theme focus; repeated real Tab; Enter on zoom
- 观察：{"trail": [{"tag": "BUTTON", "aria": "当前视觉风格：经典。选择视觉风格", "text": "\n        \n        经典\n        \n      "}, {"tag": "BUTTON", "aria": "进入演示模式", "text": "\n      \n      演示\n    "}, {"tag": "BUTTON", "aria": "导出图表", "text": "\n        导出 \n      "}, {"tag": "svg", "aria": null, "text": "\n        agi-mode 有边界的进化循环\n        由 Archify "}, {"tag": "g", "aria": "直接关系浏览器", "text": "使用方向键浏览关系。按 Enter 或空格键固定详情；按 Escape 清除。出自明确父版"}, {"tag": "g", "aria": "检查第 1/9 条关系：出自明确父版本的候选 到 固定协议下比较，连接到。按 Enter 查看详情。", "text": "出自明确父版本的候选 → 固定协议下比较 · 连接到"}, {"tag": "g", "aria": "聚焦固定目标与边界，目标 · 指标 · 底线 · 预算, 单轮进化（有边界） › 固定边界", "text": "\n          固定目标与边界 · 目标 · 指标 · 底线 · 预算 · 单轮进化"}, {"tag": "g", "aria": "聚焦提出机制假设，会推翻它的观察, 单轮进化（有边界） › 固定边界", "text": "\n          提出机制假设 · 会推翻它的观察 · 单轮进化（有边界） › 固定边"}, {"tag": "g", "aria": "聚焦出自明确父版本的候选，不堆叠无法归因的猜想, 单轮进化（有边界） › 单轮机制实验", "text": "\n          出自明确父版本的候选 · 不堆叠无法归因的猜想 · 单轮进化（有边界"}, {"tag": "g", "aria": "聚焦固定协议下比较，先判有效性，再比收益, 单轮进化（有边界） › 单轮机制实验", "text": "\n          固定协议下比较 · 先判有效性，再比收益 · 单轮进化（有边界） ›"}, {"tag": "g", "aria": "聚焦记录去留与理由，保留 · 撤回 · 未定 · 无效, 单轮进化（有边界） › 单轮机制实验", "text": "\n          记录去留与理由 · 保留 · 撤回 · 未定 · 无效 · 单轮进化"}, {"tag": "g", "aria": "聚焦冻结候选，基线 / 最佳 / 已确认, 单轮进化（有边界） › 冻结与确认", "text": "\n          冻结候选 · 基线 / 最佳 / 已确认 · 单轮进化（有边界） ›"}, {"tag": "g", "aria": "聚焦连续停滞先诊断，任务 · 指导 · 执行 · 评测, 停滞诊断 › 单轮机制实验", "text": "\n          连续停滞先诊断 · 任务 · 指导 · 执行 · 评测 · 停滞诊断"}, {"tag": "g", "aria": "聚焦在未见任务上独立确认，用于下一轮即不再未见, 独立确认 › 冻结与确认", "text": "\n          在未见任务上独立确认 · 用于下一轮即不再未见 · 独立确认 › 冻"}, {"tag": "g", "aria": "语义图例", "text": "\n          图例\n          \n            \n       "}, {"tag": "g", "aria": "检查方法步骤，1 个节点", "text": "\n            \n            方法步骤\n          1"}, {"tag": "BUTTON", "aria": "追踪有向路径", "text": "路径"}, {"tag": "BUTTON", "aria": "打开语义雷达", "text": "地图"}, {"tag": "BUTTON", "aria": "打开语义透镜", "text": "透镜"}, {"tag": "BUTTON", "aria": "查找节点", "text": ""}, {"tag": "BUTTON", "aria": "打开图表指南", "text": ""}, {"tag": "BUTTON", "aria": "再次放大以显示标签和注释。重置图表视图", "text": "阅读100%"}, {"tag": "BUTTON", "aria": "放大", "text": ""}], "zoom": "阅读125%"}

### 操作 73 · U1

- 条件：360；见截图出处
- 动作：reader horizontal wheel 320
- 观察：{"before": 2, "after": 322, "hostWidth": 360}

### 操作 74 · U2

- 条件：360；见截图出处
- 动作：record image focus; ArrowRight
- 观察：{"left": 40, "width": 320, "scroll": 720}

## 补充说明

- 三组 U1 打印均为浏览器直接生成：系统 dark 未手选、系统 light 未手选、系统 dark 手选 dark；各四页，由 pdftoppm 直接栅格化，未编辑图像。
- 初轮阅读器拖动在桌面产生位置变化，360 下同次拖动未改变位置；后续新上下文实际横向滚轮从 2 到 322。
- 无脚本：宿主主题按钮不可见，阅读器的主题按钮仍可见；正文、图及图例存在。
- 未拦截请求或修改 UI；只有存储拒绝状态模拟 localStorage 不可用。
