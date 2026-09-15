#!/usr/bin/env bash
# 冒烟：换令牌 → 写一条带标记的记录 → 告诉你怎么在界面里找到它。
# 接日志的第一步跑它，比读文档快：三个前提里哪个不成立，它会直接说出来。
#
#   PORTAL_BASE_URL=https://<门户域名> TENANT_ID=<租户UUID> APP_KEY=<你的key> \
#   SA_CLIENT_ID=… SA_CLIENT_SECRET=… ./smoke-ingest.sh
#
# 已有长期密钥就跳过换令牌：ACCESS_KEY=xsak_… ./smoke-ingest.sh
# 令牌/密钥永远不打印。
set -euo pipefail

: "${PORTAL_BASE_URL:?缺 PORTAL_BASE_URL（门户公开源，形如 https://portal.example.com）}"
: "${TENANT_ID:?缺 TENANT_ID（目标租户 UUID）}"
: "${APP_KEY:?缺 APP_KEY（你的 App key，用来拼期望的流名）}"
API_BASE_URL="${API_BASE_URL:-$PORTAL_BASE_URL}"
STREAM="${STREAM:-console}"
MARKER="portal-logging-smoke-$(date +%s)"

jget() { python3 -c 'import json,sys;d=json.load(sys.stdin);print(eval("d"+sys.argv[1]) if d else "")' "$1" 2>/dev/null || true; }

if [ -n "${ACCESS_KEY:-}" ]; then
  TOKEN="$ACCESS_KEY"
  echo "① 用长期访问密钥（跳过换令牌）"
else
  : "${SA_CLIENT_ID:?缺 SA_CLIENT_ID（平台注入 <PREFIX>_SA_CLIENT_ID）}"
  : "${SA_CLIENT_SECRET:?缺 SA_CLIENT_SECRET}"
  RESP=$(curl -sS -X POST "$API_BASE_URL/api/tokens/service" \
    -u "$SA_CLIENT_ID:$SA_CLIENT_SECRET" -H 'content-type: application/json' \
    -d "{\"grant_type\":\"client_credentials\",\"tenant_id\":\"$TENANT_ID\",\"scope\":\"observability.ingest\"}")
  OK=$(printf '%s' "$RESP" | jget '["ok"]')
  if [ "$OK" != "True" ]; then
    echo "✗ 换令牌失败：$(printf '%s' "$RESP" | jget '["error"]["code"]')"
    echo "  APP_NOT_INSTALLED ⇒ 你的 App 没装在这个租户（设计如此）。其余见 references/contract.md §6。"
    exit 1
  fi
  SCOPE=$(printf '%s' "$RESP" | jget '["data"]["scope"]')
  TOKEN=$(printf '%s' "$RESP" | jget '["data"]["access_token"]')
  if [ -z "$SCOPE" ]; then
    echo "✗ 令牌签出来了，但 scope 是空的 —— 平台还没把「日志与监控」登记为平台基础服务应用"
    echo "  （或默认授予里没有 observability.ingest）。往下写一定 403，先找平台管理员登记。"
    exit 1
  fi
  echo "① 换到服务令牌，scope=$SCOPE"
fi

echo "② 写一条：stream=$STREAM marker=$MARKER"
W=$(curl -sS -o /tmp/pl-smoke.json -w '%{http_code}' -X POST \
  "$PORTAL_BASE_URL/svc/observability/v1/ingest/$STREAM" \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d "[{\"message\":\"$MARKER hello from smoke\",\"level\":\"info\",\"service\":\"$APP_KEY\",\"host\":\"$(hostname)\"}]")
BODY=$(cat /tmp/pl-smoke.json); rm -f /tmp/pl-smoke.json
CODE=$(printf '%s' "$BODY" | jget '["code"]')
FAILED=$(printf '%s' "$BODY" | jget '["status"][0]["failed"]')

if [ "$W" != "200" ]; then
  echo "✗ HTTP $W —— 401=凭证无效/已撤销；403=身份类型不对或 scope 不含 ingest。见 contract.md §6"
  echo "  $BODY"; exit 1
fi
if [ "$CODE" != "200" ] || [ "${FAILED:-1}" != "0" ]; then
  echo "✗ HTTP 200 但没落库（这正是「一直 200、界面查不到」的来源）：$BODY"; exit 1
fi

echo "③ 写入成功（HTTP 200 + code:200 + failed:0）"
cat <<TIP

下一步，在「日志与监控」App 里核对（用你自己的账号，不是这把密钥）：
  · 流名应为  app_${APP_KEY}_${STREAM}
  · 搜        $MARKER
  · 时间范围选【最近 15 分钟】—— 时间轴是到达时间，范围写窄了就是"什么都没有"

搜不到就按这个次序查：流名前缀 → 时间范围 → 令牌的 azp 是不是你以为的那个。
TIP
