#!/usr/bin/env node
// Authenticated maintenance collector. It records bytes and process facts only.
// Deliberately self-contained: its sole source dependency is this harness member.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {isDeepStrictEqual as same} from 'node:util';
import {fileURLToPath} from 'node:url';

export const COLLECTOR_MEMBER = 'tools/phoenix-diagram/tests/support/capture-p3-stdio.mjs';
export const KIND = 'phoenix-diagram-p3-stdio-capture';
export const HELP =
    'Usage: node capture-p3-stdio.mjs --mode installed|migration --program ABSOLUTE --cwd PROJECT --candidate-version VERSION_JSON --stdout NEW --stderr NEW --evidence-dir NEW_DIRECTORY -- PD_ARGV...\nRecords original stdout/stderr and independent before/after installation snapshots; no semantic or visual verdict.\n';
const hash = bytes => ({sha256: createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length});
const encode = value => Buffer.from(JSON.stringify(value, null, 2) + '\n');
function need(value, message) {
  if (!value) throw new Error('P3 stdio capture: ' + message);
}
function physical(stat) {
  return {
    device: String(stat.dev),
    inode: String(stat.ino),
    links: Number(stat.nlink),
    mode: Number(stat.mode & 0o7777n),
    uid: Number(stat.uid)
  };
}
function snapshot(file) {
  need(
      typeof file === 'string' && path.isAbsolute(file) && path.normalize(file) === file &&
          !/[\0\r\n]/.test(file),
      'canonical absolute path required');
  const before = fs.lstatSync(file, {bigint: true});
  need(
      before.isFile() && !before.isSymbolicLink() && before.nlink === 1n &&
          Number(before.uid) === process.getuid() && fs.realpathSync(file) === file,
      'owned single-link ordinary file required');
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK);
  try {
    const opened = fs.fstatSync(fd, {bigint: true}), content = fs.readFileSync(fd),
          after = fs.fstatSync(fd, {bigint: true}), current = fs.lstatSync(file, {bigint: true});
    need(
        same(physical(before), physical(opened)) && same(physical(opened), physical(after)) &&
            same(physical(after), physical(current)) && opened.mtimeNs === after.mtimeNs &&
            opened.ctimeNs === after.ctimeNs && after.size === BigInt(content.length),
        'file changed while reading');
    return {path: file, identity: hash(content), binding: physical(after), content};
  } finally {
    fs.closeSync(fd);
  }
}
function ref({content, ...value}) {
  return value;
}
function unchanged(file) {
  need(same(ref(snapshot(file.path)), ref(file)), 'original file changed');
}
function directory(file, privateOnly = false) {
  const stat = fs.lstatSync(file, {bigint: true});
  need(
      stat.isDirectory() && !stat.isSymbolicLink() && Number(stat.uid) === process.getuid() &&
          fs.realpathSync(file) === file && (!privateOnly || Number(stat.mode & 0o7777n) === 0o700),
      'owned canonical directory required; output parent must be 0700');
  const {links, ...value} = physical(stat);
  return value;
}
function exists(file) {
  try {
    fs.lstatSync(file);
    return true;
  } catch (error) {
    if (error.code === 'ENOENT') return false;
    throw error;
  }
}
function write(file, bytes) {
  const fd = fs.openSync(
      file, fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW,
      0o600);
  try {
    fs.writeFileSync(fd, bytes);
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
  return ref(snapshot(file));
}
function observe(project, file, save) {
  let ancestor = path.dirname(file);
  while (!exists(ancestor)) {
    need(ancestor.startsWith(project + path.sep), 'absent slot leaves project');
    ancestor = path.dirname(ancestor);
  }
  const parent = {path: ancestor, binding: directory(ancestor)};
  if (!exists(file)) return {path: file, state: 'absent', parent};
  const original = snapshot(file);
  return {
    path: file,
    state: 'present',
    parent,
    identity: original.identity,
    binding: original.binding,
    snapshot: write(save, original.content)
  };
}
function installation(project, out, phase) {
  const names = [
    '.phoenix-diagram/install/active.json',
    ...['.agents', '.claude', '.codex'].map(host => `${host}/skills/phoenix-diagram/scripts/phoenix-diagram`)
  ];
  return {
    active: observe(project, path.join(project, names[0]), path.join(out, `${phase}-active.json`)),
    launchers: names.slice(1).map(
        (name, index) =>
            observe(project, path.join(project, name), path.join(out, `${phase}-launcher-${index}`)))
  };
}
function originalState(value) {
  const copy = structuredClone(value);
  for (const row of [copy.active, ...copy.launchers]) delete row.snapshot;
  return copy;
}
export function parseCaptureArgs(argv) {
  const result = {}, names = {
    '--mode': 'mode',
    '--program': 'program',
    '--cwd': 'cwd',
    '--candidate-version': 'candidateVersion',
    '--stdout': 'stdout',
    '--stderr': 'stderr',
    '--evidence-dir': 'evidenceDir'
  };
  let at = 0;
  for (; at < argv.length && argv[at] !== '--'; at += 2) {
    const key = names[argv[at]], value = argv[at + 1];
    need(
        key && !Object.hasOwn(result, key) && typeof value === 'string' && value && !/[\0\r\n]/.test(value),
        'unique explicit options required');
    need(
        key === 'mode' || path.isAbsolute(value) && path.normalize(value) === value,
        'absolute paths required');
    result[key] = value;
  }
  need(
      Object.keys(result).length === Object.keys(names).length &&
          ['installed', 'migration'].includes(result.mode) && argv[at] === '--' && argv.length > at + 1,
      'all capture options and explicit PD argv required');
  result.argv = argv.slice(at + 1);
  need(result.argv.every(value => typeof value === 'string' && !/[\0\r\n]/.test(value)), 'invalid PD argv');
  return result;
}
export async function captureStdio(options) {
  const project = directory(options.cwd), program = snapshot(options.program),
        version = snapshot(options.candidateVersion), candidate = JSON.parse(version.content),
        self = snapshot(fileURLToPath(import.meta.url));
  need((program.binding.mode & 0o111) !== 0, 'program must be executable');
  need(
      ['installed', 'migration'].includes(options.mode) && Array.isArray(options.argv) && options.argv.length,
      'capture mode/argv missing');
  need(
      options.mode !== 'migration' || options.argv[0] === 'migrate',
      'migration capture only executes migrate');
  need(
      options.stdout !== options.stderr &&
          new Set([options.stdout, options.stderr, options.evidenceDir]).size === 3,
      'capture slots must be distinct');
  const parents = new Map();
  for (const file of [options.stdout, options.stderr, options.evidenceDir]) {
    need(
        file.startsWith(options.cwd + path.sep) &&
            !file.startsWith(path.join(options.cwd, '.phoenix-diagram') + path.sep) && !exists(file),
        'use new project-local capture slots outside installation state');
    parents.set(path.dirname(file), directory(path.dirname(file), true));
  }
  fs.mkdirSync(options.evidenceDir, {mode: 0o700});
  const outBinding = directory(options.evidenceDir, true),
        before = installation(options.cwd, options.evidenceDir, 'before');
  if (options.mode === 'installed') {
    need(before.active.state === 'present', 'installed active receipt missing');
    const active = JSON.parse(fs.readFileSync(before.active.snapshot.path)),
          relative = path.relative(options.cwd, options.program),
          owner = active.ownership?.find(row => row.path === relative);
    need(
        /^\.(?:agents|claude)\/skills\/phoenix-diagram\/scripts\/phoenix-diagram$/.test(relative) &&
            active.schemaVersion === 2 && active.tool === 'phoenix-diagram' &&
            active.project?.device === project.device && active.project.inode === project.inode && owner &&
            Object.entries({...program.binding, ...program.identity})
                .every(([key, value]) => owner.identity[key] === value),
        'installed launcher ownership differs');
    need(
        active.bundleManifestSha256 === candidate.bundleSha256 &&
            active.engine?.sourceInputSha256 === candidate.sourceInputSha256 &&
            Object.values(candidate.engines || {})
                .some(row => same(active.engine.binary, {sha256: row.sha256, bytes: row.size})),
        'installed candidate differs');
  } else
    need(
        Object.values(candidate.engines || {})
            .some(row => same(program.identity, {sha256: row.sha256, bytes: row.size})),
        'migration program is not the fixed candidate engine');
  const candidateVersion = write(path.join(options.evidenceDir, 'candidate-version.json'), version.content),
        fds = [], startedAt = new Date().toISOString();
  let child;
  try {
    for (const file of [options.stdout, options.stderr])
      fds.push(fs.openSync(
          file, fs.constants.O_RDWR | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW,
          0o600));
    const created = fds.map(fd => physical(fs.fstatSync(fd, {bigint: true})));
    const result = await new Promise((resolve, reject) => {
      child =
          spawn(options.program, options.argv, {cwd: options.cwd, stdio: ['ignore', ...fds], detached: true});
      const kill = signal => {
        if (child.exitCode === null && child.signalCode === null) try {
            process.kill(-child.pid, signal);
          } catch (error) {
            if (error.code !== 'ESRCH') reject(error);
          }
      };
      const interrupt = () => kill('SIGTERM'), timer = setTimeout(() => kill('SIGKILL'), 300000),
            cleanup = () => {
              clearTimeout(timer);
              process.removeListener('SIGTERM', interrupt);
              process.removeListener('SIGINT', interrupt);
            };
      process.once('SIGTERM', interrupt);
      process.once('SIGINT', interrupt);
      child.once('error', error => {
        cleanup();
        reject(error);
      });
      child.once('close', (exitCode, signal) => {
        cleanup();
        resolve({exitCode, signal});
      });
    });
    for (const fd of fds) fs.fsyncSync(fd);
    const streams = [snapshot(options.stdout), snapshot(options.stderr)];
    need(streams.every((file, index) => same(file.binding, created[index])), 'captured stream slot changed');
    for (const [parent, binding] of parents)
      need(same(directory(parent, true), binding), 'capture parent changed');
    need(
        same(directory(options.cwd), project) && same(directory(options.evidenceDir, true), outBinding),
        'project/evidence directory changed');
    unchanged(program);
    unchanged(version);
    unchanged(self);
    const after = installation(options.cwd, options.evidenceDir, 'after');
    if (options.mode === 'installed')
      need(same(originalState(before), originalState(after)), 'installation changed during ordinary command');
    const dto = {
      schemaVersion: 1,
      kind: KIND,
      tool: 'phoenix-diagram',
      mode: options.mode,
      collector: ref(self),
      node: {path: fs.realpathSync(process.execPath), version: process.version},
      program: ref(program),
      argv: options.argv,
      cwd: options.cwd,
      project,
      candidateVersion,
      installationBefore: before,
      installationAfter: after,
      startedAt,
      finishedAt: new Date().toISOString(),
      ...result,
      stdout: ref(streams[0]),
      stderr: ref(streams[1])
    };
    const captureFile = write(path.join(options.evidenceDir, 'capture.json'), encode(dto));
    return {...dto, captureFile};
  } finally {
    for (const fd of fds) fs.closeSync(fd);
  }
}
export async function main(argv = process.argv.slice(2)) {
  if (argv.length === 1 && argv[0] === '--help') {
    process.stdout.write(HELP);
    return 0;
  }
  try {
    const result = await captureStdio(parseCaptureArgs(argv));
    process.stdout.write(JSON.stringify(result) + '\n');
    return result.exitCode ?? 4;
  } catch (error) {
    process.stderr.write(error.message + '\n');
    return 4;
  }
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  process.exitCode = await main();
