# 租户终端用户：能力、管理入口与接入选择

> 适用于顾客、学生、网站访客注册后的账号等 `kind=enduser` 主体。本文及同目录参考可随 skill 整体复制到独立 App 仓库，不需要门户源码。
> 以下是平台实现的能力范围；目标环境的版本、安装、授权和渠道配置仍须实际核验。示例中的尖括号内容均为占位。

## 按任务继续阅读

- Web BFF / iOS 托管注册、登录、找回、PKCE、自省、刷新、本人重新认证：读 [终端用户认证协议](end-user-auth.md)。
- App 后端建号、密码重置、客户端登记、批量同步、迁移、条款、隐私请求、处置日志、推送：读 [账号生命周期与隐私接口](end-user-lifecycle.md)。
- 门户成员的原生客户端登录：读 [成员原生客户端](native-clients.md)。成员的 `/oauth/*`、`publicClient` 和 `redirectUrls` 不用于本文终端用户。

## 1. 先确定身份与归属

| 对象 | 身份与职责 |
| --- | --- |
| 门户成员 | `kind=user`，使用门户及其 iframe 微应用，拥有成员角色、组、PID/ACL，可能占 App 成员席位 |
| 终端用户 | `kind=enduser`，不进入门户成员目录、不占成员席位、不能因此登录门户后台；身份与认证由平台托管，业务资料及业务权限归 App |
| App 服务账号 | `kind=service`，代表后端管理获准命名空间，不是终端用户本人的认证证明 |

终端身份命名空间固定为 `(tenantId, ownerAppKey, siteKey)`。同一邮箱在两个空间是两个主体；
`endUserId` 是全局唯一、不会回收给另一人的稳定业务关联标识。删除保留 ID / `externalRef` 墓碑，
释放旧登录标识符；同一个 `externalRef` 不能用重复登记静默复活。

先与租户约定身份 owner：可以是声明了 realm 的业务 App，也可以使用平台内建的 `user-center`
作为共享通行证 owner。多个客户端绑定同一个 profile 时可共用身份与认证域浏览器登录状态，
但各自的应用会话、业务资料和权限不会自动互通。服务账号绑定的 owner 必须与所选身份空间一致；
不能靠请求 body 冒充 `user-center` 或其他 owner。

`user-center` 当前是有前端的仓内微应用，后端由门户 API 承担；没有独立
`user-center-server` 或 `/svc/user-center`。不要在外部 App 清单里再声明一份用户中心。

## 2. 启用条件与分工

| 谁负责 | 配置或事实 |
| --- | --- |
| 平台管理员 | 认证域名白名单（精确 HTTPS origin）、独立于 `SESSION_SECRET` 的 `SETTINGS_ENC_KEY`、平台 SMTP、邮箱验证码运行参数、终端用户容量权益及服务账号授权 |
| 身份 owner App | 自己的 manifest 声明 `endUserRealms`；需要服务管理时申请相应特权 scope |
| 租户获授权的操作人 | 安装用户中心、分配用户中心权限；在用户中心登记 profile、运行策略、条款和 Apple 配置 |
| 接入 App 后端 | 经服务接口登记自己的客户端和精确回调；保存应用会话、业务资料、业务授权，并办理本 App 的隐私待办 |

身份 owner 必须已安装，租户须 active，realm 必须允许该 `siteKey`，且租户启用了
`portal.end_users`。托管登录还需启用的 profile/client、已登记回调、可用认证域名与独立加密密钥。
Apple 另需有效供应商配置；邮箱注册/找回另需平台 SMTP 与完整验证码配置。缺项应报告未就绪，
不能回退到另一套宽松认证。已有密文的环境不能直接替换加密密钥。
平台验证码固定使用平台发件身份，不借用租户或 App 的 SMTP 配置。

若由业务 App 自己承载身份，manifest 可声明：

```json
{
  "endUserRealms": { "mode": "dynamic" },
  "privilegedServiceScopes": [
    { "scope": "endusers.read", "reason": "读取本应用的终端身份与接入登记" },
    { "scope": "endusers.clients", "reason": "登记本应用的认证客户端及精确回调" }
  ]
}
```

固定空间改用 `{"mode":"fixed","siteKeys":["customers"]}`。这里只声明范围，不创建 profile/client，
也不授予额度。特权 scope 必须审批；不放进成员 `scopes` 或普通 `serviceScopes`。
只做托管登录不必为此取得可改密码的 `endusers.manage`；按实际接口最小化申请。

## 3. 租户管理界面：四个页面

用户中心需要显式授予 landing / 页面 / 动作权限，租户 admin 的 ACL bypass 不生效。
安装会建立预置“用户中心”角色，但不会自动发给全员。页面 PID 为
`user-center:page:endusers`、`privacy`、`hosted`、`apple`（后三者同样使用 `user-center:page:` 前缀）。
动作分为 `enduser.revoke`、`enduser.status.manage`、`enduser.privacy.retry`、
`enduser.privacy.request`、`hosted.register`、`apple.manage`，使用 `user-center:action:` 前缀。
管理 API `/api/v1/user-center/*` 另要求用户态 TDT 与 `user_center.read/write`；
它不是给 App 后端使用的服务管理接口。

| 页面/区块 | 可以做什么 | 关键边界 |
| --- | --- | --- |
| 终端用户列表 | 按 App、siteKey、active/locked/archived/deleted 筛选；每页 50 条继续加载；看脱敏账号、externalRef、在线会话数、最近登录 | 默认隐藏 deleted 墓碑；卸载后仍有数据的 owner 为只读 |
| 精确定位与详情 | 完整邮箱/手机号精确查找；看标识类型、语言、创建/登录/锁定时间、主联系方式与验证状态、密码有无/首登改密要求 | UI 无用户名或模糊搜索；无业务姓名、订单、学籍等资料 |
| 会话与封禁 | 查看浏览器/设备会话、设备 ID、脱敏 IP、UA、最近活动；单条/全部下线；封禁与解封 | 封禁即 archived，继续占额；解封不恢复会话、不解除登录失败锁或恢复隔离 |
| 登记自检 | 区分没声明 realm、realm 撤销、siteKey 越界、允许但未登记 profile；查看认证域名、方法、注册开关、客户端与回调 | 只读，不在此卡片修改 |
| 个人数据请求 | 受理 export/rectify/erase，填来源单号、期限、备注；按状态/主体引用查请求，追踪参与方、回执、逾期与保留说明，补遗漏方 | 只有三类动作，无独立 anonymize；更正不提供直接改身份资料表单 |
| 清理工作项 | 在用户详情看清理进度、重试次数、错误、下次尝试、执行租约及人工记录；按权限重试 | 接收重试不等于完成；不抢正在执行的任务，不重试 done |
| 保留策略 | 配置网站用途同意证据、认证事件、已办结请求、条款接受证据的期限、依据、责任方、法务保全；预演计数/执行清理 | 未配无统一默认期限；法务保全阻止清理；条款接受期限从主体删除起算 |
| 托管登录/profile | 搜索、新建、编辑、启停；选择 owner/siteKey、名称、认证域名、password/apple、自助注册、Apple 配置；预览确认与修订冲突比较 | owner/siteKey 创建后不可改绑；域名只能选平台白名单；已有非空 Apple 配置引用不可原地改绑 |
| 客户端 | 查看 ID、web_bff/ios 类型、名称、状态、协议、回调；改名、启停 | UI 不能新建或改回调；停用只阻止新登录，不撤销已有会话 |
| 条款与政策 | terms/privacy/自定义 docKey；简中必填、英/繁可选；全文/摘要/版本查看；注册必选、立即/定时生效、继承上一版、预览发布 | 已发布不可编辑、删除、撤回，只能发新版；接受证据另经服务 API 查询 |
| Apple 登录 | 配置团队、Key、Services ID、primaryAppId、subjectDomain、回调源、私钥及状态；轮换 Key、启停、预览确认 | 私钥加密、只进不出；已有身份坐标不可改绑；无删除配置 |

profile 的高级参数使用明确值：

- 保留期（秒）：`pendingRegistrationSeconds`、`terminalFlowSeconds`、`proofSeconds`、
  `authEventSeconds`、`receiptSeconds`、`providerFailureSeconds`。
- 限流（次/分钟）：`clientPerMinute`、`namespacePerMinute`。

管理 UI 没有建号、批量导入、直接改标识/联系方式、设置/重置密码、解除失败锁定或直接删号按钮。
页面删号通过 erase 请求受理；App 的对应服务能力见 [生命周期接口](end-user-lifecycle.md)。

## 4. 容量与运维

`portal.end_users` 是租户共享、无时间窗口的资源容量：每个新主体占 1 格，不按自然人去重。
跨 App/siteKey 的不同主体分别计数；重复幂等登记不重复占额；archived 仍占额，deleted 归还。
满容量只阻止新建，不踢已有用户。平台配置套餐/授予/增购，租户可看用量并设更严上限。
不要用成员 `seatBased` 代替；业务资源的 `seatRoles` 是另一类配额，不能替代或重复结算平台身份容量。

平台提供认证域名与邮箱验证码参数/就绪状态管理、按用户状态计数、清理 pending/failed/最旧积压巡检；
跨租户清理项查询与重试另有运维 API。主体总行数包含墓碑，不等于容量用量。
清理、处置日志归档验证和灾备重施属于平台运维；App 仍需消费自己的处置日志与隐私待办。

## 5. 当前限制与验收

- 托管方法是 password/apple；Google 未实现。`bind_apple`、`add_password`、`unlink_apple`
  是保留用途，当前拒绝，不能仅凭字段存在实现绑定/解绑按钮。
- Apple 按系统浏览器托管流程接入；当前未提供原生 Apple SDK 票据直接兑换终端会话的合同。
  终端推送明确交付的契约是 Sites Web Push，不据成员能力推断终端 iOS APNs/FCM 可用。
- 站点/设备面已有手机 OTP login/bind/recover、换号/解绑逻辑，但本仓当前没有生产短信 sender 注册，
  会报 `channel_adapter_missing`；不是“填配置就能发短信”，也不是托管 profile 的登录方法。
- 邮箱发送的是数字验证码，没有 magic-link 登录。验证邮箱只建立交易联系依据，不授予营销同意。
- 删除是清身份凭据、撤销会话、释放容量并保留墓碑，再异步清通知与 Apple 授权；
  身份已删、平台清理完成、跨系统请求办结分别判断。平台不承诺同步清空 App、备份或外部副本。
- 对需跨库同步的 owner，直接写入受限；例如 `lms-school` 须走命令，用户中心不能直接封禁/解封。
- 用户中心无本人注销、导出下载或 Web Push 设备管理按钮；这些通过本人/接入方接口提供。

交付时实际验证：身份空间和客户端登记、注册/登录/找回/首改、错误 state/verifier/回调、
刷新单飞与退出、单会话和全部撤销、封禁解封、跨租户/owner/site 拒绝、容量不足、
隐私参与方回执与保留例外。若接入 Apple、短信、原生端或推送，分别验证真实供应商/设备/浏览器链路；
源码能力和元数据不替代这些验收。
