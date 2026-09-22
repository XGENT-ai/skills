#!/usr/bin/env node
/**
 * xgent-app-release · 换一枚私有包只读令牌
 *
 *   node npm-token.mjs            # 打印 export XGENT_NPM_AUTH_TOKEN='…'（可 eval）
 *   node npm-token.mjs --raw      # 只打印令牌本体（管道用）
 *   node npm-token.mjs --npmrc    # 打印可直接追加进 .npmrc 的三行
 *   node npm-token.mjs --check    # 只体检：能不能换到、还剩多久，不打印令牌
 *
 * `@xgent/{release-cli,shared,portal-sdk,portal-ui}` 都装在私有仓上 —— **发版用的 CLI 本身也在**，
 * 所以 `.npmrc` 没配好时 `npx @xgent/release-cli` 第一条就 E404。你**不需要**任何云账号或 CLI：
 * 用已有的发布令牌（`XGENT_RELEASE_TOKEN`，就是发版那枚）向门户换一枚 ≤12 h 的
 * **只读**令牌即可。门户持那把云凭据，轮换时你这边零改动。
 *
 * 取值优先级：命令行参数 > 环境变量 > 本地配置文件（`.xgent-registry.env` 等）。
 * 令牌**只走 stdout**，不落盘、不进日志；诊断信息一律走 stderr，所以
 * `eval "$(node npm-token.mjs)"` 与 `node npm-token.mjs --raw > /dev/null` 都是安全的。
 *
 * 零依赖，纯 Node ≥18（fetch 内置）。可以整个文件拷进任何 App 仓。
 * 退出码：换到 → 0；配置缺失/被拒/不可达 → 1。
 */
import { loadConfig, parseArgs, resolveRelease, warnIfProxyIgnored } from "./registry-config.mjs";

const args = parseArgs(process.argv.slice(2), ["raw", "npmrc", "check", "help"]);
if (args.help) {
  console.error("用法: node npm-token.mjs [--raw|--npmrc|--check] [--key <listingKey>] [--portal <url>] [--config <path>]");
  process.exit(0);
}

const cfg = loadConfig(args.config);
const { key, portal, token } = resolveRelease(args, cfg);

const die = (msg, ...hints) => {
  console.error(`✗ ${msg}`);
  for (const h of hints) console.error(`  → ${h}`);
  process.exit(1);
};

if (!key) die("缺少 LISTING_KEY", "在 .xgent-registry.env 里写 LISTING_KEY=<你的应用标识>，或用 --key 传");
if (!portal) die("缺少门户地址", "在 .xgent-registry.env 里写 TARGET_XGENT_PLATFORM=<门户地址>，或用 --portal 传");
if (!token) die("缺少发布令牌", "在 .xgent-registry.env 里写 XGENT_RELEASE_TOKEN=xrel_…（就是发版用的那枚）");

warnIfProxyIgnored();

const url = `${portal}/api/market/release/${encodeURIComponent(key)}/npm-token`;
let res;
try {
  res = await fetch(url, {
    headers: { authorization: `Bearer ${token}`, accept: "application/json" },
    signal: AbortSignal.timeout(30_000),
  });
} catch (e) {
  die(`连不上门户：${e?.message ?? e}`, `确认 ${portal} 可达（公司网络 / VPN）`, "稍后重试；这条链路超时不代表令牌有问题");
}

if (res.status === 401) die("发布令牌无效或已失效（401）", "去门户「应用详情 › 凭证」重新生成一枚，或让平台管理员补发");
if (res.status === 404) die(`门户上没有 ${key} 这个应用，或令牌绑的是别的应用（404）`, "核对 LISTING_KEY 与令牌是否同一个应用");
if (res.status === 429) die("触发限流（429）", "稍等一分钟再试；CI 里别在每个 job 都换一次，换一次传下去");

const body = await res.json().catch(() => null);
if (!res.ok || !body) die(`门户返回 ${res.status}`, "把这一行贴给平台管理员");

if (body.ok === false) {
  const code = body.error?.code ?? "UNKNOWN";
  const msg = body.error?.message ?? "";
  if (code === "NPM_REGISTRY_NOT_CONFIGURED")
    die("门户还没配置私有包仓库凭据", "让平台管理员去「控制台 › 平台设置 › 私有包仓库」填一次", "这不是你这边的问题，无需改任何配置");
  if (code === "NPM_REGISTRY_UNAUTHORIZED")
    die("门户持有的仓库凭据被拒了", "让平台管理员检查「私有包仓库」设置里的凭据是否过期", "这不是你这边的问题");
  if (code === "NPM_REGISTRY_UNAVAILABLE")
    die("私有包仓库暂时不可达", "过几分钟重试", "CI 里给这一步加一次重试即可");
  die(`${code}${msg ? `：${msg}` : ""}`);
}

const { registry, scopes, token: authToken, expiresAt } = body.data ?? {};
if (!authToken || !registry) die("门户返回的内容不完整", "把这一行贴给平台管理员");

const mins = Math.max(0, Math.round((Date.parse(expiresAt) - Date.now()) / 60000));
console.error(`✓ 已换到只读令牌，有效期约 ${Math.floor(mins / 60)} 小时 ${mins % 60} 分（至 ${expiresAt}）`);

if (args.check) process.exit(0);

if (args.raw) {
  console.log(authToken);
} else if (args.npmrc) {
  // registry 形如 https://<host>/npm/<repo>/ —— .npmrc 的 //<host>/<path>:_authToken 要去掉协议。
  const bare = registry.replace(/^https?:/, "");
  for (const s of scopes ?? ["@xgent"]) console.log(`${s}:registry=${registry}`);
  console.log(`${bare}:_authToken=${authToken}`);
  console.log(`${bare}:always-auth=true`);
} else {
  console.log(`export XGENT_NPM_AUTH_TOKEN='${authToken}'`);
  console.log(`export XGENT_NPM_REGISTRY='${registry}'`);
}
