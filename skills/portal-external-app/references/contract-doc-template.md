# 对接契约文档模板（外部服务 → 平台）

> 泛化自已交付的知识库契约（`docs/knowledge-app-contract.md`，**门户仓文件，App 自己的 repo 里没有**；本模板已自包含，不必去找）。每个外部服务接入前都要按此结构交一份契约文档，且**与已实现代码 1:1 对齐**（理想状态：有回归测试保证路由表与代码同步）。没有这份文档的接入不算完成。

按节填写，方括号处替换：

```markdown
# `<listingKey>` App — Platform Contract (resource server → portal)

## 1. Listing 元数据
| 字段 | 值 |
| --- | --- |
| listingKey / aud | `<key>`（覆盖变量名如 `<KEY>_AUDIENCE`） |
| version | `x.y.z` |
| type | micro / service |
| scopes | `<key>.read`, `<key>.write`, …（+ `audit.write` 如写审计） |
| dependencies | 如 `["files"]`（说明是装机拓扑占位还是运行时真调用） |
| exchangeTargets | 本服务作为交换发起方要读的目标（无则空） |
| embedUrl | micro 填 `/apps/<key>/`；service 留空 |

（micro 附 navItems JSON。）

## 2. 运行时契约（已实现）
- 自省端点、Basic 凭证、信封解包（claims = body.data ?? body）、缓存策略；
- 四道闸各自的失败码（401 INVALID_TOKEN / 403 INSUFFICIENT_SCOPE / FORBIDDEN / INSUFFICIENT_PERMISSION）；
- 服务态 TDT 的处理（scope-only；哪些用户拥有的写操作拒绝服务态）；
- 租户隔离与限流参数。

## 3. 路由 → scope → PID 表（核心，逐条列）
| Route | method | scope | PID | 额外门 |
| --- | --- | --- | --- | --- |
| `/v1/...` | POST | `<key>.read` | `<key>:page:xxx` | |
| `/v1/...` | POST | `<key>.write` | `<key>:action:xxx` | require admin / require user ctx |
| `/health` 等 | GET | — | — | public |
（新增路由的缺省门策略要写明——fail-safe 默认落最严的写门，绝不无门。）

## 4. ACL Manifest JSON（如声明 ACL；纯 scope 鉴权则写明 aclManifest: null）
（完整 JSON：pages/actions/roleTemplates；DataScope 支持哪些档、不支持的档如何退化。）

## 5. Env 契约（镜像不含 .env）
| var | required | default/example | 说明 |
| --- | --- | --- | --- |
（**列镜像实际读取的变量名**；有门户契约别名的写清映射；端口变量覆盖顺序、是否读裸 PORT；
门户三变量 all-or-nothing + fail-fast 行为。必填项**同步声明进 manifest 的 `requiredEnv`**
——键集合一变发布提案会转入人工审核，平台在批准前就看到；不声明的静默改名无门可拦，
症状是换版后 refusing to start 而发布侧只看到成功。）

## 6. 平台侧 checklist（对方的工作量）
（listing/ACL 登记、服务账号创建并回传凭证、/svc 白名单、交换 grant、
（micro）dist 托管与 CORS/CSP、（有配置面）<key>-token + config Section。标出阻塞项。）

## 7. 运维命令
- DB 迁移：**镜像内的迁移 argv**（填进 `deployDescriptor.migrateArgs`）+ 幂等/建议锁/退出码保证；
- 每租户 bootstrap：需要则给出精确 SQL/命令（tenants.id 必须 = 门户租户 UUID），不需要则明说；
- 判活/就绪：GET /health 与 /healthz 的返回形状与 200/503 语义；
- 镜像交付方式（docker save + sha256 / 私有 registry）与架构（amd64+arm64）。

## 6b. 你用了平台的哪些公共能力（逐项声明「用」还是「自建」）
- 文件存储 / 站内通知 / 统一审计 / 定时任务 / 用量上报 / 结构化内容 / 选人（通讯录）；
- 任何一项填「自建」都要写明理由 —— 默认答案是用平台的（基座能力走 `/api/v1/*`，
  文件/解析/大模型这类 App 型服务走 `/svc/<key>` + 令牌交换）。

## 7b. 配额（若有「每租户上限」诉求）
- 模型：`seatBased`（门户成员席位，门户在 TDT 签发时收口）或 `seatRoles: [...]`（你自管账号，你自己收口）——二选一；
- 若 seatRoles：列出角色 key，并说明你在哪条建号/建资源路径上调 `POST /api/v1/seats/quota` 收口，
  以及 `available=null`（unlimited）时放行的处理；
- 角色 key 由**你在 `app.manifest.json` 里声明**（`"seatRoles": ["service"]`，需 backed listing）；
  各套餐的数值不进 manifest、由平台管理员在控制台配 —— 在这里**提议**一组（free/pro/ultra），并列出你会上报的 gauge 指标名；
- **不要**在自己库里存租户上限；**不要**在 manifest 里申请 `seats.read`（SERVICE_ONLY，会被拒收，由平台在 SA_DEFS 给）。
- 机制全貌（三态处理、兜底值、存量租户、已用上报）见 [quota-and-seats.md](quota-and-seats.md)。

## 8. 自测（无 UI 时的 curl 验收）
（缺/错 token → 401/403；正确 aud+scope → 200 信封。）

## 9. 信任边界（如有）
（自有认证面清单：API-Key/节点 Token/admin 会话/gRPC 端口——哪些绝不从 /svc 暴露；
MCP/stdio 之类免鉴权面的使用限制；破玻璃通道的启用条件。）
```

审阅要点（平台方收到契约时逐项验）：

- §3 路由表是否覆盖**全部** `/v1` 业务路由，公开路由是否只有健康/指标；
- §5 是否写清镜像实读变量名（对照 compose/K8s 注入排一遍）；
- §6b 是否有「自建」项而没有理由 —— 自己存文件 / 自己发通知 / 自己记审计是最常见的三处；
- §7b 若有配额诉求：模型是否二选一说清、role key 是否已在 manifest 的 `seatRoles` 里声明、是否误在 manifest 里申请 seats.read、`available=null` 是否按放行处理；
- §7 两问是否有显式答案：迁移 argv（**必须在镜像里**，不接受「脚本在我们 repo」）、每租户 bootstrap（目前门户无钩子，要显式定方案）；
- §9 自有认证面是否与 `/svc` 隔离。
