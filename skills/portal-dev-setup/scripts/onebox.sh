#!/usr/bin/env bash
# portal-dev-setup — 在【你自己 App 的 repo 里】起一个真实门户（一盒 / one-box）用于本地联调。
#
#   onebox.sh init --image <门户镜像tag> --key <你的listingKey>
#                              首次用：登录仓库并拉门户镜像 → 从镜像里取出 compose 资产 →
#                              生成 compose.env（挑空闲端口、随机密钥），落在 ./portal-onebox/。
#                              已存在的 compose.env 不覆盖（要整份重来加 --force）。
#   onebox.sh pull             只（重）拉门户镜像 —— 换版本时用
#   onebox.sh env              生效的关键 env + 配置告警（同键多值、端口、NODE_ENV…）
#   onebox.sh status           env 摘要 + 容器状态 + 宿主侧健康探测
#   onebox.sh smoke            只跑宿主侧健康探测
#   onebox.sh chain            打印它将要用的那串 compose 参数
#   onebox.sh dc <args...>     docker compose <拼好的参数> <args...>
#
# 【拉取凭证】镜像仓库不开放匿名拉取，需要一份只读 puller 凭证。按顺序找第一个存在的：
#   $XGENT_REGISTRY_CONFIG → ./.xgent-registry.env → ${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env
#   → ~/.xgent-registry.env → <skill 目录>/registry.env
# KEY=value 两项：REGISTRY（域名，不带协议）· PULLER_AUTH（base64 的「用户名:口令」）。
# 同名环境变量优先，CI 里注入即可不必落盘。模板：<skill 目录>/puller.env.example
# 两个镜像本地都已有（比如 docker load 的离线 tar）时会跳过登录与拉取。
#
# 【一盒目录】放 docker-compose.yml 与 compose.env 的那个，按序查找：
#   $XGENT_ONEBOX_HOME → ./portal-onebox → ./deploy → <git 根>/portal-onebox → <git 根>/deploy
# 判据是同时有 docker-compose.yml 与 onebox/docker-compose.onebox.yml —— 你自己 repo 里
# 那个装别的东西的 deploy/ 不会被误认。
set -euo pipefail

c_red=$'\033[31m'; c_grn=$'\033[32m'; c_ylw=$'\033[33m'; c_dim=$'\033[2m'; c_off=$'\033[0m'
warn() { printf '%s! %s%s\n' "$c_ylw" "$*" "$c_off"; }
info() { printf '%s>%s %s\n' "$c_dim" "$c_off" "$*"; }
die()  { printf '%sERROR: %s%s\n' "$c_red" "$*" "$c_off" >&2; exit 1; }

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# --- 镜像仓库凭证 -------------------------------------------------------------
# 仓库域名与口令都不写在这个 skill 里（域名属内部信息，口令是密钥）。与仓内推镜像那套
# 用同一份 .xgent-registry.env，只是这里读的是【只读】的 puller 那两项。
load_registry_config() {
  local c f key val
  if [ -z "${REGISTRY_CONFIG:-}" ]; then
    for c in "${XGENT_REGISTRY_CONFIG:-}" "$PWD/.xgent-registry.env" \
             "${XDG_CONFIG_HOME:-$HOME/.config}/xgent/registry.env" \
             "$HOME/.xgent-registry.env" "$SKILL_DIR/registry.env"; do
      [ -n "$c" ] && [ -f "$c" ] && { REGISTRY_CONFIG="$c"; break; }
    done
  fi
  f="${REGISTRY_CONFIG:-}"
  [ -n "$f" ] && [ -f "$f" ] || return 0
  while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in ''|'#'*) continue ;; esac
    key="${line%%=*}"; val="${line#*=}"
    case "$key" in *[!A-Za-z0-9_]*|"") continue ;; esac
    val="${val%\"}"; val="${val#\"}"; val="${val%\'}"; val="${val#\'}"
    case "$key" in
      REGISTRY|PROJECT|PULLER_AUTH) [ -n "${!key:-}" ] || printf -v "$key" '%s' "$val" ;;
    esac
  done < "$f"
  # base64 不是加密，同组可读就等于把口令摊在那儿
  if [ -n "${PULLER_AUTH:-}" ]; then
    case "$(ls -l "$f" | cut -c1-10)" in ??????---*) ;; *) warn "$f 含 PULLER_AUTH 且同组/其他人可读 —— chmod 600 $f";; esac
  fi
}

require_puller() {
  load_registry_config
  if [ -z "${REGISTRY:-}" ] || [ -z "${PULLER_AUTH:-}" ]; then
    die "$(cat <<MSG
没有 puller 凭证，拉不到门户镜像 —— 仓库不开放匿名拉取，一盒起不来。

先找【你们自己的开发团队】要一份 puller key：这是一对只读拉取凭证，按团队发放，
不是谁的个人密码（一个团队共用一份，有人离职要找他们轮换）。拿到后：

  cp $SKILL_DIR/puller.env.example ./.xgent-registry.env
  chmod 600 ./.xgent-registry.env

填两项：
  REGISTRY=<仓库域名>        # 不带 https://、不带端口、无尾斜杠
  PULLER_AUTH=<base64>       # base64 的「用户名:口令」

PULLER_AUTH 就是 ~/.docker/config.json 里 auths.<REGISTRY>.auth 那个值；
自己算：  printf '%s' '<用户名>:<口令>' | base64

查找顺序：\$XGENT_REGISTRY_CONFIG → ./.xgent-registry.env →
          \${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env → ~/.xgent-registry.env →
          $SKILL_DIR/registry.env
同名环境变量优先，CI 里注入 REGISTRY / PULLER_AUTH 即可，不必落盘。

⚠️ base64 不是加密，base64 -d 一条命令就还原成明文：别提交进 git，别转发出团队。
   已经拿到离线镜像 tar 的，先 docker load 两个镜像，本脚本会跳过拉取这一步。
MSG
)"
  fi
  case "$REGISTRY" in *://*) die "REGISTRY 不要带协议（去掉 http:// 或 https://）：$REGISTRY";; esac
  case "$REGISTRY" in */*) die "REGISTRY 只写域名，别带项目名：$REGISTRY";; esac
  local dec
  dec="$(printf '%s' "$PULLER_AUTH" | base64 -d 2>/dev/null || printf '%s' "$PULLER_AUTH" | base64 -D 2>/dev/null || true)"
  case "$dec" in
    *:*) PULLER_USER="${dec%%:*}"; PULLER_SECRET="${dec#*:}" ;;
    *) die "PULLER_AUTH 不是 base64 的「用户名:口令」。自己算一遍：printf '%s' '<用户名>:<口令>' | base64" ;;
  esac
}

registry_login() {
  info "登录 ${REGISTRY}（用户 ${PULLER_USER}，只读）…"
  printf '%s' "$PULLER_SECRET" | docker login "$REGISTRY" -u "$PULLER_USER" --password-stdin >/dev/null 2>&1 \
    || die "docker login $REGISTRY 失败。口令可能已被轮换 —— 先 docker logout $REGISTRY 再让开发团队补一份新的 PULLER_AUTH。（陈旧凭证不会提示你去登录，只会一直 401。）"
}

# do_pull REF HINT —— docker pull 的 "What's Next" 提示走 stderr，>/dev/null 挡不住；
# 但 2>&1 会把真错误一起吞掉，所以收进临时文件、只在失败时回显。
do_pull() {
  local ref="$1" hint="$2" errf; errf="$(mktemp)"
  info "拉 ${ref} …"
  if ! docker pull "$ref" >/dev/null 2>"$errf"; then
    local msg; msg="$(tail -3 "$errf")"; rm -f "$errf"
    die "docker pull $ref 失败。${hint:+$hint
}$msg"
  fi
  rm -f "$errf"
}

# pull_images RUNTIME PROXY —— 两个镜像本地都在就跳过（离线 docker load 的场合）
pull_images() {
  local rt="$1" px="$2"
  if docker image inspect "$rt" >/dev/null 2>&1 && docker image inspect "$px" >/dev/null 2>&1; then
    info "两个镜像本地已有，跳过拉取（要强制重拉用 $0 pull --image ${rt}）"
    return 0
  fi
  local host="${rt%%/*}"
  case "$host" in
    *.*|*:*) ;;                       # 带域名的私有仓
    *) host="" ;;                     # 没有域名段 = docker hub
  esac
  require_puller
  if [ -n "$host" ] && [ "$host" != "$REGISTRY" ]; then
    die "镜像域名（${host}）与配置里的 REGISTRY（${REGISTRY}）对不上 —— 登录 A 却去 B 拉，必然 401。对齐这两个再来。"
  fi
  registry_login
  do_pull "$rt" "tag 写错了？这个仓库 tag 不可变、不接受 latest，以开发团队给的那行为准。"
  do_pull "$px" "代理镜像名是从 runtime 镜像推出来的（同仓同版本、仓库名 proxy），有的部署不叫这个 —— 用 --proxy-image 显式指定。"
}

# --- 定位一盒目录 -------------------------------------------------------------
find_home() {
  local d git_root
  if [ -n "${XGENT_ONEBOX_HOME:-}" ]; then printf '%s' "$XGENT_ONEBOX_HOME"; return; fi
  git_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
  for d in ./portal-onebox ./deploy ${git_root:+"$git_root/portal-onebox" "$git_root/deploy"}; do
    [ -f "$d/docker-compose.yml" ] && [ -f "$d/onebox/docker-compose.onebox.yml" ] && { (cd "$d" && pwd); return; }
  done
  printf ''
}

require_home() {
  HOME_DIR="$(find_home)"
  [ -n "$HOME_DIR" ] || die "没找到一盒目录（要有 docker-compose.yml + onebox/docker-compose.onebox.yml）。首次用先跑： $0 init --image <门户镜像tag> --key <你的listingKey>"
  ENV_FILE="$HOME_DIR/compose.env"
  [ -f "$ENV_FILE" ] || die "$HOME_DIR 里没有 compose.env。跑 $0 init 生成，或手工按 compose.env.example 拼。"
}

# --- env-file 解析（docker compose 是「后定义者胜」，这份文件是拼装出来的）--------
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

load_env() {
  APP_KEY="$(_eff APP_KEY)"; APP_IMAGE="$(_eff APP_IMAGE)"; APP_FRONTEND_DIST="$(_eff APP_FRONTEND_DIST)"
  CATALOG="$(_eff XGENT_APP_CATALOG files,ingest,llm-gateway,git)"
  HTTP_PORT="$(_eff HTTP_PORT 80)"; PROJECT="$(_eff COMPOSE_PROJECT_NAME xgent)"
  BASE_URL="http://localhost"; [ "$HTTP_PORT" = "80" ] || BASE_URL="http://localhost:$HTTP_PORT"
}

# --- compose 参数拼装 ---------------------------------------------------------
# app-dev.yml 里的 ${APP_IMAGE:?} / ${APP_KEY:?} 是【解析期】求值的（与 profile 无关），
# 所以还没填自己 App 那两行时必须整层不带，否则连 ps 都跑不了。
compose_args() {
  printf '%s\0' --env-file "$ENV_FILE" \
    -f "$HOME_DIR/docker-compose.yml" \
    -f "$HOME_DIR/onebox/docker-compose.onebox.yml"
  if [ -n "$APP_KEY" ] && [ -n "$APP_IMAGE" ]; then
    printf '%s\0' -f "$HOME_DIR/app-devkit/docker-compose.app-dev.yml"
    [ -n "$APP_FRONTEND_DIST" ] && printf '%s\0' -f "$HOME_DIR/app-devkit/docker-compose.app-frontend.yml"
    printf '%s\0' --profile app-external
  fi
  printf '%s\0' --profile local-infra
  local k; for k in ${CATALOG//,/ }; do printf '%s\0' --profile "app-$k"; done
}
dc() { local a=(); while IFS= read -r -d '' x; do a+=("$x"); done < <(compose_args); docker compose "${a[@]}" "$@"; }

# --- init ---------------------------------------------------------------------
port_free() { ! (exec 3<>"/dev/tcp/127.0.0.1/$1") >/dev/null 2>&1; }
pick_port() { local p; for p in "$@"; do port_free "$p" && { printf '%s' "$p"; return; }; done; printf '%s' "$1"; }
rand_hex() { openssl rand -hex 32 2>/dev/null || head -c32 /dev/urandom | od -An -tx1 | tr -d ' \n'; }

cmd_init() {
  local image="${XGENT_IMAGE:-}" proxy="" key="" home="./portal-onebox" force=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --image) image="${2:?}"; shift 2 ;;
      --proxy-image) proxy="${2:?}"; shift 2 ;;
      --key) key="${2:?}"; shift 2 ;;
      --home) home="${2:?}"; shift 2 ;;
      --force) force=1; shift ;;
      *) die "init: 不认识的参数 '$1'" ;;
    esac
  done
  [ -n "$image" ] || die "init 需要门户镜像 tag：--image <registry>/<项目>/one-box:<版本>。找平台团队要它和只读拉取账号；这个仓库 tag 不可变、不接受 latest。"
  [ -n "$proxy" ] || proxy="${image%/*}/proxy:${image##*:}"   # 代理镜像同仓同版本
  [ -n "$key" ] || warn "没给 --key，先用占位 your-app —— 起 app-backend 之前必须把 compose.env 里的 APP_KEY 改成你的 listingKey（它同时是 /svc/<key>、iframe 路径、scope 命名空间和令牌 aud）。"
  key="${key:-your-app}"

  # 先把路径算出来但【不建目录】—— 下面任何一步停下时都不该留一个空壳
  home="$(cd "$(dirname "$home")" 2>/dev/null && pwd || printf '%s' "$PWD")/$(basename "$home")"
  local envf="$home/compose.env"
  if [ -f "$envf" ] && [ "$force" = 0 ]; then
    die "$envf 已存在（里面是这台机器的密钥）。要整份重来加 --force；只想补配置就直接往它末尾加。"
  fi

  # 1) 备齐镜像 —— 没有 puller 凭证就在这里停下，而不是让人跑到一半才发现
  pull_images "$image" "$proxy"
  mkdir -p "$home"

  # 2) 从镜像里取 compose 资产 —— 版本天然与镜像对齐，不需要门户仓
  info "从 $image 取出 compose 资产…"
  local tmp cid; tmp="$(mktemp -d)"
  cid="$(docker create "$image")" || die "docker create 失败。先 docker login 到镜像仓库，并确认 tag 写对了。"
  docker cp "$cid:/app/deploy" "$tmp/deploy" >/dev/null || { docker rm -f "$cid" >/dev/null; die "镜像里没有 /app/deploy —— 这多半不是门户一盒镜像。"; }
  docker rm -f "$cid" >/dev/null
  # 只留联调用得上的：compose + 一盒增量 + devkit + postgres 建库脚本 + 两个 manifest 样例
  ( cd "$tmp/deploy" && rm -rf .DS_Store k8s oauth pm2 caddy knowledge-devkit onebox/Dockerfile \
      app-devkit/manifests/pagebuilder.manifest.json app-devkit/manifests/task-gateway.manifest.json 2>/dev/null || true )
  ( cd "$tmp/deploy" && tar -cf - . ) | ( cd "$home" && tar -xf - )
  rm -rf "$tmp"

  # 3) 挑空闲端口 —— 首次用最常见的翻车就是 80/5432/6379/9000 已被别的东西占着
  local hp hsp pgp rdp mnp mncp base
  hp="$(pick_port 80 8080 8090 18080)"; hsp="$(pick_port 443 8443 18443)"
  pgp="$(pick_port 5432 15432 25432)"; rdp="$(pick_port 6379 16379 26379)"
  mnp="$(pick_port 9000 19000 29000)"; mncp="$(pick_port 9001 19001 29001)"
  base="http://localhost"; [ "$hp" = "80" ] || base="http://localhost:$hp"

  # 4) 三段模板 + 本机覆盖块（覆盖块必须在最末尾）
  cat "$home/compose.env.example" "$home/onebox/compose.env.onebox.example" \
      "$home/app-devkit/compose.env.app.example" > "$envf"
  cat >> "$envf" <<EOF

# ============================================================================
# 本机覆盖 —— 由 onebox.sh init 生成。docker compose 是【后定义者胜】，所以这一段
# 必须留在文件最末尾；以后要改配置，也往这下面加，别回上面改。
# ============================================================================
COMPOSE_PROJECT_NAME=onebox-$key
NODE_ENV=development
DEV_MOCK_OAUTH=true

XGENT_IMAGE=$image
XGENT_PROXY_IMAGE=$proxy

# 宿主发布端口（init 挑的是当时空闲的口）。改了 HTTP_PORT 要同步下面两行。
HTTP_PORT=$hp
HTTPS_PORT=$hsp
POSTGRES_PORT=$pgp
REDIS_PORT=$rdp
MINIO_PORT=$mnp
MINIO_CONSOLE_PORT=$mncp
PORTAL_BASE_URL=$base
FILES_APP_URL=$base

# 门户自身密钥（本机随机，只在这台机器上有意义）
SESSION_SECRET=$(rand_hex)
TDT_SIGNING_KEY=$(rand_hex)
PLATFORM_ADMIN_KEY=$(rand_hex)
FILES_ENC_KEY=$(rand_hex)
LLM_GATEWAY_SECRET_ENC_KEY=$(rand_hex)

# files 的服务账号：种子把这里的值种进库，files-server 也读这里 —— 同源即一致。
# 用已知的 dev 明文，这样重跑种子/换机器都能对上（一盒是调试底座，不是门户）。
FILES_SA_CLIENT_SECRET=files-dev-resource-key
FILES_RESOURCE_KEY=files-dev-resource-key

# 一盒不装 ffmpeg：这一项必须留空，填 auto 会让每次转换去 exec 一个不存在的二进制。
PREVIEW_MEDIA_CONVERTER_URL=

# ---- 你的 App（填完 APP_IMAGE 才会起 app-backend）----
APP_KEY=$key
APP_IMAGE=
# micro（有前端、iframe 嵌入）才要，且必须是【绝对路径】、目录里有 index.html：
#APP_FRONTEND_DIST=/abs/path/to/your/frontend/dist
EOF

  # 5) 别让它进版本库：里面是密钥，且随时能从镜像重新生成
  local root; root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
  if [ -n "$root" ] && [ -f "$root/.gitignore" ] && ! grep -qx "$(basename "$home")/" "$root/.gitignore" 2>/dev/null; then
    printf '\n# 门户一盒本地联调栈（含密钥，可用 onebox.sh init 重新生成）\n%s/\n' "$(basename "$home")" >> "$root/.gitignore"
    info "已把 $(basename "$home")/ 加进 .gitignore"
  fi

  echo
  printf '%s一盒资产就位：%s%s\n' "$c_grn" "$home" "$c_off"
  printf '  门户地址   %s\n' "$base"
  printf '  端口       http=%s pg=%s redis=%s minio=%s/%s\n' "$hp" "$pgp" "$rdp" "$mnp" "$mncp"
  printf '  项目名     onebox-%s\n' "$key"
  echo
  echo "下一步（顺序本身是契约，见 SKILL.md §3）。先记一个短名："
  echo "  S=$0"
  echo
  echo "  1. 编辑 $envf 末尾：填 APP_IMAGE（micro 再加 APP_FRONTEND_DIST）"
  echo "  2. \$S dc up -d postgres redis minio"
  echo "  3. \$S dc run --rm portal-api bun run db:migrate"
  echo "  4. \$S dc run --rm portal-api bun run db:seed:onebox"
  echo "  5. \$S dc run --rm -v \"\$PWD/<放 manifest 的目录>:/devkit:ro\" portal-api bun run register-app /devkit/app.manifest.json"
  echo "  6. \$S dc up -d   然后  \$S smoke"
}

cmd_pull() {
  local image="${XGENT_IMAGE:-}" proxy=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --image) image="${2:?}"; shift 2 ;;
      --proxy-image) proxy="${2:?}"; shift 2 ;;
      *) die "pull: 不认识的参数 '$1'" ;;
    esac
  done
  if [ -z "$image" ] && [ -n "$(find_home)" ]; then          # 已 init 过就从 compose.env 取
    ENV_FILE="$(find_home)/compose.env"
    [ -f "$ENV_FILE" ] && { image="$(_eff XGENT_IMAGE)"; [ -n "$proxy" ] || proxy="$(_eff XGENT_PROXY_IMAGE)"; }
  fi
  [ -n "$image" ] || die "pull 需要 --image <门户镜像tag>（或先 init，之后它会从 compose.env 里读）。"
  [ -n "$proxy" ] || proxy="${image%/*}/proxy:${image##*:}"
  require_puller; registry_login
  do_pull "$image" "tag 写错了？这个仓库 tag 不可变、不接受 latest，以开发团队给的那行为准。"
  do_pull "$proxy" "代理镜像名是从 runtime 镜像推出来的，有的部署不叫 proxy —— 用 --proxy-image 指定。"
  printf '%s两个镜像就位%s\n' "$c_grn" "$c_off"
  echo "换版本的话，记得同步 compose.env 末尾的 XGENT_IMAGE / XGENT_PROXY_IMAGE，再 dc up -d"
}

# --- 其余子命令 ---------------------------------------------------------------
cmd_env() {
  echo "一盒目录:    $HOME_DIR"
  echo "compose.env: $ENV_FILE"
  echo
  local k v n
  for k in COMPOSE_PROJECT_NAME XGENT_IMAGE XGENT_PROXY_IMAGE XGENT_APP_CATALOG NODE_ENV DEV_MOCK_OAUTH \
           HTTP_PORT HTTPS_PORT POSTGRES_PORT REDIS_PORT MINIO_PORT MINIO_CONSOLE_PORT PORTAL_BASE_URL \
           APP_KEY APP_IMAGE APP_FRONTEND_DIST PREVIEW_MEDIA_CONVERTER_URL; do
    v="$(_eff "$k")"
    n="$(_assignments | awk -F'\t' -v k="$k" '$2==k{seen[$3]=1} END{print length(seen)}')"
    printf '  %-28s %s' "$k" "${v:-${c_dim}(空)${c_off}}"
    [ "$n" -gt 1 ] && printf '   %s← 同键 %s 个不同值，取最后一个%s' "$c_ylw" "$n" "$c_off"
    printf '\n'
  done
  echo
  [ "$PROJECT" = "xgent" ] && warn "COMPOSE_PROJECT_NAME 还是模板默认的 xgent —— 同机另一套 compose 会被当成同一项目（容器被接管、命名卷共享）。改成 onebox-<你的 listingKey>。"
  [ "$(_eff NODE_ENV)" = "development" ] || warn "NODE_ENV 不是 development —— 镜像烘的是 production，不覆盖则 portal-api 拒绝在 DEV_MOCK_OAUTH=true 下启动，register-app 也拒跑。"
  [ "$(_eff DEV_MOCK_OAUTH)" = "true" ] || warn "DEV_MOCK_OAUTH 不是 true —— 没有 dev 登录门，浏览器进不去。"
  [ -n "$(_eff PREVIEW_MEDIA_CONVERTER_URL)" ] && warn "PREVIEW_MEDIA_CONVERTER_URL 非空 —— 一盒不装 ffmpeg，非空会让每次转换 exec 一个不存在的二进制。留空。"
  case "$(_eff XGENT_IMAGE)" in *:latest) warn "XGENT_IMAGE 用了 latest —— 这个仓库 tag 不可变、不接受可变指针，写具体版本。";; esac
  [ -z "$APP_IMAGE" ] && warn "APP_IMAGE 还没填 —— 你的 app-backend 不会被拉起。（门户本身照常可用；先把门户跑通本来也是对的第一步。）"
  case "$(_eff PORTAL_BASE_URL)" in
    ""|"http://localhost:$HTTP_PORT") ;;
    "http://localhost") [ "$HTTP_PORT" = "80" ] || warn "PORTAL_BASE_URL 是 http://localhost 但 HTTP_PORT=$HTTP_PORT —— 浏览器侧的绝对链接会指错端口。";;
    *) warn "PORTAL_BASE_URL（$(_eff PORTAL_BASE_URL)）与 HTTP_PORT（${HTTP_PORT}）对不上。";;
  esac
  if [ -n "$APP_FRONTEND_DIST" ]; then
    [ -f "$APP_FRONTEND_DIST/index.html" ] || warn "APP_FRONTEND_DIST 下没有 index.html（${APP_FRONTEND_DIST}）—— iframe 会 404。要绝对路径、且指向构建产物目录。"
  fi
  return 0
}

cmd_smoke() {
  echo "宿主侧健康探测（${BASE_URL}）"
  local k keys="$CATALOG" tmp; tmp="$(mktemp)"
  case ",$CATALOG," in *",$APP_KEY,"*) ;; *) [ -n "$APP_KEY" ] && keys="$CATALOG,$APP_KEY";; esac
  _probe() { # _probe LABEL PATH
    local code body
    # curl 自己在失败时就打印 000；再 || echo 000 会拼成 000000
    code="$(curl -s -m 8 -o "$tmp" -w '%{http_code}' "$BASE_URL$2" 2>/dev/null || true)"
    code="${code:-000}"
    body="$(head -c 160 "$tmp" 2>/dev/null || true)"
    if [ "$code" = "200" ]; then printf '  %s✓%s %-22s %s\n' "$c_grn" "$c_off" "$1" "$body"
    else printf '  %s✗%s %-22s HTTP %s  %s\n' "$c_red" "$c_off" "$1" "$code" "$body"; fi
  }
  _probe "/health" /health
  for k in ${keys//,/ }; do _probe "/svc/$k/health" "/svc/$k/health"; done
  rm -f "$tmp"
  echo
  echo "${c_dim}404 = /svc 放行 map 没写成，或反代在写 map 之前就起了（跑 register-app，再 dc exec reverse-proxy caddy reload）${c_off}"
  echo "${c_dim}502 = 后端容器没起/崩了/没听 8080/网络别名 <key>-server 没命中（dc logs <key>-server 或 app-backend）${c_off}"
  echo "${c_dim}000 = 反代本身没起，或你探的端口不是 HTTP_PORT（dc ps / dc logs reverse-proxy）${c_off}"
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
  init)   shift; cmd_init "$@" ;;
  pull)   shift; cmd_pull "$@" ;;
  env)    require_home; load_env; cmd_env ;;
  smoke)  require_home; load_env; cmd_smoke ;;
  status) require_home; load_env; cmd_status ;;
  chain)  require_home; load_env; { while IFS= read -r -d '' x; do printf '%s ' "$x"; done < <(compose_args); echo; } ;;
  dc)     require_home; load_env; shift; dc "$@" ;;
  -h|--help|help) sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' ;;
  *)      die "未知子命令 '$1'。用 init | pull | env | status | smoke | chain | dc <args...>" ;;
esac
