'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const crypto = require('node:crypto');
const { spawn, spawnSync } = require('node:child_process');
const publisher = process.env.PHOENIX_PUBLICATION_TEST_PUBLISHER || path.resolve(__dirname, '../../scripts/publish-phoenix-ui-r2.mjs');
const checkout = path.resolve(__dirname, '../..');
const packageBytes = fs.readFileSync(path.join(checkout, 'package.json'));
const binBytes = fs.readFileSync(path.join(checkout, 'bin/phoenix-ui.js'));
const portableBytes = fs.readFileSync(path.join(checkout, 'skills/phoenix-ui/src/scripts/phoenix-ui'));
const bootstrapBytes = fs.readFileSync(path.join(checkout, 'skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs'));
const launcherBytes = fs.readFileSync(path.join(checkout, 'skills/phoenix-ui/src/scripts/phoenix-launcher.cjs'));
const bootstrap = require('../../skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs');
const tools = require('../../tools/phoenix-ui/build-tools.lock.json');
const targets = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64'];
const hosts = ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'];
const phases = {
  prepare: 'Explicit online preparation', audit: 'Audit prepared dependencies offline',
  light: 'Lightweight repository tests', build: 'Build native and WASM from the prepared cache',
  smoke: 'Native and portable Node runtime smoke', runtime: 'Full Phoenix runtime tests offline',
};
const runId = '123456', head = 'a'.repeat(40), tree = 'b'.repeat(40);
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const json = value => JSON.stringify(value, null, 2) + '\n';
const write = (file, bytes) => { fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, bytes); };
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8'));
function updateInputs(f, target, change) {
  const dir = path.join(f.extracted, `phoenix-ui-${target}`), file = path.join(dir, 'build-record.json');
  const record = readJson(file); change(record.inputs); record.inputSha256 = hash(JSON.stringify(record.inputs)); write(file,json(record));
  const manifest = readJson(path.join(dir,'VERSION.json')); manifest.sourceInputSha256 = record.inputSha256;
  for (const name of ['VERSION.json','runtime/vendor/phoenix-ui/VERSION.json','runtime/skills/phoenix-ui/scripts/ENGINE.json']) write(path.join(dir,name),json(manifest));
  const smoke = readJson(path.join(dir,'runtime-smoke-node25.json')); smoke.manifestSha256 = hash(json(manifest)); write(path.join(dir,'runtime-smoke-node25.json'),json(smoke));
  if (target === 'linux-x64') { const file = path.join(f.extracted,'phoenix-ui-node18-consumer-evidence/runtime-smoke-node18.json'); const node18 = readJson(file); node18.manifestSha256 = smoke.manifestSha256; write(file,json(node18)); }
  const generatedFile = path.join(dir,'runtime/skills/phoenix-ui/GENERATED.json'), generated = readJson(generatedFile);
  generated.outputs.skill['scripts/ENGINE.json'] = hash(json(manifest));
  for (const name of Object.keys(generated.outputs.providers)) generated.outputs.providers[name] = hash(json(manifest));
  write(generatedFile,json(generated));
}
const winapiDeclarations = [
  ['winapi-i686-pc-windows-gnu', 'ac3b87c63620426dd9b991e5ce0329eff545bccbbb34f3be09ff6fb6ab51b7b6'],
  ['winapi-x86_64-pc-windows-gnu', '712e227841d057c1ee1cd2fb22fa7e5a5461ae8e48fa2ca79ec42cfc1931183f'],
].map(([name,checksum]) => ({name,version:'0.4.0',license:'MIT/Apache-2.0',checksum,sourceUnmodified:true,sourceUrl:`https://static.crates.io/crates/${name}/${name}-0.4.0.crate`,licenses:[],licenseLocations:[`https://docs.rs/crate/${name}/0.4.0/source/Cargo.toml.orig`,`https://docs.rs/crate/${name}/0.4.0/source/src/lib.rs`]}));
function addLicenseDeclaration(f, entries, kind = 'rust') {
  for (const target of targets) {
    const dir=path.join(f.extracted,`phoenix-ui-${target}`), catalog=readJson(path.join(dir,'THIRD-PARTY.json'));
    catalog[kind].push(...entries);
    for (const name of ['THIRD-PARTY.json','runtime/vendor/phoenix-ui/THIRD-PARTY.json']) write(path.join(dir,name),json(catalog));
  }
  f.repack();
}

async function fixture(t, control = {}) {
  const evidence = process.env.PHOENIX_PUBLICATION_EVIDENCE_DIR;
  if (evidence) fs.mkdirSync(evidence, { recursive: true });
  const root = fs.mkdtempSync(path.resolve(evidence || os.tmpdir(), 'm6-publication-'));
  const extracted = path.join(root, 'fixtures'), archives = path.join(root, 'archives');
  const outputs = [], requests = [], results = [], objects = new Map(), publicObjects = new Map(), publishers = new Set();
  let releaseHold, reportHeld, holdOnce = false;
  const hold = new Promise(resolve => { releaseHold = resolve; });
  const held = new Promise(resolve => { reportHeld = resolve; });
  let failed = false, rendezvous;
  const barrier = new Promise(resolve => { rendezvous = resolve; });
  let firstPuts = 0;
  const server = http.createServer(async (request, response) => {
    const key = decodeURIComponent(request.url.replace(/^\/(?:bucket|public)\//, ''));
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    const bytes = Buffer.concat(chunks);
    const event = { method: request.method, route: request.url.startsWith('/public/') ? 'public' : 'storage', key, condition: request.headers['if-none-match'], size: bytes.length, sha256: hash(bytes) };
    requests.push(event);
    const finish = (status, body = '') => { event.status = status; response.writeHead(status); response.end(body); };
    if (request.method === 'PUT') {
      if (control.concurrent && key.includes('darwin-arm64')) {
        if (++firstPuts === 2) rendezvous();
        await Promise.race([barrier, new Promise(resolve => { const timer = setTimeout(resolve, 10000); timer.unref(); })]);
      }
      response.setHeader('content-type', 'application/xml');
      if (event.condition !== '*') return finish(400, '<Error><Code>InvalidRequest</Code></Error>');
      if (control.holdKey && key.includes(control.holdKey) && !holdOnce) {
        holdOnce = true; reportHeld(event); await hold;
        return finish(503, '<Error><Code>ServiceUnavailable</Code><Message>owned publisher interruption</Message></Error>');
      }
      if (control.failKey && key.includes(control.failKey) && !failed) {
        failed = true; return finish(503, '<Error><Code>ServiceUnavailable</Code></Error>');
      }
      if (objects.has(key)) return finish(412, '<Error><Code>PreconditionFailed</Code><Message>key exists</Message></Error>');
      objects.set(key, bytes);
      response.setHeader('etag', `"${hash(bytes)}"`);
      return finish(200);
    }
    if (request.method === 'GET' && objects.has(key)) {
      const bytes = event.route === 'public' && publicObjects.has(key) ? publicObjects.get(key) : objects.get(key);
      return finish(200, control.corruptReadback ? Buffer.alloc(bytes.length, 0x78) : control.shortReadback ? bytes.subarray(1) : bytes);
    }
    return finish(404, '<Error><Code>NoSuchKey</Code></Error>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  t.after(async () => {
    releaseHold();
    for (const active of publishers) active.child.kill('SIGKILL');
    await Promise.all([...publishers].map(active => active.result));
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
    if (evidence) {
      write(path.join(root, 'results.json'), json({ test: t.name, adapter: process.env.PHOENIX_PUBLICATION_REAL_AWS ? 'real-aws-cli-and-curl' : 'aws-cli-protocol-fixture-and-real-curl', results, requests, objects: [...objects].map(([key, bytes]) => ({ key, size: bytes.length, sha256: hash(bytes) })) }));
    } else fs.rmSync(root, { recursive: true, force: true });
  });
  const blob = Buffer.from('Tested resource bytes.\n'), blobHash = hash(blob);
  const bundle = { schema: 1, derivedRuntimeManifest: 'skills/phoenix-ui/scripts/ENGINE.json', providers: { '.claude': { files: { 'skills/phoenix-ui/SKILL.md': blobHash, 'skills/phoenix-ui/scripts/phoenix-ui': hash(portableBytes), 'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs': hash(bootstrapBytes), 'skills/phoenix-ui/scripts/phoenix-launcher.cjs': hash(launcherBytes) }, exec: ['skills/phoenix-ui/scripts/phoenix-ui'] } } };
  const bundleBytes = Buffer.from(json(bundle));
  const license = Buffer.from('Fixture license, no real native acceptance.\n');
  const licenses = [{ path: `THIRD-PARTY-LICENSES/${hash(license)}.txt`, sha256: hash(license) }];
  const inventory = { schemaVersion: 1, inputs: {}, rust: [{ name: 'fixture', version: '1.0.0', license: 'MIT', licenses }], node: [], bundled: [], resources: [] };
  const meta = { schemaVersion: 1, distribution: 'development', toolVersion: '0.1.0', npmPackageVersion: '0.7.0-rc.0',
    sourceCommit: head, sourceTree: tree, sourceState: 'dirty', bundleSchema: 1, bundleSha256: hash(bundleBytes), reviewSchema: 1 };
  const items = [], jobs = [];
  for (let i = 0; i < targets.length; i++) {
    const target = targets[i], dir = path.join(extracted, `phoenix-ui-${target}`);
    const asset = `phoenix-ui-${target}`;
    const binary = Buffer.from("#!/usr/bin/env node\nconsole.log(process.argv[2] === 'engine-probe' ? 'phoenix-ui-engine 0.1.0' : '0.1.0');\n");
    const engine = { asset, size: binary.length, sha256: hash(binary) };
    const inputs = { 'package.json': hash(packageBytes), 'bin/phoenix-ui.js': hash(binBytes), 'tools/phoenix-ui/Cargo.toml': 'c'.repeat(64), 'tools/phoenix-ui/crates/live/assets/bundle-inputs.json': hash(target), 'tools/phoenix-ui/crates/live/assets/detect-antipatterns-browser.js': hash(`generated-${target}`) };
    const manifest = { ...meta, sourceInputSha256: hash(JSON.stringify(inputs)), engines: { [target]: engine } };
    write(path.join(dir, asset), binary);
    fs.chmodSync(path.join(dir, asset), 0o755);
    write(path.join(dir, 'VERSION.json'), json(manifest));
    write(path.join(dir, 'build-record.json'), json({ schemaVersion: 1, sourceCommit: head, sourceTree: tree, sourceState: 'dirty', target, host: hosts[i], tools, inputs, inputSha256: manifest.sourceInputSha256, binary: engine, bundleSha256: meta.bundleSha256, checks: { engineIdentity: 'passed', browserInputs: 'passed', skillGeneration: 'passed', fullRuntimeTests: 'not-run-by-build-script' } }));
    const checks = ['engine-probe', 'version', 'root-help', 'local-command-catalog', 'static-html-detect', 'local-context', 'hook-session-start', 'node-missing-engine', 'node-explicit-offline-install', 'node-engine-version', 'node-context', 'local-bundle-check'].map(name => ({ name, exitCode: name === 'static-html-detect' ? 2 : name === 'node-missing-engine' ? 4 : 0 }));
    const smoke = { schemaVersion: 1, status: 'passed', target, mode: 'native', host: { platform: target.split('-')[0], architecture: target.split('-')[1], node: 'v25.2.1' }, manifestSha256: hash(json(manifest)), binarySha256: engine.sha256, checks };
    write(path.join(dir, 'runtime-smoke-node25.json'), json(smoke));
    write(path.join(dir, 'runtime/vendor/phoenix-ui/bundle/manifest.json'), bundleBytes);
    write(path.join(dir, `runtime/vendor/phoenix-ui/bundle/blobs/${blobHash}`), blob);
    for (const data of [portableBytes, bootstrapBytes, launcherBytes]) write(path.join(dir, `runtime/vendor/phoenix-ui/bundle/blobs/${hash(data)}`), data);
    write(path.join(dir, 'runtime/skills/phoenix-ui/SKILL.md'), blob);
    write(path.join(dir, 'runtime/bin/phoenix-ui.js'), binBytes);
    fs.chmodSync(path.join(dir, 'runtime/bin/phoenix-ui.js'), 0o755);
    write(path.join(dir, 'runtime/skills/phoenix-ui/scripts/phoenix-ui'), portableBytes);
    fs.chmodSync(path.join(dir, 'runtime/skills/phoenix-ui/scripts/phoenix-ui'), 0o755);
    write(path.join(dir, 'runtime/skills/phoenix-ui/scripts/phoenix-bootstrap.cjs'), bootstrapBytes);
    write(path.join(dir, 'runtime/skills/phoenix-ui/scripts/phoenix-launcher.cjs'), launcherBytes);
    for (const name of ['runtime/vendor/phoenix-ui/VERSION.json', 'runtime/skills/phoenix-ui/scripts/ENGINE.json']) write(path.join(dir, name), json(manifest));
    const generatedInputs = { 'tools/phoenix-ui/Cargo.toml': inputs['tools/phoenix-ui/Cargo.toml'] };
    write(path.join(dir, 'runtime/skills/phoenix-ui/GENERATED.json'), json({ schemaVersion: 1, generator: 'Bun 1.3.14', inputSha256: hash(JSON.stringify(generatedInputs)), inputs: generatedInputs, bundleSha256: meta.bundleSha256, engineManifest: 'verified', outputs: { skill: { 'SKILL.md': blobHash, 'scripts/ENGINE.json': hash(json(manifest)), 'scripts/phoenix-ui': hash(portableBytes), 'scripts/phoenix-bootstrap.cjs': hash(bootstrapBytes), 'scripts/phoenix-launcher.cjs':hash(launcherBytes) }, providers: { 'claude-code/.claude/skills/phoenix-ui/scripts/ENGINE.json': hash(json(manifest)), 'codex/.codex/skills/phoenix-ui/scripts/ENGINE.json': hash(json(manifest)) }, bundle: { 'manifest.json': hash(bundleBytes), [`blobs/${blobHash}`]: blobHash, [`blobs/${hash(portableBytes)}`]: hash(portableBytes), [`blobs/${hash(bootstrapBytes)}`]: hash(bootstrapBytes), [`blobs/${hash(launcherBytes)}`]:hash(launcherBytes) }, notices: { LICENSE: hash('Fixture LICENSE\n'), 'NOTICE.md': hash('Fixture NOTICE.md\n') } } }));
    for (const prefix of ['', 'runtime/vendor/phoenix-ui/']) {
      write(path.join(dir, `${prefix}THIRD-PARTY.json`), json(inventory));
      write(path.join(dir, `${prefix}THIRD-PARTY-NOTICES.md`), 'Fixture notices.\n');
      write(path.join(dir, prefix, licenses[0].path), license);
    }
    for (const name of ['LICENSE', 'NOTICE.md']) write(path.join(dir, 'runtime/vendor/phoenix-ui', name), `Fixture ${name}\n`);
    const evidenceDir = path.join(extracted, `phoenix-ui-${target}-evidence`);
    write(path.join(evidenceDir, `${target}-ci-results.json`), json({ schemaVersion: 1, workflowRun: runId, sourceCommit: head, target, phases: Object.fromEntries(Object.keys(phases).map(name => [name, 'success'])) }));
    jobs.push({ id: i + 10, run_id: Number(runId), head_sha: head, name: `Native ${target}`, status: 'completed', conclusion: 'success', steps: Object.values(phases).map(name => ({ name, status: 'completed', conclusion: 'success' })) });
    items.push(`phoenix-ui-${target}`, `phoenix-ui-${target}-evidence`);
    if (target === 'linux-x64') write(path.join(extracted, 'phoenix-ui-node18-consumer-evidence/runtime-smoke-node18.json'), json({ ...smoke, host: { ...smoke.host, node: 'v18.20.8' } }));
  }
  items.push('phoenix-ui-node18-consumer-evidence');
  jobs.push({ id: 20, run_id: Number(runId), head_sha: head, name: 'Node 18 portable consumer smoke', status: 'completed', conclusion: 'success', steps: [{ name: 'Run portable local installation and runtime on Node 18', status: 'completed', conclusion: 'success' }] });
  const run = { id: Number(runId), head_sha: head, status: 'completed', conclusion: 'success', repository: { id: 1, full_name: 'XGENT-ai/skills' }, head_repository: { id: 1, full_name: 'XGENT-ai/skills' } };
  const artifacts = { total_count: items.length, artifacts: items.map((name, i) => ({ id: i + 100, name, expired: false, workflow_run: { id: Number(runId), repository_id: 1, head_repository_id: 1, head_sha: head } })) };
  function repack() {
    fs.mkdirSync(archives, { recursive: true });
    const packed = spawnSync('python3', ['-c', 'import sys,json,zipfile,pathlib\np=json.loads(sys.stdin.read())\nfor name in p["names"]:\n with zipfile.ZipFile(pathlib.Path(p["archives"])/(name+".zip"),"w",zipfile.ZIP_DEFLATED) as z:\n  d=pathlib.Path(p["extracted"])/name\n  for f in sorted(d.rglob("*")):\n   if f.is_file() or f.is_dir(): z.write(f,str(f.relative_to(d)))',], { input: JSON.stringify({ names: items, archives, extracted }), encoding: 'utf8' });
    assert.equal(packed.status, 0, packed.stderr);
    for (const artifact of artifacts.artifacts) artifact.digest = `sha256:${hash(fs.readFileSync(path.join(archives, `${artifact.name}.zip`)))}`;
    write(path.join(root, 'run.json'), json(run));
    write(path.join(root, 'jobs.json'), json({ total_count: jobs.length, jobs }));
    write(path.join(root, 'artifacts.json'), json(artifacts));
  }
  repack();
  const fakeBin = path.join(root, 'fake-bin');
  write(path.join(fakeBin, 'aws'), `#!/usr/bin/env node\nconst fs = require('node:fs');\nconst args = process.argv.slice(2);\nconst get = flag => args[args.indexOf(flag) + 1];\nif (args.includes('--generate-cli-skeleton')) { console.log('{"IfNoneMatch":""}'); process.exit(0); }\nconst put = args[1] === 'put-object', download = args[1] === 'get-object';\nif (args[0] !== 's3api' || (!put && !download) || (put && get('--if-none-match') !== '*')) process.exit(2);\nfetch(get('--endpoint-url') + '/' + get('--bucket') + '/' + get('--key'), put ? {method:'PUT',headers:{'if-none-match':'*'},body:fs.readFileSync(get('--body'))} : {}).then(async r => { if(!r.ok){ const body = await r.text(); console.error(body);process.exitCode=1; }else{if(download)fs.writeFileSync(args.at(-1),Buffer.from(await r.arrayBuffer()));console.log('{}');}}).catch(e => { console.error(e.message);process.exitCode=1; });\n`);
  fs.chmodSync(path.join(fakeBin, 'aws'), 0o755);
  const publisherTemp = path.resolve(root, 'publisher-temp'); fs.mkdirSync(publisherTemp);
  const env = { ...process.env, AGENT_RELEASE_R2_BUCKET: 'bucket', AGENT_RELEASE_R2_ENDPOINT: base,
    AGENT_RELEASE_R2_PUBLIC_BASE: `${base}/public`, AGENT_RELEASE_R2_PREFIX: '', AGENT_RELEASE_R2_ACCESS_KEY: 'fixture-access', AGENT_RELEASE_R2_SECRET_KEY: 'fixture-secret',
    AWS_MAX_ATTEMPTS: '1', NO_PROXY: '127.0.0.1', no_proxy: '127.0.0.1', TMPDIR: publisherTemp };
  if (!process.env.PHOENIX_PUBLICATION_REAL_AWS) env.PATH = [fakeBin, process.env.PATH].join(path.delimiter);
  function startPublisher(extra = [], output = path.join(root, `release-${outputs.length}`)) {
    outputs.push(output);
    const args = [publisher, '--run-id', runId, '--source-commit', head, '--source-tree', tree,
      '--run-record', path.join(root, 'run.json'), '--jobs-record', path.join(root, 'jobs.json'), '--artifacts-record', path.join(root, 'artifacts.json'), '--archives-dir', archives, '--output-dir', output, ...extra];
    const child = spawn(process.execPath, args, { env, stdio: ['ignore', 'pipe', 'pipe'] });
    const active = { child, output };
    active.result = new Promise((resolve, reject) => {
      let stdout = '', stderr = '';
      child.stdout.on('data', data => { stdout += data; }); child.stderr.on('data', data => { stderr += data; });
      child.once('error', reject);
      child.once('close', (code, signal) => { publishers.delete(active); const result = { code, signal, pid: child.pid, stdout, stderr, output, argv: args }; results.push(result); resolve(result); });
    });
    publishers.add(active);
    return active;
  }
  const publish = (extra, output) => startPublisher(extra, output).result;
  return { root, extracted, archives, run, jobs, artifacts, repack, publish, startPublisher, held, releaseHold, requests, objects, publicObjects, meta, env };
}

test('publication preserves ZIP Unix 755 engine and launcher permissions so frozen entrypoints execute directly', async t => {
  const f = await fixture(t);
  const archive = path.join(f.archives, 'phoenix-ui-darwin-arm64.zip');
  const attributes = spawnSync('python3', ['-c', 'import sys,zipfile,json\nwith zipfile.ZipFile(sys.argv[1]) as z: print(json.dumps({n:oct(z.getinfo(n).external_attr >> 16) for n in ["phoenix-ui-darwin-arm64","runtime/bin/phoenix-ui.js"]}))', archive], {encoding:'utf8'});
  assert.equal(attributes.status, 0, attributes.stderr);
  assert.deepEqual(JSON.parse(attributes.stdout), { 'phoenix-ui-darwin-arm64': '0o100755', 'runtime/bin/phoenix-ui.js': '0o100755' });
  const result = await f.publish(); assert.equal(result.code, 0, result.stderr);
  const engine = path.join(result.output, 'phoenix-ui-darwin-arm64');
  const launcher = path.join(result.output, 'runtime/bin/phoenix-ui.js');
  const project = path.join(f.root, 'entrypoint-project'); fs.mkdirSync(path.join(project,'.git'), {recursive:true});
  const env = {...f.env, PHOENIX_UI_HOME:path.join(f.root,'entrypoint-cache')};
  const install = spawnSync(launcher, ['engine','install','--project',project,'--release-dir',result.output,'--json'], {env,encoding:'utf8'});
  const runs = [[engine, ['engine-probe'], 'phoenix-ui-engine 0.1.0'], [launcher, ['--version'], '0.1.0'], [path.join(result.output, 'runtime/skills/phoenix-ui/scripts/phoenix-ui'), ['--version'], '0.1.0']].map(([file, args, expected]) => ({ file, mode: fs.statSync(file).mode & 0o7777, expected, result: spawnSync(file, args, {env,cwd:project,encoding:'utf8'}) }));
  write(path.join(f.root, 'entrypoint-results.json'), json(runs.map(r => ({file:r.file,mode:r.mode,status:r.result.status,error:r.result.error?.message,stdout:r.result.stdout,stderr:r.result.stderr}))));
  assert.equal(install.status,0,install.error?.message || install.stderr);
  for (const run of runs) { assert.equal(run.mode, 0o755); assert.equal(run.result.status, 0, run.result.error?.message || run.result.stderr); assert.equal(run.result.stdout.trim(), run.expected); }
});

test('publication restores 644 transport entrypoints only through verified engine, bundle exec and npm bin roles', async t => {
  const f = await fixture(t);
  for (const target of targets) for (const name of [`phoenix-ui-${target}`, 'runtime/bin/phoenix-ui.js', 'runtime/skills/phoenix-ui/scripts/phoenix-ui']) fs.chmodSync(path.join(f.extracted,`phoenix-ui-${target}`,name),0o644);
  f.repack();
  const result = await f.publish(); assert.equal(result.code,0,result.stderr);
  const project = path.join(f.root,'transport-project'); fs.mkdirSync(path.join(project,'.git'),{recursive:true});
  const env = {...f.env,PHOENIX_UI_HOME:path.join(f.root,'transport-cache')}, launcher = path.join(result.output,'runtime/bin/phoenix-ui.js');
  const install = spawnSync(launcher,['engine','install','--project',project,'--release-dir',result.output,'--json'],{env,cwd:project,encoding:'utf8'});
  const entrypoints = [`phoenix-ui-${bootstrap.platformTarget()}`, 'runtime/bin/phoenix-ui.js', 'runtime/skills/phoenix-ui/scripts/phoenix-ui'];
  const runs = entrypoints.map(name => { const file=path.join(result.output,name); const run=spawnSync(file,['--version'],{env,cwd:project,encoding:'utf8'}); return {name,mode:fs.statSync(file).mode&0o7777,status:run.status,error:run.error?.message,stdout:run.stdout,stderr:run.stderr}; });
  write(path.join(f.root,'transport-entrypoint-results.json'),json({install:{status:install.status,error:install.error?.message,stdout:install.stdout,stderr:install.stderr},runs}));
  assert.equal(install.status,0,install.error?.message || install.stderr);
  for (const run of runs) { assert.equal(run.mode,0o755); assert.equal(run.status,0,run.error||run.stderr); assert.equal(run.stdout.trim(),'0.1.0'); }
  assert.equal(fs.statSync(path.join(result.output,'runtime/skills/phoenix-ui/scripts/phoenix-bootstrap.cjs')).mode&0o7777,0o644);
  const handoff=readJson(path.join(result.output,'release-handoff.json'));
  assert.deepEqual([...new Set(handoff.transportPermissions.map(p=>p.role))].sort(),['bundle-exec','engine','npm-bin']);
  assert(handoff.transportPermissions.every(p=>p.before===0o644&&p.after===0o755));
});

for (const kind of ['package checkout mismatch','missing npm bin anchor','npm bin byte drift']) test(`publication refuses a candidate with ${kind} before upload`, async t => {
  const f = await fixture(t);
  for (const target of targets) {
    if (kind === 'package checkout mismatch') updateInputs(f,target,inputs=>{inputs['package.json']='d'.repeat(64);});
    if (kind === 'missing npm bin anchor') updateInputs(f,target,inputs=>{delete inputs['bin/phoenix-ui.js'];});
    if (kind === 'npm bin byte drift') fs.appendFileSync(path.join(f.extracted,`phoenix-ui-${target}`,'runtime/bin/phoenix-ui.js'),'// modified runtime\n');
  }
  f.repack(); const result = await f.publish();
  assert.equal(result.code,1); assert.match(result.stderr,/checkout package|candidate npm bin/i);
  assert.equal(f.requests.length,0);
});

test('publication explicitly resumes after SIGKILL of its owned publisher while retaining already uploaded objects', {timeout:15000}, async t => {
  const f = await fixture(t, {holdKey:'darwin-x64'}), active = f.startPublisher();
  await f.held;
  const lock = path.join(active.output, '.publish-lock/owner.lock');
  const lockInode = fs.statSync(lock).ino;
  assert.equal(f.objects.size, 1);
  assert.equal(readJson(path.join(active.output, 'publication-record.json')).objects[0].upload, 'uploaded');
  assert.equal(active.child.kill('SIGKILL'), true);
  const killed = await active.result; assert.equal(killed.signal, 'SIGKILL');
  f.releaseHold();
  assert(fs.statSync(path.join(active.output, '.publish-lock')).isDirectory());
  const retry = await f.publish(['--resume'], active.output);
  assert.equal(retry.code, 0, retry.stderr);
  assert.equal(f.objects.size, 4);
  assert.equal(readJson(path.join(active.output, 'publication-record.json')).objects.filter(o => o.upload === 'reused').length, 1);
  assert.equal(f.requests.filter(r => r.method === 'PUT' && r.status === 200).length, 4);
  assert.equal(fs.statSync(lock).ino, lockInode, 'Resume must keep the original OS lock inode');
});

test('publication resume cannot acquire the output OS lock of a live owned publisher', {timeout:15000}, async t => {
  const f = await fixture(t, {holdKey:'darwin-x64'}), active = f.startPublisher();
  await f.held;
  const lock = path.join(active.output, '.publish-lock/owner.lock'), inode = fs.statSync(lock).ino;
  const retry = await f.publish(['--resume'], active.output);
  assert.equal(retry.code, 1); assert.match(retry.stderr, /publication output is locked/i);
  assert.equal(f.objects.size, 1);
  assert.equal(fs.statSync(lock).ino, inode);
  assert.equal(active.child.kill('SIGKILL'), true); await active.result; f.releaseHold();
});

test('publication rejects FIFO inputs without blocking or reading them', {timeout:5000}, async t => {
  const f = await fixture(t), file = path.join(f.root, 'run.json'); fs.unlinkSync(file);
  const fifo = spawnSync('python3', ['-c', 'import os,sys;os.mkfifo(sys.argv[1])', file], {encoding:'utf8'});
  assert.equal(fifo.status, 0, fifo.stderr);
  const result = await f.publish();
  assert.equal(result.code, 1); assert.match(result.stderr, /Not a regular file/);
  assert.equal(f.requests.length, 0);
});

test('publication restores ordinary Unix file and directory modes while dropping special permission bits', async t => {
  const f = await fixture(t);
  const modes = Object.fromEntries(targets.map(target => [`phoenix-ui-${target}`, {
    [`phoenix-ui-${target}`]: 0o104755, 'runtime/bin/phoenix-ui.js': 0o106755,
    'runtime/bin/': 0o043750, 'runtime/vendor/phoenix-ui/NOTICE.md': 0o106640,
  }]));
  const rewrite = spawnSync('python3', ['-c', 'import sys,json,zipfile,os\np=json.loads(sys.stdin.read())\nfor name,modes in p["modes"].items():\n archive=os.path.join(p["archives"],name+".zip")\n with zipfile.ZipFile(archive) as source: items=[(i,source.read(i.filename)) for i in source.infolist()]\n with zipfile.ZipFile(archive,"w") as destination:\n  for i,data in items:\n   if i.filename in modes: i.external_attr=(modes[i.filename]<<16)|(i.external_attr&0xffff)\n   destination.writestr(i,data)',], {input:JSON.stringify({archives:f.archives,modes}),encoding:'utf8'});
  assert.equal(rewrite.status, 0, rewrite.stderr);
  for (const artifact of f.artifacts.artifacts) artifact.digest = `sha256:${hash(fs.readFileSync(path.join(f.archives, `${artifact.name}.zip`)))}`;
  write(path.join(f.root, 'artifacts.json'), json(f.artifacts));
  const result = await f.publish(); assert.equal(result.code, 0, result.stderr);
  const actual = Object.fromEntries(['phoenix-ui-darwin-arm64', 'runtime/bin/phoenix-ui.js', 'runtime/bin', 'runtime/vendor/phoenix-ui/NOTICE.md'].map(name => [name, fs.statSync(path.join(result.output,name)).mode & 0o7777]));
  write(path.join(f.root, 'safe-mode-results.json'), json({zipModes:modes,outputModes:actual}));
  assert.deepEqual(actual, {'phoenix-ui-darwin-arm64':0o755,'runtime/bin/phoenix-ui.js':0o755,'runtime/bin':0o750,'runtime/vendor/phoenix-ui/NOTICE.md':0o640});
});

for (const kind of ['archive input', 'output directory', 'control directory', 'lock file']) test(`publication refuses a symlink or unexpected type for its ${kind} without changing the target`, async t => {
  const f = await fixture(t);
  const sentinel = path.join(f.root, 'user-sentinel'); write(sentinel, 'user bytes\n');
  const output = path.join(f.root, 'safe-output');
  if (kind === 'archive input') {
    const zip = path.join(f.archives, 'phoenix-ui-darwin-arm64.zip'), original = `${zip}.original`;
    fs.renameSync(zip, original); fs.symlinkSync(path.resolve(original), zip);
  } else if (kind === 'output directory') fs.symlinkSync(path.resolve(f.extracted), output);
  else {
    fs.mkdirSync(output);
    if (kind === 'control directory') fs.symlinkSync(path.resolve(f.extracted), path.join(output, '.publish-lock'));
    else { fs.mkdirSync(path.join(output, '.publish-lock')); fs.symlinkSync(path.resolve(sentinel), path.join(output, '.publish-lock/owner.lock')); }
  }
  const result = await f.publish(kind === 'archive input' ? [] : ['--resume'], output);
  assert.equal(result.code, 1); assert.match(result.stderr, /ELOOP|ENOTDIR|symbolic|symlink/i);
  assert.equal(f.requests.length, 0); assert.equal(fs.readFileSync(sentinel,'utf8'), 'user bytes\n');
});

test('publication refuses a non-regular control lock without truncating it', async t => {
  const f = await fixture(t), output = path.join(f.root, 'directory-lock');
  fs.mkdirSync(path.join(output, '.publish-lock/owner.lock'), {recursive:true});
  write(path.join(output, '.publish-lock/owner.lock/user-data'), 'keep\n');
  const result = await f.publish(['--resume'], output);
  assert.equal(result.code, 1); assert.match(result.stderr, /Is a directory|EISDIR|Invalid publication lock/);
  assert.equal(f.requests.length, 0);
  assert.equal(fs.readFileSync(path.join(output, '.publish-lock/owner.lock/user-data'),'utf8'), 'keep\n');
});

test('publication uploads only tested bytes and freezes a complete bootstrap manifest after every public readback', async t => {
  const f = await fixture(t), result = await f.publish();
  assert.equal(result.code, 0, result.stderr);
  const manifest = bootstrap.manifestAt(path.join(result.output, 'VERSION.json'));
  assert.equal(manifest.data.distribution, 'release');
  assert.deepEqual(Object.keys(manifest.data.engines), targets);
  assert.equal(manifest.data.sourceTree, tree);
  assert.equal(f.objects.size, 4);
  assert.equal(f.requests.filter(r => r.method === 'GET').length, 4);
  assert(f.requests.filter(r => r.method === 'PUT').every(r => r.condition === '*' && r.status === 200));
  assert(f.requests.every(r => r.method !== 'HEAD'));
  const handoff = readJson(path.join(result.output, 'release-handoff.json'));
  assert.equal(handoff.manifestSha256, manifest.hash);
  assert.equal(handoff.artifacts.length, 9);
  assert.deepEqual(handoff.allowedInputDifferences, ['tools/phoenix-ui/crates/live/assets/bundle-inputs.json', 'tools/phoenix-ui/crates/live/assets/detect-antipatterns-browser.js']);
  assert.deepEqual(Object.keys(handoff.platforms), targets);
  assert.equal(handoff.bundleSha256, f.meta.bundleSha256);
  assert(fs.readFileSync(path.join(result.output, 'runtime/skills/phoenix-ui/scripts/ENGINE.json')).equals(manifest.bytes));
  const generated = readJson(path.join(result.output, 'runtime/skills/phoenix-ui/GENERATED.json'));
  for (const [name, digest] of Object.entries(generated.outputs.providers)) if (name.endsWith('/scripts/ENGINE.json')) assert.equal(digest, manifest.hash);
  const target = bootstrap.platformTarget();
  const installed = await bootstrap.installEngine(manifest, target, undefined, result.output, { ...f.env, PHOENIX_UI_HOME: path.join(f.root, 'cache') });
  assert.equal(hash(fs.readFileSync(installed)), manifest.data.engines[target].sha256);
  const downloaded = await bootstrap.installEngine(manifest, target, undefined, undefined, { ...f.env, PHOENIX_UI_HOME: path.join(f.root, 'http-cache') });
  assert.equal(hash(fs.readFileSync(downloaded)), manifest.data.engines[target].sha256);
  assert.equal(f.requests.filter(r => r.method === 'GET' && r.route === 'public').length, 5);
  assert.equal(handoff.localSubstitute, true);
  assert.equal(handoff.finalTarballAcceptance, 'not-run');
});

test('publication consumes the two locked CI22 winapi target declarations without invented license terms', async t => {
  const f=await fixture(t); addLicenseDeclaration(f,winapiDeclarations);
  const result=await f.publish(); assert.equal(result.code,0,result.stderr);
  const catalog=readJson(path.join(result.output,'THIRD-PARTY.json'));
  assert.deepEqual(catalog.rust.slice(-2),winapiDeclarations);
  assert.equal(f.objects.size,4);
});

const badLicenseDeclarations = {
  'unknown dependency without full terms': entry => { entry.name='not-the-locked-winapi'; },
  'another version without full terms': entry => { entry.version='0.4.1'; },
  'MPL without full terms': entry => { entry.name='selectors'; entry.license='MPL-2.0'; },
  'different license expression': entry => { entry.license='GPL-3.0'; },
  'wrong locked checksum': entry => { entry.checksum='0'.repeat(64); },
  'wrong crate archive source': entry => { entry.sourceUrl='https://example.invalid/winapi.crate'; },
  'modified crate source': entry => { entry.sourceUnmodified=false; },
  'missing exact source locations': entry => { entry.licenseLocations.pop(); },
  'arbitrary source locations': entry => { entry.licenseLocations[1]='https://example.invalid/LICENSE'; },
};
for (const [name,change] of Object.entries(badLicenseDeclarations)) test(`publication rejects ${name} rather than replacing license terms with locations`, async t => {
  const f=await fixture(t), entry=structuredClone(winapiDeclarations[0]); change(entry); addLicenseDeclaration(f,[entry]);
  const result=await f.publish(); assert.equal(result.code,1); assert.match(result.stderr,/Missing preserved license/);
  assert.equal(f.requests.length,0); assert(!fs.existsSync(path.join(result.output,'VERSION.json')));
});
test('publication does not apply the Rust declaration exception to a Node inventory entry', async t => {
  const f=await fixture(t); addLicenseDeclaration(f,winapiDeclarations,'node');
  const result=await f.publish(); assert.equal(result.code,1); assert.match(result.stderr,/Missing preserved license/);
  assert.equal(f.requests.length,0);
});

const preflightCases = {
  'missing platform': f => { f.artifacts.artifacts = f.artifacts.artifacts.filter(a => a.name !== 'phoenix-ui-linux-arm64'); f.artifacts.total_count--; },
  'failed required job step': f => { f.jobs[0].steps.find(s => s.name === phases.runtime).conclusion = 'failure'; },
  'failed phase record': f => { const p = path.join(f.extracted, 'phoenix-ui-darwin-arm64-evidence/darwin-arm64-ci-results.json'); const r = readJson(p); r.phases.runtime = 'cancelled'; write(p, json(r)); },
  'wrong source commit': f => { f.run.head_sha = 'c'.repeat(40); },
  'wrong source tree': f => { const p = path.join(f.extracted, 'phoenix-ui-darwin-arm64/build-record.json'); const r = readJson(p); r.sourceTree = 'c'.repeat(40); write(p, json(r)); },
  'wrong repository': f => { f.run.repository.full_name = 'untrusted/skills'; },
  'wrong build input digest': f => { const p = path.join(f.extracted, 'phoenix-ui-darwin-arm64/build-record.json'); const r = readJson(p); r.inputSha256 = 'c'.repeat(64); write(p, json(r)); },
  'unknown cross-platform input difference': f => {
    const dir = path.join(f.extracted, 'phoenix-ui-linux-arm64'), p = path.join(dir, 'build-record.json');
    const r = readJson(p); r.inputs['tools/phoenix-ui/Cargo.toml'] = 'd'.repeat(64); r.inputSha256 = hash(JSON.stringify(r.inputs)); write(p, json(r));
    const m = readJson(path.join(dir, 'VERSION.json')); m.sourceInputSha256 = r.inputSha256;
    for (const name of ['VERSION.json', 'runtime/vendor/phoenix-ui/VERSION.json', 'runtime/skills/phoenix-ui/scripts/ENGINE.json']) write(path.join(dir, name), json(m));
    const smoke = readJson(path.join(dir, 'runtime-smoke-node25.json')); smoke.manifestSha256 = hash(json(m)); write(path.join(dir, 'runtime-smoke-node25.json'), json(smoke));
    const g = readJson(path.join(dir, 'runtime/skills/phoenix-ui/GENERATED.json')); g.outputs.skill['scripts/ENGINE.json'] = hash(json(m));
    for (const name of Object.keys(g.outputs.providers)) g.outputs.providers[name] = hash(json(m));
    g.inputs['tools/phoenix-ui/Cargo.toml'] = r.inputs['tools/phoenix-ui/Cargo.toml']; g.inputSha256 = hash(JSON.stringify(g.inputs)); write(path.join(dir, 'runtime/skills/phoenix-ui/GENERATED.json'), json(g));
  },
  'changed tested binary': f => { write(path.join(f.extracted, 'phoenix-ui-darwin-arm64/phoenix-ui-darwin-arm64'), 'tampered'); },
  'changed resource blob': f => { const p = path.join(f.extracted, 'phoenix-ui-darwin-arm64/runtime/vendor/phoenix-ui/bundle/blobs'); write(path.join(p, fs.readdirSync(p)[0]), 'tampered'); },
  'changed preserved license': f => { const p = path.join(f.extracted, 'phoenix-ui-darwin-arm64/THIRD-PARTY-LICENSES'); write(path.join(p, fs.readdirSync(p)[0]), 'tampered'); },
  'missing Node 18 acceptance': f => { f.jobs.find(j => j.name === 'Node 18 portable consumer smoke').conclusion = 'skipped'; },
};
const preflightErrors = {
  'missing platform': /Missing or duplicate platform artifact/, 'failed required job step': /Failed or missing CI step Full Phoenix/,
  'failed phase record': /phase runtime did not succeed/, 'wrong source commit': /Run source commit differs/,
  'wrong source tree': /build record source tree differs/, 'wrong repository': /Run repository differs/,
  'wrong build input digest': /build input digest differs/, 'unknown cross-platform input difference': /Unknown cross-platform input difference/,
  'changed tested binary': /tested binary digest differs/, 'changed resource blob': /Bundle resource digest differs/,
  'changed preserved license': /Preserved license digest differs/, 'missing Node 18 acceptance': /Node 18 portable consumer smoke did not succeed/,
};
for (const [name, change] of Object.entries(preflightCases)) test(`publication rejects ${name} before any upload`, async t => {
  const f = await fixture(t); change(f); f.repack();
  const result = await f.publish();
  assert.equal(result.code, 1, result.stdout);
  assert.match(result.stderr, preflightErrors[name]);
  assert.equal(f.requests.length, 0);
  assert(!fs.existsSync(path.join(result.output, 'VERSION.json')));
});

test('publication rejects an archive digest mismatch before extraction or upload', async t => {
  const f = await fixture(t);
  fs.appendFileSync(path.join(f.archives, 'phoenix-ui-darwin-arm64.zip'), 'tampered');
  const result = await f.publish();
  assert.equal(result.code, 1); assert.match(result.stderr, /archive.*digest/i); assert.equal(f.requests.length, 0);
});

test('publication refuses an existing key without an explicit resume, including identical bytes', async t => {
  const f = await fixture(t); assert.equal((await f.publish()).code, 0);
  const result = await f.publish();
  assert.equal(result.code, 1); assert.match(result.stderr, /exists|existing/i);
  assert(!fs.existsSync(path.join(result.output, 'VERSION.json')));
  assert.equal(f.requests.filter(r => r.method === 'PUT' && r.status === 412).length, 1);
});

test('publication resume rejects different existing bytes', async t => {
  const f = await fixture(t); assert.equal((await f.publish()).code, 0);
  const first = [...f.objects.keys()][0]; f.objects.set(first, Buffer.alloc(f.objects.get(first).length, 0x78));
  const result = await f.publish(['--resume']);
  assert.equal(result.code, 1); assert.match(result.stderr, /digest|SHA-256/i);
  assert(!fs.existsSync(path.join(result.output, 'VERSION.json')));
});

test('publication resume reads storage bytes and rejects a differing object even if a public cache serves the expected bytes', async t => {
  const f = await fixture(t); assert.equal((await f.publish()).code, 0);
  const first = [...f.objects.keys()][0]; f.publicObjects.set(first, f.objects.get(first));
  f.objects.set(first, Buffer.alloc(f.objects.get(first).length, 0x78));
  const result = await f.publish(['--resume']);
  assert.equal(result.code, 1); assert.match(result.stderr, /digest|SHA-256/i);
  assert(f.requests.some(r => r.method === 'GET' && r.route === 'storage'));
  assert(!fs.existsSync(path.join(result.output, 'VERSION.json')));
});

test('publication rejects missing generation evidence before any upload', async t => {
  const f = await fixture(t);
  fs.rmSync(path.join(f.extracted, 'phoenix-ui-darwin-arm64/runtime/skills/phoenix-ui/GENERATED.json'));
  f.repack();
  const result = await f.publish();
  assert.equal(result.code, 1); assert.match(result.stderr, /GENERATED.json|generation evidence/);
  assert.equal(f.requests.length, 0);
});

test('publication preserves partial uploads and explicitly resumes only matching existing bytes', async t => {
  const f = await fixture(t, { failKey: 'darwin-x64' });
  const first = await f.publish();
  assert.equal(first.code, 1); assert.equal(f.objects.size, 1);
  assert(!fs.existsSync(path.join(first.output, 'VERSION.json')));
  const record = readJson(path.join(first.output, 'publication-record.json'));
  assert.equal(record.status, 'failed'); assert.equal(record.objects.filter(o => o.upload === 'uploaded').length, 1);
  const retry = await f.publish(['--resume'], first.output);
  assert.equal(retry.code, 0, retry.stderr); assert.equal(f.objects.size, 4);
  const resumed = readJson(path.join(first.output, 'publication-record.json'));
  assert.equal(resumed.objects.filter(o => o.upload === 'reused').length, 1);
  assert.equal(f.requests.filter(r => r.method === 'PUT' && r.status === 200).length, 4);
});

test('publication never freezes VERSION when public readback size or digest differs', async t => {
  const f = await fixture(t, { corruptReadback: true }), result = await f.publish();
  assert.equal(result.code, 1); assert.match(result.stderr, /digest|SHA-256/i);
  assert.equal(f.objects.size, 4); assert(!fs.existsSync(path.join(result.output, 'VERSION.json')));
});

test('publication rejects a public readback with a different size', async t => {
  const f = await fixture(t, { shortReadback: true }), result = await f.publish();
  assert.equal(result.code, 1); assert.match(result.stderr, /size\/SHA-256 digest differs/);
  assert(!fs.existsSync(path.join(result.output, 'VERSION.json')));
});

test('concurrent publishers use conditional writes so one wins and the other cannot overwrite the same keys', async t => {
  const f = await fixture(t, { concurrent: true });
  const results = await Promise.all([f.publish(), f.publish()]);
  assert.deepEqual(results.map(r => r.code).sort(), [0, 1]);
  assert.equal(f.objects.size, 4);
  const firstPuts = f.requests.filter(r => r.method === 'PUT' && r.key.includes('darwin-arm64'));
  assert.deepEqual(firstPuts.map(r => r.status).sort(), [200, 412]);
  assert(firstPuts.every(r => r.condition === '*'));
  assert(!fs.existsSync(path.join(results.find(r => r.code !== 0).output, 'VERSION.json')));
});
