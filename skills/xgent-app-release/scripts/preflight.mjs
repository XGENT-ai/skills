#!/usr/bin/env node
/**
 * xgent-app-release · 发布预检
 *
 *   node preflight.mjs --dist dist [--version 1.4.2] [--key <listingKey>]
 *                      [--portal https://portal.example.com] [--config path] [--offline]
 *
 * `--key` 缺省时从本地配置文件的 LISTING_KEY 读（与 release-cli 同一份）。
 *
 * 挡的是同一类事故：构建成功、命令返回 ✓、线上却白屏或发不出去。这些成因
 * tsc / lint / 单测一条都发现不了，因为它们全都发生在「产物形状」与「凭证」上。
 *
 * 零依赖，纯 Node ≥18（fetch 内置）。可以整个文件拷进任何 App 仓。
 * 退出码：有 ✗ → 1，否则 0（! 只是提醒，不拦）。
 */
import { readFileSync, statSync, readdirSync, rmSync, mkdtempSync, existsSync } from "node:fs";
import { join, extname } from "node:path";
import { tmpdir, homedir } from "node:os";
import { spawnSync } from "node:child_process";

const MAX_BYTES = 64 * 1024 * 1024; // 门户侧上限，超了直接 VALIDATION_FAILED
const VERSION_RE = /^[A-Za-z0-9][A-Za-z0-9._+-]{0,63}$/; // 与门户逐字一致
const KEY_RE = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/;

const errs = [];
const warns = [];
const oks = [];
const err = (m) => errs.push(m);
const warn = (m) => warns.push(m);
const ok = (m) => oks.push(m);

function parseArgs(argv) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--offline") out.offline = true;
    else if (a.startsWith("--")) out[a.slice(2)] = argv[++i];
    else out._.push(a);
  }
  return out;
}

/** 与 release-cli 同一份本地配置：身份（LISTING_KEY）在文件里，密钥与门户地址不在。 */
function configKey(explicit) {
  const isFile = (p) => { try { return existsSync(p) && statSync(p).isFile(); } catch { return false; } };
  const named = explicit ?? process.env.XGENT_REGISTRY_CONFIG;
  if (named && !isFile(named)) { console.error(`✗ 配置文件不存在：${named}`); process.exit(1); }
  const path = named ?? [
    "./.xgent-registry.env",
    join(process.env.XDG_CONFIG_HOME || join(homedir(), ".config"), "xgent", "registry.env"),
    join(homedir(), ".xgent-registry.env"),
  ].find(isFile);
  if (!path) return "";
  for (const line of readFileSync(path, "utf8").split("\n")) {
    if (line.trim().startsWith("#")) continue;
    const m = line.match(/^\s*(?:export\s+)?LISTING_KEY\s*=\s*(.*)$/);
    if (m) return m[1].trim().replace(/^(['"])(.*)\1$/, "$2");
  }
  return "";
}

const args = parseArgs(process.argv.slice(2));
const key = args.key ?? process.env.LISTING_KEY ?? configKey(args.config);
const dist = args.dist ?? "dist";
const version = args.version ?? "";
const portal = (args.portal ?? process.env.XGENT_PORTAL_URL ?? "").replace(/\/+$/, "");
const token = (process.env.XGENT_RELEASE_TOKEN ?? "").trim();

if (!key) {
  console.error("用法: node preflight.mjs [--key <listingKey>] --dist dist [--version v] [--portal url] [--config path] [--offline]");
  console.error("      缺 --key 时从本地配置文件的 LISTING_KEY 读（./.xgent-registry.env 等）");
  process.exit(1);
}
if (!KEY_RE.test(key)) err(`--key "${key}" 形状不合法：只能小写字母/数字/连字符，且不能以连字符开头或结尾`);

/* ── 1. 产物目录与根 index.html ─────────────────────────────────────────
   门户解包后就找根目录的 index.html；找不到直接拒收。最常见的成因不是构建
   坏了，而是把仓库根 / 上一层目录当成了 --dist。 */
let distStat = null;
try {
  distStat = statSync(dist);
} catch {
  err(`产物目录不存在：${dist}（是不是还没构建，或路径写错了？）`);
}
if (distStat && !distStat.isDirectory()) err(`${dist} 不是目录`);

let html = null;
if (distStat?.isDirectory()) {
  try {
    html = readFileSync(join(dist, "index.html"), "utf8");
    ok(`根目录有 index.html`);
  } catch {
    // 帮一把：如果它躺在下一层，几乎肯定是 --dist 指错了一层
    let hint = "";
    try {
      const sub = readdirSync(dist, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .find((d) => {
          try { statSync(join(dist, d.name, "index.html")); return true; } catch { return false; }
        });
      if (sub) hint = ` —— 但 ${join(dist, sub.name)}/index.html 在，--dist 应该指那一层`;
    } catch {}
    err(`产物根目录没有 index.html${hint}`);
  }
}

/* ── 2. base 前缀（翻车率第一名）──────────────────────────────────────
   产物在生产被挂到 /apps/<key>/ 下。base 少了，页面本身能 200（宿主 iframe
   拿得到 index.html），但里面每个 /assets/… 请求都打到站点根 → 404 → 白屏。
   构建、类型检查、单测全绿，只有人眼在浏览器里看得出来。 */
if (html) {
  const base = `/apps/${key}/`;
  const refs = [...html.matchAll(/(?:src|href)\s*=\s*["']([^"']+)["']/gi)].map((m) => m[1]);
  const rooted = refs.filter((r) => r.startsWith("/"));
  const bad = rooted.filter((r) => !r.startsWith(base));
  const good = rooted.filter((r) => r.startsWith(base));
  const relative = refs.filter((r) => !/^([a-z]+:)?\/\//i.test(r) && !r.startsWith("/") && !r.startsWith("#") && !r.startsWith("data:"));
  const external = refs.filter((r) => /^https?:\/\//i.test(r));

  if (bad.length) {
    err(
      `index.html 里有 ${bad.length} 个绝对路径没走 base ${base}：${bad.slice(0, 3).join(" ")}${bad.length > 3 ? " …" : ""}\n` +
        `      → 构建时 base 没设对。vite: base: command === 'build' ? '${base}' : '/'`,
    );
  } else if (good.length) {
    ok(`资源路径都带 base ${base}（${good.length} 处）`);
  } else if (relative.length) {
    warn(
      `index.html 用的是相对路径（${relative.slice(0, 3).join(" ")}）而不是 ${base}。` +
        `资源本身能加载，但客户端路由进到嵌套路径后相对基准会变，容易在二级页面上炸。建议显式设 base。`,
    );
  } else {
    warn(`index.html 里没解析到任何 src/href —— 确认这是构建产物而不是模板？`);
  }
  if (external.length) {
    warn(
      `index.html 直连了外部地址：${external.slice(0, 3).join(" ")}${external.length > 3 ? " …" : ""}\n` +
        `      → 生产受 per-App CSP 管，自助面改不了它。XHR/fetch 那类可由 manifest 的 embedCsp.connectSrc 放行；` +
        `字体/样式/图片走的是 font-src / style-src / img-src，能不能放行要问平台管理员。最稳的做法是把资源打进产物、不外链。`,
    );
  }
}

/* ── 3. 打包形状与体积 ────────────────────────────────────────────────
   顺带验证 tar 可用：release-cli 用的就是这条命令，它跑不了发布也跑不了。 */
if (distStat?.isDirectory()) {
  const temp = mkdtempSync(join(tmpdir(), "xgent-preflight-"));
  try {
    const out = join(temp, "dist.tgz");
    const r = spawnSync("tar", ["czf", out, "-C", dist, "."], { stdio: ["ignore", "ignore", "pipe"] });
    if (r.status !== 0) {
      err(`tar 打包失败（release-cli 用的是同一条命令）：${String(r.stderr).slice(0, 200)}`);
    } else {
      const size = statSync(out).size;
      const mb = (size / 1024 / 1024).toFixed(1);
      const entries = readdirSync(dist).length;
      if (size > MAX_BYTES) err(`打包后 ${mb}MB，超过门户 64MB 上限 —— 先查 source map / 未压缩素材是不是打进来了`);
      else ok(`打包 ${mb}MB · ${entries} 个顶层条目`);
    }
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

/* ── 4. dev 残留 ─────────────────────────────────────────────────────
   cross-origin vite dev 那条联调通路会让人往代码里写 localhost:53xx。留在
   产物里的话，生产 CSP 直接拦掉，症状是「本地好好的，线上功能静默不动」。 */
if (distStat?.isDirectory()) {
  const hits = [];
  const scan = (dir, depth = 0) => {
    if (depth > 4 || hits.length >= 5) return;
    for (const e of readdirSync(dir, { withFileTypes: true })) {
      const p = join(dir, e.name);
      if (e.isDirectory()) scan(p, depth + 1);
      else if ([".js", ".mjs", ".css", ".html", ".json"].includes(extname(e.name))) {
        let s;
        try {
          if (statSync(p).size > 8 * 1024 * 1024) continue;
          s = readFileSync(p, "utf8");
        } catch { continue; }
        if (/https?:\/\/(localhost|127\.0\.0\.1):\d+/.test(s)) hits.push(p);
        if (hits.length >= 5) return;
      }
    }
  };
  scan(dist);
  if (hits.length) warn(`产物里有 localhost 地址：${hits.join(" ")} —— 生产 CSP 会拦掉，确认是死代码还是真会被调用`);
}

/* ── 5. 版本号形状 ───────────────────────────────────────────────────── */
if (version) {
  if (!VERSION_RE.test(version)) err(`--version "${version}" 门户会拒：只能字母/数字/. _ + -，且 ≤64 字符`);
  else ok(`版本号 ${version} 形状合法（记得它必须比线上那版新 —— 同号重发会让「线上是哪一版」说不清）`);
} else {
  warn(`没传 --version：发布时它是必填项，且每次都要 bump（哪怕只换产物）`);
}

/* ── 6. 令牌 ─────────────────────────────────────────────────────────
   放最后但最值得做：令牌过期/吊销/发给了别的 key，只有真调一次才现形。 */
if (!token) {
  warn(`环境里没有 XGENT_RELEASE_TOKEN —— 发布时必需（平台管理员在控制台签发，明文只显示一次）`);
} else if (!token.startsWith("xrel_")) {
  err(`XGENT_RELEASE_TOKEN 形状不对：应以 xrel_ 开头（拿成别的令牌了？）`);
} else if (args.offline) {
  ok(`令牌形状正确（--offline，未联网校验）`);
} else if (!portal) {
  warn(`没有门户地址（--portal 或 XGENT_PORTAL_URL），跳过令牌联网校验`);
} else {
  const url = `${portal}/api/market/release/${encodeURIComponent(key)}`;
  try {
    const res = await fetch(url, { headers: { authorization: `Bearer ${token}` } });
    if (res.status === 401) err(`令牌无效 / 已吊销 / 已过期（门户每次调用实时查库，吊销即刻生效）`);
    else if (res.status === 404) err(`令牌不是为 ${key} 签发的（也可能该 listing 不存在 —— 门户故意不区分这两种）`);
    else {
      const body = await res.json().catch(() => null);
      const d = body?.data;
      if (!d) warn(`令牌校验返回了预期外的响应（HTTP ${res.status}）—— 确认 --portal 指的是门户而不是别的服务`);
      else {
        ok(`令牌有效 · ${d.key} · 前缀 ${d.prefix}… · ${d.expiresAt ? `${d.expiresAt} 过期` : "不过期"}`);
        if (d.expiresAt) {
          const days = (new Date(d.expiresAt) - new Date()) / 86400000;
          if (days < 7) warn(`令牌还有 ${days.toFixed(1)} 天过期 —— 提前找平台管理员续，别卡在发布当天`);
        }
      }
    }
  } catch (e) {
    warn(`令牌联网校验失败（${e?.message ?? e}）—— 网络不通就用 --offline 跳过，但发布时还是会验`);
  }
}

/* ── 报告 ───────────────────────────────────────────────────────────── */
for (const m of oks) console.log(`✓ ${m}`);
for (const m of warns) console.log(`! ${m}`);
for (const m of errs) console.log(`✗ ${m}`);
console.log(
  errs.length
    ? `\n预检未通过：${errs.length} 项必须先修。修完重跑，别硬发 —— 门户拒收时不改任何状态，但白屏类问题它拦不住。`
    : `\n预检通过${warns.length ? `（${warns.length} 条提醒，自行判断）` : ""}。可以 publish 了。`,
);
process.exit(errs.length ? 1 : 0);
