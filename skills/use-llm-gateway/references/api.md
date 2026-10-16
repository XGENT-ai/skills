# 统一调用 API

## 基址、凭证与模型

`GATEWAY_BASE` 是去掉尾斜杠的网关服务根地址，不含 `/v1`，例如 `https://<portal-host>/svc/llm-gateway`。`PORTAL_BASE` 用于门户签票；只提供门户 API 的内部地址不一定提供 `/svc` 代理。示例变量名不要求替换调用方已有配置。

App 服务账号须绑定真实调用 App，具备目标租户资格和实际授权；调用 App 与网关在目标租户正常安装。按需要在本 App 的 `serviceScopes` 申请或复用平台授予的 `llm_gateway.read` / `llm_gateway.write`，不修改门户数据库自授权。

签票是门户接口，使用服务账号 HTTP Basic：

```http
POST <PORTAL_BASE>/api/tokens/service
Authorization: Basic <clientId 与 secret>
Content-Type: application/json

{"grant_type":"client_credentials","tenant_id":"<tenant-id>","scope":"llm_gateway.read llm_gateway.write"}
```

检查 HTTP 状态及门户信封 `ok`，从 `data.access_token` 取票，按 `data.expires_in` 续期，并核对实际返回 scope。按环境、租户、SA 与 scope 集合缓存；不打印密钥或完整 token。门户翻译的专用内部身份不是外部 App 可复用的公共账号。

| 模型发现 | 凭证与响应 | 选型方式 |
| --- | --- | --- |
| `GET /api/v1/models?kind=<kind>&routable=true` | 用户或绑定 App 的服务 TDT + read；`{ok,data:Model[]}` | `model` 是调用值，`displayName` 是展示值，按 `availableKinds` 选能力 |
| `GET /v1/models` | API Key 或绑定 App 的服务令牌；原生 OpenAI list | `data[].id` 是模型名；不是带完整 availableKinds 的管理目录 |

API Key 不能因能调用 `/v1/models` 就被当作管理目录 TDT。目录 kind 为 `chat,embedding,image,tts,stt,rerank,video,ocr,moderation`；TTS 对应 capability=text_to_speech，生成任务 kind=speech。静态可路由不保证预算、参数和上游此刻可用，不硬编码示例供应商模型。

## 请求头

| 头 | 约束 |
| --- | --- |
| `Authorization` | `Bearer <API Key 或绑定 App 的服务令牌>` |
| `X-Attribution-Ref` | 可选，1–128 个 `0x21–0x7e` 可见 ASCII 字符，不含空格；无键和流式也可带。写日志/台账，不发上游、不授权、不路由 |
| `X-Actor-User-Id` | 可选，同样1–128个非空格可见 ASCII；只作审计，不授权、不发上游、不参与摘要 |
| `Idempotency-Key` | 可选，1–200个非空格可见 ASCII；仅非流式调用 POST，不能与 stream:true 同用 |
| `X-Gateway-Failover` | default / refused-only，默认 default；显式发送必须带键，与该意图绑定 |
| `Prefer` | 媒体交付选 respond-async 或 wait=正整数秒；不能重复/同时指定，已知格式错误为400 |

没有值时省略可选头；头值不作 trim。不要把中文正文、敏感信息或带空格描述直接当标记。网关不提供按标记的额度和业务汇总，由调用方维护。

default 可在失败后重试/换候选，可能产生多次供应商费用。refused-only 只在明确工作前拒绝时换候选；超时、断连、5xx或开始工作后的拒绝停止。它不禁止推理的业务修复或审核的多条目调用，不承诺整单只请求一次供应商。

## 响应类调用

下列成功体均为原生响应，不套 `{ok,data}`；合法 Prefer 不把它们变为任务。

| POST 路径 | 请求关键字段 | 成功解析 |
| --- | --- | --- |
| `/v1/chat/completions` | model,messages，可选stream | OpenAI completion / SSE；文本取 choices[0].message.content |
| `/v1/responses` | OpenAI Responses 请求 | Responses对象/SSE，不按chat解析 |
| `/v1/embeddings` | model,input | embeddings JSON |
| `/v1/rerank` | model,query,documents，可选top_n | 重排JSON |
| `/v1/audio/transcriptions` | multipart model,file；可选language,response_format,temperature | 默认转写JSON，显式格式按相应协议解析 |
| `/v1/ocr` | multipart model,file图片；format=text或markdown | `{text,format,model}`，没有任务/资产；不把PDF直接当图片上传 |
| `/v1/inference/structured` | instruction,jsonSchema；可选locale,maxOutputTokens,inputs | 结果取output；模型由网关配置选择 |
| `/v1/moderation/check` | 可选purpose，必需items | `{results:[...]}`；调用方不能指定model |
| `/v1/audio/voices` | multipart request JSON `{model,name?,language?}` + sample音频 | 音色对象voiceId/status，克隆不是生成任务；HTTP200可能携带status=failed |

结构化推理保留 camelCase 字段，不改成 json_schema/max_output_tokens。纯文本例：

```json
{"instruction":"返回一个标题","jsonSchema":{"type":"object","properties":{"title":{"type":"string"}},"required":["title"],"additionalProperties":false},"locale":"zh-CN","maxOutputTokens":256}
```

结构化推理成功体的usage保留业务TokenUsage（如totalTokens）；`/v1/calls`里的usage是台账归一投影。按各自DTO解析，不把台账字段直接套在业务结果上。

带图片用 multipart 的 request JSON，声明 `inputs:[{"type":"image","partName":"p0"}]`，二进制部分名为p0。未声明、重复或缺失文件均被拒绝，不另加body幂等字段。

审核文本例：

```json
{"purpose":"generated","items":[{"id":"output-1","kind":"text","text":"待审核文本"}]}
```

带图改用 multipart 的 meta JSON：`{"purpose":"listing","items":[{"id":"cover-1","kind":"image","part":"p0"}]}`，另传p0图片字节，可混合文本项。purpose可选review/comment/listing/form/generated。审核与结构化推理不接受图片外链或JSON base64；文件类型和大小由目标部署校验。

审核按id读取decision/categories/reasonCode。只有allow是放行；review可能是人工复核、上游拒绝、不可用或输出不可解析；unsupported可能是没有视觉路由。整单或逐项失败均不能默认为allow。

音色资源另有 `GET /v1/audio/voices/{id}` 和 `DELETE /v1/audio/voices/{id}`，使用原主体。删除资源不删除克隆调用键或首次克隆响应；合成时把可用voiceId传到speech请求的voice字段，克隆音色固定到所属渠道。

## 图片、语音、视频

| POST 路径 | 请求 |
| --- | --- |
| `/v1/images/generations` | JSON model,prompt；可选n,size,aspect_ratio,seed,negative_prompt,response_format,group |
| `/v1/images/edits` | multipart model,prompt,image或重复image[]文件部分；其他图片字段按模型支持情况提供 |
| `/v1/audio/speech` | JSON model,input；可选voice,response_format,speed,group，不用旧speech.text/voiceId包装 |
| `/v1/videos/generations` | JSON model,prompt；可选duration秒、size,aspect_ratio,seed,negative_prompt,group，不是durationSec |

不向所有模型强塞seed、尺寸、负面提示词或格式；图生图传标准文件部分，不用旧references包装。

| 调用 | 不带Prefer | respond-async | wait=N |
| --- | --- | --- | --- |
| 图片 | 200 `{created,data:[{b64_json}]}`；response_format=url则data中为网关资源url | 202任务 | 限期内完成为200图片，否则202任务 |
| 语音 | 200音频字节 | 202任务 | 限期内完成为200音频，否则202任务 |
| 视频 | 202任务 | 202任务 | 完成为200任务对象，否则202任务；都不是视频字节 |

生成成功头带X-Gateway-Task-Id和X-Gateway-Request-Id；应用偏好后带Preference-Applied。默认同步等待预算耗尽为504 TASK_RUNNING，从error.details.taskId续查，见[恢复规则](recovery.md)。

API Key的allowTasks=false拒绝视频以及图片/语音的respond-async、wait=N；仍可同步图片/语音，查询其自己的任务、下载、ACK及取消，不能拦住同步超时后的恢复。allowRealtime另约束实时协议。

## 请求示例

GATEWAY_TOKEN从安全配置加载。模型占位须换成当前租户目录值；示例键不能被多次独立执行业务复用，实际键先保存到业务attempt。

```sh
curl "$GATEWAY_BASE/v1/chat/completions" \
  -H "Authorization: Bearer $GATEWAY_TOKEN" \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: job-copy-attempt-1' \
  -H 'X-Attribution-Ref: job-copy' \
  -H 'X-Gateway-Failover: refused-only' \
  -d '{"model":"<text-model>","messages":[{"role":"user","content":"写一个标题"}]}'

# multipart边界交给客户端生成，不手写Content-Type。
curl "$GATEWAY_BASE/v1/ocr" \
  -H "Authorization: Bearer $GATEWAY_TOKEN" \
  -H 'Idempotency-Key: job-ocr-attempt-1' \
  -F 'model=<ocr-model>' -F 'format=markdown' -F 'file=@page.png'

curl -i "$GATEWAY_BASE/v1/videos/generations" \
  -H "Authorization: Bearer $GATEWAY_TOKEN" \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: job-video-attempt-1' \
  -H 'Prefer: respond-async' \
  -d '{"model":"<video-model>","prompt":"水面上的涟漪"}'
```

真实用户正文用调用方JSON序列化器生成请求，不做字符串拼接。OpenAI SDK适用于对应兼容端点；扩展推理、审核、OCR、台账及资产可复用HTTP客户端，不猜不存在的SDK方法。`/anthropic/*`、`/gemini/*`和realtime接受两种公共凭证，但保持原生协议形状，不支持Idempotency-Key，不套用 `/v1` 的重放语义。
