# 采集器：三种形态 + 可直接抄的配置

三种形态写入面完全一样（[contract.md](contract.md)），差别只在「从哪儿把行读出来」。下面的 fluent-bit 配置在真镜像上跑通过，**照抄再改路径/环境变量**比自己重写省一整天——尤其是多行合并那一段，它有两个静默坑。

- [A 容器 stdout → 日志驱动 → 采集器](#a-容器-stdout)
- [B 采集器 tail 日志文件](#b-文件-tail)
- [C 应用内直接批量 POST](#c-应用内直接打点)
- [采集器怎么知道自己写成功了](#采集器怎么知道自己写成功了)
- [级别判定（两种形态共用）](#级别判定)
- [多行合并的两个坑](#多行合并的两个坑)

---

## A 容器 stdout

应用零改动：容器的 stdout/stderr 由 docker 的日志驱动转给采集器。适合容器化交付。

```yaml
# 你的服务：把日志转给采集器（fluentd-async 让采集器没起来时容器照常启动）
services:
  my-service:
    logging:
      driver: fluentd
      options:
        fluentd-address: 127.0.0.1:24224
        fluentd-async: "true"
        fluentd-async-reconnect-interval: 5s
        tag: my-service          # ⚠️ 不要带点，见下面 rewrite_tag 那段

  log-collector:
    image: fluent/fluent-bit:3.2.10
    restart: unless-stopped
    environment:
      OBS_HOST: ${PORTAL_HOST}              # 门户公开源的主机名（不带 scheme）
      OBS_STREAM: console
      OBS_SOURCE_HOST: ${HOSTNAME}
    env_file: [./collector.env]             # 只放 OBS_ACCESS_KEY，权限 600
    ports: ["127.0.0.1:24224:24224"]
    volumes:
      - ./fluent-bit:/fluent-bit/etc:ro
      - collector-state:/var/lib/fluent-bit
volumes: { collector-state: {} }
```

```ini
# fluent-bit/fluent-bit.conf
[SERVICE]
    Flush            5
    Grace            5
    Log_Level        info
    Parsers_File     parsers.conf
    storage.path     /var/lib/fluent-bit/storage
    storage.sync     normal
    HTTP_Server      Off

[INPUT]
    Name             forward
    Listen           0.0.0.0
    Port             24224
    storage.type     filesystem

# ⚠️ docker 的日志驱动给 stdout 与 stderr 【同一个 tag】，而多行合并按 tag 缓冲：
# 不先拆开，stderr 那条栈的续行会被接到上一条 stdout 记录屁股后面（实测如此）。
# Match_Regex 挑「还没拆过的 tag」——日志驱动的 tag 不含点、拆出来的含点，所以重新入链的
# 记录不会再次命中这条规则（否则就是死循环）。新接进来的容器 tag 也不能带点。
[FILTER]
    Name                  rewrite_tag
    Match_Regex           ^[^.]+$
    Rule                  $source ^(stdout|stderr)$ console.$TAG.$1 false
    Emitter_Name          console_split

[FILTER]
    Name                  multiline
    Match                 console.*
    multiline.key_content log
    multiline.parser      js_stack

[FILTER]
    Name             lua
    Match            console.*
    script           enrich.lua
    call             enrich

[OUTPUT]
    Name             http
    Match            console.*
    Host             ${OBS_HOST}
    Port             443
    tls              On
    tls.verify       On
    URI              /svc/observability/v1/ingest/${OBS_STREAM}
    Format           json
    Json_date_key    timestamp
    Json_date_format iso8601
    Header           Authorization Bearer ${OBS_ACCESS_KEY}
    Header           Content-Type application/json
    Retry_Limit      8
    Log_response_payload Off
    storage.total_limit_size 64M
```

```lua
-- fluent-bit/enrich.lua —— docker 驱动的记录 {log, source, container_name} → 统一字段
local host = os.getenv("OBS_SOURCE_HOST") or "unknown"

function enrich(tag, ts, record)
  local name = string.gsub(record["container_name"] or tag or "unknown", "^/", "")
  record["service"] = string.match(name, "^[%w]+%-(.-)%-%d+$") or name   -- compose 容器名 → 服务名
  record["fd"] = record["source"] or "stdout"
  record["host"] = host
  record["message"] = record["log"]
  record["level"] = level_of(record["fd"], record["log"] or "")          -- 见「级别判定」
  record["log"] = nil; record["source"] = nil
  record["container_id"] = nil; record["container_name"] = nil
  return 2, ts, record
end
```

---

## B 文件 tail

进程管理器把 stdout 写成文件时用这条。与 A 的差别只在 INPUT 与 service 的来源。

```ini
[INPUT]
    Name             tail
    # ⚠️ 每个文件一个 tag（`*` 展开成文件路径）：多行合并按 tag 缓冲，
    # 多个文件共用一个 tag 时 A 进程的栈会把 B 进程随后那行吸进来。
    Tag              console.*
    Path             /logs/app.out.log,/logs/app.err.log
    Path_Key         file
    Parser           line
    DB               /var/lib/fluent-bit/tail.db
    Refresh_Interval 10
    Rotate_Wait      30
    Skip_Long_Lines  On
    storage.type     filesystem
    # Read_from_Head 默认 Off：首次启动不回灌历史几万行。要补历史再临时打开。
```

```lua
-- 从被跟随的文件名派生 service / fd：/logs/<service>.<out|err>.log
function enrich(tag, ts, record)
  local service, fd = string.match(record["file"] or "", "([%w%-]+)%.(%a+)%.log$")
  record["service"] = service or "unknown"
  record["fd"] = (fd == "err") and "stderr" or "stdout"
  record["host"] = os.getenv("OBS_SOURCE_HOST") or "unknown"
  record["level"] = level_of(record["fd"], record["message"] or "")
  record["file"] = nil
  return 2, ts, record
end
```

如果进程管理器给每行加了时间戳前缀（很多都会），用一个正则 parser 把它切掉，正文才干净：

```ini
[PARSER]
    Name         line
    Format       regex
    Regex        ^(?<time>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}): (?<message>.*)$
    Time_Key     time
    Time_Format  %Y-%m-%dT%H:%M:%S
    Time_Offset  +0800          # 前缀不带时区就得固定一个，按机器时区填
    Time_Keep    Off
    # ⚠️ 空行也要带上空的 message：缺了这个键，多行合并器认不出它是续行、直接放行，
    # 一条错误转储会被中间那个空行劈成两半，后半截还丢了 Path_Key ⇒ service 变成 unknown。
    Skip_Empty_Values Off
```

---

## C 应用内直接打点

日志量小、想要结构化字段与精确级别时最省事。要点：**批量、异步、丢得起**。

```ts
const BUF: Record<string, unknown>[] = [];

export function log(level: "info" | "warn" | "error", message: string, ctx: Record<string, unknown> = {}) {
  BUF.push({ level, message, service: SERVICE, host: HOST, ...ctx });
  if (BUF.length >= 200) void flush();          // 满一批就走
}

async function flush() {
  if (!BUF.length) return;
  const batch = BUF.splice(0, BUF.length);      // 先摘出来：flush 期间新来的行不阻塞
  try {
    const token = await serviceToken();         // 缓存到 expires_in - 10s，见 contract.md §2
    const r = await fetch(`${PORTAL_BASE_URL}/svc/observability/v1/ingest/console`, {
      method: "POST",
      headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
      body: JSON.stringify(batch),
      signal: AbortSignal.timeout(5000),
    });
    // 只看 HTTP 200 会把「收下了但没落库」当成功
    const body = await r.json().catch(() => null);
    if (!r.ok || body?.code !== 200) console.error("[log] ingest rejected", r.status, body?.status);
  } catch (e) {
    // 丢弃。日志不值得把主流程拖垮，也不值得无限重试堆成内存泄漏。
    console.error("[log] ingest failed", (e as Error).message);
  }
}

setInterval(() => void flush(), 5000).unref();  // 定时兜底
process.on("beforeExit", () => void flush());   // 退出前冲一次
```

**不要**在请求处理路径上 `await flush()`：远端抖一下，你的接口就跟着抖。

---

## 采集器怎么知道自己写成功了

这是形态 A/B 最容易漏的一环：**写入的响应体不在你眼前**。fluent-bit 只会在自己的日志里留一行

```
[2026-01-01 00:00:00] [ info] [output:http:http.0] <host>:443, HTTP status=200
```

而「HTTP 200 但 `status[].failed>0`」——记录被判拒、根本没落库——在这行里**看起来一模一样**。401/403 也只是按 `Retry_Limit` 重试几次后丢弃：日志静静地没了，采集器一切正常。

所以：

- **验收不要看采集器的日志**，看冒烟那一条 curl（你拿得到响应体，能判两层）+ 界面真的搜到。`scripts/smoke-ingest.sh` 就是干这个的。
- **排障时**临时把响应打印打开，看完再关掉（响应体会一直进采集器日志）：

  ```ini
  [OUTPUT]
      Log_response_payload On
  ```

- **长期监控**：采集器进程本身要有人看（重启、`[error]` 行、积压）。日志链路断了不会有人来告诉你——它的症状就是「最近怎么没日志了」，而那通常是很久以后才有人注意到。

## 级别判定

产生方给的级别最准。搬运方只能猜，猜的次序要**先精确后启发式**，否则会把例行 warn 整批判成 error：

```lua
-- ① 正文 JSON 里的 "lvl"/"level" —— 产生方给的，直接用
-- ② warn 标记 —— 必须排在 fd 之前：console.warn 一类也走 stderr
-- ③ fd == stderr ⇒ error
-- ④ stdout 上的错误标记 —— 第三方库（连接失败那种）并不按 fd 分级
-- ⑤ 其余 info
function level_of(fd, msg)
  local lvl = msg:match('"lvl"%s*:%s*"(%a+)"') or msg:match('"level"%s*:%s*"(%a+)"')
  if lvl == "error" or lvl == "warn" or lvl == "info" or lvl == "debug" then return lvl end
  local head = msg:match("^[^{]*") or msg            -- 只看 JSON 之前那截，见下
  if head:match("⚠") or head:match("[Ww]arn") or head:match("WARN") then return "warn" end
  if fd == "stderr" then return "error" end
  if head:match("✗") or head:match("❌") or head:match("[Ee]rror") or head:match("ERROR")
    or head:match("[Ff]ail") or head:match("FAIL") or head:match("失败") or head:match("异常") then return "error" end
  if msg:match('"error"%s*:%s*"') then return "error" end
  return "info"
end
```

④ 只看 `{` 之前那一截，是因为正文里的 JSON 常含 `delivery_failed` 这类**字段名/事件名**，整条搜关键词会把一条 info 判成 error；JSON 部分只认 `"error":"…"`（有内容的错误字段）。

产生方那边对应地做一件事就够：打错误行时把级别写进正文的 JSON。例如 `[api] request failed {"lvl":"error","path":"/x","status":500}`，采集器走 ① 直接命中，不再需要猜。

---

## 多行合并的两个坑

```ini
[MULTILINE_PARSER]
    Name         js_stack
    Type         regex
    Flush        2
    rule         "start_state"   "/^.*$/"                                  "cont"
    rule         "cont"          "/^(\s|\d+ \||error: |\w+Error: |$)/"     "cont"
```

start_state 收下任意一行；下一行**看着像续行**就并进去，否则冲出上一条、另起一条。续行的四种形状：前导空白（`    at …` / `   errno:`）、代码帧（`262 |  …`）、`error:` / `SyntaxError:` 抬头、空行。

1. **按 tag 缓冲**：一个 tag 一条流。多个日志源共用一个 tag ⇒ A 的栈吸走 B 的下一行。tail 用 `Tag console.*`（每文件一个），docker 驱动用 `rewrite_tag` 按 `source` 拆。
2. **空行必须带着 `message` 键进来**（`Skip_Empty_Values Off`）：解析器丢掉空值时，合并器认不出它是续行而直接放行，一条转储被劈成两半，后半截还丢了文件名派生的字段。

验证方式：拿一段**真的**崩溃输出（不是自己编的三行）喂进去，看它是不是**一条**记录、级别是不是 error。本机跑一遍就知道：

```bash
docker run --rm -v "$PWD/etc:/fluent-bit/etc:ro" -v "$PWD/logs:/logs:ro" \
  fluent/fluent-bit:3.2.10          # 把 OUTPUT 换成 Name stdout / Format json_lines 再跑
```
