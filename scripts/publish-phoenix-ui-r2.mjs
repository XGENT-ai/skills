#!/usr/bin/env node
// Consume fixed GitHub run/jobs/artifacts JSON and archives/<artifact-name>.zip.
// Export the existing AGENT_RELEASE_R2_* variables before running; this script
// never reads .env.cf, builds an engine, packs npm, or publishes npm.
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import bootstrap from '../skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const targets = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64'];
const hosts = ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'];
const phases = {
  prepare: 'Explicit online preparation', audit: 'Audit prepared dependencies offline',
  light: 'Lightweight repository tests', build: 'Build native and WASM from the prepared cache',
  smoke: 'Native and portable Node runtime smoke', runtime: 'Full Phoenix runtime tests offline',
};
const smokeChecks = { 'engine-probe': 0, version: 0, 'root-help': 0, 'local-command-catalog': 0,
  'static-html-detect': 2, 'local-context': 0, 'hook-session-start': 0, 'node-missing-engine': 4,
  'node-explicit-offline-install': 0, 'node-engine-version': 0, 'node-context': 0, 'local-bundle-check': 0 };
const generatedInputs = ['tools/phoenix-ui/crates/live/assets/bundle-inputs.json', 'tools/phoenix-ui/crates/live/assets/detect-antipatterns-browser.js'];
const derivedFiles = ['dist/phoenix-ui/VERSION.json', 'skills/phoenix-ui/scripts/ENGINE.json', 'skills/phoenix-ui/GENERATED.json'];
// These locked target crates ship source license declarations, not standalone
// terms. Match the producer's exact fixed source shape; this is not a general
// licenseLocations fallback (and never replaces complete MPL terms).
const winapiDeclarations = {
  'winapi-i686-pc-windows-gnu': 'ac3b87c63620426dd9b991e5ce0329eff545bccbbb34f3be09ff6fb6ab51b7b6',
  'winapi-x86_64-pc-windows-gnu': '712e227841d057c1ee1cd2fb22fa7e5a5461ae8e48fa2ca79ec42cfc1931183f',
};
function declaredWinapiLicense(entry) {
  return Object.hasOwn(winapiDeclarations, entry.name) && entry.version === '0.4.0'
    && entry.license === 'MIT/Apache-2.0' && entry.checksum === winapiDeclarations[entry.name]
    && entry.sourceUnmodified === true
    && entry.sourceUrl === `https://static.crates.io/crates/${entry.name}/${entry.name}-0.4.0.crate`
    && Array.isArray(entry.licenseLocations) && entry.licenseLocations.length === 2
    && ['Cargo.toml.orig', 'src/lib.rs'].every(file => entry.licenseLocations.includes(`https://docs.rs/crate/${entry.name}/0.4.0/source/${file}`));
}
const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const json = value => JSON.stringify(value, null, 2) + '\n';
const safePath = name => typeof name === 'string' && name && !/[\\\x00:]/.test(name) && !name.split('/').some(p => !p || p === '.' || p === '..');
function bytes(file) {
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK);
  try { assert(fs.fstatSync(fd).isFile(), `Not a regular file: ${file}`); return fs.readFileSync(fd); }
  finally { fs.closeSync(fd); }
}
function directory(dir) {
  const fd = fs.openSync(dir, fs.constants.O_RDONLY | fs.constants.O_DIRECTORY | fs.constants.O_NOFOLLOW);
  try { assert(fs.fstatSync(fd).isDirectory(), `Not a directory: ${dir}`); }
  finally { fs.closeSync(fd); }
}
const readJson = file => JSON.parse(bytes(file));
const digest = file => ({ size: fs.lstatSync(file).size, sha256: sha256(bytes(file)) });
function engineDigests(generated, hash) {
  generated.outputs.skill['scripts/ENGINE.json'] = hash;
  for (const name of Object.keys(generated.outputs.providers)) if (name.endsWith('/skills/phoenix-ui/scripts/ENGINE.json')) generated.outputs.providers[name] = hash;
  return generated;
}
function inventory(dir, prefix = '') {
  return Object.fromEntries(fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name, 'en')).flatMap(entry => {
    const name = `${prefix}${entry.name}`, file = path.join(dir, entry.name);
    assert(entry.isDirectory() || entry.isFile(), `Non-regular release resource: ${name}`);
    return entry.isDirectory() ? Object.entries(inventory(file, `${name}/`)) : [[name, digest(file)]];
  }));
}
function copyArtifact(source, destination) {
  fs.cpSync(source, destination, { recursive: true });
  const directories = (from, to) => {
    if (!fs.lstatSync(from).isDirectory()) return;
    for (const name of fs.readdirSync(from)) directories(path.join(from, name), path.join(to, name));
    fs.chmodSync(to, fs.lstatSync(from).mode & 0o777);
  };
  directories(source, destination);
}
function command(program, args, env = process.env) {
  const result = spawnSync(program, args, { env, encoding: 'utf8', timeout: 600000, maxBuffer: 8 * 1024 * 1024 });
  if (result.error) throw result.error;
  return result;
}
function successful(program, args, env) {
  const result = command(program, args, env);
  assert.equal(result.status, 0, `${program} failed (exit ${result.status}): ${result.stderr.trim()}`);
  return result.stdout;
}
function extract(archive, destination) {
  // ZIP parsing/extraction belongs to Python's standard library. Validate every
  // member before extracting anything; GitHub digests bind the original ZIP.
  successful('python3', ['-c', `import sys,zipfile,stat,os
with zipfile.ZipFile(sys.argv[1]) as z:
 seen=set()
 assert sum(i.file_size for i in z.infolist()) <= 1024*1024*1024, 'Oversized artifact'
 for i in z.infolist():
  n=i.filename.rstrip('/')
  assert n and not n.startswith('/') and not any(p in ('','.','..') for p in n.split('/')) and not any(c in n for c in ('\\\\',':','\\0')), 'Unsafe archive path'
  assert n not in seen, 'Duplicate archive path'
  seen.add(n)
  assert stat.S_IFMT(i.external_attr >> 16) in (0,stat.S_IFREG,stat.S_IFDIR), 'Non-regular archive member'
  assert i.file_size <= 256*1024*1024, 'Oversized archive member'
 z.extractall(sys.argv[2])
 # Restore only ordinary rwx bits. Files first; deepest directories last so
 # restrictive parent modes cannot obstruct extraction or child chmod.
 for i in sorted(z.infolist(), key=lambda i: (i.is_dir(), -i.filename.count('/'))):
  if i.create_system == 3:
   os.chmod(os.path.join(sys.argv[2],i.filename), (i.external_attr >> 16) & 0o777)`, archive, destination]);
}
function successfulJob(jobs, options, name, steps) {
  const matches = jobs.filter(j => j.name === name);
  assert.equal(matches.length, 1, `Missing or duplicate CI job ${name}`);
  const job = matches[0];
  assert.equal(String(job.run_id), options.runId, `${name} run ID differs`);
  assert.equal(job.head_sha, options.sourceCommit, `${name} source commit differs`);
  assert(job.status === 'completed' && job.conclusion === 'success', `${name} did not succeed`);
  for (const name of steps) {
    const matches = job.steps.filter(s => s.name === name);
    assert(matches.length === 1 && matches[0].status === 'completed' && matches[0].conclusion === 'success', `Failed or missing CI step ${name}`);
  }
  return job.id;
}
function smoke(record, manifest, target, node) {
  assert(record.schemaVersion === 1 && record.status === 'passed' && record.target === target && record.mode === 'native', `Failed or missing native smoke ${target}/Node${node}`);
  assert.equal(record.host.platform, target.split('-')[0], 'Smoke host platform differs');
  assert.equal(record.host.architecture, target.split('-')[1], 'Smoke host architecture differs');
  assert.equal(record.host.node.split('.')[0], `v${node}`, 'Smoke Node version differs');
  assert.equal(record.binarySha256, manifest.data.engines[target].sha256, 'Smoke binary digest differs');
  assert.equal(record.manifestSha256, manifest.hash, 'Smoke manifest digest differs');
  for (const [name, exitCode] of Object.entries(smokeChecks)) {
    const checks = record.checks.filter(c => c.name === name);
    assert(checks.length === 1 && checks[0].exitCode === exitCode && !checks[0].error, `Failed or missing smoke check ${name}`);
  }
}

export function validateHandoff(options, staging) {
  assert(/^[1-9]\d*$/.test(options.runId), 'A fixed run ID is required');
  for (const key of ['sourceCommit', 'sourceTree']) assert(/^[a-f0-9]{40}$/.test(options[key]), `A fixed ${key} is required`);
  const run = readJson(options.runRecord), jobRecord = readJson(options.jobsRecord), artifactRecord = readJson(options.artifactsRecord);
  assert.equal(String(run.id), options.runId, 'Run ID differs');
  assert.equal(run.head_sha, options.sourceCommit, 'Run source commit differs');
  assert(run.status === 'completed' && run.conclusion === 'success', 'CI run did not succeed');
  assert.equal(run.repository.full_name, 'XGENT-ai/skills', 'Run repository differs');
  assert.equal(run.head_repository.full_name, 'XGENT-ai/skills', 'Run head repository differs');
  assert.equal(run.repository.id, run.head_repository.id, 'Run head repository ID differs');
  assert.equal(jobRecord.total_count, jobRecord.jobs.length, 'Incomplete CI jobs record');
  assert.equal(artifactRecord.total_count, artifactRecord.artifacts.length, 'Incomplete CI artifacts record');
  directory(options.archivesDir);
  const artifactNames = targets.flatMap(t => [`phoenix-ui-${t}`, `phoenix-ui-${t}-evidence`]).concat('phoenix-ui-node18-consumer-evidence');
  const artifacts = [];
  for (const name of artifactNames) {
    const found = artifactRecord.artifacts.filter(a => a.name === name);
    assert.equal(found.length, 1, `Missing or duplicate platform artifact ${name}`);
    const a = found[0];
    assert(!a.expired, `Expired artifact ${name}`);
    assert.equal(String(a.workflow_run.id), options.runId, `${name} artifact run ID differs`);
    assert.equal(a.workflow_run.head_sha, options.sourceCommit, `${name} artifact source commit differs`);
    assert.equal(a.workflow_run.repository_id, run.repository.id, `${name} artifact repository differs`);
    assert.equal(a.workflow_run.head_repository_id, run.repository.id, `${name} artifact head repository differs`);
    const archiveBytes = bytes(path.join(options.archivesDir, `${name}.zip`));
    assert.equal(a.digest, `sha256:${sha256(archiveBytes)}`, `${name} archive digest differs`);
    const archive = path.join(staging, `${name}.zip`), directory = path.join(staging, name);
    fs.writeFileSync(archive, archiveBytes, { flag: 'wx' });
    extract(archive, directory);
    artifacts.push({ id: a.id, name, archiveSha256: sha256(archiveBytes), archiveSize: archiveBytes.length });
  }
  const platforms = {}, manifests = [], buildInputs = [], resources = [], generatedRecords = [];
  const lockedTools = readJson(path.join(root, 'tools/phoenix-ui/build-tools.lock.json'));
  for (const [index, target] of targets.entries()) {
    const directory = path.join(staging, `phoenix-ui-${target}`), runtime = path.join(directory, 'runtime');
    const manifest = bootstrap.manifestAt(path.join(directory, 'VERSION.json')), m = manifest.data;
    assert(m.distribution === 'development' && Object.keys(m.engines).length === 1 && m.engines[target], `Invalid native artifact manifest ${target}`);
    const build = readJson(path.join(directory, 'build-record.json'));
    assert(build.schemaVersion === 1 && build.target === target && build.host === hosts[index], `Build host or target differs ${target}`);
    for (const [record, label] of [[m, 'manifest'], [build, 'build record']]) {
      assert.equal(record.sourceCommit, options.sourceCommit, `${target} ${label} source commit differs`);
      assert.equal(record.sourceTree, options.sourceTree, `${target} ${label} source tree differs`);
    }
    assert.deepEqual(build.tools, lockedTools, `${target} pinned tools differ`);
    assert.deepEqual(build.binary, m.engines[target], `${target} binary metadata differs`);
    assert(build.inputs && Object.keys(build.inputs).length, 'Build inputs are missing');
    for (const [name, hash] of Object.entries(build.inputs)) assert(safePath(name) && /^[a-f0-9]{64}$/.test(hash), 'Invalid build input');
    assert.equal(sha256(JSON.stringify(build.inputs)), build.inputSha256, `${target} build input digest differs`);
    assert.equal(m.sourceInputSha256, build.inputSha256, `${target} manifest input digest differs`);
    assert.deepEqual(digest(path.join(directory, build.binary.asset)), { size: build.binary.size, sha256: build.binary.sha256 }, `${target} tested binary digest differs`);
    for (const name of ['engineIdentity', 'browserInputs', 'skillGeneration']) assert.equal(build.checks[name], 'passed', `${target} build check ${name} failed`);
    assert.equal(build.bundleSha256, m.bundleSha256, `${target} build bundle digest differs`);
    const bundleDir = path.join(runtime, 'dist/phoenix-ui/bundle'), bundleBytes = bytes(path.join(bundleDir, 'manifest.json'));
    assert.equal(sha256(bundleBytes), m.bundleSha256, `${target} bundle digest differs`);
    const bundle = JSON.parse(bundleBytes);
    assert(bundle.schema === 1 && bundle.providers && Object.keys(bundle.providers).length && bundle.derivedRuntimeManifest === 'skills/phoenix-ui/scripts/ENGINE.json', 'Invalid bundle contract');
    const blobs = new Set();
    for (const provider of Object.values(bundle.providers)) for (const [name, hash] of Object.entries(provider.files)) {
      assert(safePath(name) && /^[a-f0-9]{64}$/.test(hash), 'Invalid bundle resource'); blobs.add(hash);
    }
    for (const hash of blobs) assert.equal(digest(path.join(bundleDir, 'blobs', hash)).sha256, hash, 'Bundle resource digest differs');
    for (const name of derivedFiles.slice(0, 2)) assert(bytes(path.join(runtime, name)).equals(manifest.bytes), `${target} derived manifest differs`);
    const catalog = readJson(path.join(directory, 'THIRD-PARTY.json'));
    assert(catalog.schemaVersion === 1 && catalog.rust.length && Array.isArray(catalog.node) && Array.isArray(catalog.bundled) && Array.isArray(catalog.resources), 'Missing license inventory');
    for (const [kind, entries] of [['rust', catalog.rust], ['node', catalog.node], ['bundled', catalog.bundled], ['resources', catalog.resources]]) for (const entry of entries) {
      assert(entry.license && Array.isArray(entry.licenses)
        && (entry.licenses.length || kind === 'rust' && declaredWinapiLicense(entry)), 'Missing preserved license');
      for (const license of entry.licenses) {
        assert(safePath(license.path) && license.path.startsWith('THIRD-PARTY-LICENSES/') && /^[a-f0-9]{64}$/.test(license.sha256), 'Invalid license path or digest');
        assert.equal(digest(path.join(directory, license.path)).sha256, license.sha256, 'Preserved license digest differs');
      }
    }
    const inv = inventory(runtime);
    const generated = readJson(path.join(runtime, 'skills/phoenix-ui/GENERATED.json'));
    assert(generated.schemaVersion === 1 && generated.engineManifest === 'verified', 'Missing verified generation evidence');
    assert.equal(generated.inputSha256, sha256(JSON.stringify(generated.inputs)), 'Generation input digest differs');
    assert.equal(generated.bundleSha256, m.bundleSha256, 'Generation bundle digest differs');
    assert.equal(generated.outputs.skill['scripts/ENGINE.json'], manifest.hash, 'Generation engine digest differs');
    for (const [name, hash] of Object.entries(generated.outputs.providers)) {
      assert(safePath(name) && /^[a-f0-9]{64}$/.test(hash), 'Invalid generated provider resource');
      if (name.endsWith('/skills/phoenix-ui/scripts/ENGINE.json')) assert.equal(hash, manifest.hash, 'Generation provider engine digest differs');
    }
    for (const [section, prefix] of [['skill', 'skills/phoenix-ui/'], ['bundle', 'dist/phoenix-ui/bundle/'], ['notices', 'dist/phoenix-ui/']]) {
      for (const [name, hash] of Object.entries(generated.outputs[section])) {
        assert(safePath(name) && /^[a-f0-9]{64}$/.test(hash), 'Invalid generated resource');
        assert.equal(inv[`${prefix}${name}`]?.sha256, hash, `Generated resource digest differs: ${name}`);
      }
    }
    for (const name of ['THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md', ...Object.keys(inventory(path.join(directory, 'THIRD-PARTY-LICENSES'), 'THIRD-PARTY-LICENSES/'))]) {
      assert.deepEqual(inv[`dist/phoenix-ui/${name}`], digest(path.join(directory, name)), 'Runtime license sidecar differs');
    }
    for (const name of ['LICENSE', 'NOTICE.md']) assert(inv[`dist/phoenix-ui/${name}`]?.size, `Missing ${name}`);
    const phase = readJson(path.join(staging, `phoenix-ui-${target}-evidence`, `${target}-ci-results.json`));
    assert(phase.schemaVersion === 1 && phase.target === target && phase.sourceCommit === options.sourceCommit && phase.workflowRun === options.runId, `${target} phase identity differs`);
    for (const name of Object.keys(phases)) assert.equal(phase.phases[name], 'success', `${target} phase ${name} did not succeed`);
    const jobId = successfulJob(jobRecord.jobs, options, `Native ${target}`, Object.values(phases));
    smoke(readJson(path.join(directory, 'runtime-smoke-node25.json')), manifest, target, 25);
    platforms[target] = { jobId, inputSha256: build.inputSha256, manifestSha256: manifest.hash,
      binary: build.binary, bundleSha256: m.bundleSha256, host: build.host, tools: build.tools,
      phases: phase.phases, checks: { ...build.checks, nativeSmoke: 'passed', fullRuntimePhase: phase.phases.runtime }, resources: inv };
    manifests.push(manifest); buildInputs.push(build.inputs); generatedRecords.push(engineDigests(generated, 'derived-release-manifest'));
    resources.push(Object.fromEntries(Object.entries(inv).filter(([name]) => !derivedFiles.includes(name))));
  }
  for (let i = 1; i < targets.length; i++) {
    for (const key of ['toolVersion', 'npmPackageVersion', 'bundleSchema', 'bundleSha256', 'reviewSchema']) assert.equal(manifests[i].data[key], manifests[0].data[key], `Platform ${key} differs`);
    assert.deepEqual(Object.keys(buildInputs[i]), Object.keys(buildInputs[0]), 'Platform input keys differ');
    for (const key of Object.keys(buildInputs[0])) if (!generatedInputs.includes(key)) assert.equal(buildInputs[i][key], buildInputs[0][key], `Unknown cross-platform input difference: ${key}`);
    assert.deepEqual(resources[i], resources[0], 'Platform runtime resources differ');
    assert.deepEqual(generatedRecords[i], generatedRecords[0], 'Platform generation evidence differs beyond derived engine digest');
  }
  const node18JobId = successfulJob(jobRecord.jobs, options, 'Node 18 portable consumer smoke', ['Run portable local installation and runtime on Node 18']);
  smoke(readJson(path.join(staging, 'phoenix-ui-node18-consumer-evidence/runtime-smoke-node18.json')), manifests[2], 'linux-x64', 18);
  // ZIP transport may discard execute bits. Roles come from the tested engine,
  // verified bundle exec declarations and the candidate-bound npm bin entry.
  const packageBytes = bytes(path.join(root, 'package.json')), packageMeta = JSON.parse(packageBytes);
  const npmBin = packageMeta.bin?.['phoenix-ui'];
  assert(safePath(npmBin), 'Candidate npm bin declaration is missing or invalid');
  const transportPermissions = [];
  const executable = (target, relative, role, expectedHash) => {
    const file = path.join(staging, `phoenix-ui-${target}`, relative), stat = fs.lstatSync(file);
    assert(stat.isFile(), `Candidate executable is not regular: ${target}/${relative}`);
    assert.equal(sha256(bytes(file)), expectedHash, `Candidate executable digest differs: ${target}/${relative}`);
    const before = stat.mode & 0o777, after = before | 0o111;
    fs.chmodSync(file, after);
    transportPermissions.push({ target, path: relative, role, sha256: expectedHash, before, after });
  };
  for (const [i, target] of targets.entries()) {
    assert.equal(sha256(packageBytes), buildInputs[i]['package.json'], `${target} checkout package digest differs from candidate`);
    assert.equal(packageMeta.version, manifests[i].data.npmPackageVersion, `${target} checkout package version differs from candidate`);
    const runtimeBinHash = platforms[target].resources[npmBin]?.sha256;
    assert(runtimeBinHash && runtimeBinHash === buildInputs[i][npmBin], `${target} candidate npm bin input digest is missing or differs: ${npmBin}`);
    executable(target, platforms[target].binary.asset, 'engine', platforms[target].binary.sha256);
    executable(target, `runtime/${npmBin}`, 'npm-bin', runtimeBinHash);
    const bundle = readJson(path.join(staging, `phoenix-ui-${target}`, 'runtime/dist/phoenix-ui/bundle/manifest.json'));
    const executed = new Set();
    for (const provider of Object.values(bundle.providers)) {
      assert(Array.isArray(provider.exec), 'Invalid bundle exec declaration');
      for (const name of provider.exec) {
        assert(safePath(name) && provider.files[name], `Invalid bundle exec path: ${name}`);
        assert.equal(platforms[target].resources[name]?.sha256, provider.files[name], `Cannot locate matching runtime bundle exec: ${target}/${name}`);
        if (!executed.has(name)) executable(target, `runtime/${name}`, 'bundle-exec', provider.files[name]);
        executed.add(name);
      }
    }
  }
  const allowedInputDifferences = generatedInputs.filter(key => buildInputs.some(inputs => inputs[key] !== buildInputs[0][key]));
  return { schemaVersion: 1, repository: run.repository.full_name, runId: options.runId, sourceCommit: options.sourceCommit,
    sourceTree: options.sourceTree, artifacts, platforms, allowedInputDifferences, allowedInputValues: Object.fromEntries(allowedInputDifferences.map(key => [key, Object.fromEntries(targets.map((target, i) => [target, buildInputs[i][key]]))])),
    bundleSha256: manifests[0].data.bundleSha256, node18: { jobId: node18JobId, smoke: 'passed' }, transportPermissions, canonicalTarget: targets[0], manifest: manifests[0].data };
}

// Preserve the existing publisher's HTTP_PROXY -> https_proxy semantics.
export function proxyEnv(env) {
  if (env.https_proxy || env.HTTPS_PROXY || env.all_proxy || env.ALL_PROXY) return { ...env };
  const proxy = env.http_proxy || env.HTTP_PROXY;
  return proxy ? { ...env, https_proxy: proxy } : { ...env };
}
function settings(env) {
  const requireVar = name => { assert(env[name], `Missing ${name}; export the maintainer R2 environment first`); return env[name]; };
  const config = { bucket: requireVar('AGENT_RELEASE_R2_BUCKET'), endpoint: requireVar('AGENT_RELEASE_R2_ENDPOINT'), publicBase: requireVar('AGENT_RELEASE_R2_PUBLIC_BASE').replace(/\/+$/, ''), prefix: (env.AGENT_RELEASE_R2_PREFIX || '').replace(/^\/+|\/+$/g, '') };
  for (const value of [config.endpoint, config.publicBase]) {
    const url = new URL(value);
    assert(!url.username && !url.password && !url.search && !url.hash && (url.protocol === 'https:' || url.protocol === 'http:' && ['127.0.0.1', '[::1]', 'localhost'].includes(url.hostname)), 'R2 endpoints require HTTPS or loopback HTTP');
  }
  assert(!config.prefix || safePath(config.prefix), 'Invalid R2 prefix');
  const awsEnv = { ...proxyEnv(env), AWS_ACCESS_KEY_ID: requireVar('AGENT_RELEASE_R2_ACCESS_KEY'), AWS_SECRET_ACCESS_KEY: requireVar('AGENT_RELEASE_R2_SECRET_KEY'), AWS_DEFAULT_REGION: 'auto', AWS_REGION: 'auto', AWS_REQUEST_CHECKSUM_CALCULATION: 'when_required', AWS_MAX_ATTEMPTS: '1', AWS_EC2_METADATA_DISABLED: 'true', AWS_PAGER: '' };
  for (const key of ['AWS_SESSION_TOKEN', 'AWS_PROFILE', 'AWS_DEFAULT_PROFILE']) delete awsEnv[key];
  successful('aws', ['s3api', 'put-object', '--if-none-match', '*', '--generate-cli-skeleton', 'input'], awsEnv);
  successful('curl', ['--version'], proxyEnv(env));
  return { ...config, awsEnv, curlEnv: proxyEnv(env) };
}
function publicReadback(object, config, staging) {
  const file = path.join(staging, 'public-readback');
  successful('curl', ['-fsSL', '--retry', '3', '--retry-delay', '2', '--max-time', '600', '--output', file, object.url], config.curlEnv);
  assert.deepEqual(digest(file), { size: object.size, sha256: object.sha256 }, `Public readback size/SHA-256 digest differs: ${object.key}`);
}
function existingReadback(object, config, staging) {
  const file = path.join(staging, 'existing-readback');
  successful('aws', ['s3api', 'get-object', '--bucket', config.bucket, '--key', object.key, '--endpoint-url', config.endpoint, file], config.awsEnv);
  assert.deepEqual(digest(file), { size: object.size, sha256: object.sha256 }, `Existing object size/SHA-256 digest differs: ${object.key}`);
}
function writeRecord(file, value) {
  const temp = `${file}.${process.pid}.tmp`;
  fs.writeFileSync(temp, json(value)); fs.renameSync(temp, file);
}
async function outputLock(outputDir) {
  directory(outputDir);
  const control = path.join(outputDir, '.publish-lock');
  fs.mkdirSync(control, { recursive: true }); directory(control);
  const guard = spawn('python3', ['-c', `import os,sys,stat,fcntl
out=os.open(sys.argv[1],os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
directory=os.open('.publish-lock',os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=out)
fd=os.open('owner.lock',os.O_RDWR|os.O_CREAT|os.O_NOFOLLOW,0o600,dir_fd=directory)
s=os.fstat(fd)
assert stat.S_ISREG(s.st_mode) and s.st_nlink == 1 and s.st_size == 0, 'Invalid publication lock file'
try: fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB)
except BlockingIOError:
 print('Publication output is locked by another publisher',file=sys.stderr)
 sys.exit(3)
print('locked',flush=True)
# Only the publisher owns this pipe's write end. SIGKILL closes it as well.
sys.stdin.buffer.read()`, outputDir], { stdio: ['pipe', 'pipe', 'pipe'] });
  let diagnostic = '';
  guard.stderr.on('data', chunk => { diagnostic += chunk; });
  guard.stdin.on('error', () => {}); // Lock refusal may close stdin before release.
  const exited = new Promise(resolve => {
    guard.once('error', error => resolve({ error }));
    guard.once('close', (code, signal) => resolve({ code, signal }));
  });
  try {
    await new Promise((resolve, reject) => {
      guard.stdout.once('data', chunk => chunk.toString() === 'locked\n' ? resolve() : reject(new Error('Invalid publication lock handshake')));
      guard.once('error', reject);
      guard.once('close', code => reject(new Error(diagnostic.trim() || `Publication lock guard exited ${code}`)));
    });
  } catch (error) { guard.stdin.end(); await exited; throw error; }
  return async () => {
    guard.stdin.end(); const result = await exited;
    assert(result.code === 0 && !result.error, diagnostic.trim() || 'Publication lock guard failed');
  };
}

export async function publishRelease(options, env = process.env) {
  const staging = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-publication-'));
  let record, releaseLock;
  try {
    const handoff = validateHandoff(options, staging), config = settings(env);
    const manifest = { ...handoff.manifest, distribution: 'release', engines: {} };
    const objects = targets.map(target => {
      const binary = handoff.platforms[target].binary;
      const key = [config.prefix, 'phoenix-ui', `v${manifest.npmPackageVersion}`, target, binary.asset].filter(Boolean).join('/');
      const object = { target, key, url: `${config.publicBase}/${key}`, ...binary, upload: 'pending', readback: 'pending' };
      manifest.engines[target] = { ...binary, url: object.url };
      return object;
    });
    const manifestBytes = Buffer.from(json(manifest)), requestSha256 = sha256(json({ handoff, bucket: config.bucket, endpoint: config.endpoint, objects }));
    const out = options.outputDir, recordFile = path.join(out, 'publication-record.json');
    if (fs.existsSync(out)) assert(options.resume, 'Output directory exists; explicit --resume is required');
    else fs.mkdirSync(out, { recursive: true });
    releaseLock = await outputLock(out);
    if (fs.existsSync(recordFile)) assert.equal(readJson(recordFile).requestSha256, requestSha256, 'Resume candidate or release URL plan differs');
    else assert.equal(fs.readdirSync(out).length, 1, 'Resume output has no matching publication record');
    if (fs.existsSync(path.join(out, 'VERSION.json'))) assert(bytes(path.join(out, 'VERSION.json')).equals(manifestBytes), 'Frozen VERSION differs; use a new version');
    record = { schemaVersion: 1, status: 'running', runId: handoff.runId, sourceCommit: handoff.sourceCommit, sourceTree: handoff.sourceTree, requestSha256, objects };
    writeRecord(recordFile, record);
    for (const object of objects) {
      const file = path.join(staging, `phoenix-ui-${object.target}`, object.asset);
      const put = command('aws', ['s3api', 'put-object', '--bucket', config.bucket, '--key', object.key, '--body', file, '--endpoint-url', config.endpoint, '--content-type', 'application/octet-stream', '--if-none-match', '*'], config.awsEnv);
      if (put.status !== 0) {
        if (/(PreconditionFailed|ConditionalRequestConflict)/.test(put.stderr)) {
          assert(options.resume, `Object already exists: ${object.key}; explicit --resume first verifies its bytes`);
          existingReadback(object, config, staging); object.upload = 'reused';
        } else throw new Error(`Conditional upload failed for ${object.key} (aws exit ${put.status}): ${put.stderr.trim()}`);
      } else object.upload = 'uploaded';
      writeRecord(recordFile, record);
    }
    for (const object of objects) {
      publicReadback(object, config, staging); object.readback = 'passed'; writeRecord(recordFile, record);
    }
    // Freeze only after every public URL has returned the fixed tested bytes.
    const canonical = path.join(staging, `phoenix-ui-${handoff.canonicalTarget}`);
    copyArtifact(path.join(canonical, 'runtime'), path.join(out, 'runtime'));
    for (const name of ['THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md', 'THIRD-PARTY-LICENSES']) copyArtifact(path.join(canonical, name), path.join(out, name));
    for (const name of ['LICENSE', 'NOTICE.md']) fs.copyFileSync(path.join(canonical, 'runtime/dist/phoenix-ui', name), path.join(out, name));
    for (const object of objects) fs.copyFileSync(path.join(staging, `phoenix-ui-${object.target}`, object.asset), path.join(out, object.asset));
    for (const name of derivedFiles.slice(0, 2)) fs.writeFileSync(path.join(out, 'runtime', name), manifestBytes);
    const generatedFile = path.join(out, 'runtime', derivedFiles[2]);
    fs.writeFileSync(generatedFile, json(engineDigests(readJson(generatedFile), sha256(manifestBytes))));
    const versionFile = path.join(out, 'VERSION.json');
    if (fs.existsSync(versionFile)) assert(bytes(versionFile).equals(manifestBytes), 'Frozen VERSION differs; use a new version');
    else fs.writeFileSync(versionFile, manifestBytes, { flag: 'wx', mode: 0o444 });
    const provenance = { ...handoff }; delete provenance.manifest;
    const releaseFiles = inventory(out);
    for (const name of ['publication-record.json', 'release-handoff.json']) delete releaseFiles[name];
    for (const name of Object.keys(releaseFiles)) if (name.startsWith('.publish-lock/')) delete releaseFiles[name];
    const frozen = { ...provenance, status: 'ready-for-pack', requestSha256, manifestSha256: sha256(manifestBytes), manifestSize: manifestBytes.length, derivedReleaseFiles: derivedFiles, files: releaseFiles,
      next: 'Use these exact VERSION/ENGINE/resource bytes for the final npm pack; record and validate that same tarball before any npm publish.',
      localSubstitute: ['127.0.0.1', '[::1]', 'localhost'].includes(new URL(config.endpoint).hostname), publicUrlReadback: 'passed', finalTarballAcceptance: 'not-run', npmPublication: 'not-requested' };
    writeRecord(path.join(out, 'release-handoff.json'), frozen);
    record.status = 'frozen'; record.manifestSha256 = frozen.manifestSha256; writeRecord(recordFile, record);
    return { status: 'frozen', runId: handoff.runId, sourceCommit: handoff.sourceCommit, sourceTree: handoff.sourceTree, manifestSha256: frozen.manifestSha256, outputDir: out, handoff: path.join(out, 'release-handoff.json'), objects };
  } catch (error) {
    if (record) { record.status = 'failed'; record.error = error.message; writeRecord(path.join(options.outputDir, 'publication-record.json'), record); }
    throw error;
  } finally {
    try { if (releaseLock) await releaseLock(); }
    finally { fs.rmSync(staging, { recursive: true, force: true }); }
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const options = {}, flags = { '--run-id': 'runId', '--source-commit': 'sourceCommit', '--source-tree': 'sourceTree', '--run-record': 'runRecord', '--jobs-record': 'jobsRecord', '--artifacts-record': 'artifactsRecord', '--archives-dir': 'archivesDir', '--output-dir': 'outputDir' };
    const args = process.argv.slice(2);
    for (let i = 0; i < args.length; i++) {
      if (args[i] === '--resume' && !options.resume) options.resume = true;
      else { const name = flags[args[i]]; assert(name && args[i + 1] && !options[name], `Unknown, duplicate, or missing argument ${args[i]}`); options[name] = args[++i]; }
    }
    for (const name of Object.values(flags)) assert(options[name], `Required --${name.replace(/[A-Z]/g, c => `-${c.toLowerCase()}`)}`);
    for (const name of ['runRecord', 'jobsRecord', 'artifactsRecord', 'archivesDir', 'outputDir']) options[name] = path.resolve(options[name]);
    console.log(json(await publishRelease(options)));
  } catch (error) { console.error(`Error: ${error.message}`); process.exitCode = 1; }
}
