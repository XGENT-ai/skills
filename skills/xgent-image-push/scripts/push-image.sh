#!/usr/bin/env bash
# 把 App 镜像推到 <PROJECT> 私有仓库（Harbor）。
#
#   ROBOT_USER='robot$<PROJECT>+<app>' ROBOT_SECRET=… ./push-image.sh <app> v1.2.3
#
# 顺序：链路预检 → 凭证与 tag 冲突检查 → 保留策略预警 → login → build（锁 amd64）
#       → 架构核对 → push → 验证 → logout
# 任何一项不过就停下，不把时间浪费在注定失败的 push 上。
#
# 自包含：只依赖 docker + curl（不需要 jq），不读取任何外部配置文件，可单独复制走。
set -euo pipefail
export LC_ALL=C

REGISTRY="${REGISTRY:-<REGISTRY>}"
PROJECT="${PROJECT:-<PROJECT>}"
PLATFORM="${PLATFORM:-linux/amd64}"
RETAIN_K="${RETAIN_K:-10}"
# TLS 握手耗时阈值（毫秒）。大陆内 20–80ms；（地域已移除） → （地域已移除）实测 RTT 440ms，握手会到 900ms+
CONNECT_WARN_MS="${CONNECT_WARN_MS:-300}"

APP=""; TAG=""
CONTEXT="."; DOCKERFILE=""
DO_BUILD=1; DRY_RUN=0; FORCE_TAG=0; SKIP_LINK=0; KEEP_LOGIN=0

usage() {
  cat <<'USAGE'
用法：push-image.sh <app> <tag> [选项]

  <app>   App 名 = repository 名（<app> / <app> / <app> / <app>）
  <tag>   本次发版的 tag。不可变——每次发版换一个新的

选项：
  --context <dir>      构建上下文，默认 .
  --dockerfile <path>  Dockerfile 路径，默认由 docker 自己找
  --no-build           镜像已经在本地，只推不建
  --dry-run            只做只读检查，不 login/build/push
  --force-tag          允许覆盖已存在的 tag（⚠️ 覆盖不会触发部署侧重建）
  --skip-link-check    跳过链路预检（⚠️ 境外推送实测 49KB/s，跳过前先读 SKILL.md ①）
  --keep-login         结束后不 docker logout
  --retain-k <n>       保留策略的 K，仅用于预警文案，默认 10
  --platform <p>       构建平台，默认 linux/amd64（生产机都是 x86_64）

环境变量：
  ROBOT_USER    形如 robot$<PROJECT>+<app>（注意在 shell 里要用单引号）
  ROBOT_SECRET  robot 密钥，从 CI secret 取，不要写进文件
  REGISTRY      默认 <REGISTRY>
  PROJECT       默认 <PROJECT>
USAGE
}

# ── 输出 ────────────────────────────────────────────────────────────
if [[ -t 1 ]]; then R=$'\033[31m'; G=$'\033[32m'; Y=$'\033[33m'; B=$'\033[1m'; N=$'\033[0m'
else R=""; G=""; Y=""; B=""; N=""; fi
log()  { printf '%s\n' "$*"; }
step() { printf '\n%s▸ %s%s\n' "$B" "$*" "$N"; }
ok()   { printf '%s✔%s %s\n' "$G" "$N" "$*"; }
warn() { printf '%s⚠%s %s\n' "$Y" "$N" "$*" >&2; }
die()  { printf '%s✘%s %s\n' "$R" "$N" "$*" >&2; exit 1; }

# ── 参数 ────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)         usage; exit 0 ;;
    --context)         CONTEXT="${2:?--context 缺少值}"; shift 2 ;;
    --dockerfile)      DOCKERFILE="${2:?--dockerfile 缺少值}"; shift 2 ;;
    --retain-k)        RETAIN_K="${2:?--retain-k 缺少值}"; shift 2 ;;
    --platform)        PLATFORM="${2:?--platform 缺少值}"; shift 2 ;;
    --no-build)        DO_BUILD=0; shift ;;
    --dry-run)         DRY_RUN=1; shift ;;
    --force-tag)       FORCE_TAG=1; shift ;;
    --skip-link-check) SKIP_LINK=1; shift ;;
    --keep-login)      KEEP_LOGIN=1; shift ;;
    -*)                die "未知选项：$1（--help 看用法）" ;;
    *)                 if [[ -z "$APP" ]]; then APP="$1"; elif [[ -z "$TAG" ]]; then TAG="$1"
                       else die "多余的参数：$1"; fi; shift ;;
  esac
done

[[ -n "$APP" && -n "$TAG" ]] || { usage; exit 2; }
[[ "$APP" =~ ^[a-z0-9][a-z0-9._-]*$ ]] || die "App 名只能是小写字母/数字/.-_：$APP"
[[ "$TAG" =~ ^[A-Za-z0-9_][A-Za-z0-9._-]{0,127}$ ]] || die "tag 不合法：$TAG"
command -v docker >/dev/null || die "没找到 docker"
command -v curl   >/dev/null || die "没找到 curl"
: "${ROBOT_USER:?未设置 ROBOT_USER（形如 'robot\$<PROJECT>+$APP'，注意单引号）}"
: "${ROBOT_SECRET:?未设置 ROBOT_SECRET —— 从 CI secret 取，不要写进文件}"

IMAGE="${REGISTRY}/${PROJECT}/${APP}:${TAG}"

case "$ROBOT_USER" in
  "robot\$${PROJECT}+${APP}") : ;;
  robot\$*) warn "ROBOT_USER 是 ${ROBOT_USER}，但要推的是 ${APP} —— 权限是项目级所以可能推得动，但请只推自己那一个 repository" ;;
  *) warn "ROBOT_USER 不是 robot\$${PROJECT}+${APP} 的形式；若 shell 把 \$ 展开了，用单引号" ;;
esac

# 凭证只落在 600 的临时 curl 配置里，不进 argv（共享 runner 上 ps 看得到 argv）
umask 077
TMP_DIR="$(mktemp -d)"; trap 'rm -rf "$TMP_DIR"' EXIT
CURLRC="$TMP_DIR/curlrc"; BODY="$TMP_DIR/body"
_esc="${ROBOT_SECRET//\\/\\\\}"; _esc="${_esc//\"/\\\"}"
printf 'user = "%s:%s"\n' "$ROBOT_USER" "$_esc" > "$CURLRC"
unset _esc

harbor_get() {  # $1=path → 打印 http_code，body 落在 $BODY
  local c=""
  c="$(curl -sS -o "$BODY" -w '%{http_code}' --max-time 60 --config "$CURLRC" "https://${REGISTRY}$1")" || true
  printf '%s' "${c:-000}"
}

extract_tags() {  # stdin: tags/list 的 JSON → 每行一个 tag
  sed -n 's/.*"tags"[[:space:]]*:[[:space:]]*\[\([^]]*\)\].*/\1/p' \
    | tr ',' '\n' | sed -e 's/^[[:space:]]*"//' -e 's/"[[:space:]]*$//' -e '/^$/d' -e '/^null$/d'
}

log "${B}镜像：${N}${IMAGE}"
log "${B}账号：${N}${ROBOT_USER}"
[[ $DRY_RUN -eq 1 ]] && warn "--dry-run：只做只读检查，不会 login/build/push"

# ── 1. 链路预检（①：境外推送 49KB/s，先挡住）─────────────────────────
step "1/7 链路预检"
if [[ $SKIP_LINK -eq 1 ]]; then
  warn "已跳过。境外 → （地域已移除）的推送实测 49KB/s（2.93GB ≈ 17 小时），确认你在大陆再继续"
else
  # 量的是 TLS 握手耗时（appconnect - connect），不是 TCP 握手：
  # 挂了代理时 TCP 只连到本地代理（几十微秒），量不出真实距离；
  # 而 TLS 握手是与 registry 本体完成的，穿过 CONNECT 隧道也照样反映端到端 RTT。
  proxy="${HTTPS_PROXY:-${https_proxy:-}}"
  if [[ -n "$proxy" ]] && [[ ",${NO_PROXY:-${no_proxy:-}}," != *",${REGISTRY},"* ]]; then
    warn "检测到 HTTPS 代理（${proxy}）—— push 也会走它。下面的数字反映的是**代理链路**，
   不是直连（地域已移除）；代理本身也可能成为上传瓶颈"
  fi
  best=""
  for _ in 1 2 3; do
    sample="$(curl -sS -o /dev/null --max-time 15 \
              -w '%{http_code} %{time_connect} %{time_appconnect} %{time_total}' \
              "https://${REGISTRY}/v2/")" || true
    read -r c tc ta tt <<<"${sample:-}"
    # http_code=000 表示这次根本没连上 —— curl 仍会打印 0.000000 的耗时，不能当成"很快"
    [[ -n "${c:-}" && "$c" != "000" ]] || continue
    t="$(awk -v a="${ta:-0}" -v b="${tc:-0}" -v tot="${tt:-0}" \
         'BEGIN{d=a-b; if(a+0<=0||d<=0) d=tot; print d}')"
    best="$(awk -v a="$best" -v b="$t" 'BEGIN{print (a==""||b+0<a+0)?b:a}')"
  done
  [[ -n "$best" ]] || die "连不上 https://${REGISTRY} —— 先查 DNS / 出网策略 / 代理，这不是仓库的问题"
  ms="$(awk -v s="$best" 'BEGIN{printf "%.0f", s*1000}')"
  if [[ "$ms" -gt "$CONNECT_WARN_MS" ]]; then
    log "TLS 握手 ${ms}ms（阈值 ${CONNECT_WARN_MS}ms）"
    die "这台机器大概率在境外。境外 → （地域已移除）推送实测 49KB/s、丢包 35%，一个 2.93GB 的镜像要 17 小时，
   表现是 push 卡在 Retrying 直到超时，重试无效。请换大陆 runner。
   （拉取方向是好的，所以「我能 pull」不能说明推得动 —— 推拉不对称）
   确实要试：加 --skip-link-check"
  fi
  ok "TLS 握手 ${ms}ms —— 在大陆网段的正常范围"
fi

# ── 2. 凭证 + tag 冲突（③：tag 不可变）──────────────────────────────
# 放在 build 之前：凭证坏掉的话，此刻就红，不用等构建跑完
step "2/7 凭证与 tag 冲突检查"
code="$(harbor_get "/v2/${PROJECT}/${APP}/tags/list?n=1000")"
case "$code" in
  200) : ;;
  401|403) die "凭证被拒（HTTP $code）。密钥错、robot 被吊销或轮换过 —— 找运维查 robot 状态" ;;
  404) warn "repository ${PROJECT}/${APP} 还不存在 —— 这是第一次推它。确认 App 名没写错：$APP" ;;
  000) die "请求失败（超时或 TLS）。若报 x509：服务端是正式证书，别加 --insecure-registry，先查本机 CA 库与系统时间" ;;
  *)   die "查 tag 列表失败（HTTP $code）：$(head -c 200 "$BODY")" ;;
esac

TAGS=""
[[ "$code" == 200 ]] && TAGS="$(extract_tags < "$BODY")"
TAG_COUNT=0; [[ -n "$TAGS" ]] && TAG_COUNT="$(printf '%s\n' "$TAGS" | wc -l | tr -d ' ')"

if [[ -n "$TAGS" ]] && printf '%s\n' "$TAGS" | grep -qx -- "$TAG"; then
  if [[ $FORCE_TAG -eq 1 ]]; then
    warn "tag ${TAG} 已存在，--force-tag 覆盖。⚠️ 部署侧按镜像引用的变化触发重建，
   覆盖同一个 tag 不会触发重建 —— 线上还跑着旧的那份，且没有可回退的目标"
  else
    die "tag ${TAG} 已经存在。tag 是不可变的：换一个新 tag（推荐 git short sha 或版本号），
   真要覆盖加 --force-tag，但要先明白重建不会自动发生"
  fi
fi
ok "凭证可用；repository 现有 ${TAG_COUNT} 个 tag"

# ── 3. 保留策略预警（④：只留最近推送的 K 个）─────────────────────────
step "3/7 保留策略预警"
if [[ "$TAG_COUNT" -ge "$RETAIN_K" ]]; then
  warn "已有 ${TAG_COUNT} 个 tag，保留策略只留**最近推送的 ${RETAIN_K} 个**（每天 02:00 执行，
   周日 03:00 GC）。判据是「最近 push 的」不是「最近在用的」——
   再推下去会把更老的挤掉，包括**生产可能正在跑的那个**。
   当时没有任何征兆，等到 Pod 重调度/扩容那天才 ImagePullBackOff。
   ⇒ 确认生产在跑的 tag 不在最老的几个里；调试镜像不要往生产 repo 推"
  log "现有 tag：$(printf '%s\n' "$TAGS" | tr '\n' ' ')"
else
  ok "${TAG_COUNT}/${RETAIN_K} —— 离保留上限还有空间"
fi

if [[ $DRY_RUN -eq 1 ]]; then
  step "dry-run 结束"
  log "接下来会做：docker login → docker build --platform ${PLATFORM} → 架构核对 → docker push"
  log "完整引用：${IMAGE}"
  exit 0
fi

# ── 4. login ────────────────────────────────────────────────────────
step "4/7 docker login"
printf '%s' "$ROBOT_SECRET" | docker login "$REGISTRY" -u "$ROBOT_USER" --password-stdin >/dev/null \
  || die "docker login 失败"
LOGGED_IN=1
cleanup() {
  rm -rf "$TMP_DIR"
  if [[ "${LOGGED_IN:-0}" == 1 && $KEEP_LOGIN -eq 0 ]]; then
    docker logout "$REGISTRY" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT
ok "已登录 ${REGISTRY}（结束时会自动 logout，别把凭证留在共享 runner 上）"

# ── 5. build（②：锁 amd64）──────────────────────────────────────────
if [[ $DO_BUILD -eq 1 ]]; then
  step "5/7 docker build --platform ${PLATFORM}"
  build_args=(build --platform "$PLATFORM" -t "$IMAGE")
  [[ -n "$DOCKERFILE" ]] && build_args+=(-f "$DOCKERFILE")
  build_args+=("$CONTEXT")
  docker "${build_args[@]}" || die "构建失败 —— 先修 Dockerfile，跟仓库无关"
  ok "构建完成"
else
  step "5/7 跳过构建（--no-build）"
  docker image inspect "$IMAGE" >/dev/null 2>&1 || die "本地没有 ${IMAGE} —— 去掉 --no-build，或先 docker tag"
fi

# ── 6. 架构核对 ─────────────────────────────────────────────────────
step "6/7 架构核对"
actual="$(docker image inspect "$IMAGE" --format '{{.Os}}/{{.Architecture}}')"
if [[ "$actual" != "$PLATFORM" ]]; then
  die "镜像实际是 ${actual}，要求 ${PLATFORM}。
   生产机都是 x86_64：架构不对的镜像**推得上去、拉得下来、容器起不来**（exec format error），
   看着像仓库坏了其实不是。用 --platform ${PLATFORM} 重新构建"
fi
ok "$actual"

# ── 7. push + 验证 ──────────────────────────────────────────────────
step "7/7 docker push"
docker push "$IMAGE" || die "推送失败。中断的话直接重推即可（已传上去的层会跳过）；
   若表现是一直 Retrying / 速度只有几十 KB/s，是 runner 在境外，换机器，重试没用；
   ⚠️ 别反复中断大镜像 —— 每次中断会在对象存储留下未完成分片"

code="$(harbor_get "/v2/${PROJECT}/${APP}/tags/list?n=1000")"
if [[ "$code" == 200 ]] && extract_tags < "$BODY" | grep -qx -- "$TAG"; then
  ok "服务端已确认 ${TAG} 存在"
else
  warn "推送命令成功，但服务端 tag 列表里暂时没查到（HTTP $code）—— 稍等重查，仍没有就找运维"
fi

digest="$(docker image inspect "$IMAGE" --format '{{if .RepoDigests}}{{index .RepoDigests 0}}{{end}}' 2>/dev/null || true)"

printf '\n%s══ 完成 ══%s\n' "$B" "$N"
log "镜像引用  ${IMAGE}"
[[ -n "$digest" ]] && log "digest    ${digest}"
log "架构      ${actual}"
log "tag 数    $((TAG_COUNT + 1))/${RETAIN_K}"
log ""
log "部署由部署侧侧触发。首次接入还要把**当前生产在跑的那个 tag** 也原样推进来，"
log "否则部署侧切换镜像地址的那一刻拉不到同名 tag。"
