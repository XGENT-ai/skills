#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Adapted from scripts/pack-phoenix-ui-release.mjs and the tar reader in
// scripts/check-phoenix-deps.mjs. Phoenix changes: isolated diagram candidate
// assembly, four native build receipts, frozen loopback URLs and no publishing.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { gunzipSync } from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { validateDevelopmentArtifact, gitIdentity, sourceInputs, readSourceInputManifest } from './build-phoenix-diagram.mjs';

export const TARGETS = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64'];
export const DEFAULT_PORT = 48765;
const triples = ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'];
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const overrides = ['dist/phoenix-diagram/VERSION.json', 'dist/phoenix-diagram/bundle/ENGINE.json', 'skills/phoenix-diagram/ENGINE.json'];
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const json = value => Buffer.from(JSON.stringify(value, null, 2) + '\n');
const digest = bytes => ({ size: bytes.length, sha256: sha(bytes) });
const sorted = values => [...values].sort((a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b)));
const safe = name => typeof name === 'string' && name.length > 0 && !/[\\\0:]/.test(name) && name.split('/').every(part => part && part !== '.' && part !== '..' && !/[. ]$/.test(part));
function portNumber(port) { assert(Number.isInteger(port) && port > 0 && port < 65536, 'Candidate port must be a fixed integer from 1 to 65535'); return port; }
function directory(file) { const stat = fs.lstatSync(file); assert(stat.isDirectory() && !stat.isSymbolicLink(), `Expected a regular directory, not a symlink: ${file}`); }
function regular(file) {
  const stat = fs.lstatSync(file);
  assert(stat.isFile() && !stat.isSymbolicLink() && stat.nlink === 1, `Expected a singly linked regular file: ${file}`);
  return stat;
}
function read(file) { regular(file); return fs.readFileSync(file); }
function command(program, args, options = {}) {
  const result = spawnSync(program, args, { encoding: 'utf8', maxBuffer: 256 * 1024 * 1024, ...options });
  assert(!result.error && result.status === 0, `${program}: ${result.error?.message || result.stderr?.toString() || `exit ${result.status}`}`);
  return result.stdout;
}
function equalMaps(left, right, label) {
  assert(left instanceof Map && right instanceof Map, `${label} byte inventory is missing`);
  assert.deepEqual(sorted(left.keys()), sorted(right.keys()), `${label} inventory differs across platforms`);
  for (const [name, bytes] of left) assert(Buffer.isBuffer(bytes) && bytes.equals(right.get(name)), `${label} bytes differ: ${name}`);
}
function legalDigest(files) { return sha(Buffer.from(JSON.stringify(sorted(files.keys()).map(name => [name, files.get(name).length, sha(files.get(name))])))); }

// This metadata boundary is also useful for rejection/schema tests. Production
// assembly obtains every record only through validateDevelopmentArtifact below.
export function assertArtifactAgreement(artifacts, expected = {}) {
  assert(Array.isArray(artifacts) && artifacts.length === 4, 'Exactly four native platform artifacts are required');
  const byTarget = new Map();
  for (const artifact of artifacts) {
    assert(TARGETS.includes(artifact?.target) && !byTarget.has(artifact.target), 'Unknown or duplicate platform artifact');
    byTarget.set(artifact.target, artifact);
  }
  const ordered = TARGETS.map(target => { assert(byTarget.has(target), `Missing platform ${target}`); return byTarget.get(target); });
  const first = ordered[0], firstReceipt = first.receipt;
  for (const [index, artifact] of ordered.entries()) {
    const { target, version, receipt, binary, bundle, licenses, sourceInputs: inputs } = artifact;
    assert(receipt?.schemaVersion === 1 && receipt.tool === 'phoenix-diagram' && receipt.distribution === 'development' && receipt.target === target, `Invalid native build receipt ${target}`);
    assert(receipt.sourceState === 'clean' && receipt.sourceReference === receipt.sourceCommit && /^[a-f0-9]{40}$/.test(receipt.sourceCommit) && /^[a-f0-9]{40}$/.test(receipt.sourceTree), `Candidate source must be a clean committed tree: ${target}`);
    assert.deepEqual([receipt.sourceCommit, receipt.sourceTree, receipt.sourceInputSha256], [firstReceipt.sourceCommit, firstReceipt.sourceTree, firstReceipt.sourceInputSha256], `Source commit/tree/input differs: ${target}`);
    assert(inputs && Buffer.isBuffer(inputs.bytes) && sha(inputs.bytes) === inputs.sha256 && inputs.sha256 === receipt.sourceInputSha256 && Buffer.from(JSON.stringify(inputs.entries)).equals(inputs.bytes), `Source input tuple bytes differ: ${target}`);
    assert(inputs.bytes.equals(first.sourceInputs.bytes), `Source input bytes differ across platforms: ${target}`);
    assert(version?.schemaVersion === 1 && version.distribution === 'development' && version.tool === 'phoenix-diagram' && version.toolVersion === '0.2.0' && version.inputContractSchema === 1 && version.receiptSchema === 1 && version.viewerRevision === 'phoenix-v2', `Invalid development version/contracts: ${target}`);
    assert(version.sourceCommit === receipt.sourceCommit && version.sourceInputSha256 === receipt.sourceInputSha256 && receipt.toolVersion === version.toolVersion && receipt.npmPackageVersion === version.npmPackageVersion, `Package/version/source identity differs: ${target}`);
    const { engines, ...identity } = version, { engines: firstEngines, ...firstIdentity } = first.version;
    assert.deepEqual(identity, firstIdentity, `Package/version/bundle identity differs: ${target}`);
    assert.deepEqual(Object.keys(engines), [target], `Development engine must name only its native platform: ${target}`);
    assert(Buffer.isBuffer(binary?.bytes) && binary.asset === `phoenix-diagram-${target}` && binary.size === binary.bytes.length && binary.sha256 === sha(binary.bytes), `Actual binary digest/size differs: ${target}`);
    const binaryIdentity = { asset: binary.asset, size: binary.size, sha256: binary.sha256 };
    assert.deepEqual(receipt.binary, binaryIdentity, `Receipt binary identity differs: ${target}`);
    assert.deepEqual({ asset: engines[target].asset, size: engines[target].size, sha256: engines[target].sha256 }, binaryIdentity, `Version binary identity differs: ${target}`);
    assert(Buffer.isBuffer(bundle?.bytes) && sha(bundle.bytes) === bundle.sha256 && bundle.sha256 === version.bundleSha256 && receipt.bundle?.sha256 === bundle.sha256 && receipt.bundle.size === bundle.bytes.length, `Bundle manifest bytes differ: ${target}`);
    assert(bundle.bytes.equals(first.bundle.bytes), `Bundle manifest differs across platforms: ${target}`);
    equalMaps(bundle.files, first.bundle.files, 'Bundle');
    assert(licenses instanceof Map && licenses.size > 0 && artifact.licenseSha256 === legalDigest(licenses) && receipt.legal?.sha256 === artifact.licenseSha256, `License byte inventory digest differs: ${target}`);
    equalMaps(licenses, first.licenses, 'License');
    assert.deepEqual(receipt.viewer, firstReceipt.viewer, `Viewer identity differs: ${target}`);
    assert(receipt.viewer?.revision === version.viewerRevision, `Viewer contract differs: ${target}`);
    const tools = receipt.tools;
    assert(tools?.pins && tools.observed && tools.sources, `Native build tools are missing: ${target}`);
    for (const key of ['pins', 'observed', 'sources']) assert.deepEqual(tools[key], firstReceipt.tools[key], `Pinned/observed tools differ: ${target}`);
    assert(tools.observed.rust === tools.pins.rust && tools.observed.cargo === tools.pins.rust && tools.observed.node === tools.pins.node && tools.observed.mrBoxington === tools.pins.mrBoxington, `Native observed tool versions differ from pins: ${target}`);
    const native = receipt.native, triple = triples[index];
    assert(receipt.host === triple && native?.host === triple && native.target === triple && native.status === 'pass' && native.sourceInputsMatch === 'pass', `Native host/target/source evidence differs: ${target}`);
    assert(native.engineProbe?.status === 'pass' && native.engineProbe.stdout === 'phoenix-diagram-engine 0.2.0 input=1 receipt=1 viewer=phoenix-v2\n', `Native probe evidence differs: ${target}`);
    const engine = native.engineDoctor?.engine;
    assert(native.engineDoctor?.status === 'pass' && native.engineDoctor.toolVersion === version.toolVersion && engine?.sourceInputSha256 === inputs.sha256 && engine.inputContractSchema === 1 && engine.receiptSchema === 1 && engine.viewerRevision === 'phoenix-v2', `Native doctor source/contract differs: ${target}`);
    assert.deepEqual(engine.binary, { sha256: binary.sha256, bytes: binary.size }, `Native doctor binary differs: ${target}`);
    for (const check of ['lockedOffline', 'sourceStable', 'engineProbe', 'sourceInputs']) assert(receipt.checks?.[check] === 'pass', `Native build check ${check} is not pass: ${target}`);
  }
  if (expected.git) assert(expected.git.sourceState === 'clean' && expected.git.sourceCommit === firstReceipt.sourceCommit && expected.git.sourceTree === firstReceipt.sourceTree, 'Checkout source must match the same clean committed HEAD/tree');
  if (expected.sourceInputs) assert(expected.sourceInputs.bytes.equals(first.sourceInputs.bytes), 'Checkout source inputs differ from native builds');
  if (expected.npmPackageVersion) assert.equal(first.version.npmPackageVersion, expected.npmPackageVersion, 'Owning npm package version differs');
  return { artifacts: ordered, source: { commit: firstReceipt.sourceCommit, tree: firstReceipt.sourceTree, inputSha256: firstReceipt.sourceInputSha256 }, npmPackageVersion: first.version.npmPackageVersion, toolVersion: first.version.toolVersion };
}
export function candidateManifest(artifacts, { port = DEFAULT_PORT } = {}) {
  portNumber(port);
  const agreement = assertArtifactAgreement(artifacts);
  return { ...agreement.artifacts[0].version, distribution: 'candidate', engines: Object.fromEntries(agreement.artifacts.map(({ target, binary }) => [target, { asset: binary.asset, size: binary.size, sha256: binary.sha256, url: `http://127.0.0.1:${port}/${binary.asset}` }])) };
}
export function validateCandidateInputs({ repoRoot = root, artifactsDir, port = DEFAULT_PORT } = {}) {
  portNumber(port); assert(artifactsDir, 'Missing four-platform artifacts directory');
  repoRoot = path.resolve(repoRoot); artifactsDir = path.resolve(artifactsDir);
  directory(artifactsDir);
  assert.deepEqual(sorted(fs.readdirSync(artifactsDir)), sorted(TARGETS), 'Artifacts directory must contain exactly the four native platform directories');
  for (const target of TARGETS) directory(path.join(artifactsDir, target));
  const artifacts = TARGETS.map(target => {
    const artifact = validateDevelopmentArtifact(path.join(artifactsDir, target), { repoRoot, requireClean: true });
    assert.equal(artifact.target, target, 'Artifact directory and native target differ');
    const receiptBytes = read(path.join(artifactsDir, target, 'build-receipt.json'));
    assert.deepEqual(JSON.parse(receiptBytes), artifact.receipt, 'Build receipt changed during validation');
    return { ...artifact, receiptBytes };
  });
  const pkg = JSON.parse(read(path.join(repoRoot, 'package.json')));
  assert.equal(pkg.name, '@xgent-ai/skills', 'Unexpected owning npm package');
  const git = gitIdentity(repoRoot), inputs = sourceInputs(repoRoot);
  const agreement = assertArtifactAgreement(artifacts, { git, sourceInputs: inputs, npmPackageVersion: pkg.version });
  assert.equal(process.version, `v${artifacts[0].receipt.tools.pins.node}`, 'Pack with the pinned maintenance Node');
  return { ...agreement, repoRoot, artifactsDir, port, pkg, git, inputs };
}

function forbidden(name) {
  return /^(?:tools|local|node_modules)\//.test(name) || name.split('/').some(part => ['.git', 'node_modules', '.ssh', '.npmrc', '.netrc', 'credentials.json', 'id_rsa', 'id_ed25519'].includes(part) || /^\.env(?:\.|$)/.test(part)) || TARGETS.some(target => path.posix.basename(name) === `phoenix-diagram-${target}`);
}
function maintainedSources(repoRoot, pkg, commit) {
  assert(Array.isArray(pkg.files) && pkg.files.length && pkg.files.every(name => safe(name) && !/[*?\[\]{}]/.test(name) && !forbidden(name)), 'Unsupported or unsafe package.files allowlist');
  assert(pkg.bin && typeof pkg.bin === 'object' && !Array.isArray(pkg.bin) && Object.values(pkg.bin).every(safe), 'Invalid npm bin mapping');
  for (const hook of ['prepack', 'prepare', 'postpack']) assert(!pkg.scripts?.[hook], `Unexpected pack lifecycle ${hook}; candidate packing does not build`);
  const tree = command('git', ['ls-tree', '-rz', '--full-tree', commit], { cwd: repoRoot }).split('\0').filter(Boolean);
  const selected = tree.map(line => {
    const match = /^(\d+) blob ([a-f0-9]{40})\t([\s\S]+)$/.exec(line);
    return match && { mode: parseInt(match[1], 8), object: match[2], name: match[3] };
  }).filter(entry => entry && (['package.json', '.gitignore'].includes(entry.name) || !entry.name.includes('/') && /^(?:readme|licen[sc]e|copying)(?:\..*)?$/i.test(entry.name) || pkg.files.some(name => entry.name === name || entry.name.startsWith(`${name}/`))));
  assert(selected.length, 'No maintained npm package sources');
  for (const entry of selected) assert(safe(entry.name) && !forbidden(entry.name) && [0o100644, 0o100755].includes(entry.mode), `Unsafe or nonregular maintained package source ${entry.name}`);
  const batch = command('git', ['cat-file', '--batch'], { cwd: repoRoot, input: selected.map(entry => entry.object).join('\n') + '\n', encoding: null });
  const result = new Map(); let cursor = 0;
  for (const entry of selected) {
    const newline = batch.indexOf(10, cursor), header = batch.subarray(cursor, newline).toString().split(' '), size = Number(header[2]);
    assert(header[0] === entry.object && header[1] === 'blob' && Number.isSafeInteger(size) && size >= 0, 'Invalid Git blob batch');
    const bytes = batch.subarray(newline + 1, newline + 1 + size); cursor = newline + 2 + size;
    assert(bytes.length === size && batch[cursor - 1] === 10, 'Truncated maintained Git blob');
    if (!overrides.includes(entry.name)) assert(read(path.join(repoRoot, entry.name)).equals(bytes), `Maintained package byte differs from clean HEAD: ${entry.name}`);
    result.set(entry.name, { bytes, mode: entry.mode & 0o777 });
  }
  assert.equal(cursor, batch.length);
  for (const name of ['package.json', 'docs/phoenix-diagram.md', ...Object.values(pkg.bin)]) assert(result.has(name), `Missing maintained npm entry ${name}`);
  return result;
}
function runtimeSources(artifact) {
  const files = new Map();
  for (const [name, bytes] of artifact.bundle.files) {
    files.set(`dist/phoenix-diagram/bundle/${name}`, bytes);
    files.set(`skills/phoenix-diagram/${name}`, bytes);
  }
  files.set('dist/phoenix-diagram/bundle/manifest.json', artifact.bundle.bytes);
  for (const [name, bytes] of artifact.licenses) files.set(`dist/phoenix-diagram/${name}`, bytes);
  return files;
}
export function assertRuntimeSources(sources, artifact) {
  const runtime = runtimeSources(artifact);
  for (const [name, bytes] of runtime) assert(sources.get(name)?.bytes.equals(bytes), `Shared diagram package input is missing or differs; freeze it before packing: ${name}`);
  for (const name of sources.keys()) if (name.startsWith('dist/phoenix-diagram/') || name.startsWith('skills/phoenix-diagram/')) assert(runtime.has(name) || overrides.includes(name), `Extra or unlisted diagram runtime input: ${name}`);
  return runtime;
}
function write(file, bytes, mode = 0o644) { fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, bytes, { flag: 'wx', mode }); fs.chmodSync(file, mode); }
function inventory(rootDir, excluded = new Set()) {
  const entries = [];
  function visit(prefix) {
    const current = prefix ? path.join(rootDir, prefix) : rootDir; directory(current);
    for (const name of fs.readdirSync(current)) {
      const relative = prefix ? `${prefix}/${name}` : name;
      assert(safe(relative), `Unsafe candidate path ${relative}`);
      const file = path.join(rootDir, relative), stat = fs.lstatSync(file);
      if (stat.isDirectory() && !stat.isSymbolicLink()) visit(relative);
      else if (!excluded.has(relative)) entries.push({ path: relative, ...digest(read(file)) });
    }
  }
  visit(''); return entries.sort((a, b) => Buffer.compare(Buffer.from(a.path), Buffer.from(b.path)));
}
export function candidateFiles(directory, sourceInputSha256) {
  const inputs = readSourceInputManifest(read(path.join(directory, 'source-inputs.json')));
  assert.equal(inputs.sha256, sourceInputSha256, 'Candidate source input digest differs from its source identity');
  return inventory(directory, new Set(['candidate.json']));
}
export function readTarballInventory(archive) {
  const data = gunzipSync(archive, { maxOutputLength: 512 * 1024 * 1024 });
  const entries = new Map(); let nextPath;
  for (let offset = 0; offset + 512 <= data.length;) {
    const header = data.subarray(offset, offset + 512);
    if (header.every(byte => byte === 0)) { assert(data.subarray(offset).every(byte => byte === 0), 'Nonzero data after npm tar terminator'); break; }
    const string = (from, to) => header.subarray(from, to).toString().replace(/\0.*$/, '');
    const prefix = string(345, 500), name = nextPath ?? `${prefix ? `${prefix}/` : ''}${string(0, 100)}`;
    const size = parseInt(string(124, 136).trim(), 8), mode = parseInt(string(100, 108).trim(), 8), checksum = parseInt(string(148, 156).trim(), 8);
    let actualChecksum = 0; for (let i = 0; i < 512; i++) actualChecksum += i >= 148 && i < 156 ? 32 : header[i];
    assert(checksum === actualChecksum && Number.isSafeInteger(size) && size >= 0 && offset + 512 + size <= data.length && Number.isInteger(mode) && !(mode & 0o7000), 'Invalid npm tar header/checksum/size/mode');
    const body = data.subarray(offset + 512, offset + 512 + size), type = header[156];
    if ([0, 48].includes(type)) {
      assert(name.startsWith('package/') && safe(name.slice(8)) && !forbidden(name.slice(8)) && !entries.has(name.slice(8)), `Unsafe or duplicate npm tarball member ${name}`);
      entries.set(name.slice(8), { ...digest(body), mode: mode & 0o777 }); nextPath = undefined;
    } else if (type === 76) nextPath = body.toString().replace(/\0.*$/, '');
    else if (type === 120) {
      for (let at = 0; at < body.length;) {
        const space = body.indexOf(32, at), length = Number(body.subarray(at, space).toString());
        assert(Number.isSafeInteger(length) && length > space - at && at + length <= body.length, 'Invalid npm tar extended header');
        const record = body.subarray(space + 1, at + length - 1).toString();
        if (record.startsWith('path=')) nextPath = record.slice(5);
        at += length;
      }
    } else {
      const directoryName = name.replace(/\/$/, '');
      assert(type === 53 && (directoryName === 'package' || directoryName.startsWith('package/')) && safe(directoryName), `Unsupported npm tarball member type ${type}`);
      nextPath = undefined;
    }
    offset += 512 + Math.ceil(size / 512) * 512;
  }
  assert(entries.size > 0, 'Empty npm tarball'); return entries;
}
function verifyTarball(file, sources, required) {
  const entries = readTarballInventory(read(file));
  for (const [name, entry] of entries) {
    const expected = sources.get(name); assert(expected, `Unexpected packed source ${name}`);
    assert.deepEqual(entry, { ...digest(expected.bytes), mode: expected.mode }, `Packed byte or mode differs ${name}`);
  }
  for (const name of required) assert(entries.has(name), `Required package runtime absent ${name}`);
  return entries;
}
function assertInputsCurrent(validated) {
  const currentGit = gitIdentity(validated.repoRoot);
  assert.deepEqual(currentGit, validated.git, 'Checkout source changed during candidate packing');
  assert(sourceInputs(validated.repoRoot).bytes.equals(validated.inputs.bytes), 'Source inputs changed during candidate packing');
  for (const artifact of validated.artifacts) {
    assert(read(path.join(artifact.directory, 'build-receipt.json')).equals(artifact.receiptBytes), 'Native build receipt changed during candidate packing');
    assert(read(path.join(artifact.directory, 'source-inputs.json')).equals(artifact.sourceInputs.bytes), 'Native source-input tuples changed during candidate packing');
    assert(read(path.join(artifact.directory, 'VERSION.json')).equals(artifact.versionBytes), 'Development VERSION changed during candidate packing');
    assert.deepEqual(digest(read(artifact.binary.path)), { size: artifact.binary.size, sha256: artifact.binary.sha256 }, 'Native engine changed during candidate packing');
    assert(read(path.join(artifact.directory, 'bundle/manifest.json')).equals(artifact.bundle.bytes), 'Native bundle manifest changed during candidate packing');
    for (const [name, bytes] of artifact.bundle.files) assert(read(path.join(artifact.directory, 'bundle', name)).equals(bytes), `Native bundle changed: ${name}`);
    for (const [name, bytes] of artifact.licenses) assert(read(path.join(artifact.directory, name)).equals(bytes), `Native license changed: ${name}`);
  }
}
export async function packCandidate(options = {}) {
  const validated = validateCandidateInputs(options);
  assert(options.outputDir, 'Missing candidate output directory');
  const outputDir = path.resolve(options.outputDir);
  assert(!fs.existsSync(outputDir), 'Candidate output already exists; do not generate a second tarball for this candidate');
  const sources = maintainedSources(validated.repoRoot, validated.pkg, validated.source.commit);
  const runtime = assertRuntimeSources(sources, validated.artifacts[0]);
  const versionBytes = json(candidateManifest(validated.artifacts, { port: validated.port }));
  for (const name of overrides) sources.set(name, { bytes: versionBytes, mode: 0o644 });
  const temp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram candidate pack ')));
  const stage = path.join(temp, 'package');
  const record = { schemaVersion: 1, tool: 'phoenix-diagram', status: 'packing', handoff: 'pending-consumer', npmPackageVersion: validated.npmPackageVersion, toolVersion: validated.toolVersion, source: validated.source, port: validated.port, targets: TARGETS, releaseDir: 'release-dir' };
  let started = false;
  try {
    for (const [name, file] of sources) write(path.join(stage, name), file.bytes, file.mode);
    fs.mkdirSync(outputDir, { recursive: true, mode: 0o700 });
    write(path.join(outputDir, 'candidate.json'), json(record)); started = true;
    write(path.join(outputDir, 'source-inputs.json'), validated.artifacts[0].sourceInputs.bytes);
    const release = path.join(outputDir, 'release-dir');
    write(path.join(release, 'VERSION.json'), versionBytes);
    write(path.join(release, 'bundle/ENGINE.json'), versionBytes);
    write(path.join(release, 'bundle/manifest.json'), validated.artifacts[0].bundle.bytes);
    for (const [name, bytes] of validated.artifacts[0].bundle.files) write(path.join(release, 'bundle', name), bytes, validated.artifacts[0].bundle.manifest.files.find(file => file.path === name).mode);
    for (const [name, bytes] of validated.artifacts[0].licenses) write(path.join(release, name), bytes);
    for (const artifact of validated.artifacts) write(path.join(release, artifact.binary.asset), artifact.binary.bytes, 0o755);
    record.buildReceipts = validated.artifacts.map(artifact => {
      const name = `build-receipts/${artifact.target}.json`; write(path.join(outputDir, name), artifact.receiptBytes);
      return { target: artifact.target, path: name, ...digest(artifact.receiptBytes) };
    });
    const userConfig = path.join(temp, 'npm-user.npmrc'), globalConfig = path.join(temp, 'npm-global.npmrc');
    write(userConfig, ''); write(globalConfig, '');
    const env = { ...process.env };
    for (const name of Object.keys(env)) if (/^npm_config_/i.test(name) || /^(?:NODE_OPTIONS|NPM_TOKEN|NODE_AUTH_TOKEN)$/i.test(name)) delete env[name];
    Object.assign(env, { npm_config_userconfig: userConfig, npm_config_globalconfig: globalConfig, npm_config_cache: path.join(temp, 'npm-cache'), npm_config_update_notifier: 'false' });
    const npm = path.resolve(path.dirname(process.execPath), '../lib/node_modules/npm/bin/npm-cli.js'); regular(npm);
    const listing = JSON.parse(command(process.execPath, [npm, 'pack', '--offline', '--ignore-scripts', '--json', '--pack-destination', outputDir], { cwd: stage, env, timeout: 600000 }));
    assert(Array.isArray(listing) && listing.length === 1 && listing[0].filename === path.basename(listing[0].filename) && listing[0].filename.endsWith('.tgz'), 'npm must generate exactly one candidate tarball');
    const tarball = listing[0].filename;
    assert.deepEqual(fs.readdirSync(outputDir).filter(name => name.endsWith('.tgz')), [tarball], 'Multiple candidate tarballs were generated');
    const required = ['package.json', 'docs/phoenix-diagram.md', ...Object.values(validated.pkg.bin), ...runtime.keys(), ...overrides];
    const packed = verifyTarball(path.join(outputDir, tarball), sources, required);
    assert.deepEqual(listing[0].files.map(file => file.path).sort(), [...packed.keys()].sort(), 'npm listing and actual tarball inventory differ');
    for (const file of listing[0].files) assert.equal(file.size, packed.get(file.path).size, `npm size differs from actual tar member ${file.path}`);
    assertInputsCurrent(validated);
    record.tarball = { path: tarball, ...digest(read(path.join(outputDir, tarball))) };
    record.derivation = { distribution: 'candidate', version: { path: 'release-dir/VERSION.json', ...digest(versionBytes) }, bundleManifest: { path: 'release-dir/bundle/manifest.json', ...digest(validated.artifacts[0].bundle.bytes) }, stagedOverrides: overrides.map(name => ({ path: name, ...digest(versionBytes) })), parentVersions: validated.artifacts.map(artifact => ({ target: artifact.target, ...digest(artifact.versionBytes) })) };
    record.files = candidateFiles(outputDir, validated.source.inputSha256);
    record.status = 'assembled';
    fs.writeFileSync(path.join(outputDir, 'candidate.json'), json(record));
    return record;
  } catch (error) {
    if (started) fs.writeFileSync(path.join(outputDir, 'candidate.json'), json({ ...record, status: 'failed', error: error.message }));
    throw error;
  } finally { fs.rmSync(temp, { recursive: true, force: true }); }
}

const usage = 'Usage: node scripts/pack-phoenix-diagram-candidate.mjs --artifacts-dir DIR --output-dir DIR [--port PORT]\nDefault candidate port: 48765. URLs and the single tarball remain frozen; public publication and consumer acceptance are separate.\n';
export async function main(args) {
  if (args.length === 1 && ['--help', '-h'].includes(args[0])) { process.stdout.write(usage); return 0; }
  const options = {};
  for (let i = 0; i < args.length; i++) {
    const equals = args[i].indexOf('='), flag = equals < 0 ? args[i] : args[i].slice(0, equals), inline = equals < 0 ? undefined : args[i].slice(equals + 1);
    assert(['--artifacts-dir', '--output-dir', '--port'].includes(flag) && !Object.hasOwn(options, flag), usage);
    const value = inline ?? args[++i]; assert(value && !value.startsWith('--'), usage); options[flag] = value;
  }
  assert(options['--artifacts-dir'] && options['--output-dir'], usage);
  const port = options['--port'] === undefined ? DEFAULT_PORT : Number(options['--port']);
  const record = await packCandidate({ artifactsDir: options['--artifacts-dir'], outputDir: options['--output-dir'], port });
  process.stdout.write(json(record)); return 0;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main(process.argv.slice(2)).then(code => { process.exitCode = code; }).catch(error => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
