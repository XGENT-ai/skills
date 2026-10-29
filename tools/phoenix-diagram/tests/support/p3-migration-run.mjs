// Fixed public npm migration cases. The inert fixture never supplies a
// historical Diagram installer and no receipt/journal is manufactured here.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { isDeepStrictEqual as same } from 'node:util';
import { fileURLToPath } from 'node:url';
import { privateDirectory, writeNew, bound, readSnapshot, readBound, identity, verifyActualTransport } from './p3-state.mjs';
import { readEvidenceBytes } from '../../../../scripts/test-phoenix-diagram-core.mjs';
import { verifyCandidateCohort } from '../../../../scripts/check-phoenix-diagram-candidate.mjs';
import { proveControllerOwnership } from './p3-processes.mjs';
import { readMigrationFixture, materializeKnownSource, MIGRATION_FIXTURE_IDENTITY } from './p3-migration.mjs';

const STATE = '.phoenix-diagram/install', ACTIVE = STATE + '/active.json', LEGACY = '.agents/skills/archify';
const KIND = 'p3-public-migration-acceptance', ID = /^[a-f0-9]{32}$/;
export const LEGACY_V1_DESCRIPTOR_MEMBER = 'tools/phoenix-diagram/tests/fixtures/migration/legacy-candidate.v1.json';
const CLAIMANT = Buffer.from('Later authored claimant must survive migration recovery.\n');
const PROTECTED = ['business.json', 'AGENTS.md', '.claude/settings.json', '.claude/skills/archify/custom.txt', '.phoenix-ui/keep.txt'];
const PHASES = ['install-journal-ready', 'install-after-retire', 'install-after-link', 'install-before-active', 'install-after-active', 'install-committed', 'migrate-before-legacy-remove', 'migrate-after-legacy-remove', 'rollback-after-file', 'rollback-old-journal-retired', 'rollback-old-journal-published', 'rollback-audit-archived', 'rollback-compat-published'];
export const MIGRATION_CASES = Object.freeze([
  ...['legacy-2.17', 'legacy-2.17-when-to-use', 'reference-3.1.0'].map(sourceId => ({ id: 'known-' + sourceId, kind: 'known', sourceId })),
  ...PHASES.map(phase => ({ id: phase, kind: 'checkpoint', phase, signal: 'SIGKILL', requiresLegacyV1: phase === 'rollback-compat-published', seedV2: ['install-after-retire', 'install-after-link', 'rollback-old-journal-retired', 'rollback-old-journal-published'].includes(phase) })),
  { id: 'migration-sigint', kind: 'checkpoint', phase: 'migrate-after-legacy-remove', signal: 'SIGINT' },
  ...['dry-run', 'unconfirmed', 'tampered', 'extra-file', 'external-manager', 'personal', 'legacy-symlink', 'legacy-hardlink', 'legacy-mode'].map(id => ({ id, kind: 'refusal' })),
  ...['later-legacy', 'later-managed', 'same-content-managed', 'later-backup', 'later-parent'].map(id => ({ id, kind: 'later' })),
].map(Object.freeze));
const argsFor = (project, mode = 'apply', id) => mode === 'install' ? ['install', '--project', project, '--providers', 'codex', '--json'] : mode === 'apply' ? ['migrate', '--from-archify', '--project', project, '--providers', 'codex', '--confirm-remove-archify', '--json'] : ['migrate', '--' + mode, id, '--project', project, '--json'];
const metadata = stat => ({ device: String(stat.dev), inode: String(stat.ino), mode: Number(stat.mode & 0o7777n), uid: Number(stat.uid), links: Number(stat.nlink) });
const physical = row => ({ device: row.device, inode: row.inode });
function relative(name) { assert(typeof name === 'string' && name && !/[\\:\0-\x1f\x7f]/.test(name) && name.split('/').every(part => part && part !== '.' && part !== '..'), 'Unsafe observed migration path'); return name; }
function tree(project) {
  const root = fs.lstatSync(project, { bigint: true }); assert(root.isDirectory() && !root.isSymbolicLink() && (root.mode & 0o7777n) === 0o700n && root.uid === BigInt(process.getuid()), 'Observe only the owned private project');
  const entries = [];
  function visit(directory, prefix = '') {
    for (const name of fs.readdirSync(directory).sort()) {
      const member = relative(prefix ? prefix + '/' + name : name), file = path.join(project, member), stat = fs.lstatSync(file, { bigint: true });
      const row = { path: member, ...metadata(stat) };
      if (stat.isSymbolicLink()) { row.type = 'symlink'; row.target = fs.readlinkSync(file); }
      else if (stat.isDirectory()) { row.type = 'directory'; visit(file, member); }
      else { assert(stat.isFile(), 'Unexpected special migration file'); row.type = 'file'; Object.assign(row, readSnapshot(file, { allowMultipleLinks: true }).identity); }
      entries.push(row);
    }
  }
  visit(project); entries.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  return { schemaVersion: 1, kind: 'p3-migration-tree', root: project, physical: metadata(root), entries };
}
function saveTree(directory, name, project) { return writeNew(path.join(directory, name + '.tree.json'), tree(project)); }
function copyOriginal(directory, name, file) { const snapshot = readSnapshot(file); return { original: bound(snapshot), copy: writeNew(path.join(directory, name), snapshot.content) }; }
function sealed(value) { assert(typeof value.checksum === 'string', 'Journal has no checksum'); assert.equal(value.checksum, identity(Buffer.from(JSON.stringify({ ...value, checksum: '' }) + '\n')).sha256, 'Original journal checksum differs'); return value; }
function loadJournal(project, id) {
  assert(ID.test(id), 'Invalid explicit migration ID'); const archived = path.join(project, STATE, 'transactions', id, 'rolled-back-v2.json');
  const file = fs.existsSync(archived) ? archived : path.join(project, STATE, 'journals', id + '.json'), snapshot = readSnapshot(file), journal = sealed(JSON.parse(snapshot.content));
  assert.equal(journal.transactionId, id); assert.equal(journal.tool, 'phoenix-diagram'); assert(same(journal.project, physical(metadata(fs.lstatSync(project, { bigint: true })))), 'Journal belongs to another physical project');
  const transaction = path.join(project, STATE, 'transactions', id), stat = fs.lstatSync(transaction, { bigint: true }); assert(stat.isDirectory() && !stat.isSymbolicLink() && (stat.mode & 0o7777n) === 0o700n && stat.uid === BigInt(process.getuid()));
  assert(same(journal.transaction, physical(metadata(stat))), 'Journal belongs to another transaction directory'); return { file, journal, transaction };
}
function activeId(project) { const value = sealed(JSON.parse(readSnapshot(path.join(project, ACTIVE)).content)); assert(ID.test(value.transactionId)); return value.transactionId; }
function locateApplying(project) {
  const candidates = fs.readdirSync(path.join(project, STATE, 'journals')).filter(name => /^[a-f0-9]{32}\.json$/.test(name)).map(name => loadJournal(project, name.slice(0, -5))).filter(row => ['applying', 'recovering'].includes(row.journal.phase));
  assert.equal(candidates.length, 1, 'Exactly one durable applying journal must select recovery; stdout/marker is not ownership'); return candidates[0];
}
function parsePs(text) { return text.trim().split('\n').filter(Boolean).map(line => { const match = /^\s*(\d+)\s+(\d+)\s+(\d+)\s+(.*)$/.exec(line); assert(match, 'Invalid original ps row'); return { pid: Number(match[1]), ppid: Number(match[2]), pgid: Number(match[3]), command: match[4] }; }); }
function ownership(child, engine, pid, command, directory) {
  const rows = [], outputs = [], queries = []; let current = pid;
  while (rows.length < 32) {
    assert(Number.isSafeInteger(current) && current > 0 && !rows.some(row => row.pid === current), 'Invalid native PID ancestry');
    const argv = ['-ww', '-o', 'pid=,ppid=,pgid=,args=', '-p', String(current)], stdout = execFileSync('/bin/ps', argv, { encoding: 'utf8' }); const parsed = parsePs(stdout); assert.equal(parsed.length, 1); assert.equal(parsed[0].pid, current);
    queries.push(argv); outputs.push(stdout); rows.push(parsed[0]); if (current === child.pid) break; current = parsed[0].ppid;
  }
  const proof = proveControllerOwnership(child, engine, rows, { controllerPid: pid, command });
  return { proof, program: '/bin/ps', queries, stdout: writeNew(path.join(directory, 'owned-ancestry.stdout'), outputs.join('')), observedAt: new Date().toISOString() };
}
async function groupGone(pid) { const end = Date.now() + 10000; while (Date.now() < end) { try { process.kill(-pid, 0); } catch (error) { if (error.code === 'ESRCH') return { operation: 'kill(-ownedPgid,0)', processGroup: pid, result: 'ESRCH', observedAt: new Date().toISOString() }; throw error; } await delay(25); } throw new Error('Owned interrupted process group did not exit'); }
function processFile(directory, role, args, env, result) { const raw = result.processResult || result; return writeNew(path.join(directory, role + '.process.json'), { schemaVersion: 1, kind: 'p3-migration-public-process', args, environmentOverrides: env, exitCode: raw.exitCode, signal: raw.signal ?? null, actualTransport: result.actualTransport || raw.actualTransport || null, stdout: writeNew(path.join(directory, role + '.stdout'), raw.stdout || ''), stderr: writeNew(path.join(directory, role + '.stderr'), raw.stderr || '') }); }
function binding(candidate, consumer) { assert(candidate?.source && candidate?.version && consumer?.engine?.identity && consumer?.publicNpm && consumer?.candidate?.ownedTarball, 'Authenticated candidate and public npm consumer are required'); assert(same(candidate.engine.identity, consumer.engine.identity)); assert(same(candidate.tgz.identity, consumer.candidate.ownedTarball.identity), 'Consumer uses another candidate tgz'); assert.equal(consumer.publicNpm.bin, 'phoenix-diagram'); assert.equal(consumer.publicNpm.enginePath, consumer.engine.path); assert.equal(consumer.publicNpm.tarball, consumer.candidate.ownedTarball.path); assert.equal(consumer.engine.engineIdentity?.sourceInputSha256, candidate.source.inputSha256); return { source: candidate.source, bundleSha256: candidate.version.bundleSha256, tarball: consumer.candidate.ownedTarball, engine: consumer.engine, publicNpm: consumer.publicNpm }; }
function historicalDescriptor(bytes, legacyV1) {
  return verifyCandidateCohort(legacyV1.candidate, { legacyDescriptor: bytes });
}
function terminalStatus(cases) { return cases.some(row => row.status === 'failed') ? 'failed' : cases.every(row => row.status === 'pass') ? 'pass' : 'incomplete'; }

async function runCase(definition, context) {
  const { directory, fixture, consumer, invoke, legacyV1 } = context, engine = consumer.engine, project = privateDirectory(path.join(directory, 'project'));
  materializeKnownSource(fixture, definition.sourceId || 'legacy-2.17', project);
  for (const name of PROTECTED) { fs.mkdirSync(path.dirname(path.join(project, name)), { recursive: true, mode: 0o700 }); fs.writeFileSync(path.join(project, name), 'Authored shared file: ' + name + '\n', { flag: 'wx', mode: 0o600 }); }
  const entry = { id: definition.id, status: 'failed', project, processes: [], trees: {}, journals: [], diagnostics: [] };
  async function call(role, args, { env = {}, onSpawn, json = true, old = false } = {}) {
    const selected = old ? legacyV1 : { consumer, invoke }; readBound(selected.consumer.engine, 'same migration executable');
    try { const result = await selected.invoke(selected.consumer.engine, args, { env, onSpawn, json, timeoutMs: 600000 }); entry.processes.push({ role, old, file: processFile(directory, role, args, env, result) }); return result; }
    catch (error) { if (error.processResult) entry.processes.push({ role, old, file: processFile(directory, role, args, env, error) }); throw error; }
  }
  const observe = name => { entry.trees[name] = saveTree(directory, name, project); };
  const journal = role => { const current = loadJournal(project, entry.transactionId); const original = copyOriginal(directory, role + '.journal.json', current.file); entry.journals.push({ role, ...original }); return current; };
  async function interrupt(args, phase, signal) {
    const checkpoint = privateDirectory(path.join(directory, 'checkpoint'));
    entry.checkpointDirectory = { path: checkpoint, ...metadata(fs.lstatSync(checkpoint, { bigint: true })) };
    const result = await call('interrupted', args, { json: false, env: { PHOENIX_DIAGRAM_ACCEPTANCE: '1', PHOENIX_DIAGRAM_CHECKPOINT_DIR: checkpoint, PHOENIX_DIAGRAM_CHECKPOINT_PHASE: phase }, onSpawn: async child => {
      const end = Date.now() + 300000, file = path.join(checkpoint, 'checkpoint.json'); let marker;
      while (Date.now() < end) {
        if (child.exitCode !== null || child.signalCode !== null) throw new Error('Public npm process exited before migration checkpoint');
        try { const current = readSnapshot(file); marker = JSON.parse(current.content); break; } catch (error) { if (!['ENOENT', 'core/file-binding', 'core/stale-evidence'].includes(error.code) && !(error instanceof SyntaxError)) throw error; }
        await delay(20);
      }
      assert(marker, 'Migration checkpoint timed out'); assert.equal(marker.schemaVersion, 1); assert.equal(marker.phase, phase); assert(path.isAbsolute(marker.target) && !path.relative(project, marker.target).startsWith('..'), 'Checkpoint target escaped the owned project');
      entry.ownership = ownership(child, engine, marker.pid, args[0], directory);
      // Select ID from durable journal/active, then cross-check the marker.
      if (!entry.transactionId) entry.transactionId = phase === 'install-committed' ? activeId(project) : locateApplying(project).journal.transactionId;
      const current = loadJournal(project, entry.transactionId); assert.equal(marker.recoveryDirectory, current.transaction);
      entry.checkpoint = copyOriginal(directory, 'checkpoint.original.json', file); journal('checkpoint'); observe('checkpoint');
      entry.interruption = { signal, processGroup: child.pid, deliveredAt: new Date().toISOString(), selection: 'durable-journal-id', stdoutUsedForRecovery: false };
      process.kill(-child.pid, signal);
    } });
    assert(entry.interruption, 'No actual interruption was sent');
    assert(result.signal === signal || (signal === 'SIGINT' && result.signal === null && result.exitCode === 130), 'Public process did not exit from the requested actual signal');
    entry.groupExit = await groupGone(entry.interruption.processGroup);
  }
  try {
    if (definition.kind === 'refusal') {
      const file = path.join(project, LEGACY, 'SKILL.md');
      if (definition.id === 'tampered') fs.appendFileSync(file, '\ncustomized\n');
      if (definition.id === 'extra-file') fs.writeFileSync(path.join(project, LEGACY, 'extra.txt'), CLAIMANT);
      if (definition.id === 'external-manager') fs.writeFileSync(path.join(project, 'skills-lock.json'), '{"skills":{"archify":{}}}\n');
      if (definition.id === 'legacy-mode') fs.chmodSync(file, 0o664);
      if (definition.id === 'legacy-hardlink') fs.linkSync(file, path.join(directory, 'outside-owned-hardlink'));
      if (definition.id === 'legacy-symlink') { fs.renameSync(path.join(project, LEGACY), path.join(directory, 'owned-link-target')); fs.symlinkSync(path.join(directory, 'owned-link-target'), path.join(project, LEGACY)); }
      observe('before'); let args = argsFor(project), env = {};
      if (definition.id === 'dry-run') args = [...args, '--dry-run'];
      if (definition.id === 'unconfirmed') args = args.filter(arg => arg !== '--confirm-remove-archify');
      if (definition.id === 'personal') env = { HOME: project };
      await call('refusal', args, { env }); observe('after');
    } else {
      if (definition.seedV2 || definition.id === 'later-backup') await call('seed-v2', argsFor(project, 'install'));
      if (definition.requiresLegacyV1) { await call('seed-v1', argsFor(project, 'install'), { old: true }); entry.v1Active = copyOriginal(directory, 'v1-active.original.json', path.join(project, ACTIVE)); assert.equal(JSON.parse(readBound(entry.v1Active.copy, 'real v1 active').content).schemaVersion, 1); const first = loadJournal(project, activeId(project)); assert.equal(first.journal.schemaVersion, 1); entry.transactionId = first.journal.transactionId; journal('v1-original'); }
      observe('before');
      const reversePhase = definition.phase?.startsWith('rollback-');
      if (definition.requiresLegacyV1) await interrupt(argsFor(project, 'rollback', entry.transactionId), definition.phase, definition.signal);
      else if (reversePhase || (definition.kind === 'later' && !['later-legacy', 'later-parent'].includes(definition.id))) {
        await call('apply', argsFor(project)); entry.transactionId = activeId(project); journal('committed'); observe('applied');
        if (reversePhase) await interrupt(argsFor(project, 'rollback', entry.transactionId), definition.phase, definition.signal);
      } else if (definition.kind === 'checkpoint' || ['later-legacy', 'later-parent'].includes(definition.id)) await interrupt(argsFor(project), definition.phase || 'migrate-after-legacy-remove', definition.signal || 'SIGKILL');
      else { await call('apply', argsFor(project)); entry.transactionId = activeId(project); journal('committed'); observe('applied'); }
      if (definition.kind === 'later') {
        if (definition.id === 'later-legacy') fs.writeFileSync(path.join(project, LEGACY, 'LICENSE'), CLAIMANT, { flag: 'wx', mode: 0o644 });
        if (definition.id === 'later-parent') { const parent = path.join(project, '.agents/skills'); fs.renameSync(parent, path.join(directory, 'owned-displaced-parent')); fs.mkdirSync(parent, { mode: 0o700 }); fs.mkdirSync(path.join(parent, 'archify'), { mode: 0o700 }); fs.writeFileSync(path.join(parent, 'archify/LICENSE'), CLAIMANT, { mode: 0o644 }); }
        if (['later-managed', 'same-content-managed'].includes(definition.id)) { const target = path.join(project, '.agents/skills/phoenix-diagram/SKILL.md'), bytes = fs.readFileSync(target), mode = fs.statSync(target).mode & 0o7777; fs.renameSync(target, path.join(directory, 'owned-original-managed')); fs.writeFileSync(target, definition.id === 'same-content-managed' ? bytes : CLAIMANT, { flag: 'wx', mode }); fs.chmodSync(target, mode); }
        if (definition.id === 'later-backup') { const current = loadJournal(project, entry.transactionId), op = current.journal.operations.find(op => op.backupPath); assert(op, 'Expected a genuine existing-v2 backup'); fs.appendFileSync(path.join(current.transaction, op.backupPath), CLAIMANT); }
        observe('modified'); await call('protected', argsFor(project, ['later-legacy', 'later-parent'].includes(definition.id) ? 'recover' : 'rollback', entry.transactionId)); journal('protected'); observe('after');
      } else {
        await call('recover', argsFor(project, 'recover', entry.transactionId)); journal('recovered'); observe('recovered');
        await call('rollback', argsFor(project, 'rollback', entry.transactionId)); journal('rolled-back'); observe('rolled-back');
        await call('repeat', argsFor(project, 'rollback', entry.transactionId)); observe('repeat');
      }
    }
    entry.status = 'pass';
  } catch (error) { entry.diagnostics.push({ message: error.message, code: error.code || 'migration/acceptance' }); }
  return entry;
}

export async function runMigrationAcceptance({ candidate, consumer, outputDir, invoke, fixture, legacyV1, legacyDescriptor } = {}) {
  assert(typeof outputDir === 'string' && path.isAbsolute(outputDir), 'Provide a new absolute migration outputDir'); const root = privateDirectory(outputDir);
  const report = { schemaVersion: 1, kind: KIND, outputDir: root, status: 'incomplete', startedAt: new Date().toISOString(), binding: null, fixture: fixture || null, legacyV1: null, legacyDescriptor: null, cases: [], diagnostics: [] }; let parsed;
  try { report.binding = binding(candidate, consumer); assert(typeof invoke === 'function', 'Provide the real public npm invoke adapter'); assert(fixture, 'Provide the transferred fixed inert fixture'); parsed = readMigrationFixture(readBound(fixture, 'fixed inert migration fixture').content, { sourceInputs: candidate.sourceInputs.entries }); if (legacyV1) { assert(typeof legacyV1.invoke === 'function'); const original = legacyDescriptor ? readBound(legacyDescriptor, 'explicit historical v1 descriptor') : readSnapshot(fileURLToPath(new URL('../fixtures/migration/legacy-candidate.v1.json', import.meta.url))); historicalDescriptor(original.content, legacyV1); report.legacyDescriptor = writeNew(path.join(root, 'legacy-candidate.input.json'), original.content); report.legacyV1 = binding(legacyV1.candidate, legacyV1.consumer); } }
  catch (error) { report.diagnostics.push({ message: error.message, code: 'migration/missing-prerequisite' }); }
  for (const definition of MIGRATION_CASES) {
    if (!parsed || (definition.requiresLegacyV1 && !report.legacyV1)) { report.cases.push({ id: definition.id, status: 'not-run', diagnostics: [{ code: 'migration/missing-prerequisite', message: definition.requiresLegacyV1 ? 'Requires a separately authenticated genuine historical v1 package and public transport; inert Archify bytes cannot replace it' : 'Requires bound candidate, actual public npm transport and exact inert migration fixture' }] }); continue; }
    const directory = privateDirectory(path.join(root, definition.id)); const entry = await runCase(definition, { directory, fixture: parsed, consumer, invoke, legacyV1 });
    if (entry.status === 'pass') { try { verifyCase(entry, definition, report, { candidate, consumer, legacyV1 }, parsed); } catch (error) { entry.status = 'failed'; entry.diagnostics.push({ message: error.message, code: 'migration/evidence' }); } }
    report.cases.push(entry);
  }
  report.status = terminalStatus(report.cases); report.closedAt = new Date().toISOString();
  const record = writeNew(path.join(root, 'index.json'), report); return { status: report.status, record, diagnostics: report.diagnostics };
}

function readTree(reference, entry, read) {
  const value = JSON.parse(read(reference, 'original project tree')); assert.equal(value.schemaVersion, 1); assert.equal(value.kind, 'p3-migration-tree'); assert.equal(value.root, entry.project);
  assert.equal(value.physical.mode, 0o700); assert(Number.isInteger(value.physical.uid)); assert(Array.isArray(value.entries));
  let previous = ''; for (const row of value.entries) { relative(row.path); assert(row.path > previous, 'Tree entries are not uniquely sorted'); previous = row.path; assert(['file', 'directory', 'symlink'].includes(row.type)); if (row.type === 'file') assert(Number.isSafeInteger(row.bytes) && /^[a-f0-9]{64}$/.test(row.sha256)); }
  return value;
}
const fileAt = (tree, name) => tree.entries.find(row => row.path === name);
function contentAndPhysical(row, expected, links) { assert(row?.type === 'file', 'Expected original ordinary file observation'); for (const key of ['device', 'inode', 'mode', 'uid', 'bytes', 'sha256']) assert.equal(row[key], expected[key], 'Observed file identity differs: ' + key); if (links !== undefined) assert.equal(row.links, links); }
function sameEntries(before, after, names) { for (const name of names) assert(same(fileAt(before, name), fileAt(after, name)), 'Protected authored file changed: ' + name); }
function fixedSource(tree, fixture, sourceId) {
  const source = fixture.sources.find(source => source.id === sourceId); assert(source); const files = tree.entries.filter(row => row.path.startsWith(LEGACY + '/') && row.type === 'file');
  assert.equal(files.length, source.files.length, 'Observed legacy source is incomplete');
  for (const file of source.files) { const row = fileAt(tree, LEGACY + '/' + file.path); assert(row?.type === 'file' && row.links === 1); for (const key of ['bytes', 'sha256', 'mode']) assert.equal(row[key], file[key]); }
}
function originalCopy(value, tree, entry, read, label) {
  assert(value?.copy && value.original?.binding && same(value.copy.identity, value.original.identity), label + ' lost its original byte identity');
  const bytes = read(value.copy, label), member = relative(path.relative(entry.project, value.original.path));
  const observed = fileAt(tree, member); assert(observed?.type === 'file', label + ' was not observed in the physical project');
  assert(same(physical(observed), physical(value.original.binding)) && observed.mode === value.original.binding.mode && observed.sha256 === value.original.identity.sha256 && observed.bytes === value.original.identity.bytes, label + ' original binding differs from tree observation');
  return bytes;
}
function verifyNoLegacyBackup(tree, journal) {
  assert(journal.operations.every(op => op.path === ACTIVE || /^\.(?:agents|claude|aider|continue|cursor|gemini|github|kiro|opencode|roo|trae|windsurf)\/skills\/phoenix-diagram\//.test(op.path)), 'Journal adopted a legacy skill write');
  for (const row of tree.entries.filter(row => row.path.startsWith(STATE + '/'))) {
    assert(!row.path.includes('/archify/') && !row.path.endsWith('/archify') && !/\.(?:tar|tgz|gz)$/.test(row.path), 'Private installation state contains a legacy target backup');
    const match = new RegExp('^' + STATE.replaceAll('.', '\\.') + '/transactions/' + journal.transactionId + '/before/(.+)$').exec(row.path);
    if (!match || row.type !== 'file') continue;
    if (/^\.phoenix-diagram-lock-[a-f0-9]{64}\.lock$/.test(match[1])) continue;
    assert(/^\d+$/.test(match[1]), 'Unknown target backup member'); const op = journal.operations[Number(match[1])]; assert(op?.backup && op.backupPath === 'before/' + match[1], 'Backup does not belong to a new Diagram write'); contentAndPhysical(row, op.backup, 1);
  }
}
function verifyActive(tree, active, selected, candidate) {
  sealed(active); assert(same(active.engine, selected.engine.engineIdentity), 'Installed active names another engine');
  assert.equal(active.bundleManifestSha256, candidate.version.bundleSha256);
  const member = name => { const found = candidate.members.get('dist/phoenix-diagram/bundle/' + name); assert(found, 'Authenticated tgz lacks bundle member ' + name); return found; };
  const manifest = JSON.parse(member('manifest.json').bytes), engine = member('ENGINE.json');
  assert.equal(active.engineManifestSha256, identity(engine.bytes).sha256);
  const expected = [...manifest.files.map(file => ({ path: '.agents/skills/phoenix-diagram/' + file.path, bytes: file.size, sha256: file.sha256, mode: file.mode })), { path: '.agents/skills/phoenix-diagram/ENGINE.json', ...identity(engine.bytes), mode: 0o644 }];
  assert.equal(active.ownership.length, expected.length, 'Installed ownership is not the complete fixed bundle');
  assert.equal(tree.entries.filter(row => row.type === 'file' && row.path.startsWith('.agents/skills/phoenix-diagram/')).length, expected.length);
  for (const file of expected) { const owned = active.ownership.find(owned => owned.path === file.path); assert(owned, 'Installed bundle member is unowned'); contentAndPhysical(fileAt(tree, file.path), owned.identity, 1); for (const key of ['bytes', 'sha256', 'mode']) assert.equal(owned.identity[key], file[key], 'Installed bytes differ from unique candidate tgz'); }
  const encoded = Buffer.from(JSON.stringify(active) + '\n'), row = fileAt(tree, ACTIVE); assert(row?.type === 'file' && row.sha256 === identity(encoded).sha256 && row.bytes === encoded.length, 'Visible active differs from original bound ownership');
}
function checkReceipt(process, definition, { id, statuses, policy = false } = {}) {
  assert.equal(process.signal, null); assert.equal(process.exitCode, 0); const value = process.receipt;
  assert.equal(value?.tool, 'phoenix-diagram'); assert.equal(value.command, process.args[0]); assert.equal(value.project, process.args[process.args.indexOf('--project') + 1]); assert(statuses.includes(value.status), 'Public command returned another status');
  if (id) assert.equal(value.transactionId, id, 'Recovery/rollback used another ID');
  if (policy) { assert.equal(value.legacyBackupPolicy, 'none'); assert.equal(value.legacyRestoration, 'not-supported'); if (!definition.requiresLegacyV1) assert(value.reinstallInstructions?.length, 'Migration omitted actual no-backup reinstallation instructions'); }
}
function verifyCheckpoint(entry, definition, report, processes, trees, journals, read) {
  assert(entry.checkpoint?.original?.binding?.mode === 0o600 && entry.checkpoint.original.binding.links === 1, 'Checkpoint marker was not an original private file');
  assert(same(entry.checkpoint.copy.identity, entry.checkpoint.original.identity));
  const marker = JSON.parse(read(entry.checkpoint.copy, 'actual checkpoint marker')), process = processes.get('interrupted'), tree = trees.checkpoint, journal = journals.get('checkpoint');
  assert(marker.schemaVersion === 1 && marker.phase === (definition.phase || 'migrate-after-legacy-remove')); assert.equal(marker.recoveryDirectory, path.join(entry.project, STATE, 'transactions', entry.transactionId));
  assert.equal(entry.checkpoint.original.path, path.join(path.dirname(entry.project), 'checkpoint/checkpoint.json'));
  assert.equal(entry.checkpointDirectory?.path, path.dirname(entry.checkpoint.original.path)); assert.equal(entry.checkpointDirectory.mode, 0o700); assert.equal(entry.checkpointDirectory.uid, tree.physical.uid);
  assert(entry.ownership?.program === '/bin/ps' && entry.ownership.queries?.length, 'Original process ancestry is required');
  const rows = parsePs(read(entry.ownership.stdout, 'original owned ps stdout').toString('utf8')); assert.equal(rows.length, entry.ownership.queries.length);
  for (const [index, query] of entry.ownership.queries.entries()) assert(same(query, ['-ww', '-o', 'pid=,ppid=,pgid=,args=', '-p', String(rows[index].pid)]));
  const proof = proveControllerOwnership({ pid: process.actualTransport.pid }, report.binding.engine, rows, { controllerPid: marker.pid, command: process.args[0] }); assert(same(proof, entry.ownership.proof), 'Saved ancestry is not the recomputed owned native process');
  assert.equal(entry.interruption?.signal, definition.signal || 'SIGKILL'); assert.equal(entry.interruption.processGroup, proof.outerPid); assert.equal(entry.interruption.selection, 'durable-journal-id'); assert.equal(entry.interruption.stdoutUsedForRecovery, false);
  assert(Number.isFinite(Date.parse(entry.interruption.deliveredAt))); assert(process.signal === entry.interruption.signal || (entry.interruption.signal === 'SIGINT' && process.signal === null && process.exitCode === 130), 'No observed interrupted public-process exit');
  assert.equal(entry.groupExit?.operation, 'kill(-ownedPgid,0)'); assert.equal(entry.groupExit.processGroup, proof.outerPid); assert.equal(entry.groupExit.result, 'ESRCH'); assert(Date.parse(entry.groupExit.observedAt) >= Date.parse(process.actualTransport.closedAt));
  assert(same(process.environmentOverrides, { PHOENIX_DIAGRAM_ACCEPTANCE: '1', PHOENIX_DIAGRAM_CHECKPOINT_DIR: path.join(path.dirname(entry.project), 'checkpoint'), PHOENIX_DIAGRAM_CHECKPOINT_PHASE: definition.phase || 'migrate-after-legacy-remove' }), 'Maintenance gate was not exact');
  assert(journal && same(journal.project, physical(tree.physical))); const transaction = fileAt(tree, path.join(STATE, 'transactions', entry.transactionId)); assert(transaction?.type === 'directory' && transaction.mode === 0o700 && same(journal.transaction, physical(transaction)));
  const member = relative(path.relative(entry.project, marker.target)), row = fileAt(tree, member), phase = marker.phase;
  if (phase === 'install-journal-ready') { assert.equal(marker.target, path.join(entry.project, STATE, 'journals', entry.transactionId + '.json')); assert.equal(journal.phase, 'applying'); assert.equal(journal.completed, 0); assert.equal(journal.pending, null); assert(journal.migration.roots.every(root => root.entries.every(file => !file.removed))); }
  else if (phase.startsWith('migrate-')) { const root = journal.migration?.roots.find(root => root.pending !== null); assert(root && Number.isSafeInteger(root.pending)); const target = root.entries[root.pending]; assert.equal(member, target.path); assert.equal(target.removed, false); if (phase === 'migrate-before-legacy-remove') contentAndPhysical(row, target.file, 1); else assert(!row, 'After-remove checkpoint still exposes the removed entry'); }
  else if (['install-after-retire', 'install-after-link', 'install-before-active', 'install-after-active'].includes(phase)) {
    assert(Number.isSafeInteger(journal.pending)); const op = journal.operations[journal.pending]; assert.equal(member, op.path);
    if (phase === 'install-after-retire') { assert(!row); if (op.before) contentAndPhysical(fileAt(tree, STATE + '/transactions/' + entry.transactionId + '/' + op.retiredPath), op.before, 1); }
    if (phase === 'install-after-link') contentAndPhysical(row, op.after, 2);
    if (phase === 'install-before-active') { assert.equal(member, ACTIVE); if (op.before) contentAndPhysical(row, op.before, 1); else assert(!row); }
    if (phase === 'install-after-active') { assert.equal(member, ACTIVE); contentAndPhysical(row, op.after, 1); }
  } else if (phase === 'install-committed') { assert.equal(journal.phase, 'committed'); assert.equal(journal.completed, journal.operations.length); assert.equal(journal.pending, null); assert.equal(member, ACTIVE); contentAndPhysical(row, journal.operations.at(-1).after, 1); }
  else if (phase === 'rollback-after-file') { assert.equal(journal.phase, 'rolling-back'); const reverse = journal.rollback.operations.find(reverse => journal.operations[reverse.index].path === member); assert(reverse && !reverse.done); if (reverse.restored) contentAndPhysical(row, reverse.restored, 1); else assert(!row); }
  else if (phase.startsWith('rollback-old-journal-')) { const previous = journal.rollback?.previousJournal; assert(previous); assert.equal(member, STATE + '/journals/' + previous.transactionId + '.json'); if (phase.endsWith('retired')) { assert(!row); contentAndPhysical(fileAt(tree, STATE + '/transactions/' + entry.transactionId + '/retired/rollback-journal'), journal.rollback.previousJournalIdentity, 1); } else contentAndPhysical(row, journal.rollback.previousJournalAfter, 1); }
  else if (phase === 'rollback-audit-archived') { assert.equal(journal.phase, 'rolled-back'); assert.equal(member, STATE + '/transactions/' + entry.transactionId + '/rolled-back-v2.json'); }
  else if (phase === 'rollback-compat-published') { assert.equal(journal.phase, 'rolled-back'); assert(journal.rollback?.compatibility); assert.equal(member, STATE + '/journals/' + entry.transactionId + '.json'); contentAndPhysical(row, journal.rollback.compatibility.after, 1); const compatibility = sealed(journal.rollback.compatibility.journal); assert.equal(compatibility.schemaVersion, 1); assert.equal(compatibility.phase, 'recovered'); }
  else assert.fail('Unknown fixed migration checkpoint');
}

function verifyCase(entry, definition, report, context, fixture) {
  const read = (reference, label) => readEvidenceBytes(reference, { bytesFor: context.bytesFor, label });
  assert.equal(entry.id, definition.id); assert.equal(entry.project, path.join(report.outputDir, entry.id, 'project')); assert.equal(entry.status, 'pass');
  const trees = Object.fromEntries(Object.entries(entry.trees).map(([name, ref]) => [name, readTree(ref, entry, read)])); assert(trees.before);
  for (const observed of Object.values(trees)) { assert(same(physical(observed.physical), physical(trees.before.physical)) && observed.physical.mode === trees.before.physical.mode && observed.physical.uid === trees.before.physical.uid, 'Project root was replaced'); sameEntries(trees.before, observed, PROTECTED); }
  const processes = new Map();
  const expectedRoles = definition.kind === 'refusal' ? ['refusal'] : [
    ...(definition.seedV2 || definition.id === 'later-backup' ? ['seed-v2'] : []),
    ...(definition.requiresLegacyV1 ? ['seed-v1'] : []),
    ...(definition.kind === 'known' || (definition.phase?.startsWith('rollback-') && !definition.requiresLegacyV1) || (definition.kind === 'later' && !['later-legacy', 'later-parent'].includes(definition.id)) ? ['apply'] : []),
    ...(definition.kind === 'checkpoint' || ['later-legacy', 'later-parent'].includes(definition.id) ? ['interrupted'] : []),
    ...(definition.kind === 'later' ? ['protected'] : ['recover', 'rollback', 'repeat']),
  ];
  assert.deepEqual(entry.processes.map(saved => saved.role), expectedRoles, 'Actual fixed public process sequence differs');
  for (const saved of entry.processes) {
    assert(!processes.has(saved.role), 'Duplicate public process role'); const original = JSON.parse(read(saved.file, 'original public migration process'));
    assert.equal(original.schemaVersion, 1); assert.equal(original.kind, 'p3-migration-public-process');
    assert.equal(saved.old, saved.role === 'seed-v1'); const selected = saved.old ? report.legacyV1 : report.binding; assert(selected, 'Unexpected historical execution');
    verifyActualTransport(original.actualTransport, selected.publicNpm, original.args, []); assert(Number.isFinite(Date.parse(original.actualTransport.closedAt)) && Date.parse(original.actualTransport.closedAt) >= Date.parse(original.actualTransport.startedAt));
    const stdout = read(original.stdout, 'original migration stdout').toString('utf8'); read(original.stderr, 'original migration stderr');
    original.receipt = saved.role === 'interrupted' ? null : JSON.parse(stdout);
    if (!['interrupted', 'refusal'].includes(saved.role)) assert(same(original.environmentOverrides, {}), 'Unexpected process environment override');
    if (original.receipt?.engine) assert(same(original.receipt.engine, selected.engine.engineIdentity), 'Receipt belongs to another native engine');
    processes.set(saved.role, original);
  }
  const journals = new Map();
  for (const saved of entry.journals) {
    assert(!journals.has(saved.role)); const name = { 'v1-original': 'before', committed: 'applied', checkpoint: 'checkpoint', recovered: 'recovered', 'rolled-back': 'rolled-back', protected: 'after' }[saved.role]; assert(name && trees[name]);
    const value = sealed(JSON.parse(originalCopy(saved, trees[name], entry, read, 'original durable migration journal'))); assert.equal(value.transactionId, entry.transactionId); assert.equal(value.tool, 'phoenix-diagram'); assert(same(value.project, physical(trees.before.physical)));
    assert(value.operations?.length && value.active.transactionId === entry.transactionId); if (saved.role !== 'protected') verifyNoLegacyBackup(trees[name], value); journals.set(saved.role, value);
  }
  if (definition.kind === 'refusal') {
    const process = processes.get('refusal'); assert(process); let expected = argsFor(entry.project);
    if (definition.id === 'dry-run') expected = [...expected, '--dry-run']; if (definition.id === 'unconfirmed') expected = expected.filter(arg => arg !== '--confirm-remove-archify');
    assert(same(process.args, expected)); assert(same(process.environmentOverrides, definition.id === 'personal' ? { HOME: entry.project } : {}));
    assert.equal(process.signal, null); assert.equal(process.exitCode, definition.id === 'dry-run' ? 0 : 3); assert(same(trees.before.entries, trees.after.entries), 'Refused/read-only operation modified the actual target tree');
    if (['dry-run', 'unconfirmed', 'external-manager', 'personal'].includes(definition.id)) fixedSource(trees.before, fixture, 'legacy-2.17');
    const skill = fileAt(trees.before, LEGACY + '/SKILL.md'), originalSkill = fixture.sources[0].files.find(file => file.path === 'SKILL.md');
    if (definition.id === 'tampered') assert(skill.sha256 !== originalSkill.sha256, 'Source tamper was not observed');
    if (definition.id === 'extra-file') assert.equal(fileAt(trees.before, LEGACY + '/extra.txt')?.sha256, identity(CLAIMANT).sha256);
    if (definition.id === 'external-manager') assert.equal(fileAt(trees.before, 'skills-lock.json')?.sha256, identity(Buffer.from('{"skills":{"archify":{}}}\n')).sha256);
    if (definition.id === 'legacy-mode') assert.equal(skill?.mode, 0o664);
    if (definition.id === 'legacy-hardlink') assert.equal(skill?.links, 2);
    if (definition.id === 'legacy-symlink') { const link = fileAt(trees.before, LEGACY); assert.equal(link?.type, 'symlink'); assert.equal(link.target, path.join(path.dirname(entry.project), 'owned-link-target')); }
    if (definition.id === 'dry-run') { assert.equal(process.receipt.status, 'planned'); assert.equal(process.receipt.legacyBackupPolicy, 'none'); }
    else assert(process.receipt.status !== 'installed' && (process.receipt.conflicts?.length || process.receipt.diagnostics?.length), 'Refusal lacks actual diagnostics');
    assert.equal(entry.processes.length, 1); return;
  }
  assert(ID.test(entry.transactionId)); fixedSource(trees.before, fixture, definition.sourceId || 'legacy-2.17');
  if (definition.seedV2 || definition.id === 'later-backup') { assert(same(processes.get('seed-v2')?.args, argsFor(entry.project, 'install'))); checkReceipt(processes.get('seed-v2'), definition, { statuses: ['installed'] }); }
  if (definition.requiresLegacyV1) {
    assert(report.legacyV1 && context.legacyV1, 'Genuine separately bound historical v1 package is required'); assert(same(processes.get('seed-v1')?.args, argsFor(entry.project, 'install'))); checkReceipt(processes.get('seed-v1'), definition, { statuses: ['installed'] });
    const active = sealed(JSON.parse(originalCopy(entry.v1Active, trees.before, entry, read, 'actual historical active'))); assert.equal(active.schemaVersion, 1); assert.equal(journals.get('v1-original')?.schemaVersion, 1); assert.equal(active.transactionId, entry.transactionId); verifyActive(trees.before, active, report.legacyV1, context.legacyV1.candidate);
  }
  if (processes.has('apply')) { assert(same(processes.get('apply').args, argsFor(entry.project))); checkReceipt(processes.get('apply'), definition, { id: entry.transactionId, statuses: ['installed'], policy: true }); assert.equal(journals.get('committed')?.phase, 'committed'); verifyActive(trees.applied, journals.get('committed').active, report.binding, context.candidate); assert(!trees.applied.entries.some(row => row.path === LEGACY || row.path.startsWith(LEGACY + '/')), 'Successful apply retained legacy files'); }
  if (definition.kind === 'checkpoint' || ['later-legacy', 'later-parent'].includes(definition.id)) {
    const expected = argsFor(entry.project, definition.phase?.startsWith('rollback-') ? 'rollback' : 'apply', entry.transactionId); assert(same(processes.get('interrupted')?.args, expected)); verifyCheckpoint(entry, definition, report, processes, trees, journals, read);
  }
  if (definition.kind === 'later') {
    const process = processes.get('protected'), mode = ['later-legacy', 'later-parent'].includes(definition.id) ? 'recover' : 'rollback'; assert(same(process?.args, argsFor(entry.project, mode, entry.transactionId))); assert.equal(process.signal, null); assert.equal(process.exitCode, 3);
    const ownJournal = STATE + '/journals/' + entry.transactionId + '.json', visible = tree => tree.entries.filter(row => mode !== 'recover' || row.path !== ownJournal);
    assert(same(visible(trees.modified), visible(trees.after)), 'Recovery/rollback changed a later claimant or unrelated physical entry');
    const beforeJournal = journals.get(mode === 'recover' ? 'checkpoint' : 'committed'), afterJournal = journals.get('protected'); assert(beforeJournal && afterJournal);
    assert(same({ ...beforeJournal, phase: '', checksum: '' }, { ...afterJournal, phase: '', checksum: '' }), 'Conflict advanced removal or managed-file progress');
    assert.equal(afterJournal.phase, mode === 'recover' ? 'recovering' : beforeJournal.phase);
    if (['later-managed', 'same-content-managed'].includes(definition.id)) { const name = '.agents/skills/phoenix-diagram/SKILL.md', before = fileAt(trees.applied, name), after = fileAt(trees.modified, name); assert(before && after && before.inode !== after.inode); if (definition.id === 'same-content-managed') assert(before.bytes === after.bytes && before.sha256 === after.sha256 && before.mode === after.mode); else assert.equal(after.sha256, identity(CLAIMANT).sha256); }
    if (definition.id === 'later-backup') { const op = beforeJournal.operations.find(op => op.backupPath); assert(op); const backup = fileAt(trees.modified, STATE + '/transactions/' + entry.transactionId + '/' + op.backupPath); assert(backup && backup.sha256 !== op.backup.sha256, 'Backup tamper was not observed'); }
    if (definition.id === 'later-parent') assert.notEqual(fileAt(trees.checkpoint, '.agents/skills')?.inode, fileAt(trees.modified, '.agents/skills')?.inode);
    if (definition.id === 'later-legacy' || definition.id === 'later-parent') assert.equal(fileAt(trees.after, LEGACY + '/LICENSE')?.sha256, identity(CLAIMANT).sha256);
    return;
  }
  for (const [role, statuses] of [['recover', definition.phase?.startsWith('rollback-') ? ['rolled-back'] : ['recovered', 'unchanged']], ['rollback', ['rolled-back']], ['repeat', ['rolled-back']]]) { assert(same(processes.get(role)?.args, argsFor(entry.project, role === 'recover' ? 'recover' : 'rollback', entry.transactionId))); checkReceipt(processes.get(role), definition, { id: entry.transactionId, statuses, policy: true }); }
  const final = journals.get('rolled-back'); assert.equal(final?.phase, 'rolled-back'); assert.equal(final.rollback.completed, final.operations.length); assert(same(trees['rolled-back'].entries, trees.repeat.entries), 'Repeated rollback changed the terminal tree');
  if (journals.get('recovered')?.phase === 'committed') verifyActive(trees.recovered, journals.get('recovered').active, report.binding, context.candidate);
  if (final.rollback.restoredActive) verifyActive(trees.repeat, final.rollback.restoredActive, report.binding, context.candidate);
  else assert(!trees.repeat.entries.some(row => row.type === 'file' && row.path.startsWith('.agents/skills/phoenix-diagram/')), 'Fresh rollback retained new managed files');
  if (definition.requiresLegacyV1) { fixedSource(trees.repeat, fixture, 'legacy-2.17'); assert(!fileAt(trees.repeat, ACTIVE)); }
  else for (const observed of [trees.recovered, trees['rolled-back'], trees.repeat]) assert(!observed.entries.some(row => row.path === LEGACY || row.path.startsWith(LEGACY + '/')), 'Recovery/rollback rebuilt a removed legacy skill');
}

export function verifyMigrationAcceptance(scope, { bytesFor, candidate, consumer, legacyV1 } = {}) {
  const read = (reference, label) => readEvidenceBytes(reference, { bytesFor, label });
  const report = scope.record ? JSON.parse(read(scope.record, 'original migration acceptance index')) : scope;
  assert.equal(report.schemaVersion, 1); assert.equal(report.kind, KIND); assert(path.isAbsolute(report.outputDir));
  assert.deepEqual(report.cases.map(row => row.id), MIGRATION_CASES.map(row => row.id), 'Fixed migration case inventory differs');
  assert(report.cases.every(row => ['pass', 'failed', 'not-run'].includes(row.status))); const status = terminalStatus(report.cases); assert.equal(report.status, status); if (scope.record) assert.equal(scope.status, status);
  let fixture;
  if (report.cases.some(row => row.status === 'pass')) {
    assert(candidate && consumer, 'Verify successful cases against the authenticated candidate and original public consumer'); assert(same(report.binding, binding(candidate, consumer)), 'Migration used another candidate/transport'); assert(same(report.fixture?.identity, MIGRATION_FIXTURE_IDENTITY));
    fixture = readMigrationFixture(read(report.fixture, 'same complete inert migration fixture'), { sourceInputs: candidate.sourceInputs.entries });
    if (report.legacyV1) { assert(legacyV1); assert(same(report.legacyV1, binding(legacyV1.candidate, legacyV1.consumer))); historicalDescriptor(read(report.legacyDescriptor, 'same fixed historical v1 descriptor'), legacyV1); }
  }
  for (const [index, entry] of report.cases.entries()) { const definition = MIGRATION_CASES[index]; if (entry.status === 'pass') verifyCase(entry, definition, report, { bytesFor, candidate, consumer, legacyV1 }, fixture); else assert(entry.diagnostics?.length, 'An unexecuted/failed migration case needs its actual diagnostic'); }
  return { status, cases: report.cases.length, passed: report.cases.filter(row => row.status === 'pass').length, notRun: report.cases.filter(row => row.status === 'not-run').map(row => row.id), scope: bytesFor ? 'Authenticated original bytes; no copied inode ownership claim' : 'Original live evidence bindings', diagnostics: report.diagnostics };
}
