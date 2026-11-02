'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const cli = path.join(root, 'bin', 'xgent-skills.js');

function project(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'xgent-install-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  return dir;
}

function run(dir, flags = [], input, env = process.env) {
  // 模拟 TTY,确认交互终端也不会询问或创建文档。
  const prefix = input === undefined ? [] : ['-e', 'process.stdin.isTTY = true; require(process.argv[1]);'];
  return spawnSync(process.execPath, [...prefix, cli, 'install', dir, '--no-phoenix-ui', '--no-phoenix-diagram', ...flags], {
    input: input ?? '', encoding: 'utf8', timeout: 10000, env,
  });
}

for (const input of [undefined, 'y\n']) {
  test(`${input === undefined ? 'non-interactive' : 'interactive'} install only suggests optional AGENTS.md optimization`, (t) => {
    const dir = project(t);
    const result = run(dir, [], input);
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stdout, /\[y\/N\]/);
    for (const name of ['AGENTS.md', 'CLAUDE.md', 'PRODUCT.md', 'DESIGN.md']) {
      assert.equal(fs.existsSync(path.join(dir, name)), false);
    }
    assert.match(result.stdout, /可选.*optimize-agents-md.*opinionated DSL.*AGENTS\.md.*不喜欢.*可忽略/);
    assert.match(result.stdout, /\/optimize-agents-md/);
    assert.match(result.stdout, /\$optimize-agents-md/);
    assert.ok(fs.existsSync(path.join(dir, '.claude', 'hooks', 'xgent-statusline.js')));
    assert.equal(fs.existsSync(path.join(dir, '.claude', 'skills')), false);
  });
}

test('install preserves settings and is idempotent', (t) => {
  const dir = project(t);
  fs.mkdirSync(path.join(dir, '.claude'));
  const settings = path.join(dir, '.claude', 'settings.json');
  fs.writeFileSync(settings, JSON.stringify({ env: { PROJECT_SETTING: 'preserve' } }));
  const flags = ['--providers=claude,.cursor,.agents'];
  const first = run(dir, flags);
  assert.equal(first.status, 0, first.stderr);
  assert.doesNotMatch(first.stdout, /\[y\/N\]/);
  assert.deepEqual(JSON.parse(fs.readFileSync(settings)).env, { PROJECT_SETTING: 'preserve' });
  fs.utimesSync(settings, 1000, 1000);
  const second = run(dir, flags);
  assert.equal(second.status, 0, second.stderr);
  assert.equal(fs.statSync(settings).mtimeMs, 1000000);
  assert.deepEqual(JSON.parse(fs.readFileSync(settings)).env, { PROJECT_SETTING: 'preserve' });
});

test('install succeeds without xgent-init or AGENTS templates in the package', (t) => {
  const pkg = project(t);
  const dir = project(t);
  for (const relative of ['package.json', 'bin', '.claude/hooks',
    'skills/phoenix-ui/scripts', 'skills/phoenix-diagram/scripts']) {
    fs.cpSync(path.join(root, relative), path.join(pkg, relative), { recursive: true });
  }
  const result = spawnSync(process.execPath, [path.join(pkg, 'bin', 'xgent-skills.js'),
    'install', dir, '--no-phoenix-ui', '--no-phoenix-diagram'], {
    input: '', encoding: 'utf8', timeout: 10000,
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.existsSync(path.join(dir, 'AGENTS.md')), false);
  assert.match(result.stdout, /optimize-agents-md/);
  assert.ok(fs.existsSync(path.join(dir, '.claude', 'hooks', 'xgent-statusline.js')));
});

test('install preserves existing documents and skill without prompting', (t) => {
  const dir = project(t);
  const skillDir = path.join(dir, '.cursor', 'skills', 'xgent-init');
  fs.mkdirSync(skillDir, { recursive: true });
  const skill = path.join(skillDir, 'SKILL.md');
  fs.writeFileSync(skill, 'existing skill');
  const documents = ['AGENTS.md', 'CLAUDE.md', 'PRODUCT.md', 'DESIGN.md'];
  for (const name of documents) {
    fs.writeFileSync(path.join(dir, name), `existing ${name}`);
    fs.utimesSync(path.join(dir, name), 1000, 1000);
  }
  for (const input of [undefined, 'y\n']) {
    const result = run(dir, [], input);
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stdout, /\[y\/N\]/);
    for (const name of documents) {
      assert.equal(fs.readFileSync(path.join(dir, name), 'utf8'), `existing ${name}`);
      assert.equal(fs.statSync(path.join(dir, name)).mtimeMs, 1000000);
    }
    assert.equal(fs.readFileSync(skill, 'utf8'), 'existing skill');
  }
});

for (const flags of [['--xgent-init'], ['--no-xgent-init'], ['--providers=unknown'], ['--context-goal-guard'], ['--force']]) {
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
  const result = spawnSync(process.execPath, [cli, 'install', dir, '--no-phoenix-diagram', '--no-impeccable'], {
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
  const result = spawnSync(process.execPath, [cli, 'install', dir], {
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
  const result = run(dir);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /不是合法 JSON/);
  assert.deepEqual(fs.readdirSync(dir), ['.claude']);
  assert.deepEqual(fs.readdirSync(path.join(dir, '.claude')), ['settings.json']);
});
