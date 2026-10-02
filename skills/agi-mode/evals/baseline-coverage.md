# 基线覆盖与差异

供维护者审查本模式相对参考版本的能力保留与改进假设，普通任务不加载。参考材料是本仓开发环境中的 `local/poteto-mode/SKILL.md`、`local/poteto-mode/playbooks/` 和各 `principle-*/SKILL.md`。这些材料不是安装后的运行依赖。

下表按本次实际阅读的 23 个 playbook 文件建立对应。源文件名及其步骤可在上述目录核对；覆盖某个入口不等于证明行为等价，也不意味着参考版本缺少其他优点。

这里映射的是工作流能力。参考版本的编排存储 CLI、PR watcher 和 worktree 审计脚本没有在本版本中复刻；本版本通过当前环境可用工具执行相应操作。需要专用后台监听或大规模调度吞吐的任务，仍需验证目标环境是否具备相应能力，不能由这张表推导工具层完全等价。

## 原能力的承接位置

| 参考 playbook 文件 | 必须保留的能力 | 本版本位置与处理 |
| --- | --- | --- |
| `investigation.md` | 只读解释与有依据的判断 | [调查](../playbooks/investigation.md)，保留只读边界 |
| `bug-fix.md` | 原入口复现、因果定位、同条件复验 | [缺陷修复](../playbooks/bug-fix.md)，保留失败到通过证据 |
| `feature.md` | 数据形状、依赖切分、实际入口验证 | [功能交付](../playbooks/feature.md)，不强制每个功能都委派 |
| `refactoring.md` | 固定行为契约、迁移调用方、等价验证 | [重构与迁移](../playbooks/migration-refactoring.md)，对真实外部消费者保留必要兼容窗口 |
| `prototype.md` | 廉价隔离原型、真实观察、据此决策 | [设计与原型](../playbooks/design-prototype.md)，区分原型结论与生产完成 |
| `perf-issue.md` | 基线、瓶颈机制、修改后测量 | [性能优化](../playbooks/optimization.md)，保留一次性改进模式 |
| `hillclimb.md` | 固定测量、逐轮记录、保留或撤回、突破平台期 | [Hillclimb](../playbooks/hillclimb.md)，独立管理搜索与最佳候选，增加最终确认与协议变更处理 |
| `runtime-forensics.md` | 活进程捕获、缩小机制、映射源码 | [运行时与离线取证](../playbooks/forensics.md)，将采样与改变现场分开 |
| `trace-forensics.md` | 固定捕获的解析、聚合、符号和配对比较 | [运行时与离线取证](../playbooks/forensics.md)，保留独立离线入口 |
| `visual-parity.md` | 修改前基线、逐项图像差分、不篡改门槛 | [界面交付](../playbooks/ui-delivery.md) 的严格视觉等价分支，零差异要求保持零差异 |
| `authoring-a-skill.md` | 编写指令、结构与引用检查 | [Skill 编写与修订](../playbooks/authoring-a-skill.md)，行为对照交给独立 Eval |
| `eval.md` | 自然请求、候选与评判盲化、统一尺度、核对真实轨迹 | [Eval](../playbooks/eval.md)，独立覆盖多类候选，增加可比运行、无效运行与保留样本处理 |
| `babysit.md` | 单次检查、评论处理、持续推进到可合并 | [PR 生命周期](../playbooks/pr-lifecycle.md)，按请求选择模式 |
| `opening-a-pr.md` | 差异整理、提交说明、base 与 head 核对 | [PR 生命周期](../playbooks/pr-lifecycle.md)，草稿与就绪按用户要求 |
| `shipping.md` | 连续已验证区间、版本时效、逐个合并与读回 | [合并与发布](../playbooks/shipping.md) + [项目编排](../playbooks/program.md)，保留栈前沿核对 |
| `autonomous-run.md` | 可检查退出条件、持续循环、决策记录 | [自主任务总控](../playbooks/autonomous-run.md)，达标后结束，不把停滞当成功 |
| `orchestrate.md` | 样本试跑、有限在途、任务账目、失联与迟到结果 | [项目与队列编排](../playbooks/program.md)，规模小时缩减记录与协调层级 |
| `autopilot-full.md` | 独立单元全生命周期、验证后按授权合并 | [项目与队列编排](../playbooks/program.md) 的独立队列模式 + [发布](../playbooks/shipping.md) |
| `autopilot-stack.md` | 构建并验证线性 PR 栈，留给用户落地 | [项目与队列编排](../playbooks/program.md) 的线性堆叠模式，单一拓扑修改者 |
| `multi-phase-plan.md` | 可执行单元、依赖、真实验证与交付边界 | [设计与原型](../playbooks/design-prototype.md) 的实施计划分支，按任务风险确定验证类别 |
| `session-pickup.md` | 继承已有进展，从正确位置继续 | [会话交接](../playbooks/session-handoff.md)，按证据适用范围重验 |
| `pause-safely.md` | 停止工作、保存产物、明确恢复入口 | [会话交接](../playbooks/session-handoff.md)，不强制为暂停创建提交 |
| `worktree-cleanup.md` | 工具枚举、使用状态核对、清理与空间读回 | [资源清理](../playbooks/resource-cleanup.md)，不将未跟踪或忽略文件自动判为可删 |

## 原则的承接与扩展

下列 23 个源原则已逐篇阅读。源名称对应 `local/poteto-mode/<名称>/SKILL.md`，表中说明保留的决策及合并、调整的位置。三个复杂度原则合并为一个，领域建模与类型纪律合并为一个，其余各有承接；共形成 20 个基础原则。

| 参考原则目录 | 本版本位置 | 保留或调整的决策 |
| --- | --- | --- |
| `principle-laziness-protocol` | [最少复杂度](../principles/minimize-complexity.md) | 最小充分变化，去掉不创造价值的间接层 |
| `principle-subtract-before-you-add` | [最少复杂度](../principles/minimize-complexity.md) | 先检查范围内可移除部分，再增加结构 |
| `principle-minimize-reader-load` | [最少复杂度](../principles/minimize-complexity.md) | 同时考虑追踪层数与隐藏状态，不只看行数 |
| `principle-foundational-thinking` | [基础先行](../principles/foundations-first.md) | 数据、访问路径与真实依赖先于后续实现 |
| `principle-redesign-from-first-principles` | [从约束重设计](../principles/redesign-from-constraints.md) | 将新约束纳入整体结构，再分阶段抵达 |
| `principle-experience-first` | [使用者结果](../principles/experience-first.md) | 最终用户、调用方与维护者共同决定质量 |
| `principle-exhaust-the-design-space` | [方案比较](../principles/explore-alternatives.md) | 对真实设计分叉比较机制不同的方案，数量随必要信息决定 |
| `principle-outcome-oriented-execution` | [结果责任](../principles/outcome-ownership.md) | 围绕最终状态组织过渡，区分隔离工作区与在用系统 |
| `principle-model-the-domain` | [领域结构与类型](../principles/domain-models-and-types.md) | 以领域知识与状态归属组织结构，减少散落规则 |
| `principle-type-system-discipline` | [领域结构与类型](../principles/domain-models-and-types.md) | 不合法状态、语义类型、穷尽检查与权威 schema |
| `principle-boundary-discipline` | [保证失效的边界](../principles/boundary-discipline.md) | 保证仍有效时避免重复校验；失效后重新建立保证 |
| `principle-make-operations-idempotent` | [可核对效果](../principles/idempotent-effects.md) | 重跑与部分完成可恢复，超时先对账 |
| `principle-migrate-callers-then-delete-legacy-apis` | [迁移闭合](../principles/complete-migrations.md) | 保留原原则的内部可控前提，明确外部兼容窗口 |
| `principle-separate-before-serializing-shared-state` | [状态归属](../principles/state-ownership.md) | 先分离独立写入，必要共享使用实际并发机制 |
| `principle-prove-it-works` | [真实结果](../principles/prove-real-outcomes.md) | 真实产物与入口证据，不靠代理信号或自报 |
| `principle-fix-root-causes` | [修复根因](../principles/root-cause.md) | 复现、验证因果、修复机制；区分止损与根治 |
| `principle-sequence-verifiable-units` | [可验证单元](../principles/verifiable-units.md) | 按真实依赖与检查边界推进，保持集成验证 |
| `principle-test-behavior-not-implementation` | [行为契约](../principles/behavioral-tests.md) | 检验消费者结果；允许本身构成契约的禁止副作用与关系检查 |
| `principle-attack-the-premise` | [检验共同前提](../principles/challenge-premises.md) | 重复失败审查共同假设，不预设一定是参与者分配问题 |
| `principle-build-the-lever` | [构建杠杆](../principles/build-leverage.md) | 工具服务吞吐与复核，先样本后扩展，以实际收益决定是否构建 |
| `principle-guard-the-context-window` | [可恢复上下文](../principles/context-as-state.md) | 隔离大材料、保留出处、渐进加载与恢复入口 |
| `principle-never-block-on-the-human` | [自主决策](../principles/autonomous-decisions.md) | 主动查证和实施，关键偏好与确实缺失的权限才提问 |
| `principle-encode-lessons-in-structure` | [结构化学习](../principles/structural-learning.md) | 重复教训变成执行机制，并检验误报与适用范围 |

另外将五个控制问题明确为独立原则。这表示本版本对它们设置了可触发的决策入口，不表示参考版本完全没有相关思想：

- [信息价值](../principles/information-first.md)：以能否改变决策选择观察，限制无效调查。
- [验证器反证](../principles/falsifiable-verification.md)：用已知错误检验尺子，工具变更使旧判定失效。
- [证据时效](../principles/evidence-lifetime.md)：按输入与环境传播失效，减少误复用与无差别重验。
- [关键路径](../principles/critical-path-throughput.md)：优化已验证交付速度，限制在途积压。
- [明确收敛](../principles/bounded-convergence.md)：区分必要工作与额外探索，以真实状态结束循环。

## 超越基线需要验证的假设

这些是可以反驳的改进假设，不能由文档结构直接判为成立。

| 设计改变 | 希望改善的结果 | 失败信号与验证方法 |
| --- | --- | --- |
| 完整主流程加明确的阶段切换 | 减少授权范围内的中途收工和重复规划 | 在“诊断→修复→PR”等复合任务中，观察遗漏阶段与用户催促次数 |
| Eval 与 Hillclimb 独立，判定协议与搜索状态分离 | 保留通用评测与持续进化能力，减少调优对判定的污染 | 比较纯评测、持续搜索、协议变更、最终确认失败等场景的实际行为 |
| 前提、观测、能力三类阻塞分别处置 | 降低无信息重试，更快取得可用证据 | 在相同顽固缺陷中比较重复动作、错误改动和首个有效观察的成本 |
| 输入与产物指纹绑定验收 | 更早发现旧日志、丢失证据和变更后沿用绿灯 | 对记录注入输入变化、日志变化、缺项、契约变化，检查是否拒绝复用 |
| 真实依赖和外部消费者决定迁移方式 | 避免为追求简洁提前破坏兼容 | 滚动升级任务中实际运行新旧消费者，检查结果和退出条件 |
| 模式按实际能力执行，不依赖固定模型与平台插件 | 降低缺工具时的停摆与虚构执行 | 单 agent、缺浏览器、无后台机制的条件下，检查交付与未验证声明 |
| 只加载当前 playbook，规模决定协调成本 | 简单任务更短，复杂任务保持完整性 | 同一任务集比较完成率、上下文消耗、调用次数和总成本 |

## 验证层级

1. **结构与覆盖**：链接、格式、路由和源入口对应可直接检查。
2. **确定性机制**：运行 `python3 -B -m unittest discover -s evals -p 'test_*.py'`，测试证据检查器面对真实临时文件变化的行为。这不测试模型是否会正确使用它。
3. **工作流推演**：用 [场景](scenarios.md) 检查流程是否提供必要动作和边界。推演只能暴露设计矛盾。
4. **真实行为对照**：在相同任务、模型、工具和预算下比较参考模式与本模式，保留工具轨迹及产物，进行重复试验。此层未执行前不能称本模式整体更强。

做真实对照时至少覆盖普通开发、顽固缺陷、滚动迁移、严格视觉还原、PR 队列和上下文恢复。判定目标完成、真实回归、授权边界、证据诚实、用户干预与成本；不以篇幅、规则条数或 playbook 数量评分。
