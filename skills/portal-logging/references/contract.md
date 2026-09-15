# 契约：端点、凭据、响应、查询、计量

本文件是「写代码时要查的那几行」。判断与流程在 [SKILL.md](../SKILL.md)。

## 1. 地址

| 用途 | 地址 | 从哪来 |
| --- | --- | --- |
| 换服务令牌 | `{API_BASE_URL}/api/tokens/service` | 平台注入 `API_BASE_URL`（门户内部源） |
| 写入日志 | `{PORTAL_BASE_URL}/svc/observability/v1/ingest/<stream>` | 平台注入 `PORTAL_BASE_URL`（门户公开源） |
| 写入指标/链路（OTLP） | `{PORTAL_BASE_URL}/svc/observability/v1/ingest/otlp/{logs,metrics,traces}` | 同上 |
| 查询（人用） | 「日志与监控」App 界面 | — |
| 查询（程序，需用户身份） | `{PORTAL_BASE_URL}/svc/observability/api/t<租户UUID去掉横线>/_search?type=logs` | — |

`/svc/<key>/…` 是**所有部署形态都通**的那条路（门户反代按 key 转发）。同机直连对方容器是优化，不是默认——换个部署形态就断。

⚠️ OTLP 三条入口的协议细节与日志入口不同，接之前先跟服务方确认版本行为；本 skill 只保证 `/v1/ingest/<stream>` 这条 JSON 路径。

## 2. 换服务令牌

```bash
curl -sS -X POST "$API_BASE_URL/api/tokens/service" \
  -u "$MY_SA_CLIENT_ID:$MY_SA_CLIENT_SECRET" \
  -H 'content-type: application/json' \
  -d '{"grant_type":"client_credentials","tenant_id":"<租户 UUID>","scope":"observability.ingest"}'
```

响应是门户统一信封：

```json
{ "ok": true, "data": { "access_token": "…", "token_type": "Bearer", "expires_in": 900, "scope": "observability.ingest" } }
```

- **先看 `ok`，再看 `data`**；失败时是 `{"ok":false,"error":{"code":"…"}}`，HTTP 仍可能是 200（业务失败不用状态码表达）。
- **`scope` 为空 = 没拿到写入权**：往下写一定 403。回 SKILL.md §1.1。
- `scope` 省略时给的是这把钥匙**当前全部**权限；显式写 `observability.ingest` 更窄、更好排查。
- 缓存到 `expires_in - 10s`。别每条日志换一次。

## 3. 长期访问密钥（采集器用）

门户控制台 › 服务账号 › 访问密钥：选目标租户、**只勾 `observability.ingest`**、起个能认出用途的名字、按需设到期。明文**只显示一次**。

- 一个服务账号最多 10 把活跃密钥。
- 撤销/轮换后旧密钥立刻 401（采集器会按重试上限丢弃，重签后重启即可）。
- 落盘权限收紧到只有采集器进程能读；**别进仓库、别进镜像、别打进日志**。
- 密钥能签出来的 scope 不会超过它所属服务账号当下能拿到的——签发时报「无法授予」就是前提没成立，不是名字写错。

## 4. 写入

```bash
curl -sS -X POST "$PORTAL_BASE_URL/svc/observability/v1/ingest/console" \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '[{"message":"hello","level":"info","service":"my-service","host":"node-1"}]'
```

成功响应形如 `{"code":200,"status":[{"successful":1,"failed":0,…}]}`。**两层都要判**：HTTP 2xx 且 `code:200` 且 `failed:0`。

- 请求体是**记录数组**；一次一批，别一条一个请求。
- 字段是随写随建的：你发什么字段，流里就有什么列。所以字段名一旦定下就别改来改去（改名 = 老数据查不到）。
- 实际流名 `app_<azp>_<stream>`，`azp` 来自令牌，等于你的 appKey。**响应体里的 `status[].name` 就是实际落点**——不用猜，也不用回头查。
- ⚠️ **key 里的 `-` 在流名里会被归一成 `__`**：`llm-gateway` 的流是 `app_llm__gateway_console`。写入端不用管，**查询时要用归一后的名字**，否则得到的是 `stream not found`（HTTP 400），看着像数据没进去。
- 时间轴用的是**到达时间**；日志自带时间是普通字段。要对齐就带 `_timestamp`（微秒 epoch），并先冒烟验证这个版本认不认。

## 5. 查询（程序化）

只认**用户身份的令牌**（`aud` = 日志与监控、scope 含 `observability.read`）。服务密钥打查询面是 `401` —— 按设计如此，不是坏了。

```
POST {PORTAL_BASE_URL}/svc/observability/api/t<租户UUID去横线>/_search?type=logs
{ "query": { "sql": "SELECT service, level, message FROM \"app_<key>_console\" ORDER BY _timestamp DESC",
             "from": 0, "size": 20, "start_time": <微秒>, "end_time": <微秒> } }
```

- 时间范围是**微秒** epoch——不传或范围写窄了，结果就是「什么都没有」。这是「明明写进去了却查不到」的头号原因。
- 流名要带前缀：`app_<key>_<stream>`。
- `GET …/api/t<租户>/streams` 列出有哪些流：查不到自己那条，说明一条都没写进去过。

## 6. 错误码矩阵

| HTTP / code | 含义 | 下一步 |
| --- | --- | --- |
| 换令牌 `ok:false` + `APP_NOT_INSTALLED` | 你的 App 没装在这个租户 | 设计如此；换租户或先装 |
| 换令牌 200 但 `scope` 空 | 平台没把日志与监控登记为基础服务应用 / 默认授予没有 ingest | 找平台管理员登记 |
| 写入 `401` | 凭证缺失 / 伪造 / 已撤销 / 已过期 | 重签；轮换过的旧密钥立刻失效 |
| 写入 `403` | 身份类型不对（用户身份写入）或 scope 不含 ingest | 换服务身份；核对签出的 scope |
| 写入 200 但 `failed>0` | 记录被拒（形状/大小） | 看 `status[]` 里的原因，别忽略 |
| 查询 `401` | 拿服务密钥查 | 换用户身份 |
| 查询 `stream not found` | 这个流从没写进过东西 | 回到写入那一层，先看成功判据 |
| 查询 `stream not found`，但你确信写成功了 | key 里有 `-`，流名被归一成 `__` | 用 `…/streams` 列一下真实流名 |

## 7. 计量与留存

写入按字节计量（平台的用量面能看到），存储占用同理。两条实践：**生产不开 debug**（一条 debug 的成本和一条 error 一样）、**正文里别塞大 blob**（要看的是能定位的 ID）。

留存期由平台/租户在日志与监控里配置：**日志不是归档**。要长期留证的东西走审计面。
