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

const PKG_ROOT = path.join(__dirname, '..');
const HOOKS_SRC_DIR = path.join(PKG_ROOT, '.claude', 'hooks');
const VENDOR_DIR = path.join(PKG_ROOT, 'vendor', 'impeccable');
const BUNDLE_DIR = path.join(VENDOR_DIR, 'bundle');
const BLOBS_DIR = path.join(BUNDLE_DIR, 'blobs');
const MANIFEST_FILE = path.join(BUNDLE_DIR, 'manifest.json');

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
// 只收了 macOS 的二进制,别的平台落到"跳过",由 launcher 首次运行时自己去
// GitHub 下载(上游原本的行为)。
function installEngineBinary(meta, force) {
  const arch = { arm64: 'arm64', x64: 'x64' }[process.arch] || process.arch;
  const target = `${process.platform === 'darwin' ? 'darwin' : process.platform}-${arch}`;
  const engine = meta.engines && meta.engines[target];
  if (!engine) {
    console.log(`  跳过  engine 二进制:只收了 macOS 的版本,${target} 会在首次运行时联网下载`);
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

function usage() {
  console.log(`用法: npx @xgent-ai/skills <command>

命令:
  install [dir]   为目标项目(默认当前目录)安装 .claude/hooks 下的全部
                  hook,在 .claude/settings.json 中启用对应配置,并装上
                  vendor 的 impeccable(skills + hooks,以及按需下载的
                  engine 二进制)
  help            显示本帮助

install 选项:
  --no-impeccable       只装 XGENT 的 hooks 与 settings,跳过 impeccable
  --providers=a,b       指定 impeccable 装进哪些 harness 目录(默认按项目里
                        已有的目录判断,都没有时装 ${DEFAULT_TARGETS.join(' 和 ')})
  --force               强制重装,并允许覆盖非法 JSON 的 hook 配置(先存 .bak)
`);
}

function install(args) {
  const flags = args.filter((a) => a.startsWith('--'));
  const dirArg = args.find((a) => !a.startsWith('--'));
  const unknown = flags.filter((f) => f !== '--no-impeccable' && f !== '--force' && !f.startsWith('--providers='));
  if (unknown.length > 0) {
    console.error(`错误: 未知选项: ${unknown.join(', ')}\n`);
    usage();
    process.exit(1);
  }
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

  console.log('  提示  项目文档由 xgent-init skill 生成(npx skills add XGENT-ai/skills --skill xgent-init)');
  console.log(`完成: ${targetDir}`);
}

const [cmd, ...args] = process.argv.slice(2);
switch (cmd) {
  case 'install':
    install(args);
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
