'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const cli = path.join(root, 'bin', 'xgent-skills.js');
const template = path.join(root, 'skills', 'xgent-init', 'references', 'external-app-AGENTS.template.md');

function project(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'xgent-install-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  return dir;
}

function run(dir, flags = [], input, env = process.env) {
  // 仅模拟终端检测,输入仍经过 CLI 的 readline 询问和回答处理。
  const prefix = input === undefined ? [] : ['-e', 'process.stdin.isTTY = true; require(process.argv[1]);'];
  const guardFlags = flags.some((flag) => /^--(no-)?context-goal-guard(?:$|-threshold=)/.test(flag)) ? [] : ['--no-context-goal-guard'];
  const claudeGuardFlags = flags.some((flag) => flag.includes('claude-context-goal-guard')) ? [] : ['--no-claude-context-goal-guard'];
  return spawnSync(process.execPath, [...prefix, cli, 'install', dir, '--no-impeccable', ...guardFlags, ...claudeGuardFlags, ...flags], {
    input: input ?? '', encoding: 'utf8', timeout: 10000, env,
  });
}

function runGuard(dir, flags = [], input, env) {
  return run(dir, ['--no-xgent-init', '--context-goal-guard', ...flags], input, env);
}

function guardManifest(dir) {
  return JSON.parse(fs.readFileSync(path.join(dir, '.codex', 'hooks.json'), 'utf8'));
}

test('guard opt-in copies the script, creates both hooks and prints manual steps', (t) => {
  const dir = project(t);
  const result = runGuard(dir);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.readFileSync(path.join(dir, '.codex', 'hooks', 'context-goal-guard.py'), 'utf8'),
    fs.readFileSync(path.join(root, 'hooks', 'codex', 'context-goal-guard.py'), 'utf8'));
  const manifest = guardManifest(dir);
  for (const event of ['PreToolUse', 'Stop']) {
    assert.equal(manifest.hooks[event].length, 1);
    assert.equal(manifest.hooks[event][0].hooks[0].timeout, 3);
    assert.equal(manifest.hooks[event][0].hooks[0].type, 'command');
    assert.match(manifest.hooks[event][0].hooks[0].command, /context-goal-guard\.py' --threshold 65$/);
  }
  assert.match(result.stdout, /严格超过 65%/);
  for (const text of ['/hooks', '信任', '0.160.0', '65%', 'paused', '/goal resume', '/goal pause', '新开对话']) {
    assert.ok(result.stdout.includes(text), text);
  }
});

test('guard merge preserves mixed handlers and fields and is idempotent', (t) => {
  const dir = project(t);
  const codex = path.join(dir, '.codex');
  fs.mkdirSync(codex);
  const file = path.join(codex, 'hooks.json');
  const custom = { type: 'command', command: 'custom-hook', timeout: 8 };
  const impeccable = { type: 'command', command: '".agents/skills/impeccable/scripts/impeccable" hook' };
  const mixed = { matcher: 'Bash', customField: true, hooks: [custom,
    { type: 'command', command: 'python3 "/old/.codex/hooks/context-goal-guard.py"', timeout: 9 }] };
  const existing = { version: 1, description: 'keep', custom: { preserve: true }, hooks: {
    PreToolUse: [mixed], Stop: [{ hooks: [impeccable] }], SessionStart: [{ hooks: [custom] }],
  } };
  fs.writeFileSync(file, JSON.stringify(existing));
  const first = runGuard(dir, ['--providers=.claude']);
  assert.equal(first.status, 0, first.stderr);
  const next = guardManifest(dir);
  assert.deepEqual(next.custom, existing.custom);
  assert.equal(next.description, existing.description);
  assert.equal(next.version, existing.version);
  assert.deepEqual(next.hooks.SessionStart, existing.hooks.SessionStart);
  assert.deepEqual(next.hooks.PreToolUse[0], { ...mixed, hooks: [custom] });
  assert.deepEqual(next.hooks.Stop[0], existing.hooks.Stop[0]);
  assert.equal(next.hooks.PreToolUse.length, 2);
  assert.equal(next.hooks.Stop.length, 2);
  const script = path.join(codex, 'hooks', 'context-goal-guard.py');
  fs.utimesSync(script, 1000, 1000);
  fs.utimesSync(file, 1000, 1000);
  const second = runGuard(dir);
  assert.equal(second.status, 0, second.stderr);
  assert.deepEqual(guardManifest(dir), next);
  assert.equal(fs.statSync(file).mtimeMs, 1000000);
  assert.equal(fs.statSync(script).mtimeMs, 1000000);
});

test('guard and bundled impeccable hooks survive repeated full installations together', (t) => {
  const dir = project(t);
  const cache = project(t);
  const meta = JSON.parse(fs.readFileSync(path.join(root, 'vendor', 'impeccable', 'VERSION.json')));
  const engine = path.join(root, 'vendor', 'impeccable', 'engine', `${process.platform}-${process.arch}`, 'impeccable');
  if (meta.engines[`${process.platform}-${process.arch}`]) {
    if (!fs.existsSync(engine)) return t.skip('local vendor engine needed for offline test');
    const dest = path.join(cache, '.impeccable', 'bin', meta.engineVersion);
    fs.mkdirSync(dest, { recursive: true });
    fs.copyFileSync(engine, path.join(dest, 'impeccable'));
  }
  const command = ['-e', 'require("node:os").homedir = () => process.env.XGENT_INSTALL_TEST_CACHE; require(process.argv[1]);',
    cli, 'install', dir, '--providers=.agents', '--no-xgent-init', '--context-goal-guard'];
  let first;
  for (let i = 0; i < 2; i += 1) {
    const result = spawnSync(process.execPath, command, {
      encoding: 'utf8', input: '', timeout: 10000, env: { ...process.env, XGENT_INSTALL_TEST_CACHE: cache },
    });
    assert.equal(result.status, 0, result.stderr);
    const manifest = guardManifest(dir);
    for (const event of ['PreToolUse', 'Stop']) {
      const handlers = manifest.hooks[event].flatMap((entry) => entry.hooks);
      assert.equal(handlers.filter((handler) => handler.command.includes('context-goal-guard.py')).length, 1);
    }
    for (const event of ['PostToolUse', 'Stop']) {
      const handlers = manifest.hooks[event].flatMap((entry) => entry.hooks);
      assert.ok(handlers.some((handler) => handler.command.includes('skills/impeccable/scripts/impeccable')));
    }
    if (first) assert.deepEqual(manifest, first);
    first = manifest;
  }
});

test('guard command quotes shell characters and uses the canonical project path', (t) => {
  const base = project(t);
  const dir = path.join(base, "My 'Project $HOME `id` $(touch injected)");
  fs.mkdirSync(dir);
  const alias = path.join(base, 'alias');
  fs.symlinkSync(dir, alias);
  const result = runGuard(alias);
  assert.equal(result.status, 0, result.stderr);
  const command = guardManifest(dir).hooks.PreToolUse[0].hooks[0].command;
  const hook = spawnSync('/bin/sh', ['-c', command], { cwd: base, input: JSON.stringify({ hook_event_name: 'Stop' }), encoding: 'utf8' });
  assert.equal(hook.status, 0, hook.stderr);
  assert.doesNotThrow(() => JSON.parse(hook.stdout));
  assert.ok(command.includes(fs.realpathSync(base)));
  assert.equal(fs.existsSync(path.join(base, 'injected')), false);
});

test('missing Python fails guard installation before creating files and gives guidance', (t) => {
  const dir = project(t);
  const result = runGuard(dir, [], undefined, { ...process.env, PATH: dir });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Python 3\.9/);
  assert.match(result.stderr, /python3 --version/);
  assert.deepEqual(fs.readdirSync(dir), []);
});

for (const raw of ['invalid json', '[]', '{"hooks":{"Stop":{}}}', '{"hooks":null}']) {
  test(`guard refuses malformed manifest ${raw} without overwriting it`, (t) => {
    const dir = project(t);
    fs.mkdirSync(path.join(dir, '.codex'));
    const file = path.join(dir, '.codex', 'hooks.json');
    fs.writeFileSync(file, raw);
    const result = runGuard(dir);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /未修改/);
    assert.equal(fs.readFileSync(file, 'utf8'), raw);
    assert.equal(fs.existsSync(path.join(dir, '.codex', 'hooks')), false);
  });
}

test('force backs up invalid guard JSON before replacing it', (t) => {
  const dir = project(t);
  fs.mkdirSync(path.join(dir, '.codex'));
  const file = path.join(dir, '.codex', 'hooks.json');
  fs.writeFileSync(file, 'invalid json');
  const result = runGuard(dir, ['--force']);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.readFileSync(`${file}.bak`, 'utf8'), 'invalid json');
  assert.equal(guardManifest(dir).hooks.Stop.length, 1);
});

for (const input of ['y\n', ' YES \n', 'n\n', '\n', '']) {
  test(`guard interactive answer ${JSON.stringify(input)} respects opt-in`, (t) => {
    const dir = project(t);
    // 直接调用 CLI,不传 guard 参数,覆盖默认询问。
    const actual = spawnSync(process.execPath, ['-e', 'process.stdin.isTTY = true; require(process.argv[1]);',
      cli, 'install', dir, '--no-impeccable', '--no-xgent-init'], { input, encoding: 'utf8', timeout: 10000 });
    assert.equal(actual.status, 0, actual.stderr);
    assert.match(actual.stdout, /是否.*Codex.*\[y\/N\]/);
    assert.equal(fs.existsSync(path.join(dir, '.codex', 'hooks.json')), /^(y|yes)$/i.test(input.trim()));
  });
}

test('both interactive questions consume their own answers', (t) => {
  const dir = project(t);
  const result = spawnSync(process.execPath, ['-e', 'process.stdin.isTTY = true; require(process.argv[1]);',
    cli, 'install', dir, '--no-impeccable'], { input: 'y\ny\n', encoding: 'utf8', timeout: 10000 });
  assert.equal(result.status, 0, result.stderr);
  assertAgents(dir);
  assert.equal(guardManifest(dir).hooks.Stop.length, 1);
});

test('non-interactive default and explicit guard skip leave existing Codex files untouched', (t) => {
  const dir = project(t);
  fs.mkdirSync(path.join(dir, '.codex'));
  const file = path.join(dir, '.codex', 'hooks.json');
  fs.writeFileSync(file, 'existing config');
  for (const flags of [[], ['--no-context-goal-guard']]) {
    const result = spawnSync(process.execPath, [cli, 'install', dir, '--no-impeccable', '--no-xgent-init', ...flags],
      { input: '', encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stdout, /\[y\/N\]/);
    assert.equal(fs.readFileSync(file, 'utf8'), 'existing config');
    assert.equal(fs.existsSync(path.join(dir, '.codex', 'hooks')), false);
  }
});

test('conflicting guard flags fail before creating files', (t) => {
  const dir = project(t);
  const result = runGuard(dir, ['--no-context-goal-guard']);
  assert.equal(result.status, 1);
  assert.deepEqual(fs.readdirSync(dir), []);
});

function assertAgents(dir) {
  assert.equal(fs.readFileSync(path.join(dir, 'AGENTS.md'), 'utf8'), fs.readFileSync(template, 'utf8'));
  for (const name of ['CLAUDE.md', 'PRODUCT.md', 'DESIGN.md']) {
    assert.equal(fs.existsSync(path.join(dir, name)), false);
  }
  for (const provider of ['.claude', '.cursor', '.agents']) {
    assert.equal(fs.existsSync(path.join(dir, provider, 'skills', 'xgent-init')), false);
  }
}

for (const answer of ['y\n', ' YES \n']) {
  test(`confirmation ${JSON.stringify(answer)} creates only AGENTS.md alongside hooks`, (t) => {
    const dir = project(t);
    const result = run(dir, [], answer);
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /\[y\/N\]/);
    assertAgents(dir);
    assert.ok(fs.existsSync(path.join(dir, '.claude', 'hooks', 'xgent-statusline.js')));
  });
}

for (const answer of ['n\n', '\n', 'maybe\n', '']) {
  test(`answer ${JSON.stringify(answer)} skips AGENTS.md and still installs hooks`, (t) => {
    const dir = project(t);
    const result = run(dir, [], answer);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(fs.existsSync(path.join(dir, 'AGENTS.md')), false);
    assert.ok(fs.existsSync(path.join(dir, '.claude', 'hooks', 'xgent-statusline.js')));
  });
}

test('non-interactive install skips AGENTS.md without waiting for input', (t) => {
  const dir = project(t);
  const result = run(dir);
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.stdout, /\[y\/N\]/);
  assert.equal(fs.existsSync(path.join(dir, 'AGENTS.md')), false);
});

test('explicit creation is independent of providers, preserves settings and is idempotent', (t) => {
  const dir = project(t);
  fs.mkdirSync(path.join(dir, '.claude'));
  const settings = path.join(dir, '.claude', 'settings.json');
  fs.writeFileSync(settings, JSON.stringify({ env: { PROJECT_SETTING: 'preserve' } }));
  const flags = ['--xgent-init', '--providers=claude,.cursor,.agents'];
  const first = run(dir, flags);
  assert.equal(first.status, 0, first.stderr);
  assert.doesNotMatch(first.stdout, /\[y\/N\]/);
  assertAgents(dir);
  assert.deepEqual(JSON.parse(fs.readFileSync(settings)).env, { PROJECT_SETTING: 'preserve' });
  const agents = path.join(dir, 'AGENTS.md');
  fs.utimesSync(agents, 1000, 1000);
  const second = run(dir, flags);
  assert.equal(second.status, 0, second.stderr);
  assert.equal(fs.statSync(agents).mtimeMs, 1000000);
  assertAgents(dir);
});

test('existing AGENTS.md is preserved without prompting, even with --force', (t) => {
  const dir = project(t);
  const agents = path.join(dir, 'AGENTS.md');
  fs.writeFileSync(agents, 'project conventions');
  for (const flags of [[], ['--xgent-init'], ['--xgent-init', '--force']]) {
    const result = run(dir, flags, 'y\n');
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stdout, /\[y\/N\]/);
    assert.equal(fs.readFileSync(agents, 'utf8'), 'project conventions');
  }
});

test('explicit skip bypasses the prompt and preserves existing documents and skill', (t) => {
  const dir = project(t);
  const skillDir = path.join(dir, '.cursor', 'skills', 'xgent-init');
  fs.mkdirSync(skillDir, { recursive: true });
  const skill = path.join(skillDir, 'SKILL.md');
  fs.writeFileSync(skill, 'existing skill');
  const claude = path.join(dir, 'CLAUDE.md');
  fs.writeFileSync(claude, 'existing Claude instructions');
  const result = run(dir, ['--no-xgent-init'], 'y\n');
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.stdout, /\[y\/N\]/);
  assert.equal(fs.existsSync(path.join(dir, 'AGENTS.md')), false);
  assert.equal(fs.readFileSync(skill, 'utf8'), 'existing skill');
  assert.equal(fs.readFileSync(claude, 'utf8'), 'existing Claude instructions');
});

for (const flags of [['--xgent-init', '--no-xgent-init'], ['--xgent-init', '--providers=unknown']]) {
  test(`invalid options ${flags.join(' ')} fail before writing files`, (t) => {
    const dir = project(t);
    const result = run(dir, flags);
    assert.equal(result.status, 1);
    assert.deepEqual(fs.readdirSync(dir), []);
  });
}

const CLAUDE_GUARD_COMMAND = 'node "$CLAUDE_PROJECT_DIR/.claude/hooks/context-goal-guard.js" --threshold 70';

function runClaudeGuard(dir, flags = [], input, env) {
  return run(dir, ['--no-xgent-init', '--claude-context-goal-guard', ...flags], input, env);
}

function claudeSettings(dir) {
  return JSON.parse(fs.readFileSync(path.join(dir, '.claude', 'settings.local.json'), 'utf8'));
}

function claudeGuardHandlers(settings, event) {
  return settings.hooks[event].flatMap((entry) => entry.hooks).filter((handler) => handler.command.includes('context-goal-guard.js'));
}

test('Claude guard opt-in copies the script, adds both local handlers and prints manual steps', (t) => {
  const dir = project(t);
  const result = runClaudeGuard(dir);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.readFileSync(path.join(dir, '.claude', 'hooks', 'context-goal-guard.js'), 'utf8'),
    fs.readFileSync(path.join(root, 'hooks', 'claude', 'context-goal-guard.js'), 'utf8'));
  const settings = claudeSettings(dir);
  for (const event of ['PostToolBatch', 'Stop']) {
    assert.deepEqual(settings.hooks[event], [{ hooks: [{ type: 'command', command: CLAUDE_GUARD_COMMAND, timeout: 5 }] }]);
  }
  assert.deepEqual(Object.keys(settings.hooks).sort(), ['PostToolBatch', 'Stop']);
  assert.equal(fs.existsSync(path.join(dir, '.codex')), false);
  assert.equal(JSON.parse(fs.readFileSync(path.join(dir, '.claude', 'settings.json'), 'utf8')).hooks, undefined);
  for (const text of ['claude --version', '2.1.288', '信任', '/hooks', 'settings.local.json', '70%', 'Goal paused',
    '/clear', '/goal', 'Stop hook error', 'disableAllHooks', 'context_window_size']) {
    assert.ok(result.stdout.includes(text), text);
  }
});

test('installed Claude guard command runs from a project path with shell characters', (t) => {
  const base = project(t);
  const dir = path.join(base, "My 'Project $HOME `id` $(touch injected)");
  fs.mkdirSync(dir);
  const result = runClaudeGuard(dir);
  assert.equal(result.status, 0, result.stderr);
  const hook = spawnSync('/bin/sh', ['-c', claudeSettings(dir).hooks.Stop[0].hooks[0].command], {
    cwd: base, input: JSON.stringify({ hook_event_name: 'Stop', stop_hook_active: false, transcript_path: path.join(dir, 'none') }),
    encoding: 'utf8', env: { ...process.env, CLAUDE_PROJECT_DIR: dir },
  });
  assert.equal(hook.status, 0, hook.stderr);
  assert.match(JSON.parse(hook.stdout).systemMessage, /检测不可用/);
  assert.equal(fs.existsSync(path.join(base, 'injected')), false);
});

test('Claude guard merge preserves other settings and handlers and is idempotent', (t) => {
  const dir = project(t);
  const claude = path.join(dir, '.claude');
  fs.mkdirSync(claude);
  const file = path.join(claude, 'settings.local.json');
  const custom = { type: 'command', command: 'custom-hook', timeout: 8 };
  const old = { type: 'command', command: 'node "/old/place/.claude/hooks/context-goal-guard.js"', timeout: 9 };
  const mixed = { matcher: 'Bash', customField: true, hooks: [custom, old] };
  const existing = { permissions: { allow: ['Bash(ls)'] }, env: { KEEP: '1' }, hooks: {
    PostToolBatch: [mixed], Stop: [{ hooks: [old] }, { hooks: [custom] }], SessionStart: [{ hooks: [custom] }],
  } };
  fs.writeFileSync(file, JSON.stringify(existing));
  const first = runClaudeGuard(dir);
  assert.equal(first.status, 0, first.stderr);
  const next = claudeSettings(dir);
  assert.deepEqual(next.permissions, existing.permissions);
  assert.deepEqual(next.env, existing.env);
  assert.deepEqual(next.hooks.SessionStart, existing.hooks.SessionStart);
  assert.deepEqual(next.hooks.PostToolBatch, [{ ...mixed, hooks: [custom] },
    { hooks: [{ type: 'command', command: CLAUDE_GUARD_COMMAND, timeout: 5 }] }]);
  assert.deepEqual(next.hooks.Stop, [{ hooks: [custom] },
    { hooks: [{ type: 'command', command: CLAUDE_GUARD_COMMAND, timeout: 5 }] }]);
  const script = path.join(claude, 'hooks', 'context-goal-guard.js');
  fs.utimesSync(script, 1000, 1000);
  fs.utimesSync(file, 1000, 1000);
  const second = runClaudeGuard(dir);
  assert.equal(second.status, 0, second.stderr);
  assert.match(second.stdout, /未变 {2}\.claude\/settings\.local\.json/);
  assert.deepEqual(claudeSettings(dir), next);
  assert.equal(fs.statSync(file).mtimeMs, 1000000);
  assert.equal(fs.statSync(script).mtimeMs, 1000000);
});

test('Claude guard and bundled impeccable Claude hooks survive repeated full installations together', (t) => {
  const dir = project(t);
  const cache = project(t);
  const meta = JSON.parse(fs.readFileSync(path.join(root, 'vendor', 'impeccable', 'VERSION.json')));
  const engine = path.join(root, 'vendor', 'impeccable', 'engine', `${process.platform}-${process.arch}`, 'impeccable');
  if (meta.engines[`${process.platform}-${process.arch}`]) {
    if (!fs.existsSync(engine)) return t.skip('local vendor engine needed for offline test');
    const dest = path.join(cache, '.impeccable', 'bin', meta.engineVersion);
    fs.mkdirSync(dest, { recursive: true });
    fs.copyFileSync(engine, path.join(dest, 'impeccable'));
  }
  const command = ['-e', 'require("node:os").homedir = () => process.env.XGENT_INSTALL_TEST_CACHE; require(process.argv[1]);',
    cli, 'install', dir, '--providers=.claude', '--no-xgent-init', '--no-context-goal-guard', '--claude-context-goal-guard'];
  let first;
  for (let i = 0; i < 2; i += 1) {
    const result = spawnSync(process.execPath, command, {
      encoding: 'utf8', input: '', timeout: 10000, env: { ...process.env, XGENT_INSTALL_TEST_CACHE: cache },
    });
    assert.equal(result.status, 0, result.stderr);
    const settings = claudeSettings(dir);
    for (const event of ['PostToolBatch', 'Stop']) assert.equal(claudeGuardHandlers(settings, event).length, 1);
    assert.ok(Object.values(settings.hooks).flat().flatMap((entry) => entry.hooks)
      .some((handler) => handler.command.includes('skills/impeccable/scripts/impeccable')));
    if (first) assert.deepEqual(settings, first);
    first = settings;
  }
});

for (const raw of ['invalid json', '[]', '{"hooks":{"Stop":{}}}', '{"hooks":null}']) {
  test(`Claude guard refuses malformed settings ${raw} without overwriting it`, (t) => {
    const dir = project(t);
    fs.mkdirSync(path.join(dir, '.claude'));
    const file = path.join(dir, '.claude', 'settings.local.json');
    fs.writeFileSync(file, raw);
    const result = runClaudeGuard(dir);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /未修改/);
    assert.equal(fs.readFileSync(file, 'utf8'), raw);
    assert.equal(fs.existsSync(path.join(dir, '.claude', 'hooks')), false);
  });
}

test('force backs up invalid Claude settings before replacing them', (t) => {
  const dir = project(t);
  fs.mkdirSync(path.join(dir, '.claude'));
  const file = path.join(dir, '.claude', 'settings.local.json');
  fs.writeFileSync(file, 'invalid json');
  const result = runClaudeGuard(dir, ['--force']);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.readFileSync(`${file}.bak`, 'utf8'), 'invalid json');
  assert.equal(claudeGuardHandlers(claudeSettings(dir), 'Stop').length, 1);
});

for (const [input, codex, claude] of [['n\ny\n', false, true], ['y\nn\n', true, false], ['y\nyes\n', true, true], ['\n', false, false]]) {
  test(`Codex and Claude guard questions answer ${JSON.stringify(input)} independently`, (t) => {
    const dir = project(t);
    const result = spawnSync(process.execPath, ['-e', 'process.stdin.isTTY = true; require(process.argv[1]);',
      cli, 'install', dir, '--no-impeccable', '--no-xgent-init'], { input, encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /是否.*Codex.*\[y\/N\]/);
    assert.match(result.stdout, /是否.*Claude Code.*\[y\/N\]/);
    assert.equal(fs.existsSync(path.join(dir, '.codex', 'hooks.json')), codex);
    assert.equal(fs.existsSync(path.join(dir, '.claude', 'settings.local.json')), claude);
    assert.equal(fs.existsSync(path.join(dir, '.claude', 'hooks', 'context-goal-guard.js')), claude);
  });
}

test('non-interactive default and explicit Claude guard skip leave local settings untouched', (t) => {
  const dir = project(t);
  fs.mkdirSync(path.join(dir, '.claude'));
  const file = path.join(dir, '.claude', 'settings.local.json');
  fs.writeFileSync(file, 'existing config');
  for (const flags of [[], ['--no-claude-context-goal-guard']]) {
    const result = spawnSync(process.execPath, [cli, 'install', dir, '--no-impeccable', '--no-xgent-init', '--no-context-goal-guard', ...flags],
      { input: '', encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stdout, /\[y\/N\]/);
    assert.equal(/--claude-context-goal-guard 安装/.test(result.stdout), flags.length === 0);
    assert.equal(fs.readFileSync(file, 'utf8'), 'existing config');
    assert.equal(fs.existsSync(path.join(dir, '.claude', 'hooks', 'context-goal-guard.js')), false);
  }
});

test('conflicting Claude guard flags fail before creating files', (t) => {
  const dir = project(t);
  const result = runClaudeGuard(dir, ['--no-claude-context-goal-guard']);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /--claude-context-goal-guard 与 --no-claude-context-goal-guard 不能同时使用/);
  assert.deepEqual(fs.readdirSync(dir), []);
});

test('help lists both Claude guard flags', () => {
  const result = spawnSync(process.execPath, [cli, 'help'], { encoding: 'utf8' });
  assert.match(result.stdout, /--claude-context-goal-guard/);
  assert.match(result.stdout, /--no-claude-context-goal-guard/);
});

function guardCommands(manifest, events, marker) {
  return events.map((event) => manifest.hooks[event].flatMap((entry) => entry.hooks)
    .filter((handler) => handler.command.includes(marker)).map((handler) => handler.command));
}

test('threshold flags set each guard separately, imply installation and survive reinstalls', (t) => {
  const dir = project(t);
  const first = spawnSync(process.execPath, ['-e', 'process.stdin.isTTY = true; require(process.argv[1]);',
    cli, 'install', dir, '--no-impeccable', '--no-xgent-init',
    '--context-goal-guard-threshold=60', '--claude-context-goal-guard-threshold=75'], { input: '', encoding: 'utf8', timeout: 10000 });
  assert.equal(first.status, 0, first.stderr);
  assert.doesNotMatch(first.stdout, /\[y\/N\]/);
  assert.match(first.stdout, /严格超过 60%/);
  assert.match(first.stdout, /严格超过 75%/);
  const codex = () => guardCommands(guardManifest(dir), ['PreToolUse', 'Stop'], 'context-goal-guard.py');
  const claude = () => guardCommands(claudeSettings(dir), ['PostToolBatch', 'Stop'], 'context-goal-guard.js');
  for (const [command] of codex()) assert.match(command, / --threshold 60$/);
  for (const [command] of claude()) assert.equal(command, CLAUDE_GUARD_COMMAND.replace('70', '75'));

  const kept = run(dir, ['--no-xgent-init', '--context-goal-guard', '--claude-context-goal-guard']);
  assert.equal(kept.status, 0, kept.stderr);
  for (const commands of [...codex(), ...claude()]) assert.equal(commands.length, 1);
  for (const [command] of codex()) assert.match(command, / --threshold 60$/);
  for (const [command] of claude()) assert.match(command, / --threshold 75$/);
  assert.match(kept.stdout, /严格超过 75%/);

  const changed = run(dir, ['--no-xgent-init', '--claude-context-goal-guard-threshold=68']);
  assert.equal(changed.status, 0, changed.stderr);
  for (const [command] of claude()) assert.match(command, / --threshold 68$/);
  for (const [command] of codex()) assert.match(command, / --threshold 60$/);
});

for (const flag of ['--context-goal-guard-threshold', '--claude-context-goal-guard-threshold']) {
  for (const value of ['0', '100', '6.5', '065', 'abc', '']) {
    test(`${flag}=${value} fails before creating files`, (t) => {
      const dir = project(t);
      const result = run(dir, ['--no-xgent-init', `${flag}=${value}`]);
      assert.equal(result.status, 1);
      assert.match(result.stderr, new RegExp(`${flag}.*1–99`));
      assert.deepEqual(fs.readdirSync(dir), []);
    });
  }
  test(`${flag} conflicts with skipping the same guard`, (t) => {
    const dir = project(t);
    const skip = flag.replace(/^--/, '--no-').replace(/-threshold$/, '');
    const result = spawnSync(process.execPath, [cli, 'install', dir, '--no-impeccable', '--no-xgent-init',
      '--no-context-goal-guard', '--no-claude-context-goal-guard', `${flag}=70`], { input: '', encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 1);
    assert.match(result.stderr, new RegExp(`${skip}`));
    assert.deepEqual(fs.readdirSync(dir), []);
  });
}
