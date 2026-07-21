# LLM-GATEWAY-1 · XGENT.ai Portal · LLM Gateway App 开发计划

> 本期交付一个面向租户的独立 App「LLM Gateway」：统一大模型入口，负责把用户请求转发到不同模型供应商，并处理渠道选择、失败重试、模型管理、调用日志、用量统计、异步媒体任务和在线测试。
>
> 形态参考文件管理 / 研发项目管理 / 学校管理等独立 App：`llm-gateway-server` 自带后端和数据库，`llm-gateway-app` 作为 Portal 内 iframe 微应用；Portal 侧只新增市场清单、服务注册和必要 scope。

---

## 0. 本期范围与关键决策

| 决策点 | 选择 | 含义 |
| --- | --- | --- |
| App 形态 | **独立后端 App** | Gateway 是高吞吐、强 I/O、强流式代理服务，不并入 `apps/api`；独立端口、独立数据库、独立路由和日志。 |
| 对外协议 | **OpenAI 兼容优先，Claude / Gemini 兼容补齐** | 先保证 OpenAI SDK 生态可直接接入，再提供 Claude / Gemini 风格端点和转换层。 |
| 上游适配 | **Provider Adapter 插件化** | 每个供应商一个 adapter，统一暴露 chat / image / audio / embedding / rerank / realtime / task 能力。 |
| 路由策略 | **模型映射 + 渠道策略** | 同一模型可绑定多个渠道，按分组、优先级、权重、状态、限流和失败熔断选择上游。 |
| 日志与用量 | **请求级完整记录** | 每次调用记录模型、渠道、token、耗时、首包时间、错误、用户、租户；支持个人和租户管理员视角。 |
| 异步任务 | **统一任务抽象** | Midjourney / Suno / Sora / Kling / Jimeng / Gemini/Veo 等统一为 submit / get / cancel / callback。 |
| 鉴权 | **Portal TDT + Gateway API Token** | 管理端走 Portal 会话/TDT；程序调用走租户内 API Token，Token 绑定用户、scope、限额和过期时间。 |
| 前端 | **控制台 + Playground + Chat** | 管理员管理模型/渠道/日志/监控；普通用户可看个人日志并使用 Playground / Chat。 |

### 0.1 明确不在本期

- 不做计费扣款和发票系统；只沉淀 token / 请求 / 成本估算数据，为后续计费留字段。
- 不承诺所有厂商所有接口 100% 等价；本期以统一抽象和核心能力可用为目标。
- 不做复杂 Prompt 管理、Agent 编排、工作流编排；Gateway 只负责模型接入、路由和调用。
- 不做跨租户共享供应商密钥；每个租户的渠道和密钥完全隔离。
- 不把上游完整请求体和响应体长期明文保存；默认只保存元数据、错误摘要和可选调试采样。

### 0.2 角色与权限

| 角色 | 能力 |
| --- | --- |
| 普通用户 | 使用 API Token 调用允许的模型；查看自己的调用日志；使用 Playground / Chat。 |
| 租户管理员 | 管理本租户渠道、模型、路由策略、API Token、租户日志、用量和监控面板。 |
| Portal 平台管理员 | 本期不新增平台级运营后台；后续可扩展查看跨租户健康状态和全局供应商模板。 |

---

## 1. 拓扑与工作区

```
apps/
  llm-gateway-server/   ★新 · 独立后端 (Bun + Elysia + Drizzle + PostgreSQL + Redis) :4500
                        OpenAI/Claude/Gemini 兼容 API、上游 adapter、渠道路由、日志、任务、Realtime WS
  llm-gateway-app/      ★新 · Portal 微应用 (Vite + React + TypeScript + shadcn/ui + Tailwind) :5180
                        渠道管理、模型管理、日志、用量、Playground、Chat、监控面板
  api/                  Portal 后端 · 仅新增 marketplace seed、服务账号 seed、scope、env 字段
  web/                  Portal 壳 · SERVICE_REGISTRY 加 llm-gateway，CSP / env 放行
packages/
  shared/               新增 gateway scope / 错误码 / 可选 DTO
  portal-sdk/           复用 sdk.callService("llm-gateway", ...)
```

端口约定：

| 服务 | 端口 |
| --- | --- |
| Portal API | `3000` |
| Portal Web | `5173` |
| LLM Gateway App | `5180` |
| LLM Gateway Server | `4500` |

### 1.1 环境变量

```ini
# llm-gateway-server
LLM_GATEWAY_DATABASE_URL=postgres://postgres:postgres@localhost:5432/xgent-llm-gateway
LLM_GATEWAY_SERVER_PORT=4500
LLM_GATEWAY_PUBLIC_BASE_URL=http://localhost:4500
# 接受的 TDT aud = 市场清单 listingKey/appKey（独立后端的第一道安全闸，见 §7.1）
LLM_GATEWAY_AUDIENCE=llm-gateway
# Portal 自省端点 + 服务账号 Basic 凭证：平台管理员在控制台建一个带 token.introspect 能力的
# 服务账号（/api/console/service-accounts），网关用 clientId:secret 走 HTTP Basic 自省。
PORTAL_INTROSPECT_URL=http://localhost:3000/api/tokens/introspect
LLM_GATEWAY_SA_CLIENT_ID=llm-gateway-server
LLM_GATEWAY_SA_CLIENT_SECRET=<服务账号密钥·控制台轮换时只显示一次>
# dev 通道 fallback：x-resource-key 仅 DEV_MOCK_OAUTH=true 时门户才可能比对，且门户只认
# FILES_RESOURCE_KEY 一把 key，不识别本 App 的 key；生产无效，切勿当自省主凭证。
LLM_GATEWAY_RESOURCE_KEY=
# CORS：宿主代理（sdk.callService）跑在 Portal web 源，必填；App 源仅供独立调试/健康检查。
PORTAL_BASE_URL=http://localhost:5173
LLM_GATEWAY_APP_URL=http://localhost:5180
# 可选：缺省降级为进程内 Map（自省缓存 + 限流），保持依赖轻量。
REDIS_CONN_STRING=redis://localhost:6379/7

# apps/web
VITE_LLM_GATEWAY_APP_URL=http://localhost:5180
VITE_LLM_GATEWAY_SERVER_BASE=http://localhost:4500
```

供应商 API Key 不放入环境变量作为租户渠道配置；由租户管理员在 Gateway 控制台新增渠道后加密入库。

---

## 2. 核心能力分层

### 2.1 对外 API 层

Gateway 对外暴露三类 API：

| 类型 | 路径前缀 | 说明 |
| --- | --- | --- |
| OpenAI 兼容 | `/v1/*` | 优先支持 OpenAI SDK：chat completions、responses、images、audio、embeddings、models、rerank 扩展、realtime。 |
| Claude 兼容 | `/anthropic/v1/*` | 支持 Messages API 风格请求，内部转换成 Gateway 标准调用。 |
| Gemini 兼容 | `/gemini/v1beta/*` | 支持 generateContent / streamGenerateContent / embedContent 等常用接口。 |
| Gateway 管理 API | `/api/v1/*` | 渠道、模型、Token、日志、用量、任务、健康面板，供 Portal 微应用调用。 |
| Realtime WebSocket | `/v1/realtime` | OpenAI Realtime 兼容入口；按模型路由到支持 realtime 的上游。 |

业务响应遵循项目约定：管理 API 使用 `HTTP 200 + { ok, data/error }` 信封；兼容 API 则尽量保持对应厂商 SDK 预期格式和错误结构。

### 2.2 标准调用模型

内部统一为 `GatewayRequest`：

```ts
type GatewayCapability =
  | "chat"
  | "responses"
  | "image"
  | "speech_to_text"
  | "text_to_speech"
  | "embedding"
  | "rerank"
  | "realtime"
  | "async_task";

type GatewayRequest = {
  tenantId: string;
  userId: string;
  tokenId?: string;
  protocol: "openai" | "claude" | "gemini" | "gateway";
  capability: GatewayCapability;
  model: string;
  group?: string;
  stream?: boolean;
  input: unknown;
  headers: Record<string, string>;
  timeoutMs?: number;
};
```

每个兼容入口只负责：

1. 鉴权。
2. 解析模型和能力。
3. 转换成 `GatewayRequest`。
4. 调用路由器。
5. 把标准响应转换回对应协议格式。

---

## 3. Provider Adapter 设计

### 3.1 Adapter 统一接口

```ts
interface ProviderAdapter {
  provider: ProviderCode;
  capabilities: GatewayCapability[];
  listModels(channel: ChannelConfig): Promise<ProviderModel[]>;
  healthCheck(channel: ChannelConfig): Promise<ChannelHealthResult>;
  invoke(request: RoutedGatewayRequest): Promise<GatewayResponse>;
  stream?(request: RoutedGatewayRequest): AsyncIterable<GatewayStreamEvent>;
  openRealtime?(request: RealtimeRouteContext): Promise<RealtimeBridge>;
  submitTask?(request: AsyncTaskSubmitRequest): Promise<AsyncTaskSubmitResult>;
  getTask?(request: AsyncTaskGetRequest): Promise<AsyncTaskResult>;
  cancelTask?(request: AsyncTaskCancelRequest): Promise<AsyncTaskCancelResult>;
}
```

### 3.2 本期 Adapter 清单

| Provider | 能力优先级 | 备注 |
| --- | --- | --- |
| OpenAI | chat / responses / image / audio / embedding / realtime | 作为兼容基准。 |
| Azure OpenAI | chat / responses / image / audio / embedding | 处理 deployment name、api-version、endpoint 差异。 |
| Claude | chat / messages / vision | Claude 兼容入口和 OpenAI 输入互转。 |
| Gemini | chat / vision / embedding / video task | generateContent / streamGenerateContent / Veo 类任务。 |
| OpenRouter | chat / image / embedding | 按 OpenAI 兼容实现，补充 provider routing header。 |
| DeepSeek | chat | OpenAI 兼容。 |
| Moonshot | chat | OpenAI 兼容。 |
| 阿里通义 | chat / image / audio / embedding | DashScope adapter。 |
| 百度文心 | chat / embedding | 千帆接口 adapter。 |
| 腾讯混元 | chat / image | Hunyuan adapter。 |
| 智谱 | chat / embedding | BigModel adapter。 |
| 火山引擎 | chat / image / video task | Ark / 即梦相关任务。 |
| SiliconFlow | chat / embedding / rerank | OpenAI 兼容 + rerank。 |
| Ollama | chat / embedding | 本地部署，租户配置 base URL。 |
| Xinference | chat / embedding / rerank | 私有部署，租户配置 base URL。 |
| Midjourney | image task | 异步任务 adapter。 |
| Suno | music task | 异步任务 adapter。 |
| Sora | video task | 异步任务 adapter。 |
| Kling | video task | 异步任务 adapter。 |
| Jimeng | video task | 异步任务 adapter。 |

实现顺序：先 OpenAI / Azure OpenAI / Claude / Gemini / OpenRouter / DeepSeek / Ollama，后补国内厂商和媒体任务。

### 3.3 密钥与敏感配置

- `channels.secretConfig` 使用应用级密钥加密后存储，数据库中不保存明文。
- 前端编辑渠道时，已保存密钥只展示 `sk-****abcd` 形式；更新时可选择保留或替换。
- 测试渠道时只返回可用状态、模型清单和错误摘要，不回显完整上游错误中的密钥片段。

---

## 4. 路由与容错

### 4.1 路由输入

路由器根据以下信息选择渠道：

- `tenantId`
- `model`
- `capability`
- `group`
- API Token 允许的模型范围
- 模型路由配置
- 渠道启停状态
- 渠道健康状态
- 渠道优先级
- 渠道权重
- 渠道限流情况
- 熔断状态

### 4.2 路由算法

1. 找到本租户启用的 `model_aliases` 或 `model_routes`。
2. 过滤支持当前 capability 的渠道。
3. 过滤 API Token 未授权的模型 / 分组。
4. 排除 `disabled`、`unhealthy`、熔断中、限流中的渠道。
5. 按 `priority` 升序分层。
6. 同优先级内按 `weight` 加权随机。
7. 调用失败时按失败类型决定是否重试或切换渠道。

### 4.3 失败处理

| 失败类型 | 行为 |
| --- | --- |
| 连接超时 / DNS / TLS / 5xx | 可重试，优先切换其他渠道。 |
| 上游 429 | 标记短期限流，切换其他渠道；无可用渠道时返回限流错误。 |
| 上游 401 / 403 | 标记渠道配置错误，不继续用该渠道重试。 |
| 模型不存在 | 记录路由配置错误，不重试同渠道。 |
| 请求参数错误 | 直接返回用户错误，不切换渠道。 |
| 流式响应中断 | 记录 partial failure；是否重试由兼容协议决定，默认不自动重放已开始的流。 |

### 4.4 健康状态与熔断

- 每个渠道维护 `healthy / degraded / unhealthy / disabled`。
- 连续失败达到阈值后进入熔断，熔断窗口结束后允许半开探测。
- 管理员可手动执行渠道测试，测试结果写入 `channel_checks`。
- 后台定时健康检查只对启用渠道运行，避免无意义打上游。

---

## 5. 数据模型

> `llm-gateway-server` 使用独立数据库，所有业务表带 `tenantId`，所有查询强制租户隔离。

### 5.1 管理配置表

| 表 | 关键字段 | 说明 |
| --- | --- | --- |
| `gateway_channels` | `id`, `tenantId`, `name`, `provider`, `baseUrl`, `secretConfigEncrypted`, `capabilities[]`, `status`, `priority`, `weight`, `group`, `rateLimitConfig`, `timeoutMs`, `retryConfig`, `metadata` | 上游渠道。 |
| `gateway_channel_models` | `channelId`, `providerModel`, `normalizedModel`, `capability`, `contextWindow`, `inputModalities[]`, `outputModalities[]`, `pricing`, `enabled` | 渠道支持的模型。 |
| `gateway_models` | `tenantId`, `model`, `displayName`, `capabilities[]`, `status`, `defaultGroup`, `metadata` | 租户对外暴露的模型目录。 |
| `gateway_model_routes` | `tenantId`, `model`, `capability`, `channelId`, `providerModel`, `priority`, `weight`, `group`, `enabled` | 模型到渠道的路由配置。 |
| `gateway_api_tokens` | `tenantId`, `userId`, `name`, `tokenHash`, `prefix`, `scopes[]`, `allowedModels[]`, `allowedGroups[]`, `expiresAt`, `status`, `lastUsedAt` | 程序调用 Token。 |
| `gateway_settings` | `tenantId`, `defaultTimeoutMs`, `logPayloadMode`, `retentionDays`, `dashboardConfig` | 租户级设置。 |

### 5.2 调用日志与用量表

| 表 | 关键字段 | 说明 |
| --- | --- | --- |
| `gateway_call_logs` | `id`, `tenantId`, `userId`, `tokenId`, `requestId`, `protocol`, `capability`, `model`, `provider`, `channelId`, `providerModel`, `success`, `errorCode`, `errorMessage`, `statusCode`, `latencyMs`, `firstTokenMs`, `stream`, `inputTokens`, `outputTokens`, `totalTokens`, `estimatedCost`, `createdAt` | 每次调用一行。 |
| `gateway_call_attempts` | `callLogId`, `attemptNo`, `channelId`, `provider`, `startedAt`, `finishedAt`, `latencyMs`, `success`, `errorCode`, `errorMessage` | 一次请求可能多次重试。 |
| `gateway_usage_daily` | `tenantId`, `userId?`, `model?`, `channelId?`, `date`, `requestCount`, `successCount`, `errorCount`, `inputTokens`, `outputTokens`, `estimatedCost` | 日维度聚合，供 Dashboard 快速查询。 |
| `gateway_rate_limit_events` | `tenantId`, `channelId`, `model`, `scope`, `reason`, `createdAt` | 限流事件记录。 |

### 5.3 异步任务表

| 表 | 关键字段 | 说明 |
| --- | --- | --- |
| `gateway_tasks` | `id`, `tenantId`, `userId`, `tokenId`, `provider`, `channelId`, `taskType`, `model`, `status`, `upstreamTaskId`, `inputSummary`, `result`, `errorCode`, `errorMessage`, `submittedAt`, `completedAt`, `expiresAt` | 图片、音乐、视频等异步任务。 |
| `gateway_task_events` | `taskId`, `eventType`, `payload`, `createdAt` | 任务状态变更轨迹。 |

### 5.4 健康检查表

| 表 | 关键字段 | 说明 |
| --- | --- | --- |
| `gateway_channel_checks` | `channelId`, `status`, `latencyMs`, `modelsFound`, `errorCode`, `errorMessage`, `checkedAt` | 手动 / 自动渠道检测记录。 |
| `gateway_provider_incidents` | `tenantId`, `provider`, `channelId`, `level`, `message`, `startedAt`, `resolvedAt` | 可选，用于监控面板展示异常。 |

---

## 6. API 设计

### 6.1 OpenAI 兼容 API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `GET` | `/v1/models` | 返回租户可用模型。 |
| `POST` | `/v1/chat/completions` | 文本 / 多模态对话，支持 stream。 |
| `POST` | `/v1/responses` | Responses API 兼容入口。 |
| `POST` | `/v1/images/generations` | 图片生成。 |
| `POST` | `/v1/audio/transcriptions` | 语音转文字。 |
| `POST` | `/v1/audio/speech` | 语音合成。 |
| `POST` | `/v1/embeddings` | Embedding。 |
| `POST` | `/v1/rerank` | Rerank 扩展接口。 |
| `GET` | `/v1/realtime` | Realtime WebSocket。 |

### 6.2 Claude 兼容 API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `POST` | `/anthropic/v1/messages` | Claude Messages API。 |
| `POST` | `/anthropic/v1/messages?stream=true` | Claude 流式输出。 |
| `GET` | `/anthropic/v1/models` | Claude 风格模型列表。 |

### 6.3 Gemini 兼容 API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `POST` | `/gemini/v1beta/models/:model:generateContent` | Gemini 内容生成。 |
| `POST` | `/gemini/v1beta/models/:model:streamGenerateContent` | Gemini 流式生成。 |
| `POST` | `/gemini/v1beta/models/:model:embedContent` | Gemini embedding。 |
| `GET` | `/gemini/v1beta/models` | Gemini 风格模型列表。 |

### 6.4 Gateway 管理 API

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| `GET` | `/api/v1/bootstrap` | user | 控制台初始化数据。 |
| `GET` | `/api/v1/channels` | admin | 渠道列表。 |
| `POST` | `/api/v1/channels` | admin | 新增渠道。 |
| `PATCH` | `/api/v1/channels/:id` | admin | 修改渠道。 |
| `POST` | `/api/v1/channels/:id/enable` | admin | 启用渠道。 |
| `POST` | `/api/v1/channels/:id/disable` | admin | 停用渠道。 |
| `DELETE` | `/api/v1/channels/:id` | admin | 删除渠道。 |
| `POST` | `/api/v1/channels/:id/test` | admin | 测试渠道可用性。 |
| `POST` | `/api/v1/channels/:id/sync-models` | admin | 拉取上游模型清单。 |
| `GET` | `/api/v1/models` | user | 模型列表。 |
| `POST` | `/api/v1/models` | admin | 新增对外模型。 |
| `PATCH` | `/api/v1/models/:model` | admin | 修改模型。 |
| `PUT` | `/api/v1/models/:model/routes` | admin | 配置模型渠道、优先级和权重。 |
| `GET` | `/api/v1/api-tokens` | user | API Token 列表。 |
| `POST` | `/api/v1/api-tokens` | user | 创建 API Token。 |
| `DELETE` | `/api/v1/api-tokens/:id` | user/admin | 撤销 API Token。 |
| `GET` | `/api/v1/logs` | user/admin | 普通用户看自己，管理员可看租户。 |
| `GET` | `/api/v1/usage/summary` | user/admin | 用量概览。 |
| `GET` | `/api/v1/tasks` | user/admin | 异步任务列表。 |
| `POST` | `/api/v1/tasks` | user | 提交异步任务。 |
| `GET` | `/api/v1/tasks/:id` | user/admin | 查询任务结果。 |
| `POST` | `/api/v1/tasks/:id/cancel` | user/admin | 取消任务。 |
| `GET` | `/api/v1/monitoring/dashboard` | admin | Dashboard 面板数据。 |

### 6.5 健康与监控 API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `GET` | `/healthz` | 进程存活检查。 |
| `GET` | `/readyz` | 数据库、Redis、Portal introspection 可用性检查。 |
| `GET` | `/metrics` | Prometheus 风格指标，后续可接监控系统。 |
| `GET` | `/api/v1/monitoring/portal-card` | Portal Dashboard 展示用摘要：请求量、成功率、平均耗时、异常渠道数。 |

---

## 7. 鉴权与权限控制

### 7.1 管理端鉴权

Portal 微应用通过 `sdk.callService("llm-gateway", ...)` 调 Gateway 管理 API。`llm-gateway-server` 是独立资源服务器，**不验 JWT 签名**（HS256 密钥只在 Portal），而是把 TDT 发给 Portal 自省端点换取已解析声明后自己设闸（参考 `apps/qbank-server/src/lib/gate.ts`）：

1. Portal host 为当前用户签发 `aud=llm-gateway` 的 TDT；宿主代理（`callService`）带 `Authorization: Bearer` 转发。
2. `llm-gateway-server` 调 `POST /api/tokens/introspect`（服务账号 Basic 鉴权）验证 TDT，结果缓存到 `exp` 前（上限 60s）。无效/已撤销 TDT 也是**成功**自省、返回 `active:false`，不要当传输错误。
3. **四道闸，缺一不可**：
   - `aud === "llm-gateway"`（否则 `INVALID_TOKEN` / 401）—— 跨应用隔离的权威闸，**不能省**；`callService` 只是转发便利层，不替你授权。
   - 所需 `scope ∈ scopes`（否则 `INSUFFICIENT_SCOPE` / 403）。
   - 结构性管理操作看 `role === "admin"`（`requireAdmin`；角色来自 Portal 成员关系，非 TDT claim）。
   - 细粒度操作看 ACL：`bypass || permissions 命中目标 PID`（`requirePerm`，见 §7.4）。
4. **租户隔离**：每条查询按 `claims.tenant_id` 收口。

新增 scope（定义在 `packages/shared/src/scopes.ts`；命名空间用下划线前缀 `llm_gateway`，与 listingKey / aud 的连字符写法 `llm-gateway` 分属两套命名空间）：

| Scope | 用途 |
| --- | --- |
| `llm_gateway.read` | 查看模型、个人日志、Playground、Chat。 |
| `llm_gateway.write` | 创建 API Token、提交任务。 |
| `llm_gateway.admin` | 让管理 API 进入该用户的授权申请范围（渠道、模型路由、租户日志、监控的**可见性**）。 |

> scope 表达「应用能代表用户做什么」，**不表达角色**。管理操作的权威闸是上面第 3 道的 `role === "admin"` 与第 4 道的 ACL；`llm_gateway.admin` **不能**仅凭持有就放行管理操作（避免「持 scope 即管理员」的错觉）。

### 7.2 API Token 鉴权

调用兼容 API 使用：

```http
Authorization: Bearer xgw_...
```

Token 创建时只展示一次明文，数据库保存哈希。Token 可绑定：

- 过期时间
- scopes
- allowed models
- allowed groups
- 每分钟请求数
- 每日 token 上限
- 是否允许异步任务
- 是否允许 realtime

### 7.3 审计

以下操作写审计日志：

- 创建 / 修改 / 删除渠道
- 启停渠道
- 更新渠道密钥
- 同步模型
- 修改模型路由
- 创建 / 撤销 API Token
- 查看敏感配置摘要
- 手动测试渠道

### 7.4 权限模型（ACL Manifest）

仅靠 `role=admin` 粗粒度门无法表达「普通用户能用哪些能力、能否提交异步任务、能否看租户日志」这类细粒度。LLM Gateway 与现有大 App（spms / files / sms / qbank）一致，在 `apps/api/src/modules/acl/manifests.ts` 注册一份 `AclManifest`（按 `listingKey` 进 `MANIFEST_REGISTRY`、seed 盖章到清单），从而：① 进「权限管理 › 角色矩阵」可授权；② `defaultForMember` 项授予 `member` 基线；③ 自省回灌 `permissions[]` / `bypass`，供后端 `requirePerm` 做安全门、前端 `sdk.acl.can()` 做 UX 门。

草案（PID 语法 `llm-gateway:<page|action>:<key>`）：

| 类型 | key | 说明 | defaultForMember |
| --- | --- | --- | --- |
| page | `chat` | Chat 页 | ✓ |
| page | `playground` | Playground | ✓ |
| page | `logs` | 调用日志（`supportedScopes: ["own","all"]`：own=自己，all=租户管理员） | ✓（own） |
| page | `usage` | 用量统计 | ✓（own） |
| page | `tasks` | 异步任务 | ✓（own） |
| page | `tokens` | API Token 管理 | ✓ |
| page | `channels` | 渠道管理 | |
| page | `models` | 模型与路由管理 | |
| page | `monitoring` | 健康与监控 | |
| action | `token.create` | 创建 API Token | ✓ |
| action | `task.submit` | 提交异步任务 | ✓ |
| action | `channel.manage` | 增删改启停渠道、改密钥（`dangerous`） | |
| action | `channel.test` | 测试渠道 / 同步模型 | |
| action | `route.manage` | 配置模型路由优先级 / 权重 | |

roleTemplates：`llm-gateway-user`（chat / playground / 个人日志 / 建 token / 提交任务）、`llm-gateway-operator`（叠加渠道 / 模型 / 路由 / 监控）、`llm-gateway-admin`（`llm-gateway:*`）。

> `member` 基线（`defaultForMember`）由 `reconcileTenantRoles` 在 `db:seed` 时对账；运行时安装不自动补种，管理员可在角色矩阵手动调整。

---

## 8. 前端控制台

### 8.1 路由

| 路由 | 角色 | 说明 |
| --- | --- | --- |
| `/` | user | Gateway 概览。 |
| `/chat` | user | Chat 页面。 |
| `/playground` | user | 多协议在线测试。 |
| `/logs` | user/admin | 调用日志。 |
| `/usage` | user/admin | 用量统计。 |
| `/tasks` | user/admin | 异步任务。 |
| `/tokens` | user | API Token 管理。 |
| `/admin/channels` | admin | 渠道管理。 |
| `/admin/channels/new` | admin | 新增渠道。 |
| `/admin/channels/:id` | admin | 渠道详情、密钥、模型、健康检查。 |
| `/admin/models` | admin | 模型管理。 |
| `/admin/models/:model/routes` | admin | 路由优先级和权重配置。 |
| `/admin/monitoring` | admin | 健康与监控。 |
| `/settings` | admin | 租户级 Gateway 设置。 |

### 8.2 页面能力

**渠道管理**

- 新增 / 修改 / 启停 / 删除渠道。
- 选择 provider，动态显示 provider 所需字段。
- 测试连接，展示延迟、错误摘要、可用模型。
- 同步模型清单并选择哪些模型暴露给租户。
- 配置渠道分组、优先级、权重、限流和超时。

**模型管理**

- 展示对外模型名、能力、状态、绑定渠道数。
- 支持模型 alias：例如 `gpt-4o` 可路由到 OpenAI / Azure / OpenRouter。
- 拖拽或表单调整路由优先级和权重。
- 标记默认分组，如 `default`、`low-cost`、`high-quality`、`domestic`。

**日志查看**

- 筛选：时间、用户、Token、模型、渠道、provider、成功/失败、错误码、流式、任务类型。
- 关键列：模型、渠道、token、成功状态、错误原因、总耗时、首包时间、token 消耗、成本估算。
- 普通用户只能看自己的日志；租户管理员可看整租户。

**Playground**

- 支持 Chat、Image、Audio STT、Audio TTS、Embedding、Rerank、Async Task。
- 可选择协议视角：OpenAI / Claude / Gemini / Gateway。
- 展示原始请求、响应、耗时、路由到的渠道。

**Chat**

- 面向普通用户的简洁对话页。
- 支持模型选择、流式输出、图片输入（模型支持时）、历史会话本地保存。

**监控面板**

- 当前可用渠道数 / 异常渠道数。
- 近 24 小时请求量、成功率、P95 延迟、平均首包时间。
- Top 模型、Top 用户、Top 错误。
- Portal Dashboard card 使用同一份摘要 API。

---

## 9. 异步媒体任务

### 9.1 统一任务生命周期

```
submitted -> running -> succeeded
                    -> failed
                    -> canceled
                    -> expired
```

### 9.2 任务 API

提交：

```http
POST /api/v1/tasks
```

```json
{
  "taskType": "video",
  "model": "kling-v1",
  "input": {
    "prompt": "A cinematic city skyline at sunset",
    "duration": 5
  }
}
```

查询：

```http
GET /api/v1/tasks/:id
```

返回：

```json
{
  "id": "task_...",
  "status": "succeeded",
  "provider": "kling",
  "model": "kling-v1",
  "result": {
    "assets": [
      { "type": "video", "url": "https://..." }
    ]
  }
}
```

### 9.3 轮询与回调

- 上游支持 callback 时，Gateway 提供 callback endpoint 并校验签名。
- 上游只支持查询时，后台 worker 按 provider 策略轮询。
- 任务结果可保存 URL 和元数据；大文件不落 Gateway 数据库。
- 任务到期后标记 `expired`，按租户 retention 策略清理。

---

## 10. 日志、用量与成本估算

### 10.1 请求日志采集点

每次请求记录：

- `requestId`
- tenant / user / token
- protocol / capability
- requested model / routed provider model
- provider / channel
- stream 标记
- success / error
- latencyMs
- firstTokenMs
- inputTokens / outputTokens / totalTokens
- estimatedCost
- retry attempts

### 10.2 Token 统计

优先级：

1. 使用上游返回的 usage。
2. Adapter 根据 provider 响应字段映射 usage。
3. 上游不返回时，使用 tokenizer 估算。
4. 无法估算时记录 `usageEstimated=false`。

### 10.3 Payload 保存策略

默认只保存：

- 请求摘要：消息数量、附件类型、输入长度。
- 响应摘要：输出长度、finish reason。
- 错误摘要。

租户管理员可开启短期调试采样：

- 采样率可配置。
- 自动脱敏 Authorization / API Key / Cookie。
- 默认 7 天过期。

---

## 11. Realtime WebSocket

### 11.1 入口

```http
GET /v1/realtime?model=...
Authorization: Bearer xgw_...
```

### 11.2 处理流程

1. 校验 API Token 是否允许 realtime。
2. 根据 model / group 路由到支持 realtime 的 channel。
3. Gateway 建立上游 WebSocket。
4. 双向转发事件，同时注入 requestId 和日志上下文。
5. 连接关闭后写入调用日志：连接时长、错误、事件数量。

### 11.3 限制

- Realtime 自动重连由客户端负责；Gateway 不在中途切换上游连接。
- Realtime 渠道失败进入健康状态统计，但不复用普通 chat 的重试逻辑。

---

## 12. 测试计划

### 12.1 单元测试

- Provider adapter 请求转换。
- OpenAI / Claude / Gemini 协议转换。
- 路由算法：优先级、权重、分组、状态、限流、熔断。
- API Token 哈希、权限、过期校验。
- usage 映射和成本估算。

### 12.2 集成测试

- 使用 mock provider server 覆盖 chat / stream / image / embedding / rerank。
- 渠道失败后自动切换。
- 上游 429 标记限流。
- 渠道测试和模型同步。
- 日志写入和 usage 聚合。
- 异步任务 submit / poll / callback。

### 12.3 前端验证

- 管理员完整流程：新增渠道 -> 测试 -> 同步模型 -> 配置路由 -> Playground 调通 -> 日志可见。
- 普通用户流程：创建 API Token -> Chat 使用 -> 查看个人日志。
- 权限验证：普通用户不能进入渠道管理和租户日志。
- 响应式验证：控制台主要页面在桌面和移动宽度不溢出。

---

## 13. 分阶段里程碑

### M1 · App 骨架与 Portal 接入

交付：

- 新增 `apps/llm-gateway-server` 和 `apps/llm-gateway-app` workspace；根 `package.json` 增列 `dev:all`，并加 `dev:llm-gateway` 单起、`db:llm-gateway:generate/migrate`、`db:seed:llm-gateway`、`bootstrap` 脚本（对照 qbank）。
- Portal marketplace seed 增加 LLM Gateway 清单：`type: micro` + `embedUrl`(:5180) + `allowedOrigins`。
- Portal `SERVICE_REGISTRY` 增加 `llm-gateway → VITE_LLM_GATEWAY_SERVER_BASE`；apps/web CSP `frame-src` 放行 App 源。
- 新增 scope：`llm_gateway.read/write/admin`（`packages/shared/src/scopes.ts`）。
- 在 `apps/api/src/modules/acl/manifests.ts` 注册 `LLM_GATEWAY_MANIFEST`（§7.4）并进 `MANIFEST_REGISTRY`、seed 盖章到清单。
- 平台控制台建一个带 `token.introspect` 能力的服务账号供网关自省（§7.1）。
- Gateway server 完成 TDT introspection（服务账号 Basic + 四道闸：aud / scope / role / ACL）和网关自有 API Token 基础结构；CORS 白名单含 `PORTAL_BASE_URL`（宿主代理源）。
- 健康检查 `/healthz`、`/readyz` 可用。

验收：

- Portal 可打开 LLM Gateway App。
- 微应用可通过 `sdk.callService("llm-gateway", "/api/v1/bootstrap")` 拿到当前用户和租户信息。
- 非授权用户无法访问管理 API；`aud` 非 `llm-gateway` 的 TDT 被拒（401）。

### M2 · OpenAI 兼容调用与基础路由

交付：

- OpenAI compatible `/v1/models`、`/v1/chat/completions`、stream。
- Provider adapter：OpenAI、Azure OpenAI、OpenRouter、DeepSeek、Ollama。
- 渠道、模型、模型路由数据表和管理 API。
- 路由器支持模型、分组、优先级、权重、启停状态。
- 调用日志和 attempts 记录。

验收：

- 使用 OpenAI SDK 指向 Gateway base URL 可完成 chat 调用。
- 同一模型配置两个渠道时可按权重分流。
- 主渠道失败时切换备用渠道。
- 日志能看到模型、渠道、耗时、成功状态。

### M3 · 管理控制台

交付：

- 渠道管理页面。
- 模型管理和路由配置页面。
- API Token 管理页面。
- 日志查看页面。
- 用量概览页面。
- Playground chat 测试。

验收：

- 管理员不用改数据库即可新增渠道、测试渠道、同步模型、配置路由。
- 普通用户能创建 API Token 并使用 Playground。
- 权限边界符合角色要求。

### M4 · 多协议与多能力

交付：

- Claude compatible `/anthropic/v1/messages`。
- Gemini compatible generateContent / streamGenerateContent / embedContent。
- 能力扩展：image、audio transcription、audio speech、embedding、rerank。
- Provider adapter 扩展：Claude、Gemini、通义、百度文心、腾讯混元、智谱、火山、SiliconFlow、Xinference。

验收：

- Claude SDK / Gemini SDK 的核心调用可通过 Gateway。
- Playground 可测试 Chat / Image / Audio / Embedding / Rerank。
- 不支持某能力的渠道不会被路由选中。

### M5 · 容错、限流、健康与监控

交付：

- 渠道健康检查。
- 429 限流识别。
- 熔断和半开探测。
- 租户 / token / channel 限流。
- `/metrics` 和 Portal Dashboard card。
- usage daily 聚合任务。

验收：

- 异常渠道会自动降级并在监控页显示。
- 限流中的渠道不会继续被路由命中。
- Dashboard 展示近 24 小时请求量、成功率、P95 延迟和异常渠道数。

### M6 · 异步图片 / 音乐 / 视频任务

交付：

- `gateway_tasks` / `gateway_task_events`。
- 任务提交、查询、取消 API。
- Adapter：Midjourney、Suno、Sora、Kling、Jimeng、Gemini/Veo。
- 任务页面。
- callback 和 polling worker。

验收：

- 可提交图片 / 音乐 / 视频任务并查询结果。
- 任务失败有错误原因。
- 任务状态变更有事件轨迹。

### M7 · Realtime WebSocket

交付：

- `/v1/realtime` WebSocket 入口。
- Realtime 路由和上游桥接。
- Realtime 连接日志。
- API Token realtime 权限控制。

验收：

- 支持 realtime 的模型可建立双向连接。
- 禁止 realtime 的 Token 无法连接。
- 连接关闭后可在日志中看到连接耗时和错误状态。

---

## 14. 风险与处理

| 风险 | 处理 |
| --- | --- |
| 各厂商协议差异大 | 内部标准模型只覆盖共同核心；provider 特有参数放 `providerOptions`，不污染通用接口。 |
| 流式重试容易产生重复输出 | 流开始前可重试；流开始后默认不自动重放，只记录 partial failure。 |
| Token 统计不一致 | 优先使用上游 usage；无 usage 时显式标记估算，不伪装精确。 |
| 国内外供应商错误码不统一 | Adapter 统一映射为 Gateway 错误码，同时保留 upstream error summary。 |
| 密钥泄露风险 | 加密存储、日志脱敏、前端不回显、审计密钥更新。 |
| 异步任务结果文件大 | Gateway 只保存 URL 和元数据，不代理长期大文件存储。 |
| Realtime 连接占用资源 | 单 token / 单租户连接数限制，超时断开，监控活跃连接数。 |

---

## 15. 首期最小可用闭环

如果需要先压缩为一个最小可上线版本，建议只做：

1. 独立 Gateway App 接入 Portal。
2. API Token。
3. 渠道管理：OpenAI / Azure OpenAI / OpenRouter / DeepSeek / Ollama。
4. 模型路由：优先级 + 权重 + 启停。
5. OpenAI compatible `/v1/models`、`/v1/chat/completions`、stream。
6. 调用日志：模型、渠道、token、成功、错误、耗时、首包时间、token。
7. Playground Chat。
8. 健康检查和 Portal Dashboard card。

该闭环可以验证 Gateway 的核心价值：用户用一套 OpenAI SDK 接入，管理员可在 Portal 内配置多个上游渠道，并在失败时自动切换和查看日志。
