# M4 · 本机Reader的单例取证与点击坐标修复

- 计划：[PHOENIX-DIAGRAM-P2-gaps](../PHOENIX-DIAGRAM-P2-gaps.md) §9.5/V-5/V-7。
- 更新：2026-10-27T11:39:31+08:00。
- 基线：从dirty@8b8eb2a实施，三处维护修改已提交`298638d`，未推送；本记录的后续文档提交不回填自己的SHA。产品SourceInput仍357d503d，原v2、原完整consumer和旧cohort身份保持；其余进度及私有证据仍仅在本工作树。
- 状态：点击坐标竞态已有实际先红后绿，27项局部、真实同延迟单例及同源完整轻量复核全部通过；首轮npx锁失败保留。M4仍3/4，原完整消费仍failed，不把诊断拼成正式通过。

## 实际发现与最小修改

原完整消费的legacy architecture在“下游可达”失败，但相同HTML的旧轮次和本次无延迟单例均通过。源码中的镜头事务默认420ms，探索仅等250ms；原点击脚本在`scrollIntoView()`后立即量坐标。只给下游mousePressed增加一次100ms传输延迟后，按钮上移94px，原点从按钮落入`relationship-lens-list`，复现同一“downstream reach did not activate”失败。

因此只修改维护点击取点：等待当前`data-camera-transaction`结束，再让滚动触发的面板布局绘制两帧，最后量坐标。原可用/可见检查、真实CDP按下/释放、输入序列、Chrome参数和全部15秒门保持；没有脚本调用DOM click、修改Reader产品、Native或质量门。取点函数移入现有page probes以直接验证真实序列化函数。

这是已复现的维护竞态修复；原失败没有CDP逐步轨迹，不能倒填其历史点击位置或宣称两项原超时同因。

## 独立诊断与结果

| 范围 | 实际控制 | 结果 / 证据 |
| --- | --- | --- |
| 原workflow Reader | 原controller、原HTML及15秒门 | 既有单例20/20、Reader通过；未复现原Input.dispatchMouseEvent超时，见[probe-2结果](../../../local/phoenix-diagram/p2-gaps-distribution-v2-diagnostic-1/probe-2/root-outcome.json)，保持原完整consumer失败 |
| legacy architecture基线 | 原封存harness、原HTML c4427f32，增加只读命中/状态观察 | [diagnostic-2](../../../local/phoenix-diagram/p2-gaps-distribution-v2-diagnostic-2/run-2/terminal.json)实际exit0，11项探索和两卡通过；前置系统工具多硬链接误校验及低CPU门失败均保留，启动浏览器前已修正私有核验器，未改变旧证据规则 |
| 坐标竞态复现 | 同一原HTML和旧harness，仅首次下游按下延迟100ms | [diagnostic-3](../../../local/phoenix-diagram/p2-gaps-distribution-v2-diagnostic-3/run-3/terminal.json)实际exit1；延迟前Y449.984命中按钮，100ms后命中关系列表，按钮上移94px；原输入保持。两次前置资源拒绝没有启动浏览器，不计浏览器重试 |
| 点击回归 | 直接序列化page probe，模拟镜头完成后继续发生的滚动面板布置 | 原450旧坐标断言实际red；等待后原测试及控制器全部27/27通过，日志`diagnostic-3/click-point-{red,green}.log`。VM只证明维护取点，不代替浏览器 |
| 修复后的同条件复验 | 当前维护源码、同一原HTML、同一次100ms延迟 | [diagnostic-5](../../../local/phoenix-diagram/p2-gaps-distribution-v2-diagnostic-5/run-1/terminal.json)实际exit0，11项和两卡全过；延迟前后Y352.984均命中下游按钮，CDP零error，原输入保持。新维护文件另绑定，不冒称使用旧controller |
| 原offline lifecycle Reader | 原harness、原HTML、原Node/Chrome/隔离环境与15秒门 | [diagnostic-4](../../../local/phoenix-diagram/p2-gaps-distribution-v2-diagnostic-4/probe-1/terminal.json)实际exit0；20/20导出、Reader、打印和动效全部pass，CDP零error，最长evaluate约2288ms。未复现原Runtime.evaluate超时，不判原完整消费已恢复 |

各浏览器串行，均在两帧CPU空闲≥40%、load<100、空间≥3GiB的原门通过后才启动；等待只重复资源观察，浏览器每项只运行一次。私有环境路径重绑到各自新目录，原环境的其他值、真实Node/Chrome、原harness输入均认证。Root实际看过基线和失败截图；截图只帮助定位，不冒充正式四图验收。

原consumer-state仍为1340568B/SHA`ff566f21d16b889e8f6b6577b6f76e6c274e9fa3a4e92225df1d9d4e1523805a`。原冻结14个FileRefs和旧tgz再次验证保持；Native sourceInputs仍357d503d。本次没有运行Native、模型、完整consumer、聚合或正式候选重打包/发布；npm入口用例仅在自己的临时目录生成测试包。新增维护代码不进入Native sourceInputs/bundle，已通过且未变化的Rust/UI运行检查保留。

轻量首轮实际exit1：525项中498pass/1fail/26skip，唯一npx显式包help调用报`ECOMPROMISED / Lock compromised`，Python组尚未运行。原日志`diagnostic-5/npm-test.log`保留；未修改npm或用例，原`test/npm-entrypoint.test.js`隔离实际exit0，约10秒、1/1通过。该隔离未复现锁失败，不推断其CPU或缓存根因。

随后同源、同命令`npm test`完整复核实际exit0：Node525项，499pass/26skip/0fail；Python22/24/2/13项均OK，见`diagnostic-5/npm-test-recheck.log`。没有放宽测试、延长命令超时或修改npm缓存锁实现。

## 后续与正式边界

三处维护改动已提交298638d。新的正式同包覆盖见[失效字段表](M4-default-legend-fix.md#95-实际失效字段与新正式覆盖)；原两次超时仍未建立可复现根因，原完整消费和v2的48passed/14failed/2incomplete保持。不得把本次修复概括为业务事实、交接路径、禁读实现或提问形式问题均已解决，也不自动进入新64题或轮换模型。

如用户另行明确进入新正式范围，动作限定为：冻结357d503d产品源码、现有d4a10828 bundle和当前维护支撑；同一新候选实际完成四平台/精确Node18、本机全范围及62正式+2反例；原题、原D12模型路线、D22/D23判据和停止2保持，每角色一次主执行，失败按§9.5限次处理。新入口只补通用维护调用，不夹带业务答案。保留全部旧失败，只有真实符合聚合条件才取五大ZIP与归档；不再自动另开下一轮、换模型或接受错误事实。当前没有新正式cohort、重包或模型执行。
