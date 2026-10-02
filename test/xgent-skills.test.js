'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const cli = path.join(root, 'bin', 'xgent-skills.js');
const source = path.join(root, 'skills', 'xgent-init');

function project(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'xgent-install-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  return dir;
}

function run(dir, flags = [], input) {
  // 仅模拟终端检测,输入仍经过 CLI 的 readline 询问和回答处理。
  const prefix = input === undefined ? [] : ['-e', 'process.stdin.isTTY = true; require(process.argv[1]);'];
  return spawnSync(process.execPath, [...prefix, cli, 'install', dir, '--no-impeccable', ...flags], {
    input: input ?? '', encoding: 'utf8', timeout: 10000,
  });
}

function snapshot(dir) {
  return Object.fromEntries(fs.readdirSync(dir, { withFileTypes: true }).map((entry) => {
    const file = path.join(dir, entry.name);
    return [entry.name, entry.isDirectory() ? snapshot(file) : fs.readFileSync(file, 'utf8')];
  }));
}

function installed(dir, provider = '.claude') {
  return path.join(dir, provider, 'skills', 'xgent-init');
}

for (const answer of ['y\n', ' YES \n']) {
  test(`confirmation ${JSON.stringify(answer)} installs the complete skill`, (t) => {
    const dir = project(t);
    const result = run(dir, [], answer);
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /\[y\/N\]/);
    assert.deepEqual(snapshot(installed(dir)), snapshot(source));
    for (const name of ['CLAUDE.md', 'AGENTS.md', 'PRODUCT.md', 'DESIGN.md']) {
      assert.equal(fs.existsSync(path.join(dir, name)), false);
    }
  });
}

for (const answer of ['n\n', '\n', 'maybe\n', '']) {
  test(`answer ${JSON.stringify(answer)} skips the skill and still installs hooks`, (t) => {
    const dir = project(t);
    const result = run(dir, [], answer);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(fs.existsSync(installed(dir)), false);
    assert.ok(fs.existsSync(path.join(dir, '.claude', 'hooks', 'xgent-statusline.js')));
  });
}

test('non-interactive install skips the skill without waiting for input', (t) => {
  const dir = project(t);
  const result = run(dir);
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.stdout, /\[y\/N\]/);
  assert.equal(fs.existsSync(installed(dir)), false);
});

test('explicit installation respects providers, preserves settings and is idempotent', (t) => {
  const dir = project(t);
  fs.mkdirSync(path.join(dir, '.claude'));
  const settings = path.join(dir, '.claude', 'settings.json');
  fs.writeFileSync(settings, JSON.stringify({ env: { PROJECT_SETTING: 'preserve' } }));
  const flags = ['--xgent-init', '--providers=claude,.cursor,.agents'];
  const first = run(dir, flags);
  assert.equal(first.status, 0, first.stderr);
  assert.doesNotMatch(first.stdout, /\[y\/N\]/);
  for (const provider of ['.claude', '.cursor', '.agents']) {
    assert.deepEqual(snapshot(installed(dir, provider)), snapshot(source));
  }
  assert.deepEqual(JSON.parse(fs.readFileSync(settings)).env, { PROJECT_SETTING: 'preserve' });
  const skillFile = path.join(installed(dir), 'SKILL.md');
  fs.utimesSync(skillFile, 1000, 1000);
  const second = run(dir, flags);
  assert.equal(second.status, 0, second.stderr);
  assert.equal(fs.statSync(skillFile).mtimeMs, 1000000);

  fs.writeFileSync(skillFile, 'old version');
  fs.writeFileSync(path.join(installed(dir), 'obsolete.txt'), 'old resource');
  const update = run(dir, flags);
  assert.equal(update.status, 0, update.stderr);
  assert.deepEqual(snapshot(installed(dir)), snapshot(source));
});

test('provider detection uses directories present before creating Claude hooks', (t) => {
  const dir = project(t);
  fs.mkdirSync(path.join(dir, '.cursor'));
  const result = run(dir, ['--xgent-init']);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(snapshot(installed(dir, '.cursor')), snapshot(source));
  assert.equal(fs.existsSync(installed(dir)), false);
});

test('explicit skip bypasses the prompt and preserves an existing skill', (t) => {
  const dir = project(t);
  fs.mkdirSync(installed(dir), { recursive: true });
  fs.writeFileSync(path.join(installed(dir), 'SKILL.md'), 'existing skill');
  const result = run(dir, ['--no-xgent-init'], 'y\n');
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.stdout, /\[y\/N\]/);
  assert.equal(fs.readFileSync(path.join(installed(dir), 'SKILL.md'), 'utf8'), 'existing skill');
});

for (const flags of [['--xgent-init', '--no-xgent-init'], ['--xgent-init', '--providers=unknown']]) {
  test(`invalid options ${flags.join(' ')} fail before writing files`, (t) => {
    const dir = project(t);
    const result = run(dir, flags);
    assert.equal(result.status, 1);
    assert.deepEqual(fs.readdirSync(dir), []);
  });
}
