# 安装平台私有 npm 包

本文件可独立使用，无需加载发布 skill。`@xgent/*`（含 `release-cli`、`shared`、`portal-sdk`、`portal-server-sdk`、`portal-ui`）从平台私有 registry 安装，公共 npm 没有这些包。首次安装 App 依赖就要配置，不必等到发版。

## 凭证与路径

用 App 的发布令牌 `XGENT_RELEASE_TOKEN=xrel_…` 向门户换取有效期 ≤12 h 的只读 npm 令牌；不需要云账号、IAM key 或云厂商 CLI。镜像 Harbor 的 puller/pusher 凭证不能代替 npm 凭证。

脚本读取 `LISTING_KEY`、`TARGET_XGENT_PLATFORM`（兼容 `XGENT_PORTAL_URL`）及 `XGENT_RELEASE_TOKEN`。参数、环境变量优先于文件；配置文件定位顺序：`--config` / `XGENT_REGISTRY_CONFIG` → 仓根 `.xgent-registry.env` → `${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env` → `~/.xgent-registry.env`。

缺凭证时停止安装，请开发者从门户「开发者应用 › 你的应用 › 凭证 › 生成配置文件」取得 `.xgent-registry.env`，放到仓根、`chmod 600` 并加入 `.gitignore`。应用未建立则先申请、待平台批准；缺开发者权限请租户管理员授予「开发者」角色。由脚本读取文件，不把凭证打印到对话或日志。

命令在 App 仓根执行，按实际安装位置设置：

```bash
RELEASE_SKILL_DIR="<xgent-app-release 的 SKILL.md 所在目录>"
npm config get @xgent:registry
```

`undefined` 或指向公共 npm 表示尚未正确配置。registry 地址使用门户返回值，不猜地址。

## 配置与安装

已有项目 `.npmrc` 时保留无关配置，只合并/更新返回的 scope 与认证键，避免重复追加过期令牌。

没有 `.npmrc` 时可直接生成本地配置（含明文短期令牌，先确保该文件已被 git 忽略）：

```bash
node "$RELEASE_SKILL_DIR/scripts/npm-token.mjs" --npmrc > .npmrc
chmod 600 .npmrc
npm config get @xgent:registry
```

使用仓库既定包管理器安装依赖。不要把含令牌的 `.npmrc` 提交到 Git；过期时重新换取并更新认证键。

需要提交 `.npmrc` 或用于 CI 时，使用变量引用：

```ini
@xgent:registry=${XGENT_NPM_REGISTRY}
//<门户返回的 registry host>/<路径>/:_authToken=${XGENT_NPM_AUTH_TOKEN}
```

认证键的 host 与路径必须匹配门户返回的 registry；占位部分须替换，令牌保持变量引用。在同一个 shell 中先换令牌，再安装：

```bash
eval "$(node "$RELEASE_SKILL_DIR/scripts/npm-token.mjs")"
npm ci  # 使用本仓实际包管理器与锁文件对应的命令
```

CI 不把短期令牌持久存成凭证文件；不同 step 若不共享环境，使用 CI 的 secret/env 传递机制。变量为空会以空令牌访问并返回 401。脚本令牌输出只走 stdout，诊断走 stderr；stdout 只交给 shell 或受保护文件，勿直接展示。需要单独诊断换令牌时用 `--check`，它不打印令牌。

## 错误处理与边界

| 症状 | 处理 |
| --- | --- |
| E404 指向 `registry.npmjs.org/@xgent…` | 修正 `@xgent:registry` 映射 |
| 私有仓安装返回 401 | 核对变量是否为空、短期令牌是否过期；重新换令牌 |
| 请求版本未发布 | 停止安装，请平台团队发布该版本 |
| `NPM_REGISTRY_NOT_CONFIGURED` / `NPM_REGISTRY_UNAUTHORIZED` | 平台配置缺失或仓库凭据被拒，请平台管理员处理 |
| `NPM_REGISTRY_UNAVAILABLE` | 稍后重试一次；仍失败则报告 |

**不许 vendor 或绕过 SDK**：不复制 `@xgent/*` 源码/产物进仓、不以 `file:` / `link:` / tar 包替代依赖，不为绕过安装失败重写 SDK 已有能力。平台轮换云凭据无需 App 改配置，重新换取 npm 令牌即可。

## 令牌 API

已有脚本封装在本 skill 的 `scripts/npm-token.mjs`，依赖同目录的 `registry-config.mjs`；无需另写调用器。

```text
GET <TARGET_XGENT_PLATFORM>/api/market/release/<key>/npm-token
Authorization: Bearer xrel_…
→ 200 { ok: true, data: { registry, scopes: ["@xgent"], token, expiresAt } }
```

令牌为域级只读，可安装包，不能发布或删除。接口 401 表示发布令牌缺失/失效；404 须核对 App key 与令牌归属；429 须降低换取频率。平台侧配置错误通过 200 + `ok: false` 和错误码返回，见上表。门户侧缓存换取结果，CI 每次运行按需取得，不在每个 job 重复请求。
