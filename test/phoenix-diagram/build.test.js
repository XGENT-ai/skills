'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const { pathToFileURL } = require('node:url');
const root = path.resolve(__dirname, '../..');
const moduleReady = import(pathToFileURL(path.join(root, 'scripts/build-phoenix-diagram.mjs')));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
function temporary(t) {
  const directory = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram build test ')));
  t.after(() => fs.rmSync(directory, { force: true, recursive: true }));
  return directory;
}
function write(directory, name, contents, mode = 0o644) {
  const file = path.join(directory, name);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, contents, { mode }); fs.chmodSync(file, mode);
  return file;
}
function sourceFixture(t) {
  const directory = temporary(t);
  for (const name of ['package.json', 'tools/phoenix-diagram/Cargo.toml', 'tools/phoenix-diagram/Cargo.lock', 'tools/phoenix-diagram/build.rs',
    'tools/phoenix-diagram/src/check.rs', 'tools/phoenix-diagram/src/check/path.rs', 'tools/phoenix-diagram/assets/svg-definitions.svg',
    'tools/phoenix-diagram/viewer/viewer.css', 'tools/phoenix-diagram/upstream/SOURCE.json', 'tools/phoenix-diagram/upstream/legacy-2.17/schemas/common.schema.json',
    'tools/phoenix-diagram/migration-sources/archify-v1.json',
    ...['web-app.architecture', 'agent-tool-call.workflow', 'async-job-roundtrip.sequence', 'product-analytics.dataflow', 'deployment-release.lifecycle'].map(name => `skills/phoenix-diagram/examples/legacy.${name}.json`),
    'skills/phoenix-diagram/schemas/common.schema.json']) {
    write(directory, name, fs.readFileSync(path.join(root, name)));
  }
  fs.mkdirSync(path.join(directory, 'skills/phoenix-diagram'), { recursive: true });
  return directory;
}
function legalSourceFixture(t) {
  const directory = temporary(t);
  for (const name of ['LICENSE', 'tools/phoenix-diagram/NOTICE.md', 'tools/phoenix-diagram/upstream/LICENSE-archify-MIT.txt',
    'tools/phoenix-diagram/upstream/JetBrainsMono-OFL.txt', 'tools/phoenix-diagram/src/browser/LICENSE-APACHE-2.0',
    'tools/phoenix-diagram/assets/icu77-LICENSE.txt', 'tools/phoenix-diagram/build-tools.lock.json', 'tools/phoenix-diagram/Cargo.toml']) {
    write(directory, name, fs.readFileSync(path.join(root, name)));
  }
  write(directory, 'tools/phoenix-diagram/assets/license-sources.json', JSON.stringify({ schemaVersion: 1, rust: [] }));
  return directory;
}
// File-codec fixture only: these are actual frozen full terms, without a native
// engine or a claim that this catalog came from an installed Rust toolchain.
function legalResourceFixture(t) {
  const directory = temporary(t), files = new Map();
  const resource = (source, name) => {
    const bytes = fs.readFileSync(path.join(root, source)), digest = sha(bytes);
    name ||= `THIRD-PARTY-LICENSES/${digest}.txt`;
    files.set(name, bytes); return { path: name, size: bytes.length, sha256: digest, originalLocation: source };
  };
  const mit = resource('LICENSE'), apache = resource('tools/phoenix-diagram/src/browser/LICENSE-APACHE-2.0');
  const runtime = [
    { name: 'Archify', license: 'MIT', licenses: [resource('tools/phoenix-diagram/upstream/LICENSE-archify-MIT.txt')] },
    { name: 'JetBrains Mono', license: 'OFL-1.1', licenses: [resource('tools/phoenix-diagram/upstream/JetBrainsMono-OFL.txt')] },
    { name: 'Phoenix UI CDP/install adaptations', license: 'Apache-2.0', licenses: [apache] },
    { name: 'ICU 77.1 native text and SystemV time adaptations', license: 'Unicode-3.0', licenses: [resource('tools/phoenix-diagram/assets/icu77-LICENSE.txt')] },
    { name: 'Rust standard library', version: '1.99.0', license: 'MIT OR Apache-2.0', licenses: [
      ...['Apache-2.0.txt', 'BSD-2-Clause.txt', 'CC-BY-SA-4.0.txt', 'GCC-exception-3.1.txt', 'GPL-2.0-only.txt', 'GPL-3.0-or-later.txt',
        'ISC.txt', 'LLVM-exception.txt', 'MIT.txt', 'NCSA.txt', 'OFL-1.1.txt', 'Unicode-3.0.txt'].map(name =>
        resource('LICENSE', `THIRD-PARTY-LICENSES/rust-std-1.99.0/licenses/${name}`)),
      resource('tools/phoenix-diagram/NOTICE.md', 'THIRD-PARTY-LICENSES/rust-std-1.99.0/COPYRIGHT-library.html'),
    ] },
  ];
  const locked = [{ name: 'phoenix-diagram', version: '0.2.0' }];
  const catalog = { schemaVersion: 1, tool: 'phoenix-diagram', runtime, rust: [{ ...locked[0], source: 'local', license: 'MIT', licenses: [mit] }] };
  for (const name of ['LICENSE', 'NOTICE.md']) files.set(name, fs.readFileSync(path.join(root, name === 'LICENSE' ? name : 'tools/phoenix-diagram/NOTICE.md')));
  files.set('THIRD-PARTY-NOTICES.md', Buffer.from('File-codec fixture; no built artifact or native-platform claim.\n'));
  const save = () => write(directory, 'THIRD-PARTY.json', JSON.stringify(catalog, null, 2) + '\n');
  for (const [name, bytes] of files) write(directory, name, bytes);
  save(); return { directory, locked, catalog, files, save };
}

test('actual Node --version output binds the same version in observation and detail verification', async () => {
  const api = await moduleReady, actual = spawnSync(process.execPath, ['--version'], { encoding: 'utf8' });
  assert.equal(actual.status, 0, actual.stderr); assert.equal(actual.stdout.trim(), process.version);
  // The Node output is real. The other three are format fixtures; this test
  // neither invokes Cargo/Rust nor claims native build acceptance.
  const details = { node: actual.stdout, rustc: 'rustc 1.99.0', cargo: 'cargo 1.99.0', mbx: 'mbx 1.21.1' };
  assert.deepEqual(api.observedToolVersions(details), { rust: '1.99.0', cargo: '1.99.0', node: process.versions.node, mrBoxington: '1.21.1' });
  assert.equal(api.observedToolVersions({ ...details, node: 'not a version' }).node, undefined);
});

test('real compiler-artifact hardlinks are probed through an owned single-link snapshot without weakening identity', {
  skip: !process.env.PHOENIX_DIAGRAM_BIN ? 'Requires explicit stable PHOENIX_DIAGRAM_BIN; no automatic build or download' : !['darwin-arm64','darwin-x64','linux-x64','linux-arm64'].includes(`${process.platform}-${process.arch}`) ? 'Native probe requires a supported Unix host' : false,
}, async t => {
  const api = await moduleReady, directory = temporary(t), source = fs.realpathSync(process.env.PHOENIX_DIAGRAM_BIN);
  const original = fs.statSync(source, { bigint: true }), binaryBytes = fs.readFileSync(source), sourceDigest = sha(binaryBytes);
  t.after(() => {
    const current = fs.statSync(source, { bigint: true });
    assert.equal(current.ino, original.ino); assert.equal(current.nlink, original.nlink);
    assert.equal(sha(fs.readFileSync(source)), sourceDigest, 'The supplied release/cohort engine must remain untouched');
  });
  const executable = write(directory, 'compiler artifact', binaryBytes, 0o700);
  const before = spawnSync(executable, ['doctor', '--json'], { encoding: 'utf8', timeout: 15000 });
  const native = JSON.parse(before.stdout); assert.ok(native.engine, before.stdout + before.stderr);
  assert.equal(native.engine.binary.sha256, sourceDigest);
  fs.linkSync(executable, path.join(directory, 'cargo hardlink alias'));
  assert.equal(fs.statSync(executable).nlink, 2);
  const actual = spawnSync(executable, ['doctor', '--json'], { encoding: 'utf8', timeout: 15000 });
  assert.equal(actual.status, 4); assert.match(actual.stdout, /engine\/current-binary/); assert.match(actual.stdout, /output\/file-identity-or-links-changed/);
  const options = { binaryBytes, sourceSha: native.engine.sourceInputSha256, target: `${process.platform}-${process.arch}` };
  const result = api.probeNativeArtifact(executable, options);
  assert.deepEqual(result.doctor.engine, native.engine);
  assert.notEqual(result.probeExecutable, executable);
  assert.equal(fs.existsSync(result.probeExecutable), false, 'The private probe executable is cleaned after the real doctor exits');
  assert.equal(result.compilerLinks, 2);
  assert.equal(fs.statSync(executable).nlink, 2);
  assert.deepEqual(fs.readFileSync(executable), binaryBytes);
  assert.throws(() => api.probeNativeArtifact(executable, { ...options, sourceSha: '0'.repeat(64) }), /sourceInputSha256/);
  for (const [field, value] of [['inputContractSchema', 2], ['receiptSchema', 2], ['viewerRevision', 'foreign']]) {
    assert.throws(() => api.validateEngine({ ...native.engine, [field]: value }, options.sourceSha, result.binary), new RegExp(field));
  }
  for (const [field, value] of [['sha256', '0'.repeat(64)], ['bytes', binaryBytes.length + 1]]) {
    assert.throws(() => api.validateEngine({ ...native.engine, binary: { ...native.engine.binary, [field]: value } }, options.sourceSha, result.binary), new RegExp(`binary\\.${field}`));
  }
  assert.throws(() => api.probeNativeArtifact(executable, { ...options, binaryBytes: Buffer.concat([binaryBytes, Buffer.from('changed snapshot')]) }), /binary.*changed|binary.*differ/i);
  const mkdtemp = fs.mkdtempSync;
  t.mock.method(fs, 'mkdtempSync', (...args) => {
    const owned = mkdtemp(...args); fs.appendFileSync(executable, 'changed original artifact'); return owned;
  });
  assert.throws(() => api.probeNativeArtifact(executable, options), /binary changed while probing/i);
});

test('failed doctor diagnostics remain visible instead of a combined identity assertion', async t => {
  if (process.platform === 'win32') { t.skip('Unix protocol refusal fixture'); return; }
  const api = await moduleReady, directory = temporary(t);
  // Refusal-only protocol fixture. Every successful native probe is tested
  // above using a real supplied engine, never this script.
  const executable = write(directory, 'refused engine', `#!${process.execPath}\nif(process.argv[2]==='engine-probe')console.log('phoenix-diagram-engine 0.2.0 input=1 receipt=1 viewer=phoenix-v2');else{console.log(JSON.stringify({toolVersion:'0.2.0',diagnostics:[{code:'engine/current-binary',message:'output/file-identity-or-links-changed'}]}));process.exitCode=4;}\n`, 0o700);
  assert.throws(() => api.probeNativeArtifact(executable, { binaryBytes: fs.readFileSync(executable), sourceSha: 'a'.repeat(64), target: `${process.platform}-${process.arch}` }), error => /engine identity is missing/.test(error.message) && /"exitCode":4/.test(error.message) && /engine\/current-binary/.test(error.message) && /output\/file-identity-or-links-changed/.test(error.message));
});

test('source tuples follow native roots: embedded schemas count, skill text and derived ENGINE do not', async t => {
  const api = await moduleReady, directory = sourceFixture(t);
  write(directory, 'skills/phoenix-diagram/ENGINE.json', '{"derived":true}\n');
  write(directory, 'skills/phoenix-diagram/SKILL.md', '---\nname: phoenix-diagram\n---\n');
  const inputs = api.sourceInputs(directory);
  assert.ok(inputs.entries.some(row => row[0] === 'package.json'));
  assert.ok(inputs.entries.some(row => row[0] === 'tools/phoenix-diagram/build.rs'));
  assert.ok(inputs.entries.some(row => row[0] === 'skills/phoenix-diagram/schemas/common.schema.json'));
  assert.equal(inputs.entries.filter(row => row[0].startsWith('skills/phoenix-diagram/examples/legacy.')).length, 5);
  assert.ok(!inputs.entries.some(row => row[0].endsWith('/ENGINE.json') || row[0].endsWith('/SKILL.md')));
  assert.deepEqual(inputs.bytes, Buffer.from(JSON.stringify(inputs.entries)));
  assert.equal(inputs.sha256, sha(inputs.bytes));
  for (const [name, size, digest] of inputs.entries) {
    const bytes = fs.readFileSync(path.join(directory, name));
    assert.equal(size, bytes.length); assert.equal(digest, sha(bytes));
  }
  write(directory, 'skills/phoenix-diagram/ENGINE.json', '{"derived":"changed"}\n');
  write(directory, 'skills/phoenix-diagram/SKILL.md', '---\nname: phoenix-diagram\ndescription: revised\n---\n');
  assert.equal(api.sourceInputs(directory).sha256, inputs.sha256);
  fs.appendFileSync(path.join(directory, 'skills/phoenix-diagram/schemas/common.schema.json'), '\n');
  assert.notEqual(api.sourceInputs(directory).sha256, inputs.sha256);
});

test('source paths use full UTF-8 byte order for module siblings and Unicode, never component or UTF-16 sorting', async t => {
  const api = await moduleReady, directory = sourceFixture(t);
  write(directory, 'tools/phoenix-diagram/src/\uE000.rs', fs.readFileSync(path.join(root, 'tools/phoenix-diagram/src/lib.rs')));
  write(directory, 'tools/phoenix-diagram/src/🌳.rs', fs.readFileSync(path.join(root, 'tools/phoenix-diagram/src/lib.rs')));
  const names = api.sourceInputs(directory).entries.map(row => row[0]);
  assert.ok(names.indexOf('tools/phoenix-diagram/src/check.rs') < names.indexOf('tools/phoenix-diagram/src/check/path.rs'));
  assert.ok(names.indexOf('tools/phoenix-diagram/src/\uE000.rs') < names.indexOf('tools/phoenix-diagram/src/🌳.rs'));
  assert.deepEqual(names, [...names].sort((a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b))));
});

test('nonregular source entries and source mutations stop the build identity', async t => {
  const api = await moduleReady, directory = sourceFixture(t), before = api.sourceInputs(directory);
  fs.appendFileSync(path.join(directory, 'tools/phoenix-diagram/src/check.rs'), '\n');
  assert.throws(() => api.assertSourceInputsUnchanged(before, api.sourceInputs(directory)), /Source changed/);
  if (process.platform !== 'win32') {
    fs.symlinkSync(path.join(root, 'LICENSE'), path.join(directory, 'tools/phoenix-diagram/src/foreign.rs'));
    assert.throws(() => api.sourceInputs(directory), /nonregular|symbolic/i);
  }
});

test('source manifest reader rejects noncanonical, duplicate, unsorted and forged tuples', async t => {
  const api = await moduleReady, directory = sourceFixture(t), inputs = api.sourceInputs(directory);
  assert.equal(api.readSourceInputManifest(inputs.bytes).sha256, inputs.sha256);
  for (const rows of [[...inputs.entries].reverse(), [inputs.entries[0], inputs.entries[0]], [['../foreign', 1, 'a'.repeat(64)]], [['a', -1, 'a'.repeat(64)]], [['a', 1, 'bad']]]) {
    assert.throws(() => api.readSourceInputManifest(Buffer.from(JSON.stringify(rows))));
  }
  assert.throws(() => api.readSourceInputManifest(Buffer.concat([inputs.bytes, Buffer.from('\n')])), /canonical/);
});

test('Cargo JSON selects exactly the reported native diagram binary and its own build-script output', async t => {
  const api = await moduleReady, directory = temporary(t), workspace = path.join(root, 'tools/phoenix-diagram');
  const packageId = `path+file://${workspace}#0.2.0`;
  const executable = path.join(workspace, 'target/debug/phoenix-diagram');
  const artifact = { reason: 'compiler-artifact', package_id: packageId, manifest_path: path.join(workspace, 'Cargo.toml'), target: { name: 'phoenix-diagram', kind: ['bin'] }, profile: { test: false }, executable };
  const buildScript = { reason: 'build-script-executed', package_id: packageId, out_dir: directory };
  const encode = records => records.map(record => JSON.stringify(record)).join('\n') + '\n';
  const selected = api.selectCompilerArtifact(encode([buildScript, artifact]), workspace);
  assert.equal(selected.executable, executable); assert.equal(selected.outDir, directory);
  for (const records of [[artifact], [buildScript], [buildScript, artifact, artifact], [buildScript, { ...artifact, target: { name: 'phoenix-ui', kind: ['bin'] } }], [buildScript, { ...artifact, executable: null }]]) {
    assert.throws(() => api.selectCompilerArtifact(encode(records), workspace), /artifact|build.script|binary/i);
  }
  assert.throws(() => api.selectCompilerArtifact('not JSON\n', workspace), /JSON/);
});

test('maintained distribution bundle matches the source skill before native builds', async () => {
  const bundle = (await moduleReady).bundleFiles(root), directory = path.join(root, 'dist/phoenix-diagram/bundle');
  for (const [name, bytes] of bundle.files) {
    const file = path.join(directory, name), stat = fs.lstatSync(file);
    assert.ok(stat.isFile() && !stat.isSymbolicLink(), `Distribution bundle is not a regular file: ${name}`);
    assert.ok(fs.readFileSync(file).equals(bytes), `Distribution bundle differs from the source skill: ${name}`);
    assert.equal(stat.mode & 0o777, bundle.manifest.files.find(row => row.path === name).mode, `Distribution bundle mode differs: ${name}`);
  }
  assert.ok(fs.readFileSync(path.join(directory, 'manifest.json')).equals(bundle.bytes), 'Distribution bundle manifest differs from the source skill');
  const versionBytes = fs.readFileSync(path.join(root, 'dist/phoenix-diagram/VERSION.json'));
  assert.equal(JSON.parse(versionBytes).bundleSha256, sha(bundle.bytes), 'VERSION must bind the maintained bundle manifest');
  for (const file of ['skills/phoenix-diagram/ENGINE.json', 'dist/phoenix-diagram/bundle/ENGINE.json']) {
    assert.deepEqual(fs.readFileSync(path.join(root, file)), versionBytes, `${file} must match VERSION byte for byte`);
  }
});

test('bundle copies only the independent skill allowlist and leaves ENGINE out of manifest files', async t => {
  const api = await moduleReady, directory = sourceFixture(t), skill = 'skills/phoenix-diagram';
  write(directory, `${skill}/SKILL.md`, '---\nname: phoenix-diagram\ndescription: Build inventory fixture\n---\n');
  for (const name of ['phoenix-diagram', 'phoenix-launcher.cjs', 'phoenix-bootstrap.cjs']) write(directory, `${skill}/scripts/${name}`, fs.readFileSync(path.join(root, skill, 'scripts', name)), name === 'phoenix-bootstrap.cjs' ? 0o644 : 0o755);
  for (const kind of ['common', 'architecture', 'workflow', 'sequence', 'dataflow', 'lifecycle', 'erd', 'class', 'tree', 'timeline', 'waterfall']) write(directory, `${skill}/schemas/${kind}.schema.json`, fs.readFileSync(path.join(root, skill, 'schemas', `${kind}.schema.json`)));
  write(directory, `${skill}/ENGINE.json`, '{"derived":true}\n');
  const bundle = api.bundleFiles(directory);
  assert.ok(bundle.files.has('SKILL.md')); assert.ok(!bundle.files.has('ENGINE.json'));
  assert.ok(!bundle.manifest.files.some(file => file.path === 'ENGINE.json'));
  assert.equal(bundle.sha256, sha(bundle.bytes));
  write(directory, `${skill}/hooks/settings.json`, '{}');
  assert.throws(() => api.bundleFiles(directory), /allowlist|Unsupported|unsafe/i);
});

test('locked Rust inventory is complete and license metadata cannot replace full matching materials', async () => {
  const api = await moduleReady, lock = fs.readFileSync(path.join(root, 'tools/phoenix-diagram/Cargo.lock'), 'utf8');
  const packages = api.parseCargoLock(lock);
  assert.ok(packages.length > 100); assert.ok(packages.some(p => p.name === 'phoenix-diagram' && p.version === '0.2.0'));
  assert.throws(() => api.collectLicenses({ repoRoot: root, metadata: { packages: [] }, locked: packages }), /lock|metadata|package/i);
  assert.throws(() => api.parseCargoLock('version = 4\n'), /Cargo.lock|inventory/);
});

test('license collection requires actual prepared Rust standard-library resources', async t => {
  const api = await moduleReady, directory = legalSourceFixture(t), locked = [{ name: 'phoenix-diagram', version: '0.2.0' }];
  const metadata = { packages: [{ ...locked[0], license: 'MIT', source: null, manifest_path: path.join(directory, 'tools/phoenix-diagram/Cargo.toml') }] };
  assert.throws(() => api.collectLicenses({ repoRoot: directory, metadata, locked }), /Rust.*sysroot|standard.library.*resource/i);
  const rustSysroot = path.join(directory, 'empty sysroot'); fs.mkdirSync(rustSysroot);
  assert.throws(() => api.collectLicenses({ repoRoot: directory, metadata, locked, rustSysroot }), /standard.library|ENOENT/i);
});

test('legal-resource codec binds complete terms, preserves relative standard-library files and rejects corruption or foreign slots', async t => {
  const api = await moduleReady, fixture = legalResourceFixture(t), { directory, locked, catalog, files, save } = fixture;
  const read = () => api.readLicenseResources(directory, locked);
  const result = read();
  assert.throws(() => api.readLicenseResources(directory, locked, { requireBrands: true }), /P3.*brand/i);
  for (const [name, bytes] of files) assert.deepEqual(result.files.get(name), bytes, name);
  assert.ok(result.files.has('THIRD-PARTY-LICENSES/rust-std-1.99.0/COPYRIGHT-library.html'));
  const original = catalog.runtime[0].licenses, term = path.join(directory, original[0].path), bytes = fs.readFileSync(term);
  catalog.runtime[0].licenses = [{ license: 'MIT' }]; fs.unlinkSync(term); save();
  assert.throws(read, /full|complete|terms|resource/i);
  fs.writeFileSync(term, bytes); catalog.runtime[0].licenses = []; save();
  assert.throws(read, /full|complete|terms|runtime/i); catalog.runtime[0].licenses = original; save();
  fs.appendFileSync(term, '\n'); assert.throws(read, /digest/); fs.writeFileSync(term, bytes);
  const extra = write(directory, 'THIRD-PARTY-LICENSES/extra.txt', bytes); assert.throws(read, /Extra|missing/); fs.unlinkSync(extra);
  fs.unlinkSync(term); assert.throws(read, /ENOENT/);
  if (process.platform !== 'win32') {
    fs.symlinkSync(path.join(root, 'LICENSE'), term); assert.throws(read, /regular|symbolic/); fs.unlinkSync(term);
    fs.linkSync(path.join(directory, 'LICENSE'), term); assert.throws(read, /hard links/); fs.unlinkSync(term);
  }
  fs.mkdirSync(term); assert.throws(read, /regular|symbolic/);
});

test('build CLI rejects occupied output and source aliases before invoking any maintenance tool', async t => {
  const directory = temporary(t), script = path.join(root, 'scripts/build-phoenix-diagram.mjs');
  const run = outputDir => spawnSync(process.execPath, [script, '--development', '--output-dir', outputDir], { cwd: root, encoding: 'utf8', env: { ...process.env, PATH: '' } });
  const occupied = path.join(directory, 'occupied'), sentinel = write(directory, 'occupied/sentinel', fs.readFileSync(path.join(root, 'LICENSE')));
  const before = fs.readFileSync(sentinel), conflict = run(occupied);
  assert.equal(conflict.status, 1); assert.match(conflict.stderr, /new or empty/); assert.deepEqual(fs.readFileSync(sentinel), before);
  if (process.platform !== 'win32') {
    const alias = path.join(directory, 'source alias'); fs.symlinkSync(path.join(root, 'tools/phoenix-diagram/assets'), alias);
    const output = path.join(alias, `unused-build-${path.basename(directory)}`), result = run(output);
    assert.equal(result.status, 1); assert.match(result.stderr, /native source.input root/); assert.ok(!fs.existsSync(output));
  }
});

test('Git metadata distinguishes an actual dirty working source from its committed base', async t => {
  const api = await moduleReady, directory = temporary(t);
  const git = args => { const result = spawnSync('git', args, { cwd: directory, encoding: 'utf8' }); assert.equal(result.status, 0, result.stderr); return result.stdout.trim(); };
  git(['init', '-q']); git(['config', 'user.name', 'Build fixture']); git(['config', 'user.email', 'fixture@example.invalid']);
  write(directory, 'source.txt', fs.readFileSync(path.join(root, 'LICENSE'))); git(['add', 'source.txt']); git(['commit', '-qm', 'fixture source']);
  const clean = api.gitIdentity(directory);
  assert.equal(clean.sourceCommit, git(['rev-parse', 'HEAD'])); assert.equal(clean.sourceTree, git(['rev-parse', 'HEAD^{tree}'])); assert.equal(clean.sourceState, 'clean');
  fs.appendFileSync(path.join(directory, 'source.txt'), '\n');
  const dirty = api.gitIdentity(directory);
  assert.equal(dirty.sourceCommit, clean.sourceCommit); assert.equal(dirty.sourceTree, clean.sourceTree); assert.equal(dirty.sourceState, 'dirty'); assert.equal(dirty.sourceReference, `dirty@${clean.sourceCommit}`);
});

test('incomplete or malformed development artifacts are rejected without invoking an engine', async t => {
  const api = await moduleReady, directory = temporary(t);
  assert.throws(() => api.validateDevelopmentArtifact(directory), /Missing development VERSION\.json/);
  write(directory, 'VERSION.json', '{"schemaVersion":1,"distribution":"candidate"}\n');
  assert.throws(() => api.validateDevelopmentArtifact(directory), /must be a one-platform development VERSION/);
});
