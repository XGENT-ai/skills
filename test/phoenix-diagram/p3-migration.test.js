'use strict';
// Complete inert migration data, never a historical executable or native pass.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { createHash } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const { pathToFileURL } = require('node:url');
const { gunzipSync, gzipSync } = require('node:zlib');
const helper = '../../tools/phoenix-diagram/tests/support/p3-migration.mjs';
const producer = '../../scripts/collect-phoenix-diagram-migration-sources.mjs';
const root = path.resolve(__dirname, '../..');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const manifestPath = 'tools/phoenix-diagram/migration-sources/archify-v1.json';
function inputs() { const bytes = fs.readFileSync(path.join(root, manifestPath)); return [[manifestPath, bytes.length, sha(bytes)]]; }
function temp(t) { const directory = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-inert-migration-'))); fs.chmodSync(directory, 0o700); t.after(() => fs.rmSync(directory, { recursive: true, force: true })); return directory; }

test('fixed complete migration fixture reproduces canonical gzip and candidate manifest bytes', async () => {
  const { migrationFixtureBytes } = await import(producer), api = await import(helper);
  const first = migrationFixtureBytes(), second = migrationFixtureBytes();
  assert(first.equals(second)); assert.equal(first.readUInt32LE(4), 0);
  assert(first.equals(fs.readFileSync(path.join(root, api.MIGRATION_FIXTURE_MEMBER))));
  assert.deepEqual({ bytes: first.length, sha256: sha(first) }, api.MIGRATION_FIXTURE_IDENTITY);
  const fixture = api.readMigrationFixture(first, { sourceInputs: inputs() });
  assert.deepEqual(fixture.sources.map(source => [source.id, source.files.length]), [['legacy-2.17', 219], ['legacy-2.17-when-to-use', 219], ['reference-3.1.0', 157]]);
  assert(Buffer.from(fixture.manifest.base64, 'base64').equals(fs.readFileSync(path.join(root, manifestPath))));
  for (const source of fixture.sources) for (const file of source.files) { const bytes = Buffer.from(file.base64, 'base64'); assert.equal(bytes.length, file.bytes); assert.equal(sha(bytes), file.sha256); assert([0o644, 0o755].includes(file.mode)); }
});

test('fixed migration archives and inert fixture retain their bytes with Linux or unknown gzip OS headers', () => {
  for (const operatingSystem of [3, 255]) {
    const script = `const zlib = require('node:zlib'), gzip = zlib.gzipSync;
      zlib.gzipSync = (...args) => { const bytes = gzip(...args); bytes[9] = ${operatingSystem}; return bytes; };
      require('node:module').syncBuiltinESMExports();
      import(${JSON.stringify(pathToFileURL(path.resolve(__dirname, producer)).href)})
        .then(api => console.log(JSON.stringify(api.collectMigrationFixture())));`;
    const result = spawnSync(process.execPath, ['-e', script], { encoding: 'utf8', timeout: 30000 });
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.equal(JSON.parse(result.stdout).status, 'pass');
  }
});

test('consumer rejects changed fixture bytes, another candidate manifest, duplicate or missing binding', async () => {
  const api = await import(helper), bytes = fs.readFileSync(path.join(root, api.MIGRATION_FIXTURE_MEMBER));
  const changed = Buffer.from(bytes); changed[changed.length - 20] ^= 1;
  for (const candidate of [changed, bytes.subarray(1), gzipSync(Buffer.from('{}\n'))]) assert.throws(() => api.readMigrationFixture(candidate, { sourceInputs: inputs() }));
  for (const sourceInputs of [[], [[manifestPath, 1, 'a'.repeat(64)]], [...inputs(), ...inputs()], { files: inputs() }]) assert.throws(() => api.readMigrationFixture(bytes, { sourceInputs }));
  const payload = JSON.parse(gunzipSync(bytes)); payload.sources[0].files[0].path = '../escape';
  assert.throws(() => api.readMigrationFixture(gzipSync(Buffer.from(JSON.stringify(payload) + '\n')), { sourceInputs: inputs() }));
});

test('materializer writes all three exact sets only into fresh private projects without executing old code', async t => {
  const api = await import(helper), fixture = api.readMigrationFixture(fs.readFileSync(path.join(root, api.MIGRATION_FIXTURE_MEMBER)), { sourceInputs: inputs() }), parent = temp(t);
  for (const source of fixture.sources) {
    const project = path.join(parent, source.id); fs.mkdirSync(project, { mode: 0o700 });
    const result = api.materializeKnownSource(fixture, source.id, project);
    assert.equal(result.project, project); assert.equal(result.root, path.join(project, '.agents/skills/archify'));
    const names = fs.readdirSync(result.root, { recursive: true }).filter(name => fs.lstatSync(path.join(result.root, name)).isFile()).sort();
    assert.deepEqual(names, source.files.map(file => file.path));
    for (const file of source.files) { const actual = path.join(result.root, file.path), stat = fs.lstatSync(actual); assert(!stat.isSymbolicLink()); assert.equal(stat.nlink, 1); assert.equal(stat.mode & 0o7777, file.mode); assert.equal(sha(fs.readFileSync(actual)), file.sha256); }
    assert.equal(fs.existsSync(path.join(project, '.phoenix-diagram')), false); assert.throws(() => api.materializeKnownSource(fixture, source.id, project));
  }
});

test('materializer refuses links, existing contents, nonprivate roots and unvalidated data without touching claimants', async t => {
  const api = await import(helper), fixture = api.readMigrationFixture(fs.readFileSync(path.join(root, api.MIGRATION_FIXTURE_MEMBER)), { sourceInputs: inputs() }), parent = temp(t);
  const fresh = name => { const project = path.join(parent, name); fs.mkdirSync(project, { mode: 0o700 }); return project; };
  const nonempty = fresh('nonempty'); fs.writeFileSync(path.join(nonempty, 'business.txt'), 'preserved'); assert.throws(() => api.materializeKnownSource(fixture, 'legacy-2.17', nonempty)); assert.equal(fs.readFileSync(path.join(nonempty, 'business.txt'), 'utf8'), 'preserved');
  const privateRoot = fresh('private'), alias = path.join(parent, 'alias'); fs.symlinkSync(privateRoot, alias); assert.throws(() => api.materializeKnownSource(fixture, 'legacy-2.17', alias));
  const broad = fresh('broad'); fs.chmodSync(broad, 0o755); assert.throws(() => api.materializeKnownSource(fixture, 'legacy-2.17', broad));
  const nested = fresh('nested'); fs.symlinkSync(privateRoot, path.join(parent, 'parent-alias')); assert.throws(() => api.materializeKnownSource(fixture, 'legacy-2.17', path.join(parent, 'parent-alias', 'new')));
  assert.throws(() => api.materializeKnownSource(structuredClone(fixture), 'legacy-2.17', nested)); assert.deepEqual(fs.readdirSync(nested), []);
  assert.throws(() => api.materializeKnownSource(fixture, 'unknown', nested)); assert.throws(() => api.materializeKnownSource(fixture, 'legacy-2.17', nested, { provider: '../foreign' })); assert.deepEqual(fs.readdirSync(nested), []);
  assert.throws(() => { fixture.sources[0].files[0].path = '../changed'; });
});

test('assembly verifier reads complete fixed archives from an explicit workspace and rejects changed bytes', async t => {
  const { verifyMigrationSourceFiles } = await import(producer), project = temp(t);
  for (const name of ['migration-sources/archify-v1.json', 'upstream/archify-2.17-full-source.tar.gz', 'upstream/archify-3.1.0-source.tar.gz']) {
    const target = path.join(project, name); fs.mkdirSync(path.dirname(target), { recursive: true }); fs.copyFileSync(path.join(root, 'tools/phoenix-diagram', name), target);
  }
  const result = verifyMigrationSourceFiles({ project }); assert.equal(result.status, 'pass'); assert.deepEqual(result.sources.map(source => source.fileCount), [219, 219, 157]);
  const archive = path.join(project, 'upstream/archify-2.17-full-source.tar.gz'); const bytes = fs.readFileSync(archive);
  const changedHeader = Buffer.from(bytes); changedHeader[9] = 3; fs.writeFileSync(archive, changedHeader);
  assert.throws(() => verifyMigrationSourceFiles({ project }));
  bytes[bytes.length - 8] ^= 1; fs.writeFileSync(archive, bytes);
  assert.throws(() => verifyMigrationSourceFiles({ project }));
});
