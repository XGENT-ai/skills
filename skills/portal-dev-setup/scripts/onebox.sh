#!/usr/bin/env bash
# portal-dev-setup — 门户一盒 (one-box) 本地联调栈的薄封装。
#
# 它只做一件事：把 `docker compose` 那串又长又易漏的参数（--env-file + 四层 -f +
# 一组 --profile）按 deploy/compose.env 的**生效值**拼对，其余照直转发。漏一个 -f
# 就是另一套语义（比如少了 onebox 那层，deploy-controller 会起来 crash-loop 刷屏），
# 所以值得让一个脚本来拼。
#
#   onebox.sh env              生效的关键 env（含「同键多次定义且不一致」告警）
#   onebox.sh status           env 摘要 + 容器状态 + 宿主侧健康探测
#   onebox.sh smoke            只跑宿主侧健康探测
#   onebox.sh chain            打印将要使用的 compose 参数（调试用）
#   onebox.sh dc <args...>     docker compose <拼好的参数> <args...>
#
# 例：
#   onebox.sh dc up -d
#   onebox.sh dc run --rm portal-api bun run db:seed:onebox
#   onebox.sh dc logs -f portal-api
set -euo pipefail

ROOT="${XGENT_PORTAL_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)}"
ENV_FILE="$ROOT/deploy/compose.env"

c_red=$'\033[31m'; c_grn=$'\033[32m'; c_ylw=$'\033[33m'; c_dim=$'\033[2m'; c_off=$'\033[0m'
warn() { printf '%s! %s%s\n' "$c_ylw" "$*" "$c_off"; }
die()  { printf '%sERROR: %s%s\n' "$c_red" "$*" "$c_off" >&2; exit 1; }

[ -f "$ENV_FILE" ] || die "$ENV_FILE 不存在。先按 SKILL.md §2 三段拼装出这份文件。"

# --- env-file 解析 -----------------------------------------------------------
# docker compose 读 env-file 是「后定义者胜」，而这份文件是三段拼装出来的、同一个键
# 出现三四次很常见 —— 肉眼读第一处必错，所以一律走这里取值。
_assignments() {
  awk '/^[A-Za-z_][A-Za-z0-9_]*=/ {
    k = $0; sub(/=.*/, "", k)
    v = $0; sub(/^[^=]*=/, "", v)
    sub(/^"(.*)"$/, "\\1", v); sub(/^'"'"'(.*)'"'"'$/, "\\1", v)
    printf "%d\t%s\t%s\n", NR, k, v
  }' "$ENV_FILE"
}
_eff() { # _eff KEY [default]
  local v; v="$(_assignments | awk -F'\t' -v k="$1" '$2==k{v=$3} END{print v}')"
  [ -n "$v" ] && printf '%s' "$v" || printf '%s' "${2-}"
}

APP_KEY="$(_eff APP_KEY)"
APP_IMAGE="$(_eff APP_IMAGE)"
APP_FRONTEND_DIST="$(_eff APP_FRONTEND_DIST)"
CATALOG="$(_eff XGENT_APP_CATALOG files,ingest,llm-gateway,git)"
HTTP_PORT="$(_eff HTTP_PORT 80)"
PROJECT="$(_eff COMPOSE_PROJECT_NAME xgent)"
BASE_URL="http://localhost"; [ "$HTTP_PORT" = "80" ] || BASE_URL="http://localhost:$HTTP_PORT"

# --- compose 参数拼装 ---------------------------------------------------------
# app-dev.yml 里 ${APP_IMAGE:?} / ${APP_KEY:?} 是**解析期**求值的（与 profile 无关），
# 所以没配外部 App 时必须整层不带，否则连 `ps` 都起不来。
compose_args() {
  printf '%s\0' --env-file "$ENV_FILE" \
    -f "$ROOT/deploy/docker-compose.yml" \
    -f "$ROOT/deploy/onebox/docker-compose.onebox.yml"
  if [ -n "$APP_KEY" ] && [ -n "$APP_IMAGE" ]; then
    printf '%s\0' -f "$ROOT/deploy/app-devkit/docker-compose.app-dev.yml"
    [ -n "$APP_FRONTEND_DIST" ] && printf '%s\0' -f "$ROOT/deploy/app-devkit/docker-compose.app-frontend.yml"
    printf '%s\0' --profile app-external
  fi
  printf '%s\0' --profile local-infra
  local k; for k in ${CATALOG//,/ }; do printf '%s\0' --profile "app-$k"; done
}

dc() { local a=(); while IFS= read -r -d '' x; do a+=("$x"); done < <(compose_args); docker compose "${a[@]}" "$@"; }

# --- 子命令 -------------------------------------------------------------------
cmd_env() {
  echo "compose.env: $ENV_FILE"
  echo
  local k v n
  for k in COMPOSE_PROJECT_NAME XGENT_IMAGE XGENT_PROXY_IMAGE XGENT_APP_CATALOG NODE_ENV DEV_MOCK_OAUTH \
           HTTP_PORT HTTPS_PORT POSTGRES_PORT REDIS_PORT MINIO_PORT MINIO_CONSOLE_PORT \
           APP_KEY APP_IMAGE APP_FRONTEND_DIST PREVIEW_MEDIA_CONVERTER_URL; do
    v="$(_eff "$k")"
    n="$(_assignments | awk -F'\t' -v k="$k" '$2==k{seen[$3]=1} END{print length(seen)}')"
    printf '  %-28s %s' "$k" "${v:-${c_dim}(空)${c_off}}"
    [ "$n" -gt 1 ] && printf '   %s← 同键 %s 个不同值，取最后一个%s' "$c_ylw" "$n" "$c_off"
    printf '\n'
  done
  echo
  [ "$PROJECT" = "xgent" ] && warn "COMPOSE_PROJECT_NAME 还是模板默认的 xgent —— 同机另一套 compose 会被当成同一项目（容器被接管、命名卷共享）。改成 onebox-<你的 App key>。"
  [ "$(_eff NODE_ENV)" = "development" ] || warn "NODE_ENV 不是 development —— 镜像烘的是 production，不覆盖则 prod-guard 拒绝在 DEV_MOCK_OAUTH=true 下启动 portal-api，register-app 也拒跑。"
  [ "$(_eff DEV_MOCK_OAUTH)" = "true" ] || warn "DEV_MOCK_OAUTH 不是 true —— 没有 dev 登录门，浏览器进不去。"
  [ -n "$(_eff PREVIEW_MEDIA_CONVERTER_URL)" ] && warn "PREVIEW_MEDIA_CONVERTER_URL 非空 —— 一盒不装 ffmpeg，非空会让每次转换 exec 一个不存在的二进制。留空。"
  case "$(_eff XGENT_IMAGE)" in *:latest) warn "XGENT_IMAGE 用了 latest —— 这个仓库 tag 不可变、不接受可变指针，写具体版本。";; esac
  if [ -n "$APP_FRONTEND_DIST" ]; then
    [ -f "$APP_FRONTEND_DIST/index.html" ] || warn "APP_FRONTEND_DIST 下没有 index.html（${APP_FRONTEND_DIST}）—— iframe 会 404。要绝对路径、且指向构建产物目录。"
  fi
  return 0
}

cmd_smoke() {
  echo "宿主侧健康探测（${BASE_URL}）"
  local k code body keys="$CATALOG"
  case ",$CATALOG," in *",$APP_KEY,"*) ;; *) [ -n "$APP_KEY" ] && keys="$CATALOG,$APP_KEY";; esac
  local tmp; tmp="$(mktemp)"
  _probe() { # _probe LABEL PATH
    local code body
    code="$(curl -s -m 8 -o "$tmp" -w '%{http_code}' "$BASE_URL$2" 2>/dev/null || echo 000)"
    body="$(head -c 160 "$tmp" 2>/dev/null || true)"
    if [ "$code" = "200" ]; then printf '  %s✓%s %-22s %s\n' "$c_grn" "$c_off" "$1" "$body"
    else printf '  %s✗%s %-22s HTTP %s  %s\n' "$c_red" "$c_off" "$1" "$code" "$body"; fi
  }
  _probe "/health" /health
  for k in ${keys//,/ }; do _probe "/svc/$k/health" "/svc/$k/health"; done
  echo
  echo "${c_dim}404 = /svc 放行 map 没写成或反代在写 map 之前就起了（跑 register-app，再 dc exec reverse-proxy caddy reload）${c_off}"
  echo "${c_dim}502 = 后端容器没起/崩了/没听 8080/网络别名 <key>-server 没命中（dc logs <key>-server 或 app-backend）${c_off}"
  rm -f "$tmp"
  return 0
}

cmd_status() {
  cmd_env
  echo "容器（project: ${PROJECT}）"
  dc ps || true
  echo
  warn "portal-api / *-server 的 unhealthy 是假红：一盒镜像没装 curl，而 healthcheck 写的就是 curl —— 它永远失败。判活只看下面的探测。"
  echo
  cmd_smoke
}

case "${1-status}" in
  env)    cmd_env ;;
  smoke)  cmd_smoke ;;
  status) cmd_status ;;
  chain)  { while IFS= read -r -d '' x; do printf '%s ' "$x"; done < <(compose_args); echo; } ;;
  dc)     shift; dc "$@" ;;
  -h|--help|help) sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' ;;
  *)      die "未知子命令 '$1'。用 env | status | smoke | chain | dc <args...>" ;;
esac
