#!/usr/bin/env node
// xgent-skills — 为目标项目安装 XGENT 的 Claude Code hooks、启用相关 settings,
// 并把 vendor 的 impeccable 一并装上:skills 与 hooks 随包,engine 二进制按需从 R2 取。
// 零依赖,Node >= 18。

'use strict';

const { spawnSync } = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const readline = require('node:readline');

const PKG_ROOT = path.join(__dirname, '..');
const HOOKS_SRC_DIR = path.join(PKG_ROOT, '.claude', 'hooks');
const VENDOR_DIR = path.join(PKG_ROOT, 'vendor', 'impeccable');
const BUNDLE_DIR = path.join(VENDOR_DIR, 'bundle');
const BLOBS_DIR = path.join(BUNDLE_DIR, 'blobs');
const MANIFEST_FILE = path.join(BUNDLE_DIR, 'manifest.json');
const AGENTS_TEMPLATE = path.join(PKG_ROOT, 'skills', 'xgent-init', 'references', 'external-app-AGENTS.template.md');
const CONTEXT_GOAL_GUARD_SRC = path.join(PKG_ROOT, 'hooks', 'codex', 'context-goal-guard.py');
// 不放进 .claude/hooks:那里的文件会被无条件复制到目标项目,绕过用户选择。
const CLAUDE_CONTEXT_GOAL_GUARD_SRC = path.join(PKG_ROOT, 'hooks', 'claude', 'context-goal-guard.js');
// 与两份脚本内置的默认阈值一致;安装器总是把阈值写进 handler 命令,在 /hooks 里可见。
const CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD = 65;
const CLAUDE_CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD = 70;

// 写入目标项目 .claude/settings.json 的配置,按顶层 key 合并:
// 这里列出的 key 以本包为准覆盖,其余已有配置保持不动。
const MANAGED_SETTINGS = {
  statusLine: {
    type: 'command',
    command: 'node "$CLAUDE_PROJECT_DIR/.claude/hooks/xgent-statusline.js"',
    padding: 0,
  },
};

// ─── impeccable(vendor) ──────────────────────────────────────────────────────
// vendor/impeccable/bundle 是上游 universal.zip 去重后的样子:文件内容存进
// blobs/<sha256>,manifest.json 里每个 harness 一张 path → sha 的清单(上游给
// 19 个 harness 各放一整套,83% 的字节是同一批文件)。装的时候按清单把文件写
// 回去,结果与直接展开 zip 一致。装法照抄上游 `impeccable install` 的工程内
// (project scope)行为:写 skills/agents/commands,hook manifest 按标记剔旧再合并。

// .codex 只承载 Codex 的 hook manifest,skills 走 .agents(上游同此约定)。
const HOOK_ONLY_PROVIDERS = new Set(['.codex']);

// 项目里一个 harness 目录都没有时的兜底。上游默认 .claude + .agents;这里只留
// .claude —— 本命令本来就是给 Claude Code 装 hooks 与 statusline 的,其它 harness
// 只在项目自己有对应目录时才装,想手动指定用 --providers。
const DEFAULT_TARGETS = ['.claude'];

// 每个 harness 的 hook manifest:bundle 里的来源 → 项目里的落点。
// .claude 的落点是 settings.local.json,但如果共享的 settings.json 已经带了
// impeccable 的 hook,就以它为准(shared)。
const HOOK_ARTIFACTS = {
  '.claude': { src: ['.claude', 'settings.json'], dest: ['.claude', 'settings.local.json'], shared: ['.claude', 'settings.json'] },
  '.cursor': { src: ['.cursor', 'hooks.json'], dest: ['.cursor', 'hooks.json'] },
  '.agents': { src: ['.codex', 'hooks.json'], dest: ['.codex', 'hooks.json'] },
  '.github': { src: ['.github', 'hooks', 'impeccable.json'], dest: ['.github', 'hooks', 'impeccable.json'] },
  '.grok': { src: ['.grok', 'hooks', 'impeccable.json'], dest: ['.grok', 'hooks', 'impeccable.json'] },
};

// hook 命令里的 launcher 路径。.github 不在表里:它的 manifest 是团队共享、
// 用 $(git rev-parse --show-toplevel) 定位的可移植形式,不能改写成别的路径。
const HOOK_LAUNCHER_REL = {
  '.claude': '${CLAUDE_PROJECT_DIR}/.claude/skills/impeccable/scripts/impeccable',
  '.cursor': '.cursor/skills/impeccable/scripts/impeccable',
  '.agents': '.agents/skills/impeccable/scripts/impeccable',
  '.grok': '.grok/skills/impeccable/scripts/impeccable',
};

// 认领 impeccable 自己写的 hook 条目(剔旧用)。两代写法都要认:JS 时代的
// hook*.mjs,和现在的 launcher。与上游 crates/context/src/hook_markers.rs 对齐。
const LEGACY_HOOK_SCRIPT_MARKERS = [
  'skills/impeccable/scripts/hook-probe.mjs',
  'skills/impeccable/scripts/hook.mjs',
  'skills/impeccable/scripts/hook-before-edit.mjs',
  'skills/impeccable/scripts/hook-after-edit.mjs',
  'skills/impeccable/scripts/hook-stop.mjs',
];
const LAUNCHER_HOOK_MARKER = /skills\/impeccable\/scripts\/impeccable(?:\.cmd|\.exe)?["']?\s+hook(?:-before-edit|-probe|-after-edit|-stop)?(?:\s|$|["'&|;)])/;

// 反斜杠统一成正斜杠,行首或引号后的连续两个保留成 UNC 前缀。
function normalizeHookSeparators(command) {
  let out = '';
  let prev = null;
  let i = 0;
  while (i < command.length) {
    const ch = command[i];
    if (ch !== '\\' && ch !== '/') {
      out += ch;
      prev = ch;
      i += 1;
      continue;
    }
    let run = 0;
    while (i < command.length && (command[i] === '\\' || command[i] === '/')) {
      run += 1;
      i += 1;
    }
    out += '/';
    if (run >= 2 && (prev === null || prev === '"' || prev === "'")) out += '/';
    prev = '/';
  }
  return out;
}

function isImpeccableHookCommand(command) {
  const normalized = normalizeHookSeparators(command);
  return LEGACY_HOOK_SCRIPT_MARKERS.some((m) => normalized.includes(m)) || LAUNCHER_HOOK_MARKER.test(normalized);
}

function valueHasHookMarker(value) {
  if (typeof value === 'string') return isImpeccableHookCommand(value);
  if (Array.isArray(value)) return value.some(valueHasHookMarker);
  if (value && typeof value === 'object') return Object.values(value).some(valueHasHookMarker);
  return false;
}

function hookVerb(provider) {
  return provider === '.cursor' ? 'hook-before-edit' : 'hook';
}

function guardedHookCommand(provider) {
  const quoted = JSON.stringify(HOOK_LAUNCHER_REL[provider]);
  return `[ ! -f ${quoted} ] || ${quoted} ${hookVerb(provider)}`;
}

function windowsHookCommand(provider) {
  const quoted = JSON.stringify(`${HOOK_LAUNCHER_REL[provider]}.cmd`);
  return `if exist ${quoted} (${quoted} ${hookVerb(provider)} & exit /b)`;
}

// bundle 里的命令按 harness 目录写死(Codex 的写成了 .codex/...),装到项目里
// 要改写成该 harness 实际的 skill 路径。
function rewriteHookValue(value, provider) {
  if (!HOOK_LAUNCHER_REL[provider]) return value;
  if (typeof value === 'string') return isImpeccableHookCommand(value) ? guardedHookCommand(provider) : value;
  if (Array.isArray(value)) return value.map((v) => rewriteHookValue(v, provider));
  if (value && typeof value === 'object') {
    const next = {};
    for (const [key, item] of Object.entries(value)) next[key] = rewriteHookValue(item, provider);
    if (provider === '.agents' && typeof value.command === 'string' && isImpeccableHookCommand(value.command)) {
      next.commandWindows = windowsHookCommand(provider);
    }
    return next;
  }
  return value;
}

function stripHookEntry(entry) {
  if (!entry || typeof entry !== 'object' || Array.isArray(entry)) return entry;
  for (const key of ['command', 'commandWindows', 'args', 'bash', 'powershell']) {
    if (valueHasHookMarker(entry[key])) return null;
  }
  if (!Array.isArray(entry.hooks)) return entry;
  const kept = entry.hooks.map(stripHookEntry).filter((e) => e !== null);
  if (kept.length === 0 && entry.hooks.some(valueHasHookMarker)) return null;
  return { ...entry, hooks: kept };
}

function stripHookEntries(entries) {
  if (!Array.isArray(entries)) return [];
  return entries.map(stripHookEntry).filter((e) => e !== null);
}

function asObject(value) {
  return value && typeof value === 'object' && !Array.isArray(value) ? value : {};
}

// 上游 mergeHookManifests:先把已有的 impeccable 条目摘干净,再追加新的,
// 项目自己的 hook 原样保留。
function mergeHookManifests(existing, fresh) {
  const existingHooks = asObject(asObject(existing).hooks);
  const freshHooks = asObject(asObject(fresh).hooks);
  const merged = { ...asObject(existing) };
  if (fresh.version !== undefined) merged.version = fresh.version;
  if (fresh.description !== undefined) merged.description = fresh.description;

  const events = Object.keys(existingHooks);
  for (const event of Object.keys(freshHooks)) {
    if (!events.includes(event)) events.push(event);
  }
  const hooks = {};
  for (const event of events) {
    const entries = stripHookEntries(existingHooks[event]);
    if (Array.isArray(freshHooks[event])) entries.push(...freshHooks[event]);
    if (entries.length > 0) hooks[event] = entries;
  }
  merged.hooks = hooks;
  return merged;
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function sha256(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

// 用户设了代理就必须走代理,不能绕过去直连。curl 按 scheme 分:http_proxy 只管
// http://,而我们的地址都是 https://,所以只设了 HTTP_PROXY 的机器上它会直连 ——
// 那不是用户的本意。这里把它补成 https_proxy 交给 curl。已经设了 https/all 的
// 就原样不动,no_proxy 照旧由 curl 自己判。
function proxyEnv() {
  const env = process.env;
  if (env.https_proxy || env.HTTPS_PROXY || env.all_proxy || env.ALL_PROXY) return env;
  const http = env.http_proxy || env.HTTP_PROXY;
  return http ? { ...env, https_proxy: http } : env;
}

let manifestCache;
function bundleManifest() {
  if (!manifestCache) manifestCache = asObject(readJson(MANIFEST_FILE).providers);
  return manifestCache;
}

function providerFiles(provider) {
  return asObject(asObject(bundleManifest()[provider]).files);
}

function providerExec(provider) {
  const exec = asObject(bundleManifest()[provider]).exec;
  return new Set(Array.isArray(exec) ? exec : []);
}

function readBundleFile(provider, rel) {
  const sha = providerFiles(provider)[rel];
  return sha ? fs.readFileSync(path.join(BLOBS_DIR, sha)) : undefined;
}

// 清单里某个前缀下的条目,键换成相对该前缀的路径。
function entriesUnder(provider, prefix) {
  return Object.entries(providerFiles(provider))
    .filter(([rel]) => rel.startsWith(prefix))
    .map(([rel, sha]) => [rel.slice(prefix.length), sha]);
}

function tryReadJson(file) {
  try {
    return readJson(file);
  } catch {
    return undefined;
  }
}

function fileHasHookMarker(file) {
  if (!fs.existsSync(file)) return false;
  const parsed = tryReadJson(file);
  return !!parsed && valueHasHookMarker(asObject(parsed).hooks);
}

// 共享 manifest 已经带 hook 时,把本地那份里的 impeccable 条目摘掉,避免装两遍。
function pruneHookManifest(file) {
  if (!fileHasHookMarker(file)) return false;
  const parsed = tryReadJson(file);
  if (!parsed) return false;
  const hooks = {};
  for (const [event, entries] of Object.entries(asObject(parsed.hooks))) {
    const kept = stripHookEntries(entries);
    if (kept.length > 0) hooks[event] = kept;
  }
  const next = { ...parsed };
  if (Object.keys(hooks).length > 0) {
    next.hooks = hooks;
  } else {
    delete next.hooks;
    delete next.description;
    delete next.version;
  }
  if (Object.keys(next).length === 0) {
    fs.rmSync(file, { force: true });
  } else {
    fs.writeFileSync(file, JSON.stringify(next, null, 2) + '\n');
  }
  return true;
}

function listFilesRecursive(dir, prefix = '') {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      files.push(...listFilesRecursive(path.join(dir, entry.name), rel));
    } else {
      files.push(rel);
    }
  }
  return files;
}

// 已装的那棵树和清单是不是同一套:文件名单一致,且每个文件的 sha256 对得上。
function treeMatchesStore(dest, entries) {
  if (!fs.existsSync(dest)) return false;
  const installed = listFilesRecursive(dest).sort();
  const wanted = entries.map(([rel]) => rel).sort();
  if (installed.length !== wanted.length || installed.some((f, i) => f !== wanted[i])) return false;
  return entries.every(([rel, sha]) => sha256(fs.readFileSync(path.join(dest, rel))) === sha);
}

function writeBundleFile(dest, sha, executable) {
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.copyFileSync(path.join(BLOBS_DIR, sha), dest);
  fs.chmodSync(dest, executable ? 0o755 : 0o644);
}

// 复制后补上执行位:zip 经某些解压器或文件同步后会丢 +x,launcher 一丢 +x
// 就每次编辑都 Permission denied。
function restoreExecutableBits(skillDir) {
  for (const name of ['impeccable', 'impeccable.cmd']) {
    const file = path.join(skillDir, 'scripts', name);
    if (fs.existsSync(file)) fs.chmodSync(file, 0o755);
  }
  const binDir = path.join(skillDir, 'scripts', 'bin');
  if (!fs.existsSync(binDir)) return;
  for (const rel of listFilesRecursive(binDir)) fs.chmodSync(path.join(binDir, rel), 0o755);
}

function bundleProviders() {
  return Object.keys(bundleManifest())
    .filter((name) => !HOOK_ONLY_PROVIDERS.has(name))
    .sort();
}

// 探测要在本命令自己创建 .claude 之前做,否则任何项目看上去都"已经在用
// Claude Code"。
function detectProviders(targetDir) {
  return bundleProviders().filter((p) => fs.existsSync(path.join(targetDir, p)));
}

function resolveProviders(detected, requested) {
  const available = bundleProviders();
  if (requested) {
    const wanted = requested
      .split(',')
      .map((v) => v.trim())
      .filter(Boolean)
      .map((v) => (v.startsWith('.') ? v : `.${v}`));
    const unknown = wanted.filter((v) => !available.includes(v));
    if (unknown.length > 0) {
      console.error(`错误: 未知的 harness 目录: ${unknown.join(', ')}`);
      console.error(`可选: ${available.join(', ')}`);
      process.exit(1);
    }
    return wanted;
  }
  return detected.length > 0 ? detected : DEFAULT_TARGETS;
}

function writeFileIfChanged(dest, content) {
  if (fs.existsSync(dest) && content.equals(fs.readFileSync(dest))) return false;
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, content, { mode: 0o644 });
  return true;
}

function installProviderSkills(targetDir, provider, force) {
  const exec = providerExec(provider);
  const names = [...new Set(entriesUnder(provider, 'skills/').map(([rel]) => rel.split('/')[0]))].sort();
  for (const name of names) {
    const prefix = `skills/${name}/`;
    const entries = entriesUnder(provider, prefix);
    const dest = path.join(targetDir, provider, 'skills', name);
    if (!force && treeMatchesStore(dest, entries)) {
      console.log(`  未变  ${provider}/skills/${name}`);
      restoreExecutableBits(dest);
      continue;
    }
    const status = fs.existsSync(dest) ? '更新' : '安装';
    fs.rmSync(dest, { recursive: true, force: true });
    for (const [rel, sha] of entries) writeBundleFile(path.join(dest, rel), sha, exec.has(`${prefix}${rel}`));
    restoreExecutableBits(dest);
    console.log(`  ${status}  ${provider}/skills/${name}`);
  }
}

function installProviderFiles(targetDir, provider, kind) {
  // 只要该目录下的直属文件,和上游 install 一样不递归。
  const entries = entriesUnder(provider, `${kind}/`).filter(([rel]) => !rel.includes('/'));
  if (entries.length === 0) return;
  let changed = 0;
  for (const [rel, sha] of entries) {
    const dest = path.join(targetDir, provider, kind, rel);
    if (writeFileIfChanged(dest, fs.readFileSync(path.join(BLOBS_DIR, sha)))) changed += 1;
  }
  console.log(`  ${changed > 0 ? '写入' : '未变'}  ${provider}/${kind}/ (${entries.length} 个文件)`);
}

function installProviderHooks(targetDir, provider, force) {
  const artifact = HOOK_ARTIFACTS[provider];
  if (!artifact) return;
  const srcContent = readBundleFile(artifact.src[0], artifact.src.slice(1).join('/'));
  if (!srcContent) return;
  const dest = path.join(targetDir, ...artifact.dest);
  const destRel = path.relative(targetDir, dest);

  if (artifact.shared) {
    const shared = path.join(targetDir, ...artifact.shared);
    if (shared !== dest && fileHasHookMarker(shared)) {
      pruneHookManifest(dest);
      console.log(`  跳过  ${destRel}(${path.relative(targetDir, shared)} 里已有 impeccable hook)`);
      return;
    }
  }

  const fresh = rewriteHookValue(JSON.parse(srcContent.toString('utf8')), provider);
  let next = fresh;
  if (fs.existsSync(dest)) {
    const existing = tryReadJson(dest);
    if (existing === undefined) {
      if (!force) {
        console.error(`错误: 已有的 hook 配置不是合法 JSON: ${dest}`);
        console.error('请先手动修复,或加 --force 让本命令备份为 .bak 后覆盖。');
        process.exit(1);
      }
      fs.copyFileSync(dest, `${dest}.bak`);
    } else {
      next = mergeHookManifests(existing, fresh);
    }
  }
  const content = JSON.stringify(next, null, 2) + '\n';
  if (fs.existsSync(dest) && fs.readFileSync(dest, 'utf8') === content) {
    console.log(`  未变  ${destRel}`);
    return;
  }
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, content);
  console.log(`  写入  ${destRel} (impeccable hooks)`);
}

// 二进制不随 npm 包分发(一个平台十几 MB,见 package.json 的 files),从 R2 取:
// 地址与 sha256 都在 VERSION.json 里,由维护者跑 scripts/publish-vendor-r2.mjs 写入。
// 从源码仓库跑时 vendor/impeccable/engine 就在手边,优先用它,不必联网。
function readEngineBinary(meta, target, engine) {
  const src = path.join(VENDOR_DIR, 'engine', target, 'impeccable');
  if (fs.existsSync(src)) return fs.readFileSync(src);
  if (!engine.url) {
    console.log(`  跳过  engine 二进制:VERSION.json 里没有 ${target} 的下载地址(维护者需跑 node scripts/publish-vendor-r2.mjs)`);
    return null;
  }
  console.log(`  下载  engine v${meta.engineVersion} (${(engine.size / 1024 / 1024).toFixed(1)} MB) ← ${engine.url}`);
  // 走系统 curl 而不是 fetch:它认 http_proxy/https_proxy,自带重试,也让这段
  // 保持同步,不必把整条安装流程改成异步。
  const tmp = path.join(os.tmpdir(), `xgent-impeccable-engine-${process.pid}`);
  try {
    const result = spawnSync('curl', ['-fsSL', '--retry', '3', '--retry-delay', '2', '--max-time', '600', '-o', tmp, engine.url], { stdio: ['ignore', 'ignore', 'inherit'], env: proxyEnv() });
    if (result.status !== 0) {
      console.log(`  跳过  engine 二进制:下载失败(curl 退出码 ${result.status}),首次运行 hook 时会自己联网下载`);
      return null;
    }
    return fs.readFileSync(tmp);
  } finally {
    fs.rmSync(tmp, { force: true });
  }
}

// engine 二进制放进 ~/.impeccable/bin/<版本>/:launcher 和 npx impeccable 都认
// 这个版本化缓存,一台机器装一份,所有项目共用,之后运行也就不必联网了。
// 只收了 Apple Silicon macOS 的二进制,别的平台落到"跳过",由 launcher 首次运行时自己去
// GitHub 下载(上游原本的行为)。
function installEngineBinary(meta, force) {
  const arch = { arm64: 'arm64', x64: 'x64' }[process.arch] || process.arch;
  const target = `${process.platform === 'darwin' ? 'darwin' : process.platform}-${arch}`;
  const engine = meta.engines && meta.engines[target];
  if (!engine) {
    console.log(`  跳过  engine 二进制:只收了 Apple Silicon macOS 的版本,${target} 会在首次运行时联网下载`);
    return;
  }
  const cacheRoot = process.env.IMPECCABLE_HOME || path.join(os.homedir(), '.impeccable');
  const dest = path.join(cacheRoot, 'bin', meta.engineVersion, 'impeccable');
  const home = os.homedir();
  const destLabel = dest.startsWith(`${home}${path.sep}`) ? path.join('~', path.relative(home, dest)) : dest;
  // 缓存里已经是对的那一份就别再下一遍十几 MB。
  if (!force && fs.existsSync(dest) && sha256(fs.readFileSync(dest)) === engine.sha256) {
    console.log(`  未变  ${destLabel}`);
    return;
  }
  const content = readEngineBinary(meta, target, engine);
  if (!content) return;
  if (sha256(content) !== engine.sha256) {
    console.log(`  跳过  engine 二进制:sha256 与 VERSION.json 不符,没有落盘`);
    return;
  }
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, content, { mode: 0o755 });
  fs.chmodSync(dest, 0o755);
  console.log(`  写入  ${destLabel} (engine v${meta.engineVersion})`);
}

function installImpeccable(targetDir, options) {
  if (!fs.existsSync(MANIFEST_FILE)) {
    console.log('  跳过  impeccable:本包里没有 vendor/impeccable(先跑 node scripts/vendor-impeccable.mjs)');
    return;
  }
  const meta = readJson(path.join(VENDOR_DIR, 'VERSION.json'));
  const providers = resolveProviders(options.detected, options.providers);
  console.log(`impeccable skill v${meta.skillVersion} / engine v${meta.engineVersion} → ${providers.join(', ')}`);
  for (const provider of providers) {
    installProviderSkills(targetDir, provider, options.force);
    installProviderFiles(targetDir, provider, 'agents');
    installProviderFiles(targetDir, provider, 'commands');
    installProviderHooks(targetDir, provider, options.force);
  }
  installEngineBinary(meta, options.force);
}

// ─── install ────────────────────────────────────────────────────────────────

async function confirm(question, skipMessage, answers) {
  if (!process.stdin.isTTY) {
    console.log(skipMessage);
    return false;
  }
  process.stdout.write(question);
  const { value = '' } = await answers.next();
  return /^(y|yes)$/i.test(value.trim());
}

// 重装没给阈值 flag 时沿用旧 handler 里的值。
function guardThreshold(command) {
  return Number(/ --threshold ([1-9][0-9]?)$/.exec(command)?.[1]) || undefined;
}

function installContextGoalGuard(targetDir, force, threshold) {
  const python = spawnSync('python3', ['-c', 'import sys, sqlite3; sys.exit(0 if sys.version_info >= (3, 9) else 1)'],
    { encoding: 'utf8', timeout: 5000 });
  if (python.status !== 0) {
    throw new Error('Codex goal guard 需要含 sqlite3 的 Python 3.9 或更高版本。请先安装并用 python3 --version 核对,确保 Codex 运行环境也能找到 python3,再重试;或加 --no-context-goal-guard 跳过。');
  }
  const manifestPath = path.join(targetDir, '.codex', 'hooks.json');
  let existing = {};
  let backup = false;
  if (fs.existsSync(manifestPath)) {
    try {
      existing = readJson(manifestPath);
    } catch (error) {
      if (!(error instanceof SyntaxError)) throw error;
      if (!force) throw new Error(`${manifestPath} 不是合法 JSON,未修改;请先手动修复,或加 --force 备份为 .bak 后覆盖。`);
      backup = true;
    }
  }
  // 合法 JSON 的未知结构不能当成空配置,否则会丢失用户的 hooks。
  if (!existing || typeof existing !== 'object' || Array.isArray(existing)
      || (existing.hooks !== undefined && (!existing.hooks || typeof existing.hooks !== 'object' || Array.isArray(existing.hooks)))
      || Object.values(existing.hooks || {}).some((entries) => !Array.isArray(entries))) {
    throw new Error(`${manifestPath} 的 hooks 结构不受支持,未修改;请先手动修复为事件数组。`);
  }
  const scriptPath = path.join(fs.realpathSync(targetDir), '.codex', 'hooks', 'context-goal-guard.py');
  let previous;
  const hooks = { ...existing.hooks };
  for (const event of ['PreToolUse', 'Stop']) {
    const kept = [];
    for (const entry of hooks[event] || []) {
      if (!Array.isArray(entry?.hooks)) {
        kept.push(entry);
        continue;
      }
      const handlers = entry.hooks.filter((handler) => {
        const guard = handler?.type === 'command' && typeof handler.command === 'string'
          && /\.codex\/hooks\/context-goal-guard\.py(?:['"\s]|$)/.test(handler.command);
        if (guard) previous ??= guardThreshold(handler.command);
        return !guard;
      });
      if (handlers.length === entry.hooks.length) kept.push(entry);
      else if (handlers.length > 0) kept.push({ ...entry, hooks: handlers });
    }
    hooks[event] = kept;
  }
  const limit = threshold ?? previous ?? CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD;
  // POSIX 单引号同时保护空格、美元符号、反引号及项目路径中的单引号。
  const command = `python3 '${scriptPath.replace(/'/g, "'\\''")}' --threshold ${limit}`;
  for (const event of ['PreToolUse', 'Stop']) hooks[event].push({ hooks: [{ type: 'command', command, timeout: 3 }] });
  const script = fs.readFileSync(CONTEXT_GOAL_GUARD_SRC);
  const manifest = Buffer.from(JSON.stringify({ ...existing, hooks }, null, 2) + '\n');
  if (backup) fs.copyFileSync(manifestPath, `${manifestPath}.bak`);
  console.log(`  ${writeFileIfChanged(scriptPath, script) ? '写入' : '未变'}  .codex/hooks/context-goal-guard.py`);
  console.log(`  ${writeFileIfChanged(manifestPath, manifest) ? '写入' : '未变'}  .codex/hooks.json (context goal guard)`);
  return limit;
}

function printContextGoalGuardSteps(threshold) {
  console.log(`
Codex goal guard 已部署,请完成以下操作后使用:
  1. 在目标项目运行 codex --version;目前仅验证 Codex 0.160.0、macOS arm64。
     其他版本的会话格式不自动兼容;请先在隔离项目验证。
  2. 从目标项目启动 Codex,审阅并信任项目 .codex/ 层。
     在 Codex 对话中输入 /hooks,找到本项目 .codex/hooks.json 的
     PreToolUse 和 Stop 两个 context-goal-guard.py handler,分别审阅、信任并启用。
     核对两者来源和状态,确认其他 hooks 仍保留;配置或脚本更新后重新审阅信任。
  3. 安装与信任不构成暂停授权。请在当前 goal 的用户指令中明确(可复制):
     对当前 goal,如果本地估算上下文使用率严格超过 ${threshold}%,请先安全收尾,
     按已有任务约定保存进度和交接说明,再使用原生 goal 工具将其设为 paused
     并核对,无需二次确认。未完成工作不要标为 complete。请提醒我自己新开对话继续。
  4. 先用测试 goal 验证提醒与真实 paused 状态。/goal resume 撤销先前暂停请求,
     需重新明确策略;新开对话也需手动设置新 goal 并重新明确策略。
     暂停失败时在原对话输入 /goal pause;临时停用可在 /hooks 中禁用这两个 handler。
  提示: Python 3.9+ 必须在 Codex 运行环境可用;没有 active goal 时不会触发。
`);
}

// 与 installContextGoalGuard 形状相似,但文件、事件和匹配式都不同,暂不抽公共函数。
function installClaudeContextGoalGuard(targetDir, force, threshold) {
  const settingsPath = path.join(targetDir, '.claude', 'settings.local.json');
  let existing = {};
  let backup = false;
  if (fs.existsSync(settingsPath)) {
    try {
      existing = readJson(settingsPath);
    } catch (error) {
      if (!(error instanceof SyntaxError)) throw error;
      if (!force) throw new Error(`${settingsPath} 不是合法 JSON,未修改;请先手动修复,或加 --force 备份为 .bak 后覆盖。`);
      backup = true;
    }
  }
  // 合法 JSON 的未知结构不能当成空配置,否则会丢失用户的设置。
  if (!existing || typeof existing !== 'object' || Array.isArray(existing)
      || (existing.hooks !== undefined && (!existing.hooks || typeof existing.hooks !== 'object' || Array.isArray(existing.hooks)))
      || Object.values(existing.hooks || {}).some((entries) => !Array.isArray(entries))) {
    throw new Error(`${settingsPath} 的 hooks 结构不受支持,未修改;请先手动修复为事件数组。`);
  }
  let previous;
  const hooks = { ...existing.hooks };
  for (const event of ['PostToolBatch', 'Stop']) {
    const kept = [];
    for (const entry of hooks[event] || []) {
      if (!Array.isArray(entry?.hooks)) {
        kept.push(entry);
        continue;
      }
      const handlers = entry.hooks.filter((handler) => {
        const guard = handler?.type === 'command' && typeof handler.command === 'string'
          && /\.claude\/hooks\/context-goal-guard\.js(?:['"\s]|$)/.test(handler.command);
        if (guard) previous ??= guardThreshold(handler.command);
        return !guard;
      });
      if (handlers.length === entry.hooks.length) kept.push(entry);
      else if (handlers.length > 0) kept.push({ ...entry, hooks: handlers });
    }
    hooks[event] = kept;
  }
  const limit = threshold ?? previous ?? CLAUDE_CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD;
  const command = `node "$CLAUDE_PROJECT_DIR/.claude/hooks/context-goal-guard.js" --threshold ${limit}`;
  for (const event of ['PostToolBatch', 'Stop']) hooks[event].push({ hooks: [{ type: 'command', command, timeout: 5 }] });
  const script = fs.readFileSync(CLAUDE_CONTEXT_GOAL_GUARD_SRC);
  const settings = Buffer.from(JSON.stringify({ ...existing, hooks }, null, 2) + '\n');
  if (backup) fs.copyFileSync(settingsPath, `${settingsPath}.bak`);
  console.log(`  ${writeFileIfChanged(path.join(targetDir, '.claude', 'hooks', 'context-goal-guard.js'), script) ? '写入' : '未变'}  .claude/hooks/context-goal-guard.js`);
  console.log(`  ${writeFileIfChanged(settingsPath, settings) ? '写入' : '未变'}  .claude/settings.local.json (context goal guard)`);
  return limit;
}

function printClaudeContextGoalGuardSteps(threshold) {
  console.log(`
Claude Code goal guard 已部署,请完成以下操作后使用:
  1. 运行 claude --version;目前仅验证 Claude Code 2.1.288 交互式 CLI、macOS arm64,
     以及 claude-opus-5-5、claude-sonnet-5-5(1M)与 claude-haiku-4-5-20251001(200k)。
     可在 statusLine 输入的 context_window.context_window_size 核对窗口,不一致时不要启用。
  2. 重启 Claude Code 会话并信任本工作区;输入 /hooks,确认 PostToolBatch 与 Stop
     各有一个 context-goal-guard.js handler,来源为 .claude/settings.local.json,其他 hooks 仍在。
  3. 先用测试 goal 验证:估算上下文严格超过 ${threshold}% 时 agent 收到收尾提醒;回合结束时
     界面可能先出现「Stop hook error: …」(收尾请求,属正常),随后出现 Goal paused。
  4. 接续:在同一项目目录新开对话或 /clear,提供计划路径或交接摘要,并重新设置 /goal;
     新对话不继承 goal。原会话发消息或 --resume 会恢复 goal 并再次触发暂停,
     不再需要时在原会话 /goal clear。
  5. 停用:从 .claude/settings.local.json 删除这两个 handler 并重启会话;
     不要用 disableAllHooks,它会连 /goal 一起禁用。
  说明见 @xgent-ai/skills 包内 docs/claude-context-goal-guard.md;没有 active goal 时不会触发。
`);
}

function createAgents(targetDir) {
  const dest = path.join(targetDir, 'AGENTS.md');
  try {
    fs.copyFileSync(AGENTS_TEMPLATE, dest, fs.constants.COPYFILE_EXCL);
    console.log('  创建  AGENTS.md');
  } catch (error) {
    if (error.code !== 'EEXIST') throw error;
    console.log('  跳过  AGENTS.md (已存在,保留原文)');
  }
}

function usage() {
  console.log(`用法: npx @xgent-ai/skills <command>

命令:
  install [dir]   为目标项目(默认当前目录)安装 .claude/hooks 下的全部
                  hook,在 .claude/settings.json 中启用对应配置,并装上
                  vendor 的 impeccable(skills + hooks,以及按需下载的
                  engine 二进制),并询问是否创建通用的 AGENTS.md
                  与安装 Codex / Claude Code goal 上下文收尾提醒
  help            显示本帮助

install 选项:
  --no-impeccable       跳过 impeccable,仍安装 XGENT 的 hooks 与 settings
  --xgent-init          创建 AGENTS.md (已有则跳过),不再询问
  --no-xgent-init       跳过 AGENTS.md,不再询问
  --context-goal-guard  安装 Codex goal guard,不再询问(需要 Python 3.9+)
  --no-context-goal-guard 跳过 Codex goal guard,不再询问
  --context-goal-guard-threshold=N  Codex guard 的阈值百分比(1–99 整数,默认 ${CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD};
                        重装时不给则沿用已装的值),同时表示安装
  --claude-context-goal-guard 安装 Claude Code goal guard,不再询问
  --no-claude-context-goal-guard 跳过 Claude Code goal guard,不再询问
  --claude-context-goal-guard-threshold=N  Claude Code guard 的阈值百分比(1–99 整数,
                        默认 ${CLAUDE_CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD};重装时不给则沿用已装的值),同时表示安装
  --providers=a,b       指定 impeccable 装进哪些 harness 目录(默认按项目里
                        已有的目录判断,都没有时装 ${DEFAULT_TARGETS.join(' 和 ')})
  --force               强制重装,并允许覆盖非法 JSON 的 hook 配置(先存 .bak),
                        不覆盖已有 AGENTS.md
`);
}

async function install(args) {
  const flags = args.filter((a) => a.startsWith('--'));
  const dirArg = args.find((a) => !a.startsWith('--'));
  const unknown = flags.filter((f) => !['--no-impeccable', '--xgent-init', '--no-xgent-init', '--context-goal-guard', '--no-context-goal-guard', '--claude-context-goal-guard', '--no-claude-context-goal-guard', '--force'].includes(f)
    && !['--providers=', '--context-goal-guard-threshold=', '--claude-context-goal-guard-threshold='].some((p) => f.startsWith(p)));
  if (unknown.length > 0) {
    console.error(`错误: 未知选项: ${unknown.join(', ')}\n`);
    usage();
    process.exit(1);
  }
  if (flags.includes('--xgent-init') && flags.includes('--no-xgent-init')) {
    console.error('错误: --xgent-init 与 --no-xgent-init 不能同时使用');
    process.exit(1);
  }
  if (flags.includes('--context-goal-guard') && flags.includes('--no-context-goal-guard')) {
    console.error('错误: --context-goal-guard 与 --no-context-goal-guard 不能同时使用');
    process.exit(1);
  }
  if (flags.includes('--claude-context-goal-guard') && flags.includes('--no-claude-context-goal-guard')) {
    console.error('错误: --claude-context-goal-guard 与 --no-claude-context-goal-guard 不能同时使用');
    process.exit(1);
  }
  const thresholds = {};
  for (const [name, skip] of [['--context-goal-guard-threshold', '--no-context-goal-guard'],
    ['--claude-context-goal-guard-threshold', '--no-claude-context-goal-guard']]) {
    const flag = flags.find((f) => f.startsWith(`${name}=`));
    if (flag === undefined) continue;
    const value = flag.slice(name.length + 1);
    if (!/^[1-9][0-9]?$/.test(value)) {
      console.error(`错误: ${name} 需要 1–99 的整数百分比,收到 "${value}"`);
      process.exit(1);
    }
    if (flags.includes(skip)) {
      console.error(`错误: ${name} 与 ${skip} 不能同时使用`);
      process.exit(1);
    }
    thresholds[name] = Number(value);
  }
  const codexThreshold = thresholds['--context-goal-guard-threshold'];
  const claudeThreshold = thresholds['--claude-context-goal-guard-threshold'];
  const options = {
    impeccable: !flags.includes('--no-impeccable'),
    force: flags.includes('--force'),
    providers: flags.find((f) => f.startsWith('--providers='))?.slice('--providers='.length),
  };

  const targetDir = path.resolve(dirArg || process.cwd());
  if (!fs.existsSync(targetDir) || !fs.statSync(targetDir).isDirectory()) {
    console.error(`错误: 目标目录不存在: ${targetDir}`);
    process.exit(1);
  }

  options.detected = detectProviders(targetDir);
  if (options.providers) resolveProviders(options.detected, options.providers);
  const agentsExists = fs.existsSync(path.join(targetDir, 'AGENTS.md'));
  const askAgents = !flags.includes('--xgent-init') && !flags.includes('--no-xgent-init') && !agentsExists;
  const askGuard = !flags.includes('--context-goal-guard') && !flags.includes('--no-context-goal-guard')
    && codexThreshold === undefined;
  const askClaudeGuard = !flags.includes('--claude-context-goal-guard') && !flags.includes('--no-claude-context-goal-guard')
    && claudeThreshold === undefined;
  // 各询问共享输入迭代器,避免前一问提前读掉后一问的回答。
  const rl = process.stdin.isTTY && (askAgents || askGuard || askClaudeGuard)
    ? readline.createInterface({ input: process.stdin, output: process.stdout }) : null;
  const answers = rl?.[Symbol.asyncIterator]();
  rl?.once('SIGINT', () => { rl.close(); process.exit(130); });
  let agents;
  let guard;
  let claudeGuard;
  try {
    agents = flags.includes('--xgent-init') || (askAgents && await confirm(
      '是否同时创建通用的 AGENTS.md (含 Portal 联调与前端验收约定)? [y/N] ',
      '  跳过  AGENTS.md:非交互环境,可加 --xgent-init 创建', answers));
    guard = flags.includes('--context-goal-guard') || codexThreshold !== undefined || (askGuard && await confirm(
      `是否安装 Codex goal 上下文收尾提醒 (默认超过 ${CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD}% 时提醒收尾,需 Python 3.9+ 与 Codex 内手动信任)? [y/N] `,
      '  跳过  Codex goal guard:非交互环境,可加 --context-goal-guard 安装', answers));
    claudeGuard = flags.includes('--claude-context-goal-guard') || claudeThreshold !== undefined || (askClaudeGuard && await confirm(
      `是否安装 Claude Code goal 上下文收尾提醒 (默认超过 ${CLAUDE_CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD}% 时提醒收尾并在回合结束时暂停 goal,需重启会话并信任工作区)? [y/N] `,
      '  跳过  Claude Code goal guard:非交互环境,可加 --claude-context-goal-guard 安装', answers));
  } finally {
    rl?.close();
  }
  const codexLimit = guard && installContextGoalGuard(targetDir, options.force, codexThreshold);
  const claudeLimit = claudeGuard && installClaudeContextGoalGuard(targetDir, options.force, claudeThreshold);

  const hooksDestDir = path.join(targetDir, '.claude', 'hooks');
  fs.mkdirSync(hooksDestDir, { recursive: true });
  for (const name of fs.readdirSync(HOOKS_SRC_DIR)) {
    const content = fs.readFileSync(path.join(HOOKS_SRC_DIR, name));
    const dest = path.join(hooksDestDir, name);
    let status = '安装';
    if (fs.existsSync(dest)) {
      status = content.equals(fs.readFileSync(dest)) ? '未变' : '更新';
    }
    if (status !== '未变') {
      fs.writeFileSync(dest, content, { mode: 0o755 });
    }
    console.log(`  ${status}  .claude/hooks/${name}`);
  }

  const settingsPath = path.join(targetDir, '.claude', 'settings.json');
  let settings = {};
  if (fs.existsSync(settingsPath)) {
    try {
      settings = JSON.parse(fs.readFileSync(settingsPath, 'utf8'));
    } catch {
      console.error(`错误: ${settingsPath} 不是合法 JSON,未修改该文件,请先手动修复。`);
      process.exit(1);
    }
  }
  const before = JSON.stringify(settings);
  Object.assign(settings, MANAGED_SETTINGS);
  if (JSON.stringify(settings) === before) {
    console.log('  未变  .claude/settings.json');
  } else {
    fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2) + '\n');
    console.log('  写入  .claude/settings.json (statusLine)');
  }

  if (options.impeccable) {
    installImpeccable(targetDir, options);
    console.log('  提示  在 agent 对话里(不是终端)输入 /impeccable init 完成 impeccable 的设计上下文');
  }

  if (agents) createAgents(targetDir);
  else if (agentsExists && !flags.includes('--no-xgent-init')) console.log('  跳过  AGENTS.md (已存在,保留原文)');
  if (codexLimit) printContextGoalGuardSteps(codexLimit);
  if (claudeLimit) printClaudeContextGoalGuardSteps(claudeLimit);
  console.log(`完成: ${targetDir}`);
}

const [cmd, ...args] = process.argv.slice(2);
switch (cmd) {
  case 'install':
    install(args).catch((error) => {
      console.error(`错误: ${error.message}`);
      process.exitCode = 1;
    });
    break;
  case undefined:
  case 'help':
  case '--help':
  case '-h':
    usage();
    break;
  default:
    console.error(`未知命令: ${cmd}\n`);
    usage();
    process.exit(1);
}
