#!/usr/bin/env node
// xgent-skills — 为目标项目安装 XGENT 的 Claude Code hooks、启用相关 settings,
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
const AGENTS_TEMPLATE = path.join(PKG_ROOT, 'skills', 'xgent-init', 'references', 'external-app-AGENTS.template.md');

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
// Phoenix 的受管安装、hooks 与回执由 Rust 执行器事务化完成。
// 旧工具是否删除由 npm 安装入口询问,工具清理不备份、不迁移旧数据。

const bootstrap = require('../skills/phoenix-ui/scripts/phoenix-bootstrap.cjs');
const PHOENIX_DIR = path.join(PKG_ROOT, 'dist', 'phoenix-ui');
const diagramBootstrap = require('../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs');
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

// 退出码沿用执行器:2 参数/路径,3 冲突/待恢复/融合未就绪,4 文件或运行环境。
function runPhoenix(binary, args) {
  const result = spawnSync(binary, args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, windowsHide: true });
  if (result.error) throw phoenixFailure(4, `无法运行 Phoenix UI engine: ${result.error.message}`);
  try {
    return { code: result.status, report: JSON.parse(result.stdout) };
  } catch {
    throw phoenixFailure([2, 3].includes(result.status) ? result.status : 4,
      `Phoenix UI 安装执行器失败(exit ${result.status}): ${(result.stderr || '').trim()}`);
  }
}

// Tool directories are reinstallable. Shared settings retain unrelated entries.
function preparePhoenixTools(project, providers) {
  const refresh = [];
  const legacy = new Set();
  const configs = new Set();
  const parents = new Map();
  let managed = [];
  try {
    managed = readJson(path.join(project, '.phoenix-ui/install/active.json')).ownership
      .filter(item => item.sha256).map(item => item.path);
  } catch { /* The engine reports an absent or invalid receipt during preflight. */ }
  const present = (file) => !!fs.lstatSync(file, { throwIfNoEntry: false });
  const checked = (root, relative) => {
    const file = path.join(root, relative);
    parents.set(file, root);
    return file;
  };
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
          // An earlier explicit migration may own a Phoenix compatibility entry.
          if (root === project && managed.some(file => file === relative || file.startsWith(`${relative}/`))) continue;
          const file = checked(root, relative);
          if (present(file)) legacy.add(file);
        }
        const shared = dir === '.claude' ? ['settings.json', 'settings.local.json']
          : dir === '.gemini' ? ['settings.json']
            : ['.github', '.grok'].includes(dir) ? ['hooks/impeccable.json', 'hooks/phoenix-ui.json'] : ['hooks.json'];
        for (const relative of shared) {
          const file = checked(root, `${dir}/${relative}`);
          if (present(file)) configs.add(file);
        }
      }
    }
    if (!fs.existsSync(path.join(project, '.phoenix-ui/install'))) {
      const file = checked(project, `${provider}/skills/phoenix-ui`);
      if (present(file)) refresh.push(file);
    }
  }
  const oldCommand = value => typeof value === 'string'
    && /(?:skills[\\/]impeccable[\\/]scripts[\\/](?:impeccable(?:\.cmd)?|hook(?:-(?:probe|before-edit|after-edit|stop))?\.mjs)|(?:^|[\s"'])impeccable\s+hook\b)/.test(value);
  const oldHandler = value => value?.type === 'command'
    && [value.command, value.commandWindows].some(oldCommand);
  const originals = [...configs].map(file => ({ file, before: fs.readFileSync(file) }));
  const hasLegacy = legacy.size > 0 || originals.some(({ before }) =>
    /impeccable/.test(before.toString()) && (oldCommand(before.toString().replaceAll('\\"', '"')) || /"impeccable[^"\n]*"\s*:\s*true/.test(before.toString())));
  const covers = relative => refresh.some(file => {
    const target = path.relative(project, file).split(path.sep).join('/');
    return relative === target || relative.startsWith(`${target}/`);
  });
  return {
    hasLegacy, covers,
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
        for (const item of updates) {
          if (!fs.readFileSync(item.file).equals(item.before)) throw phoenixFailure(3, `配置在预检后发生变化,请重跑: ${item.file}`);
        }
      }
      const removals = [...refresh, ...(replace ? legacy : [])];
      for (const file of [...removals, ...updates.map(item => item.file)]) {
        for (let dir = path.dirname(file); dir !== parents.get(file); dir = path.dirname(dir)) {
          if (fs.lstatSync(dir, { throwIfNoEntry: false })?.isSymbolicLink()) throw phoenixFailure(3, `工具目录的父路径是符号链接,未修改: ${dir}`);
        }
        if (updates.some(item => item.file === file) && fs.lstatSync(file).isSymbolicLink()) throw phoenixFailure(3, `共享配置是符号链接,未修改: ${file}`);
      }
      for (const file of removals) {
        fs.rmSync(file, { recursive: true, force: true });
        console.log(`  ${refresh.includes(file) ? '重新安装' : '删除'}  ${file}`);
      }
      for (const item of updates) {
        fs.writeFileSync(item.file, item.after);
        console.log(`  更新  ${item.file} (移除 impeccable hooks/插件启用项)`);
      }
      if (hasLegacy && !replace) console.log('  保留  impeccable,与 Phoenix UI 共存');
    },
  };
}

// 预检只写用户 cache 里的 engine;dry-run 不写项目。冲突时整个安装在写项目前停下。
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
  const args = ['install', '--project', targetDir, '--bundle-root', path.join(PHOENIX_DIR, 'bundle'),
    '--auxiliary-root', path.join(PKG_ROOT, 'skills'), '--keep-impeccable', '--json'];
  if (options.providers) args.push('--providers', options.providers);
  const plan = runPhoenix(binary, [...args, '--dry-run']).report;
  const tools = preparePhoenixTools(plan.project, plan.providers);
  if (plan.status === 'conflict' && plan.conflicts.some(conflict => !tools.covers(conflict.path))) {
    printPhoenix(plan, manifest);
    throw phoenixFailure(3, 'Phoenix UI 预检发现冲突,未修改项目。解决上列冲突后重跑;或加 --no-phoenix-ui 先安装其余部分。');
  }
  return { manifest, binary, args, tools };
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
      bundleRoot: path.join(DIAGRAM_DIR, 'bundle'), packageJsonPath: path.join(PKG_ROOT, 'package.json'),
      releaseDir: options.diagramReleaseDir, env: process.env, allowInstall: true, checkReceipt: false,
    });
    const args = ['install', '--project', targetDir, '--bundle-root', ready.bundleRoot, '--json'];
    if (options.providers) args.push('--providers', options.providers);
    const planned = runDiagram(ready.enginePath, [...args, '--dry-run']);
    // A known pending transaction is recovered only by the later explicit apply.
    const recoverable = planned.code === 3 && planned.report.status === 'recovery-required'
      && planned.report.conflicts?.length === 0;
    if ((planned.code !== 0 && !recoverable) || !['planned', 'unchanged', 'recovery-required'].includes(planned.report.status)) {
      printDiagram(planned.report, ready.manifest);
      throw phoenixFailure(planned.code || 3, 'Phoenix Diagram 预检失败');
    }
    return { manifest: ready.manifest, binary: ready.enginePath, args };
  } catch (error) {
    throw phoenixFailure(Number.isInteger(error.code) ? error.code : 4, `Phoenix Diagram 未就绪,未修改项目: ${error.message}\n`
      + '  可用 --diagram-release-dir=DIR 指向匹配的完整离线目录,或加 --no-phoenix-diagram 跳过本工具。');
  }
}

async function migrateDiagram(project, options, confirmed) {
  const ready = await diagramBootstrap.preflight({
    project, exactProject: true, manifestPath: path.join(DIAGRAM_DIR, 'VERSION.json'),
    bundleRoot: path.join(DIAGRAM_DIR, 'bundle'), packageJsonPath: path.join(PKG_ROOT, 'package.json'),
    releaseDir: options.diagramReleaseDir, env: process.env, allowInstall: false, checkReceipt: false,
  });
  const args = ['migrate', '--from-archify', '--project', ready.project, '--bundle-root', ready.bundleRoot, '--json'];
  if (options.providers) args.push('--providers', options.providers);
  const planned = runDiagram(ready.enginePath, [...args, '--dry-run']);
  printDiagram(planned.report, ready.manifest);
  if (planned.code !== 0) { process.exitCode = planned.code || 3; return; }
  console.log('  Archify 仅移除上述所选项目/provider 的完整已知来源;不备份,回滚不重建旧入口,需要时按原来源重装。');
  let answer = confirmed;
  if (!answer && !process.stdin.isTTY) throw phoenixFailure(3, '非交互迁移需要显式 --confirm-remove-archify;未开始删除或新入口写入。');
  if (!answer) {
    const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
    try { answer = await confirm('确认直接移除上述 Archify 并安装 Phoenix Diagram? [y/N] ', '', rl[Symbol.asyncIterator]()); }
    finally { rl.close(); }
  }
  if (!answer) { console.log('  取消迁移;未开始删除或新入口写入。'); return; }
  const applied = runDiagram(ready.enginePath, [...args, '--confirm-remove-archify']);
  printDiagram(applied.report, ready.manifest);
  process.exitCode = applied.code || 0;
}

function printDiagram(report, manifest) {
  const { toolVersion, npmPackageVersion } = manifest.data;
  console.log(`Phoenix Diagram ${toolVersion} (@xgent-ai/skills@${npmPackageVersion}): ${PHOENIX_STATUS[report.status] || report.status}`);
  for (const item of report.removals || []) console.log(`  ${item.status}  ${item.path} (${item.source})`);
  for (const item of report.preserved || []) console.log(`  保留  ${item}`);
  if (report.rollbackCommand) console.log(`  新入口回滚  ${report.rollbackCommand}`);
  for (const item of report.reinstallInstructions || []) console.log(`  旧入口  ${item}`);
  for (const item of report.writes || []) console.log(`  写入  ${item.path}`);
  for (const item of report.conflicts || []) console.log(`  冲突  ${item.path}: ${item.message}`);
  for (const item of report.diagnostics || []) console.log(`  诊断  ${item.code}: ${item.message}`);
  if (report.recovered?.length) console.log(`  恢复  ${report.recovered.join(', ')}`);
  if (report.transactionId) console.log(`  事务  ${report.transactionId};日志 ${report.journal};备份 ${report.backup}`);
  if (report.status === 'installed') {
    for (const provider of (report.providers || []).filter(p => p !== '.codex')) console.log(`  重载  ${provider}: 新开项目会话加载 ${provider === '.agents' ? '$' : '/'}phoenix-diagram`);
  }
  if (report.recoveryCommand && report.status === 'recovery-required') console.log(`  恢复命令  ${report.recoveryCommand}`);
  for (const item of report.localIgnore || []) console.log(`  本地说明  ${item}`);
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
  if (report.recovered) console.log(`  恢复  上次中断的事务 ${report.recovered}`);
  if (report.migrationSources.length) {
    console.log(`  迁移  认领 impeccable 旧版安装(来源 @xgent-ai/skills ${report.migrationSources.join(', ')}),旧入口移入备份`);
  }
  for (const [label, paths] of [['写入', report.writes], ['合并', report.sharedJson], ['移除', report.removals]]) {
    for (const entry of groupedPaths(paths)) console.log(`  ${label}  ${entry}`);
  }
  const skillProviders = report.providers.filter((provider) => provider !== '.codex');
  const migrated = report.migrationSources.length > 0 || report.transitionEntries.length > 0;
  if (skillProviders.length) {
    console.log(`  命令  ${skillProviders.map((provider) => {
      const prefix = provider === '.agents' ? '$' : '/';
      return `${provider}: ${migrated ? `${prefix}impeccable → ` : ''}${prefix}phoenix-ui`;
    }).join(';')}`);
  }
  for (const entry of report.transitionEntries) console.log(`  提示  ${entry} 保留一个迁移主版本,只指向新命令`);
  for (const conflict of report.conflicts) console.log(`  冲突  ${conflict.path}: ${conflict.message}`);
  for (const finding of report.findings) console.log(`  ${finding.blockingIntegration ? '待处理' : '说明'}  ${finding.path}: ${finding.message}`);
  for (const item of report.unconsumed) console.log(`  未迁移  ${item.path} ${item.field}: ${item.reason}`);
  if (report.transactionId && ['installed', 'integration-pending'].includes(report.status)) {
    console.log(`  事务  ${report.transactionId};备份 ${report.backups}`);
    const project = bootstrap.shellQuote(report.project);
    console.log(`  回滚  npm exec --package=@xgent-ai/skills@${npmPackageVersion} -- phoenix-ui migrate --rollback ${report.transactionId} --project ${project}`);
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
                  hook,在 .claude/settings.json 中启用对应配置,并安装
                  Phoenix UI(设计 skill、hooks 与五个辅助 skill;engine 校验后
                  放入用户 cache;旧 impeccable 可选择替换或共存),
                  以及 Phoenix Diagram(十类图解 skill 与独立原生 engine),
                  并询问是否创建通用的 AGENTS.md
  help            显示本帮助

install 选项:
  --no-phoenix-ui       跳过 Phoenix UI,仍安装 XGENT 的 hooks 与 settings(不删除已装版本)
  --no-impeccable       已弃用,等同 --no-phoenix-ui
  --replace-impeccable  删除项目及个人级旧 impeccable 工具和 hooks,不备份
  --keep-impeccable     保留 impeccable,与 Phoenix UI 共存(无交互时默认)
  --no-phoenix-diagram  跳过 Phoenix Diagram(不删除已装版本)
  --migrate-archify     仅执行 Diagram 预检/确认/迁移,跳过 hooks/settings/AGENTS/UI;
                        使用已存在且匹配包内固定版本的引擎,不自动获取
  --confirm-remove-archify  仅配 --migrate-archify,确认移除所选项目/provider 旧入口;
                        不备份 Archify,回滚不重建旧入口,需要时按原来源重装
  --release-dir=DIR     离线安装 Phoenix UI engine:使用已核验的 release 目录,不联网
  --diagram-release-dir=DIR 离线安装 Phoenix Diagram:使用匹配的完整目录,不联网
  --xgent-init          创建 AGENTS.md (已有则跳过),不再询问
  --no-xgent-init       跳过 AGENTS.md,不再询问
  --providers=a,b       指定两个 Phoenix 工具装进哪些 harness 目录(默认按项目里已有的
                        目录判断,都没有时装 .claude;codex 同时选 .agents 与 .codex)
`);
}

async function install(args) {
  const flags = args.filter((a) => a.startsWith('--'));
  const dirArg = args.find((a) => !a.startsWith('--'));
  const unknown = flags.filter((f) => !['--no-phoenix-ui', '--no-phoenix-diagram', '--no-impeccable', '--replace-impeccable', '--keep-impeccable', '--xgent-init', '--no-xgent-init', '--migrate-archify', '--confirm-remove-archify'].includes(f)
    && !['--providers=', '--release-dir=', '--diagram-release-dir='].some((p) => f.startsWith(p)));
  if (unknown.length > 0) {
    console.error(`错误: 未知选项: ${unknown.join(', ')}\n`);
    usage();
    process.exit(1);
  }
  if (flags.includes('--confirm-remove-archify') && !flags.includes('--migrate-archify')) throw phoenixFailure(2, '--confirm-remove-archify 需要 --migrate-archify');
  if (flags.includes('--xgent-init') && flags.includes('--no-xgent-init')) {
    console.error('错误: --xgent-init 与 --no-xgent-init 不能同时使用');
    process.exit(1);
  }
  if (flags.includes('--replace-impeccable') && flags.includes('--keep-impeccable')) {
    throw phoenixFailure(1, '--replace-impeccable 与 --keep-impeccable 不能同时使用');
  }
  const releaseDir = flags.find((f) => f.startsWith('--release-dir='))?.slice('--release-dir='.length);
  const diagramReleaseDir = flags.find((f) => f.startsWith('--diagram-release-dir='))?.slice('--diagram-release-dir='.length);
  const options = {
    phoenix: !flags.includes('--no-phoenix-ui') && !flags.includes('--no-impeccable'),
    diagram: !flags.includes('--no-phoenix-diagram'),
    providers: flags.find((f) => f.startsWith('--providers='))?.slice('--providers='.length),
    releaseDir: releaseDir ? path.resolve(releaseDir) : undefined,
    diagramReleaseDir: diagramReleaseDir ? path.resolve(diagramReleaseDir) : undefined,
  };
  if (options.diagram && !diagramBootstrap.TARGETS.includes(`${process.platform}-${process.arch}`)) {
    if (diagramReleaseDir !== undefined) diagramBootstrap.platformTarget();
    options.diagram = false;
    console.log(`  跳过 Phoenix Diagram: ${process.platform}/${process.arch} 不在支持的四个 macOS/Linux 平台内。`);
  }

  if (flags.includes('--migrate-archify')) {
    const target = path.resolve(dirArg || process.cwd());
    if (!options.diagram) { console.log('  跳过 Phoenix Diagram 迁移;保留 Archify。'); return; }
    await migrateDiagram(target, options, flags.includes('--confirm-remove-archify'));
    return;
  }

  // 跳过 Phoenix 时仍按包内 bundle 校验 --providers,保持旧参数语义;启用时由执行器校验。
  if (!options.phoenix && options.providers) {
    const available = Object.keys(readJson(path.join(PHOENIX_DIR, 'bundle', 'manifest.json')).providers)
      .filter((name) => name !== '.codex');
    const unknown = options.providers.split(',').map((v) => v.trim()).filter(Boolean)
      .map((v) => (v === 'codex' ? '.agents' : v.startsWith('.') ? v : `.${v}`))
      .filter((v) => !available.includes(v));
    if (unknown.length > 0) {
      console.error(`错误: 未知的 harness 目录: ${unknown.join(', ')}`);
      console.error(`可选: ${available.join(', ')}`);
      process.exit(1);
    }
  }

  const targetDir = path.resolve(dirArg || process.cwd());
  if (!fs.existsSync(targetDir) || !fs.statSync(targetDir).isDirectory()) {
    console.error(`错误: 目标目录不存在: ${targetDir}`);
    process.exit(1);
  }
  if (flags.includes('--no-impeccable')) console.log('  提示  --no-impeccable 已弃用,等同 --no-phoenix-ui');

  // 只读预检:全部通过后才写项目文件。
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
  const agentsExists = fs.existsSync(path.join(targetDir, 'AGENTS.md'));
  const askAgents = !flags.includes('--xgent-init') && !flags.includes('--no-xgent-init') && !agentsExists;
  const phoenix = options.phoenix ? await preparePhoenix(targetDir, options) : null;
  const diagram = options.diagram ? await prepareDiagram(targetDir, options) : null;
  const askReplace = phoenix?.tools.hasLegacy && !flags.includes('--replace-impeccable') && !flags.includes('--keep-impeccable');
  const rl = process.stdin.isTTY && (askAgents || askReplace)
    ? readline.createInterface({ input: process.stdin, output: process.stdout }) : null;
  const answers = rl?.[Symbol.asyncIterator]();
  rl?.once('SIGINT', () => { rl.close(); process.exit(130); });
  let agents;
  let replace = flags.includes('--replace-impeccable');
  try {
    if (askReplace) replace = await confirm(
      '发现旧 impeccable,是否删除并替换为 Phoenix UI(含个人级入口,不备份)? [y/N] ',
      '  保留  impeccable:无交互环境默认共存;可用 --replace-impeccable 替换', answers);
    agents = flags.includes('--xgent-init') || (askAgents && await confirm(
      '是否同时创建通用的 AGENTS.md (含 Portal 联调与前端验收约定)? [y/N] ',
      '  跳过  AGENTS.md:非交互环境,可加 --xgent-init 创建', answers));
  } finally {
    rl?.close();
  }
  if (phoenix) {
    phoenix.tools.apply(replace);
    // Cleanup may change shared settings.
    if (fs.existsSync(settingsPath)) settings = readJson(settingsPath);
  }

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

  const before = JSON.stringify(settings);
  Object.assign(settings, MANAGED_SETTINGS);
  if (JSON.stringify(settings) === before) {
    console.log('  未变  .claude/settings.json');
  } else {
    fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2) + '\n');
    console.log('  写入  .claude/settings.json (statusLine)');
  }

  if (agents) createAgents(targetDir);
  else if (agentsExists && !flags.includes('--no-xgent-init')) console.log('  跳过  AGENTS.md (已存在,保留原文)');

  // XGENT 部分已完成;Phoenix 在锁内重读这些文件后再应用,中断时可重试。
  if (phoenix) {
    let applied;
    try {
      applied = runPhoenix(phoenix.binary, phoenix.args);
    } catch (error) {
      console.log(`未完成: XGENT 部分已安装;Phoenix UI 待恢复(安装被中断,可能已部分写入),按下方错误处理后重跑本命令,会先按日志恢复(可重复执行)。${diagram ? 'Phoenix Diagram 未执行。' : ''}`);
      throw error;
    }
    printPhoenix(applied.report, phoenix.manifest);
    if (applied.code !== 0) {
      process.exitCode = applied.code;
      console.log(`未完成: XGENT 部分已安装;Phoenix UI ${PHOENIX_STATUS[applied.report.status] || applied.report.status},按上方提示处理后重跑。`);
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
      console.log(`未完成: ${completed};Phoenix Diagram 可能已部分写入,按错误和项目日志重跑恢复。`);
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
