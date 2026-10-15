# 终端用户账号生命周期、条款与隐私接口

> 先读 [能力与归属](end-users.md)；登录与本人证明见 [认证协议](end-user-auth.md)。
> 下列 `/api/v1/*` 是门户基座服务面，不是 `/svc/user-center`，也不走成员的跨 App 令牌交换。
> 所有示例为占位；普通查询 DTO 脱敏，专门导出才可含明文标识和联系方式。

## 1. 服务身份与最小权限

App 后端用获得 client_credentials、租户 allowlist、正确 owner 绑定的服务账号换服务票：

```http
POST <portal-origin>/api/tokens/service
Authorization: Basic <服务账号clientId:secret的base64>
Content-Type: application/json

{"grant_type":"client_credentials","tenant_id":"<租户UUID>","scope":"endusers.read"}
```

从成功 Envelope 的 data 取 `access_token`，再 Bearer 调管理接口。Basic 只用于换票/自省，
管理接口不直接收 Basic，也不收成员或终端用户 access。tenant 来自 `tenant_id`，owner 来自 `azp`；
一般接口 body 的 tenantId/ownerAppKey 不能扩大范围，siteKey 必须落在该 owner 的 realm 内。
共享 user-center 通行证也必须使用平台明确绑定该 owner 的服务身份，不能自报 owner。

| scope | 能力 |
| --- | --- |
| `endusers.read` | 身份/业务引用/完整标识查询、会话读取与解析、主体资格、realm 登记、处置日志、导出与清理状态 |
| `endusers.manage` | 建号、设/重置凭据、迁移、同步命令、删除；也覆盖会话撤销及封禁解封 |
| `endusers.sessions` | 只撤销会话、封禁/解封；不含建号、改密、删除、迁移或命令 |
| `endusers.clients` | 客户端登记及回调治理；manage 不能代替它 |
| `privacy.requests.read/manage/receipt` | 分别读参与请求进度、拉本 App 待办、交本 App 回执 |
| `notification.external.contacts/send` | 分别管理本 App 外部联系依据、受理外部通知事件；不由成员 notification.send 替代 |

这些是 SERVICE_ONLY scope，经 manifest `privilegedServiceScopes:[{scope,reason}]` 申请审批，
不放入普通 serviceScopes 或成员 scopes；持 manage 不自动拥有 read/clients。
只取实际需要的权限，不为集中下线功能授予能改密码的 manage。

## 2. 身份管理 API

以下路径均以 `/api/v1/end-users` 为前缀，R=read，M=manage，S=manage 或 sessions：

| 方法与路径 | 权限 | 关键输入/用途 |
| --- | --- | --- |
| `POST` 空路径 | M | `siteKey,identifierKind,identifier`；可选 externalRef、credential、locale、mustChangePassword；新建或按业务引用幂等回读 |
| `GET /<id>` | R | 读取脱敏身份；不存在 data:null |
| `POST /lookup` | R | `{siteKey,externalRefs:[...]}`，最多 500 项，一批回读避免逐人请求 |
| `POST /lookup-by-identifier` | R | `{siteKey,items:[{kind,identifier}]}`；完整标识精确查找，一次最多 10 条，按标识数限流 60 条/分钟/应用/租户 |
| `POST /<id>/credential` | M | `credential`（8–200 字符）、可选 mustChangePassword；推进凭据版本、撤销全部会话 |
| `POST /<id>/archive`、`/unarchive` | S | 归档/解封；不提供“顺便解除所有阻断” |
| `POST /<id>/registration-ticket` | M | 可选 contactId，签发绑定该人邮箱联系方式的受限验证票；不能登录或拿文件 |
| `GET /<id>/sessions` | R | 会话列表；包含呈现方式、设备、脱敏 IP、UA、时间与撤销状态 |
| `POST /<id>/sessions/revoke-all` | S | 全部会话失效并递增凭据版本；包括托管浏览器登录 |
| `POST /<id>/sessions/<sessionId>/revoke` | S | 只撤一条，不递增主体凭据版本；其他设备不受影响，已撤/不存在回 revoked:0 |
| `POST /sessions/resolve` | R | 所属 App 后端将站点 cookie 会话解析为 owner/files 定向票，须同时匹配调用方及会话空间 |
| `POST /subjects/check` | R | `id,siteKey`；返回存在/当前可用/空间的受限投影，不返回联系方式；资源服务器例外由平台限定 |
| `GET /realm?siteKey=...` | R | 自己的 realm/profile/client 登记自检，未登记不是 404 |
| `GET /<id>/export`、`/<id>/privacy-status` | R | 平台数据导出/异步清理状态 |
| `DELETE /<id>` | M | 服务治理删除，返回 deleted、cleanup、quota；不是用户本人重新认证 |
| `POST /clients` | clients | 客户端及回调登记，字段与示例见认证协议 |

`identifierKind` 可为 username/email/phone。邮箱 trim 后转小写；手机号只接受完整 E.164，
不猜国家码；用户名按平台规则归一。精确查找不支持前缀/后缀/模糊，不返回墓碑，
输入任一项非法整批拒绝；审计只记数量，不记明文或可反查摘要。
归档保留容量，解封不恢复会话；直接建号/改密/归档等对 command-only owner 会被拒。

`GET /realm` 的 `realmAllows:false` 要区分没声明、已撤销、越界；允许但 profile:null 表示尚未登记。
同一注册信息可在用户中心自检卡查看。返回配置不包含运行保留参数或 Apple 私钥。

## 3. 跨库同步：五类普通命令与本人删除

业务事实在 App 库、身份事实在平台库时，不能用本地事务包 HTTP 假装原子提交。
本地持久化操作号、拟变更、业务修订与阶段，再执行：

```text
POST /api/v1/end-users/commands/<operationKey>/prepare
  {siteKey,externalRef,action,sequence,payload}
→ App 本地事务：重查前置条件，提交业务事实，记录 finalize_pending
POST /api/v1/end-users/commands/<operationKey>/finalize
  {siteKey}
本地未提交且失败时：POST 同号 /abort {siteKey,reason?}
丢回包先：GET /api/v1/end-users/commands/<operationKey>?siteKey=...
```

| 普通 action | payload |
| --- | --- |
| create_account | identifierKind、identifier、credential；可选 mustChangePassword、locale、status（active/archived） |
| set_credential | credential，可选 mustChangePassword；恢复凭据时 expectedRecoveryRunId/expectedCredentialVersion 须成对 |
| set_identifier | identifierKind、identifier |
| set_status | status：active 或 archived |
| revoke_sessions | 空对象 |

operationKey 使用 1–128 位字母、数字、`._:-`；externalRef 为稳定业务引用。
sequence 用于阻止迟到操作；同号同摘要回原命令，不同内容冲突，不能换号绕过。
prepare 后现有身份即阻断、撤会话、推进凭据版本；新建仅预留候选 ID 和容量，不产生可登录账号。
abort 不恢复已经撤销的会话；finalized 不能 abort，逆向变更须新补偿命令。
`GET /api/v1/end-users/commands` 列未终结项，接入方应持久续跑，不靠用户投诉发现卡单。

单条命令成功 Envelope 的 data：

```text
{operationKey,action,phase,sequence,externalRef,siteKey,
 endUserId,candidateEndUserId,preparedAt,settledAt,resultSummary,portalCleanup?}
phase = prepared | finalized | aborted
```

新建 prepare 的候选 ID 不等于已生效身份，endUserId/candidateEndUserId 可为 null；
终结时间 settledAt 可为 null，不回显敏感 payload。GET 不存在回 data:null。
resultSummary 是动作相关摘要，不用它替代 phase；删除的 portalCleanup 为 pending/done/failed。
常见失败分别处理 CONFLICT、END_USER_SYNC_PENDING、END_USER_REAUTH_REQUIRED；
未知业务错误保留失败状态，不按 HTTP 200 判定已经提交。

批量（单次 ≤100，逐项结果、逐主体事务）：

```text
POST /commands/batch/prepare   {siteKey,items:[{operationKey,externalRef,action,sequence,payload}]}
POST /commands/batch/finalize  {siteKey,items:[{operationKey}]}
POST /commands/batch/abort     {siteKey,items:[{operationKey,reason?}]}
→ data.items:[{operationKey,ok,command?,error?}]
```

以上仍以 `/api/v1/end-users` 为前缀。一项失败不代表整批失败，也不代表其余全部成功；
逐项落阶段再续跑。同一批重复标识可能在 prepare 就冲突，不要改用并发逐人请求规避。

第六类 `delete_account` **不是上面普通 payload 模式**。本人注销托管通行证：

1. 由当前用户按认证协议取得 `purpose=delete_passport` 的近期证明，操作号持久保存。
2. 服务后端持 manage 调同号 prepare，body **只允许**
   `siteKey,action:"delete_account",endUserId,reauthToken,sequence?`，
   不传 externalRef 或 payload。平台单次消费证明、阻断身份并撤销全部会话。
3. App 持久提交自己的业务清理后，同号 finalize；未提交不可逆清理可 abort。
   丢回包先回读，同号同摘要重试可返回原结果，不能复用证明创建另一操作。
4. 回读 command 的 `portalCleanup` / 清理分项，区分 pending/done/failed；
   它只说明平台清理，不证明所有业务资产已清除。confirm_action 证明不能用于此命令。

清理分项通过 `GET /api/v1/end-users/<id>/privacy-status` 取得：
data 含 `endUserId,status,privacyRevision,cleanup,jobs,operations?`。
operations 的每项含 operationKey、commandPhase、identity（active/blocked/deleted）、
sessions（revoked）、notificationCleanup、appleRevocation、portalCleanup、updatedAt；
前两种供应商/通知清理状态还可为 not_required。不要把 not_required 当作发送或撤权失败。

## 4. 存量迁移

`POST /api/v1/end-users/migrations/import`（M）是唯一接收外部密码哈希的入口：
输入 `siteKey,externalRef,identifierKind,identifier,passwordHash`，可选
`mustChangePassword,lockedUntil,failedLoginCount,credentialVersion,locale,status`；
接受 Argon2 / bcrypt，原归档状态可保留。联系方式导入不等于已经验证。

同 externalRef 且原哈希/标识一致回 created:false，不重复占额；不一致则停止该行交由人确认，
不得覆盖成静默改密；墓碑不复活。先备份、演练和核对，再切换入口，避免两边同时维护凭据。
迁移后平台已改过的口令不能无损回流；回滚需要冻结写入、按事实对账并安排受控重设。

## 5. 条款与接受证据

使用平台托管条款时，由租户在用户中心发布，App 按接口读取当前版本。

| 服务接口 | scope | 输入 |
| --- | --- | --- |
| `GET /api/v1/end-user-policies/current?siteKey=...` | read | 取当前生效各文档，含正文 |
| `POST /api/v1/end-user-policies/acceptances` | manage | `endUserId,docId,requestId`；可选 context:"app"、contextRef |
| `GET /api/v1/end-user-policies/acceptances` | read | `endUserId,docKey,at?`，查询某时刻之前已接受的版本 |

这里 read/manage 指 endusers.read/manage。主体和文档须在本租户/owner/site 内，
requestId 同号同内容幂等、异内容冲突。平台托管注册自动记录必选当前版本；
App 自己展示后的接受由 App 服务后端代表用户记录，终端用户 token 不直接写接受接口。

## 6. 个人数据请求、导出与保留

租户 UI 受理 export/rectify/erase，冻结身份域、网站用途同意域及本租户当时已接入 App 的参与清单，
并可补漏；不会自动发现所有历史外部系统。身份删除后请求仍可追踪。
更正请求只协调处理，平台身份标识/联系方式须走验证流程，同意证据须由本人重新选择；
不能把 rectify 当作管理员任意改资料的后门。
参与方不另建登记表：本租户 App 仍 active，且有效服务账号获准 privacy.requests.manage、
租户访问范围匹配，即可在新请求冻结清单时纳入；已有请求遗漏的参与方由有权操作人补入。
当前没有服务态或终端用户态“发起个人数据请求”接口，受理入口是用户中心；
App 可以提供自己的申请入口并安排转办，不能虚构一个平台 POST 端点。

App 后端的协同接口：

```text
GET  /api/v1/privacy-requests/assignments?state=pending&limit=...
     # privacy.requests.manage，拿自己的待办/期限/subjectRef
POST /api/v1/privacy-requests/assignments/<assignmentId>/receipt
     # privacy.requests.receipt
     {receiptId,state,note?,receiptRef?,detail?}
GET  /api/v1/privacy-requests/<requestId>
     # privacy.requests.read，仅参与方可读，他方回执detail不公开
```

待办响应为 `data.items`，每项字段：

```text
{assignmentId,requestId,requestRef,action,subjectKind:"end_user",
 subjectRef:"enduser:<主体UUID>",ownerAppKey,siteKey,policyVersion,dueAt,overdue,state,attempts}
```

requestId 是平台请求 UUID，用于读进度；requestRef 是受理来源单号，用于本人导出路径的 ref，
不能把两种引用混用，也不能把 ref 当免鉴权凭据；拼进 URL 时编码。由待办取得 ref 后，只向经过本地授权的对应用户展示。
不传 state 时默认拉 pending/unknown/failed，limit 最多 200；App 自己实现持久办理和续跑，
不能只拉一次 pending 后忽略处理结果不明的项。

receiptId 幂等；state 为 done/exception/unknown/failed。exception 必须写保留依据 note；
pending/unknown/failed 仍令整体 open，全部 done 才 completed，含保留例外为
completed_with_retention。超期待办会升级给租户 active 管理员。不要把“身份清理工作项人工重试”
写成“所有 App 隐私待办都有管理员重试按钮”。

平台持有数据的导出有两条路径：

- owner 后端经 read 调 `GET /api/v1/end-users/<id>/export`。
- 已受理的 export 请求，本人持当前有效终端用户 Bearer 调
  `GET /api/public/privacy-requests/<ref>/export`，只到请求 dueAt，不能转发为免鉴权签名 URL。

平台包包含身份、明文标识/联系方式、会话、认证事件和条款接受证据；跨系统本人包另含网站用途
同意记录。无密码/哈希/验证码/token/HMAC/密文，也不打包 App 业务文件；业务数据由各参与 App
自行交付。当前网站同意记录读取有限额，不承诺无限量历史全导出。用户中心只跟踪进度，没有下载按钮。

用户中心可维护四类平台数据的保留期限、依据、责任方及法务保全，预演/执行到期清理；
未配置的类别不会按猜测的期限清理，保全阻止到期清理。条款接受证据的期限从主体删除起算。
保留依据会体现在请求完成状态；备份与外部副本不在平台这一次清理承诺内。
各 App 仍需执行自己域的保留/清理，并如实回执。

## 7. 处置事实、恢复与通知设备

`GET /api/v1/end-users/journal?after=<cursor>&limit=<1到500>`（R）返回本 owner 的
`items:[{cursor,action,endUserId,siteKey,scope,occurredAt}],nextAfter`。
用于补拉平台删除、归档/解除、撤会话、凭据/标识失效、推送注销等事实。
游标单调但不连续，未知 action 忽略；事实可长期补拉，不能因跳号报丢数据。
满页 nextAfter 为末条游标、短页为 null；追平时仍持久保存最后已处理游标，不能重置成起点。
scope 是动作相关对象，字段须按 action 解释；命令/基线变体可增加字段，不能要求每条都有全部字段。
当前已知处置及消费含义：

| action | 关键 scope 字段（按实际记录提供） | App 如何处理 |
| --- | --- | --- |
| subject_deleted | credentialVersion、privacyRevision；命令来源还带 commandId/operationKey | 主体已删，停止关联访问并按本域清理/保留策略办理 |
| subject_archived / subject_unarchived | credentialVersion；命令来源可只有 commandId/action | 归档/解除归档事实；解除后仍重新检查当前资格，不复活旧会话 |
| session_revoked | sessionId | 只失效对应设备的本地会话/缓存 |
| sessions_revoked | credentialVersion，可有 revoked/commandId | 失效该主体全部本地登录判断，不只处理当前设备 |
| credential_invalidated / identifier_invalidated | credentialVersion 或 commandId/action，可有 via | 失效旧身份/凭据缓存，重新取得当前身份事实，不据此推断新标识值 |
| account_deletion_prepared / account_deletion_aborted | commandId、operationKey、sequence、credentialVersion | 对应注销开始阻断/中止；按持久命令续跑，中止不恢复旧会话 |
| phone_binding_revoked | bindingId、reason，可有 keptReserved | 撤销旧手机登录绑定；reserved 不代表可短信登录 |
| push_device_revoked | deviceId、reason、dispositionSeq | 使指定推送设备绑定失效 |
| push_devices_invalidated | hostGeneration、reason、count | endUserId 为 null；只按该 siteKey 处理过时主域代次的推送订阅 |
| apple_grant_received / apple_grant_revoked | profileId、configId | 供应商授权义务事实，endUserId 可为 null；不据此授予业务登录或把整站用户下线 |

**endUserId:null 不自动代表整站处置**：必须先看 action；例如 Apple grant 事件不是全站撤权。
这些是已发生事实，不能代替发送/授权前的当下资格自省；未知 action 跳过但推进已处理游标。
它与隐私 assignments 互补：前者是已发生的事实，后者是要本 App 办理并回执的请求。

灾备恢复后平台可用处置日志重施限制，并返回逐主体 recovery 信息；
不能靠重新登录、解封或改一个状态解除恢复隔离。凭据重新证明的受控重设须匹配恢复代次和
credentialVersion；具体环境的恢复流程由平台运维负责。

终端用户 Web Push 使用**真实站点主域的本人 Cookie 会话**：

```text
POST /api/public/end-user-push/subscribe  {csrf,subscription}
GET  /api/public/end-user-push/devices
POST /api/public/end-user-push/revoke     {csrf,deviceId}
```

写操作需要同源与 CSRF；服务账号、成员 TDT、单独 owner access 不能替他人登记 endpoint。
每人最多 20 台，绑定用户凭据代次及站点主域代次；改密、全量下线、删除、域名换代使旧订阅失效。
撤订不受发送开关限制；同 endpoint 换人时更新归属。此处不是成员的 APNs/FCM `/api/me/devices`。
已验证邮箱会关联平台交易通知联系依据，但营销同意必须另行取得；普通外部联系人也不天然是终端用户。

独立 BFF 的托管登录不会自动创建上述站点 Cookie；不能用 owner token 换一个 Cookie 来绕过此闸。
需要 Web Push 时先完成真实 Sites 主域的本人站点登录。这里没有终端 iOS APNs/FCM 登记合同，
也不能直接套用成员设备接口。

## 8. 外部通知：联系依据与发送接口

外部通知使用自己的服务态 scope，不走成员 `notification.send`。
manifest 的 `notificationEvents` 先登记事件/schema/模板，允许 external 受众及 transactional/marketing
用途；用途由事件目录固定，发送时不能自报为交易用途。发送另需当前安装、租户权益、平台发送模式和
真实渠道配置；off 不受理，shadow 只渲染抑制、不发送不占额，切 active 也不会补发旧 shadow 事件。

最小邮件事件清单片段（key 是本 App 内的事件名，发送 eventType 使用同一值，不再加 listingKey 前缀）：

```json
{
  "notificationEvents": [{
    "key": "order.ready",
    "schemaVersion": 1,
    "name": "订单状态更新",
    "payloadSchema": {
      "type": "object",
      "properties": { "orderRef": { "type": "string", "maxLength": 40 } },
      "required": ["orderRef"],
      "additionalProperties": false
    },
    "templateVariables": [
      { "path": "orderRef", "type": "string", "exposure": "external_safe" }
    ],
    "supportedChannels": ["email"],
    "defaultChannels": ["email"],
    "sensitivity": "normal",
    "defaultLane": "standard",
    "allowedAudienceKinds": ["external"],
    "purpose": "transactional",
    "templates": {
      "zh-CN": { "title": "订单 {{orderRef}} 状态已更新", "body": "请进入应用查看订单 {{orderRef}}。" }
    }
  }]
}
```

提交 manifest 并完成治理审批后，发送时用 `eventType:"order.ready",schemaVersion:1`、
`payload:{"orderRef":"ORDER-001"}`。sourceAppKey 来自服务身份，不在 body 冒认。
定义的 key、schemaVersion、payloadSchema、模板变量、渠道、敏感等级、lane 和用途都影响接入；
默认 audience/purpose 是 member/internal，外部事件必须显式改为 external 及交易/营销用途。
模板只能引用已声明变量；邮件/锁屏变量按外部可展示性声明，不能塞入秘密或身份令牌。
更改 schema 按新 schemaVersion 发布，省略已发布版本会使其弃用，历史受理不等于可以继续接收新事件。
缺模板可能路由失败；定义及真实发件渠道就绪后再做投递验收。

| 接口 | scope/输入 |
| --- | --- |
| `PUT /api/v1/external-notification-recipients/<externalRef>` | contacts；普通邮箱联系依据用 `{siteKey,email,locale?,expectedAddressVersion?,transactionBasis?,marketingConsent?}`，channel 缺省 email |
| `POST /api/v1/external-notification-recipients/<id>/revoke` | contacts；`{purpose:"transactional"或"marketing",consentVersion?,reason?}` |
| `DELETE /api/v1/external-notification-recipients/<id>` | contacts；`{expectedVersion?,reason?}`，清地址/渲染密文并取消排队项 |
| `POST /api/v1/external-notification-events` | send；`{producerEventId,eventType,siteKey,recipientIds,payload,schemaVersion?,occurredAt?,validUntil?,siteId?,actionPath?}` |
| `GET /api/v1/external-notification-events/<eventId>` | send；回读本 App 的脱敏事件及邮件投递结果 |

这里 contacts/send 指 notification.external.contacts/send。登记联系依据返回 recipient DTO，
其 id 才能放进 recipientIds；transactionBasis 为 `{ref,at?}`，marketingConsent 为
`{version,evidenceRef?,evidenceDigest?}`，新营销授予须高于当前 consentVersion，不能由旧授予覆盖撤回。
平台已托管的收件人地址/删除状态不能经此普通登记接口修改或复活。

平台自动关联的终端联系人使用 `enduser:<endUserId>` 稳定引用，但普通 EndUser DTO 不返回通知 recipientId，
事件接口也不接受 endUserId 代替 recipientIds。**当前无按 endUserId 回读通知 recipientId 的服务接口**；
若调用方只有终端主体 ID，不能宣称已具备直接发送能力、猜 ID 或滥用数据导出来建立地址副本。
普通 App 自有且已有合法联系依据的联系人，可按上面的登记响应保存 recipientId。
共享通行证 owner 与业务通知来源不同，也不自动互享对方联系人，须先解决获准的来源/收件人映射。

受理成功为 `{eventId,status:"accepted",duplicate,shadow}`；producerEventId 按同一业务事件稳定重试，
accepted 不等于 delivered，更不等于 read。只用 recipientIds，不接受成员 audience 或任意绝对 link；
营销须有已同意的收件人并绑定本 App 的真实 siteId；营销及带 actionPath 的交易事件还要求主域验证和
路由就绪。actionPath 为站内相对路径，实际链接由平台按站点主域生成。
事件查询、邮件送达证据、站点 Web Push 与用户阅读分别判断，不能把一个渠道的状态当作全部渠道已送达。

## 9. 接入验收

至少验证普通读取/最小scope、跨tenant/owner/site拒绝、重复与冲突登记、封禁与解封、单条/全部撤会话、
批量部分失败及丢回包续跑、导入不一致拒绝、删除墓碑及额度释放、隐私三种完成状态、自己的处置流补拉。
涉及本人删除时验证重新认证用途不能串用；涉及供应商、推送或灾备时另做实际环境验收。
业务不存在与业务失败看 Envelope，不能用 HTTP 200 代替成功判断。
