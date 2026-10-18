'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const root = path.resolve(__dirname, '../..');

function project(t) {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram joint installer '));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  return temp;
}
function install(dir, flags) {
  return spawnSync(process.execPath, [path.join(root, 'bin/xgent-skills.js'), 'install', dir, '--no-xgent-init', ...flags], { encoding: 'utf8', input: '', env: { ...process.env, PHOENIX_DIAGRAM_HOME: path.join(dir, 'cache') } });
}

function installOnWindows(dir, flags) {
  const script = `Object.defineProperty(process, 'platform', { value: 'win32' }); process.argv = [process.execPath, ...process.argv.slice(1)]; require(process.argv[1]);`;
  return spawnSync(process.execPath, ['-e', script, path.join(root, 'bin/xgent-skills.js'), 'install', dir, '--no-xgent-init', '--no-phoenix-ui', ...flags], { encoding: 'utf8', input: '' });
}

test('unsupported platforms skip the default diagram branch and preserve the XGENT installation', t => {
  const dir = project(t), result = installOnWindows(dir, []);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /跳过.*Phoenix Diagram.*win32/);
  assert.ok(fs.existsSync(path.join(dir, '.claude/hooks/xgent-statusline.js')));
  assert.ok(fs.existsSync(path.join(dir, '.claude/settings.json')));
  assert.deepEqual(fs.readdirSync(dir), ['.claude']);
  assert.ok(!fs.existsSync(path.join(dir, '.claude/skills/phoenix-diagram')));
});

test('an explicit diagram resource request still fails on unsupported platforms before writing', t => {
  const dir = project(t), result = installOnWindows(dir, [`--diagram-release-dir=${path.join(dir, 'release')}`]);
  assert.equal(result.status, 4);
  assert.match(result.stderr, /Unsupported Phoenix Diagram platform win32/);
  assert.doesNotMatch(result.stderr, /可用 --diagram-release-dir/);
  assert.deepEqual(fs.readdirSync(dir), []);
});

test('missing diagram resources stop the installer before any project write', t => {
  const dir = project(t);
  const result = install(dir, ['--no-phoenix-ui', `--diagram-release-dir=${path.join(dir, 'missing-release')}`]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Phoenix Diagram.*未修改项目/s);
  assert.deepEqual(fs.readdirSync(dir), []);
});

test('skipping both tools preserves diagram files and still installs XGENT', t => {
  const dir = project(t);
  const owned = path.join(dir, '.agents/skills/phoenix-diagram/SKILL.md');
  fs.mkdirSync(path.dirname(owned), { recursive: true });
  fs.writeFileSync(owned, 'preserve installed diagram\n');
  const result = install(dir, ['--no-phoenix-ui', '--no-phoenix-diagram']);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /完成: /);
  assert.equal(fs.readFileSync(owned, 'utf8'), 'preserve installed diagram\n');
  assert.ok(fs.existsSync(path.join(dir, '.claude/hooks/xgent-statusline.js')));
});

test('provider names are checked even when both tools are skipped', t => {
  const dir = project(t);
  const result = install(dir, ['--no-phoenix-ui', '--no-phoenix-diagram', '--providers=made-up-provider']);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /未知的 harness 目录/);
  assert.deepEqual(fs.readdirSync(dir), []);
});
