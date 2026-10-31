#!/usr/bin/env node
// The native artifact and fixed-copy pattern follows the existing
// scripts/test-phoenix-ui.mjs installation group, without its full runtime run.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const uiWorkspace = path.join(root, 'tools/phoenix-ui');
const HELP = 'Usage: node scripts/test-phoenix-diagram-install.mjs [--help]\nRun the Phoenix UI installation integration group using its already prepared tools and dependency cache. The stage builds one locked/offline UI binary and runs test/phoenix-ui/install.test.js with that fixed binary.\n';

export function selectUiCompilerArtifact(stdout) {
  let records;
  try { records = stdout.split(/\r?\n/).filter(line => line.trim()).map(line => JSON.parse(line)); }
  catch (error) { throw new Error(`Cargo UI output is not valid JSON: ${error.message}`); }
  const finished = records.filter(record => record.reason === 'build-finished');
  assert(finished.length === 1 && finished[0].success === true, 'Cargo UI build did not finish successfully');
  const artifacts = records.filter(record => record.reason === 'compiler-artifact' && record.target?.name === 'phoenix-ui'
    && record.target.kind?.includes('bin') && record.profile?.test !== true
    && record.manifest_path === path.join(uiWorkspace, 'crates/cli/Cargo.toml'));
  assert.equal(artifacts.length, 1, 'Cargo must report exactly one native UI bin artifact');
  const executable = artifacts[0].executable;
  assert(typeof executable === 'string' && path.isAbsolute(executable), 'Cargo UI executable must be an absolute path');
  return executable;
}

const sameSlot = (left, right) => left.dev === right.dev && left.ino === right.ino && left.birthtimeNs === right.birthtimeNs;
function executableSnapshot(file) {
  assert(typeof file === 'string' && path.isAbsolute(file), 'UI artifact must be an absolute path');
  const entry = fs.lstatSync(file, { bigint: true });
  assert(entry.isFile() && !entry.isSymbolicLink() && (entry.mode & 0o111n) !== 0n, 'UI artifact must be an executable ordinary, non-symbolic file');
  const fd = fs.openSync(file, fs.constants.O_RDONLY | (fs.constants.O_NOFOLLOW || 0) | (fs.constants.O_NONBLOCK || 0));
  try {
    const before = fs.fstatSync(fd, { bigint: true }); assert(sameSlot(entry, before), 'UI artifact binding changed before read');
    const bytes = fs.readFileSync(fd), after = fs.fstatSync(fd, { bigint: true }), current = fs.lstatSync(file, { bigint: true });
    for (const key of ['dev', 'ino', 'mode', 'nlink', 'size', 'mtimeNs', 'ctimeNs', 'birthtimeNs']) assert(before[key] === after[key] && after[key] === current[key], 'UI artifact binding changed during read');
    return { bytes, stat: after };
  } finally { fs.closeSync(fd); }
}

export function pinUiEngine(executable, { parent = os.tmpdir() } = {}) {
  const source = executableSnapshot(executable);
  const directory = fs.mkdtempSync(path.join(fs.realpathSync(parent), 'phoenix-diagram-ui-install-')), owned = fs.lstatSync(directory, { bigint: true });
  const file = path.join(directory, process.platform === 'win32' ? 'phoenix-ui.exe' : 'phoenix-ui');
  let created, fd;
  const verifyDirectory = () => { const stat = fs.lstatSync(directory, { bigint: true }); assert(stat.isDirectory() && !stat.isSymbolicLink() && sameSlot(stat, owned), 'UI snapshot directory ownership changed; retained'); };
  try {
    fd = fs.openSync(file, fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_WRONLY, 0o700); created = fs.fstatSync(fd, { bigint: true });
    fs.writeFileSync(fd, source.bytes); fs.fchmodSync(fd, 0o700); fs.fsyncSync(fd); fs.closeSync(fd); fd = undefined;
    const pinned = executableSnapshot(file); assert(pinned.bytes.equals(source.bytes), 'UI snapshot bytes differ from the compiler artifact');
    const verify = () => { verifyDirectory(); const current = executableSnapshot(file); assert(sameSlot(current.stat, pinned.stat) && current.stat.nlink === 1n && current.bytes.equals(pinned.bytes), 'UI snapshot file binding changed; retained'); };
    return { path: file, identity: { bytes: pinned.bytes.length, sha256: createHash('sha256').update(pinned.bytes).digest('hex') }, verify,
      cleanup() { verify(); assert.deepEqual(fs.readdirSync(directory), [path.basename(file)], 'Unknown UI snapshot entries retained'); fs.unlinkSync(file); fs.rmdirSync(directory); } };
  } catch (error) {
    if (fd !== undefined) fs.closeSync(fd);
    try { verifyDirectory(); if (created && sameSlot(fs.lstatSync(file, { bigint: true }), created)) fs.unlinkSync(file); fs.rmdirSync(directory); }
    catch (cleanup) { error.message += `; owned snapshot retained: ${cleanup.message}`; }
    throw error;
  }
}

export function childExitCode(result) { return !result?.error && Number.isInteger(result?.status) && result.status >= 0 && result.status <= 255 ? result.status : 4; }

export async function runInstallStage() {
  const record = { schemaVersion: 1, tool: 'phoenix-diagram', toolVersion: '0.2.0', command: 'test-phoenix-diagram', stage: 'install', status: 'failed',
    scope: 'Phoenix UI installation integration only', phases: { preflight: 'not-run', build: 'not-run', snapshot: 'not-run', install: 'not-run', cleanup: 'not-run' }, exitCode: 4, diagnostics: [] };
  let pinned, phase = 'preflight';
  try {
    // Deliberately lazy: --help and invalid arguments need no prepared UI files.
    const { preflight, buildEnvironment, offlineNetworkProfile } = await import('./phoenix-build-tools.mjs');
    const prepared = preflight(), env = buildEnvironment(prepared); record.phases.preflight = 'pass';
    phase = 'build';
    const args = ['exec', '--no-deps', '--', 'mbx', 'build', '--manifest-path', path.join(uiWorkspace, 'Cargo.toml'), '-p', 'phoenix-ui', '--bin', 'phoenix-ui', '--locked', '--offline', '--message-format=json'];
    const result = process.platform === 'darwin'
      ? spawnSync('sandbox-exec', ['-p', offlineNetworkProfile, 'mise', ...args], { cwd: uiWorkspace, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], maxBuffer: 64 * 1024 * 1024 })
      : spawnSync('mise', args, { cwd: uiWorkspace, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], maxBuffer: 64 * 1024 * 1024 });
    record.build = { exitCode: result.status, signal: result.signal, error: result.error?.message };
    record.exitCode = childExitCode(result);
    if (record.exitCode !== 0) { record.phases.build = 'failed'; return record; }
    record.exitCode = 4;
    const executable = selectUiCompilerArtifact(result.stdout); record.phases.build = 'pass';
    phase = 'snapshot'; pinned = pinUiEngine(executable); record.engine = { source: executable, snapshot: pinned.path, identity: pinned.identity }; record.phases.snapshot = 'pass';
    phase = 'install'; pinned.verify();
    const tested = spawnSync(process.execPath, ['--test', path.join(root, 'test/phoenix-ui/install.test.js')], { cwd: root, env: { ...env, PHOENIX_UI_BIN: pinned.path }, stdio: 'inherit' });
    record.install = { exitCode: tested.status, signal: tested.signal, error: tested.error?.message }; record.exitCode = childExitCode(tested);
    pinned.verify(); record.phases.install = record.exitCode === 0 ? 'pass' : 'failed';
  } catch (error) { record.phases[phase] = 'failed'; record.diagnostics.push({ phase, message: error.message }); if (record.exitCode === 0) record.exitCode = 4; }
  finally {
    if (pinned) try { pinned.cleanup(); record.phases.cleanup = 'pass'; }
    catch (error) { record.phases.cleanup = 'failed'; record.diagnostics.push({ phase: 'cleanup', message: error.message }); if (record.exitCode === 0) record.exitCode = 4; }
    record.status = record.exitCode === 0 ? 'pass' : 'failed';
  }
  return record;
}

export async function main(argv = process.argv.slice(2)) {
  if (argv.length === 1 && argv[0] === '--help') { process.stdout.write(HELP); return 0; }
  if (argv.length) { process.stderr.write(HELP); return 2; }
  const result = await runInstallStage(); process.stdout.write(`${JSON.stringify(result)}\n`); return result.exitCode;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) process.exitCode = await main();
