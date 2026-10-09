'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { spawn } = require('node:child_process');
const source = path.resolve(__dirname, '../../skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs');
const { sha256, platformTarget, proxyFor } = require(source);

function fixture(t, changes = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix bootstrap space '));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const scripts = path.join(root, 'skill/scripts');
  const cache = path.join(root, 'cache');
  const release = path.join(root, 'release');
  const project = path.join(root, 'project');
  for (const dir of [scripts, release, path.join(project, '.git')]) fs.mkdirSync(dir, { recursive: true });
  const program = `
const fs = require('node:fs'), path = require('node:path');
if (path.basename(process.execPath).startsWith('phoenix-ui') || require.main === module) {
  if (process.argv[1] === 'engine-probe' || process.argv[2] === 'engine-probe') {
    console.log('phoenix-ui-engine 0.1.0'); process.exit(0);
  }
  if (process.env.PHOENIX_UI_FIXTURE_TRACE) fs.appendFileSync(process.env.PHOENIX_UI_FIXTURE_TRACE, 'executed\\n');
  console.log(JSON.stringify({argv: process.argv.slice(path.basename(process.execPath).startsWith('phoenix-ui') ? 1 : 2), provider:process.env.PHOENIX_UI_PROVIDER_ID}));
  process.exit(0);
}
`;
  const preload = path.join(root, 'fixture-preload.cjs');
  fs.writeFileSync(preload, program);
  // Windows needs a real PE executable. This is a Node protocol fixture,
  // not evidence that the Phoenix native build works on Windows.
  const bytes = process.platform === 'win32' ? fs.readFileSync(process.execPath) : Buffer.from(`#!/usr/bin/env node\n${program}`);
  const target = platformTarget();
  const asset = `phoenix-ui-${target}${target.startsWith('windows-') ? '.exe' : ''}`;
  const manifest = {
    schemaVersion: 1, distribution: 'development', toolVersion: '0.1.0', npmPackageVersion: '0.7.0-rc.0',
    sourceCommit: 'a'.repeat(40), bundleSchema: 1, bundleSha256: 'b'.repeat(64), reviewSchema: 1,
    engines: { [target]: { asset, sha256: sha256(bytes), size: bytes.length } }, ...changes,
  };
  fs.copyFileSync(source, path.join(scripts, 'phoenix-bootstrap.cjs'));
  const manifestBytes = Buffer.from(JSON.stringify(manifest, null, 2) + '\n');
  for (const file of [path.join(scripts, 'ENGINE.json'), path.join(release, 'VERSION.json')]) fs.writeFileSync(file, manifestBytes);
  fs.writeFileSync(path.join(release, asset), bytes, { mode: 0o700 });
  fs.writeFileSync(path.join(release, 'THIRD-PARTY.json'), '{}\n');
  fs.writeFileSync(path.join(release, 'THIRD-PARTY-NOTICES.md'), 'Fixture sidecar contract only.\n');
  fs.mkdirSync(path.join(release, 'THIRD-PARTY-LICENSES'));
  const env = { ...process.env, PHOENIX_UI_HOME: cache, PHOENIX_UI_PROVIDER_ID: 'codex',
    PHOENIX_UI_FIXTURE_TRACE: path.join(root, 'executed.log'), HTTPS_PROXY: '', https_proxy: '', HTTP_PROXY: '', http_proxy: '', ALL_PROXY: '', all_proxy: '' };
  if (process.platform === 'win32') env.NODE_OPTIONS = `--require="${preload.split(path.sep).join('/')}"`;
  return { root, scripts, cache, release, project, bytes, target, asset, manifest, manifestBytes, env };
}

function run(f, args, input = '') {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [path.join(f.scripts, 'phoenix-bootstrap.cjs'), ...args], { cwd: f.project, env: f.env, stdio: ['pipe', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    child.stdout.on('data', data => { stdout += data; });
    child.stderr.on('data', data => { stderr += data; });
    child.once('error', reject);
    child.once('close', code => resolve({ code, stdout, stderr }));
    child.stdin.end(input);
  });
}

test('ordinary CLI missing engine returns unavailable 4 without creating a cache', async t => {
  const f = fixture(t);
  const result = await run(f, ['context', '--json']);
  assert.equal(result.code, 4);
  const json = JSON.parse(result.stdout);
  assert.equal(json.status, 'unavailable');
  assert.match(json.setupCommand, /@xgent-ai\/skills@0\.7\.0-rc\.0/);
  assert(json.setupCommand.includes(fs.realpathSync(f.project)));
  assert.equal(fs.existsSync(f.cache), false);
});

test('offline explicit install verifies and runs the pinned executable with spaces in paths', async t => {
  const f = fixture(t);
  const install = await run(f, ['engine', 'install', '--project', f.project, '--release-dir', f.release, '--json']);
  assert.equal(install.code, 0, install.stderr);
  assert.equal(JSON.parse(install.stdout).status, 'installed');
  const result = await run(f, ['context', '--target', 'a b.tsx']);
  assert.equal(result.code, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout).argv, ['context', '--target', 'a b.tsx']);
  assert.equal(fs.existsSync(path.join(f.project, '.phoenix-ui')), false, 'engine-only install must not write a project receipt');
});

test('corrupt and mismatched binary fails before executing ordinary verbs', async t => {
  const f = fixture(t);
  const dest = path.join(f.cache, 'bin/0.1.0', f.target, f.target.startsWith('windows') ? 'phoenix-ui.exe' : 'phoenix-ui');
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, Buffer.concat([f.bytes, Buffer.from('tampered')]), { mode: 0o700 });
  const result = await run(f, ['detect', '--json']);
  assert.equal(result.code, 4);
  assert.equal(fs.existsSync(f.env.PHOENIX_UI_FIXTURE_TRACE), false);
  assert.match(result.stderr, /digest or size/);
});

test('missing-engine hooks fail open once per provider session and install resets the reminder', async t => {
  const f = fixture(t);
  const event = JSON.stringify({ cwd: f.project, session_id: 's1', hook_event_name: 'PostToolUse' });
  const first = await run(f, ['hook'], event);
  assert.equal(first.code, 0);
  assert.match(JSON.parse(first.stdout).hookSpecificOutput.additionalContext, /engine install/);
  const second = await run(f, ['hook'], event);
  assert.equal(second.code, 0);
  assert.equal(second.stdout, '');
  const separate = await run(f, ['hook'], JSON.stringify({ cwd: f.project, session_id: 's2', hook_event_name: 'Stop' }));
  assert.equal(separate.code, 0);
  assert.equal(separate.stdout, '');
  assert.match(separate.stderr, /Phoenix UI unavailable/);
  const installed = await run(f, ['engine', 'install', '--release-dir', f.release]);
  assert.equal(installed.code, 0, installed.stderr);
  assert.equal(fs.readdirSync(path.join(f.cache, 'unavailable')).length, 0);
});

test('a hook with a missing project directory still fails open and deduplicates its reminder', async t => {
  const f = fixture(t);
  const event = JSON.stringify({ cwd: path.join(f.project, 'missing'), session_id: 'missing-cwd' });
  const first = await run(f, ['hook'], event);
  assert.equal(first.code, 0);
  assert.match(JSON.parse(first.stdout).hookSpecificOutput.additionalContext, /Phoenix UI unavailable/);
  const second = await run(f, ['hook'], event);
  assert.equal(second.code, 0);
  assert.equal(second.stdout, '');
  const records = fs.readdirSync(path.join(f.cache, 'unavailable')).map(name =>
    JSON.parse(fs.readFileSync(path.join(f.cache, 'unavailable', name))));
  assert.equal(records[0].version, '0.1.0');
});

test('malformed event types remain nonblocking when the cached engine is absent', async t => {
  for (const input of ['null', '[]', '17', '"text"', '{"hook_event_name":42}']) {
    const f = fixture(t);
    const result = await run(f, ['hook'], input);
    assert.equal(result.code, 0, `${input}: ${result.stderr}`);
  }
});

test('offline wrong manifest or missing license refuses cache and project writes', async t => {
  const f = fixture(t);
  fs.writeFileSync(path.join(f.release, 'VERSION.json'), JSON.stringify({ ...f.manifest, sourceCommit: 'c'.repeat(40) }));
  const mismatch = await run(f, ['engine', 'install', '--release-dir', f.release, '--json']);
  assert.equal(mismatch.code, 4);
  assert.equal(fs.existsSync(f.cache), false);
  fs.writeFileSync(path.join(f.release, 'VERSION.json'), f.manifestBytes);
  fs.unlinkSync(path.join(f.release, 'THIRD-PARTY-NOTICES.md'));
  const missing = await run(f, ['engine', 'install', '--release-dir', f.release]);
  assert.equal(missing.code, 4);
  assert.equal(fs.existsSync(f.cache), false);
});

test('receipt mismatch rejects engine-only installation before touching the cache', async t => {
  const f = fixture(t);
  const install = path.join(f.project, '.phoenix-ui/install');
  fs.mkdirSync(install, { recursive: true });
  fs.writeFileSync(path.join(install, 'receipt.json'), JSON.stringify({ schemaVersion: 1, toolVersion: '0.1.0', npmPackageVersion: '0.6.0', manifestSha256: '0'.repeat(64), engine: { target: f.target, sha256: sha256(f.bytes) } }));
  const before = fs.readFileSync(path.join(install, 'receipt.json'));
  const result = await run(f, ['engine', 'install', '--project', f.project, '--release-dir', f.release]);
  assert.equal(result.code, 4);
  assert.equal(fs.existsSync(f.cache), false);
  assert.deepEqual(fs.readFileSync(path.join(install, 'receipt.json')), before);
});

test('explicit HTTP substitute verifies the package digest; normal runs make zero requests', async t => {
  const f = fixture(t);
  let requests = 0;
  let corrupt = true;
  const server = http.createServer((req, res) => {
    requests++;
    res.end(corrupt ? Buffer.alloc(f.bytes.length, 0) : f.bytes);
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise(resolve => server.close(resolve)));
  f.manifest.engines[f.target].url = `http://127.0.0.1:${server.address().port}/fixed/engine`;
  fs.writeFileSync(path.join(f.scripts, 'ENGINE.json'), JSON.stringify(f.manifest));
  assert.equal((await run(f, ['context'])).code, 4);
  assert.equal(requests, 0);
  const refused = await run(f, ['engine', 'install']);
  assert.equal(refused.code, 4);
  assert.equal(fs.existsSync(f.cache), false);
  corrupt = false;
  const accepted = await run(f, ['engine', 'install']);
  assert.equal(accepted.code, 0, accepted.stderr);
  assert.equal(requests, 2);
  assert.equal((await run(f, ['context'])).code, 0);
  assert.equal(requests, 2);
});

test('platform fallback, proxy precedence and no_proxy keep the declared consumer contract', () => {
  assert.equal(platformTarget('win32', 'arm64'), 'windows-x64');
  assert.equal(platformTarget('linux', 'arm64'), 'linux-arm64');
  assert.throws(() => platformTarget('freebsd', 'x64'), /Unsupported/);
  const target = new URL('https://download.example.test/a');
  assert.equal(proxyFor(target, { HTTP_PROXY: 'http://127.0.0.1:3128' }).href, 'http://127.0.0.1:3128/');
  assert.equal(proxyFor(target, { HTTP_PROXY: 'http://127.0.0.1:3128', HTTPS_PROXY: 'http://127.0.0.1:3129' }).port, '3129');
  assert.equal(proxyFor(target, { HTTPS_PROXY: 'http://127.0.0.1:3129', NO_PROXY: '.example.test' }), null);
});
