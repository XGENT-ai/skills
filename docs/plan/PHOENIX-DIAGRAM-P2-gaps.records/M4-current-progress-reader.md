# M4 · 当前机械进度的只读维护入口

- 计划：[PHOENIX-DIAGRAM-P2-gaps](../PHOENIX-DIAGRAM-P2-gaps.md) §9.4/V-9。
- 更新：2026-10-27T10:54:32+08:00。
- 基线：从dirty@b64ad55实施，入口及五项维护测试已提交`063129a`，未推送；本记录随后的独立文档提交不回填自己的SHA。M4仍进行中3/4，原v2统计与失败不变。

Cl006/007/024的原独审确认修复前未实际取得当前机械进度。新增入口让宿主可以一次取得完整机械对象；这仅补齐读取支撑，不把Root回放当作作者历史读取，也不覆盖这些题的业务事实或最终交接错误。

```sh
node /Users/rockie/Documents/GitHub/xgent/skills/tools/phoenix-diagram/tests/support/read-current-progress.mjs --project /absolute/authorized/project --json
```

入口只从该项目`stdio/inspection-chain/latest.json`读取当前封存头，复用原`readInspection()`认证原始输出、完整分页、共享引用、展开深等、前驱及机械计数，再返回原`progress.json`完整对象和文件绑定。它不执行Native、不写项目文件、不恢复锁或选择旧best。锁、回退的头、未提交槽、未完成派生、身份或页面变化均以exit4、空stdout拒绝；同项目边界在打开头所指的inspection前检查。

exit0只表示读取认证成功。调用方仍须实际读取`data.status/observations/gates/stopRequired`；`not-derivable`保持不可推导，`stopRequired=true`保持立即停止，不把读取成功当图形质量通过。失败diagnostic全文读取及D22成功必要字段规则继续独立成立，本入口不代替它们。固定Native调用、停止2、质量、视觉、来源和身份门均不变。

| 检查 / 范围 | 实际步骤 | 结果与限制 |
| --- | --- | --- |
| V-9 / 读取支撑 | 新CLI缺失时先跑四项回归；再补跨项目打开前拒绝的spy回归 | 两份原red日志保留；跨项目回归实际先失败，再通过。最终采集/读取组23/23通过，含完整值、只读、无额外Native、锁/断链、原件/页篡改、停止2及不可推导 |
| 轻量维护 | `npm test` | 完整最终日志Node525项、499pass/26skip/0fail；Python22/24/2/13项均OK，已无日志写入进程。最终工具返回曾丢失，故不补造进程退出码；既有完整日志及各组结果可核验 |
| V-9 / 真实封存只读回放 | 定稿入口分别读取Cl006/007/018/024原项目 | 四次exit0；最新sequence依次7/6/2/4，Cl018如实not-derivable，其余not-applicable，各自历史gates完整保留。项目全部目录项stat快照前后相同；不是全树内容哈希或历史作者通过证明 |
| 身份隔离 | 前后核原冻结14个FileRefs、旧tgz及当前sourceInputs | 全部保持；旧collector仍26973B/SHA42508146，当前产品SourceInput仍357d503d。未改Native、bundle、dist、npm包、原冻结输入或provider会话，不需重跑未受影响的Native/UI检查 |

日志根为`local/phoenix-diagram/p2-gaps-current-progress-reader/`。首次`sealed-replay.json`绑定边界修订前的入口，保留为原诊断；定稿证明为[replay-2/sealed-replay.json](../../../local/phoenix-diagram/p2-gaps-current-progress-reader/replay-2/sealed-replay.json)，12961B/SHA`4ee252980da01c6b74e7aa3807719c57ebecef72bdedfe1508994593040311f3`，绑定定稿入口4885B/SHA`280e875f819ff9290d6c0537d5957787a6cdf94f2fb4adb9214e176ae46de0d0`。

本次只交付维护入口及回归；原factory、wrapper和64角色清单保持封存。后续若纳入新正式harness须明确额外维护资源身份与实际失效覆盖；不能把新入口拼进原会话。继续调查原本机完整消费的Reader/explore失败，原v2仍48passed/14failed/2incomplete。
