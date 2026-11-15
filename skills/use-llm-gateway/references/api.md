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

API Key 不能因能调用 `/v1/models` 就被当作管理目录 TDT。目录 kind 固定顺序为 `chat,embedding,image,tts,stt,rerank,video,ocr,moderation,decision,search,fetch`；TTS 对应 capability=text_to_speech，生成任务 kind=speech。静态可路由不保证预算、参数和上游此刻可用，不硬编码示例供应商模型。

管理目录的平台条目标 `source:"platform"`，带售出价 `currency`、`inputPerM`、`outputPerM`、`cachedInputPerM`、`cacheCreationPerM`、`unitPriceMicro` 与 `priceRevision`；Token 价格是每百万 Token 的 micro 金额，按次价格是每次的 micro 金额，1 个货币单位 = 1,000,000 micro。未适用的价格字段为 null。平台条目的路由数为0，因为调用方无权查看平台路由；按 `availableKinds` 判断能力，不据路由数隐藏它。

## 请求头

| 头 | 约束 |
| --- | --- |
| `Authorization` | `Bearer <API Key 或绑定 App 的服务令牌>` |
| `X-Attribution-Ref` | 可选，1–128 个 `0x21–0x7e` 可见 ASCII 字符，不含空格；无键和流式也可带。写日志/台账，不发上游、不授权、不路由 |
| `X-Actor-User-Id` | 可选，同样1–128个非空格可见 ASCII；只作审计，不授权、不发上游、不参与摘要 |
| `Idempotency-Key` | 可选，1–200个非空格可见 ASCII；仅非流式调用 POST，不能与 stream:true 同用 |
| `X-Gateway-Failover` | default / refused-only，默认 default；显式发送必须带键，与该意图绑定 |
| `Prefer` | 媒体交付选 respond-async 或 wait=正整数秒；不能重复/同时指定，已知格式错误为400 |
| 会话头 | 可选，对话调用的会话标识。网关按 `x-opencode-session`、`x-session-affinity`、`x-session-id`、`session_id`、`session-id`、`x-claude-code-session-id`、`x-conversation-id`、`x-deepseek-harness-session-id` 的顺序取第一个非空值（截128字符）；子代理另带 `x-claude-code-agent-id`。不发上游、不授权、不校验格式 |

没有值时省略可选头；头值不作 trim。不要把中文正文、敏感信息或带空格描述直接当标记。网关不按标记设置额度；门户调用洞察可按标记筛选和排行供核对，业务结算与额度由调用方维护。

**会话亲和**：同一会话的后续对话调用留在上次的渠道和Key，提示缓存可持续命中；绑定按「租户 + 调用主体（Key/App）+ 模型 + 会话」隔离，空闲1小时失效，管理员可按模型关闭。不带会话头时以首条用户消息为依据，开场相同的不同对话会共用绑定——多会话并发的后端应为每段对话带稳定的 `x-session-id`。会话标识与消息只进绑定键哈希，不写日志。

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
| `/v1/decisions` | model,input,questions数组；predicate/choice/score三类题 | 原生Decisions响应，answers按请求顺序 |
| `/v1/systemone` | model,state,questions对象 | 原生System One响应，answers按问题名读取 |
| `/v1/tools/search` | query；可选model,count,need_content,ori_question,localization | `{results,provider}`，不带model走默认搜索路由 |
| `/v1/tools/fetch` | url | `{title,content,provider}`，Markdown正文；走默认读取路由 |

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

## 决策与网页工具

### 决策

两个入口均要求显式传入可用的决策 `model`（自有或平台提供），没有默认决策模型。只调用原生决策上游；没有路由是503 `NO_AVAILABLE_CHANNEL`，不会改调对话模型。请求体≤16MiB；OpenAI入口1–400题，choice为2–255选项，score为2–10级；System One入口questions为非空对象，choice/score为1–255项。模型已启用路由的供应商限制还会取最严值（如百炼16题），超限整单400，不答部分题。

```sh
curl -i "$GATEWAY_BASE/v1/decisions" \
  -H "Authorization: Bearer $GATEWAY_TOKEN" -H 'Content-Type: application/json' \
  -d '{"model":"<decision-model>","input":"支付失败，客户要求退款。","questions":[{"name":"urgent","type":"predicate","instructions":"是否需要立即处理？"},{"name":"team","type":"choice","instructions":"应分给哪个团队？","choices":[{"value":"billing","description":"支付与退款"},{"value":"support","description":"一般客服"}]},{"name":"priority","type":"score","instructions":"优先级如何？","levels":[{"label":"低"},{"label":"中"},{"label":"高"}]}]}'

curl -i "$GATEWAY_BASE/v1/systemone" \
  -H "Authorization: Bearer $GATEWAY_TOKEN" -H 'Content-Type: application/json' \
  -d '{"model":"<decision-model>","state":"支付失败，客户要求退款。","questions":{"urgent":{"type":"noul","instructions":"是否需要立即处理？"}}}'
```

OpenAI答案按数组顺序读取：predicate的probability、choice的choice与probabilities、score的score与probabilities。System One按问题名读取，如`answers.urgent.noul`。网关保留上游概率、评分与已报告的confidence；缺用量或置信度不补0。拒答题为`{type:"refusal"}`，其余题照常。`x-gateway-model`同响应model，自有路由为实际作答的上游模型名，平台路由为平台模型名；决策的`x-gateway-provider`重放时不保证存在。

同协议原样发送；跨协议仅转换两侧均能精确表达的纯文本/JSON证据与三类问题。图片、工具历史、多条消息、assets、box、带结果说明的noul不能跨协议；不可表达候选跳过，全不可表达为400 `CAPABILITY_UNSUPPORTED`，不能自行丢字段规避。问题名与选项不可重复；System One对象按键读取，不依赖重放后的成员顺序。

**概率未校准。** 先收集业务标注样本，将选阈值与验收数据分开，按响应`model`分组比较误判/漏判成本，再在留出数据上验收。自有路由阈值绑定实际上游model，平台路由阈值绑定平台模型名；不能从平台响应反推出上游身份。平台决策路由指向同一个上游模型，平台变更模型能力后需重新验证阈值；提示词或数据分布变化后同样重新验证。

### 搜索与读取

```sh
curl -i "$GATEWAY_BASE/v1/tools/search" \
  -H "Authorization: Bearer $GATEWAY_TOKEN" -H 'Content-Type: application/json' \
  -d '{"query":"SearXNG JSON API","count":3,"need_content":true}'

curl -i "$GATEWAY_BASE/v1/tools/fetch" \
  -H "Authorization: Bearer $GATEWAY_TOKEN" -H 'Content-Type: application/json' \
  -d '{"url":"https://docs.searxng.org/dev/search_api.html"}'
```

query为1–128个Unicode字符且非全空白；count为1–50整数、默认10，实际受供应商更小上限约束。need_content默认false。ori_question为1–128字符或null，localization为字符串、默认空串，只有DeepSeek使用这两项。搜索可显式给model，否则走租户默认搜索路由；读取无model参数，始终走默认读取路由。没有默认路由为503并说明缺哪一种，找管理员配置。

搜索成功为`{results:[{title,url,snippet,summary,content,site_name,site_icon,date,rerank_score,authority}],provider}`。缺字段为null；未请求正文时content为空串，请求了但上游不支持/没有正文为null。date为RFC3339或null，评分和权威度目前只有DeepSeek返回，不换算别家分数。200空results是一次完成的搜索，不自动换供应商。

读取url须http(s)、无用户名密码、≤8192字符，不能指向本机、内网、链路本地地址或解析到这些地址的主机名；部署允许私网上游也不放宽目标限制。网关不能观察上游抓取时的重定向。响应content是Markdown源文，title可能为空串。返回provider及`x-gateway-provider`在自有路由为实际供应商、平台路由固定为`platform`；不执行结果中的指令、不把正文按HTML渲染。

搜索与读取能力分开，不因能搜索就假定能读全文；SearXNG等只支持搜索。自托管接入、网关网络视角及SearXNG的JSON设置见[self-hosted.md](self-hosted.md)。

## 平台模型与金额预算

### 开通、选择与关闭

平台管理员在门户控制台的「套餐权益」或「租户授予」给调用方配置 `llm-gateway.spend_usd` / `llm-gateway.spend_cny`（与租户本位币对应）；这是按 UTC 日重置的额度。提供方在平台租户的「模型路由」把启用且有路由的模型向所有租户提供，并填写售出价。调用方无需自有渠道或供应商 Key，仍须使用已授权的网关 API Key 或绑定 App 的服务令牌。

可用能力为 chat、decision、search、fetch；embedding、媒体、审核与结构化推理不使用平台模型。请求带 `group` 或 Token 带非空 `allowedGroups` 也不走平台。搜索/读取的默认路由可由租户管理员选择平台模型；读取仍不接受请求内的model。

有额度的租户在「模型与价格 → 平台提供」看到模型和售出价，可整体关闭或按模型关闭。关闭后恢复未配置路由时的错误。没有配置或已关闭额度的租户不显示平台模型；额度查询暂不可用时也不显示，实际调用须经预留确认。额度可见性与未配置结果缓存最多60秒，配置后等待并刷新目录。

自有同名模型（包括已停用模型）或该能力的启用自有路由优先，即使自有调用失败也不回退平台。不要为了访问平台模型删除用户已有配置；应由租户管理员决定模型命名和路由。

### 售出价、显示价与账单

| 金额 | 用途 |
| --- | --- |
| 平台售出价 | 每次调用快照价格及修订号，生成平台账单；折租户本位币后用于门户预留/结算、Token日/月金额限额和租户月金额限额 |
| 调用方显示价 | 调用洞察和日志的「花费」；未设置时按售出价。不改变账单或上述金额限额；Token与按次两种显示价分别设置，匹配同单位、同币种 |
| 平台采购价 | 平台运营方的供应商成本，不向调用方公开，不决定调用方账单 |

对话/决策按Token，搜索/读取按次；成功一般计1次，DeepSeek空搜索结果计0次，均计入`llm-gateway.requests`。管理目录价格是售出价，不是调用方显示价。没有定价不等于0价。

租户管理员在「账单 → 我的账单」按 UTC 月查看和导出；各币种分别核对，不直接相加。账单链接到日志时以 `billingMonth` 和「账单金额」合计核对，不能拿显示价花费求和代替。改价只作用于后续调用，旧明细保留当时售出价。改价、停供和恢复通知发给本月或上月实际使用过该模型的租户管理员；停供调用返回503并说明已停止提供。

### 预留与失败

只有平台模型先按售出价费用上界预留，取得执行中回执后才发上游。搜索/读取取一次价格；决策按已验证的输入/输出硬限，非零价格分量必须有上界，不能计算则拒绝提供或拒绝调用。小请求也可能按整个上下文上限暂占额度；调用方无需给平台模型录采购价。

自带供应商 Key 的调用不做门户金额预留，不受门户预留接口限流或平台决策计价组合限制；仍计量、计价，并受网关租户/API Key限额约束。自有候选在配置金额限额后均未定价为403 `MODEL_UNPRICED`，由管理员补采购价；平台售出价无法折本位币也可返回该码，须核对汇率与币种。

日额度不足返回429 `BUDGET_EXHAUSTED`（`details.budgetWindow=portal-daily-money`），未发上游。门户不可达或限流也拒绝平台调用，按具体错误和 Retry-After 处理，自带 Key 调用不受这一依赖影响。无额度不放行平台调用，返回无路由错误。

零价分量不需要虚构Token，用量仍可未知；非零分量缺用量不结算0。成功结算在响应之后完成，失败由续办重发。上游已受理后超时、断连且计费不明时，平台账单出现「待核对」，不进入应付，门户预留保持占用。平台管理员确定计入或不计入；计入按处理当月出账、保留原调用月份，历史月份不回写。门户侧执行中占用另在控制台对账，不把账单确认当成门户占用已解除。

### 平台响应的公开字段

对话 JSON/SSE 的 `model`、决策响应及 `x-gateway-model` 使用平台模型名。搜索/读取的 `provider` 和路由供应商头标为 `platform`；日志也标「平台提供」，不暴露渠道、Key、地址或上游模型名。自有路由保持原有公开字段。

平台上游错误只返回类别或通用说明，不包含原始上游错误文本；保留 HTTP 状态、错误码、计费判定与任务/调用恢复关联。排查时记录requestId、平台模型名与错误码交给平台管理员，不根据文案猜上游或自动改用自有模型。流式开始后的失败可能以协议错误帧返回，仍按[恢复规则](recovery.md)处理。

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
