'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const root = path.resolve(__dirname, '../..');
const moduleReady = import('../../scripts/test-phoenix-diagram-install.mjs');
function temporary(t) { const directory = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'diagram install stage ')); t.after(() => fs.rmSync(directory, { recursive: true, force: true })); return directory; }
function output(records) { return records.map(value => JSON.stringify(value)).join('\n') + '\n'; }
function records(executable, workspace = path.join(root, 'tools/phoenix-ui')) {
  return [{ reason: 'compiler-artifact', package_id: `path+file://${workspace}/crates/cli#phoenix-ui@0.1.0`, manifest_path: path.join(workspace, 'crates/cli/Cargo.toml'),
    target: { name: 'phoenix-ui', kind: ['bin'] }, profile: { test: false }, executable }, { reason: 'build-finished', success: true }];
}

test('UI selection reads the exact compiler artifact instead of assuming a target path', async t => {
  const api = await moduleReady, executable = path.join(temporary(t), 'managed output/phoenix-ui');
  const messages = records(executable);
  messages.unshift({ ...messages[0], target: { name: 'phoenix-diagram', kind: ['bin'] } });
  assert.equal(api.selectUiCompilerArtifact(output(messages)), executable);
});

test('UI artifact selection refuses malformed, missing, ambiguous, incomplete and failed Cargo output', async t => {
  const api = await moduleReady, rows = records(path.join(temporary(t), 'phoenix-ui'));
  for (const input of ['', 'not JSON\n', output([]), output([rows[1]]), output([rows[0]]), output([rows[0], rows[0], rows[1]]), output([rows[0], { ...rows[1], success: false }])]) {
    assert.throws(() => api.selectUiCompilerArtifact(input), /Cargo|artifact|build|JSON/i);
  }
  for (const changed of [{ executable: null }, { executable: 'target/debug/phoenix-ui' }, { target: { name: 'phoenix-ui', kind: ['lib'] } },
    { target: { name: 'phoenix-diagram', kind: ['bin'] } }, { profile: { test: true } }, { manifest_path: path.join(root, 'tools/phoenix-diagram/Cargo.toml') }]) {
    assert.throws(() => api.selectUiCompilerArtifact(output([{ ...rows[0], ...changed }, rows[1]])), /artifact|executable|UI|absolute/i);
  }
});

test('missing, nonexecutable, symbolic or directory artifacts fail before creating a snapshot directory', async t => {
  const api = await moduleReady, directory = temporary(t), parent = path.join(directory, 'snapshots'); fs.mkdirSync(parent);
  const text = path.join(directory, 'not executable'); fs.writeFileSync(text, fs.readFileSync(path.join(root, 'LICENSE')), { mode: 0o600 });
  const invalid = ['relative', path.join(directory, 'missing'), text, directory];
  if (process.platform !== 'win32') { const alias = path.join(directory, 'alias'); fs.symlinkSync(process.execPath, alias); invalid.push(alias); }
  for (const file of invalid) {
    assert.throws(() => api.pinUiEngine(file, { parent }), /absolute|ordinary|executable|symbolic|ENOENT|ELOOP/i);
    assert.deepEqual(fs.readdirSync(parent), []);
  }
});

test('snapshot cleanup preserves replaced slots and unknown children instead of recursively deleting them', async t => {
  const api = await moduleReady, directory = temporary(t), source = path.join(directory, 'ownership-only fixture');
  // Benign file ownership fixture. It is never launched or passed to UI tests.
  fs.writeFileSync(source, fs.readFileSync(path.join(root, 'LICENSE')), { mode: 0o700 });
  const replaced = api.pinUiEngine(source, { parent: directory });
  const replacement = path.join(directory, 'replacement'); fs.writeFileSync(replacement, 'later claimant', { mode: 0o700 }); fs.renameSync(replacement, replaced.path);
  assert.throws(() => replaced.cleanup(), /binding|ownership|changed|retained/i);
  assert.equal(fs.readFileSync(replaced.path, 'utf8'), 'later claimant');
  const children = api.pinUiEngine(source, { parent: directory }), stranger = path.join(path.dirname(children.path), 'unknown'); fs.writeFileSync(stranger, 'preserve');
  assert.throws(() => children.cleanup(), /unknown|entries|retained|ENOTEMPTY/i); assert.equal(fs.readFileSync(stranger, 'utf8'), 'preserve');
  const moved = api.pinUiEngine(source, { parent: directory }), parent = path.dirname(moved.path); fs.renameSync(parent, parent + '-retained'); fs.mkdirSync(parent); fs.writeFileSync(path.join(parent, 'later'), 'preserve');
  assert.throws(() => moved.cleanup(), /binding|ownership|changed|retained/i); assert.equal(fs.readFileSync(path.join(parent, 'later'), 'utf8'), 'preserve');
});

test('child exit status is preserved and an absent status or startup error cannot pass', async () => {
  const api = await moduleReady;
  for (const status of [0, 1, 7, 101]) assert.equal(api.childExitCode({ status }), status);
  for (const result of [{ status: null, signal: 'SIGTERM' }, { status: null, error: new Error('spawn failed') }, {}]) assert.equal(api.childExitCode(result), 4);
});

test('install-stage help and invalid arguments do not load UI tooling, touch files or run maintenance commands', t => {
  const directory = temporary(t), script = path.join(directory, 'install.mjs'); fs.copyFileSync(path.join(root, 'scripts/test-phoenix-diagram-install.mjs'), script);
  const run = args => spawnSync(process.execPath, [script, ...args], { cwd: directory, env: { ...process.env, PATH: '' }, encoding: 'utf8' });
  const help = run(['--help']); assert.equal(help.status, 0, help.stderr); assert.match(help.stdout, /UI.*install|installation/i); assert.match(help.stdout, /prepared/i);
  for (const args of [['--prepare'], ['--engine', '/foreign'], ['--help', '--other']]) { const result = run(args); assert.equal(result.status, 2); assert.match(result.stderr, /Usage:/); }
  assert.deepEqual(fs.readdirSync(directory), ['install.mjs']);
});
