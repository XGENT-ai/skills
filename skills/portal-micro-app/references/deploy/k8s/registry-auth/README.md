# 给 in-cluster `registry:2` 加认证(接 Portal Service Account)

把裸 `registry:2` 切到 **token 认证**,身份源用 Portal 的 **M2M service account**(client_id + client_secret)——`imagePullSecret` / kaniko 推送凭证就是一个 SA 的凭证。轻量、开源、可按需扩展(细粒度 scope、TLS、push/pull 角色)。Harbor 太重时用这个。

> 机制:Docker Registry v2 token spec。registry 自己不存用户,收到请求先回 `401 + Bearer realm`,客户端拿 Basic 凭证去 **broker** 换一个 registry 信任的 RS256 JWT,再带着 JWT 重试。broker = `apps/api/scripts/registry-token-broker.ts`(复用 `validateServiceAccount` + `jose` 签名,跑在 runtime 镜像上)。

## ⚠️ 已知限制:realm 必须与 registry 同源(否则 kubelet 拉不动)

**containerd / kubelet 只把凭据发给与 registry 同一 `host:port` 的 token realm。** 本 recipe 把 broker 放在另一个端口(`:5001`)作 realm,于是 **containerd 匿名去换 token → broker 返回空 access → kubelet 拉取报 `insufficient_scope`**(`docker login` / kaniko / 手动 `curl -u` 都正常,唯独 kubelet 自动拉取不行)。这条 token 认证因此**对 in-cluster 自动拉取不可用**。

要让它真能用,broker 必须和 registry **同 host:port**:前面加一个反代(Caddy/nginx),`/v2/*` → registry、`/auth` → broker,realm 设成 `…/auth`(同源)。本期单节点 registry 绑 `127.0.0.1`、未对外,直接**退回开放 registry**(`imagePullSecret` 形同虚设);token 认证留作「realm 同源版」后续重做,代码/recipe 仍在此备用。

## 组件

| 组件 | 说明 |
| --- | --- |
| `registry-token-broker.ts` | broker:`/auth` 端点,校验 SA(`service_account_secrets` 的 sha256),签发 registry JWT。**任意有效 SA → `pull`;`REGISTRY_PUSHER_CLIENT_IDS` 里的 → `push`**。 |
| RSA key + 自签证书 | broker 用 key 签 JWT;registry 用 cert 作 `ROOTCERTBUNDLE` 验签(与 TLS 无关)。 |
| registry token env | `REGISTRY_AUTH=token` + realm 指向 broker + `SERVICE`/`ISSUER` 与 broker 一致 + `ROOTCERTBUNDLE`。 |

## 部署(单节点,broker + registry 都绑 node loopback)

```bash
# 1) 签名 key + cert（RS256;PKCS8 给 jose importPKCS8）
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out key.pem
openssl req -new -x509 -key key.pem -out cert.pem -days 3650 -subj "/CN=xgent-registry-token"
kubectl -n xgent-prod create secret generic registry-token-key --from-file=key.pem --from-file=cert.pem

# 2) 建两个 SA（用 Portal 控制台 / API 更规范;演示直接 SQL，secret 自己定，hash=sha256 hex）
#    registry-puller（拉）、registry-pusher（推，列进 REGISTRY_PUSHER_CLIENT_IDS）
#    insert into service_accounts (name, client_id) values ('Registry Puller','registry-puller'),('Registry Pusher','registry-pusher');
#    insert into service_account_secrets (service_account_id, secret_hash, prefix) select id, encode(digest('<secret>','sha256'),'hex'), left('<secret>',12) from service_accounts where client_id='registry-puller';

# 3) broker（用已含该脚本的 runtime 镜像;envFrom 给全 Portal env 以满足 env.ts 校验）
kubectl -n xgent-prod apply -f deploy/k8s/registry-auth/broker.yaml

# 4) registry 切 token 模式（挂 cert + 4 个 env，见下），重启
#    REGISTRY_AUTH=token
#    REGISTRY_AUTH_TOKEN_REALM=http://localhost:5001/auth   # 客户端可达(node loopback / hostNetwork)
#    REGISTRY_AUTH_TOKEN_SERVICE=xgent-registry             # 必须 == broker SERVICE / JWT aud
#    REGISTRY_AUTH_TOKEN_ISSUER=xgent-portal                # 必须 == broker ISSUER / JWT iss
#    REGISTRY_AUTH_TOKEN_ROOTCERTBUNDLE=/auth/cert.pem      # 挂 registry-token-key 的 cert.pem
```

## 用起来

- **拉**:`imagePullSecret` = puller 凭证;chart 设 `image.pullSecret=<dockerconfigjson secret>`。
  ```bash
  kubectl -n <ns> create secret docker-registry reg-puller \
    --docker-server=localhost:5000 --docker-username=registry-puller --docker-password=<puller-secret>
  ```
- **推**(kaniko):给 kaniko 挂一个指向 pusher 凭证的 docker config(`/kaniko/.docker/config.json`,`auths["localhost:5000"].auth=base64(registry-pusher:<secret>)`),去掉 `--insecure` 之外照旧。

## 验证(本仓库已在真集群跑通)

```bash
curl -i localhost:5000/v2/                          # 401 + Www-Authenticate: Bearer realm=...
TOK=$(curl -s -u registry-puller:<secret> "http://localhost:5001/auth?service=xgent-registry&scope=repository:xgent-ai-portal:pull" | jq -r .token)
curl -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $TOK" localhost:5000/v2/xgent-ai-portal/tags/list   # 200
# puller 请求 push-scope → access 只含 pull;pusher → pull+push;错密钥 → broker 401
```

## 按需后续(本期不做)

- **细粒度 scope**:给 SA 加 `registry.pull` / `registry.push` capability(`@xgent/shared`),broker 按 capability/scope 授权,替代当前的 pusher 环境变量白名单。
- **TLS**:现在 registry + broker 只绑 `127.0.0.1`,凭证只走 loopback。要对外暴露前,用 Caddy/cert-manager 给 registry + broker 发证(token/Basic 明文过线必须 HTTPS)。
- **审计**:broker 把每次签发(谁、对哪些 repo、什么动作)写进 Portal 审计日志。
