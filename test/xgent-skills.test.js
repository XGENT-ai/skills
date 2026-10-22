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
  return spawnSync(process.execPath, [...prefix, cli, 'install', dir, '--no-phoenix-ui', '--no-phoenix-diagram', ...flags], {
    input: input ?? '', encoding: 'utf8', timeout: 10000, env,
  });
}

test('accepting AGENTS.md through the prompt creates it', (t) => {
  const dir = project(t);
  const result = spawnSync(process.execPath, ['-e', 'process.stdin.isTTY = true; require(process.argv[1]);',
    cli, 'install', dir, '--no-phoenix-diagram', '--no-impeccable'], { input: 'y\ny\n', encoding: 'utf8', timeout: 10000 });
  assert.equal(result.status, 0, result.stderr);
  assertAgents(dir);
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

test('existing AGENTS.md is preserved without prompting', (t) => {
  const dir = project(t);
  const agents = path.join(dir, 'AGENTS.md');
  fs.writeFileSync(agents, 'project conventions');
  for (const flags of [[], ['--xgent-init']]) {
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

for (const flags of [['--xgent-init', '--no-xgent-init'], ['--xgent-init', '--providers=unknown'], ['--context-goal-guard'], ['--force']]) {
  test(`invalid options ${flags.join(' ')} fail before writing files`, (t) => {
    const dir = project(t);
    const result = run(dir, flags);
    assert.equal(result.status, 1);
    assert.deepEqual(fs.readdirSync(dir), []);
  });
}

test('deprecated --no-impeccable skips Phoenix UI like --no-phoenix-ui and says so', (t) => {
  const dir = project(t);
  const home = project(t);
  const result = spawnSync(process.execPath, [cli, 'install', dir, '--no-phoenix-diagram', '--no-impeccable', '--no-xgent-init'], {
    input: '', encoding: 'utf8', timeout: 10000, env: { ...process.env, PHOENIX_UI_HOME: home },
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /--no-impeccable 已弃用,等同 --no-phoenix-ui/);
  assert.doesNotMatch(result.stdout, /Phoenix UI \d/);
  assert.ok(fs.existsSync(path.join(dir, '.claude', 'hooks')));
  assert.equal(fs.existsSync(path.join(dir, '.phoenix-ui')), false);
  assert.deepEqual(fs.readdirSync(home), []);
});

test('an unavailable Phoenix engine stops the whole install before any project write', (t) => {
  const dir = project(t);
  const home = project(t);
  fs.mkdirSync(path.join(dir, '.claude'));
  fs.writeFileSync(path.join(dir, '.claude', 'settings.json'), '{"keep":true}\n');
  const result = spawnSync(process.execPath, [cli, 'install', dir, '--xgent-init'], {
    input: '', encoding: 'utf8', timeout: 10000,
    env: { ...process.env, PHOENIX_UI_HOME: home, HTTPS_PROXY: '', https_proxy: '', HTTP_PROXY: '', http_proxy: '' },
  });
  assert.equal(result.status, 4, result.stdout + result.stderr);
  assert.match(result.stderr, /Phoenix UI engine 未就绪,未修改项目/);
  assert.match(result.stderr, /--release-dir=DIR/);
  assert.match(result.stderr, /--no-phoenix-ui/);
  assert.deepEqual(fs.readdirSync(dir).sort(), ['.claude']);
  assert.deepEqual(fs.readdirSync(path.join(dir, '.claude')), ['settings.json']);
  assert.equal(fs.readFileSync(path.join(dir, '.claude', 'settings.json'), 'utf8'), '{"keep":true}\n');
});

test('invalid settings.json stops the install before XGENT hooks are written', (t) => {
  const dir = project(t);
  fs.mkdirSync(path.join(dir, '.claude'));
  fs.writeFileSync(path.join(dir, '.claude', 'settings.json'), 'not json');
  const result = run(dir, ['--xgent-init']);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /不是合法 JSON/);
  assert.deepEqual(fs.readdirSync(dir), ['.claude']);
  assert.deepEqual(fs.readdirSync(path.join(dir, '.claude')), ['settings.json']);
});
