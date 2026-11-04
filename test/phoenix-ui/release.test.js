'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const checkout = path.resolve(__dirname, '../..');
const guard = path.join(checkout, 'scripts/require-release-package.mjs');
const release = path.join(checkout, 'scripts/release-phoenix-ui.mjs');

const runGuard = root => spawnSync(process.execPath, [guard, root], { encoding: 'utf8' });
const sha256 = bytes => require('node:crypto').createHash('sha256').update(bytes).digest('hex');

test('publishing from a checkout is refused while its Phoenix manifest is a development build', () => {
  const result = runGuard(checkout);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /npm run release:phoenix/);
  const pkg = JSON.parse(fs.readFileSync(path.join(checkout, 'package.json'), 'utf8'));
  assert.equal(pkg.scripts.prepublishOnly, 'node scripts/require-release-package.mjs');
  assert.equal(pkg.scripts['release:phoenix'], 'node scripts/release-phoenix-ui.mjs');
});

test('engine objects live under the CI run so a redo never meets an earlier attempt', async () => {
  const { engineKey } = await import(release);
  assert.equal(engineKey('vendor', 'phoenix-diagram', '0.7.9', '41410951647', 'linux-x64', 'phoenix-diagram-linux-x64'),
    'vendor/phoenix-diagram/v0.7.9/ci-41410951647/linux-x64/phoenix-diagram-linux-x64');
  assert.equal(engineKey('', 'phoenix-ui', '1.0.0', '7', 'darwin-arm64', 'phoenix-ui-darwin-arm64'), 'phoenix-ui/v1.0.0/ci-7/darwin-arm64/phoenix-ui-darwin-arm64');
});

test('an engine upload never replaces an object and accepts an existing one only with identical bytes', { skip: process.platform === 'win32' }, async t => {
  const { uploadEngine } = await import(release);
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix upload '));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  // Fake aws: a conditional put of an existing key fails like R2; get-object copies the stored bytes.
  const bin = path.join(dir, 'bin'), store = path.join(dir, 'store');
  fs.mkdirSync(bin); fs.mkdirSync(store);
  fs.writeFileSync(path.join(bin, 'aws'), `#!${process.execPath}
const fs = require('node:fs'), path = require('node:path'), args = process.argv.slice(2), get = flag => args[args.indexOf(flag) + 1];
const stored = path.join(${JSON.stringify(store)}, encodeURIComponent(get('--key')));
if (args[1] === 'put-object') {
  if (get('--if-none-match') !== '*') process.exit(2);
  if (fs.existsSync(stored)) { console.error('An error occurred (PreconditionFailed) when calling the PutObject operation'); process.exit(254); }
  fs.copyFileSync(get('--body'), stored);
} else if (args[1] === 'get-object') fs.copyFileSync(stored, args.at(-1));
else process.exit(2);
`, { mode: 0o755 });
  const config = { bucket: 'b', endpoint: 'https://r2.example', awsEnv: { ...process.env, PATH: `${bin}${path.delimiter}${process.env.PATH}` } };
  const file = path.join(dir, 'engine'), bytes = Buffer.from('engine bytes');
  fs.writeFileSync(file, bytes);
  const engine = { size: bytes.length, sha256: sha256(bytes) };
  assert.equal(uploadEngine(file, engine, 'k/engine', config, dir), 'uploaded');
  assert.equal(uploadEngine(file, engine, 'k/engine', config, dir), 'already uploaded');
  fs.writeFileSync(path.join(store, encodeURIComponent('k/other')), 'other bytes');
  assert.throws(() => uploadEngine(file, engine, 'k/other', config, dir), /already exists with different bytes/);
  assert.equal(fs.readFileSync(path.join(store, encodeURIComponent('k/other')), 'utf8'), 'other bytes');
});

test('a release manifest keeps the built engine fields and takes package identity and bundle digest from this checkout', async () => {
  const { releaseManifest } = await import(release);
  const engines = { 'linux-x64': { asset: 'phoenix-ui-linux-x64', size: 1, sha256: 'a'.repeat(64), url: 'https://download.example/x' } };
  const development = { schemaVersion: 1, distribution: 'development', toolVersion: '0.1.0', npmPackageVersion: '0.0.1', sourceCommit: 'old', sourceInputSha256: 'b'.repeat(64), bundleSha256: 'old', engines: {} };
  const identity = { version: '1.2.3', head: 'c'.repeat(40), tree: 'd'.repeat(40) };
  const ui = releaseManifest('phoenix-ui', development, engines, identity);
  assert.deepEqual(ui, { ...development, distribution: 'release', npmPackageVersion: '1.2.3', sourceCommit: identity.head, sourceTree: identity.tree,
    bundleSha256: sha256(fs.readFileSync(path.join(checkout, 'dist/phoenix-ui/bundle/manifest.json'))), engines });
  const diagram = releaseManifest('phoenix-diagram', development, engines, identity);
  assert.equal(diagram.sourceTree, undefined);
  assert.equal(diagram.bundleSha256, sha256(fs.readFileSync(path.join(checkout, 'dist/phoenix-diagram/bundle/manifest.json'))));
});

test('release credentials come only from AGENT_RELEASE_R2_* lines and exported values win', async () => {
  const { parseEnvFile, releaseEnv } = await import(release);
  assert.deepEqual(parseEnvFile([
    'export AGENT_RELEASE_R2_BUCKET="bucket"', "AGENT_RELEASE_R2_PREFIX='vendor'", '# AGENT_RELEASE_R2_SECRET_KEY=commented',
    'export OTHER_SECRET=ignored', '  export AGENT_RELEASE_R2_ENDPOINT=https://r2.example  ', '',
  ].join('\n')), { AGENT_RELEASE_R2_BUCKET: 'bucket', AGENT_RELEASE_R2_PREFIX: 'vendor', AGENT_RELEASE_R2_ENDPOINT: 'https://r2.example' });
  const env = releaseEnv({ AGENT_RELEASE_R2_BUCKET: 'exported', PATH: '/bin' }, 'export AGENT_RELEASE_R2_BUCKET=file\nexport AGENT_RELEASE_R2_PREFIX=p\nexport OTHER=x');
  assert.equal(env.AGENT_RELEASE_R2_BUCKET, 'exported');
  assert.equal(env.AGENT_RELEASE_R2_PREFIX, 'p');
  assert.equal(env.OTHER, undefined);
  assert.equal(env.PATH, '/bin');
});

test('the release uses the newest successful CI run of exactly this commit', async () => {
  const { selectRun } = await import(release);
  const head = 'a'.repeat(40), other = 'b'.repeat(40);
  const runs = [
    { databaseId: 9, headSha: other, status: 'completed', conclusion: 'success' },
    { databaseId: 5, headSha: head, status: 'completed', conclusion: 'success' },
    { databaseId: 7, headSha: head, status: 'completed', conclusion: 'success' },
    { databaseId: 6, headSha: head, status: 'completed', conclusion: 'cancelled' },
  ];
  assert.equal(selectRun(runs, head).success.databaseId, 7);
  const pending = selectRun([{ databaseId: 8, headSha: head, status: 'in_progress', conclusion: null }], head);
  assert.equal(pending.success, undefined); assert.equal(pending.pending.databaseId, 8);
  const failed = selectRun([{ databaseId: 4, headSha: head, status: 'completed', conclusion: 'failure' }], head);
  assert.equal(failed.success, undefined); assert.equal(failed.failed.databaseId, 4);
  assert.deepEqual(selectRun(runs, 'c'.repeat(40)), { success: undefined, pending: undefined, failed: undefined });
});

test('engines are reused only while no source compiled into either engine changed', async t => {
  const { assertEnginesReusable } = await import(release);
  const { sourceInputs } = await import(path.join(checkout, 'scripts/build-phoenix-diagram.mjs'));
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix reuse '));
  t.after(() => fs.rmSync(repo, { recursive: true, force: true }));
  const git = (...args) => {
    const result = spawnSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@t', ...args], { cwd: repo, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr); return result.stdout.trim();
  };
  const write = (name, text) => { fs.mkdirSync(path.dirname(path.join(repo, name)), { recursive: true }); fs.writeFileSync(path.join(repo, name), text); };
  const commit = (name, text) => { write(name, text); git('add', '-A'); git('commit', '-qm', name); };
  write('tools/phoenix-ui/crates/live/src/lib.rs', 'const JS: &str = include_str!("../../../../../skills/phoenix-ui/src/scripts/live.js");\n');
  write('tools/phoenix-ui/scripts/render.mjs', '1');
  write('skills/phoenix-ui/src/scripts/live.js', '1');
  write('skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs', '1');
  write('tools/phoenix-diagram/build.rs', 'const INPUTS: &[&str] = &[\n    "tools/phoenix-diagram/src",\n];\n');
  write('tools/phoenix-diagram/src/main.rs', '1');
  git('init', '-q'); git('add', '-A'); git('commit', '-qm', 'released');
  const previous = { version: '1.0.0', ui: { sourceCommit: git('rev-parse', 'HEAD') }, diagram: { sourceInputSha256: sourceInputs(repo).sha256 } };

  commit('skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs', '2');
  commit('tools/phoenix-ui/scripts/render.mjs', '2');
  commit('package.json', '{"version":"1.0.1"}');
  assert.doesNotThrow(() => assertEnginesReusable(previous, repo));

  commit('skills/phoenix-ui/src/scripts/live.js', '2');
  assert.throws(() => assertEnginesReusable(previous, repo), /UI engine sources changed[\s\S]*live\.js/);
  git('reset', '-q', '--hard', 'HEAD~1');
  commit('tools/phoenix-diagram/src/main.rs', '2');
  assert.throws(() => assertEnginesReusable(previous, repo), /Diagram engine sources changed/);
});

test('the npm tarball includes Diagram and install enables it unless explicitly skipped', async t => {
  const pkg = JSON.parse(fs.readFileSync(path.join(checkout, 'package.json')));
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix joint package '));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const stage = path.join(temp, 'stage');
  fs.mkdirSync(stage);
  fs.writeFileSync(path.join(stage, 'package.json'), JSON.stringify(pkg));
  for (const name of ['bin', '.claude/hooks', 'agents', 'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs', 'dist/phoenix-ui/bundle/manifest.json', 'skills/phoenix-diagram', 'dist/phoenix-diagram']) {
    fs.cpSync(path.join(checkout, name), path.join(stage, name), { recursive: true });
  }
  const packed = spawnSync('npm', ['pack', '--offline', '--ignore-scripts', '--json', '--pack-destination', temp], { cwd: stage, encoding: 'utf8' });
  assert.equal(packed.status, 0, packed.stderr);
  const [record] = JSON.parse(packed.stdout);
  assert.ok(record.files.some(item => item.path === 'bin/xgent-skills.js'));
  for (const file of ['bin/phoenix-diagram.js', 'skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs', 'dist/phoenix-diagram/bundle/manifest.json']) {
    assert.ok(record.files.some(item => item.path === file), file);
  }
  const extracted = spawnSync('tar', ['-xzf', path.join(temp, record.filename), '-C', temp], { encoding: 'utf8' });
  assert.equal(extracted.status, 0, extracted.stderr);
  const cli = path.join(temp, 'package/bin/xgent-skills.js');
  const help = spawnSync(process.execPath, [cli, 'help'], { encoding: 'utf8' });
  assert.equal(help.status, 0, help.stderr);
  assert.match(help.stdout, /Phoenix Diagram/);
  assert.match(help.stdout, /--no-phoenix-diagram/);
  const project = path.join(temp, 'project'); fs.mkdirSync(project);
  const missing = spawnSync(process.execPath, [cli, 'install', project, '--no-phoenix-ui', `--diagram-release-dir=${path.join(temp, 'missing')}`], { encoding: 'utf8' });
  assert.equal(missing.status, 4);
  assert.match(missing.stderr, /Phoenix Diagram/);
  assert.deepEqual(fs.readdirSync(project), []);
  const installed = spawnSync(process.execPath, [cli, 'install', project, '--no-phoenix-ui', '--no-phoenix-diagram'], { encoding: 'utf8' });
  assert.equal(installed.status, 0, installed.stderr);
  assert.ok(fs.existsSync(path.join(project, '.claude/hooks/xgent-statusline.js')));
  assert.equal(fs.existsSync(path.join(project, '.phoenix-diagram')), false);
});
