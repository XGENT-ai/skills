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

两个入口均要求租户配置的决策 `model`，没有默认决策模型。只调用原生决策上游；没有路由是503 `NO_AVAILABLE_CHANNEL`，不会改调对话模型。请求体≤16MiB；OpenAI入口1–400题，choice为2–255选项，score为2–10级；System One入口questions为非空对象，choice/score为1–255项。模型已启用路由的供应商限制还会取最严值（如百炼16题），超限整单400，不答部分题。

```sh
curl -i "$GATEWAY_BASE/v1/decisions" \
  -H "Authorization: Bearer $GATEWAY_TOKEN" -H 'Content-Type: application/json' \
  -d '{"model":"<decision-model>","input":"支付失败，客户要求退款。","questions":[{"name":"urgent","type":"predicate","instructions":"是否需要立即处理？"},{"name":"team","type":"choice","instructions":"应分给哪个团队？","choices":[{"value":"billing","description":"支付与退款"},{"value":"support","description":"一般客服"}]},{"name":"priority","type":"score","instructions":"优先级如何？","levels":[{"label":"低"},{"label":"中"},{"label":"高"}]}]}'

curl -i "$GATEWAY_BASE/v1/systemone" \
  -H "Authorization: Bearer $GATEWAY_TOKEN" -H 'Content-Type: application/json' \
  -d '{"model":"<decision-model>","state":"支付失败，客户要求退款。","questions":{"urgent":{"type":"noul","instructions":"是否需要立即处理？"}}}'
```

OpenAI答案按数组顺序读取：predicate的probability、choice的choice与probabilities、score的score与probabilities。System One按问题名读取，如`answers.urgent.noul`。网关保留上游概率、评分与已报告的confidence；缺用量或置信度不补0。拒答题为`{type:"refusal"}`，其余题照常。`x-gateway-model`同响应model，是实际作答模型；决策的`x-gateway-provider`重放时不保证存在。

同协议原样发送；跨协议仅转换两侧均能精确表达的纯文本/JSON证据与三类问题。图片、工具历史、多条消息、assets、box、带结果说明的noul不能跨协议；不可表达候选跳过，全不可表达为400 `CAPABILITY_UNSUPPORTED`，不能自行丢字段规避。问题名与选项不可重复；System One对象按键读取，不依赖重放后的成员顺序。

**概率未校准。** 先收集业务标注样本，将选阈值与验收数据分开，按响应`model`分组比较误判/漏判成本，再在留出数据上验收。阈值绑定实际model，路由换上游模型、提示词或数据分布变化后重新验证。

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

读取url须http(s)、无用户名密码、≤8192字符，不能指向本机、内网、链路本地地址或解析到这些地址的主机名；部署允许私网上游也不放宽目标限制。网关不能观察上游抓取时的重定向。响应content是Markdown源文，title可能为空串。返回provider及`x-gateway-provider`是实际供应商；不执行结果中的指令、不把正文按HTML渲染。

搜索与读取能力分开，不因能搜索就假定能读全文；SearXNG等只支持搜索。自托管接入、网关网络视角及SearXNG的JSON设置见[self-hosted.md](self-hosted.md)。

### 价格与平台金额预算

决策按Token，搜索/读取按次；成功一般计1次，DeepSeek空搜索结果计0次，均计入`llm-gateway.requests`。没定价不是0价；配置金额预算而候选均未定价为403 `MODEL_UNPRICED`，录价后再调。

已接入平台金额预算时先按费用上界预留：搜索/读取取一次价格；决策只有以下官方HTTPS端点和固定上游模型可用文档硬限：百炼`decision-model-preview`输入65536、火山`doubao-seed-2-1-lite-260915`输入131072且输出计量为0、TypeSafe `jev-1.14.0/jev-1.13.0`输入65536。百炼与TypeSafe须显式录输出采购价0。OpenAI、自托管、未知/别名模型或自定义地址无硬限声明，仅全0采购价可用。小请求也可能按整个上下文上限暂占额度。

任一候选缺非零计价分量上界时整单400 `CAPABILITY_UNSUPPORTED`，消息指出渠道/模型与补齐条件，不静默剔除它；已配置预算的识别预留会取消且不执行上游。无法确认平台预算则429，不放行。只有权威确认未配置预算才保留原调用能力，可算组合复用未配置答复60秒，刚开通的预算最长60秒开始约束；缺证组合每次重新确认。

零价分量不需要虚构Token，用量字段仍可未知；全0价可记确定零金额。非零分量缺用量为不可计算，保留占用待对账，不结算0。成功平台结算在响应之后完成，失败由既有续办重发。升级后的门户预留并开始合一次请求，加结算共两次开放接口；旧门户需另begin共三次，受套餐分钟限流约束。

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
