#!/usr/bin/env node
/**
 * xgent-init · 生成文档结构检查
 *
 *   node check-docs.mjs [目标仓根，默认 .]
 *
 * 挡的是模型输出的三类漂移：残留占位（用户还得手工编辑）、结构走样（impeccable
 * 按标题逐字解析，多一段少一段就读不出来）、过度替换（把 `Page<T>` 这种类型名
 * 当槽位换掉了）。内容质量不归它管。
 *
 * 零依赖，纯 Node ≥18。退出码：有 ✗ → 1，否则 0（! 只是提醒，不拦）。
 */
import { readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const KEY_RE = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/; // 与门户逐字一致
const CJK_RE = /[\u3000-\u303F\u4E00-\u9FFF\uFF00-\uFFEF]/; // 中日韩标点与汉字

const errs = [];
const warns = [];
const oks = [];
const err = (m) => errs.push(m);
const warn = (m) => warns.push(m);
const ok = (m) => oks.push(m);

const root = process.argv[2] ?? ".";
if (["-h", "--help"].includes(root)) {
  console.log("用法: node check-docs.mjs [目标仓根，默认 .]");
  process.exit(0);
}

const read = (p) => {
  try {
    const full = join(root, p);
    return statSync(full).isFile() ? readFileSync(full, "utf8") : null;
  } catch {
    return null;
  }
};

/** 去掉围栏代码块，避免把代码里的 `##`、`[…]` 当正文。 */
const stripFences = (src) => src.replace(/^```[\s\S]*?^```/gm, "");
const h2 = (src) =>
  stripFences(src)
    .split("\n")
    .filter((l) => l.startsWith("## "))
    .map((l) => l.slice(3).trim());

/* ── 0. 三份文件 ────────────────────────────────────────────────── */

const claude = read("CLAUDE.md");
const product = read("PRODUCT.md");
const design = read("DESIGN.md");

if (!claude) err("CLAUDE.md 不存在");
if (!product) err("PRODUCT.md 不存在");

let manifest = null;
const MANIFEST_GUESSES = [
  "app.manifest.json",
  "deploy/portal/app.manifest.json",
  "portal-app/app.manifest.json",
  "deploy/app.manifest.json",
];
for (const p of MANIFEST_GUESSES) {
  const raw = read(p);
  if (raw == null) continue;
  try {
    manifest = JSON.parse(raw);
    ok(`读到 ${p}`);
  } catch (e) {
    warn(`${p} 不是合法 JSON（${e?.message ?? e}），本次不拿它交叉核对`);
  }
  break;
}
if (!manifest) warn(`没找到 app.manifest.json（试过 ${MANIFEST_GUESSES.join(" ")}）—— key / type / 身份色无法交叉核对`);

/* ── 1. 残留占位（三份文件通查） ─────────────────────────────────
   R-5：生成完就能直接用，不留任何「还要手工编辑」的痕迹。 */

const RESIDUE = [
  ["<APP_KEY>", "令牌没替换"],
  ["<APP_NAME>", "令牌没替换"],
  ["<PREFIX>", "令牌没替换"],
  ["填写指引", "模板的填写指引注释没删"],
  ["✂️", "模板说明块没删干净"],
  ["[Signature Component]", "Signature Component 小节既没填也没删"],
  ["是占位值", "讲占位身份色的那句模板说明没删"],
  ["型删掉本节及以下三节", "小节标题里的模板条件没去掉"],
  ["本 App 有租户级可维护的枚举/分类表时", "小节标题里的模板条件没去掉"],
];

/** CLAUDE.md 正文本来就有的两个方括号（Goal-Driven Execution 的示例）。 */
const BRACKET_OK = new Set(["步骤", "检查点"]);

for (const [name, src] of [["CLAUDE.md", claude], ["PRODUCT.md", product], ["DESIGN.md", design]]) {
  if (!src) continue;
  for (const [token, why] of RESIDUE) {
    if (src.includes(token)) err(`${name} 残留 ${JSON.stringify(token)}：${why}`);
  }
  // 带中文的 [方括号] 都是模板槽位；markdown 链接 [文字](url) 与引用 [文字][id] 除外。
  for (const m of src.matchAll(/\[([^\][\n]*)\]/g)) {
    const next = src[m.index + m[0].length];
    if (next === "(" || next === "[") continue;
    const inner = m[1].trim();
    if (!CJK_RE.test(inner)) continue;
    if (name === "CLAUDE.md" && BRACKET_OK.has(inner)) continue;
    err(`${name} 残留占位 ${JSON.stringify(m[0].slice(0, 40))}：方括号槽位没填`);
  }
  // C-6：门户仓的文档目标仓访问不到，别在生成的文件里引它的路径。
  for (const m of src.matchAll(/`(apps|packages|docs)\/[^`\s]+`/g)) {
    warn(`${name} 出现仓路径样式 ${m[0]} —— 若指的是门户仓，目标仓访问不到，请改成 skill 名`);
  }
}

/* ── 2. PRODUCT.md：七段结构 + Register 裸词 ────────────────────── */

const PRODUCT_H2 = [
  "Register",
  "Users",
  "Product Purpose",
  "Brand Personality",
  "Anti-references",
  "Design Principles",
  "Accessibility & Inclusion",
];

if (product) {
  const got = h2(product);
  if (got.join(" | ") !== PRODUCT_H2.join(" | ")) {
    err(
      `PRODUCT.md 的二级标题必须是这七个、按这个顺序（impeccable 逐字解析）：\n` +
        `      期望  ${PRODUCT_H2.join(" / ")}\n` +
        `      实际  ${got.length ? got.join(" / ") : "（一个都没有）"}`
    );
  } else {
    ok("PRODUCT.md 七段结构正确");
  }

  const body = product.split(/^## /m)[1]; // "Register\n\nproduct\n\n"
  const lines = (body ?? "")
    .split("\n")
    .slice(1)
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith("<!--"));
  if (lines.length !== 1 || !["product", "brand"].includes(lines[0])) {
    err(`PRODUCT.md 的 ## Register 正文只能是裸词 product 或 brand（不加句号、不加解释），实际：${JSON.stringify(lines.join(" ").slice(0, 60))}`);
  } else {
    ok(`PRODUCT.md Register = ${lines[0]}`);
  }
}

/* ── 3. CLAUDE.md：key / type / PREFIX / 按形态裁剪 ─────────────── */

const MICRO_SECTIONS = ["### 前端：版头归门户", "### 下拉框：", "### App 图标："];
const DICT_SECTION = "### 字典表统一带"; // micro 型也可按需删，只在 service 型断言不存在

let type = null;
let key = null;

if (claude) {
  const t = claude.match(/type:\s*(micro|service)\b/);
  if (!t) err("CLAUDE.md 找不到 `type: micro` 或 `type: service`（Project Conventions 首句里的形态声明）");
  else {
    type = t[1];
    ok(`CLAUDE.md type = ${type}`);
  }

  const line = claude.split("\n").find((l) => l.includes("本仓交付的是"));
  const k = line?.match(/`([^`]+)`/);
  if (!k) err("CLAUDE.md 找不到 Project Conventions 首句（`本仓交付的是 … App：\\`<key>\\`…`），无法核对 key");
  else if (!KEY_RE.test(k[1])) err(`CLAUDE.md 里的 key \`${k[1]}\` 不合法，必须匹配 ${KEY_RE}`);
  else {
    key = k[1];
    ok(`CLAUDE.md key = ${key}`);
  }

  if (key) {
    const expect = key.toUpperCase().replace(/-/g, "_");
    const sa = claude.match(/([A-Z0-9_]+)_SA_CLIENT_SECRET/);
    if (!sa) err("CLAUDE.md 找不到 `<PREFIX>_SA_CLIENT_SECRET`（服务账号密钥那条）");
    else if (sa[1] !== expect) err(`CLAUDE.md 的 ${sa[1]}_SA_CLIENT_SECRET 与 key \`${key}\` 不一致，应为 ${expect}_SA_CLIENT_SECRET（key 大写、连字符换下划线）`);
    else ok(`CLAUDE.md PREFIX = ${expect}`);
  }

  if (type === "service") {
    for (const s of [...MICRO_SECTIONS, DICT_SECTION]) {
      if (claude.includes(s)) err(`CLAUDE.md 是 service 型（无前端），应删掉 \`${s}…\` 这一节`);
    }
    for (const s of ["DESIGN.md", "impeccable"]) {
      if (claude.includes(s)) err(`CLAUDE.md 是 service 型，## Design Context 里不该再提 ${s}`);
    }
  } else if (type === "micro") {
    for (const s of MICRO_SECTIONS) {
      if (!claude.includes(s)) err(`CLAUDE.md 是 micro 型，缺 \`${s}…\` 这一节`);
    }
  }

  if (key && manifest?.listingKey && manifest.listingKey !== key) {
    err(`CLAUDE.md 的 key \`${key}\` 与 manifest.listingKey \`${manifest.listingKey}\` 不一致`);
  }
  if (type && manifest?.type && manifest.type !== type) {
    err(`CLAUDE.md 的 type \`${type}\` 与 manifest.type \`${manifest.type}\` 不一致`);
  }
}

/* ── 4. DESIGN.md：存在性随形态；frontmatter + 六段 ─────────────── */

if (type === "service" && design) err("service 型（无前端）不需要 DESIGN.md，请删掉");
if (type === "micro" && !design) err("micro 型缺 DESIGN.md");

const DESIGN_H2 = ["1. Overview", "2. Colors", "3. Typography", "4. Elevation", "5. Components", "6. Do's and Don'ts"];

if (design) {
  const lines = design.split("\n");
  if (lines[0] !== "---") {
    err(`DESIGN.md 第 1 行必须是 \`---\`（YAML frontmatter 不在开头就整块解析不出来），实际：${JSON.stringify(lines[0].slice(0, 40))}`);
  } else {
    ok("DESIGN.md frontmatter 从第 1 行开始");
  }

  const end = lines.indexOf("---", 1);
  const fm = end > 0 ? lines.slice(1, end).join("\n") : "";
  if (!fm) err("DESIGN.md 的 frontmatter 没有闭合的 `---`");

  for (const field of ["name", "description"]) {
    const m = fm.match(new RegExp(`^${field}:\\s*(.+)$`, "m"));
    if (!m) {
      err(`DESIGN.md frontmatter 缺 ${field}`);
      continue;
    }
    try {
      const v = JSON.parse(m[1].trim());
      if (typeof v !== "string" || !v) throw new Error("不是非空字符串");
      ok(`DESIGN.md ${field} = ${JSON.stringify(v.slice(0, 30))}`);
    } catch {
      err(`DESIGN.md frontmatter 的 ${field} 必须是带双引号的合法字符串（值里的 " 与 \\ 要转义），实际：${m[1].trim().slice(0, 40)}`);
    }
  }

  const id = fm.match(/^\s*app-identity:\s*(.+)$/m);
  if (!id) err("DESIGN.md frontmatter 缺 colors.app-identity");
  else {
    const v = id[1].trim().replace(/^"|"$/g, "");
    if (!/^#[0-9A-Fa-f]{6}$/.test(v)) err(`DESIGN.md 的 app-identity 必须是 "#RRGGBB"，实际：${JSON.stringify(v)}`);
    else if (manifest?.color && manifest.color.toUpperCase() !== v.toUpperCase()) {
      err(`DESIGN.md 的 app-identity ${v} 与 manifest.color ${manifest.color} 不同 —— 一个在壳里一个在房里，色不一样就是同一个 App 说两种话`);
    } else ok(`DESIGN.md app-identity = ${v}${manifest?.color ? "（与 manifest.color 一致）" : ""}`);
  }

  const got = h2(design);
  if (got.join(" | ") !== DESIGN_H2.join(" | ")) {
    err(
      `DESIGN.md 正文必须恰好六段、名字与顺序照抄（Google Stitch 格式，impeccable 逐字解析）：\n` +
        `      期望  ${DESIGN_H2.join(" / ")}\n` +
        `      实际  ${got.length ? got.join(" / ") : "（一个都没有）"}`
    );
  } else {
    ok("DESIGN.md 六段结构正确");
  }

  // 防过度替换：这两个不是槽位。
  for (const [token, why] of [
    ["Page<T>", "平台分页契约的类型名"],
    ["{colors.app-identity}", "frontmatter 的 token 引用语法"],
  ]) {
    if (!design.includes(token)) err(`DESIGN.md 里 \`${token}\`（${why}）被改掉了 —— 它不是槽位，请改回来`);
  }
}

/* ── 输出 ───────────────────────────────────────────────────────── */

for (const m of oks) console.log(`  ✓ ${m}`);
for (const m of warns) console.log(`  ! ${m}`);
for (const m of errs) console.log(`  ✗ ${m}`);
console.log(`\n${errs.length ? "✗" : "✓"} ${root}：${errs.length} 个错误，${warns.length} 个提醒`);
process.exit(errs.length ? 1 : 0);
