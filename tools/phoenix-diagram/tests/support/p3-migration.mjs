// Inert complete old targets for isolated migration acceptance. No old program
// is imported or executed, and no missing bytes are fetched from source/network.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { gunzipSync } from 'node:zlib';

export const MIGRATION_FIXTURE_MEMBER = 'tools/phoenix-diagram/tests/fixtures/migration/legacy-sources.v1.json.gz';
export const MIGRATION_FIXTURE_IDENTITY = Object.freeze({ bytes: 12327222, sha256: '94198b9dfd0445ca45fdc32c4d931d35c1c9d6b2c033b2647a5be17ed490043b' });
export const MIGRATION_MANIFEST_MEMBER = 'tools/phoenix-diagram/migration-sources/archify-v1.json';
const SOURCES = Object.freeze([
  ['legacy-2.17', 219, 'd00a98aff8e461c4dd446054ea1cede3bec5b3239863fb44db3c0173b61cb7f1'],
  ['legacy-2.17-when-to-use', 219, '2943184c17a5d8f9f5b383e7460eb3f8057f8d0b3e1ffde11fbfe7caf04e6d55'],
  ['reference-3.1.0', 157, 'da4787260b911254547cf0651c46b2e48917bd7105a3b20739ce68b8fed5d66f'],
]);
const validated = new WeakSet();
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const canonical = value => Buffer.from(JSON.stringify(value) + '\n');
const hash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
function keys(value, names, label) { assert(value && typeof value === 'object' && !Array.isArray(value), label + ' must be an object'); assert.deepEqual(Object.keys(value).sort(), [...names].sort(), label + ' fields differ'); }
function safePath(name) { assert(typeof name === 'string' && name && !/[\\:\0-\x1f\x7f]/.test(name) && name.split('/').every(part => part && part !== '.' && part !== '..'), 'Unsafe migration member path'); return name; }
function decode(value, bytes, digest) { assert(typeof value === 'string' && Number.isSafeInteger(bytes) && bytes >= 0 && hash(digest), 'Invalid migration byte descriptor'); const data = Buffer.from(value, 'base64'); assert.equal(data.toString('base64'), value, 'Noncanonical migration base64'); assert.equal(data.length, bytes, 'Migration byte length differs'); assert.equal(sha256(data), digest, 'Migration byte digest differs'); return data; }
function freeze(value) { if (value && typeof value === 'object') { for (const child of Object.values(value)) freeze(child); Object.freeze(value); } return value; }

export function readMigrationFixture(bytes, { sourceInputs } = {}) {
  assert(Buffer.isBuffer(bytes), 'Supply the actual fixture bytes');
  assert.equal(bytes.length, MIGRATION_FIXTURE_IDENTITY.bytes, 'Migration fixture size differs');
  assert.equal(sha256(bytes), MIGRATION_FIXTURE_IDENTITY.sha256, 'Migration fixture digest differs');
  assert.equal(bytes.readUInt32LE(4), 0, 'Migration gzip mtime differs'); assert.equal(bytes[9], 255, 'Migration gzip OS header differs');
  const plain = gunzipSync(bytes, { maxOutputLength: 64 * 1024 * 1024 }), fixture = JSON.parse(plain);
  assert(canonical(fixture).equals(plain), 'Migration fixture JSON is not canonical');
  keys(fixture, ['schemaVersion', 'kind', 'manifest', 'sources'], 'Migration fixture');
  assert.equal(fixture.schemaVersion, 1); assert.equal(fixture.kind, 'phoenix-diagram-inert-migration-fixture');
  keys(fixture.manifest, ['path', 'bytes', 'sha256', 'base64'], 'Migration manifest bytes');
  assert.equal(fixture.manifest.path, MIGRATION_MANIFEST_MEMBER);
  const manifestBytes = decode(fixture.manifest.base64, fixture.manifest.bytes, fixture.manifest.sha256);
  assert(Array.isArray(sourceInputs) && sourceInputs.every(row => Array.isArray(row) && row.length === 3 && typeof row[0] === 'string' && Number.isSafeInteger(row[1]) && row[1] >= 0 && hash(row[2])), 'Supply authenticated candidate sourceInputs tuples');
  assert.equal(new Set(sourceInputs.map(row => row[0])).size, sourceInputs.length, 'Duplicate candidate source input');
  const rows = sourceInputs.filter(row => row[0] === MIGRATION_MANIFEST_MEMBER);
  assert.equal(rows.length, 1, 'Candidate does not bind the migration manifest');
  assert.deepEqual(rows[0], [MIGRATION_MANIFEST_MEMBER, manifestBytes.length, sha256(manifestBytes)], 'Fixture manifest is not the candidate source input');
  const manifest = JSON.parse(manifestBytes);
  assert.equal(manifest.schemaVersion, 1); assert.equal(manifest.kind, 'archify-migration-sources');
  assert.equal(manifest.policy.establishesOwnership, false); assert.equal(manifest.policy.targetInstallationBackup, false);
  assert.equal(manifest.sources?.length, SOURCES.length); assert.equal(fixture.sources?.length, SOURCES.length);
  for (const [index, [id, count, contentSha256]] of SOURCES.entries()) {
    const source = fixture.sources[index], expected = manifest.sources[index]; keys(source, ['id', 'files'], 'Migration source');
    assert.equal(source.id, id); assert.equal(expected.id, id); assert.equal(expected.fileCount, count); assert.equal(expected.contentSha256, contentSha256);
    assert(Array.isArray(source.files) && source.files.length === count, 'Incomplete migration source');
    let previous = ''; const names = new Set(), inventory = [];
    for (const file of source.files) {
      keys(file, ['path', 'bytes', 'mode', 'sha256', 'base64'], 'Migration file'); safePath(file.path);
      assert(file.path > previous && !names.has(file.path), 'Migration files must be uniquely sorted'); previous = file.path; names.add(file.path);
      assert([0o644, 0o755].includes(file.mode), 'Unsupported migration file mode'); decode(file.base64, file.bytes, file.sha256);
      inventory.push({ path: file.path, bytes: file.bytes, mode: file.mode, sha256: file.sha256 });
    }
    for (const name of names) for (let parent = path.posix.dirname(name); parent !== '.'; parent = path.posix.dirname(parent)) assert(!names.has(parent), 'Migration file/directory collision');
    assert.equal(sha256(canonical(inventory)), contentSha256, 'Migration complete source identity differs');
    assert.deepEqual(inventory, expected.files, 'Migration file set differs from candidate manifest');
  }
  validated.add(fixture); return freeze(fixture);
}

function physical(directory, { privateRoot = false } = {}) {
  const stat = fs.lstatSync(directory, { bigint: true });
  assert(stat.isDirectory() && !stat.isSymbolicLink(), 'Use an ordinary migration directory');
  if (privateRoot) assert(stat.uid === BigInt(process.getuid()) && (stat.mode & 0o7777n) === 0o700n, 'Use a fresh owned 0700 migration project');
  return { dev: stat.dev, ino: stat.ino };
}
function verifyDirectory(directory, expected) { const actual = physical(directory); assert.equal(actual.dev, expected.dev, 'Migration directory device changed'); assert.equal(actual.ino, expected.ino, 'Migration directory identity changed'); }
function noLinkParents(directory) { const parsed = path.parse(directory); let current = parsed.root; physical(current); for (const part of directory.slice(parsed.root.length).split('/').filter(Boolean)) { current = path.join(current, part); physical(current); } }

export function materializeKnownSource(fixture, sourceId, project, { provider = '.agents' } = {}) {
  assert(validated.has(fixture), 'Materialize only data returned by readMigrationFixture');
  const source = fixture.sources.find(entry => entry.id === sourceId); assert(source, 'Unknown fixed migration source');
  assert(['.agents', '.claude', '.codex'].includes(provider), 'Choose an explicit supported provider');
  assert(typeof project === 'string' && path.isAbsolute(project) && path.resolve(project) === project, 'Supply an absolute fresh project');
  noLinkParents(project); const rootIdentity = physical(project, { privateRoot: true });
  assert.equal(fs.readdirSync(project).length, 0, 'Migration project must be newly created and empty');
  const directories = new Map([[project, rootIdentity]]), root = path.join(project, provider, 'skills', 'archify');
  function directory(target) {
    if (directories.has(target)) { verifyDirectory(target, directories.get(target)); return; }
    const parent = path.dirname(target); directory(parent); verifyDirectory(project, rootIdentity);
    fs.mkdirSync(target, { mode: 0o700 }); directories.set(target, physical(target, { privateRoot: true }));
  }
  directory(root);
  for (const file of source.files) {
    const target = path.join(root, file.path), parent = path.dirname(target); directory(parent);
    const bytes = decode(file.base64, file.bytes, file.sha256); verifyDirectory(project, rootIdentity); verifyDirectory(parent, directories.get(parent));
    const descriptor = fs.openSync(target, fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW, file.mode);
    try {
      fs.writeFileSync(descriptor, bytes); fs.fchmodSync(descriptor, file.mode);
      const stat = fs.fstatSync(descriptor, { bigint: true }), current = fs.lstatSync(target, { bigint: true });
      assert(stat.isFile() && stat.nlink === 1n && current.dev === stat.dev && current.ino === stat.ino && (stat.mode & 0o7777n) === BigInt(file.mode), 'Materialized migration file identity changed');
    } finally { fs.closeSync(descriptor); }
    verifyDirectory(parent, directories.get(parent)); verifyDirectory(project, rootIdentity);
  }
  return { project, root, sourceId, fileCount: source.files.length, establishesOwnership: false, targetInstallationBackup: false };
}
