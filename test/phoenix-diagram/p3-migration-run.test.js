'use strict';
// Controller/verifier negative tests. No fake process can certify migration.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');
const modulePath = '../../tools/phoenix-diagram/tests/support/p3-migration-run.mjs';
function temp(t) { const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-migration-run-unit-'))); fs.chmodSync(root, 0o700); t.after(() => fs.rmSync(root, { recursive: true, force: true })); return root; }

test('fixed migration cases cover every durable phase and isolate the genuine v1 prerequisite', async () => {
  const { MIGRATION_CASES } = await import(modulePath);
  const phases = MIGRATION_CASES.filter(entry => entry.kind === 'checkpoint');
  assert.equal(new Set(phases.map(entry => entry.phase)).size, 13);
  assert.deepEqual(phases.filter(entry => entry.requiresLegacyV1).map(entry => entry.phase), ['rollback-compat-published']);
  assert(MIGRATION_CASES.some(entry => entry.signal === 'SIGINT'));
  for (const id of ['unconfirmed', 'dry-run', 'tampered', 'extra-file', 'external-manager', 'personal', 'legacy-symlink', 'legacy-hardlink', 'later-legacy', 'later-managed', 'same-content-managed']) assert(MIGRATION_CASES.some(entry => entry.id === id), id);
  assert.equal(MIGRATION_CASES.filter(entry => entry.kind === 'known').length, 3);
});

test('missing real candidate, public npm consumer and fixture remains unrun without invoking anything', async t => {
  const { runMigrationAcceptance, verifyMigrationAcceptance, MIGRATION_CASES } = await import(modulePath), root = temp(t);
  let calls = 0; const scope = await runMigrationAcceptance({ outputDir: path.join(root, 'run'), invoke: () => { calls++; throw new Error('must not execute'); } });
  assert.equal(calls, 0); assert.equal(scope.status, 'incomplete');
  const report = JSON.parse(fs.readFileSync(scope.record.path)); assert.equal(report.cases.length, MIGRATION_CASES.length); assert(report.cases.every(entry => entry.status === 'not-run'));
  assert.equal(verifyMigrationAcceptance(scope).status, 'incomplete');
  const original = fs.readFileSync(scope.record.path); let reads = 0;
  assert.equal(verifyMigrationAcceptance(scope, { bytesFor: ref => { reads++; assert.equal(ref.path, scope.record.path); return original; } }).status, 'incomplete'); assert.equal(reads, 1);
  assert.throws(() => verifyMigrationAcceptance(scope, { bytesFor: () => Buffer.from('{}') }));
});

test('a hand-filled pass or truncated case inventory cannot replace actual process and journal originals', async t => {
  const { runMigrationAcceptance, verifyMigrationAcceptance } = await import(modulePath), root = temp(t);
  const scope = await runMigrationAcceptance({ outputDir: path.join(root, 'run') }), report = JSON.parse(fs.readFileSync(scope.record.path));
  assert.throws(() => verifyMigrationAcceptance({ ...report, status: 'pass' }));
  assert.throws(() => verifyMigrationAcceptance({ ...report, cases: report.cases.slice(1) }));
  const forged = structuredClone(report); forged.status = 'pass'; for (const entry of forged.cases) entry.status = 'pass';
  assert.throws(() => verifyMigrationAcceptance(forged));
});

test('a missing historical package cannot be relabelled as a passed compatibility checkpoint', async t => {
  const { runMigrationAcceptance, verifyMigrationAcceptance } = await import(modulePath), root = temp(t);
  const scope = await runMigrationAcceptance({ outputDir: path.join(root, 'run') }), report = JSON.parse(fs.readFileSync(scope.record.path));
  const compatibility = report.cases.find(entry => entry.id === 'rollback-compat-published');
  assert.equal(compatibility.status, 'not-run'); assert.match(compatibility.diagnostics[0].message, /genuine historical v1/);
  compatibility.status = 'pass'; compatibility.ownership = { proved: true }; compatibility.transactionId = 'a'.repeat(32);
  assert.throws(() => verifyMigrationAcceptance(report), /authenticated candidate|original public consumer/);
});

function cleanupFixture() {
  const root = '/owned/project', transactionId = 'a'.repeat(32), parentPath = `.phoenix-diagram/install/transactions/${transactionId}/before`, cleanupPath = parentPath + '/.phoenix-diagram-output-cleanup';
  const physical = inode => ({ device: '1', inode: String(inode) }), directory = (name, inode) => ({ path: name, type: 'directory', ...physical(inode), mode: 0o700, uid: 501, links: 2 });
  const owner = { schemaVersion: 1, tool: 'phoenix-diagram', kind: 'output-cleanup-directory', parentPath: root + '/' + parentPath, parent: physical(10), directory: physical(11), recordIdentity: physical(12), checksum: '' };
  const digest = bytes => createHash('sha256').update(bytes).digest('hex');
  owner.checksum = digest(Buffer.from(JSON.stringify(owner)));
  const bytes = Buffer.from(JSON.stringify(owner) + '\n'), row = { path: cleanupPath + '/owner-v1.json', type: 'file', ...physical(12), mode: 0o600, uid: 501, links: 1, bytes: bytes.length, sha256: digest(bytes) };
  const tree = { root, physical: { ...physical(1), mode: 0o700, uid: 501 }, entries: [directory(parentPath, 10), directory(cleanupPath, 11), row] };
  return { tree, journal: { transactionId, operations: [] }, owner, bytes, row, cleanupPath, digest,
    read(reference) { assert.equal(reference.path, root + '/' + row.path); assert.deepEqual(reference.identity, { bytes: row.bytes, sha256: row.sha256 }); assert.deepEqual(reference.binding, { device: row.device, inode: row.inode, mode: row.mode, uid: row.uid, links: row.links }); return bytes; } };
}

test('the fixed cleanup owner is verified as publication metadata, with no arbitrary backup namespace', async () => {
  const { verifyNoLegacyBackup } = await import(modulePath), valid = cleanupFixture();
  assert.doesNotThrow(() => verifyNoLegacyBackup(valid.tree, valid.journal, valid.read));
  for (const mutate of [
    value => { value.tree.entries[1].mode = 0o755; },
    value => { value.tree.entries[1].uid++; },
    value => { value.row.links = 2; },
    value => { value.row.mode = 0o644; },
    value => { value.row.inode = '99'; },
    value => { value.tree.entries.push({ ...value.row, path: value.cleanupPath + '/unknown.json' }); },
    value => { value.tree.entries.push({ ...value.tree.entries[1], path: value.cleanupPath + '/unknown-directory' }); },
    value => { value.tree.entries.push({ ...value.tree.entries[1], path: value.cleanupPath.replace('.phoenix-diagram-output-cleanup', '.unknown-directory') }); },
    value => { value.tree.entries = value.tree.entries.filter(row => row !== value.row); },
    value => { value.row.path = value.row.path.replace('owner-v1.json', 'owner-v2.json'); },
    value => { value.tree.entries.push({ ...value.row, path: '.phoenix-diagram/install/archify/backup.json' }); },
  ]) { const value = cleanupFixture(); mutate(value); assert.throws(() => verifyNoLegacyBackup(value.tree, value.journal, value.read)); }
  for (const mutate of [
    owner => { owner.extra = true; }, owner => { owner.parentPath += '/other'; }, owner => { owner.parent.inode = '99'; },
    owner => { owner.directory.inode = '99'; }, owner => { owner.recordIdentity.inode = '99'; }, owner => { owner.parent.extra = true; },
  ]) {
    const value = cleanupFixture(); mutate(value.owner); value.owner.checksum = ''; value.owner.checksum = value.digest(Buffer.from(JSON.stringify(value.owner)));
    const bytes = Buffer.from(JSON.stringify(value.owner) + '\n'); value.row.bytes = bytes.length; value.row.sha256 = value.digest(bytes);
    assert.throws(() => verifyNoLegacyBackup(value.tree, value.journal, () => bytes));
  }
  for (const encode of [owner => JSON.stringify({ ...owner, checksum: '0'.repeat(64) }) + '\n', owner => JSON.stringify(owner, null, 2) + '\n']) {
    const value = cleanupFixture(), bytes = Buffer.from(encode(value.owner)); value.row.bytes = bytes.length; value.row.sha256 = value.digest(bytes);
    assert.throws(() => verifyNoLegacyBackup(value.tree, value.journal, () => bytes));
  }
});

function historicalReceiptFixture() {
  const current = { sourceInputSha256: 'current' }, old = { sourceInputSha256: 'historical' }, transactionId = 'a'.repeat(32);
  const active = { schemaVersion: 1, transactionId, engine: old, npmPackageVersion: '0.7.4', bundleManifestSha256: 'bundle', engineManifestSha256: 'version' };
  const originalV1 = { schemaVersion: 1, transactionId, active }, journal = { schemaVersion: 2, transactionId, active: structuredClone(active) };
  const receipt = { command: 'migrate', status: 'rolled-back', transactionId, transactionSchemaVersion: 2, engine: old, npmPackageVersion: active.npmPackageVersion, bundleManifestSha256: active.bundleManifestSha256, versionManifestSha256: active.engineManifestSha256 };
  return { receipt, selected: { engine: { engineIdentity: current } }, context: { definition: { id: 'rollback-compat-published', requiresLegacyV1: true }, role: 'recover', journal, originalV1, legacyV1: { engine: { engineIdentity: old } } } };
}

test('current recovery receipts bind the historical installation while transport remains separately selected', async () => {
  const { verifyMigrationReceiptEngine } = await import(modulePath);
  const current = historicalReceiptFixture(); assert.doesNotThrow(() => verifyMigrationReceiptEngine({ engine: current.selected.engine.engineIdentity }, current.selected));
  for (const role of ['recover', 'rollback', 'repeat']) { const value = historicalReceiptFixture(); value.context.role = role; assert.doesNotThrow(() => verifyMigrationReceiptEngine(value.receipt, value.selected, value.context)); }
  for (const mutate of [
    value => { value.context.role = 'apply'; }, value => { value.context.definition.requiresLegacyV1 = false; }, value => { value.context.definition.id = 'other'; },
    value => { value.receipt.engine = value.selected.engine.engineIdentity; }, value => { delete value.receipt.engine; },
    value => { value.context.journal.active.engine = { sourceInputSha256: 'other' }; }, value => { value.context.originalV1.active = { ...value.context.originalV1.active, npmPackageVersion: 'other' }; },
    value => { value.context.legacyV1.engine.engineIdentity = { sourceInputSha256: 'other' }; },
    value => { value.context.originalV1.schemaVersion = 2; }, value => { value.context.journal.schemaVersion = 1; },
    value => { value.receipt.transactionId = 'b'.repeat(32); }, value => { value.receipt.transactionSchemaVersion = 1; },
    value => { value.receipt.npmPackageVersion = 'other'; }, value => { value.receipt.bundleManifestSha256 = 'other'; }, value => { value.receipt.versionManifestSha256 = 'other'; },
  ]) { const value = historicalReceiptFixture(); mutate(value); assert.throws(() => verifyMigrationReceiptEngine(value.receipt, value.selected, value.context)); }
});
