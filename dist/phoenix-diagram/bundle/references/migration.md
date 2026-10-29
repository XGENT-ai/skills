# 项目级旧入口迁移与回滚

仅在用户明确请求迁移所选项目的旧 Archify 入口时使用。先展示精确移除范围和不备份后果，只有覆盖该范围的明确确认才进入apply；已有明确授权不重复询问。普通install或普通作图不授权迁移。迁移保留原JSON、已有独立HTML以及范围外内容，不接管个人目录、Phoenix UI、共享settings/hooks或其他provider。

旧skill是确认后直接移除，**没有目标备份**。固定历史源码是识别与显式重装依据，不是用户旧目录的备份。回滚只处理新Phoenix Diagram受管写入，不自动恢复已删除的Archify。

## 查看范围，再确认执行

沿主 SKILL 使用当前加载的 `PD`。`PROJECT` 是用户明确指定的实际项目目录，不从个人安装位置推定。skill launcher会为from-archify补入配套包内bundle-root；若直接运行原生引擎，必须显式给配套 `--bundle-root DIR`。

```sh
"$PD" migrate --from-archify --project "$PROJECT" --providers claude,codex --dry-run --json
# 仅在用户已确认上面精确范围及无旧备份后执行：
"$PD" migrate --from-archify --project "$PROJECT" --providers claude,codex --confirm-remove-archify --json
```

`--providers` 可省略：按项目已存在的已知provider目录选择；都没有时默认claude。选择codex会同时检查.agents与.codex的旧入口，新Diagram skill只写入.agents；最终以回执providers和逐项路径为准，不把示例的两个provider当全部支持名单。

dry-run只报告计划与冲突，零写入；遇pending只报告，不恢复。检查 `status`、`project`、`providers`、`migrationSources`、`removals`、`preserved`、`legacyBackupPolicy`、`reinstallInstructions`。只识别完整匹配固定来源、没有外部管理归属且物理快照稳定的项目副本；自定义/多缺文件、链接、未知来源、外部管理或个人安装应保留，不能为了过检强删。

实际apply必须带 `--confirm-remove-archify`；缺少时是 `install/confirmation-required`、exit3，删除与新入口写入都不开始。确认仅限所展示的项目/provider/目录，不授权清理其他位置。先持久化事务意图、确认范围与快照，再逐项受检查移除旧文件并发布新入口，active最后提交。没有跨整个过程的“失败就零变更”承诺；中断时交接真实已删、剩余和保留项。

联合安装的 `--migrate-archify` 属于独立的联合安装器入口，不是给 `PD migrate` 添加的参数；它先展示原生dry-run；TTY肯定答复才apply，拒绝/EOF不开始，非TTY必须显式确认标志。`--no-phoenix-diagram`仍跳过并保留旧入口；迁移分支提前返回，不进入hooks/settings/AGENTS/UI安装。仅给确认标志不构成迁移请求。

## 指定事务恢复或回滚

从本次回执取得真实 `transactionId`、`recoveryCommand`、`rollbackCommand`，优先原样使用绑定本项目的命令，不扫描后自动挑一个事务。三种mode互斥：

```sh
"$PD" migrate --recover "$TRANSACTION_ID" --project "$PROJECT" --json
"$PD" migrate --rollback "$TRANSACTION_ID" --project "$PROJECT" --dry-run --json
"$PD" migrate --rollback "$TRANSACTION_ID" --project "$PROJECT" --json
```

若部分回滚已恢复旧 launcher/ENGINE，旧引擎可能不支持 migrate；使用原回执给出的已验证新原生引擎 recoveryCommand，不换成旧 launcher，也不按 PATH 猜引擎。

recover不接受dry-run；recover/rollback都不接受bundle-root、providers或confirm-remove-archify。它们核已存在的固定正确引擎与本地事务，不下载、不更新、不重装旧skill，不要求当前active安装已完整提交。

普通 install 的 pending 恢复仍按其回执重跑原显式 install；`migrate --recover ID` 用于已确认的迁移或正在回滚的指定事务，不能代替普通安装恢复。优先执行对应回执的 `recoveryCommand`，不要只因有 journal ID 就改用迁移恢复命令。

- Recover继续指定已确认事务的安全剩余步骤；同一确认范围不重复索取确认。后来新增/改写、删除后重建或父目录身份变化保留并报告recovery-required；不凭stdout推断哪些文件已删。
- Rollback仅回滚**当前committed事务**的新入口受管写入；包括仍为v1的当前事务经原生引擎验证并扩展后的回滚，不是跳过后来安装去回滚任意历史ID。后来修改在预检时冲突，先保持零变更；中途身份变化仍保留拒绝。成功rolled-back，同一ID可幂等重试。
- 恢复之前的Diagram active语义须重新绑定恢复后的实际文件身份/checksum；不手拷旧active、journal或备份内容。Archify删除不逆向恢复，按回执reinstallInstructions另获授权后显式重装。

## 回执与后续检查

InstallReceipt公共头为 `schemaVersion/tool/toolVersion/command/status/diagnostics`。migrate的status使用 planned/installed/unchanged/conflict/recovery-required/recovered/rolled-back等安装状态，不按普通图解pass解释；cancelled只用于真实取消，不能把中断或失败写成取消。冲突/recovery-required为exit3，参数2、I/O4；仍读逐项诊断和实际结果。

重点交接：

- `transactionSchemaVersion` 与外层 `schemaVersion` 不同，不能据外层1推断journal仍为v1。
- `transactionId/journal/recoveryCommand/rollbackCommand` 定位实际事务，`engine`与manifest摘要绑定来源。
- `removals`逐项含 `path/source/status`，区分planned/removed/preserved/remaining；同时读 `preserved/conflicts/recovered/writes/unchanged`。
- `legacyBackupPolicy: "none"`、`legacyRestoration: "not-supported"`、`reinstallInstructions`明确旧skill不恢复；`backup`仅涉及Diagram受管写入，不能称为Archify备份。
- `reloadSteps`要求新建provider会话并核实际加载phoenix-diagram路径。外部/个人旧入口未处理时，不声称全机唯一入口生效。

迁移成功不代表既有图像已重新验收；需要新交付时仍保留主 SKILL 强事实、语言、Still、两次停滞即停止及 mandatory visual，不因更换入口删事实或降低验收。
