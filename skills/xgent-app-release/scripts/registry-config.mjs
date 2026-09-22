#!/usr/bin/env node
/**
 * xgent-app-release · 本地配置文件的读取（`.xgent-registry.env`）
 *
 * 从 preflight.mjs 抽出来的**唯一**一份实现 —— 两个脚本读同一个文件、按同一套优先级
 * （参数 > 环境变量 > 文件）取值。抄成两份的下场是：换个字段名只改了一处，另一个脚本
 * 悄悄退回默认值，报出来的错指向服务端。
 *
 * 零依赖，纯 Node ≥18。可以整个文件拷进任何 App 仓。
 */
import { readFileSync, statSync, existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

const isFile = (p) => {
  try { return existsSync(p) && statSync(p).isFile(); } catch { return false; }
};

/** 按固定顺序找配置文件；`explicit`（--config）或 `XGENT_REGISTRY_CONFIG` 优先。 */
export function findConfigPath(explicit) {
  const named = explicit ?? process.env.XGENT_REGISTRY_CONFIG;
  if (named) {
    if (!isFile(named)) { console.error(`✗ 配置文件不存在：${named}`); process.exit(1); }
    return named;
  }
  return [
    "./.xgent-registry.env",
    join(process.env.XDG_CONFIG_HOME || join(homedir(), ".config"), "xgent", "registry.env"),
    join(homedir(), ".xgent-registry.env"),
  ].find(isFile) ?? null;
}

/** 读出整份 KEY=VALUE（支持 `export ` 前缀与成对引号）。找不到文件回空对象。 */
export function loadConfig(explicit) {
  const path = findConfigPath(explicit);
  if (!path) return {};
  const cfg = {};
  for (const line of readFileSync(path, "utf8").split("\n")) {
    if (line.trim().startsWith("#")) continue;
    const m = line.match(/^\s*(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/);
    if (m) cfg[m[1]] = m[2].trim().replace(/^(['"])(.*)\1$/, "$2");
  }
  return cfg;
}

/** 第一个非空值。 */
export const pick = (...v) => v.find((x) => x != null && String(x).trim() !== "") ?? "";

/** 极简 argv 解析：`--flag value` 与 `--bool`（列在 bools 里的）。 */
export function parseArgs(argv, bools = []) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const name = a.slice(2);
      if (bools.includes(name)) out[name] = true;
      else out[name] = argv[++i];
    } else out._.push(a);
  }
  return out;
}

/**
 * 本机配了代理，而 Node 内置的 `fetch` **不读**那几个变量（curl / git / npm / docker 全都读）。
 * 这两个脚本只发小请求，下行正常时看不出来，所以这里**只告警、不做任何事**——真正会被它
 * 拖死的是 `publish` 的上行，那一步由 `@xgent/release-cli` ≥0.6.0 自己带开关重启进程解决。
 * 只打变量名：值里可能带 `user:password@`，而这行字要进 CI 日志。
 */
export function warnIfProxyIgnored() {
  const name = ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"].find(
    (k) => String(process.env[k] ?? "").trim() !== "",
  );
  if (!name) return;
  console.error(`! 检测到 ${name}，但 Node 内置 fetch 不读它 —— 本脚本这几个小请求直连。`);
  console.error("  发版上行慢/超时是同一个原因：用 @xgent/release-cli ≥0.6.0，它会自己带 NODE_USE_ENV_PROXY 重启走代理。");
}

/** 三样发布必需项，按 参数 > 环境变量 > 文件 取。 */
export function resolveRelease(args, cfg) {
  return {
    key: String(pick(args.key, process.env.LISTING_KEY, cfg.LISTING_KEY)),
    // TARGET_XGENT_PLATFORM 是新名字，XGENT_PORTAL_URL 保留兼容（同一件事）。
    portal: String(
      pick(args.portal, process.env.TARGET_XGENT_PLATFORM, process.env.XGENT_PORTAL_URL, cfg.TARGET_XGENT_PLATFORM, cfg.XGENT_PORTAL_URL),
    ).replace(/\/+$/, ""),
    token: String(pick(process.env.XGENT_RELEASE_TOKEN, cfg.XGENT_RELEASE_TOKEN)).trim(),
  };
}
