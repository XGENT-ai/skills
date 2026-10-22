'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const https = require('node:https');
const net = require('node:net');
const { createHash } = require('node:crypto');
const { spawn, spawnSync } = require('node:child_process');

const root = path.resolve(__dirname, '../..');
const source = path.join(root, 'skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const boot = () => require(source);
const packageVersion = () => JSON.parse(fs.readFileSync(path.join(root, 'package.json'))).version;
const supportedUnix=['darwin-arm64','darwin-x64','linux-x64','linux-arm64'].includes(`${process.platform}-${process.arch}`);
const nativeTest=(name,fn)=>test(name,{skip:!process.env.PHOENIX_DIAGRAM_BIN?'Requires explicit PHOENIX_DIAGRAM_BIN from V-10; npm test does not build or download':!supportedUnix?'Native engine supports only the four declared Unix targets':false},fn);
const filesystemTest=(name,fn)=>test(name,{skip:!supportedUnix?'POSIX resource checks apply to the four declared Unix targets':false},fn);
let native;
function realEngine() {
  if (native) return native;
  const file = process.env.PHOENIX_DIAGRAM_BIN;
  assert.ok(file,'Real engine tests require explicit PHOENIX_DIAGRAM_BIN; no automatic debug build lookup');
  assert.ok(fs.existsSync(file), 'A built native engine is required; this success path never uses a fake probe');
  const probe = spawnSync(file, ['engine-probe'], { encoding: 'utf8' });
  assert.equal(probe.status, 0, probe.stderr);
  assert.equal(probe.stdout, 'phoenix-diagram-engine 0.1.0 input=1 receipt=1 viewer=phoenix-v1\n');
  const doctor = spawnSync(file, ['doctor', '--json'], { encoding: 'utf8' });
  const engine = JSON.parse(doctor.stdout).engine;
  assert.equal(engine.inputContractSchema, 1);
  const bytes=fs.readFileSync(file);
  assert.equal(hash(bytes),engine.binary.sha256,'Native build changed during snapshot; provide a stable PHOENIX_DIAGRAM_BIN');
  const directory=fs.mkdtempSync(path.join(os.tmpdir(),'phoenix-diagram native snapshot '));
  const snapshot=path.join(directory,'phoenix-diagram');
  write(snapshot,bytes,0o700);
  process.once('exit',()=>fs.rmSync(directory,{recursive:true,force:true}));
  return native = { file:snapshot, bytes, engine };
}
function write(file, bytes, mode = 0o644) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, bytes, { mode });
  fs.chmodSync(file, mode);
}
function fixture(t, changes = {}) {
  const temp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram bootstrap ')));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const pkg = path.join(temp, 'package');
  const dist = path.join(pkg, 'dist/phoenix-diagram');
  const bundleRoot = path.join(dist, 'bundle');
  const project = path.join(temp, 'project with spaces');
  const home = path.join(temp, 'diagram-home');
  const release = path.join(temp, 'release');
  fs.mkdirSync(path.join(project, '.git'), { recursive: true });
  const packageJsonPath = path.join(pkg, 'package.json');
  write(packageJsonPath, JSON.stringify({ name: '@xgent-ai/skills', version: packageVersion() }));
  write(path.join(pkg,'bin/phoenix-diagram.js'),fs.readFileSync(path.join(root,'bin/phoenix-diagram.js')),0o755);
  for(const name of ['phoenix-bootstrap.cjs','phoenix-launcher.cjs','phoenix-diagram'])write(path.join(pkg,'skills/phoenix-diagram/scripts',name),fs.readFileSync(path.join(root,'skills/phoenix-diagram/scripts',name)),name==='phoenix-bootstrap.cjs'?0o644:0o755);
  const files = [
    ['SKILL.md', Buffer.from('---\nname: phoenix-diagram\ndescription: Diagram bootstrap fixture\n---\n'), 0o644],
    ...['phoenix-bootstrap.cjs', 'phoenix-launcher.cjs', 'phoenix-diagram'].map(name => [
      `scripts/${name}`, fs.readFileSync(path.join(root, 'skills/phoenix-diagram/scripts', name)), name === 'phoenix-bootstrap.cjs' ? 0o644 : 0o755,
    ]),
    ...['architecture', 'workflow', 'sequence', 'dataflow', 'lifecycle', 'common'].map(kind => [
      `schemas/${kind}.schema.json`, fs.readFileSync(path.join(root, 'skills/phoenix-diagram/schemas', `${kind}.schema.json`)), 0o644,
    ]),
  ];
  for (const [name, bytes, mode] of files) write(path.join(bundleRoot, name), bytes, mode);
  const inventory = { schemaVersion: 1, tool: 'phoenix-diagram', version: '0.1.0', files: files.map(([name, bytes, mode]) => ({ path: name, size: bytes.length, sha256: hash(bytes), mode })) };
  write(path.join(bundleRoot, 'manifest.json'), JSON.stringify(inventory) + '\n');
  // Metadata-only fixtures cannot execute successfully. They keep structural
  // and pre-acquisition refusal tests available in a fresh source checkout.
  const {bytes,engine}=process.env.PHOENIX_DIAGRAM_BIN?realEngine():{bytes:Buffer.from('Inert pre-execution metadata fixture; not an engine.'),engine:{sourceInputSha256:'a'.repeat(64)}};
  const engines = Object.fromEntries(boot().TARGETS.map(target => [target, { asset: `phoenix-diagram-${target}`, size: bytes.length, sha256: hash(bytes) }]));
  const version = { schemaVersion: 1, distribution: 'development', tool: 'phoenix-diagram', toolVersion: '0.1.0', npmPackageVersion: packageVersion(), sourceCommit: 'a'.repeat(40), sourceInputSha256: engine.sourceInputSha256, inputContractSchema: 1, receiptSchema: 1, viewerRevision: 'phoenix-v1', bundleSha256: hash(fs.readFileSync(path.join(bundleRoot, 'manifest.json'))), engines, ...changes };
  const manifestPath = path.join(dist, 'VERSION.json');
  const env = { ...process.env, PHOENIX_DIAGRAM_HOME: home, PHOENIX_UI_HOME: path.join(temp, 'ui-home'), HTTP_PROXY: '', http_proxy: '', HTTPS_PROXY: '', https_proxy: '', ALL_PROXY: '', all_proxy: '', NO_PROXY: '', no_proxy: '' };
  const f = { temp, pkg, dist, bundleRoot, project, home, release, packageJsonPath, manifestPath, inventory, version, env, bytes, target: supportedUnix?boot().platformTarget():boot().TARGETS[0] };
  f.options = { project, manifestPath, bundleRoot, packageJsonPath, env };
  f.seal = () => {
    version.bundleSha256 = hash(fs.readFileSync(path.join(bundleRoot, 'manifest.json')));
    const encoded = JSON.stringify(version) + '\n';
    write(manifestPath, encoded); write(path.join(bundleRoot, 'ENGINE.json'), encoded);
  };
  f.seal();
  const license = fs.readFileSync(path.join(root, 'tools/phoenix-ui/LICENSE'));
  write(path.join(dist, 'LICENSE'), fs.readFileSync(path.join(root, 'LICENSE')));
  write(path.join(dist, 'NOTICE.md'), fs.readFileSync(path.join(root, 'tools/phoenix-diagram/NOTICE.md')));
  write(path.join(dist, 'THIRD-PARTY-LICENSES/apache-2.0.txt'), license);
  write(path.join(dist, 'THIRD-PARTY.json'), JSON.stringify({ schemaVersion: 1, rust: [{ name: 'bootstrap-license-fixture', licenses: [{ path: 'THIRD-PARTY-LICENSES/apache-2.0.txt', sha256: hash(license) }] }] }));
  write(path.join(dist, 'THIRD-PARTY-NOTICES.md'), 'Fixture catalog exercises full local license bytes; this is not release license acceptance.\n');
  f.offline = () => {
    fs.rmSync(release, { recursive: true, force: true });
    fs.cpSync(dist, release, { recursive: true });
    // Real host bytes are replicated only to test four-resource inventory.
    // Other-platform native execution is a separate acceptance test.
    for (const meta of Object.values(version.engines)) fs.copyFileSync(realEngine().file, path.join(release, meta.asset), fs.constants.COPYFILE_FICLONE);
    return release;
  };
  return f;
}
function run(f, args, overrides = {}) {
  const script = `require(${JSON.stringify(source)}).main(${JSON.stringify(args)},${JSON.stringify({ ...f.options, env: undefined })}).then(code=>{process.exitCode=code;});`;
  return command(process.execPath, ['-e', script], { cwd: f.project, env: { ...f.env, ...overrides } });
}
function command(file, args, options) {
  return new Promise((resolve, reject) => {
    const child = spawn(file, args, { ...options, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    child.stdout.on('data', bytes => { stdout += bytes; });
    child.stderr.on('data', bytes => { stderr += bytes; });
    child.once('error', reject); child.once('close', code => resolve({ code, stdout, stderr }));
  });
}
async function serve(t, handler) {
  const server = http.createServer(handler);
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise(resolve => server.close(resolve)));
  return `http://127.0.0.1:${server.address().port}`;
}

test('bootstrap entry points exist and expose the isolated preflight API', () => {
  for (const file of [source, path.join(root, 'skills/phoenix-diagram/scripts/phoenix-launcher.cjs'), path.join(root, 'skills/phoenix-diagram/scripts/phoenix-diagram'), path.join(root, 'bin/phoenix-diagram.js')]) assert.ok(fs.existsSync(file), file);
  for (const name of ['preflight', 'manifestAt', 'verifyBundle', 'verifyEngine', 'verifyReceipt', 'enginePath', 'installEngine', 'main']) assert.equal(typeof boot()[name], 'function', name);
});
test('only four declared Unix platforms are accepted; proxy precedence stays explicit', () => {
  const { TARGETS, platformTarget, proxyFor } = boot();
  assert.deepEqual(TARGETS, ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64']);
  for (const target of TARGETS) assert.equal(platformTarget(...target.split('-')), target);
  assert.throws(() => platformTarget('win32', 'x64'), /Unsupported/);
  const url = new URL('https://download.example.test/engine');
  assert.equal(proxyFor(url, { HTTP_PROXY: 'http://127.0.0.1:1', HTTPS_PROXY: 'http://127.0.0.1:2' }).port, '2');
  assert.equal(proxyFor(url, { HTTPS_PROXY: 'http://127.0.0.1:2', NO_PROXY: '.example.test' }), null);
});
test('help and pinned recovery instructions work without manifests, engines or network', async t => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram help '));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const pkg = path.join(temp, 'package.json');
  write(pkg, JSON.stringify({ name: '@xgent-ai/skills', version: packageVersion() }));
  const env = { ...process.env, PHOENIX_DIAGRAM_HOME: path.join(temp, 'cache') };
  for (const args of [['--help'], ['--version'], ['recover-output','--help']]) {
    const script = `require(${JSON.stringify(source)}).main(${JSON.stringify(args)},${JSON.stringify({ manifestPath: path.join(temp, 'missing'), packageJsonPath: pkg })}).then(c=>process.exitCode=c);`;
    const result = await command(process.execPath, ['-e', script], { cwd: temp, env });
    assert.equal(result.code, 0, result.stderr); assert.match(result.stdout, /phoenix-diagram/);
    if(args[0]==='recover-output') {
      assert.match(result.stdout,/P\/\.phoenix-diagram-recovery-<id>\/publication-recovery-v1\.json/);
      assert.match(result.stdout,/P\/\.phoenix-diagram-recovery-<id>\.cleanup\.json/);
      assert.match(result.stdout,/P\/\.phoenix-diagram-output-cleanup\/<id>\.json/);
      assert.match(result.stdout,/owner-v1\.json/);
      assert.match(result.stdout,/original recovery directory.*even if.*disappeared/i);
      assert.match(result.stdout,/unknown.*ambiguous.*refus/i);
    }
  }
  assert.equal(fs.existsSync(env.PHOENIX_DIAGRAM_HOME), false);
});
filesystemTest('ordinary missing engine performs no installation and names the current fixed package', async t => {
  const f = fixture(t); let requests = 0;
  const url = await serve(t, (_req, res) => { requests++; res.end(f.bytes); });
  f.version.distribution = 'candidate';
  for (const meta of Object.values(f.version.engines)) meta.url = `${url}/engine`;
  f.seal();
  const result = await run(f, ['guide', 'workflow', '--json']);
  assert.equal(result.code, 4, result.stderr);
  const receipt = JSON.parse(result.stdout);
  assert.equal(receipt.status, 'unavailable');
  assert.ok(receipt.setupCommand.includes(path.join(f.pkg, 'bin/phoenix-diagram.js')));
  assert.doesNotMatch(receipt.setupCommand, /npm exec|--package=@xgent-ai\/skills/);
  assert.match(receipt.setupCommand, /engine install --project/);
  assert.equal(requests, 0); assert.equal(fs.existsSync(f.home), false);
  assert.deepEqual(fs.readdirSync(f.project), ['.git']);
});
filesystemTest('only release recovery points to the registry; nonrelease recovery keeps the local package or installed skill', t => {
  const f = fixture(t), { setupCommand, shellQuote, manifestAt } = boot();
  for (const distribution of ['development', 'candidate', 'release']) {
    f.version.distribution = distribution;
    for (const meta of Object.values(f.version.engines)) meta.url = 'https://download.example.test/engine';
    f.seal();
    const manifest = manifestAt(f.manifestPath), command = setupCommand(manifest, f.project, { ...f.options, releaseDir: f.release });
    assert.ok(command.includes(`--project ${shellQuote(f.project)}`));
    if (distribution === 'release') {
      assert.match(command, new RegExp(`npm exec --package=@xgent-ai/skills@${packageVersion()}`));
    } else {
      assert.ok(command.includes(shellQuote(path.join(f.pkg, 'bin/phoenix-diagram.js'))));
      assert.ok(command.includes(`--release-dir ${shellQuote(f.release)}`));
      assert.doesNotMatch(command, /npm exec|--package=@xgent-ai\/skills/);
      const installed = setupCommand(manifest, f.project, { skillDir: path.join(f.project, '.agents/skills/phoenix-diagram') });
      assert.ok(installed.includes(path.join(f.project, '.agents/skills/phoenix-diagram/scripts/phoenix-launcher.cjs')));
    }
  }
  assert.doesNotMatch(setupCommand(null, f.project, { ...f.options, manifestPath: path.join(f.temp, 'missing') }), /npm exec|--package=@xgent-ai\/skills/);
});
nativeTest('nonrelease recovery instructions restore an empty cache from the matching offline resources', async t => {
  for (const distribution of ['development', 'candidate']) {
    const f = fixture(t, { distribution });
    if (distribution === 'development') {
      f.version.engines = { [f.target]: f.version.engines[f.target] };
      f.seal();
    }
    if (distribution === 'candidate') {
      for (const meta of Object.values(f.version.engines)) meta.url = 'http://127.0.0.1:1/unavailable';
      f.seal();
    }
    f.offline();
    const unavailable = await run(f, ['doctor', '--json']);
    assert.equal(unavailable.code, 4);
    assert.equal(fs.existsSync(f.home), false);
    const instruction = JSON.parse(unavailable.stdout).setupCommand;
    assert.doesNotMatch(instruction, /npm exec|--package=@xgent-ai\/skills/);
    // The resource directory is supplied by the candidate recipient, as the
    // package's recovery instructions require; no registry lookup is involved.
    const recovery = instruction.replace(boot().shellQuote('MATCHING_RELEASE_DIR'), boot().shellQuote(f.release));
    assert.notEqual(recovery, instruction);
    const restored = await command('/bin/sh', ['-c', recovery], { cwd: f.project, env: f.env });
    assert.equal(restored.code, 0, restored.stderr);
    assert.equal(hash(fs.readFileSync(boot().enginePath(boot().manifestAt(f.manifestPath), f.target, f.env))), hash(f.bytes));
    const doctor = await run(f, ['doctor', '--json']);
    assert.equal(JSON.parse(doctor.stdout).recovery.engineInstall, instruction);
    assert.deepEqual(fs.readdirSync(f.project), ['.git']);
  }
});
filesystemTest('explicit acquisition rejects invalid project arguments without writing state', async t => {
  const f = fixture(t);
  const missing = path.join(f.temp, 'missing project');
  const regular = path.join(f.temp, 'not a directory');
  write(regular, 'regular file');
  for (const args of [
    ['engine', 'install', '--json'],
    ['engine', 'install', '--project', f.project, '--project', f.project, '--json'],
    ['engine', 'install', '--project', f.project, '--unknown', '--json'],
    ['engine', 'install', '--project', missing, '--json'],
    ['engine', 'install', '--project', regular, '--json'],
  ]) {
    const result = await run(f, args);
    assert.equal(result.code, 2, `${args.join(' ')}: ${result.stderr}`);
    assert.equal(JSON.parse(result.stdout).status, 'unavailable');
  }
  assert.equal(fs.existsSync(missing), false);
  assert.equal(fs.existsSync(f.home), false);
  assert.deepEqual(fs.readdirSync(f.project), ['.git']);
});
nativeTest('explicit complete offline installation verifies a real engine in a digest-addressed cache', async t => {
  const f = fixture(t); f.offline();
  const installed=await command(process.execPath,[path.join(f.pkg,'bin/phoenix-diagram.js'),'engine','install','--project',f.project,'--release-dir',f.release,'--json'],{cwd:f.project,env:f.env});
  assert.equal(installed.code,0,installed.stderr);assert.equal(JSON.parse(installed.stdout).scope,'engine-cache');
  const ready = await boot().preflight(f.options);
  assert.equal(ready.bundleRoot, f.bundleRoot); assert.equal(ready.versionManifestPath, f.manifestPath);
  assert.equal(ready.version.npmPackageVersion, packageVersion());
  const expected = path.join(f.home, 'bin', '0.1.0', f.target, f.version.engines[f.target].sha256, 'phoenix-diagram');
  assert.equal(ready.enginePath, expected); assert.equal(hash(fs.readFileSync(expected)), hash(f.bytes));
  assert.equal(fs.existsSync(f.env.PHOENIX_UI_HOME), false); assert.deepEqual(fs.readdirSync(f.project), ['.git']);
  const scenario = 'workflow ; $(touch never-created) with spaces';
  const actual = await command(process.execPath,[path.join(f.pkg,'bin/phoenix-diagram.js'),'guide',scenario,'--json'],{cwd:f.project,env:f.env});
  const direct = await command(realEngine().file, ['guide', scenario, '--json'], { cwd: f.project, env: { ...f.env, PHOENIX_DIAGRAM_BUNDLE_ROOT: f.bundleRoot } });
  assert.equal(actual.code, direct.code, actual.stderr); assert.deepEqual(JSON.parse(actual.stdout), JSON.parse(direct.stdout));
  assert.equal(JSON.parse(actual.stdout).scenario, scenario);
  const helpArgs=['recover-output','--help'];
  const nativeHelp=await command(realEngine().file,helpArgs,{cwd:f.project,env:f.env});
  const launchedHelp=await command(process.execPath,[path.join(f.pkg,'bin/phoenix-diagram.js'),...helpArgs],{cwd:f.project,env:f.env});
  assert.equal(launchedHelp.code,nativeHelp.code);assert.equal(launchedHelp.stdout,nativeHelp.stdout,'Verified native command help is forwarded intact');
  assert.equal(fs.existsSync(path.join(f.project, 'never-created')), false);
});
filesystemTest('package identity, bundle inventory, ENGINE bytes and file modes are verified before execution', t => {
  const f = fixture(t);
  for (const [field, value] of [['tool', 'phoenix-ui'], ['toolVersion', '0.2.0'], ['npmPackageVersion', '9.9.9'], ['inputContractSchema', 2], ['receiptSchema', 2], ['viewerRevision', 'foreign'], ['sourceCommit', 'bad'], ['sourceInputSha256', 'bad']]) {
    const original = f.version[field]; f.version[field] = value; f.seal();
    assert.throws(() => boot().manifestAt(f.manifestPath, { packageJsonPath: f.packageJsonPath }), undefined, field);
    f.version[field] = original;
  }
  f.seal();
  const manifest = boot().manifestAt(f.manifestPath, { packageJsonPath: f.packageJsonPath });
  assert.ok(boot().verifyBundle(f.bundleRoot, manifest));
  fs.appendFileSync(path.join(f.bundleRoot, 'ENGINE.json'), ' ');
  assert.throws(() => boot().verifyBundle(f.bundleRoot, manifest), /ENGINE/);
  f.seal();
  fs.chmodSync(path.join(f.bundleRoot, 'scripts/phoenix-launcher.cjs'), 0o644);
  assert.throws(() => boot().verifyBundle(f.bundleRoot, manifest), /mode|bundle/i);
});
filesystemTest('bundle paths, symlinks, unlisted files and derived ENGINE entries are refused', t => {
  const f = fixture(t);
  const raw = fs.readFileSync(path.join(f.bundleRoot, 'manifest.json'));
  for (const name of ['../outside', '/absolute', 'hooks/settings.json', 'scripts/phoenix-ui', 'ENGINE.json']) {
    const altered = structuredClone(f.inventory); altered.files[0].path = name;
    write(path.join(f.bundleRoot, 'manifest.json'), JSON.stringify(altered)); f.seal();
    assert.throws(() => boot().verifyBundle(f.bundleRoot, boot().manifestAt(f.manifestPath)), undefined, name);
  }
  write(path.join(f.bundleRoot, 'manifest.json'), raw); f.seal();
  write(path.join(f.bundleRoot, 'unlisted'), 'x');
  assert.throws(() => boot().verifyBundle(f.bundleRoot, boot().manifestAt(f.manifestPath)), /extra|unlisted/i);
  fs.unlinkSync(path.join(f.bundleRoot, 'unlisted'));
  fs.renameSync(path.join(f.bundleRoot, 'SKILL.md'), path.join(f.temp, 'skill-backup'));
  fs.symlinkSync(path.join(f.temp, 'skill-backup'), path.join(f.bundleRoot, 'SKILL.md'));
  assert.throws(() => boot().verifyBundle(f.bundleRoot, boot().manifestAt(f.manifestPath)), /symlink|regular/i);
});
nativeTest('offline completeness and byte identity are rechecked even with a valid cache; no network fallback', async t => {
  const f = fixture(t); let requests = 0;
  const url = await serve(t, (_req, res) => { requests++; res.end(f.bytes); });
  f.version.distribution = 'candidate'; for (const meta of Object.values(f.version.engines)) meta.url = `${url}/engine`;
  f.seal(); f.offline();
  await boot().preflight({ ...f.options, releaseDir: f.release, allowInstall: true });
  for (const file of ['VERSION.json', ...Object.values(f.version.engines).map(e => e.asset), 'LICENSE', 'NOTICE.md', 'THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md', 'THIRD-PARTY-LICENSES/apache-2.0.txt', 'bundle/manifest.json', 'bundle/ENGINE.json']) {
    const full = path.join(f.release, file); fs.renameSync(full, `${full}.held`);
    await assert.rejects(boot().preflight({ ...f.options, releaseDir: f.release, allowInstall: true }), undefined, file);
    fs.renameSync(`${full}.held`, full);
  }
  fs.appendFileSync(path.join(f.release, 'VERSION.json'), ' ');
  await assert.rejects(boot().preflight({ ...f.options, releaseDir: f.release, allowInstall: true }), /manifest|VERSION/i);
  assert.equal(requests, 0);
});
nativeTest('candidate loopback download is explicit, bounded and reverified on every cache hit', async t => {
  const f = fixture(t); let requests = 0; let corrupt = true;
  const url = await serve(t, (_req, res) => { requests++; res.end(corrupt ? Buffer.alloc(f.bytes.length, 0) : f.bytes); });
  f.version.distribution = 'candidate'; for (const meta of Object.values(f.version.engines)) meta.url = `${url}/engine`;
  f.seal();
  await assert.rejects(boot().preflight({ ...f.options, allowInstall: true }), /digest|size/);
  corrupt = false;
  const ready = await boot().preflight({ ...f.options, allowInstall: true });
  assert.equal(requests, 2);
  await boot().preflight(f.options); assert.equal(requests, 2);
  fs.appendFileSync(ready.enginePath, 'tampered');
  const ordinary = await run(f, ['guide', 'workflow', '--json']);
  assert.equal(ordinary.code, 4); assert.equal(requests, 2);
  assert.match(ordinary.stderr, /digest|size/);
});
test('unsafe URLs and missing platform assets fail before acquisition', t => {
  const f = fixture(t);
  for (const [distribution, url] of [['release', 'http://127.0.0.1/engine'], ['candidate', 'http://example.test/engine'], ['candidate', 'https://user:secret@example.test/engine'], ['candidate', 'file:///tmp/engine']]) {
    f.version.distribution = distribution; for (const meta of Object.values(f.version.engines)) meta.url = url;
    f.seal(); assert.throws(() => boot().manifestAt(f.manifestPath));
  }
  f.version.distribution='candidate';for(const meta of Object.values(f.version.engines))meta.url='https://download.example.test/engine';
  delete f.version.engines['linux-arm64']; f.seal(); assert.throws(() => boot().manifestAt(f.manifestPath), /platform|target/i);
});
filesystemTest('excessive redirects and oversized bodies fail closed without publishing an engine', async t => {
  const f=fixture(t);f.version.distribution='candidate';
  const url = await serve(t, (req, res) => { res.writeHead(302, { Location: req.url }); res.end(); });
  for (const meta of Object.values(f.version.engines)) meta.url = `${url}/loop`; f.seal();
  await assert.rejects(boot().preflight({ ...f.options, allowInstall: true }), /redirect/i);
  assert.equal(fs.existsSync(f.home), false);
  const oversized = await serve(t, (_req, res) => res.end('too many bytes'));
  for (const meta of Object.values(f.version.engines)) meta.url = `${oversized}/engine`;
  f.version.engines[f.target] = { ...f.version.engines[f.target], size: 1, sha256: hash(Buffer.from('x')) }; f.seal();
  await assert.rejects(boot().preflight({ ...f.options, allowInstall: true }), /exceeded.*size/i);
  assert.equal(fs.existsSync(f.home), false);
});
filesystemTest('a digest-matching foreign probe is rejected and never published to cache', async t => {
  const f=fixture(t);
  // Fake executables are confined to rejection tests. Every success uses the
  // immutable snapshot of the actual compiled native engine above.
  const bytes=Buffer.from('#!/usr/bin/env node\nconsole.log("phoenix-ui-engine 0.1.0");\n');
  const url=await serve(t,(_req,res)=>res.end(bytes));
  f.version.distribution='candidate';for(const meta of Object.values(f.version.engines))meta.url=`${url}/engine`;
  f.version.engines[f.target]={...f.version.engines[f.target],size:bytes.length,sha256:hash(bytes)};f.seal();
  await assert.rejects(boot().preflight({...f.options,allowInstall:true}),/probe identity/);
  const directory=path.dirname(boot().enginePath(boot().manifestAt(f.manifestPath),f.target,f.env));
  assert.deepEqual(fs.readdirSync(directory),[]);
});
nativeTest('staged executable publication retries only bounded transient locks and cleans failures', async t => {
  const f=fixture(t);f.offline();let attempts=0;const rename=fs.renameSync;
  t.mock.method(fs,'renameSync',(from,to)=>{
    attempts++;
    if(attempts<=2)throw Object.assign(new Error('temporary executable lock'),{code:attempts===1?'EBUSY':'EPERM'});
    return rename(from,to);
  });
  const ready=await boot().preflight({...f.options,releaseDir:f.release,allowInstall:true});
  assert.equal(attempts,3);assert.ok(fs.existsSync(ready.enginePath));
  assert.deepEqual(fs.readdirSync(path.dirname(ready.enginePath)),['phoenix-diagram']);
});
nativeTest('persistent staged rename failures stop after six attempts and preserve the original cause', async t => {
  const f=fixture(t);f.offline();let attempts=0;
  t.mock.method(fs,'renameSync',()=>{attempts++;throw Object.assign(new Error('persistent executable lock'),{code:'EBUSY'});});
  await assert.rejects(boot().preflight({...f.options,releaseDir:f.release,allowInstall:true}),error=>error.code==='EBUSY'&&error.message==='persistent executable lock');
  assert.equal(attempts,6);
  const directory=path.dirname(boot().enginePath(boot().manifestAt(f.manifestPath),f.target,f.env));
  assert.deepEqual(fs.readdirSync(directory),[]);
});
nativeTest('HTTP proxy actually transports the fixed candidate asset and arguments stay native', async t => {
  const f = fixture(t); let requests = 0;
  const proxy = await serve(t, (req, res) => { requests++; assert.equal(req.url, 'http://127.0.0.1:1/pinned'); assert.equal(req.headers['proxy-authorization'], `Basic ${Buffer.from('fixture:only').toString('base64')}`); res.end(f.bytes); });
  f.version.distribution = 'candidate'; for (const meta of Object.values(f.version.engines)) meta.url = 'http://127.0.0.1:1/pinned'; f.seal();
  await boot().preflight({ ...f.options, allowInstall: true, env: { ...f.env, HTTP_PROXY: proxy.replace('://', '://fixture:only@') } });
  assert.equal(requests, 1);
});
nativeTest('HTTPS validates certificates, uses the CONNECT tunnel, and refuses downgrade redirects', async t => {
  const certificate=path.join(__dirname,'fixtures/bootstrap/tls-cert.pem');
  const key=path.join(__dirname,'fixtures/bootstrap/tls-key.pem');
  const f=fixture(t);let tlsRequests=0;let requireProxy=false;const proxyPorts=new Set();const sockets=new Set();
  let trapRequests=0;
  const trap=await serve(t,(_req,res)=>{trapRequests++;res.end(f.bytes);});
  const secure=https.createServer({key:fs.readFileSync(key),cert:fs.readFileSync(certificate)},(req,res)=>{
    tlsRequests++;
    if(req.url==='/downgrade'){res.writeHead(302,{Location:`${trap}/engine`});res.end();return;}
    if(requireProxy&&!proxyPorts.has(req.socket.remotePort)){res.writeHead(403);res.end('CONNECT tunnel was bypassed');return;}
    res.end(f.bytes);
  });
  secure.on('connection',socket=>{sockets.add(socket);socket.once('close',()=>sockets.delete(socket));});
  secure.on('tlsClientError',()=>{});
  await new Promise(resolve=>secure.listen(0,'127.0.0.1',resolve));
  t.after(()=>{for(const socket of sockets)socket.destroy();return new Promise(resolve=>secure.close(resolve));});
  const url=`https://127.0.0.1:${secure.address().port}/engine`;
  f.version.distribution='release';for(const meta of Object.values(f.version.engines))meta.url=url;f.seal();
  const args=['engine','install','--project',f.project,'--json'];
  const untrusted=await command(process.execPath,[path.join(f.pkg,'bin/phoenix-diagram.js'),...args],{cwd:f.project,env:{...f.env,NODE_TLS_REJECT_UNAUTHORIZED:'0'}});
  assert.equal(untrusted.code,4);assert.match(untrusted.stderr,/certificate|self.signed/i);assert.equal(tlsRequests,0);
  const env={...f.env,NODE_EXTRA_CA_CERTS:certificate};
  const trusted=await command(process.execPath,[path.join(f.pkg,'bin/phoenix-diagram.js'),...args],{cwd:f.project,env});
  assert.equal(trusted.code,0,trusted.stderr);assert.equal(tlsRequests,1);
  const g=fixture(t);g.version.distribution='release';for(const meta of Object.values(g.version.engines))meta.url=url;g.seal();
  let connects=0;let authority=`127.0.0.1:${secure.address().port}`;const proxy=http.createServer();
  proxy.on('connect',(req,client,head)=>{
    connects++;assert.equal(req.url,authority);
    const upstream=net.connect(secure.address().port,'127.0.0.1',()=>{
      proxyPorts.add(upstream.localPort);client.write('HTTP/1.1 200 Connection Established\r\n\r\n');
      if(head.length)upstream.write(head);client.pipe(upstream);upstream.pipe(client);
    });
    for(const socket of [client,upstream]){sockets.add(socket);socket.once('close',()=>sockets.delete(socket));socket.on('error',()=>{client.destroy();upstream.destroy();});}
  });
  await new Promise(resolve=>proxy.listen(0,'127.0.0.1',resolve));
  t.after(()=>{for(const socket of sockets)socket.destroy();return new Promise(resolve=>proxy.close(resolve));});
  requireProxy=true;
  const tunneled=await command(process.execPath,[path.join(g.pkg,'bin/phoenix-diagram.js'),'engine','install','--project',g.project,'--json'],{cwd:g.project,env:{...g.env,NODE_EXTRA_CA_CERTS:certificate,HTTPS_PROXY:`http://127.0.0.1:${proxy.address().port}`}});
  assert.equal(tunneled.code,0,tunneled.stderr);assert.equal(connects,1);assert.equal(tlsRequests,2);
  const wrongHost=fixture(t);wrongHost.version.distribution='release';for(const meta of Object.values(wrongHost.version.engines))meta.url=url.replace('127.0.0.1','127.0.0.2');wrongHost.seal();
  authority=`127.0.0.2:${secure.address().port}`;
  const mismatch=await command(process.execPath,[path.join(wrongHost.pkg,'bin/phoenix-diagram.js'),'engine','install','--project',wrongHost.project,'--json'],{cwd:wrongHost.project,env:{...wrongHost.env,NODE_EXTRA_CA_CERTS:certificate,HTTPS_PROXY:`http://127.0.0.1:${proxy.address().port}`}});
  assert.equal(mismatch.code,4);assert.match(mismatch.stderr,/certificate.*(?:IP|altnames)|IP.*cert/i);assert.equal(connects,2);assert.equal(tlsRequests,2);
  requireProxy=false;
  const h=fixture(t);h.version.distribution='candidate';for(const meta of Object.values(h.version.engines))meta.url=url.replace('/engine','/downgrade');h.seal();
  const downgraded=await command(process.execPath,[path.join(h.pkg,'bin/phoenix-diagram.js'),'engine','install','--project',h.project,'--json'],{cwd:h.project,env:{...h.env,NODE_EXTRA_CA_CERTS:certificate}});
  assert.equal(downgraded.code,4);assert.match(downgraded.stderr,/HTTPS downgrade/);assert.equal(trapRequests,0);
});
nativeTest('a real native install yields a working installed launcher without an installed manifest.json', async t => {
  const f = fixture(t); f.offline();
  await boot().preflight({ ...f.options, allowInstall: true, releaseDir: f.release });
  const installed = await command(realEngine().file, ['install', '--project', f.project, '--bundle-root', f.bundleRoot, '--providers', 'codex', '--json'], { cwd: f.project, env: f.env });
  assert.equal(installed.code, 0, `Real native installer is required: ${installed.stderr}\n${installed.stdout}`);
  const skill = path.join(f.project, '.agents/skills/phoenix-diagram');
  assert.equal(fs.existsSync(path.join(f.project,'.codex/skills/phoenix-diagram')),false,'Codex selects .codex but manages the skill only under .agents');
  assert.equal(fs.existsSync(path.join(skill, 'manifest.json')), false);
  const actual = await command(process.execPath, [path.join(skill, 'scripts/phoenix-launcher.cjs'), 'guide', 'workflow', '--json'], { cwd: f.project, env: f.env });
  assert.equal(actual.code, 0, actual.stderr); assert.equal(JSON.parse(actual.stdout).resourceRoot, skill);
  const shell=await command(path.join(skill,'scripts/phoenix-diagram'),['guide','workflow','--json'],{cwd:f.project,env:f.env});
  assert.equal(shell.code,0,shell.stderr);assert.deepEqual(JSON.parse(shell.stdout),JSON.parse(actual.stdout));
  const launch=()=>command(process.execPath,[path.join(skill,'scripts/phoenix-launcher.cjs'),'guide','workflow','--json'],{cwd:f.project,env:f.env});
  fs.rmSync(f.home, { recursive: true });
  const unavailable = await launch();
  assert.equal(unavailable.code, 4);
  const recovery = JSON.parse(unavailable.stdout).setupCommand;
  assert.ok(recovery.includes(path.join(skill, 'scripts/phoenix-launcher.cjs')));
  assert.doesNotMatch(recovery, /npm exec|--package=@xgent-ai\/skills/);
  const restored = await command('/bin/sh', ['-c', recovery.replace(boot().shellQuote('MATCHING_RELEASE_DIR'), boot().shellQuote(f.release))], { cwd: f.project, env: f.env });
  assert.equal(restored.code, 0, restored.stderr);
  assert.equal((await launch()).code, 0);
  const activePath=path.join(f.project,'.phoenix-diagram/install/active.json');
  const activeBytes=fs.readFileSync(activePath);
  const active=JSON.parse(activeBytes);
  assert.deepEqual(active.providers,['.agents','.codex']);
  const journalPath=path.join(f.project,active.journal);
  const journalBytes=fs.readFileSync(journalPath);
  const cleanup=path.join(path.dirname(journalPath),'.phoenix-diagram-output-cleanup');
  const ownerPath=path.join(cleanup,'owner-v1.json'),ownerBytes=fs.readFileSync(ownerPath);
  fs.appendFileSync(ownerPath,' ');
  const changedOwner=await launch();assert.equal(changedOwner.code,4);assert.match(changedOwner.stderr,/cleanup owner/i);
  write(ownerPath,ownerBytes,0o600);
  const foreignCleanup=path.join(cleanup,'unknown.json');write(foreignCleanup,'{}\n',0o600);
  const pendingCleanup=await launch();assert.equal(pendingCleanup.code,4);assert.match(pendingCleanup.stderr,/cleanup.*recovery/i);
  fs.unlinkSync(foreignCleanup);
  const lockPath=path.join(path.dirname(journalPath),fs.readdirSync(path.dirname(journalPath)).find(name=>name.startsWith('.phoenix-diagram-lock-')));
  const lockBytes=fs.readFileSync(lockPath);fs.appendFileSync(lockPath,'\n');
  const changedLock=await launch();assert.equal(changedLock.code,4);assert.match(changedLock.stderr,/publication lock/i);
  write(lockPath,lockBytes,0o600);
  fs.appendFileSync(activePath,' ');
  const corrupt=await launch();assert.equal(corrupt.code,4);assert.match(corrupt.stderr,/checksum|canonical/i);
  write(activePath,activeBytes,0o600);
  const journal=JSON.parse(journalBytes);journal.phase='applying';journal.checksum='';journal.checksum=hash(Buffer.from(JSON.stringify(journal)+'\n'));
  write(journalPath,JSON.stringify(journal)+'\n',0o600);
  const pending=await launch();assert.equal(pending.code,4);assert.match(pending.stderr,/completed native journal/i);
  write(journalPath,journalBytes,0o600);
  const unknownJournal=path.join(path.dirname(journalPath),'unknown.json');write(unknownJournal,'{}\n',0o600);
  const unknown=await launch();assert.equal(unknown.code,4);assert.match(unknown.stderr,/unknown installation journal/i);
  fs.unlinkSync(unknownJournal);
  const extra=path.join(skill,'unmanaged-extra.md');write(extra,'unlisted file');
  const unlisted=await launch();assert.equal(unlisted.code,4);assert.match(unlisted.stderr,/unlisted|extra/i);
  fs.unlinkSync(extra);
  assert.equal((await launch()).code,0,'Restoring the original bytes and removing extras restores the real native ownership proof');
  fs.appendFileSync(path.join(skill, 'SKILL.md'), 'changed');
  const refused = await command(process.execPath, [path.join(skill, 'scripts/phoenix-launcher.cjs'), 'guide', 'workflow', '--json'], { cwd: f.project, env: f.env });
  assert.equal(refused.code, 4); assert.match(refused.stderr, /ownership|managed|digest/i);
});
