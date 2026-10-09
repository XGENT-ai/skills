const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { gzipSync } = require('node:zlib');
const { test } = require('node:test');
const { spawnSync } = require('node:child_process');

const checker = import(pathToFileURL(path.resolve(__dirname, '../../scripts/check-phoenix-deps.mjs')));
const now = new Date('2026-10-08T12:00:00Z');
const packages = [{ key: 'example', name: 'example', version: '1.2.3', integrity: 'sha512-AA==' }];

function tar(entries) {
  const chunks = [];
  for (const [name, value, type = '0'] of entries) {
    const bytes = Buffer.from(value);
    const header = Buffer.alloc(512);
    header.write(name); header.write(bytes.length.toString(8).padStart(11, '0'), 124);
    header.write(type, 156); header.write('ustar', 257);
    chunks.push(header, bytes, Buffer.alloc((512 - bytes.length % 512) % 512));
  }
  return gzipSync(Buffer.concat([...chunks, Buffer.alloc(1024)]));
}

async function snapshot(overrides = {}) {
  const { sha256, auditRequest } = await checker;
  const versions = JSON.parse(fs.readFileSync(path.resolve(__dirname, '../../tools/phoenix-ui/build-tools.lock.json')));
  return { queriedAt: '2026-10-08T11:00:00Z', bunVersion: versions.bun, bunLockSha256: 'lock-hash',
    endpoint: 'https://registry.npmjs.org/-/npm/v1/security/advisories/bulk', request: auditRequest(packages),
    response: {}, responseSha256: sha256('{}'), bunResponse: {}, bunResponseSha256: sha256('{}'), bunExitCode: 0,
    skippedRegistries: false, ...overrides };
}

test('Bun parser reads actual locked dependencies and preserves commas inside strings', async () => {
  const { parseBunLock } = await checker;
  const actual = parseBunLock(fs.readFileSync(path.resolve(__dirname, '../../tools/phoenix-ui/bun.lock'), 'utf8'));
  assert.ok(actual.length > 40);
  assert.ok(actual.some(p => p.name === '@babel/parser'));
  assert.ok(actual.some(p => p.name === 'fsevents'), 'optional platform dependency remains in the inventory');
  const input = '{"lockfileVersion":1,"packages":{"@scope/a":["@scope/a@1.2.3","",{"note":",}"},"sha512-AA==",],},}';
  assert.deepEqual(parseBunLock(input), [{ key: '@scope/a', name: '@scope/a', version: '1.2.3', integrity: 'sha512-AA==' }]);
  for (const value of ['file:../local', 'git+https://example.com/repo', '^1.2.3']) {
    assert.throws(() => parseBunLock(JSON.stringify({ lockfileVersion: 1, packages: { a: [value, '', {}, 'sha512-AA=='] } })), /Unpinned/);
  }
  assert.throws(() => parseBunLock('{"lockfileVersion":2,"packages":{}}'), /Unsupported/);
});

test('Cargo inventory includes duplicate crate names, workspace packages and checksums', async () => {
  const { parseCargoLock } = await checker;
  const actual = parseCargoLock(fs.readFileSync(path.resolve(__dirname, '../../tools/phoenix-ui/Cargo.lock'), 'utf8'));
  assert.ok(actual.length > 190);
  assert.equal(actual.filter(p => p.name === 'webpki-roots').length, 2);
  assert.ok(actual.some(p => !p.source));
  assert.ok(actual.filter(p => p.source).every(p => /^[a-f0-9]{64}$/.test(p.checksum)));
  assert.throws(() => parseCargoLock('[[package]]\nname = "unknown"\nversion = "1.0.0"\nsource = "registry+example"\n'), /Incomplete/);
});

test('material paths follow the single editable source after moving, while legacy imports remain readable', async t => {
  const { materialScripts } = await checker;
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-source-with-spaces-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const runtime = path.join(directory, 'tools/phoenix-ui');
  const original = path.join(runtime, 'skill/scripts');
  fs.mkdirSync(original, { recursive: true }); fs.writeFileSync(path.join(original, 'modern-screenshot.umd.js'), 'original bytes');
  assert.equal(materialScripts(directory, runtime), original);
  const source = path.join(directory, 'skills/phoenix-ui/src/scripts');
  fs.mkdirSync(path.dirname(source), { recursive: true }); fs.renameSync(original, source);
  assert.equal(materialScripts(directory, runtime), source);
  assert.equal(fs.readFileSync(path.join(materialScripts(directory, runtime), 'modern-screenshot.umd.js'), 'utf8'), 'original bytes');
});

test('Node license gate honors AND/OR grouping and refuses unknown/invalid declarations', async () => {
  const { licenseAllowed } = await checker;
  assert.equal(licenseAllowed('MIT OR GPL-3.0-only'), true);
  assert.equal(licenseAllowed('MIT AND GPL-3.0-only'), false);
  assert.equal(licenseAllowed('(MIT OR GPL-3.0-only) AND Apache-2.0'), true);
  assert.equal(licenseAllowed('MIT AND (Apache-2.0 OR GPL-3.0-only)'), true);
  for (const expression of [undefined, {}, '', 'SEE LICENSE IN COPYING', 'MIT GPL-3.0-only', 'MIT OR', '(MIT', 'MIT WITH unknown', 'MIT/Apache-2.0']) {
    assert.equal(licenseAllowed(expression), false, JSON.stringify(expression));
  }
});

test('npm snapshot binds the full request, lock, tool version, acquisition time and raw responses', async () => {
  const { validateNpmSnapshot, sha256 } = await checker;
  const good = await snapshot();
  assert.equal(validateNpmSnapshot(good, packages, 'lock-hash', now).findings, 0);
  assert.ok(good.request['modern-screenshot'], 'bundled package outside bun.lock is queried too');
  for (const changes of [
    { bunLockSha256: 'changed' }, { bunVersion: '0.0.0' }, { request: { example: ['1.2.3'] } },
    { queriedAt: '2026-08-01T00:00:00Z' }, { queriedAt: '2026-10-09T00:00:00Z' }, { queriedAt: 'invalid' },
    { response: { error: 'network failed' } }, { responseSha256: 'changed' }, { response: [] },
    { bunExitCode: 1 }, { skippedRegistries: true },
  ]) assert.throws(() => validateNpmSnapshot({ ...good, ...changes }, packages, 'lock-hash', now));
  const response = { example: [{ id: 123, severity: 'low', vulnerable_versions: '<2.0.0' }] };
  assert.throws(() => validateNpmSnapshot({ ...good, response, responseSha256: sha256(JSON.stringify(response)) }, packages, 'lock-hash', now), /unresolved security/);
  assert.throws(() => validateNpmSnapshot({ ...good, bunResponse: response, bunResponseSha256: sha256(JSON.stringify(response)) }, packages, 'lock-hash', now), /unresolved security/);
});

test('RustSec pin rejects another commit/tree, dirty files and stale content despite a fresh fetch', async () => {
  const { databasePin, validateDatabase } = await checker;
  const state = { ...databasePin, commitAt: '2026-10-07T16:40:27+02:00' };
  validateDatabase(state, '', now);
  assert.throws(() => validateDatabase({ ...state, revision: 'another' }, '', now), /revision/);
  assert.throws(() => validateDatabase({ ...state, tree: 'another' }, '', now), /tree/);
  assert.throws(() => validateDatabase(state, ' M crates/example/advisory.md', now), /modified/);
  assert.throws(() => validateDatabase(state, '?? injected-advisory.md', now), /untracked/);
  assert.throws(() => validateDatabase({ ...state, commitAt: '2026-08-01T00:00:00Z', fetchedAt: now.toISOString() }, '', now), /older than 30 days/);
});

test('explicit database preparation retrieves a pinned historical commit from a fresh shallow clone', async t => {
  const { pinDatabase } = await checker;
  const directory = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'phoenix shallow db '));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const source = path.join(directory, 'source'), clone = path.join(directory, 'clone');
  fs.mkdirSync(source);
  const git = (cwd, args) => {
    const result = spawnSync('git', ['-C', cwd, ...args], { encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout.trim();
  };
  git(source, ['init']);
  git(source, ['config', 'user.email', 'fixture@example.invalid']);
  git(source, ['config', 'user.name', 'Fixture']);
  fs.writeFileSync(path.join(source, 'advisory.md'), 'pinned advisory\n');
  git(source, ['add', 'advisory.md']); git(source, ['commit', '-m', 'pinned']);
  const revision = git(source, ['rev-parse', 'HEAD']), tree = git(source, ['rev-parse', 'HEAD^{tree}']);
  fs.writeFileSync(path.join(source, 'advisory.md'), 'later advisory\n');
  git(source, ['commit', '-am', 'later']);
  git(directory, ['clone', '--depth', '1', pathToFileURL(source).href, clone]);
  assert.equal(spawnSync('git', ['-C', clone, 'cat-file', '-e', revision]).status, 1);
  pinDatabase(clone, { url: pathToFileURL(source).href, revision, tree });
  assert.equal(git(clone, ['rev-parse', 'HEAD']), revision);
  assert.equal(git(clone, ['rev-parse', 'HEAD^{tree}']), tree);
  assert.equal(fs.readFileSync(path.join(clone, 'advisory.md'), 'utf8'), 'pinned advisory\n');
  assert.equal(git(clone, ['status', '--porcelain']), '');
  assert.throws(() => pinDatabase(clone, { url: pathToFileURL(source).href, revision: 'f'.repeat(40), tree }), /fetch pinned RustSec/);
  assert.equal(git(clone, ['rev-parse', 'HEAD']), revision);
});

test('cargo-deny evidence requires both completed checks and retains actionable findings', async () => {
  const { cargoDiagnostics } = await checker;
  const summary = { type: 'summary', fields: { advisories: { errors: 1 }, licenses: { errors: 0 } } };
  const diagnostic = { type: 'diagnostic', fields: { code: 'vulnerability', severity: 'error', message: 'TLS boundary failure',
    advisory: { id: 'RUSTSEC-2026-0285', aliases: ['GHSA-2mjx-qc3c-rqvc'], url: 'https://example.com/advisory' },
    graphs: [{ Krate: { name: 'rustls', version: '0.23.43' } }], notes: ['Solution: Upgrade to >=0.23.45', 'long source prose'] } };
  const result = cargoDiagnostics('', `${JSON.stringify(diagnostic)}\n${JSON.stringify(summary)}\n`);
  assert.equal(result.findings[0].id, 'RUSTSEC-2026-0285');
  assert.equal(result.findings[0].packages[0].version, '0.23.43');
  assert.deepEqual(result.findings[0].notes, ['Solution: Upgrade to >=0.23.45']);
  assert.throws(() => cargoDiagnostics('', JSON.stringify({ type: 'summary', fields: { advisories: {} } })), /both/);
  assert.throws(() => cargoDiagnostics('', 'failed to open database'), SyntaxError);
});

test('archive integrity reads actual bytes without executing code or extracting paths', async () => {
  const { tarMember, tarFiles } = await checker;
  const archive = tar([['package/LICENSE', 'MIT text'], ['package/dist/index.js', 'throw new Error("must not execute")']]);
  assert.equal(tarMember(archive, 'package/LICENSE').toString(), 'MIT text');
  assert.equal(tarFiles(archive).size, 2);
  assert.throws(() => tarMember(archive, 'missing'), /Missing/);
  assert.throws(() => tarFiles(tar([['../escape', 'payload']])), /Unsafe/);
  assert.throws(() => tarFiles(tar([['package/LICENSE', 'x', '2']])), /Unsupported/);
  assert.throws(() => tarFiles(tar([['package/LICENSE', 'a'], ['package/LICENSE', 'b']])), /Duplicate/);
  const longName = `package/${'nested/'.repeat(20)}LICENSE`;
  assert.equal(tarMember(tar([['././@LongLink', `${longName}\0`, 'L'], ['truncated', 'actual']]), longName).toString(), 'actual');
});

test('crate validation uses the original locked archive and detects modified/extra source', async t => {
  const { verifyCrate, sha256 } = await checker;
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-dependency-'));
  t.after(() => fs.rmSync(temporary, { recursive: true, force: true }));
  const source = path.join(temporary, 'registry/src/test-index/example-1.2.3');
  const cache = path.join(temporary, 'registry/cache/test-index');
  fs.mkdirSync(source, { recursive: true }); fs.mkdirSync(cache, { recursive: true });
  const archive = tar([['example-1.2.3/lib.rs', 'pub fn original() {}'], ['example-1.2.3/LICENSE', 'MIT']]);
  const cached = path.join(cache, 'example-1.2.3.crate');
  fs.writeFileSync(cached, archive); fs.writeFileSync(path.join(source, 'lib.rs'), 'pub fn original() {}'); fs.writeFileSync(path.join(source, 'LICENSE'), 'MIT');
  const locked = { name: 'example', version: '1.2.3', checksum: sha256(archive) };
  verifyCrate(source, locked); // .cargo-checksum.json is absent in some real prepared caches.
  fs.writeFileSync(path.join(source, '.cargo-ok'), ''); verifyCrate(source, locked);
  fs.writeFileSync(path.join(source, 'lib.rs'), 'pub fn modified() {}');
  assert.throws(() => verifyCrate(source, locked), /modified registry source/);
  fs.writeFileSync(path.join(source, 'lib.rs'), 'pub fn original() {}'); fs.writeFileSync(path.join(source, 'extra.rs'), 'extra');
  assert.throws(() => verifyCrate(source, locked), /extra registry source/);
  fs.unlinkSync(path.join(source, 'extra.rs')); fs.writeFileSync(cached, Buffer.from('corrupted archive'));
  assert.throws(() => verifyCrate(source, locked), /archive differs/);
});

test('distribution gate requires actual identical texts and corresponding MPL source evidence', async t => {
  const { assertDistribution, sha256 } = await checker;
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-notices-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const text = 'MPL full terms'; const notice = 'Recipients can obtain corresponding source.';
  const hash = sha256(text); const license = { path: `THIRD-PARTY-LICENSES/${hash}.txt`, sha256: hash };
  const catalog = { rust: [{ name: 'covered', version: '1.2.3', license: 'MPL-2.0', sourceUnmodified: true,
    sourceUrl: 'https://static.crates.io/crates/covered/covered-1.2.3.crate', checksum: 'a'.repeat(64), licenses: [license] }], node: [], bundled: [], resources: [] };
  fs.mkdirSync(path.join(directory, 'THIRD-PARTY-LICENSES'));
  fs.writeFileSync(path.join(directory, 'THIRD-PARTY.json'), `${JSON.stringify(catalog, null, 2)}\n`);
  fs.writeFileSync(path.join(directory, 'THIRD-PARTY-NOTICES.md'), notice);
  assert.throws(() => assertDistribution(directory, catalog, notice), /missing or changed/);
  fs.writeFileSync(path.join(directory, license.path), text);
  assert.equal(assertDistribution(directory, catalog, notice).requiredFiles, 3);
  catalog.rust[0].sourceUnmodified = false;
  assert.throws(() => assertDistribution(directory, catalog, notice), /corresponding source/);
  catalog.rust[0].sourceUnmodified = true; fs.writeFileSync(path.join(directory, license.path), 'changed text');
  assert.throws(() => assertDistribution(directory, catalog, notice), /missing or changed/);
});
