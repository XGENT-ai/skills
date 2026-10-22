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

function writeFileIfChanged(dest, content) {
  if (fs.existsSync(dest) && content.equals(fs.readFileSync(dest))) return false;
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, content, { mode: 0o644 });
  return true;
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

function printDiagram(report, manifest) {
  const { toolVersion, npmPackageVersion } = manifest.data;
  console.log(`Phoenix Diagram ${toolVersion} (@xgent-ai/skills@${npmPackageVersion}): ${PHOENIX_STATUS[report.status] || report.status}`);
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

// 重装没给阈值 flag 时沿用旧 handler 里的值。
function guardThreshold(command) {
  return Number(/ --threshold ([1-9][0-9]?)$/.exec(command)?.[1]) || undefined;
}

// 先校验并算出内容,返回写入动作;任何项目写入都在全部预检之后。
function prepareContextGoalGuard(targetDir, force, threshold) {
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
  // 运行时从 Git 根目录定位,避免配置绑定安装机器或会话子目录。
  const command = `python3 "$(git rev-parse --show-toplevel)/.codex/hooks/context-goal-guard.py" --threshold ${limit}`;
  const handler = { type: 'command', command, timeout: 3 };
  // 已有且仅有同一 handler 时保留原字节和位置:Phoenix 安装器随后会在同一文件追加自己的分组。
  const installed = !backup && ['PreToolUse', 'Stop'].every((event) => {
    const guards = (existing.hooks?.[event] || []).flatMap((entry) => (Array.isArray(entry?.hooks) ? entry.hooks : []))
      .filter((item) => /\.codex\/hooks\/context-goal-guard\.py(?:['"\s]|$)/.test(item?.command ?? ''));
    return guards.length === 1 && JSON.stringify(guards[0]) === JSON.stringify(handler);
  });
  for (const event of ['PreToolUse', 'Stop']) hooks[event].push({ hooks: [handler] });
  const script = fs.readFileSync(CONTEXT_GOAL_GUARD_SRC);
  const manifest = installed ? fs.readFileSync(manifestPath) : Buffer.from(JSON.stringify({ ...existing, hooks }, null, 2) + '\n');
  return {
    limit,
    write() {
      if (backup) fs.copyFileSync(manifestPath, `${manifestPath}.bak`);
      console.log(`  ${writeFileIfChanged(scriptPath, script) ? '写入' : '未变'}  .codex/hooks/context-goal-guard.py`);
      console.log(`  ${writeFileIfChanged(manifestPath, manifest) ? '写入' : '未变'}  .codex/hooks.json (context goal guard)`);
    },
  };
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
  提示: 请安装到 Git 仓库根目录;Codex 运行环境必须能找到 git 和 Python 3.9+。
        hook 运行时从 Git 根目录定位脚本;没有 active goal 时不会触发。
`);
}

// 与 prepareContextGoalGuard 形状相似,但文件、事件和匹配式都不同,暂不抽公共函数。
function prepareClaudeContextGoalGuard(targetDir, force, threshold) {
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
  return {
    limit,
    write() {
      if (backup) fs.copyFileSync(settingsPath, `${settingsPath}.bak`);
      console.log(`  ${writeFileIfChanged(path.join(targetDir, '.claude', 'hooks', 'context-goal-guard.js'), script) ? '写入' : '未变'}  .claude/hooks/context-goal-guard.js`);
      console.log(`  ${writeFileIfChanged(settingsPath, settings) ? '写入' : '未变'}  .claude/settings.local.json (context goal guard)`);
    },
  };
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
                  hook,在 .claude/settings.json 中启用对应配置,并安装
                  Phoenix UI(设计 skill、hooks 与五个辅助 skill;engine 校验后
                  放入用户 cache;旧 impeccable 可选择替换或共存),
                  以及 Phoenix Diagram(五类图解 skill 与独立原生 engine),
                  并询问是否创建通用的 AGENTS.md;
                  Codex / Claude Code goal 上下文收尾提醒默认不安装
  help            显示本帮助

install 选项:
  --no-phoenix-ui       跳过 Phoenix UI,仍安装 XGENT 的 hooks 与 settings(不删除已装版本)
  --no-impeccable       已弃用,等同 --no-phoenix-ui
  --replace-impeccable  删除项目及个人级旧 impeccable 工具和 hooks,不备份
  --keep-impeccable     保留 impeccable,与 Phoenix UI 共存(无交互时默认)
  --no-phoenix-diagram  跳过 Phoenix Diagram(不删除已装版本)
  --release-dir=DIR     离线安装 Phoenix UI engine:使用已核验的 release 目录,不联网
  --diagram-release-dir=DIR 离线安装 Phoenix Diagram:使用匹配的完整目录,不联网
  --xgent-init          创建 AGENTS.md (已有则跳过),不再询问
  --no-xgent-init       跳过 AGENTS.md,不再询问
  --context-goal-guard  显式安装 Codex goal guard(默认不安装,需要 Python 3.9+)
  --no-context-goal-guard 跳过 Codex goal guard(默认行为,保留已有 guard)
  --context-goal-guard-threshold=N  Codex guard 的阈值百分比(1–99 整数,默认 ${CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD};
                        重装时不给则沿用已装的值),同时表示安装
  --claude-context-goal-guard 显式安装 Claude Code goal guard(默认不安装)
  --no-claude-context-goal-guard 跳过 Claude Code goal guard(默认行为,保留已有 guard)
  --claude-context-goal-guard-threshold=N  Claude Code guard 的阈值百分比(1–99 整数,
                        默认 ${CLAUDE_CONTEXT_GOAL_GUARD_DEFAULT_THRESHOLD};重装时不给则沿用已装的值),同时表示安装
  --providers=a,b       指定两个 Phoenix 工具装进哪些 harness 目录(默认按项目里已有的
                        目录判断,都没有时装 .claude;codex 同时选 .agents 与 .codex)
  --force               允许覆盖非法 JSON 的 guard 配置(先存 .bak),不覆盖已有 AGENTS.md;
                        不绕过 Phoenix 受管安装及共享配置的冲突保护
`);
}

async function install(args) {
  const flags = args.filter((a) => a.startsWith('--'));
  const dirArg = args.find((a) => !a.startsWith('--'));
  const unknown = flags.filter((f) => !['--no-phoenix-ui', '--no-phoenix-diagram', '--no-impeccable', '--replace-impeccable', '--keep-impeccable', '--xgent-init', '--no-xgent-init', '--context-goal-guard', '--no-context-goal-guard', '--claude-context-goal-guard', '--no-claude-context-goal-guard', '--force'].includes(f)
    && !['--providers=', '--release-dir=', '--diagram-release-dir=', '--context-goal-guard-threshold=', '--claude-context-goal-guard-threshold='].some((p) => f.startsWith(p)));
  if (unknown.length > 0) {
    console.error(`错误: 未知选项: ${unknown.join(', ')}\n`);
    usage();
    process.exit(1);
  }
  if (flags.includes('--xgent-init') && flags.includes('--no-xgent-init')) {
    console.error('错误: --xgent-init 与 --no-xgent-init 不能同时使用');
    process.exit(1);
  }
  if (flags.includes('--replace-impeccable') && flags.includes('--keep-impeccable')) {
    throw phoenixFailure(1, '--replace-impeccable 与 --keep-impeccable 不能同时使用');
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
  const releaseDir = flags.find((f) => f.startsWith('--release-dir='))?.slice('--release-dir='.length);
  const diagramReleaseDir = flags.find((f) => f.startsWith('--diagram-release-dir='))?.slice('--diagram-release-dir='.length);
  const options = {
    phoenix: !flags.includes('--no-phoenix-ui') && !flags.includes('--no-impeccable'),
    diagram: !flags.includes('--no-phoenix-diagram'),
    force: flags.includes('--force'),
    providers: flags.find((f) => f.startsWith('--providers='))?.slice('--providers='.length),
    releaseDir: releaseDir ? path.resolve(releaseDir) : undefined,
    diagramReleaseDir: diagramReleaseDir ? path.resolve(diagramReleaseDir) : undefined,
  };
  if (options.diagram && !diagramBootstrap.TARGETS.includes(`${process.platform}-${process.arch}`)) {
    if (diagramReleaseDir !== undefined) diagramBootstrap.platformTarget();
    options.diagram = false;
    console.log(`  跳过 Phoenix Diagram: ${process.platform}/${process.arch} 不在支持的四个 macOS/Linux 平台内。`);
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
  const guard = flags.includes('--context-goal-guard') || codexThreshold !== undefined;
  const claudeGuard = flags.includes('--claude-context-goal-guard') || claudeThreshold !== undefined;
  let codexGuardPlan = guard ? prepareContextGoalGuard(targetDir, options.force, codexThreshold) : null;
  let claudeGuardPlan = claudeGuard ? prepareClaudeContextGoalGuard(targetDir, options.force, claudeThreshold) : null;
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
    // Cleanup may change shared settings that the optional guards also merge.
    if (fs.existsSync(settingsPath)) settings = readJson(settingsPath);
    codexGuardPlan = guard ? prepareContextGoalGuard(targetDir, options.force, codexThreshold) : null;
    claudeGuardPlan = claudeGuard ? prepareClaudeContextGoalGuard(targetDir, options.force, claudeThreshold) : null;
  }
  codexGuardPlan?.write();
  claudeGuardPlan?.write();

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
  if (codexGuardPlan) printContextGoalGuardSteps(codexGuardPlan.limit);
  if (claudeGuardPlan) printClaudeContextGoalGuardSteps(claudeGuardPlan.limit);

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
