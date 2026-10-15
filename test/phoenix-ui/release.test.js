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

function packageRoot(t, manifest) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix release guard '));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.mkdirSync(path.join(root, 'dist/phoenix-ui'), { recursive: true });
  fs.writeFileSync(path.join(root, 'dist/phoenix-ui/VERSION.json'), JSON.stringify(manifest));
  return root;
}
const runGuard = root => spawnSync(process.execPath, [guard, root], { encoding: 'utf8' });
const engine = url => ({ asset: 'phoenix-ui-linux-x64', sha256: 'a'.repeat(64), size: 1, ...(url ? { url } : {}) });

test('publishing from a checkout is refused while its Phoenix manifest is a development build', () => {
  const result = runGuard(checkout);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /npm run release:phoenix/);
  const pkg = JSON.parse(fs.readFileSync(path.join(checkout, 'package.json'), 'utf8'));
  assert.equal(pkg.scripts.prepublishOnly, 'node scripts/require-release-package.mjs');
  assert.equal(pkg.scripts['release:phoenix'], 'node scripts/release-phoenix-ui.mjs');
});

test('the publish guard accepts only a release manifest whose engines all have public HTTPS URLs', t => {
  assert.equal(runGuard(packageRoot(t, { distribution: 'development', engines: { 'linux-x64': engine() } })).status, 1);
  assert.equal(runGuard(packageRoot(t, { distribution: 'release', engines: {} })).status, 1);
  assert.equal(runGuard(packageRoot(t, { distribution: 'release', engines: { 'linux-x64': engine('http://127.0.0.1:18800/x') } })).status, 1);
  assert.equal(runGuard(packageRoot(t, { distribution: 'release', engines: { 'linux-x64': engine('https://download.example/x'), 'linux-arm64': engine() } })).status, 1);
  assert.equal(runGuard(packageRoot(t, { distribution: 'release', engines: { 'linux-x64': engine('https://download.example/x') } })).status, 0);
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
