import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

// Pack the npm tarball exactly once from a frozen release produced by publish-phoenix-ui-r2.mjs.
// Usage: node scripts/pack-phoenix-ui-release.mjs --release-dir DIR --handoff FILE --build-record FILE
//   --manifest-sha256 HEX --run-id N --source-commit SHA --source-tree SHA --output-dir DIR [--execute]
const here = path.dirname(fileURLToPath(import.meta.url)), root = path.resolve(here, '..');
const bootstrap = createRequire(import.meta.url)(path.join(root, 'skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs'));
const required = ['--release-dir', '--handoff', '--build-record', '--manifest-sha256', '--run-id', '--source-commit', '--source-tree', '--output-dir'];
const options = {};
for (let i = 2; i < process.argv.length; i++) {
  const flag = process.argv[i];
  if (flag === '--execute') { assert(!options[flag], 'Duplicate --execute'); options[flag] = true; }
  else { assert(required.includes(flag) && process.argv[i + 1] && !options[flag], `Invalid argument ${flag}`); options[flag] = process.argv[++i]; }
}
for (const flag of required) assert(options[flag], `Required ${flag}; use the actual successful final CI handoff`);
assert(/^[1-9]\d*$/.test(options['--run-id']));
for (const flag of ['--source-commit', '--source-tree']) assert(/^[a-f0-9]{40}$/.test(options[flag]), `Invalid ${flag}`);
assert(/^[a-f0-9]{64}$/.test(options['--manifest-sha256']));
for (const flag of ['--release-dir', '--handoff', '--build-record', '--output-dir']) options[flag] = path.resolve(options[flag]);
const safe = name => typeof name === 'string' && name && !/[\\\x00:]/.test(name) && !name.split('/').some(part => !part || part === '.' || part === '..');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const json = value => JSON.stringify(value, null, 2) + '\n';
const regular = file => assert(fs.lstatSync(file).isFile() && !fs.lstatSync(file).isSymbolicLink(), `Expected regular file ${file}`);
const bytes = file => { regular(file); return fs.readFileSync(file); };
const digest = file => { const data = bytes(file); return { size: data.length, sha256: sha(data) }; };
function directory(dir) { assert(fs.lstatSync(dir).isDirectory() && !fs.lstatSync(dir).isSymbolicLink(), `Expected directory ${dir}`); }
function inventory(dir, prefix = '', release = false) {
  directory(dir); const result = {};
  for (const entry of fs.readdirSync(dir)) {
    if (release && ['publication-record.json', 'release-handoff.json'].includes(entry)) { regular(path.join(dir, entry)); continue; }
    if (release && entry === '.publish-lock') { directory(path.join(dir, entry)); continue; }
    const relative = `${prefix}${entry}`, file = path.join(dir, entry); assert(safe(relative));
    if (fs.lstatSync(file).isDirectory()) Object.assign(result, inventory(file, `${relative}/`));
    else result[relative] = digest(file);
  }
  return result;
}
function command(program, argv, extra = {}) {
  const result = spawnSync(program, argv, { cwd: root, maxBuffer: 256 * 1024 * 1024, ...extra });
  assert(!result.error && result.status === 0, `${program}: ${result.error?.message || result.stderr?.toString()}`); return result.stdout;
}
const git = argv => command('git', argv).toString().trim();
assert.equal(git(['rev-parse', 'HEAD']), options['--source-commit'], 'Checkout HEAD differs');
assert.equal(git(['rev-parse', 'HEAD^{tree}']), options['--source-tree'], 'Checkout tree differs');
const release = options['--release-dir'], handoffBytes = bytes(options['--handoff']), handoff = JSON.parse(handoffBytes);
const manifest = bootstrap.manifestAt(path.join(release, 'VERSION.json'));
assert.equal(manifest.hash, options['--manifest-sha256']); assert.equal(manifest.data.distribution, 'release');
assert.equal(handoff.status, 'ready-for-pack'); assert.equal(handoff.localSubstitute, false); assert.equal(handoff.publicUrlReadback, 'passed');
assert.equal(handoff.runId, options['--run-id']); assert.equal(handoff.sourceCommit, options['--source-commit']); assert.equal(handoff.sourceTree, options['--source-tree']);
assert.equal(manifest.data.sourceCommit, handoff.sourceCommit); assert.equal(manifest.data.sourceTree, handoff.sourceTree);
assert.equal(handoff.manifestSha256, manifest.hash); assert.equal(handoff.manifestSize, manifest.bytes.length);
assert.deepEqual(inventory(release, '', true), handoff.files, 'Frozen release bytes differ from handoff');
const derived = ['vendor/phoenix-ui/VERSION.json', 'skills/phoenix-ui/scripts/ENGINE.json', 'skills/phoenix-ui/GENERATED.json'];
assert.deepEqual(handoff.derivedReleaseFiles, derived);
const targets = ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64'];
assert.deepEqual(Object.keys(handoff.platforms).sort(), targets); assert.deepEqual(Object.keys(manifest.data.engines).sort(), targets);
for (const target of targets) {
  const platform = handoff.platforms[target], engine = manifest.data.engines[target];
  assert.equal(platform.checks.nativeSmoke, 'passed'); assert.equal(platform.checks.fullRuntimePhase, 'success');
  for (const phase of ['prepare', 'audit', 'light', 'build', 'smoke', 'runtime']) assert.equal(platform.phases[phase], 'success');
  assert.deepEqual({ asset: engine.asset, size: engine.size, sha256: engine.sha256 }, platform.binary);
  assert.deepEqual(digest(path.join(release, engine.asset)), { size: engine.size, sha256: engine.sha256 });
}
// Every frozen URL must be the public object this publication uploaded (or reused) and read back.
const publication = JSON.parse(bytes(path.join(release, 'publication-record.json')));
assert.equal(publication.status, 'frozen'); assert.equal(publication.manifestSha256, manifest.hash);
for (const [target, engine] of Object.entries(manifest.data.engines)) {
  const object = publication.objects.find(item => item.target === target);
  assert(object && object.readback === 'passed' && ['uploaded', 'reused'].includes(object.upload), `Public object not read back ${target}`);
  assert(engine.url.startsWith('https://'), `Release URL must use HTTPS ${target}`); assert.equal(engine.url, object.url, `Frozen public URL changed ${target}`);
}
const canonical = handoff.platforms[handoff.canonicalTarget]; assert(canonical);
const buildBytes = bytes(options['--build-record']), build = JSON.parse(buildBytes);
assert.equal(build.schemaVersion, 1); assert.equal(build.target, handoff.canonicalTarget);
assert.equal(build.sourceCommit, handoff.sourceCommit); assert.equal(build.sourceTree, handoff.sourceTree);
assert.equal(sha(JSON.stringify(build.inputs)), canonical.inputSha256); assert.equal(build.inputSha256, canonical.inputSha256);
assert.equal(build.inputSha256, manifest.data.sourceInputSha256); assert.deepEqual(build.binary, canonical.binary); assert.deepEqual(build.tools, canonical.tools);
assert.equal(process.version, `v${canonical.tools.node}`, 'Pack with the candidate maintenance Node');
const generatedAssets = ['tools/phoenix-ui/crates/live/assets/bundle-inputs.json', 'tools/phoenix-ui/crates/live/assets/detect-antipatterns-browser.js'];
const generatedInputDifferences = [];
for (const [relative, expected] of Object.entries(build.inputs)) {
  assert(safe(relative) && /^[a-f0-9]{64}$/.test(expected)); const actual = sha(bytes(path.join(root, relative)));
  if (actual !== expected) {
    assert(generatedAssets.includes(relative), `Maintained compiler input differs ${relative}`);
    generatedInputDifferences.push({ relative, checkoutSha256: actual, canonicalCIInputSha256: expected, reason: 'Generated platform asset; not used as a pack source' });
  }
}
assert.equal(sha(bytes(path.join(root, 'package.json'))), build.inputs['package.json']);
assert.equal(sha(bytes(path.join(root, 'bin/phoenix-ui.js'))), build.inputs['bin/phoenix-ui.js']);
const pkg = JSON.parse(bytes(path.join(root, 'package.json'))); assert.equal(pkg.name, '@xgent-ai/skills'); assert.equal(pkg.version, manifest.data.npmPackageVersion);
for (const hook of ['prepack', 'prepare', 'postpack']) assert(!pkg.scripts?.[hook], `Unexpected pack lifecycle ${hook}; do not run a build`);
assert(Array.isArray(pkg.files) && pkg.files.every(name => safe(name) && !/[*?\[\]{}]/.test(name)), 'Unsupported package files selector');
// Select the same maintained files as the existing package, including npm's root notices.
const selected = command('git', ['ls-tree', '-rz', '--full-tree', handoff.sourceCommit]).toString().split('\0').filter(Boolean).map(line => {
  const match = /^(\d+) blob ([a-f0-9]{40})\t(.+)$/.exec(line); return match && { mode: parseInt(match[1], 8) & 0o777, object: match[2], name: match[3] };
}).filter(entry => entry && (['package.json', '.gitignore'].includes(entry.name) || !entry.name.includes('/') && /^(?:readme|licen[sc]e|copying)(?:\..*)?$/i.test(entry.name) || pkg.files.some(name => entry.name === name || entry.name.startsWith(`${name}/`))));
assert(selected.length && selected.every(entry => safe(entry.name)));
const blobBatch = command('git', ['cat-file', '--batch'], { input: selected.map(entry => entry.object).join('\n') + '\n' });
const sources = new Map(); let cursor = 0;
for (const entry of selected) {
  const newline = blobBatch.indexOf(10, cursor), header = blobBatch.subarray(cursor, newline).toString().split(' ');
  assert.equal(header[0], entry.object); assert.equal(header[1], 'blob');
  const size = Number(header[2]), data = blobBatch.subarray(newline + 1, newline + 1 + size); cursor = newline + 2 + size;
  assert.equal(blobBatch[cursor - 1], 10); assert.equal(data.length, size);
  if (!derived.includes(entry.name)) assert(bytes(path.join(root, entry.name)).equals(data), `Current maintained pack source differs from candidate ${entry.name}`);
  assert(entry.mode === 0o644 || entry.mode === 0o755, `Unexpected candidate file mode ${entry.name}`);
  sources.set(entry.name, { ...entry, data, size, sha256: sha(data) });
}
assert.equal(cursor, blobBatch.length);
for (const bin of Object.values(pkg.bin)) assert(sources.has(bin), `Missing candidate bin ${bin}`);
const runtime = Object.fromEntries(Object.entries(handoff.files).filter(([name]) => name.startsWith('runtime/')).map(([name, value]) => [name.slice(8), value]));
for (const [relative, expected] of Object.entries(canonical.resources)) if (!derived.includes(relative)) assert.deepEqual(runtime[relative], expected, `Transported runtime differs ${relative}`);
for (const relative of derived.slice(0, 2)) assert.equal(runtime[relative]?.sha256, manifest.hash);
const generated = JSON.parse(bytes(path.join(release, 'runtime/skills/phoenix-ui/GENERATED.json')));
assert.equal(generated.engineManifest, 'verified'); assert.equal(generated.inputSha256, sha(JSON.stringify(generated.inputs))); assert.equal(generated.bundleSha256, manifest.data.bundleSha256);
for (const [name, expected] of Object.entries(generated.outputs.providers)) if (name.endsWith('/skills/phoenix-ui/scripts/ENGINE.json')) assert.equal(expected, manifest.hash, `GEN provider ENGINE differs ${name}`);
for (const [section, prefix] of [['skill', 'skills/phoenix-ui/'], ['bundle', 'vendor/phoenix-ui/bundle/'], ['notices', 'vendor/phoenix-ui/']]) {
  for (const [name, expected] of Object.entries(generated.outputs[section])) assert.equal(runtime[`${prefix}${name}`]?.sha256, expected, `GEN binding differs ${section}/${name}`);
}
assert.equal(runtime['vendor/phoenix-ui/bundle/manifest.json']?.sha256, manifest.data.bundleSha256);
for (const name of ['LICENSE', 'NOTICE.md', 'THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md']) assert.deepEqual(runtime[`vendor/phoenix-ui/${name}`], handoff.files[name], `License sidecar differs ${name}`);
assert(Object.keys(runtime).some(name => name.startsWith('vendor/phoenix-ui/THIRD-PARTY-LICENSES/')));
const expectedStageFiles = { ...Object.fromEntries([...sources].map(([name, source]) => [name, { size: source.size, sha256: source.sha256 }])), ...runtime };
const plan = { runId: handoff.runId, sourceCommit: handoff.sourceCommit, sourceTree: handoff.sourceTree, manifestSha256: manifest.hash, handoffSha256: sha(handoffBytes), buildRecordSha256: sha(buildBytes),
  scriptSha256: sha(bytes(fileURLToPath(import.meta.url))), packageJsonSha256: sources.get('package.json').sha256, canonicalInputSha256: build.inputSha256, generatedInputDifferences,
  maintainedSources: Object.fromEntries([...sources].map(([name, source]) => [name, { size: source.size, sha256: source.sha256 }])), runtime, finalConsumerAcceptance: 'not-run', npmPublication: 'not-requested' };
const requestSha256 = sha(JSON.stringify(plan)), out = options['--output-dir'], stage = path.join(out, 'package'), tarballs = path.join(out, 'tarballs');
function verifyTarball(file, stageFiles) {
  const entries = JSON.parse(command('python3', ['-c', `import sys,tarfile,json,hashlib
out={}
with tarfile.open(sys.argv[1],'r:gz') as archive:
 for item in archive:
  if item.isdir(): continue
  if not item.isfile() or not item.name.startswith('package/'): raise ValueError('Unsafe tarball member')
  name=item.name[8:]
  if not name or any(p in ('','.', '..') for p in name.split('/')) or name in out or item.mode&0o7000: raise ValueError('Unsafe or duplicate tarball path/mode')
  raw=archive.extractfile(item).read()
  out[name]={'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest(),'mode':item.mode&0o777}
print(json.dumps(out))`, file]).toString());
  for (const [name, entry] of Object.entries(entries)) {
    assert.deepEqual({ size: entry.size, sha256: entry.sha256 }, stageFiles[name], `Packed byte differs ${name}`);
    assert(!name.startsWith('tools/') && !name.startsWith('vendor/impeccable/') && !name.startsWith('skills/phoenix-ui/src/'), `Unexpected source/legacy payload ${name}`);
  }
  for (const name of ['package.json', ...Object.values(pkg.bin), ...Object.keys(runtime).filter(name => path.basename(name) !== '.npmignore')]) assert(entries[name], `Required runtime/package byte absent ${name}`);
  const exec = [...Object.values(pkg.bin), ...handoff.transportPermissions.filter(item => item.target === handoff.canonicalTarget && item.path.startsWith('runtime/')).map(item => item.path.slice(8))];
  for (const name of exec) assert(entries[name]?.mode & 0o111, `Packed executable lost its declared role ${name}`);
  return entries;
}
if (!options['--execute']) { console.log(json({ status: 'pack-inputs-checked-only', ...plan, requestSha256, outputDir: out })); process.exit(0); }
if (!fs.existsSync(out)) fs.mkdirSync(out); directory(out);
const recordFile = path.join(out, 'pack-record.json');
if (fs.existsSync(tarballs)) {
  directory(tarballs); const existing = fs.readdirSync(tarballs).filter(name => name.endsWith('.tgz'));
  if (existing.length) {
    assert.equal(existing.length, 1, 'Multiple tarballs; do not pack again');
    const record = JSON.parse(bytes(recordFile)); assert.equal(record.status, 'frozen-single-tarball-pending-consumer'); assert.equal(record.requestSha256, requestSha256);
    assert.equal(record.tarball.file, existing[0]); const file = path.join(tarballs, existing[0]);
    assert.deepEqual(digest(file), { size: record.tarball.size, sha256: record.tarball.sha256 });
    assert.deepEqual(record.stageFiles, expectedStageFiles, 'Existing pack sources no longer match the fixed candidate/runtime');
    assert.deepEqual(inventory(stage), record.stageFiles); assert.deepEqual(verifyTarball(file, record.stageFiles), record.tarball.files);
    console.log(json({ status: 'existing-tarball-verified', file, tarballSha256: record.tarball.sha256, manifestSha256: manifest.hash, packExecuted: false })); process.exit(0);
  }
}
// Persistent exclusive marker: failure/interruption never silently invokes a second pack.
fs.writeFileSync(path.join(out, 'pack-request.json'), json({ ...plan, requestSha256 }), { flag: 'wx' });
const record = { status: 'staging', ...plan, requestSha256, node: { executable: process.execPath, version: process.version } };
const save = () => fs.writeFileSync(recordFile, json(record)); save();
try {
  assert(!fs.existsSync(stage), 'Stage already exists; inspect the previous attempt'); fs.mkdirSync(stage); fs.mkdirSync(tarballs);
  for (const source of sources.values()) { const dest = path.join(stage, source.name); fs.mkdirSync(path.dirname(dest), { recursive: true }); fs.writeFileSync(dest, source.data, { flag: 'wx', mode: source.mode }); }
  for (const relative of Object.keys(runtime)) {
    const source = path.join(release, 'runtime', relative), dest = path.join(stage, relative); fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.copyFileSync(source, dest); fs.chmodSync(dest, fs.lstatSync(source).mode & 0o777); assert.deepEqual(digest(dest), runtime[relative]);
  }
  record.stageFiles = inventory(stage);
  assert.deepEqual(record.stageFiles, expectedStageFiles, 'Stage contains bytes outside the fixed candidate/runtime plan');
  fs.writeFileSync(path.join(out, 'release-handoff.input.json'), handoffBytes, { flag: 'wx' }); fs.writeFileSync(path.join(out, 'build-record.input.json'), buildBytes, { flag: 'wx' });
  const npm = path.resolve(path.dirname(process.execPath), '../lib/node_modules/npm/bin/npm-cli.js'); regular(npm);
  const npmUser = path.join(out, 'npm-user.npmrc'), npmGlobal = path.join(out, 'npm-global.npmrc');
  fs.writeFileSync(npmUser, '', { flag: 'wx' }); fs.writeFileSync(npmGlobal, '', { flag: 'wx' });
  const env = { ...process.env };
  for (const name of Object.keys(env)) if (/^npm_config_(?:userconfig|globalconfig|cache|update_notifier)$/i.test(name)) delete env[name];
  Object.assign(env, { npm_config_userconfig: npmUser, npm_config_globalconfig: npmGlobal, npm_config_cache: path.join(out, 'npm-cache'), npm_config_update_notifier: 'false' });
  record.packCommand = { program: process.execPath, argv: [npm, 'pack', '--offline', '--json', '--pack-destination', tarballs], cwd: stage }; record.status = 'pack-started'; save();
  const result = spawnSync(record.packCommand.program, record.packCommand.argv, { cwd: stage, env, encoding: 'utf8', timeout: 600000, maxBuffer: 64 * 1024 * 1024 });
  fs.writeFileSync(path.join(out, 'npm-pack.stdout.json'), result.stdout || ''); fs.writeFileSync(path.join(out, 'npm-pack.stderr'), result.stderr || '');
  record.packCommand.exitCode = result.status; record.packCommand.signal = result.signal; record.packCommand.error = result.error?.message || null; save();
  assert(!result.error && result.status === 0, 'Single pack failed; preserve this output and do not automatically pack again');
  const listing = JSON.parse(result.stdout); assert.equal(listing.length, 1); const item = listing[0]; assert(item.filename === path.basename(item.filename) && item.filename.endsWith('.tgz'));
  const file = path.join(tarballs, item.filename), entries = verifyTarball(file, record.stageFiles);
  assert.deepEqual(item.files.map(entry => entry.path).sort(), Object.keys(entries).sort(), 'npm/tar actual file lists differ');
  for (const entry of item.files) assert.equal(entry.size, entries[entry.path].size);
  record.tarball = { file: item.filename, ...digest(file), files: entries }; record.status = 'frozen-single-tarball-pending-consumer'; save();
  console.log(json({ status: record.status, file, tarballSha256: record.tarball.sha256, manifestSha256: manifest.hash, record: recordFile, finalConsumerAcceptance: 'not-run' }));
} catch (error) { record.status = 'failed'; record.error = error.message; save(); console.error(error.message); process.exitCode = 1; }
