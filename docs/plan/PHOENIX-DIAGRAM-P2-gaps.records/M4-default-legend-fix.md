# M4 · v2之后的默认图例修复诊断

当前为**最小修复及诊断已通过，源码已提交b64ad55**。按计划§9.5处理已定位的产品缺口；本记录不改变v2的48passed/14failed/2incomplete，也不构造新正式cohort。当前正式完成条件和M4未完成状态保持。

## 修改与身份边界

根因已由原失败图、schema和默认值函数交叉核实：success是良好结果且可有出边，legacy默认却写成终态；workflow backend是通用处理类型，五语言默认却声明Agent执行者。已将legacy两处默认改为成功/success，将workflow五语言默认改为中性处理逻辑，同步五个维护目录和共用生成目录/模板。9个产品文件、12个源字符串，显式legend/translations优先级和输入schema不变。

修前源码输入身份实际为`a45958a28c32f0a1968294baf9c3e6bb94e2bb59b38fcf57f35b0565056c11cf`；9个运行文件修后为`5666fefc8ed832b62ce8867eab7da40c33b4916e138e9771ddc6b1480bf69ce6`。随后同步viewer/README维护说明，最终为`357d503df78344a684f96c89c671f1816b756c6a79cb907e1bf87fc7e92eb619`；仍171文件，对原基线10项差异，仅多一份README。装配器注释也同步说明维护目录来自快照，不改变算法或生成物。三份清单位于`local/phoenix-diagram/p2-gaps-default-legend-fix/source-inputs-{before,after,final}.json`。

独立development Native已实际构建：14968464B，SHA `5a7ef3c4df8e203df874806b3ca437b83fde6947ac3ae76741466e6111e93e2d`；sourceInput为最终357d503d，bundle仍为`d4a1082892fb609b50c5dad95eecddcfd39f9b68fac71d7f292552dfe87d25e9`。build-receipt记录实际8d7f92f/dirty，locked offline、sourceStable、probe、sourceInputs均pass；没有冒称新提交或正式包。尚未更新dist、ENGINE或npm包，不重跑作者。

旧v2全部原件及其旧Source21包继续保留；当前工作树修复是新的产品源码。原旧身份的CI、宿主、Native和本机证据不能冒充新身份结果。模板内嵌目录变化也影响其他图型的新HTML身份，不能因视觉可能相同就拼接旧记录；新正式冻结及实际重新绑定覆盖仍须另行明确。

## §9.5 实际失效字段与新正式覆盖

以下仅列出已发生的身份变化及其后果，不启动新正式冻结或全量。原v2清单绑定SourceInput a45958a2、bundle d4a10828和dev tgz b89dc910；本机consumer另绑定同来源CI候选tgz 2f30de2d，两份载体的区别继续保留。

| 字段 / 范围 | 当前事实 | 正式收口需要的覆盖 |
| --- | --- | --- |
| Native sourceInput / 二进制 | 已由a45958a2变为357d503d；当前只有独立darwin-arm64新Native 5a7ef3c4 | 旧四平台二进制、Node18消费和原CI ZIP仍证明旧身份；不能填新身份。新正式候选须由同一冻结源码实际构建所需平台并重新绑定 |
| bundle / npm载体与安装 | bundle仍d4a10828；新Native尚未进入dist或tgz | 不因bundle相同而沿用旧tgz/安装/launcher/交付身份；新包须有真实字节、安装及引擎核验，不能手写或替换旧ENGINE摘要 |
| HTML / delivery / 四图与Reader | 模板及全语言目录内嵌于HTML；两份新诊断graph/cards与原值相等，HTML身份已独立 | 新同包V-4六区及12失败组、V-5十五例及Reader/导出/恢复/拒网/explore/corpus/evidence/locale、V-7同包消费须按新身份实际取得产物、检查、四图与登记。旧单例通过和原失败均不可拼成新完整consumer通过 |
| 宿主与反例cohort | 原62正式+2额外ERD仍48/14/2，新源码/包不能拼回原冻结清单 | 若另立新正式冻结，64角色的包/引擎、请求、会话、交付与归档绑定均需同一清单实际闭合；旧业务正向判断可作参考，不能代替新正式行。尚无新cohort或授权后的新执行结果 |
| 维护支撑 / collector | 原collector42508146未变；只读入口063129a未接入原wrapper；维护点击坐标竞态已修，见[诊断记录](M4-consumer-diagnostics.md) | 未来接线需绑定新入口及core依赖、当前controller/page probe，重做相关harness/fresh审计和V-9支撑核验；不改原14refs或冒称旧作者使用了它 |
| 已过维护检查 | b64ad55运行修复的test:diagram/test:phoenix已过；只读入口的npm及局部已过，相关运行源码未再变化 | 保留各自实际覆盖，不为候选元数据变化重复无关UI或全部维护测试；新包、真实浏览器、平台和宿主检查仍各自独立 |

未被上述具体修复覆盖的最终路径、业务事实、提问形式、禁读实现等原失败继续列于[M4逐角色结果](M4-v2-bounded-round.md)，不把新一轮执行当作已经落实的修法。是否进入新的正式范围须在上述身份边界内另行明确；不能自动换模型、循环改规则或重打产品包抽签。

## 已执行验证

- 修前Node Reader回归实际exit1，返回Agent logic而非Processing logic；修前Native回归实际exit101，分别看到Agent logic和terminal success。首轮新增测试的合同配置错误另留原日志，修正测试设置后才取上述两项产品失败，不把测试配置错误当产品红灯。
- 修后Reader五语言默认值/覆写与现有目录共9项全部通过；原装配器check通过。
- `npm test`已实际exit0：Node520项、494pass/26skip/0fail，Python22/24/2/13项全部OK。
- 原workflow/lifecycle两个oracle与HEAD原件逐字节相同，见`unchanged-oracles.json`。参考几何测试改用公开显式标签固定原展示，保留完整几何断言；新默认值另由真实parse/compile/render及Reader回归验证。
- `npm run test:diagram`已实际exit0，完整Rust366项pass；Node两组239pass/5skip及138pass/0fail，最终maintenance状态pass，明确realBrowser/providerSessions不由此命令证明。该次为运行修复源码5666fefc；维护说明同步后，仅README造成源码输入身份变化，9个运行文件逐字节相同。最终357d503d又实际跑i18n/render共11项、Reader9项、原装配器check，全部通过，见`metadata-only-finalization.json`。
- `npm run test:phoenix`首次因未设置现有wasm-opt路径在preflight失败，原日志保留。已实际找到本机既有darwin-arm64 binaryen117并核版本，只为该进程设置WASM_OPT后重启；原进程已实际exit0：Rust1272pass/0fail、Node8pass、8474向量零差异、860/860 oracle通过。原失败仍保留；未安装工具或改全局设置。
- 原冻结14个FileRefs仍实际可核验；旧collector及所有原作者材料未修改。

日志根：`local/phoenix-diagram/p2-gaps-default-legend-fix/`。新增测试设置曾命中显式/持久合同冲突，以及legacy只接受en/zh-CN且不接受translations；均只修新增测试，未扩大旧合同。旧参考测试加入显式原标签后，输入保持断言改为核实际提交clone，完整几何仍比原oracle；所有原失败日志保留。

## 两份原输入的实际四图诊断

在新目录逐字节复制Cl017原1721B candidate及Code049原4274B input，原件FileRefs/字节/物理绑定前后保持。以新Native仅各执行一次finalize，两份四门均pass；HTML graph及cards与旧交付逐值相等。当前SourceInput/二进制、配置、浏览器回执与四图身份独立绑定，不冒充旧同包产物。

Root已用view_image original逐张查看八张新PNG。人工流程的五节点/三泳道/审批两路及控件完整，图例为处理逻辑5；本例无说明卡片，索引完整。生命周期十一状态/三带/三卡完整，Live→Health Paused→Rolled Back可循，图例success2不再断言全部终态。暗亮两尺寸均无关系遮字、截断或水平溢出。

实际观察写入各新root-actual-review.json并调用review-record，回读同一ID的当前summary：两份automated pass、visual passed、delivery complete。交付ID分别`ba98f3a7-38b9-464f-9210-c33cb87d996a`和`822e4c09-e106-45b4-9187-6c56f58e89af`；候选/HTML/配置/引擎/browser身份在登记前后相等。见[诊断闭合证明](../../../local/phoenix-diagram/p2-gaps-default-legend-fix/visual/reviewed-outcome.json)4263B/SHA `c18bea82dffe3046e2b6a1edae3ee1841d94266a72bb4146a63de99ac01fd09c`。

运行前实测load1=10.27、CPU idle=51.93%、可用空间69256724480B，并核当前UI测试无Chrome子进程，故利用独立空闲窗口串行生成；不把资源差异当作旧Reader超时根因。Root直接调用新Native做修复诊断，没有伪造宿主捕获/安装记录、作者复验或正式cohort通过。

## 后续

9个运行文件、4处测试和2处维护说明共15文件已提交`b64ad55`，仅提交本次修复，原用户dirty保留。完整[验证索引](../../../local/phoenix-diagram/p2-gaps-default-legend-fix/verification.json)6756B/SHA`9660cb439642b7cebebd263b6a1a8d50209ecf200fcfdc2b58b86bbb289c330f`绑定日志、构建、最终身份及实际八图。新结果只记修复诊断，不回判原失败通过；同包分发与新正式cohort的实际失效覆盖仍按§9.5另行明确，不自动重包、更换模型或启动新64题。本机完整消费的原失败、模型最终事实/路径/机械progress/提问形式问题及两项技术中断不由此修复概括解决。
