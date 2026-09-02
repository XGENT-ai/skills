#!/usr/bin/env bash
# portal-dev-setup — 在【你自己 App 的 repo 里】起一个真实门户（一盒 / one-box）用于本地联调。
#
#   onebox.sh init --key <你的listingKey> [--image <门户镜像tag>]
#                              首次用：登录仓库并拉门户镜像 → 从镜像里取出 compose 资产 →
#                              生成 compose.env（挑空闲端口、随机密钥），落在 ./portal-onebox/。
#                              已存在的 compose.env 不覆盖（要整份重来加 --force）。
#                              不给 --image 就用 <REGISTRY>/<PROJECT>/one-box:latest —— 一盒
#                              和它的反代都挂了 latest，开新项目不用先去问 tag。
#   onebox.sh up               ★ 按顺序把整套铺起来（迁移→种子→各库→注册→起栈），幂等可重跑。
#                              顺序本身是契约（种子会 truncate、反代启动时才读 /svc map），
#                              这条命令就是为了不让人自己记它。
#   onebox.sh doctor           ★ 体检：把排查表里能自动判的都判一遍，每条给一行可直接粘的修法。
#                              跑不通先跑它，别翻文档。
#   onebox.sh add <key>        ★ 把一个平台侧 App 拉进来陪调（如 omni-parser 多模态解析）：
#                              注册清单 + 建库 + 起容器。非破坏性、幂等。
#   onebox.sh pull             只（重）拉门户镜像 —— 换版本、或 latest 移动过之后用
#   onebox.sh env              生效的关键 env + 配置告警（同键多值、端口、NODE_ENV…）
#   onebox.sh status           env 摘要 + 容器状态 + 宿主侧健康探测
#   onebox.sh smoke            只跑宿主侧健康探测
#   onebox.sh chain            打印它将要用的那串 compose 参数
#   onebox.sh dc <args...>     docker compose <拼好的参数> <args...>
#
# 【拉取凭证】镜像仓库不开放匿名拉取，需要一份只读 puller 凭证。按顺序找第一个存在的：
#   $XGENT_REGISTRY_CONFIG → ./.xgent-registry.env → ${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env
#   → ~/.xgent-registry.env → <skill 目录>/registry.env
# KEY=value 三项：REGISTRY（域名，不带协议）· PULLER_AUTH（base64 的「用户名:口令」）·
# ONEBOX_PROJECT（【门户一盒镜像】所在的项目名 —— 省掉 --image 时用它拼
#   <REGISTRY>/<ONEBOX_PROJECT>/one-box:latest）。⚠️ 它多半【不等于】PROJECT：后者是
#   xgent-image-push 用来推【你自己 App 镜像】的项目，两者同仓不同项目。缺 ONEBOX_PROJECT
#   时才回退到 PROJECT（老配置兼容），拿它拼出来的引用十有八九 not found。
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
      REGISTRY|PROJECT|ONEBOX_PROJECT|PULLER_AUTH|MANIFEST_STORE|MANIFEST_STORE_READ_TOKEN) [ -n "${!key:-}" ] || printf -v "$key" '%s' "$val" ;;
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
  do_pull "$rt" "两种可能，按顺序查：① 项目名不对 —— 一盒镜像与你自己 App 的镜像【不在同一个项目】下（配 ONEBOX_PROJECT，或直接 --image 给全）；② tag 写错 —— 一盒只有 :latest（可变指针）和 :v<版本>-<7位sha>（钉住某一版）两种。"
  do_pull "$px" "代理镜像名是从 runtime 镜像推出来的（同仓同版本、仓库名 proxy），有的部署不叫这个 —— 用 --proxy-image 显式指定。"
}

# 不给 --image 时用的默认镜像。一盒与它的反代都挂了 latest 指针 —— 这类镜像是【给人手工
# docker pull】的调试件，没有"部署侧按镜像引用触发重建"那条链，可变 tag 的代价不成立，
# 而"新人不知道该用哪个 tag"这个洞是真的。要钉住某一版就显式 --image <...>:v<版本>-<sha>。
default_image() {
  load_registry_config
  # ⚠️ 一盒镜像与【你自己 App 的镜像】不在同一个项目下。ONEBOX_PROJECT 是前者；
  # PROJECT 是后者（xgent-image-push 推镜像用的那个）。只有 ONEBOX_PROJECT 缺席时才
  # 回退到 PROJECT —— 那是老配置的兼容路径，拼出来的引用多半 not found，所以回退时
  # 明说一句，别让人对着一个指错方向的 404 猜 tag。
  local proj="${ONEBOX_PROJECT:-}"
  if [ -z "$proj" ] && [ -n "${PROJECT:-}" ]; then
    proj="$PROJECT"
    warn "配置里没有 ONEBOX_PROJECT，回退用 PROJECT=${PROJECT} 拼一盒镜像 —— 那通常是【推你自己 App 镜像】的项目，不是一盒所在的项目。拉不到就去问开发团队要一盒的项目名，填进 ONEBOX_PROJECT。"
  fi
  [ -n "${REGISTRY:-}" ] && [ -n "$proj" ] || return 1
  printf '%s/%s/one-box:latest' "$REGISTRY" "$proj"
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
  HTTP_PORT="$(_eff HTTP_PORT 80)"; COMPOSE_PROJECT="$(_eff COMPOSE_PROJECT_NAME xgent)"
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
  [ -n "$image" ] || image="$(default_image || true)"
  [ -n "$image" ] || die "init 不知道该拉哪个镜像：给 --image <registry>/<一盒项目>/one-box:<版本>，或在 puller 配置里补上 REGISTRY 与 ONEBOX_PROJECT（补了就默认用 <REGISTRY>/<ONEBOX_PROJECT>/one-box:latest，不必再去问 tag）。⚠️ ONEBOX_PROJECT 是【一盒镜像】的项目，不是你推自己 App 镜像的那个 PROJECT。"
  case "$image" in *:latest) info "用 ${image}（可变指针）—— 它跟着最新一版走，开工前 $0 pull 一次就是最新；要钉住某一版改用 --image <...>:v<版本>-<sha>。";; esac
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

  # 门户的 /health 报的 version 取自 APP_VERSION 环境变量，compose 缺省填 "dev" ——
  # 于是一盒对使用者【完全不自报版本】。而「照文档做但行为不符」十有八九就是镜像比文档旧
  # （CR-4：自动授予租户那段代码比某一版镜像新一天，症状是市场里没有卡片且零报错）。
  # 这里把 tag + 镜像 ID + 构建日期钉进 compose.env，`/health` 与 `status` 立刻能回答
  # 「我跑的是哪一版」。取不到就留空，不编。
  local img_id img_created ver=""
  img_id="$(docker image inspect "$image" --format '{{.Id}}' 2>/dev/null | cut -c8-14 || true)"
  img_created="$(docker image inspect "$image" --format '{{.Created}}' 2>/dev/null | cut -c1-10 || true)"
  # 取不到镜像元信息（离线 load 的、docker 不认的）就留空 —— 宁可不自报，也不编一个版本号。
  if [ -n "$img_id" ]; then ver="onebox ${image##*:} ${img_id}${img_created:+ built ${img_created}}"; fi

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
# 这台一盒的身份（/health 的 version 字段读它）。换镜像后重跑 init，或自己改这一行。
APP_VERSION=$ver

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
  [ -n "$image" ] || image="$(default_image || true)"
  [ -n "$image" ] || die "pull 不知道该拉哪个镜像：给 --image <门户镜像tag>，或先 init（之后它会从 compose.env 里读），或在 puller 配置里补上 REGISTRY 与 ONEBOX_PROJECT（默认 one-box:latest）。"
  [ -n "$proxy" ] || proxy="${image%/*}/proxy:${image##*:}"
  require_puller; registry_login
  do_pull "$image" "tag 写错了？一盒只有两种：:latest（可变指针，跟着最新一版走）和 :v<版本>-<7位sha>（钉住某一版）。"
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
  [ "$COMPOSE_PROJECT" = "xgent" ] && warn "COMPOSE_PROJECT_NAME 还是模板默认的 xgent —— 同机另一套 compose 会被当成同一项目（容器被接管、命名卷共享）。改成 onebox-<你的 listingKey>。"
  [ "$(_eff NODE_ENV)" = "development" ] || warn "NODE_ENV 不是 development —— 镜像烘的是 production，不覆盖则 portal-api 拒绝在 DEV_MOCK_OAUTH=true 下启动，register-app 也拒跑。"
  [ "$(_eff DEV_MOCK_OAUTH)" = "true" ] || warn "DEV_MOCK_OAUTH 不是 true —— 没有 dev 登录门，浏览器进不去。"
  [ -n "$(_eff PREVIEW_MEDIA_CONVERTER_URL)" ] && warn "PREVIEW_MEDIA_CONVERTER_URL 非空 —— 一盒不装 ffmpeg，非空会让每次转换 exec 一个不存在的二进制。留空。"
  case "$(_eff XGENT_IMAGE)" in *:latest) info "XGENT_IMAGE 是 latest（可变指针）—— 本地已经有同名镜像时 compose 不会回仓库看一眼，开工前先 $0 pull；要钉住某一版就写成 :v<版本>-<sha>。";; esac
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

  # dev 登录入口。登录页那枚「本地开发账号」按钮由 /auth/providers 的 dev 字段驱动 ——
  # 字段在而按钮不在，就是这版镜像的前端与 API 对不齐（换镜像，或直接走 /auth/dev/start）；
  # 字段不在就是 DEV_MOCK_OAUTH 没生效。此前这一步只能靠读门户源码才知道（CR-4 #4）。
  curl -s -m 8 -o "$tmp" "$BASE_URL/auth/providers" 2>/dev/null || true
  if grep -q '"dev"' "$tmp" 2>/dev/null; then
    printf '  %s✓%s %-22s %s\n' "$c_grn" "$c_off" "dev 登录" "登录页的「本地开发账号」按钮；按钮没出现就直接开 ${BASE_URL}/auth/dev/start"
  else
    printf '  %s✗%s %-22s %s\n' "$c_red" "$c_off" "dev 登录" "/auth/providers 里没有 dev —— DEV_MOCK_OAUTH 没生效（compose.env 末尾要有 NODE_ENV=development + DEV_MOCK_OAUTH=true，改完 dc up -d portal-api）"
  fi
  rm -f "$tmp"
  echo
  echo "${c_dim}404 = /svc 放行 map 没写成，或反代在写 map 之前就起了（跑 register-app，再 dc exec reverse-proxy caddy reload）${c_off}"
  echo "${c_dim}502 = 后端容器没起/崩了/没听 8080/网络别名 <key>-server 没命中（dc logs <key>-server 或 app-backend）${c_off}"
  echo "${c_dim}000 = 反代本身没起，或你探的端口不是 HTTP_PORT（dc ps / dc logs reverse-proxy）${c_off}"
  return 0
}

cmd_status() {
  cmd_env
  echo "容器（project: ${COMPOSE_PROJECT}）"
  dc ps || true
  echo
  warn "portal-api / *-server 的 unhealthy 是假红：一盒镜像没装 curl，而 healthcheck 写的就是 curl —— 它永远失败。判活只看下面的探测。"
  echo
  cmd_smoke
}

# --- 小工具 -------------------------------------------------------------------
_env_set() { # _env_set KEY VALUE —— 追加到 compose.env 末尾（后定义者胜），已是同值则跳过
  local k="$1" v="$2"
  [ "$(_eff "$k")" = "$v" ] && return 0
  printf '%s=%s\n' "$k" "$v" >> "$ENV_FILE"
  info "compose.env += ${k}=${v}"
}
_svc_running() { [ -n "$(dc ps -q "$1" 2>/dev/null || true)" ]; }
_ok()   { printf '  %s✓%s %s\n' "$c_grn" "$c_off" "$*"; }
_bad()  { printf '  %s✗%s %s\n' "$c_red" "$c_off" "$1"; shift; [ $# -gt 0 ] && printf '      %s↳ %s%s\n' "$c_dim" "$*" "$c_off"; DOCTOR_BAD=$((DOCTOR_BAD+1)); return 0; }
_note() { printf '      %s↳ %s%s\n' "$c_dim" "$*" "$c_off"; }

# --- up：按顺序铺一遍（幂等）---------------------------------------------------
cmd_up() {
  [ -n "$APP_KEY" ] || die "compose.env 里还没有 APP_KEY —— 先跑 $0 init --key <你的listingKey>。"
  info "① 基础设施（postgres / redis / minio）"
  dc up -d postgres redis minio
  info "   等 postgres 就绪…"
  local i=0; until dc exec -T postgres pg_isready -U postgres >/dev/null 2>&1; do
    i=$((i+1)); [ "$i" -gt 60 ] && die "postgres 60s 还没就绪，看 $0 dc logs postgres"
    sleep 1
  done
  info "② 门户库迁移 + 一盒种子（★ 破坏性：truncate cascade）"
  dc run --rm portal-api bun run db:migrate
  dc run --rm portal-api bun run db:seed:onebox
  info "③ 目录里各服务自己的库"
  local k
  for k in ${CATALOG//,/ }; do
    case "$k" in files|ingest|llm-gateway|git) dc run --rm portal-api bun run "db:$k:migrate" || warn "db:$k:migrate 失败，继续";; esac
  done
  info "③b 你自己的库 xgent-${APP_KEY}"
  _ensure_db "xgent-${APP_KEY}"
  if [ -n "${ONEBOX_MANIFEST:-}" ] || [ -f "app.manifest.json" ]; then
    local mf="${ONEBOX_MANIFEST:-app.manifest.json}"
    info "④ 注册你的 App（${mf}）"
    dc run --rm -v "$(cd "$(dirname "$mf")" && pwd):/devkit:ro" portal-api bun run register-app "/devkit/$(basename "$mf")" \
      || warn "register-app 失败 —— 看上面的报错；改完重跑 $0 up 即可（幂等）"
  else
    warn "④ 跳过 register-app：当前目录没有 app.manifest.json。有的话设 ONEBOX_MANIFEST=<路径> 再跑一次 $0 up。"
  fi
  info "⑤ 起门户三件套 + 基础服务" ; dc up -d
  echo; info "跑一次体检："; cmd_doctor
}

_ensure_db() { # _ensure_db <dbname>
  if dc exec -T postgres psql -U postgres -lqt 2>/dev/null | cut -d'|' -f1 | grep -qw "$1"; then
    info "   库 $1 已存在"
  else
    dc exec -T postgres psql -U postgres -c "CREATE DATABASE \"$1\"" >/dev/null && info "   建库 $1"
  fi
}

# --- add：把一个平台侧 App 拉进来陪调 ------------------------------------------
# 关键设计：compose 片段【从 manifest 生成】，不是每个 App 在 skill 里写一份 YAML —— 那样
# 每接一个新 App 就得改一次 skill，等于把问题搬了个地方。manifest 里已经有全部所需：
#   deployDescriptor.image → 镜像（相对名按 <REGISTRY>/<ONEBOX_PROJECT>/ 补全，版本由清单钉住）
#   deployDescriptor.port  → 容器内端口        listingKey → 网络别名 <key>-server + 库名 xgent-<key>
#   serviceAccount.clientId→ 自省身份          type       → micro 要不要前端
# 密钥【不从 manifest 来】：一盒现生成一个，同时写进本地门户 DB 与容器 env。所以清单里
# 有没有明文密钥都无所谓（release 提交的那份本来就被 prod 模式拒收）。
#
# 少数 App 有 manifest 描述不了的自带 infra（knowledge 要一个 Chroma 边车）。那种情况下
# skill 的 services/<key>.extra.yml 补一层——只有例外才需要文件，不是每个 App 都要。
_manifest_json() { # _manifest_json KEY  → 把 manifest JSON 打到 stdout
  local key="$1"
  if [ -n "${ADD_MANIFEST:-}" ]; then cat "$ADD_MANIFEST"; return; fi
  if [ -n "${ADD_FROM:-}" ]; then
    # ⚠️ 门户【所有】200 都是信封 {ok,data} —— manifest 在 data.manifest，不在响应根。
    #    把响应体直接当 manifest 解，症状是「字段全 undefined」而不是「解析失败」，更难查。
    # 头必须走数组：`${X:+-H "a: b"}` 不加引号会按空白拆成四个参数（连引号一起）。
    local http body; local -a hdr=()
    if [ -n "${MANIFEST_STORE_READ_TOKEN:-}" ]; then hdr=(-H "Authorization: Bearer ${MANIFEST_STORE_READ_TOKEN}"); fi
    # 刻意不加 -f：4xx 的响应体正是要读的那一份（业务失败走 200 + 错误体，闸走 4xx）。
    body="$(curl -sS -m 20 -w $'\n%{http_code}' ${hdr[@]+"${hdr[@]}"} \
      "${ADD_FROM%/}/api/market/catalog/${key}" 2>/dev/null)" \
      || die "连不上 ${ADD_FROM} —— 检查地址与网络，或用 --manifest <本地文件> 顶上。"
    http="${body##*$'\n'}"; body="${body%$'\n'*}"
    case "$http" in
      404) die "${ADD_FROM} 没有目录接口（那个门户没开目录能力，或地址不对）。出路：--manifest <本地文件>，或不带 --from 用镜像自带的样例清单。" ;;
      401) die "目录读取令牌无效或已失效 —— 在 .xgent-registry.env 里更新 MANIFEST_STORE_READ_TOKEN（找门户团队要新的）。" ;;
      200) ;;
      *)   die "目录返回 HTTP ${http}。出路：--manifest <本地文件>，或不带 --from 用镜像自带的样例清单。" ;;
    esac
    # data:null = 目录里还没有这个 key；ok:false = 业务失败。
    # 出错信息走【stdout】：`dc run` 的 stderr 混着 compose 自己的噪音，捞不干净。
    local mout rc
    mout="$(printf '%s' "$body" | dc run --rm --no-deps -T portal-api bun -e '
      let raw=""; for await (const c of Bun.stdin.stream()) raw += new TextDecoder().decode(c);
      let r; try { r = JSON.parse(raw); } catch { console.log("目录返回的不是 JSON"); process.exit(3); }
      if (r?.ok === false) { console.log(r?.error?.message ?? "目录拒绝了这次请求"); process.exit(4); }
      if (r?.data == null) process.exit(5);
      if (!r?.data?.manifest) { console.log("目录条目里没有 manifest 字段"); process.exit(3); }
      console.log(JSON.stringify(r.data.manifest));
    ' 2>/dev/null)"; rc=$?
    local outs="出路：--manifest <本地文件>，或不带 --from 用镜像自带的样例清单。"
    case "$rc" in
      0) printf '%s' "$mout"; return ;;
      5) die "目录里还没有 ${key}（那个 App 还没发过版？）。${outs}" ;;
      4) die "目录拒绝了这次请求：${mout}。${outs}" ;;
      *) die "读取目录条目失败：${mout:-未知错误}。${outs}" ;;
    esac
  fi
  # 默认：一盒镜像自带的那批样例清单
  dc run --rm --no-deps -T portal-api cat "/app/deploy/app-devkit/manifests/${key}.manifest.json" 2>/dev/null \
    || die "这版一盒镜像里没有 ${key} 的清单。三条出路：① ${0} pull 换新镜像；② --manifest <你手上那份 app.manifest.json>；③ --from <在线门户地址> 从平台目录拉。"
}

cmd_add() {
  local key="" want_image=""
  ADD_MANIFEST=""; ADD_FROM=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --image)    want_image="${2:-}"; shift 2 ;;
      --manifest) ADD_MANIFEST="${2:-}"; shift 2 ;;
      --from)     ADD_FROM="${2:-}"; shift 2 ;;
      -*) die "未知参数 $1（支持 --image <ref> / --manifest <文件> / --from <门户地址>）" ;;
      *) key="$1"; shift ;;
    esac
  done
  [ -n "$key" ] || die "用法：${0} add <key> [--image <ref>] [--manifest <文件>] [--from <门户地址>]"
  # 清单来源优先级：--manifest <文件> > --from / MANIFEST_STORE > 镜像自带样例。
  # 配了 MANIFEST_STORE 就不必每次敲 --from（地址与只读令牌都在 .xgent-registry.env 里）。
  load_registry_config || true
  [ -n "${ADD_FROM:-}" ] || ADD_FROM="${MANIFEST_STORE:-}"

  info "① 取清单"
  local mj; mj="$(_manifest_json "$key")"
  [ -n "$mj" ] || die "清单是空的。"

  # 用 portal-api 里的 bun 解析（不对宿主机的 python/jq 做任何假设）
  local fields
  fields="$(printf '%s' "$mj" | dc run --rm --no-deps -T portal-api bun -e '
    let raw=""; for await (const c of Bun.stdin.stream()) raw += new TextDecoder().decode(c);
    const m = JSON.parse(raw); const d = m.deployDescriptor ?? {};
    const q = (v) => String(v ?? "").replace(/\n/g, " ");
    console.log(`M_KEY=${q(m.listingKey)}`);
    console.log(`M_TYPE=${q(m.type)}`);
    console.log(`M_IMAGE=${q(d.image)}`);
    console.log(`M_PORT=${q(d.port ?? 8080)}`);
    console.log(`M_HEALTH=${q(d.healthPath ?? "/health")}`);
    console.log(`M_SA=${q((m.serviceAccount ?? {}).clientId)}`);
    console.log(`M_ENV=${(m.requiredEnv ?? []).map((e) => (typeof e === "string" ? e : e.key)).join(" ")}`);
    // 非空 ⇒ 它是跨应用交换的【发起方】，需要一把 App Secret（下面与 SA 密钥同源生成注入）。
    console.log(`M_XTARGETS=${(m.exchangeTargets ?? []).join(" ")}`);
  ' 2>/dev/null)" || die "解析清单失败 —— 它可能不是一份合法的 app.manifest.json。"
  local M_KEY="" M_TYPE="" M_IMAGE="" M_PORT="" M_HEALTH="" M_SA="" M_ENV="" M_XTARGETS=""
  eval "$(printf '%s' "$fields" | sed 's/^\([A-Z_]*\)=\(.*\)$/\1="\2"/')"
  [ "$M_KEY" = "$key" ] || die "清单里的 listingKey 是「${M_KEY}」，与你要加的「${key}」对不上。"
  info "   ${M_KEY}  type=${M_TYPE}  port=${M_PORT}  sa=${M_SA}"

  # ② 镜像：清单里的相对名按 <REGISTRY>/<ONEBOX_PROJECT>/ 补全（版本由清单钉住，可复现）
  local ivar ref; ivar="$(printf '%s' "$key" | tr 'a-z-' 'A-Z_')_IMAGE"
  ref="${want_image:-$(_eff "$ivar")}"
  if [ -z "$ref" ]; then
    [ -n "$M_IMAGE" ] || die "清单里没有 deployDescriptor.image，也没给 --image。"
    case "$M_IMAGE" in
      */*) ref="$M_IMAGE" ;;                       # 已经带项目/域名，原样用
      *) load_registry_config || true
         [ -n "${REGISTRY:-}" ] || die "还没配拉取凭证 —— 先按 SKILL.md §0 放一份 .xgent-registry.env。"
         ref="${REGISTRY}/${ONEBOX_PROJECT:-${PROJECT:-}}/${M_IMAGE}" ;;
    esac
  fi
  info "② 镜像 ${ref}"
  if docker image inspect "$ref" >/dev/null 2>&1; then info "   本地已有，跳过拉取"
  else
    require_puller; registry_login
    do_pull "$ref" "配合调试用的 App 都发在一盒同一个项目（ONEBOX_PROJECT=${ONEBOX_PROJECT:-未配}）下；确实不在就用 --image 给全引用。"
  fi
  _env_set "$ivar" "$ref"

  # ③ 密钥：一盒现生成，两侧同源（清单里有没有明文都不影响）
  local svar sec; svar="$(printf '%s' "$key" | tr 'a-z-' 'A-Z_')_SA_SECRET"
  sec="$(_eff "$svar")"; [ -n "$sec" ] || { sec="$(rand_hex | cut -c1-32)"; _env_set "$svar" "$sec"; }
  # 声明了 exchangeTargets 的 App 是跨应用交换的【发起方】，它在 /oauth/token 上用
  # `sk_<key>` + App Secret 换对方的令牌。目录里的清单按红线剥掉了 exchangeInitiatorSecret，
  # 而 register-app 的 dev 模式只在【清单里有这个字段】时才 wireInitiatorSecret ——
  # 不补这一把，该 App 的跨应用交换会稳定 401，且症状出现在「发起方」而不是这里。
  local avar asec=""; avar="$(printf '%s' "$key" | tr 'a-z-' 'A-Z_')_APP_SECRET"
  if [ -n "$M_XTARGETS" ]; then
    asec="$(_eff "$avar")"; [ -n "$asec" ] || { asec="$(rand_hex | cut -c1-32)"; _env_set "$avar" "$asec"; }
  fi

  info "④ 注册清单（非破坏性、幂等）"
  # register-app 不带 --prod 是 dev 模式，而 dev 模式【强制要求】serviceAccount.secret；
  # 目录里的清单按红线只留 clientId。把③那把注进去 —— 于是门户库里那把与容器 env 里那把
  # 才真的是同一把（这一条对镜像自带样例同样生效：它们自带的明文 dev secret 与容器 env
  # 里那把本来就不是同一把，只是没人验过自省）。临时文件用完即删。
  # 两把密钥走 stdin 的前两行，不走 `-e`：命令行参数同机 `ps` 可见。
  printf '%s\n%s\n%s' "$sec" "$asec" "$mj" | dc run --rm --no-deps -T portal-api bun -e '
    let raw=""; for await (const c of Bun.stdin.stream()) raw += new TextDecoder().decode(c);
    const a = raw.indexOf("\n"); const b = raw.indexOf("\n", a + 1);
    const secret = raw.slice(0, a);
    const appSecret = raw.slice(a + 1, b);
    const m = JSON.parse(raw.slice(b + 1));
    m.serviceAccount = { ...(m.serviceAccount ?? {}), secret };
    // 空 = 这个 App 不是交换发起方；写进去反而会让 register-app 白跑一次 wire。
    if (appSecret) m.exchangeInitiatorSecret = appSecret;
    console.log(JSON.stringify(m, null, 2));
  ' > "$HOME_DIR/.add-$key.manifest.json" || die "注入 dev 密钥失败。"
  dc run --rm --no-deps -v "$HOME_DIR:/devkit:ro" portal-api bun run register-app "/devkit/.add-$key.manifest.json" \
    || { rm -f "$HOME_DIR/.add-$key.manifest.json"; die "注册失败 —— 看上面的报错。"; }
  rm -f "$HOME_DIR/.add-$key.manifest.json"

  info "⑤ 建库 xgent-${key}"
  dc up -d postgres >/dev/null 2>&1 || true
  _ensure_db "xgent-${key}"

  # ⑥ 从 manifest 生成 compose 片段
  local gen="$HOME_DIR/generated"; mkdir -p "$gen"
  local out="$gen/${key}.yml" PREFIX; PREFIX="$(printf '%s' "$key" | tr 'a-z-' 'A-Z_')"
  {
    echo "# 由 onebox.sh add ${key} 从 manifest 生成 —— 不要手改，重跑 add 会覆盖。"
    echo "services:"
    echo "  ${key}-server:"
    echo "    image: \${${ivar}:?}"
    echo "    platform: \${${PREFIX}_PLATFORM:-\${APP_PLATFORM:-}}"
    echo "    restart: unless-stopped"
    # ⚠️ 绝对路径。compose 里的相对路径按【第一个 -f】的目录解析（= 一盒根），不是按本文件
    #    所在目录 —— 写 ../compose.env 会解析到一盒根的上一级，报 env file not found（实测）。
    #    这是生成文件、随机器走、已被 .gitignore 挡住，绝对路径没有副作用。
    echo "    env_file: [\"${ENV_FILE}\"]"
    echo "    environment:"
    echo "      PORT: \"${M_PORT}\""
    echo "      PORTAL_INTROSPECT_URL: http://portal-api:3000/api/tokens/introspect"
    echo "      ${PREFIX}_SA_CLIENT_ID: ${M_SA}"
    echo "      ${PREFIX}_SA_CLIENT_SECRET: \${${svar}}"
    # 发起方才有：门户库里那把（register-app 从清单读）与容器里这把必须同源，否则
    # 跨应用交换在【发起方】上 401，而错看起来像是被调方的问题。
    [ -n "$M_XTARGETS" ] && echo "      ${PREFIX}_APP_SECRET: \${${avar}}"
    echo "      ${PREFIX}_PG_DSN: postgres://postgres:postgres@postgres:5432/xgent-${key}"
    echo "      DATABASE_URL: postgres://postgres:postgres@postgres:5432/xgent-${key}"
    echo "    expose: [\"${M_PORT}\"]"
    echo "    networks: { default: { aliases: [${key}-server] } }"
  } > "$out"
  info "⑥ 生成 ${out#$HOME_DIR/}"
  [ -n "$M_ENV" ] && _note "清单声明了 requiredEnv：${M_ENV} —— 值归平台，按需补进 compose.env 末尾。"

  # 例外：manifest 描述不了的自带 infra（如 knowledge 的 Chroma）
  local extra_args=() extra="$SKILL_DIR/services/${key}.extra.yml"
  [ -f "$extra" ] && { extra_args=(-f "$extra"); info "   叠加自带依赖 services/${key}.extra.yml"; }

  info "⑦ 起容器"
  XGENT_ONEBOX_HOME="$HOME_DIR" dc -f "$out" "${extra_args[@]}" up -d "${key}-server"

  echo; info "⑧ 冒烟"
  local i=0 code=""
  while [ "$i" -lt 20 ]; do
    code="$(curl -s -m 5 -o /dev/null -w '%{http_code}' "${BASE_URL}/svc/${key}${M_HEALTH}" || true)"
    [ "$code" = "200" ] && break; i=$((i+1)); sleep 2
  done
  if [ "$code" = "200" ]; then _ok "/svc/${key}${M_HEALTH} 200"
  else _bad "/svc/${key}${M_HEALTH} HTTP ${code:-000}（等了 40s）" "${0} dc logs ${key}-server —— 多半是镜像认的 env 变量名与上面生成的那几个不同名，补进 compose.env 末尾即可"; fi

  echo
  info "还差最后一步（浏览器里点一下）：rockie@xgent.ai 登控制台 → 租户 → 可用应用 → 勾上「${key}」保存。"
  _note "service 型 App 的勾选【就是】安装；不装的话跨应用交换拿不到它的 scope。"
  # wireInitiatorSecret 只能写【已安装】的 apps 行 —— 先勾选安装、再重跑一次 add 才落得下去。
  [ -n "$M_XTARGETS" ] && _note "它声明了 exchangeTargets（${M_XTARGETS}）—— 勾选安装【之后】再跑一次 ${0} add ${key}，那把 App Secret 才写得进已安装的行（幂等，不会重复建东西）。"
  echo
  info "然后在【你自己】的 manifest 里两处一起加，再 ${0} up："
  printf '    "scopes":          […, "%s.read", …]\n' "$(printf '%s' "$key" | tr '-' '_')"
  printf '    "exchangeTargets": ["%s"]\n' "$key"
}

# --- doctor：把排查表能自动判的都判一遍 ----------------------------------------
DOCTOR_BAD=0
cmd_doctor() {
  DOCTOR_BAD=0
  echo "一盒体检（${BASE_URL}）"
  echo

  echo "① 镜像版本"
  local iv; iv="$(_eff XGENT_IMAGE)"
  if docker image inspect "$iv" >/dev/null 2>&1; then
    if docker run --rm --entrypoint sh "$iv" -c '[ -d /srv/www/apps ]' >/dev/null 2>&1; then
      _ok "门户镜像带有共享目录预建（v1.2.0+）"
    else
      _bad "门户镜像是旧版（缺 /srv/www/apps 等预建目录）" "$0 pull && $0 dc down -v && $0 up —— 顺序是【先换镜像、再删卷】，只做一半都不解决"
    fi
  else _note "本地没有 ${iv}，跳过（$0 pull）"; fi

  echo "② 命名卷属主（EACCES 一族的根因）"
  if _svc_running reverse-proxy; then
    local bad_dirs=""
    local d
    for d in /srv/www/apps /etc/caddy/svc-allow /etc/caddy/apps-csp; do
      local owner; owner="$(dc exec -T -u root reverse-proxy stat -c '%u' "$d" 2>/dev/null | tr -d '\r' || echo "")"
      [ "$owner" = "1000" ] || bad_dirs="$bad_dirs $d"
    done
    if [ -z "$bad_dirs" ]; then _ok "三个共享卷都归 bun(1000)"
    else _bad "这些卷不归 portal-api：$bad_dirs" "$0 dc exec -u root reverse-proxy chown -R 1000:1000$bad_dirs"; fi
    # 内联 key 却存在 map 文件 ⇒ duplicate input ⇒ 反代整个起不来
    local dup; dup="$(dc exec -T reverse-proxy sh -c 'ls /etc/caddy/svc-allow/ 2>/dev/null' 2>/dev/null | tr -d '\r' | grep -E '^(knowledge|omni-parser|task-gateway|pagebuilder)\.map$' | tr '\n' ' ' || true)"
    if [ -n "$dup" ]; then _bad "内联 key 又写了 map：$dup" "这会让 Caddy 拒绝整份配置（全站 502）。$0 dc exec -u root reverse-proxy sh -c 'cd /etc/caddy/svc-allow && rm -f $dup' && $0 dc restart reverse-proxy"
    else _ok "没有与内联 key 撞车的 map 文件"; fi
  else _note "reverse-proxy 没在跑，跳过（$0 up）"; fi

  echo "③ 架构匹配"
  local host_arch img_arch; host_arch="$(docker version --format '{{.Server.Arch}}' 2>/dev/null || echo '?')"
  if [ -n "$APP_IMAGE" ] && docker image inspect "$APP_IMAGE" >/dev/null 2>&1; then
    img_arch="$(docker image inspect --format '{{.Architecture}}' "$APP_IMAGE" 2>/dev/null || echo '?')"
    if [ "$img_arch" = "$host_arch" ] || [ -n "$(_eff APP_PLATFORM)" ]; then _ok "APP_IMAGE ($img_arch) 与本机 ($host_arch) 兼容"
    else _bad "APP_IMAGE 是 ${img_arch}，本机是 $host_arch" "compose.env 末尾加：APP_PLATFORM=linux/${img_arch}（Apple Silicon 走 Rosetta，慢但能跑）"; fi
  else _note "APP_IMAGE 未填或本地没有，跳过"; fi

  echo "④ 数据库"
  if _svc_running postgres; then
    local want="$CATALOG"; [ -n "$APP_KEY" ] && want="$CATALOG,$APP_KEY"
    local dbs; dbs="$(dc exec -T postgres psql -U postgres -lqt 2>/dev/null | cut -d'|' -f1 | tr -d ' \r' || true)"
    local miss=""
    for k in ${want//,/ }; do
      case "$k" in llm-gateway|ingest) ;; esac
      printf '%s\n' "$dbs" | grep -qx "xgent-$k" || miss="$miss xgent-$k"
    done
    if [ -z "$miss" ]; then _ok "目录里每个 App 的库都在"
    else _bad "缺库：$miss" "$0 dc exec postgres psql -U postgres -c 'CREATE DATABASE \"<库名>\"'（$0 add <key> 会顺手建）"; fi
  else _note "postgres 没在跑，跳过"; fi

  # 栈没起的时候，⑤⑥ 每一格都会红，而真正的下一步只有一个 —— 别拿 8 行红字淹掉它。
  if ! _svc_running portal-api; then
    echo "⑤⑥ 目录/路由"
    _bad "栈还没起来（portal-api 没在跑）" "${0} up —— 它会按顺序铺完再自动跑一次体检"
    echo; printf '%s先把栈起来，其余的判断才有意义。%s\n' "$c_ylw" "$c_off"
    return 0
  fi

  echo "⑤ 目录 vs 容器"
  for k in ${CATALOG//,/ }; do
    case "$k" in files|ingest|llm-gateway|git) continue;; esac
    if _svc_running "${k}-server"; then _ok "$k 的容器在跑"
    else _bad "$k 在 XGENT_APP_CATALOG 里但容器没起" "$0 add ${k}（清单已注册的话它只会补建库+起容器）"; fi
  done

  echo "⑥ 路由与健康"
  cmd_smoke >/dev/null 2>&1 || true
  local keys="$CATALOG"; [ -n "$APP_KEY" ] && keys="$CATALOG,$APP_KEY"
  local code
  code="$(curl -s -m 8 -o /dev/null -w '%{http_code}' "$BASE_URL/health" || true)"
  [ "$code" = "200" ] && _ok "门户 /health 200" || _bad "门户 /health HTTP ${code:-000}" "000=反代没起或端口不对（$0 dc ps）；其它看 $0 dc logs portal-api"
  for k in ${keys//,/ }; do
    code="$(curl -s -m 8 -o /dev/null -w '%{http_code}' "$BASE_URL/svc/$k/health" || true)"
    case "$code" in
      200) _ok "/svc/$k 200";;
      404) _bad "/svc/$k 404" "放行没写成或反代在写之前就起了：$0 dc exec reverse-proxy caddy reload";;
      502) _bad "/svc/$k 502" "后端不在/没听 8080/别名没命中：$0 dc logs ${k}-server";;
      *)   _bad "/svc/$k HTTP ${code:-000}" "$0 dc ps";;
    esac
  done

  echo
  if [ "$DOCTOR_BAD" -eq 0 ]; then printf '%s一切正常。%s\n' "$c_grn" "$c_off"
  else printf '%s%s 项要处理 —— 每条下面那行可以直接粘。%s\n' "$c_ylw" "$DOCTOR_BAD" "$c_off"
       printf '%s还有判不了的？排查表按「你看到什么」编排：references/troubleshooting.md%s\n' "$c_dim" "$c_off"; fi
  return 0
}

case "${1-status}" in
  init)   shift; cmd_init "$@" ;;
  pull)   shift; cmd_pull "$@" ;;
  env)    require_home; load_env; cmd_env ;;
  smoke)  require_home; load_env; cmd_smoke ;;
  status) require_home; load_env; cmd_status ;;
  chain)  require_home; load_env; { while IFS= read -r -d '' x; do printf '%s ' "$x"; done < <(compose_args); echo; } ;;
  up)     require_home; load_env; cmd_up ;;
  doctor) require_home; load_env; cmd_doctor ;;
  add)    require_home; load_env; shift; cmd_add "$@" ;;
  dc)     require_home; load_env; shift; dc "$@" ;;
  # 打印开头那段注释直到第一条非注释行 —— 别写死行号，加一条子命令就会截断（踩过）。
  -h|--help|help) awk 'NR>1 && /^#/ {sub(/^# ?/,""); print; next} NR>1 {exit}' "${BASH_SOURCE[0]}" ;;
  *)      die "未知子命令 '$1'。用 init | up | doctor | add <key> | pull | env | status | smoke | chain | dc <args...>" ;;
esac
