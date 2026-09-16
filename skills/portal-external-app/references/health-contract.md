# App `/health`：与平台一致的判定与发布前检查

**平铺 JSON 与 Envelope 都支持，外部 App 与内建 App 使用相同判据。** 已有健康接口不需要仅为切换包装格式而修改。匿名 `GET /health`，或 `deployDescriptor.healthPath` 指定的路径，应能直接报告服务是否就绪。

## 1. 平台实际怎样判断

按下面的顺序判断，部署控制器、平台信息页、`health:deployed` 巡检及本 skill 的检查脚本保持一致：

1. HTTP 非 **2xx**：不健康。
2. HTTP 2xx，JSON 对象顶层含 `ok`：仅当 **`ok === true && data.db === true`** 才健康；缺失字段、`false`、字符串 `"ok"` / `"true"` 都不能代替布尔 `true`。
3. HTTP 2xx，响应没有顶层 `ok`：健康。平台沿用 HTTP 状态判定，不检查平铺字段。

`service`、`redis`、`time` 不参与平台判定；服务名带不带 `-server` 不影响结果。非 JSON / 空响应的 2xx 也沿用第 3 条兼容规则。这是现有检测行为；新接口建议返回有诊断信息的 JSON，不要据此把登录页或无依赖检查的端点当就绪接口。

| HTTP / 响应 | 平台结论 |
| --- | --- |
| 200 + `{"service":"wish-list","db":"ok","redis":"ok","time":"…"}` | 健康：平铺响应 |
| 200 + `{"service":"payhub","db":"ok","redis":"disabled","time":"…"}` | 健康：平铺响应 |
| 200 + `{"ok":true,"data":{"service":"pagebuilder-server","db":true,"redis":true}}` | 健康：合法 Envelope |
| 200 + `{"ok":true,"data":{"db":"ok"}}` | 不健康：信封内 db 必须是布尔 true |
| 200 + `{"ok":true,"db":true}` | 不健康：缺 data.db |
| 200 + `{"ok":false}` 或 `{"ok":true,"data":{"db":false}}` | 不健康 |
| 503 + 任意响应 | 不健康：HTTP 优先 |
| 200 + `{"db":"down"}` | **仍会判健康**：平铺依赖故障应返回 503，不能只改正文 |

必要依赖（包括 Redis）故障时建议统一返回 **503**。特别是 Envelope 的 `data.redis:false` 不会单独让平台判失败，必须通过 HTTP 状态报告必要 Redis 故障。健康响应不要包含密钥、连接串或原始异常。

## 2. 两种正确实现范例（Bun / Elysia）

以下使用 App 已有的 `db` / `sql` / `redis`；只有确实未使用 Redis 时，`redis` 才为 `null`。连接的查询/命令超时应短于平台单次探测的 5 秒（例如 3 秒）。选择一种响应写法即可。

```ts
app.get('/health', async ({ set }) => {
  const [dbUp, redisUp] = await Promise.all([
    db.execute(sql`select 1`).then(() => true, () => false),
    redis === null
      ? Promise.resolve(true)
      : redis.ping().then((reply) => reply === 'PONG', () => false),
  ]);
  set.status = dbUp && redisUp ? 200 : 503;

  // 写法 A：平铺。依赖故障由上面的 503 表达。
  return {
    service: 'my-app',
    db: dbUp ? 'ok' : 'down',
    redis: redis === null ? 'disabled' : redisUp ? 'ok' : 'down',
    time: new Date().toISOString(),
  };

  // 写法 B：用下面的 return 替换上面的 return，允许使用等价的 ok({...})。
  // return { ok: true, data: { service: 'my-app', db: dbUp, redis: redisUp, time: new Date().toISOString() } };
});
```

采用 A 时，响应中间件应保留平铺结构；若改为 B，`data.db` 必须同步改为布尔值。不能只给字符串状态套 `ok(...)`，那会得到 `data.db:"ok"`，被平台判失败。

## 3. 发布前检查

本 skill 自带 [scripts/verify-health.ts](../scripts/verify-health.ts)，只依赖 Bun；旁边的 `health-verdict.ts` 与门户运行时同源，导出 skill 时会校验一致性。在 App repo 根目录执行；`SKILL_DIR` 表示本 skill 的 `SKILL.md` 所在目录（本文件的上一级），按实际加载位置填写。URL 和 `HEALTH_APP` 按实际替换：

```bash
SKILL_DIR="<本 skill 的 SKILL.md 所在目录>"
HEALTH_APP=my-app

# 待发布镜像的直连路径。
bun "$SKILL_DIR/scripts/verify-health.ts" http://127.0.0.1:8080/health "$HEALTH_APP"

# 接入一盒后的代理路径。
bun "$SKILL_DIR/scripts/verify-health.ts" "http://localhost/svc/$HEALTH_APP/health" "$HEALTH_APP"
```

默认 `ready` 模式，平台会判健康才退出 **0**；失败退出 **1**，参数错误退出 **2**。`listingKey` 仅用于输出标签，不额外校验响应中的 service。脚本的请求、JSON 解析和判定与平台一致，不另加 Content-Type、字段类型或命名规则；因此这是就绪检查，不是 JSON schema 验证器。

新增/修改健康接口时，在隔离测试环境制造依赖故障后执行（不要断开生产共享依赖）：

```bash
bun "$SKILL_DIR/scripts/verify-health.ts" http://127.0.0.1:8080/health "$HEALTH_APP" not-ready
```

`not-ready` 要求收到平台会判失败的响应，例如 503 或 `data.db:false`；连接失败仍退出 1，不能替代故障响应的验证。恢复依赖后重跑 `ready`。Docker healthy / `curl -f` 只看 HTTP 时，无法覆盖信封的 db 判据。
