#!/usr/bin/env node
// xgent-skills — 为目标项目安装 XGENT 的 Claude Code hooks 与 agents、启用相关 settings,
// 并安装 Phoenix UI:skills 与 hooks 随包,engine 按包内 manifest 校验后放入用户 cache。
// 零依赖,Node >= 18。

'use strict';

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const readline = require('node:readline');

const PKG_ROOT = path.join(__dirname, '..');
const HOOKS_SRC_DIR = path.join(PKG_ROOT, '.claude', 'hooks');
const AGENTS_SRC_DIR = path.join(PKG_ROOT, 'agents');

// 写入目标项目 .claude/settings.json 的配置,按顶层 key 合并:
// 这里列出的 key 以本包为准覆盖,其余已有配置保持不动。
const MANAGED_SETTINGS = {
  statusLine: {
    type: 'command',
    command: 'node "$CLAUDE_PROJECT_DIR/.claude/hooks/xgent-statusline.js"',
    padding: 0,
  },
};

// ─── Phoenix UI ─────────────────────────────────────────────────────────────
// Phoenix 工具文件由 Rust 执行器直接覆盖(符号链接的 skill 目录换成副本),
// 共享 hooks 只更新本工具条目。旧工具是否删除由 npm 安装入口询问,不备份、不迁移旧数据。

const bootstrap = require('../skills/phoenix-ui/scripts/phoenix-bootstrap.cjs');
const PHOENIX_DIR = path.join(PKG_ROOT, 'dist', 'phoenix-ui');
const includesDiagram = Boolean(readJson(path.join(PKG_ROOT, 'package.json')).bin['phoenix-diagram']);
const diagramBootstrap = includesDiagram ? require('../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs') : null;
const DIAGRAM_DIR = path.join(PKG_ROOT, 'dist', 'phoenix-diagram');
const PHOENIX_STATUS = {
  planned: '计划', installed: '已安装', unchanged: '未变', conflict: '冲突,未激活',
  'recovery-required': '待恢复', 'integration-pending': '已安装但融合未就绪',
};

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function phoenixFailure(code, message) {
  return Object.assign(new Error(message), { code });
}

// 退出码沿用执行器:2 参数/路径,4 文件或运行环境。
function runPhoenix(binary, args) {
  const result = spawnSync(binary, args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, windowsHide: true });
  if (result.error) throw phoenixFailure(4, `无法运行 Phoenix UI engine: ${result.error.message}`);
  try {
    return { code: result.status, report: JSON.parse(result.stdout) };
  } catch {
    throw phoenixFailure(result.status === 2 ? 2 : 4,
      `Phoenix UI 安装执行器失败(exit ${result.status}): ${(result.stderr || '').trim()}`);
  }
}

// Old Impeccable tools in the project and the user's home. Shared settings
// keep every unrelated entry.
function preparePhoenixTools(project, providers) {
  const legacy = new Set();
  const configs = new Set();
  const present = (file) => !!fs.lstatSync(file, { throwIfNoEntry: false });
  const home = fs.realpathSync(os.homedir());
  for (const provider of providers) {
    const roots = provider === '.agents' || provider === '.codex'
      ? ['.agents', '.codex'] : [provider];
    for (const root of [project, home]) {
      for (const dir of roots) {
        for (const relative of [
          `${dir}/skills/impeccable`, `${dir}/commands/impeccable.md`,
          ...['asset-producer', 'documenter', 'finish-reviewer', 'manual-edit-applier']
            .map(name => `${dir}/agents/impeccable-${name}.md`),
        ]) {
          const file = path.join(root, relative);
          if (present(file)) legacy.add(file);
        }
        const shared = dir === '.claude' ? ['settings.json', 'settings.local.json']
          : dir === '.gemini' ? ['settings.json']
            : ['.github', '.grok'].includes(dir) ? ['hooks/impeccable.json', 'hooks/phoenix-ui.json'] : ['hooks.json'];
        for (const relative of shared) {
          const file = path.join(root, dir, relative);
          if (present(file)) configs.add(file);
        }
      }
    }
  }
  const oldCommand = value => typeof value === 'string'
    && /(?:skills[\\/]impeccable[\\/]scripts[\\/](?:impeccable(?:\.cmd)?|hook(?:-(?:probe|before-edit|after-edit|stop))?\.mjs)|(?:^|[\s"'])impeccable\s+hook\b)/.test(value);
  const oldHandler = value => value?.type === 'command'
    && [value.command, value.commandWindows].some(oldCommand);
  const originals = [...configs].map(file => ({ file, before: fs.readFileSync(file) }));
  const hasLegacy = legacy.size > 0 || originals.some(({ before }) =>
    /impeccable/.test(before.toString()) && (oldCommand(before.toString().replaceAll('\\"', '"')) || /"impeccable[^"\n]*"\s*:\s*true/.test(before.toString())));
  return {
    hasLegacy,
    apply(replace) {
      const updates = [];
      if (replace) {
        const clean = value => {
          if (Array.isArray(value)) return value.filter(item => !oldHandler(item))
            .flatMap(item => {
              const next = clean(item);
              return item?.hooks?.length > 0 && Array.isArray(next.hooks) && next.hooks.length === 0 ? [] : [next];
            });
          if (!value || typeof value !== 'object') return value;
          return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, clean(item)]));
        };
        for (const item of originals) {
          let data;
          try { data = JSON.parse(item.before.toString('utf8')); }
          catch { throw phoenixFailure(1, `无法移除旧 hooks,配置不是合法 JSON: ${item.file}`); }
          const next = { ...data };
          if (data.hooks) next.hooks = clean(data.hooks);
          if (data.enabledPlugins) next.enabledPlugins = Object.fromEntries(Object.entries(data.enabledPlugins)
            .map(([key, value]) => [key, key.startsWith('impeccable') ? false : value]));
          if (JSON.stringify(data) !== JSON.stringify(next)) updates.push({ ...item, after: JSON.stringify(next, null, 2) + '\n' });
        }
        for (const file of legacy) {
          fs.rmSync(file, { recursive: true, force: true });
          console.log(`  删除  ${file}`);
        }
      }
      for (const item of updates) {
        fs.writeFileSync(item.file, item.after);
        console.log(`  更新  ${item.file} (移除 impeccable hooks/插件启用项)`);
      }
      if (hasLegacy && !replace) console.log('  保留  impeccable,与 Phoenix UI 共存');
    },
  };
}

// 预检只写用户 cache 里的 engine;dry-run 不写项目。配置无效时整个安装在写项目前停下。
async function preparePhoenix(targetDir, options) {
  const manifest = bootstrap.manifestAt(path.join(PHOENIX_DIR, 'VERSION.json'));
  const target = bootstrap.platformTarget();
  let binary;
  try {
    binary = await bootstrap.installEngine(manifest, target, targetDir, options.releaseDir, process.env);
  } catch (error) {
    throw phoenixFailure(error.code || 4, `Phoenix UI engine 未就绪,未修改项目: ${error.message}\n`
      + '  可用 --release-dir=DIR 指向已核验的离线 release 目录,或加 --no-phoenix-ui 先安装其余部分。');
  }
  const args = ['install', '--project', targetDir, '--bundle-root', path.join(PHOENIX_DIR, 'bundle'), '--json'];
  if (options.providers) args.push('--providers', options.providers);
  const plan = runPhoenix(binary, [...args, '--dry-run']).report;
  return { manifest, binary, args, tools: preparePhoenixTools(plan.project, plan.providers) };
}

function runDiagram(binary, args) {
  const result = spawnSync(binary, args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, windowsHide: true });
  if (result.error) throw phoenixFailure(4, `无法运行 Phoenix Diagram engine: ${result.error.message}`);
  try {
    return { code: result.status, report: JSON.parse(result.stdout) };
  } catch {
    throw phoenixFailure([2, 3].includes(result.status) ? result.status : 4,
      `Phoenix Diagram 安装执行器失败(exit ${result.status}): ${(result.stderr || '').trim()}`);
  }
}

async function prepareDiagram(targetDir, options) {
  try {
    const ready = await diagramBootstrap.preflight({
      project: targetDir, manifestPath: path.join(DIAGRAM_DIR, 'VERSION.json'),
      bundleRoot: path.join(DIAGRAM_DIR, 'bundle'), releaseDir: options.diagramReleaseDir, env: process.env, allowInstall: true,
    });
    const args = ['install', '--project', targetDir, '--bundle-root', ready.bundleRoot, '--json'];
    if (options.providers) args.push('--providers', options.providers);
    const planned = runDiagram(ready.enginePath, [...args, '--dry-run']);
    if (planned.code !== 0 || !['planned', 'unchanged'].includes(planned.report.status)) {
      printDiagram(planned.report, ready.manifest);
      throw phoenixFailure(planned.code || 3, 'Phoenix Diagram 预检失败');
    }
    return { manifest: ready.manifest, binary: ready.enginePath, args };
  } catch (error) {
    throw phoenixFailure(Number.isInteger(error.code) ? error.code : 4, `Phoenix Diagram 未就绪,未修改项目: ${error.message}\n`
      + '  可用 --diagram-release-dir=DIR 指向匹配的完整离线目录,或加 --no-phoenix-diagram 跳过本工具。');
  }
}

function printDiagram(report, manifest) {
  const { toolVersion, npmPackageVersion } = manifest.data;
  console.log(`Phoenix Diagram ${toolVersion} (@xgent-ai/skills@${npmPackageVersion}): ${PHOENIX_STATUS[report.status] || report.status}`);
  for (const item of report.writes || []) console.log(`  写入  ${item.path}`);
  for (const item of report.removals || []) console.log(`  移除  ${item}`);
  for (const item of report.diagnostics || []) console.log(`  诊断  ${item.code}: ${item.message}`);
  if (report.status === 'installed') {
    for (const provider of (report.providers || []).filter(p => p !== '.codex')) console.log(`  重载  ${provider}: 新开项目会话加载 ${provider === '.agents' ? '$' : '/'}phoenix-diagram`);
  }
}

function groupedPaths(paths) {
  const groups = new Map();
  for (const file of paths) {
    const parts = file.split('/');
    const key = parts.length > 3 && ['skills', 'agents', 'commands'].includes(parts[1]) ? `${parts.slice(0, 3).join('/')}/` : file;
    groups.set(key, (groups.get(key) || 0) + 1);
  }
  return [...groups].map(([key, count]) => (count > 1 ? `${key} (${count} 个文件)` : key));
}

function reloadStep(provider) {
  if (provider === '.claude') return '.claude: 重启 Claude Code 会话,在 /hooks 核对 Phoenix 设计 hook';
  if (provider === '.agents') return '.agents/.codex: 在 Codex 中运行 /hooks,重新审阅并信任本项目 .codex/hooks.json 的 Phoenix handler';
  return `${provider}: 重新打开该工具的会话以加载新的 skill 与 hooks`;
}

function printPhoenix(report, manifest) {
  const { toolVersion, npmPackageVersion } = manifest.data;
  console.log(`Phoenix UI ${toolVersion} (@xgent-ai/skills@${npmPackageVersion}): ${PHOENIX_STATUS[report.status] || report.status}`);
  for (const [label, paths] of [['写入', report.writes], ['合并', report.sharedJson], ['移除', report.removals]]) {
    for (const entry of groupedPaths(paths)) console.log(`  ${label}  ${entry}`);
  }
  const skillProviders = report.providers.filter((provider) => provider !== '.codex');
  if (skillProviders.length) {
    console.log(`  命令  ${skillProviders.map((provider) => `${provider}: ${provider === '.agents' ? '$' : '/'}phoenix-ui`).join(';')}`);
  }
  if (report.status === 'installed') {
    for (const provider of skillProviders) console.log(`  重载  ${reloadStep(provider)}`);
  }
}

async function confirm(question, skipMessage, answers) {
  if (!process.stdin.isTTY) {
    console.log(skipMessage);
    return false;
  }
  process.stdout.write(question);
  const { value = '' } = await answers.next();
  return /^(y|yes)$/i.test(value.trim());
}

function usage() {
  console.log(`用法: npx @xgent-ai/skills <command>

命令:
  install [dir]   按选中的 provider 为目标项目(默认当前目录)安装:
                  .claude 写入全部 hook、agent 并在 settings.json 中启用
                  对应配置,.agents 写入全部 agent;并安装
                  Phoenix UI(设计 skill 与 hooks;engine 校验后
                  放入用户 cache;旧 impeccable 可选择替换或共存)${includesDiagram ? ',\n                  以及 Phoenix Diagram(十类图解 skill 与独立原生 engine)' : ''}
  help            显示本帮助

install 选项:
  --no-phoenix-ui       跳过 Phoenix UI,仍安装 XGENT 的 hooks、agents 与 settings(不删除已装版本)
  --no-impeccable       已弃用,等同 --no-phoenix-ui
  --replace-impeccable  删除项目及个人级旧 impeccable 工具和 hooks,不备份
  --keep-impeccable     保留 impeccable,与 Phoenix UI 共存(无交互时默认)
${includesDiagram ? `  --no-phoenix-diagram  跳过 Phoenix Diagram(不删除已装版本)
` : ''}  --release-dir=DIR     离线安装 Phoenix UI engine:使用已核验的 release 目录,不联网
${includesDiagram ? '  --diagram-release-dir=DIR 离线安装 Phoenix Diagram:使用匹配的完整目录,不联网\n' : ''}  --providers=a,b       指定本次安装的 harness 目录(默认按项目里已有的目录判断,
                        都没有时装 .claude;codex 同时选 .agents 与 .codex)
`);
}

// install 统一选一次 provider,再显式传给两个执行器:写入 XGENT 部分后才探测目录,
// 会把刚建的目录也算进去。Codex 的 skill 和 hook 分居 .agents 与 .codex,统一记为 .agents。
function selectProviders(requested, targetDir) {
  const known = Object.keys(readJson(path.join(PHOENIX_DIR, 'bundle', 'manifest.json')).providers);
  const dirs = (requested || '').split(',').map((v) => v.trim()).filter(Boolean)
    .map((v) => (v === 'codex' ? '.agents' : v.startsWith('.') ? v : `.${v}`));
  const unknown = dirs.filter((dir) => !known.includes(dir));
  if (unknown.length > 0) {
    console.error(`错误: 未知的 harness 目录: ${unknown.join(', ')}`);
    console.error(`可选: ${known.join(', ')}`);
    process.exit(1);
  }
  const existing = known.filter((dir) => fs.statSync(path.join(targetDir, dir), { throwIfNoEntry: false })?.isDirectory());
  const selected = dirs.length > 0 ? dirs : existing.length > 0 ? existing : ['.claude'];
  return [...new Set(selected.map((dir) => (dir === '.codex' ? '.agents' : dir)))];
}

async function install(args) {
  const flags = args.filter((a) => a.startsWith('--'));
  const dirArg = args.find((a) => !a.startsWith('--'));
  const unknown = flags.filter((f) => !['--no-phoenix-ui', '--no-impeccable', '--replace-impeccable', '--keep-impeccable',
    ...(includesDiagram ? ['--no-phoenix-diagram'] : [])].includes(f)
    && !['--providers=', '--release-dir=', ...(includesDiagram ? ['--diagram-release-dir='] : [])].some((p) => f.startsWith(p)));
  if (unknown.length > 0) {
    console.error(`错误: 未知选项: ${unknown.join(', ')}\n`);
    usage();
    process.exit(1);
  }
  if (flags.includes('--replace-impeccable') && flags.includes('--keep-impeccable')) {
    throw phoenixFailure(1, '--replace-impeccable 与 --keep-impeccable 不能同时使用');
  }
  const releaseDir = flags.find((f) => f.startsWith('--release-dir='))?.slice('--release-dir='.length);
  const diagramReleaseDir = flags.find((f) => f.startsWith('--diagram-release-dir='))?.slice('--diagram-release-dir='.length);
  const options = {
    phoenix: !flags.includes('--no-phoenix-ui') && !flags.includes('--no-impeccable'),
    diagram: includesDiagram && !flags.includes('--no-phoenix-diagram'),
    providers: flags.find((f) => f.startsWith('--providers='))?.slice('--providers='.length),
    releaseDir: releaseDir ? path.resolve(releaseDir) : undefined,
    diagramReleaseDir: diagramReleaseDir ? path.resolve(diagramReleaseDir) : undefined,
  };
  if (options.diagram && !diagramBootstrap.TARGETS.includes(`${process.platform}-${process.arch}`)) {
    if (diagramReleaseDir !== undefined) diagramBootstrap.platformTarget();
    options.diagram = false;
    console.log(`  跳过 Phoenix Diagram: ${process.platform}/${process.arch} 不在支持的四个 macOS/Linux 平台内。`);
  }

  const targetDir = path.resolve(dirArg || process.cwd());
  if (!fs.existsSync(targetDir) || !fs.statSync(targetDir).isDirectory()) {
    console.error(`错误: 目标目录不存在: ${targetDir}`);
    process.exit(1);
  }
  const providers = selectProviders(options.providers, targetDir);
  options.providers = providers.join(',');
  const claudeSelected = providers.includes('.claude');
  if (flags.includes('--no-impeccable')) console.log('  提示  --no-impeccable 已弃用,等同 --no-phoenix-ui');

  // 只读预检:全部通过后才写项目文件。
  const settingsPath = path.join(targetDir, '.claude', 'settings.json');
  let settings = {};
  if (claudeSelected && fs.existsSync(settingsPath)) {
    try {
      settings = JSON.parse(fs.readFileSync(settingsPath, 'utf8'));
    } catch {
      console.error(`错误: ${settingsPath} 不是合法 JSON,未修改该文件,请先手动修复。`);
      process.exit(1);
    }
  }
  const phoenix = options.phoenix ? await preparePhoenix(targetDir, options) : null;
  const diagram = options.diagram ? await prepareDiagram(targetDir, options) : null;
  const askReplace = phoenix?.tools.hasLegacy && !flags.includes('--replace-impeccable') && !flags.includes('--keep-impeccable');
  const rl = process.stdin.isTTY && askReplace
    ? readline.createInterface({ input: process.stdin, output: process.stdout }) : null;
  const answers = rl?.[Symbol.asyncIterator]();
  rl?.once('SIGINT', () => { rl.close(); process.exit(130); });
  let replace = flags.includes('--replace-impeccable');
  try {
    if (askReplace) replace = await confirm(
      '发现旧 impeccable,是否删除并替换为 Phoenix UI(含个人级入口,不备份)? [y/N] ',
      '  保留  impeccable:无交互环境默认共存;可用 --replace-impeccable 替换', answers);
  } finally {
    rl?.close();
  }
  if (phoenix) {
    phoenix.tools.apply(replace);
    // Cleanup may change shared settings.
    if (claudeSelected && fs.existsSync(settingsPath)) settings = readJson(settingsPath);
  }

  for (const [srcDir, destRelative, mode] of [
    ...(claudeSelected ? [[HOOKS_SRC_DIR, '.claude/hooks', 0o755], [AGENTS_SRC_DIR, '.claude/agents', 0o644]] : []),
    ...(providers.includes('.agents') ? [[AGENTS_SRC_DIR, '.agents/agents', 0o644]] : []),
  ]) {
    const destDir = path.join(targetDir, destRelative);
    fs.mkdirSync(destDir, { recursive: true });
    for (const name of fs.readdirSync(srcDir)) {
      const content = fs.readFileSync(path.join(srcDir, name));
      const dest = path.join(destDir, name);
      let status = '安装';
      if (fs.existsSync(dest)) {
        status = content.equals(fs.readFileSync(dest)) ? '未变' : '更新';
      }
      if (status !== '未变') {
        fs.writeFileSync(dest, content, { mode });
      }
      console.log(`  ${status}  ${destRelative}/${name}`);
    }
  }

  if (claudeSelected) {
    const before = JSON.stringify(settings);
    Object.assign(settings, MANAGED_SETTINGS);
    if (JSON.stringify(settings) === before) {
      console.log('  未变  .claude/settings.json');
    } else {
      fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2) + '\n');
      console.log('  写入  .claude/settings.json (statusLine)');
    }
  }

  // XGENT 部分已完成;Phoenix 直接覆盖工具文件,中断后重跑。
  if (phoenix) {
    let applied;
    try {
      applied = runPhoenix(phoenix.binary, phoenix.args);
    } catch (error) {
      console.log(`未完成: XGENT 部分已安装;Phoenix UI 安装未完成,处理下方错误后重跑本命令覆盖安装。${diagram ? 'Phoenix Diagram 未执行。' : ''}`);
      throw error;
    }
    printPhoenix(applied.report, phoenix.manifest);
    if (applied.code !== 0) {
      process.exitCode = applied.code;
      console.log(`未完成: XGENT 部分已安装;Phoenix UI ${PHOENIX_STATUS[applied.report.status] || applied.report.status},处理后重跑。`);
      if (diagram) console.log('  待执行  Phoenix Diagram');
      return;
    }
    console.log('  提示  在 agent 对话里(不是终端)输入 /phoenix-ui init 完成设计上下文;Codex 用 $phoenix-ui');
  }
  if (diagram) {
    const completed = `XGENT 已安装${phoenix ? ',Phoenix UI 已完成' : ''}`;
    let applied;
    try {
      applied = runDiagram(diagram.binary, diagram.args);
    } catch (error) {
      console.log(`未完成: ${completed};Phoenix Diagram 安装未完成,处理错误后重跑本命令覆盖安装。`);
      throw error;
    }
    printDiagram(applied.report, diagram.manifest);
    if (applied.code !== 0) {
      process.exitCode = applied.code;
      console.log(`未完成: ${completed};Phoenix Diagram ${applied.report.status},保留前段结果,按上述提示重跑。`);
      return;
    }
  }
  console.log(`完成: ${targetDir}`);
  console.log('  可选  在 agent 对话中用 /optimize-agents-md(Codex 用 $optimize-agents-md)以 opinionated DSL 改良已有 AGENTS.md;不喜欢这套风格可忽略。');
}

const [cmd, ...args] = process.argv.slice(2);
switch (cmd) {
  case 'install':
    install(args).catch((error) => {
      console.error(`错误: ${error.message}`);
      process.exitCode = Number.isInteger(error.code) ? error.code : 1;
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
