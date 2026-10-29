'use strict';
// Controller/verifier negative tests. No fake process can certify migration.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
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
