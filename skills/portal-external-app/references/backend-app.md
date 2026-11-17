# 门户仓内独立后端 App

仅在任务明确涉及 xgent-ai-portal 仓内 `apps/<key>-server` + `apps/<key>-app` 时使用。外部项目没有这些路径；出仓 App 即使原先在 monorepo，也使用自己的 manifest 和镜像，不恢复 `LISTING_DEFS` / `MANIFEST_REGISTRY` 副本。

## 实现路径

1. 先读当前仓库规则，访问 `apps/api` 前读其 `CLAUDE.md`。参考仍在仓内的 `apps/spms-server` 的 gate/identity 与组织方式，不依赖已出仓目录。
2. 新增内置 App 才创建 server/app 双包、独立 DB 和空闲 dev 端口；普通 API 改动不重复执行整条注册/部署链。
3. 鉴权统一读 [集成契约 §3](integration-contract.md#3-运行时鉴权四道闸)，服务账号签票与撤销读 [服务令牌](service-tokens.md)。沿用现有鉴权层，新 Bun/TypeScript 后端使用 `@xgent/portal-server-sdk`；不再复制另一套自省信封与缓存实现。
4. 注册、scope、ACL、非破坏性 bootstrap、部署与用量接线读 [仓内接线](backend-wiring.md)。权限修改须检查已安装租户同步与角色 reconcile，fresh install 不能代替升级验证。
5. 有前端时读 [微应用开发](micro-app.md)；配置先读 [配置分层](config-placement.md)。业务接口统一信封、服务端分页，迁移按仓库现有 SQL 方式。
6. 本地一律 `bun run dev:all`，不与单服务 dev 脚本叠跑。新 workspace 同步根锁文件、Dockerfile COPY、构建及部署清单。

## 验证与部署边界

- 新增后端的验证入口放 `apps/<key>-server/scripts/verify-*.ts` 并接入仓库验证链，覆盖真实 TDT、租户隔离、ACL 和核心业务结果。局部改动运行最小相关检查；完整新 App 交付与里程碑按仓库要求做回归，有 UI 时从宿主做浏览器验收。
- 仓内 `DEPLOYABLE_APP_SERVICES` 的 built-in 后端不能成为套件成员：共享单元的同名 DNS 别名会冲突，准入报 `BUNDLE_BUILTIN_MEMBER`。需要套件时先按授权范围出仓为 descriptor App，再建套件；发布口径见 `xgent-app-release`。
