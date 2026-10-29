#!/usr/bin/env node
/**
 * xgent-app-release · 发布预检
 *
 *   node preflight.mjs --dist dist [--version 1.4.2] [--key <listingKey>]
 *                      [--manifest deploy/portal/app.manifest.json]
 *                      [--portal https://portal.example.com] [--config path] [--offline]
 *
 * `--manifest` 缺省时按常见路径自己找；找到就一并检查文案字段形状。
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
// 本地配置文件的读取只有一份实现（同目录 registry-config.mjs，npm-token.mjs 也用它）：
// 抄成两份的下场是换个字段名只改了一处，另一个脚本悄悄退回默认值。
import { loadConfig, parseArgs, pick, warnIfProxyIgnored } from "./registry-config.mjs";
import { normalizeEmbedCsp } from "../references/embed-csp.mjs";

const MAX_BYTES = 200 * 1024 * 1024; // 门户单包上限（MAX_DIST_BYTES），超了直接 VALIDATION_FAILED；整个请求另有 256 MiB 上限
const VERSION_RE = /^[A-Za-z0-9][A-Za-z0-9._+-]{0,63}$/; // 与门户逐字一致
const KEY_RE = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/;

const errs = [];
const warns = [];
const oks = [];
const err = (m) => errs.push(m);
const warn = (m) => warns.push(m);
const ok = (m) => oks.push(m);

const args = parseArgs(process.argv.slice(2), ["offline"]);
const cfg = loadConfig(args.config);
const key = String(pick(args.key, process.env.LISTING_KEY, cfg.LISTING_KEY));
const dist = args.dist ?? "dist";
const version = args.version ?? "";
// TARGET_XGENT_PLATFORM 是新名字，XGENT_PORTAL_URL 保留兼容（同一件事）。
const portal = String(pick(args.portal, process.env.TARGET_XGENT_PLATFORM, process.env.XGENT_PORTAL_URL, cfg.TARGET_XGENT_PLATFORM, cfg.XGENT_PORTAL_URL)).replace(/\/+$/, "");
const token = String(pick(process.env.XGENT_RELEASE_TOKEN, cfg.XGENT_RELEASE_TOKEN)).trim();

if (!key) {
  console.error("用法: node preflight.mjs [--key <listingKey>] --dist dist [--version v] [--manifest path] [--portal url] [--config path] [--offline]");
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
        `      → 生产受 per-App CSP 管。XHR/fetch 可通过 publish --manifest 申报 embedCsp.connectSrc，治理审核批准后再验实际响应头；` +
        `script/style/font/img/media 需各自来源声明；扩展指令要求目标门户已启用严格 CSP 与 v2 网关，预检无法确认启用状态。`,
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
      if (size > MAX_BYTES) err(`打包后 ${mb} MiB，超过门户单包 200 MiB 上限 —— 先查 source map / 未压缩素材是不是打进来了`);
      else ok(`打包 ${mb} MiB · ${entries} 个顶层条目`);
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
  warn(`找不到发布令牌 —— 在 .xgent-registry.env 里写 XGENT_RELEASE_TOKEN=xrel_…（平台管理员在控制台签发，明文只显示一次）`);
} else if (!token.startsWith("xrel_")) {
  err(`XGENT_RELEASE_TOKEN 形状不对：应以 xrel_ 开头（拿成别的令牌了？）`);
} else if (args.offline) {
  ok(`令牌形状正确（--offline，未联网校验）`);
} else if (!portal) {
  warn(`没有门户地址（--portal / TARGET_XGENT_PLATFORM），跳过令牌联网校验`);
} else {
  warnIfProxyIgnored();
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

/* ── 7. manifest 文案字段形状 ─────────────────────────────────────────
   门户把 manifest 的文案分两处存：展示字段进 text 列（只存一种语言），
   `scopeLabels` 与 `aclManifest` 里的 label 进 jsonb（支持按门户语言解析的多语对象）。
   两组的形状要求【不一样】，而写错的那一组【不会报错】：给 `tagline` 一个
   `{"zh-CN":…,"en":…}` 对象，会被拼成字面量 `[object Object]` 存进去，然后出现在
   每个租户的应用卡片和详情上。构建、发布、审批，整条链路没有任何一处会提示你。 */
const MANIFEST_GUESSES = [
  "deploy/portal/app.manifest.json", "portal-app/app.manifest.json",
  "deploy/app.manifest.json", "app.manifest.json",
];
const manifestPath = args.manifest ?? MANIFEST_GUESSES.find((p) => { try { return statSync(p).isFile(); } catch { return false; } });
if (!manifestPath) {
  warn(`没找到 app.manifest.json（试过 ${MANIFEST_GUESSES.join(" ")}）—— 用 --manifest <path> 指给我，文案字段形状就没人查了`);
} else {
  let mf = null;
  try {
    mf = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch (e) {
    err(`${manifestPath} 不是合法 JSON：${e?.message ?? e}`);
  }
  if (mf && typeof mf === "object") {
    if (Object.hasOwn(mf, "embedCsp")) {
      try {
        normalizeEmbedCsp(mf.embedCsp);
        ok("manifest.embedCsp 通过生产严格语法检查（不代表已获批或已加载）");
      } catch (e) {
        err(`manifest.embedCsp 不符合严格声明语法：${e.message}`);
      }
      warn("能力门未核验：strict 默认关闭；扩展 CSP 需平台确认已发布兼容版本并受控启用 strict 与 v2 网关。whoami 成功不证明此能力已启用。");
      warn("embedCsp 省略沿用；null 清空；完整对象替换原声明，未写指令会撤回。来源增删均需治理审核。");
    }
    const isObj = (v) => v !== null && typeof v === "object";
    const shown = (v) => JSON.stringify(v)?.slice(0, 60) ?? String(v);

    /* ① text 列 = 单语纯字符串。`name` 是唯一被容忍的：门户会把对象拍平成 zh-CN 存下去
       （不报错，但另外两种语言就此丢了）。其余给对象一律变成 "[object Object]"。 */
    if (isObj(mf.name)) {
      warn(
        `manifest.name 是对象（${shown(mf.name)}）—— 门户会把它拍平成 zh-CN 一种语言存下去。` +
          `应用名不支持多语，写纯字符串，别指望三语生效。`,
      );
    } else if (mf.name !== undefined && typeof mf.name !== "string") {
      err(`manifest.name 必须是字符串`);
    }
    for (const f of ["tagline", "desc", "icon", "color", "cat", "version", "embedUrl", "serviceBaseUrl", "helpEntry"]) {
      const v = mf[f];
      if (v === undefined || v === null || typeof v === "string") continue;
      err(
        `manifest.${f} 必须是字符串，现在是${Array.isArray(v) ? "数组" : typeof v}（${shown(v)}）。` +
          (isObj(v) ? `\n      → 这个字段不支持多语：给对象会被存成字面量 "[object Object]"，直接显示给每个租户。多语只有 scopeLabels 和 aclManifest 里的 label 支持。` : ""),
      );
    }
    for (const [i, n] of (Array.isArray(mf.navItems) ? mf.navItems : []).entries())
      if (n && typeof n.label !== "string")
        err(`manifest.navItems[${i}].label 必须是字符串（导航项标题不支持多语；现在是${shown(n.label)}）`);
    if (mf.dashboard !== undefined)
      err(`manifest.dashboard 不是有效字段 —— Dashboard 组件声明在顶层 dashboardWidgets（数组）里；嵌套的 dashboard.widgets 门户从不读取，新门户提交即拒`);
    const WIDGET_TYPES = ["metric", "activity", "insight", "topic_grid", "topic_list"];
    for (const [i, w] of (Array.isArray(mf.dashboardWidgets) ? mf.dashboardWidgets : []).entries()) {
      for (const f of ["title", "subtitle", "description"])
        if (w && w[f] !== undefined && typeof w[f] !== "string")
          err(`manifest.dashboardWidgets[${i}].${f} 必须是字符串（Widget 文案不支持多语）`);
      if (w && !WIDGET_TYPES.includes(w.type))
        err(`manifest.dashboardWidgets[${i}].type 只能是 ${WIDGET_TYPES.join(" / ")}（现在是${shown(w.type)}；注意是下划线）`);
      if (w && w.owner !== key)
        err(`manifest.dashboardWidgets[${i}].owner 必须等于 listingKey「${key}」（现在是${shown(w.owner)}）`);
    }

    /* ② scopeLabels = 同意屏正文，jsonb，**支持**多语对象。这里查的是三种「配了不生效」：
       键不在 scopes 里（写入时被静默丢弃）、多语对象缺 zh-CN（回退终点）、
       替平台基础 scope 写文案（那是平台的措辞，各 App 各写一份只会互相打架）。 */
    const declared = new Set(Array.isArray(mf.scopes) ? mf.scopes : []);
    const ns = [key, key.replace(/-/g, "_")];
    const labels = isObj(mf.scopeLabels) ? mf.scopeLabels : {};
    const LOCALES = ["zh-CN", "zh-TW", "en"];
    let labelOk = 0;
    for (const [s, v] of Object.entries(labels)) {
      if (Array.isArray(mf.scopes) && !declared.has(s)) {
        err(
          `manifest.scopeLabels 有一条 "${s}"，但它不在 manifest.scopes 里 —— 门户写入时会把它【静默丢弃】，` +
            `同意屏上那条 scope 回落成键名。要么补进 scopes，要么删掉这条文案。`,
        );
        continue;
      }
      if (!ns.some((p) => s === p || s.startsWith(`${p}.`))) {
        warn(
          `manifest.scopeLabels 替 "${s}" 写了文案，但它不在你的命名空间（${key}.）里 —— ` +
            `平台基础 scope 的说明归平台统一维护：各 App 各写一份，同一条权限在不同应用的同意屏上措辞就不一致了。` +
            `缺文案找平台补，别在自己清单里补。`,
        );
      }
      if (typeof v === "string") { labelOk++; continue; }
      if (!isObj(v) || Array.isArray(v)) { err(`manifest.scopeLabels["${s}"] 只能是字符串或 { "zh-CN": …, "zh-TW": …, "en": … } 对象`); continue; }
      if (typeof v["zh-CN"] !== "string" || !v["zh-CN"].trim())
        err(`manifest.scopeLabels["${s}"] 缺 "zh-CN" —— 它是回退终点，缺了在没有对应语言时会退到别的语言甚至键名`);
      const extra = Object.keys(v).filter((k) => !LOCALES.includes(k));
      if (extra.length) warn(`manifest.scopeLabels["${s}"] 里的 ${extra.join("/")} 门户界面用不到（只认 ${LOCALES.join(" / ")}）`);
      if (typeof v["zh-CN"] === "string") labelOk++;
    }
    const appScopes = [...declared].filter((s) => ns.some((p) => s === p || s.startsWith(`${p}.`)));
    const noLabel = appScopes.filter((s) => !(s in labels));
    if (noLabel.length)
      warn(
        `你自己命名空间的 scope 有 ${noLabel.length} 条没写 scopeLabels：${noLabel.slice(0, 4).join(" ")}${noLabel.length > 4 ? " …" : ""}` +
          `\n      → 门户对 App 命名空间的 scope 没有内建文案，同意屏上会直接显示键名。用户看到 "${noLabel[0]}" 是不会点同意的。`,
      );
    if (labelOk) ok(`scopeLabels ${labelOk} 条形状合法`);

    /* ③ usageMetrics[].label 必须是对象：语言键与 scopeLabels 同一套 zh-CN / en / zh-TW，
       旧形状 zh / en / tw 门户仍收并归一。缺 zh-CN（旧形状缺 zh）门户提交即拒（连提案都不落）。
       判定与门户 normalizeMetricLabel 同口径：取 zh-CN ?? zh，须是非空白字符串。 */
    for (const [i, u] of (Array.isArray(mf.usageMetrics) ? mf.usageMetrics : []).entries()) {
      const l = u?.label;
      const zh = isObj(l) ? l["zh-CN"] ?? l.zh : undefined;
      if (typeof zh !== "string" || !zh.trim())
        err(
          `manifest.usageMetrics[${i}].label 必须是 { "zh-CN": "…" }（en / zh-TW 可选，缺省回退 zh-CN）——` +
            `必须是对象，不能是纯字符串；旧形状 { "zh": "…" } 仍可用。缺 zh-CN 门户提交即拒。`,
        );
    }
    /* ③c adminPages：门户嵌进管理面的 App 自有页面。{ platform?, tenant? }，每级 { route, label }；
       route 是 App 内路由（/ 开头、不能 //），label 是字符串或带 zh-CN 的多语对象。门户提交面会再判一次。 */
    if (mf.adminPages !== undefined && mf.adminPages !== null) {
      if (!isObj(mf.adminPages) || Array.isArray(mf.adminPages)) err(`manifest.adminPages 必须是对象 { platform?, tenant? }`);
      else
        for (const [level, decl] of Object.entries(mf.adminPages)) {
          if (level !== "platform" && level !== "tenant") { err(`manifest.adminPages 只支持 platform / tenant（得到「${level}」）`); continue; }
          if (decl === null) continue;
          const extra = isObj(decl) ? Object.keys(decl).filter((k) => k !== "route" && k !== "label") : [];
          const route = decl?.route, label = decl?.label;
          if (!isObj(decl) || extra.length) err(`manifest.adminPages.${level} 必须是 { route, label }${extra.length ? `（多了 ${extra.join("、")}）` : ""}`);
          else if (typeof route !== "string" || !route.startsWith("/") || route.startsWith("//") || /(^|\/)\.\.(\/|$)/.test(route))
            err(`manifest.adminPages.${level}.route 必须是 App 内路由（以 / 开头，不能 //、不能 ..）`);
          else if (!(typeof label === "string" ? label.trim() : isObj(label) && typeof label["zh-CN"] === "string" && label["zh-CN"].trim()))
            err(`manifest.adminPages.${level}.label 必须是非空字符串，或带 "zh-CN" 的多语对象`);
        }
    }
    /* ③d tenantConfig：service 的受限动态配置 schema。门户提交面会做完整校验，这里先挡住常见形状错误。 */
    if (mf.tenantConfig !== undefined && mf.tenantConfig !== null) {
      const tc = mf.tenantConfig;
      const textOk = (value) => typeof value === "string" ? !!value.trim() : isObj(value) && typeof value["zh-CN"] === "string" && !!value["zh-CN"].trim();
      if (!isObj(tc) || Array.isArray(tc)) err(`manifest.tenantConfig 必须是 { label, path, scopes, schema }`);
      else {
        if (mf.type !== "service") err(`manifest.tenantConfig 只能由 type=service 声明`);
        if (!textOk(tc.label)) err(`manifest.tenantConfig.label 必须是非空字符串或带 zh-CN 的多语对象`);
        if (typeof tc.path !== "string" || !tc.path.startsWith("/") || tc.path.startsWith("//") || /[?#]/.test(tc.path) || /(^|\/)\.\.(\/|$)/.test(tc.path))
          err(`manifest.tenantConfig.path 必须是不带 query / fragment 的同源绝对路径`);
        if (!Array.isArray(tc.scopes) || !tc.scopes.length || tc.scopes.some((scope) => typeof scope !== "string" || !scope.trim()))
          err(`manifest.tenantConfig.scopes 必须是非空字符串数组`);
        else if (Array.isArray(mf.scopes)) for (const scope of tc.scopes) if (!mf.scopes.includes(scope)) err(`manifest.tenantConfig.scopes 的「${scope}」未声明在 listing scopes 中`);
        const walk = (node, where, depth) => {
          if (!isObj(node) || Array.isArray(node)) { err(`${where} 必须是对象`); return; }
          if (depth > 4) { err(`${where} 嵌套超过 4 层`); return; }
          if (node.type === "object") {
            if (!isObj(node.properties) || Array.isArray(node.properties) || !Object.keys(node.properties).length) { err(`${where}.properties 必须是非空对象`); return; }
            for (const [key, child] of Object.entries(node.properties)) {
              if (!/^[A-Za-z_][A-Za-z0-9_]{0,63}$/.test(key) || ["__proto__", "prototype", "constructor"].includes(key)) err(`${where}.properties 的键「${key}」非法`);
              walk(child, `${where}.properties.${key}`, depth + 1);
            }
            return;
          }
          if (!["integer", "number", "select", "boolean"].includes(node.type)) { err(`${where}.type 只支持 integer / number / select / boolean / object`); return; }
          if (!textOk(node.title)) err(`${where}.title 必须是非空字符串或带 zh-CN 的多语对象`);
          if (node.description !== undefined && !textOk(node.description)) err(`${where}.description 形状非法`);
          if (["integer", "number"].includes(node.type)) {
            for (const field of ["default", "scale", "minimum", "maximum"]) if (node[field] !== undefined && (typeof node[field] !== "number" || !Number.isFinite(node[field]))) err(`${where}.${field} 必须是有限数字`);
            if (node.scale !== undefined && node.scale <= 0) err(`${where}.scale 必须是正数`);
            if (node.unit !== undefined && !textOk(node.unit)) err(`${where}.unit 形状非法`);
          } else if (node.type === "select") {
            if (!Array.isArray(node.options) || !node.options.length) err(`${where}.options 不能为空`);
            else for (const option of node.options) if (!isObj(option) || typeof option.value !== "string" || !option.value.trim() || !textOk(option.label)) err(`${where}.options 每项必须是 { value, label }`);
          } else if (node.default !== undefined && typeof node.default !== "boolean") err(`${where}.default 必须是布尔值`);
        };
        walk(tc.schema, "manifest.tenantConfig.schema", 0);
      }
    }
    /* ③b requiredEnvNotes：只能给 requiredEnv 里的键写说明，非空、≤200 字（与门户同口径）。 */
    if (mf.requiredEnvNotes !== undefined) {
      const declaredEnv = new Set((Array.isArray(mf.requiredEnv) ? mf.requiredEnv : [])
        .map((e) => (typeof e === "string" ? e : typeof e?.key === "string" ? e.key : null)).filter(Boolean));
      if (!isObj(mf.requiredEnvNotes) || Array.isArray(mf.requiredEnvNotes))
        err(`manifest.requiredEnvNotes 必须是对象 { "键名": "说明" }`);
      else
        for (const [k, note] of Object.entries(mf.requiredEnvNotes)) {
          if (!declaredEnv.has(k)) err(`manifest.requiredEnvNotes 的「${k}」不在 requiredEnv 里 —— 说明只能写给声明过的键`);
          else if (typeof note !== "string" || !note.trim()) err(`manifest.requiredEnvNotes 的「${k}」必须是非空字符串`);
          else if (note.trim().length > 200) err(`manifest.requiredEnvNotes 的「${k}」超过 200 字（它显示在审批屏上，不是文档）`);
        }
    }
    /* ④ 像库 / Redis 连接串、却不按约定命名的键：平台自动供给不到它，落到审批屏就是
       「平台管理员多点一下」。**只 warn 不阻断** —— 键名归 App，这里只是告诉你便宜的写法。 */
    const PREFIX = key ? key.toUpperCase().replace(/-/g, "_") : null;
    const CONVENTIONAL = new Set([PREFIX ? `${PREFIX}_DATABASE_URL` : "", "REDIS_CONN_STRING"].filter(Boolean));
    const envKeys = (Array.isArray(mf.requiredEnv) ? mf.requiredEnv : [])
      .map((e) => (typeof e === "string" ? e : typeof e?.key === "string" ? e.key : null))
      .filter(Boolean);
    // requiredServices[].envKey 显式绑定过的键，平台照样自动供给，不算「供给不到」。
    const BOUND = new Set((Array.isArray(mf.requiredServices) ? mf.requiredServices : []).map((s) => s?.envKey).filter((k) => typeof k === "string"));
    const unbound = envKeys.filter((k) => /(DATABASE_URL|_DSN|POSTGRES|REDIS)/.test(k) && !CONVENTIONAL.has(k) && !BOUND.has(k));
    if (unbound.length)
      warn(
        `这几个键看着是库 / Redis 连接串，但平台自动供给不到它们：${unbound.slice(0, 4).join(" ")}${unbound.length > 4 ? " …" : ""}` +
          `\n      → 改用约定名就零手填：${PREFIX ? `${PREFIX}_DATABASE_URL` : "<PREFIX>_DATABASE_URL"} / REDIS_CONN_STRING；` +
          `名字改不了（上游定死）就在 requiredServices 里用 envKey 绑定：{ "kind": "postgres", "name": "main", "envKey": "${unbound[0]}" }。` +
          `\n      不改也能发 —— 只是平台管理员要在审批屏上逐个手选服务取值，多等一轮。`,
      );

    if (!errs.some((m) => m.startsWith(`manifest.`) || m.startsWith(`${manifestPath} `)))
      ok(`${manifestPath} 文案字段形状合法`);
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
