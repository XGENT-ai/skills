# 幂等、任务恢复与旧调用迁移

## 意图键与丢回包

键空间为 `(租户,调用主体,键)`，响应类和任务类共用。API Key按每把Key隔离；服务令牌按App的azp隔离，同App续签不创建新主体。换Key或换App不是恢复原调用的方法。

同键比较包含业务请求、真实kind、标记与失败切换策略，文件按字节摘要比较。actor不参与摘要；Prefer只改变交付方式。因此可以改变等待偏好，不能用同键把chat改视频、换提示词或改标记。

一个键至多认领一次业务执行，失败键不能再认领。重放返回原结果/错误并带 `Idempotent-Replayed:true`。取消、删音色、ACK、正文过期都不释放键；占用永久保留。

| 操作 | 协议 |
| --- | --- |
| 回读 | `GET /v1/calls?key=<URL编码的键>`，200原生调用视图；未记录404 NOT_FOUND |
| 封存 | `POST /v1/calls/seal`，JSON `{"key":"<原意图键>"}`；未用键生成永久sealed墓碑，已用键原样回读；不取消执行、不删除结果 |

视图有face=response/task、kind/state、requestId/taskId、charge/usage/error/proofRef与result。响应结果为 `result:{kind:"response",body:...}`；任务结果为 `{kind:"task",taskId,status,assets}`。回读HTTP200也可以表示原业务失败，error不等于新的顶层传输失败。正文失效后result可为null，键仍占用。

丢回包先回读原键。404只代表此刻未记录，不是确定未执行，原请求可能仍在路上。仍需完成原意图时，在授权范围内用原键、原请求重传；决定放弃时封存原键，再按返回事实结束或对账，防止晚到提交执行。不能用新键绕过冲突或不确定结果。

## 计费证据与重试

`charge={state,final,reason,ambiguousAttempts}` 是网关掌握的工作证据，不替代供应商账单。

- charged：已有结果或异步受理；任务后来失败/取消也可能收费。
- not_charged：明确工作前拒绝、队列内从未派发等；只有 `charge.state=not_charged && charge.final=true` 才是可释放预留的免费终局证据。
- unknown：证据不确定或仍有执行者。即使final=true也不是免费；不自动释放预算或再生成，转回读/对账或明确的业务决定。

错误details.chargeState不含完整final证据。资源GET/ACK的错误只说明本次资源操作，不能推导原生成免费。取消不是退款，也不承诺上游已停止；ambiguousAttempts是可能另行计费的尝试数。

| HTTP/code | 处置 |
| --- | --- |
| 400校验/能力/文件错误 | 修正请求或模型；若键已认领，原键仍重放错误，明确重做才建立新attempt |
| 401/403 | 查Key或服务票的状态、归属、安装、scope，不换用户票/更宽账号绕过 |
| 404 | 查基址、版本、模型、租户和主体；不据此认定免费重生成 |
| 409 IDEMPOTENCY_CONFLICT/IDEMPOTENCY_KEY_SEALED | 核对原请求或封存决定，停止以此键提交不同意图 |
| 409 TASK_RUNNING | 响应类带键调用执行中，回读同键 |
| 409 CONFLICT | 如终态取消/取消后同步重放；读原任务，不转成新生成授权 |
| 410 DELIVERY_GONE | 字节/正文已ACK或过期，读调用方归档；不能重置原键 |
| 429预算/限流/队列满 | 根据budgetWindow、Retry-After及观察上限等待或停止，不无界重试、不自动换键 |
| 502/503或断连 | 结果可能未知，优先同键回读/对账 |
| 504 TASK_RUNNING | 保存details.taskId，查原任务，不再POST生成 |

`/v1` 顶层错误用真实HTTP状态与 `{error:{message,type,code,details}}`。charged/unknown错误带 `x-should-retry:false`；缺该头也不是自动重试或免费证明。关闭或接管SDK的自动写重试，按项目安装版本配置。流式首帧后中断可能用协议错误帧表达，不能自动重生成整段；流式与Anthropic/Gemini/realtime不支持按键重放。

## 媒体任务

保存成功头X-Gateway-Task-Id、202体的id，或504错误details.taskId。任务对象为 `object:"gateway.task"`，字段id/kind/model/status/created/data/usage/charge；不是旧taskId/assets管理DTO。

| 方法与路径 | 行为 |
| --- | --- |
| `GET /v1/tasks/{id}` | 归属检查后刷新原任务 |
| `POST /v1/tasks/{id}/cancel` | 返回取消后任务；终态409，取消可能已收费 |
| `GET /v1/tasks/{id}/assets/{assetKey}/content` | 网关字节、Content-Type、X-Asset-Sha256及no-store |
| `POST /v1/tasks/{id}/assets/{assetKey}/ack` | `{task_id,asset_key,acked:true}`；删除交付字节，重复ACK幂等 |

queued/submitted/running是执行中，succeeded后消费data[]中的asset_key/mime/size_bytes/sha256/width/height/duration_ms/expires_at/url。asset_key取响应，不固定为0或数组下标；路径段和查询键分别按URL规则编码。

failed/canceled/expired通常停止自动等待。UPSTREAM_TASK_TIMEOUT是自动轮询预算耗尽的特殊情形，服务支持按原任务重新查询：保留taskId及显式续查入口，不改为生成，不承诺刷新一定成功。客户端观察期限到达也应保存可恢复关联并停止本轮等待，不无限轮询。

`data[].url` 是相对网关服务根目录的私有路径。实际地址用去尾斜杠的GATEWAY_BASE加返回的 `/v1/...`；用 `new URL('/v1/...', portalOrigin)` 会丢掉 `/svc/llm-gateway`。使用原主体获取字节，通过本App授权接口/归档文件供前端展示，不当作公开CDN，也不把服务Bearer交给供应商地址。

缓冲默认图片/语音24小时、视频7天且可由租户配置，实际看expires_at，不代替永久存储。逐资产读取、校验大小/摘要、持久化及提交业务关联后再ACK；中断后恢复未完成资产，重复消费不重复创建业务产物。ACK/过期/资产不存在为410，跨主体先404。停止等待不等于取消。

## 旧调用迁移

统一版本删除旧调用，无兼容别名。确认部署版本和共同切换安排后，同时迁路径、凭证、字段、成功/错误解析、持久化与恢复逻辑；不因404自动试旧接口。

| 原调用 | 统一后的调用 |
| --- | --- |
| `/api/v1/media/generations` image generate | `/v1/images/generations`；count/aspectRatio/negativePrompt改n/aspect_ratio/negative_prompt |
| 同上image edit，request/references | `/v1/images/edits`，标准multipart及image/image[]文件 |
| 同上speech，speech.text/voiceId/params.format | `/v1/audio/speech`，input/voice/response_format |
| 同上video，params.duration | `/v1/videos/generations`，duration为秒，统一任务 |
| 同上ocr | `/v1/ocr`，multipart model/file/format，直接JSON，无任务资产 |
| `/api/v1/inference/structured` | `/v1/inference/structured`，键移到头，失败键不重跑，用户TDT不可用 |
| `/api/v1/moderation/check` | `/v1/moderation/check`，items/meta multipart保持，不传model |
| `/api/v1/media/tasks/{id}`及子资源 | `/v1/tasks/{id}`及cancel/content/ack |
| `/v1/videos/generations/{id}`及content | `/v1/tasks/{id}`及 `/assets/{assetKey}/content` |
| `/api/v1/media/voices` | `/v1/audio/voices`，request/sample multipart，键可选且与其他调用共用 |
| `/api/v1/service-calls`及seal | `/v1/calls?key=`、`/v1/calls/seal`，原生视图，未记录404 |
| `/api/v1/media/models`、`/api/v1/inference/models` | `GET /api/v1/models`，speech目录类别为tts，仍读管理信封 |
| `POST /api/v1/tasks` | 删除，用各公开生成端点创建 |
| body sourceRef/attributionRef/failover/actorUserId | Idempotency-Key/X-Attribution-Ref/X-Gateway-Failover/X-Actor-User-Id头 |
| 调用信封 `{ok,data}` / `{ok:false,error}` | 原生成功体、真实HTTP错误；保留的管理目录不变 |

推理重放从结果体replayed:true改为Idempotent-Replayed头；台账face和result.kind的chat/media改response/task。旧params.strength不再支持，不能猜一个新字段替代。
