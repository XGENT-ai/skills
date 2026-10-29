'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');
const { pathToFileURL } = require('node:url');
const { gzipSync } = require('node:zlib');
const { spawnSync } = require('node:child_process');
const net = require('node:net');
const root = path.resolve(__dirname, '../..');
const moduleReady = import(pathToFileURL(path.join(root, 'scripts/pack-phoenix-diagram-candidate.mjs')));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const npmPackageVersion = JSON.parse(fs.readFileSync(path.join(root, 'package.json'))).version;
const targets = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64'];
const triples = ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'];

// These inert records exercise rejection and schema assembly only. They are
// never passed to packCandidate and do not represent native-platform evidence.
function records() {
  const inputBytes = Buffer.from(JSON.stringify([['package.json', 10, 'a'.repeat(64)]]));
  const bundleBytes = Buffer.from('{"schemaVersion":1,"tool":"phoenix-diagram","version":"0.2.0","files":[]}\n');
  const license = fs.readFileSync(path.join(root, 'LICENSE'));
  const licenses = new Map([['LICENSE', license]]);
  const licenseSha256 = sha(Buffer.from(JSON.stringify([['LICENSE', license.length, sha(license)]])));
  return targets.map((target, index) => {
    const bytes = Buffer.from(`Inert metadata-only artifact for ${target}; not executable.`);
    const binary = { asset: `phoenix-diagram-${target}`, size: bytes.length, sha256: sha(bytes) };
    const version = { schemaVersion: 1, distribution: 'development', tool: 'phoenix-diagram', toolVersion: '0.2.0', npmPackageVersion, sourceCommit: 'b'.repeat(40), sourceInputSha256: sha(inputBytes), inputContractSchema: 1, receiptSchema: 1, viewerRevision: 'phoenix-v2', bundleSha256: sha(bundleBytes), engines: { [target]: binary } };
    const receipt = { schemaVersion: 1, tool: 'phoenix-diagram', toolVersion: '0.2.0', npmPackageVersion, distribution: 'development', target, host: triples[index], sourceCommit: version.sourceCommit, sourceTree: 'c'.repeat(40), sourceState: 'clean', sourceReference: version.sourceCommit, sourceInputSha256: version.sourceInputSha256, binary, bundle: { path: 'bundle/manifest.json', size: bundleBytes.length, sha256: version.bundleSha256 }, viewer: { revision: 'phoenix-v2', template: { path: 'viewer/viewer.html', size: 1, sha256: 'd'.repeat(64) }, sourceManifest: { path: 'viewer/source-manifest.json', size: 1, sha256: 'e'.repeat(64) } }, tools: { pins: { node: '25.2.1', rust: '1.99.0', mrBoxington: '1.21.1' }, observed: { node: '25.2.1', rust: '1.99.0', cargo: '1.99.0', mrBoxington: '1.21.1' }, sources: [] }, legal: { sha256: licenseSha256 }, native: { status: 'pass', host: triples[index], target: triples[index], engineProbe: { status: 'pass', stdout: 'phoenix-diagram-engine 0.2.0 input=1 receipt=1 viewer=phoenix-v2\n' }, engineDoctor: { status: 'pass', toolVersion: '0.2.0', engine: { sourceInputSha256: version.sourceInputSha256, inputContractSchema: 1, receiptSchema: 1, viewerRevision: 'phoenix-v2', binary: { sha256: binary.sha256, bytes: binary.size } } }, sourceInputsMatch: 'pass' }, checks: { lockedOffline: 'pass', sourceStable: 'pass', engineProbe: 'pass', sourceInputs: 'pass', fullRuntimeTests: 'not-run-by-build-script' } };
    return { directory: `/metadata-only/${target}`, target, version, versionBytes: Buffer.from(JSON.stringify(version) + '\n'), receipt, sourceInputs: { entries: JSON.parse(inputBytes), bytes: inputBytes, sha256: sha(inputBytes) }, bundle: { manifest: JSON.parse(bundleBytes), bytes: bundleBytes, sha256: sha(bundleBytes), files: new Map([['SKILL.md', Buffer.from('metadata only')]]) }, licenses, licenseSha256, binary: { ...binary, bytes, path: `/metadata-only/${binary.asset}` } };
  });
}
function temporary(t) {
  const directory = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram candidate ')));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  return directory;
}
function archive(rows) {
  const blocks=[];
  for(const [name,body=Buffer.alloc(0),type='0',mode=0o644] of rows) {
    const bytes=Buffer.from(body),header=Buffer.alloc(512);
    header.write(name,0,100);header.write(mode.toString(8).padStart(7,'0')+'\0',100,8);
    header.write(bytes.length.toString(8).padStart(11,'0')+'\0',124,12);header[156]=type.charCodeAt(0);
    header.fill(32,148,156);const checksum=header.reduce((sum,byte)=>sum+byte,0);
    header.write(checksum.toString(8).padStart(6,'0')+'\0 ',148,8);
    blocks.push(header,bytes,Buffer.alloc((512-bytes.length%512)%512));
  }
  return gzipSync(Buffer.concat([...blocks,Buffer.alloc(1024)]));
}

test('candidate profile is explicit and install-only contexts cannot stand in for full consumption',async t=>{
  const api=await import('../../scripts/check-phoenix-diagram-candidate.mjs'),target=api.targetForHost(),directory=temporary(t);
  const base=['--candidate-dir','/candidate','--target',target];assert.equal(api.parseOptions(base).profile,undefined);assert.equal(api.parseOptions([...base,'--profile','p3']).profile,'p3');assert.throws(()=>api.parseOptions([...base,'--profile','unknown']),/profile/i);
  const file=path.join(directory,'install-context.json');fs.writeFileSync(file,JSON.stringify({schemaVersion:1,kind:'phoenix-diagram-candidate-install-context',scope:'installation-only',target,profile:'p2',phase:'installed',status:'pass',automatedStatus:'not-run'}));
  assert.throws(()=>api.readConsumerContext({resume:file,target,profile:'p2'},{}),/scope|consumption|automatic/i);
  fs.writeFileSync(file,JSON.stringify({schemaVersion:1,kind:'phoenix-diagram-candidate-consumer',target,profile:'p3',phase:'pending-review',status:'automated-pass',automatedStatus:'pass'}));
  assert.throws(()=>api.readCandidateTransport({resume:file,target,profile:'p3'},{}),/scope|consumption|installation/i);
  assert.throws(()=>api.readConsumerContext({resume:file,target,profile:'p2'},{}),/profile/i);
});

test('consumer browser temporary sockets bind below deeply nested evidence roots', async t => {
  const api = await import('../../scripts/check-phoenix-diagram-candidate.mjs');
  const runRoot = path.join(temporary(t), 'long-evidence-'.repeat(10));
  fs.mkdirSync(runRoot, { mode: 0o700 });
  const { env } = api.makeEnvironment(runRoot);
  const socketDirectory = fs.mkdtempSync(path.join(env.TMPDIR, 'com.google.Chrome.'));
  const socketPath = path.join(socketDirectory, 'SingletonSocket');
  const server = net.createServer(socket => socket.end('ready'));
  let client;
  t.after(async () => {
    client?.destroy();
    if (server.listening) await new Promise(resolve => server.close(resolve));
    fs.rmSync(socketDirectory, { recursive: true, force: true });
  });
  assert.equal(fs.statSync(socketDirectory).mode & 0o777, 0o700);
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(socketPath, resolve);
  });
  client = net.createConnection(socketPath);
  client.setEncoding('utf8');
  let received = '';
  await new Promise((resolve, reject) => {
    client.on('data', chunk => { received += chunk; });
    client.once('end', resolve);
    client.once('error', reject);
  });
  assert.equal(received, 'ready');
});

test('candidate metadata rejects a missing or duplicate platform and dirty or mismatched source identities', async () => {
  const api = await moduleReady;
  assert.throws(() => api.assertArtifactAgreement(records().slice(0, 3)), /four|platform/i);
  const duplicate = records(); duplicate[3] = duplicate[2];
  assert.throws(() => api.assertArtifactAgreement(duplicate), /duplicate|platform/i);
  for (const [field, value] of [['sourceState', 'dirty'], ['sourceCommit', 'f'.repeat(40)], ['sourceTree', 'f'.repeat(40)], ['sourceInputSha256', 'f'.repeat(64)]]) {
    const artifacts = records(); artifacts[1].receipt[field] = value;
    assert.throws(() => api.assertArtifactAgreement(artifacts), /source|clean|commit|tree|input/i, field);
  }
});
test('candidate metadata cannot substitute a native success flag for bound host, probe, doctor and build checks', async () => {
  const api = await moduleReady;
  for (const mutate of [
    record => { record.receipt.host = 'x86_64-unknown-linux-gnu'; },
    record => { record.receipt.native.engineProbe.stdout = 'phoenix-ui-engine 0.1.0\n'; },
    record => { record.receipt.native.engineDoctor.engine.binary.sha256 = 'f'.repeat(64); },
    record => { record.receipt.native.engineDoctor.toolVersion = '9.9.9'; },
    record => { record.receipt.checks.sourceStable = 'not-run'; },
    record => { record.receipt.native.sourceInputsMatch = 'not-run'; },
  ]) {
    const artifacts = records(); mutate(artifacts[0]);
    assert.throws(() => api.assertArtifactAgreement(artifacts), /native|host|probe|doctor|check|source/i);
  }
});
test('candidate metadata rejects changed binary, source tuples, bundle, licenses, versions and pinned tools', async () => {
  const api = await moduleReady;
  for (const mutate of [
    record => { record.binary.bytes = Buffer.from('changed binary'); },
    record => { record.sourceInputs.bytes = Buffer.from('[]'); },
    record => { record.bundle.files.set('SKILL.md', Buffer.from('changed skill')); },
    record => { record.licenses.set('LICENSE', Buffer.from('changed license')); },
    record => { record.version.npmPackageVersion = '9.9.9'; },
    record => { record.receipt.tools.pins.mrBoxington = '9.9.9'; },
    record => { record.receipt.viewer.template.sha256 = 'f'.repeat(64); },
  ]) {
    const artifacts = records(); mutate(artifacts[1]);
    assert.throws(() => api.assertArtifactAgreement(artifacts), /binary|input|bundle|license|package|version|tool|viewer/i);
  }
});
test('metadata-only candidate schema fixes all four loopback URLs and never claims consumer acceptance', async () => {
  const api = await moduleReady;
  const artifacts = records();
  const manifest = api.candidateManifest(artifacts, { port: 48765 });
  assert.equal(manifest.distribution, 'candidate');
  assert.deepEqual(Object.keys(manifest.engines), targets);
  for (const target of targets) assert.equal(manifest.engines[target].url, `http://127.0.0.1:48765/phoenix-diagram-${target}`);
  assert.deepEqual(artifacts.map(record => record.version.distribution), targets.map(() => 'development'));
  assert.equal(manifest.sourceCommit, artifacts[0].version.sourceCommit);
  assert.equal('handoff' in manifest, false);
  for (const port of [0, -1, 65536, 1.5, '48765']) assert.throws(() => api.candidateManifest(artifacts, { port }), /port/i);
});
test('staging metadata refuses missing, changed or extra diagram runtime and license bytes', async () => {
  const api=await moduleReady,artifact=records()[0];
  function sources() {
    const selected=new Map();
    for(const [name,bytes] of artifact.bundle.files)for(const prefix of ['dist/phoenix-diagram/bundle/','skills/phoenix-diagram/'])selected.set(prefix+name,{bytes,mode:0o644});
    selected.set('dist/phoenix-diagram/bundle/manifest.json',{bytes:artifact.bundle.bytes,mode:0o644});
    for(const [name,bytes] of artifact.licenses)selected.set('dist/phoenix-diagram/'+name,{bytes,mode:0o644});
    return selected;
  }
  const absent=sources();absent.delete('dist/phoenix-diagram/LICENSE');
  assert.throws(()=>api.assertRuntimeSources(absent,artifact),/missing|freeze|differs/i);
  const changed=sources();changed.set('dist/phoenix-diagram/LICENSE',{bytes:Buffer.from('changed'),mode:0o644});
  assert.throws(()=>api.assertRuntimeSources(changed,artifact),/missing|freeze|differs/i);
  for(const name of ['dist/phoenix-diagram/THIRD-PARTY-LICENSES/stale.txt','dist/phoenix-diagram/bundle/stale.json','skills/phoenix-diagram/stale.md']) {
    const extra=sources();extra.set(name,{bytes:Buffer.from('stale'),mode:0o644});
    assert.throws(()=>api.assertRuntimeSources(extra,artifact),/extra|unlisted|runtime/i,name);
  }
});
test('filesystem preflight refuses missing platforms and symlink artifacts before any output or pack', async t => {
  const api = await moduleReady, directory = temporary(t);
  const staged=path.join(directory,'staged candidate');fs.mkdirSync(staged);
  const source=records()[0].sourceInputs;
  assert.throws(()=>api.candidateFiles(staged,source.sha256),/ENOENT|source-inputs/i,'Omitted candidate source-inputs must fail');
  const inputPath=path.join(staged,'source-inputs.json');
  fs.writeFileSync(inputPath,source.bytes);
  assert.deepEqual(api.candidateFiles(staged,source.sha256),[{path:'source-inputs.json',size:source.bytes.length,sha256:source.sha256}]);
  fs.writeFileSync(inputPath,JSON.stringify([['package.json',11,'a'.repeat(64)]]));
  assert.throws(()=>api.candidateFiles(staged,source.sha256),/source.*(?:digest|differs|identity)/i,'Changed canonical tuple bytes must fail');
  fs.writeFileSync(inputPath,Buffer.concat([source.bytes,Buffer.from('\n')]));
  assert.throws(()=>api.candidateFiles(staged,sha(fs.readFileSync(inputPath))),/canonical/i,'A matching digest cannot bless noncanonical tuples');
  const artifactsDir = path.join(directory, 'artifacts'); fs.mkdirSync(artifactsDir);
  const outputDir = path.join(directory, 'candidate');
  await assert.rejects(api.packCandidate({ repoRoot: root, artifactsDir, outputDir }), /platform|four|missing/i);
  assert.equal(fs.existsSync(outputDir), false);
  if (process.platform !== 'win32') {
    for (const target of targets) fs.mkdirSync(path.join(artifactsDir, target));
    fs.rmdirSync(path.join(artifactsDir, targets[0])); fs.symlinkSync(path.join(artifactsDir, targets[1]), path.join(artifactsDir, targets[0]));
    await assert.rejects(api.packCandidate({ repoRoot: root, artifactsDir, outputDir }), /symbolic|symlink|directory|nonregular/i);
    assert.equal(fs.existsSync(outputDir), false);
  }
});
test('actual tar parsing preserves existing Archify JS and rejects archives, credentials, native assets and unsafe members', async () => {
  const api=await moduleReady;
  const ordinary=['package/package.json','{}'];
  const js=['package/skills/archify/scripts/render.mjs','// Existing runtime remains original bytes.\n'];
  const inventory=api.readTarballInventory(archive([ordinary,js]));
  assert.deepEqual(inventory.get(js[0].slice(8)),{size:Buffer.byteLength(js[1]),sha256:sha(js[1]),mode:0o644});
  for(const name of ['package/../outside','package/tools/phoenix-diagram/upstream/source.tar.gz','package/local/prepared/oracle.json','package/node_modules/foreign/index.js','package/skills/a/.env.local','package/credentials.json','package/dist/phoenix-diagram/phoenix-diagram-linux-x64']) {
    assert.throws(()=>api.readTarballInventory(archive([ordinary,[name,'rejected']])),/unsafe|member|duplicate/i,name);
  }
  assert.throws(()=>api.readTarballInventory(archive([ordinary,ordinary])),/duplicate/i);
  assert.throws(()=>api.readTarballInventory(archive([ordinary,['package/link','', '2']])),/type|member/i);
  assert.throws(()=>api.readTarballInventory(archive([ordinary,['package-untrusted/','', '5']])),/type|member/i);
  assert.throws(()=>api.readTarballInventory(archive([['package/package.json','{}','0',0o4644]])),/mode|header/i);
});
test('candidate CLI help is read-only and malformed or duplicate options cannot start a pack', async t => {
  await moduleReady;
  const directory=temporary(t),script=path.join(root,'scripts/pack-phoenix-diagram-candidate.mjs');
  const help=spawnSync(process.execPath,[script,'--help'],{cwd:directory,encoding:'utf8'});
  assert.equal(help.status,0,help.stderr);assert.match(help.stdout,/48765/);assert.match(help.stdout,/frozen/);
  for(const args of [[],['--unknown'],['--artifacts-dir','missing','--output-dir','candidate','--port','0'],['--artifacts-dir','missing','--output-dir','candidate','--port=1=2'],['--artifacts-dir','missing','--artifacts-dir','duplicate','--output-dir','candidate']]) {
    const result=spawnSync(process.execPath,[script,...args],{cwd:directory,encoding:'utf8'});
    assert.notEqual(result.status,0);assert.match(result.stderr,/usage|port/i,JSON.stringify(args));
  }
  assert.deepEqual(fs.readdirSync(directory),[]);
});

const consumerReady = import(pathToFileURL(path.join(root, 'scripts/check-phoenix-diagram-candidate.mjs')));
const legacySampleNames = {
  architecture: 'web-app.architecture.json',
  workflow: 'agent-tool-call.workflow.json',
  sequence: 'async-job-roundtrip.sequence.json',
  dataflow: 'product-analytics.dataflow.json',
  lifecycle: 'deployment-release.lifecycle.json',
};
function legacyLayoutFixture(layout, { tuples = layout === 'phoenix-flat' } = {}) {
  // Fixed real sample bytes; inert tar-member/source-tuple records only.
  // This fixture does not certify a native build or a candidate cohort.
  const members = new Map(), entries = [['package.json', 2, sha(Buffer.from('{}'))]];
  for (const name of Object.values(legacySampleNames)) {
    const bytes = fs.readFileSync(path.join(root, 'tools/phoenix-diagram/upstream/legacy-2.17/examples', name));
    const member = layout === 'phoenix-flat' ? 'skills/phoenix-diagram/examples/legacy.' + name : 'skills/archify/examples/' + name;
    members.set(member, { bytes, size: bytes.length, sha256: sha(bytes), mode: 0o644 });
    if (tuples) entries.push([member, bytes.length, sha(bytes)]);
  }
  entries.sort((left, right) => Buffer.compare(Buffer.from(left[0]), Buffer.from(right[0])));
  return { members, sourceInputs: { entries } };
}

test('legacy sample layout selects authenticated old or flat bytes independently of profile', async () => {
  const api = await consumerReady;
  assert.equal(typeof api.selectLegacySamples, 'function');
  for (const layout of ['archify', 'phoenix-flat']) {
    const fixture = legacyLayoutFixture(layout);
    const selected = api.selectLegacySamples(fixture.members, fixture.sourceInputs);
    assert.equal(selected.layout, layout);
    for (const [type, name] of Object.entries(legacySampleNames)) {
      assert.equal(selected.paths[type], layout === 'archify' ? 'skills/archify/examples/' + name : 'skills/phoenix-diagram/examples/legacy.' + name);
    }
    for (const profile of ['p1', 'p2']) assert.deepEqual(api.selectLegacySamples(fixture.members, fixture.sourceInputs, { profile }), selected);
    if (layout === 'archify') {
      assert.throws(() => api.selectLegacySamples(fixture.members, fixture.sourceInputs, { profile: 'p3' }), /P3|new|flat|legacy.*layout/i);
      const bound = legacyLayoutFixture(layout, { tuples: true });
      assert.deepEqual(api.selectLegacySamples(bound.members, bound.sourceInputs), selected);
    } else assert.deepEqual(api.selectLegacySamples(fixture.members, fixture.sourceInputs, { profile: 'p3' }), selected);
  }
});

test('legacy sample layout rejects mixed missing unknown changed or source-unbound sets', async () => {
  const api = await consumerReady;
  for (const layout of ['archify', 'phoenix-flat']) {
    const other = legacyLayoutFixture(layout === 'archify' ? 'phoenix-flat' : 'archify');
    for (const fault of ['missing', 'mixed-one', 'mixed-all', 'unknown', 'changed-bytes', 'changed-digest', 'changed-size', 'opposite-tuples', 'partial-tuples', 'changed-tuple']) {
      const fixture = legacyLayoutFixture(layout, { tuples: true });
      const [name, file] = fixture.members.entries().next().value;
      if (fault === 'missing') fixture.members.delete(name);
      else if (fault === 'mixed-one') { const [key, value] = other.members.entries().next().value; fixture.members.set(key, value); }
      else if (fault === 'mixed-all') for (const [key, value] of other.members) fixture.members.set(key, value);
      else if (fault === 'unknown') fixture.members = new Map([...fixture.members].map(([key, value]) => ['unrecognized/' + key, value]));
      else if (fault === 'changed-bytes') { file.bytes = Buffer.concat([file.bytes, Buffer.from('\n')]); file.size = file.bytes.length; file.sha256 = sha(file.bytes); fixture.sourceInputs.entries.find(row => row[0] === name).splice(1, 2, file.size, file.sha256); }
      else if (fault === 'changed-digest') file.sha256 = 'f'.repeat(64);
      else if (fault === 'changed-size') file.size++;
      else if (fault === 'opposite-tuples') fixture.sourceInputs = legacyLayoutFixture(layout === 'archify' ? 'phoenix-flat' : 'archify', { tuples: true }).sourceInputs;
      else if (fault === 'partial-tuples') fixture.sourceInputs.entries = fixture.sourceInputs.entries.filter(row => row[0] !== name);
      else fixture.sourceInputs.entries.find(row => row[0] === name)[2] = 'f'.repeat(64);
      assert.throws(() => api.selectLegacySamples(fixture.members, fixture.sourceInputs), /legacy|sample|layout|source|digest|bytes|tuple/i, layout + ': ' + fault);
    }
  }
  const unbound = legacyLayoutFixture('phoenix-flat', { tuples: false });
  assert.throws(() => api.selectLegacySamples(unbound.members, unbound.sourceInputs), /source|tuple|identity/i);
  for (const active of ['skills/archify/SKILL.md', 'skills/archify/bin/archify.mjs', 'skills/archify/scripts/render.mjs', 'skills/Archify/SKILL.md']) {
    const fixture = legacyLayoutFixture('phoenix-flat');
    fixture.members.set(active, { bytes: Buffer.from('old active entry'), size: 16, sha256: sha(Buffer.from('old active entry')), mode: 0o644 });
    assert.throws(() => api.selectLegacySamples(fixture.members, fixture.sourceInputs), /archify|active|layout/i, active);
  }
});

test('legacy sample case manifest shares byte and source checks without filesystem fallback', async t => {
  const api = await consumerReady, directory = temporary(t), packageRoot = path.join(directory, 'not-installed');
  for (const layout of ['archify', 'phoenix-flat']) {
    const fixture = legacyLayoutFixture(layout);
    const manifest = api.caseManifest(packageRoot, fixture);
    if (layout === 'archify') assert.throws(() => api.caseManifest(packageRoot, fixture, { profile: 'p3' }), /P3|flat|layout/i);
    else assert.deepEqual(api.caseManifest(packageRoot, fixture, { profile: 'p3' }), manifest);
    assert.equal(manifest.cases.length, 15);
    for (const [type, name] of Object.entries(legacySampleNames)) {
      const old = manifest.cases.find(row => row.id === 'legacy-' + type);
      assert.equal(old.source, path.join(packageRoot, layout === 'archify' ? 'skills/archify/examples/' + name : 'skills/phoenix-diagram/examples/legacy.' + name));
      assert.equal(old.contract, 'legacy-2.17');
      assert.equal(manifest.cases.find(row => row.id === 'new-' + type).source, path.join(packageRoot, 'skills/phoenix-diagram/examples/minimal.' + type + '.json'));
    }
    assert.equal(fs.existsSync(packageRoot), false);
    const [missing, file] = fixture.members.entries().next().value, fallbackRoot = path.join(directory, 'fallback-' + layout);
    fs.mkdirSync(path.dirname(path.join(fallbackRoot, missing)), { recursive: true });
    fs.writeFileSync(path.join(fallbackRoot, missing), file.bytes);
    fixture.members.delete(missing);
    // Even an exact physical copy cannot fill an absent authenticated member.
    assert.throws(() => api.caseManifest(fallbackRoot, fixture), /legacy|sample|layout/i);
  }
});
