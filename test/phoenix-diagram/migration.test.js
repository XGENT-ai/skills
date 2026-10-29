'use strict';
// Disposable native entry-chain fixtures. Real provider loading and four-platform
// release acceptance remain separate; no fake probe can pass a native lane.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');
const { spawn, spawnSync } = require('node:child_process');
const root = path.resolve(__dirname, '../..');
const bootstrapPath = path.join(root, 'skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs');
const boot = require(bootstrapPath);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const encoded = value => Buffer.from(JSON.stringify(value) + '\n');
const write = (file, bytes, mode = 0o644) => { fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, bytes); fs.chmodSync(file, mode); };
const supported = boot.TARGETS.includes(`${process.platform}-${process.arch}`);
const nativeTest = (name, fn) => test(name, { skip: !process.env.PHOENIX_DIAGRAM_BIN ? 'Requires explicit stable PHOENIX_DIAGRAM_BIN; never builds or downloads' : !supported ? 'Native Unix target required' : false }, fn);
function tree(directory) {
  const result = {};
  const visit = prefix => { for (const name of fs.readdirSync(path.join(directory, prefix))) {
    const relative = prefix ? `${prefix}/${name}` : name, file = path.join(directory, relative), stat = fs.lstatSync(file);
    result[relative] = stat.isDirectory() ? { directory: true, mode: stat.mode & 0o7777 } : { sha256: sha(fs.readFileSync(file)), mode: stat.mode & 0o7777 };
    if (stat.isDirectory()) visit(relative);
  } };
  visit(''); return result;
}
function fixture(t) {
  const temp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix diagram migration ')));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const project = path.join(temp, 'project'), pkg = path.join(temp, 'package'), dist = path.join(pkg, 'dist/phoenix-diagram'), bundleRoot = path.join(dist, 'bundle');
  fs.mkdirSync(project); const env = { ...process.env, HOME: path.join(temp, 'personal'), PHOENIX_DIAGRAM_HOME: path.join(temp, 'cache') };
  fs.mkdirSync(env.HOME); const bytes = fs.readFileSync(process.env.PHOENIX_DIAGRAM_BIN);
  const binary = path.join(temp, 'engine'); write(binary, bytes, 0o700);
  const doctor = spawnSync(binary, ['doctor', '--json'], { encoding: 'utf8', env });
  const engine = JSON.parse(doctor.stdout).engine; assert.equal(sha(bytes), engine.binary.sha256);
  const packageJsonPath = path.join(pkg, 'package.json'); write(packageJsonPath, encoded({ name: '@xgent-ai/skills', version: JSON.parse(fs.readFileSync(path.join(root, 'package.json'))).version }));
  const files = [ ['SKILL.md', Buffer.from('---\nname: phoenix-diagram\ndescription: migration fixture\n---\n'), 0o644], ...['phoenix-bootstrap.cjs', 'phoenix-launcher.cjs', 'phoenix-diagram'].map(name => [`scripts/${name}`, fs.readFileSync(path.join(root, 'skills/phoenix-diagram/scripts', name)), name.endsWith('bootstrap.cjs') ? 0o644 : 0o755]) ];
  for (const [name, contents, mode] of files) write(path.join(bundleRoot, name), contents, mode);
  const manifest = encoded({ schemaVersion: 1, tool: 'phoenix-diagram', version: '0.2.0', files: files.map(([name, contents, mode]) => ({ path: name, size: contents.length, sha256: sha(contents), mode })) });
  write(path.join(bundleRoot, 'manifest.json'), manifest);
  const target = boot.platformTarget(), asset = `phoenix-diagram-${target}`;
  const version = { schemaVersion: 1, distribution: 'development', tool: 'phoenix-diagram', toolVersion: '0.2.0', npmPackageVersion: JSON.parse(fs.readFileSync(packageJsonPath)).version, sourceCommit: 'a'.repeat(40), sourceInputSha256: engine.sourceInputSha256, inputContractSchema: 1, receiptSchema: 1, viewerRevision: 'phoenix-v2', bundleSha256: sha(manifest), engines: { [target]: { asset, size: bytes.length, sha256: sha(bytes) } } };
  const manifestPath = path.join(dist, 'VERSION.json'); write(manifestPath, encoded(version)); write(path.join(bundleRoot, 'ENGINE.json'), encoded(version));
  const apache = fs.readFileSync(path.join(root, 'tools/phoenix-ui/LICENSE'));
  write(path.join(dist, 'LICENSE'), 'License format fixture\n'); write(path.join(dist, 'NOTICE.md'), 'Notice format fixture\n');
  write(path.join(dist, 'THIRD-PARTY-LICENSES/apache.txt'), apache); write(path.join(dist, 'THIRD-PARTY.json'), encoded({ licenses: [{ path: 'THIRD-PARTY-LICENSES/apache.txt', sha256: sha(apache) }] })); write(path.join(dist, 'THIRD-PARTY-NOTICES.md'), 'Test format fixture; not release license acceptance\n');
  const options = { project, manifestPath, bundleRoot, packageJsonPath };
  const manifestData = boot.manifestAt(manifestPath, options); write(boot.enginePath(manifestData, target, env), bytes, 0o700);
  return { temp, project, pkg, dist, bundleRoot, env, options, manifest: manifestData, target };
}
function run(f, args, options = f.options, source = bootstrapPath) {
  const code = `require(${JSON.stringify(source)}).main(${JSON.stringify(args)},${JSON.stringify(options)}).then(code=>{process.exitCode=code;});`;
  const result = spawnSync(process.execPath, ['-e', code], { env: f.env, cwd: f.project, encoding: 'utf8', timeout: 600000, maxBuffer: 64 * 1024 * 1024 });
  assert.ifError(result.error); return result;
}
function legacy(f, provider = '.agents') {
  const directory = path.join(f.temp, `extract-${provider}`); fs.mkdirSync(directory);
  const extracted = spawnSync('tar', ['-xzf', path.join(root, 'tools/phoenix-diagram/upstream/archify-2.17-full-source.tar.gz'), '-C', directory, 'legacy-2.17'], { encoding: 'utf8' }); assert.equal(extracted.status, 0, extracted.stderr);
  const target = path.join(f.project, provider, 'skills/archify'); fs.mkdirSync(path.dirname(target), { recursive: true }); fs.renameSync(path.join(directory, 'legacy-2.17'), target); return target;
}

test('bootstrap migration options strictly bind mode, explicit project and confirmation scope', () => {
  const id = 'a'.repeat(32);
  for (const args of [ ['migrate'], ['migrate', '--from-archify'], ['migrate', '--from-archify', '--recover', id, '--project', '.'], ['migrate', '--recover', id, '--project', '.', '--confirm-remove-archify'], ['migrate', '--recover', id, '--project', '.', '--dry-run'], ['migrate', '--rollback', '../foreign', '--project', '.'], ['migrate', '--from-archify', '--project', '.', '--project', '.'], ['migrate', '--from-archify', '--project', '.', '--unknown'] ]) assert.throws(() => boot.migrationOptions(args), { code: 2 });
  assert.deepEqual(boot.migrationOptions(['migrate', '--from-archify', '--project', './project']), { project: path.resolve('./project'), from: true, bundleRoot: undefined });
  assert.equal(boot.migrationOptions(['migrate', '--rollback', id, '--project=./project', '--dry-run']).from, false);
});

test('joint confirmation alone fails and migration skip never reaches shared installation', t => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram migration skip ')); t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  write(path.join(temp, '.claude/settings.json'), 'deliberately malformed shared settings'); write(path.join(temp, 'AGENTS.md'), 'authored'); const before = tree(temp);
  const call = flags => spawnSync(process.execPath, [path.join(root, 'bin/xgent-skills.js'), 'install', temp, ...flags], { encoding: 'utf8' });
  assert.equal(call(['--confirm-remove-archify']).status, 2); assert.deepEqual(tree(temp), before);
  const skipped = call(['--migrate-archify', '--confirm-remove-archify', '--no-phoenix-diagram']); assert.equal(skipped.status, 0, skipped.stderr); assert.deepEqual(tree(temp), before);
});

nativeTest('native bootstrap migrates exact project without ascending, and recover reaches native with invalid active', t => {
  const f = fixture(t), legacyPath = legacy(f); fs.mkdirSync(path.join(f.temp, '.git')); write(path.join(f.project, '.claude/settings.json'), 'shared malformed bytes'); const before = tree(f.project);
  const args = ['migrate', '--from-archify', '--project', f.project, '--providers', 'codex', '--json'];
  const dry = run(f, [...args, '--dry-run']); assert.equal(dry.status, 0, dry.stdout + dry.stderr); assert.equal(JSON.parse(dry.stdout).project, f.project); assert.deepEqual(tree(f.project), before);
  const unconfirmed = run(f, args); assert.equal(unconfirmed.status, 3, unconfirmed.stdout + unconfirmed.stderr); assert.deepEqual(tree(f.project), before);
  const applied = run(f, [...args, '--confirm-remove-archify']); assert.equal(applied.status, 0, applied.stdout + applied.stderr); const receipt = JSON.parse(applied.stdout); assert.equal(receipt.status, 'installed'); assert.equal(fs.existsSync(legacyPath), false);
  const launcher = path.join(f.project, '.agents/skills/phoenix-diagram/scripts/phoenix-launcher.cjs');
  const ordinary = spawnSync(process.execPath, [launcher, 'doctor', '--json'], { env: f.env, encoding: 'utf8' }); assert.equal(ordinary.status, 0, ordinary.stdout + ordinary.stderr);
  write(path.join(f.project, '.phoenix-diagram/install/active.json'), 'damaged active', 0o600);
  const recovery = run(f, ['migrate', '--recover', receipt.transactionId, '--project', f.project, '--json']); assert.equal(recovery.status, 3, recovery.stdout + recovery.stderr); assert.equal(JSON.parse(recovery.stdout).command, 'migrate'); assert.doesNotMatch(recovery.stderr, /ownership.*different|bootstrap.*unavailable/);
});

nativeTest('Diagram-only joint migration preserves malformed shared configuration and does not execute UI', t => {
  const f = fixture(t); legacy(f); write(path.join(f.project, '.claude/settings.json'), 'malformed shared settings'); write(path.join(f.project, 'AGENTS.md'), 'authored'); write(path.join(f.project, '.phoenix-ui/keep'), 'ui bytes');
  // Load the normal joint entry from a self-contained package fixture. UI code
  // exists for require-time compatibility, but its dist is deliberately absent.
  for (const file of ['bin/xgent-skills.js', 'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs', 'skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs']) write(path.join(f.pkg, file), fs.readFileSync(path.join(root, file)));
  const call = flags => spawnSync(process.execPath, [path.join(f.pkg, 'bin/xgent-skills.js'), 'install', f.project, '--migrate-archify', '--providers=codex', ...flags], { env: f.env, encoding: 'utf8', timeout: 600000, maxBuffer: 64 * 1024 * 1024 });
  const before = tree(f.project); const refused = call([]); assert.equal(refused.status, 3, refused.stdout + refused.stderr); assert.deepEqual(tree(f.project), before);
  const applied = call(['--confirm-remove-archify']); assert.equal(applied.status, 0, applied.stdout + applied.stderr); assert.equal(fs.readFileSync(path.join(f.project, '.claude/settings.json'), 'utf8'), 'malformed shared settings'); assert.equal(fs.readFileSync(path.join(f.project, 'AGENTS.md'), 'utf8'), 'authored'); assert.equal(fs.readFileSync(path.join(f.project, '.phoenix-ui/keep'), 'utf8'), 'ui bytes'); assert.equal(fs.existsSync(path.join(f.project, '.claude/hooks')), false); assert.equal(fs.existsSync(path.join(f.project, '.phoenix-ui/install')), false);
});

nativeTest('real frozen v1 install upgrades and rolls back to its original launcher with rebound ownership', t => {
  const oldRelease = process.env.PHOENIX_DIAGRAM_V1_RELEASE_DIR;
  if (!oldRelease) { t.skip('Requires explicit retained real v1 release; new-writer fixtures do not prove old launcher consumption'); return; }
  const f = fixture(t), oldDist = path.resolve(oldRelease), oldBundle = path.join(oldDist, 'bundle');
  const oldBootstrapPath = path.join(oldBundle, 'scripts/phoenix-bootstrap.cjs'), oldBoot = require(oldBootstrapPath);
  const oldOptions = { ...f.options, manifestPath: path.join(oldDist, 'VERSION.json'), bundleRoot: oldBundle };
  const oldManifest = oldBoot.manifestAt(oldOptions.manifestPath, oldOptions); assert.equal(oldManifest.data.sourceInputSha256, 'f89bad593a07d45f700b32a0294d8c5173cb4f6d9a658def6bfd0d910fc1d0a0'); assert.equal(oldManifest.data.bundleSha256, '384a661592aa2b7449a4f3ab4547b8405aa1a2242248ca43f44fbbebdd5d1d36'); if (f.target === 'darwin-arm64') assert.equal(oldManifest.data.engines[f.target].sha256, '4819cd2474788dbb2308c5a34231465f24ea892aea87a5da9f2a1713502d1001');
  write(oldBoot.enginePath(oldManifest, f.target, f.env), fs.readFileSync(path.join(oldDist, oldManifest.data.engines[f.target].asset)), 0o700);
  const first = run(f, ['install', '--project', f.project, '--providers', 'codex', '--json'], oldOptions, oldBootstrapPath); assert.equal(first.status, 0, first.stdout + first.stderr);
  const activePath = path.join(f.project, '.phoenix-diagram/install/active.json'); const oldActive = JSON.parse(fs.readFileSync(activePath)); assert.equal(oldActive.schemaVersion, 1);
  const launcher = path.join(f.project, '.agents/skills/phoenix-diagram/scripts/phoenix-launcher.cjs'); const oldScript = fs.readFileSync(launcher); const oldStat = fs.statSync(path.join(path.dirname(launcher), 'phoenix-bootstrap.cjs'), { bigint: true });
  legacy(f); const applied = run(f, ['migrate', '--from-archify', '--project', f.project, '--providers', 'codex', '--confirm-remove-archify', '--json']); assert.equal(applied.status, 0, applied.stdout + applied.stderr); const receipt = JSON.parse(applied.stdout); assert.equal(JSON.parse(fs.readFileSync(activePath)).schemaVersion, 2);
  const newLauncher = spawnSync(process.execPath, [launcher, 'doctor', '--json'], { env: f.env, encoding: 'utf8' }); assert.equal(newLauncher.status, 0, newLauncher.stdout + newLauncher.stderr);
  const rolled = run(f, ['migrate', '--rollback', receipt.transactionId, '--project', f.project, '--json']); assert.equal(rolled.status, 0, rolled.stdout + rolled.stderr); assert.equal(JSON.parse(rolled.stdout).status, 'rolled-back'); assert(fs.readFileSync(launcher).equals(oldScript)); assert.notEqual(fs.statSync(path.join(path.dirname(launcher), 'phoenix-bootstrap.cjs'), { bigint: true }).ino, oldStat.ino); assert.equal(JSON.parse(fs.readFileSync(activePath)).schemaVersion, 1);
  const restored = spawnSync(process.execPath, [launcher, 'doctor', '--json'], { env: f.env, encoding: 'utf8' }); assert.equal(restored.status, 0, restored.stdout + restored.stderr);
  assert.equal(JSON.parse(restored.stdout).engine.binary.sha256, oldManifest.data.engines[f.target].sha256);
  const second = run(f, ['migrate', '--rollback', receipt.transactionId, '--project', f.project, '--json']); assert.equal(second.status, 0, second.stdout + second.stderr); assert.equal(JSON.parse(second.stdout).status, 'rolled-back'); assert.equal(fs.existsSync(path.join(f.project, '.agents/skills/archify')), false);
});

nativeTest('explicit migration bundle is bound to the bootstrap fixed VERSION before native execution', t => {
  const f = fixture(t), other = path.join(f.temp, 'other-bundle'); fs.cpSync(f.bundleRoot, other, { recursive: true });
  const contents = Buffer.from('custom bundle must not bypass the fixed package'); write(path.join(other, 'SKILL.md'), contents);
  const manifestPath = path.join(other, 'manifest.json'), manifest = JSON.parse(fs.readFileSync(manifestPath));
  Object.assign(manifest.files.find(file => file.path === 'SKILL.md'), { size: contents.length, sha256: sha(contents) }); write(manifestPath, encoded(manifest));
  const version = { ...f.manifest.data, bundleSha256: sha(fs.readFileSync(manifestPath)) }; write(path.join(other, 'ENGINE.json'), encoded(version));
  const before = tree(f.project); const refused = run(f, ['migrate', '--from-archify', '--project', f.project, '--bundle-root', other, '--dry-run', '--json']);
  assert.equal(refused.status, 4, refused.stdout + refused.stderr); assert.match(refused.stderr, /Bundle manifest identity|digest differs/); assert.deepEqual(tree(f.project), before);
});

function realV1Install(f) {
  const dist = path.resolve(process.env.PHOENIX_DIAGRAM_V1_RELEASE_DIR), bundle = path.join(dist, 'bundle');
  const source = path.join(bundle, 'scripts/phoenix-bootstrap.cjs'), old = require(source);
  const options = { ...f.options, manifestPath: path.join(dist, 'VERSION.json'), bundleRoot: bundle };
  const manifest = old.manifestAt(options.manifestPath, options);
  assert.equal(manifest.data.sourceInputSha256, 'f89bad593a07d45f700b32a0294d8c5173cb4f6d9a658def6bfd0d910fc1d0a0');
  write(old.enginePath(manifest, f.target, f.env), fs.readFileSync(path.join(dist, manifest.data.engines[f.target].asset)), 0o700);
  const result = run(f, ['install', '--project', f.project, '--providers', 'codex', '--json'], options, source);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  const receipt = JSON.parse(result.stdout);
  assert.equal(JSON.parse(fs.readFileSync(path.join(f.project, '.phoenix-diagram/install/active.json'))).schemaVersion, 1);
  return { receipt, options, source, manifest };
}

nativeTest('new engine directly rolls back real committed v1, keeps its audit and permits real old reinstall', t => {
  if (!process.env.PHOENIX_DIAGRAM_V1_RELEASE_DIR) { t.skip('Requires retained real v1 release'); return; }
  const f = fixture(t); write(path.join(f.project, 'business.json'), '{"preserved":true}\n');
  const old = realV1Install(f), id = old.receipt.transactionId;
  const args = ['migrate', '--rollback', id, '--project', f.project, '--json'];
  const before = tree(f.project), dry = run(f, [...args, '--dry-run']); assert.equal(dry.status, 0, dry.stdout + dry.stderr); assert.equal(JSON.parse(dry.stdout).status, 'planned'); assert.deepEqual(tree(f.project), before);
  const rolled = run(f, args); assert.equal(rolled.status, 0, rolled.stdout + rolled.stderr); const receipt = JSON.parse(rolled.stdout); assert.equal(receipt.status, 'rolled-back'); assert.equal(receipt.transactionSchemaVersion, 2);
  assert.equal(fs.existsSync(path.join(f.project, '.phoenix-diagram/install/active.json')), false); assert.equal(fs.existsSync(path.join(f.project, '.agents/skills/phoenix-diagram/SKILL.md')), false); assert.equal(fs.readFileSync(path.join(f.project, 'business.json'), 'utf8'), '{"preserved":true}\n');
  const audit = JSON.parse(fs.readFileSync(receipt.journal)); assert.equal(audit.schemaVersion, 2); assert.equal(audit.phase, 'rolled-back'); assert(audit.operations.every(operation => operation.kind === 'file-write'));
  const after = tree(f.project), repeated = run(f, args); assert.equal(repeated.status, 0, repeated.stdout + repeated.stderr); assert.equal(JSON.parse(repeated.stdout).status, 'rolled-back'); assert.deepEqual(tree(f.project), after);
  const reinstalled = run(f, ['install', '--project', f.project, '--providers', 'codex', '--json'], old.options, old.source); assert.equal(reinstalled.status, 0, reinstalled.stdout + reinstalled.stderr);
  const launcher = path.join(f.project, '.agents/skills/phoenix-diagram/scripts/phoenix-launcher.cjs'); const used = spawnSync(process.execPath, [launcher, 'doctor', '--json'], { env: f.env, encoding: 'utf8' }); assert.equal(used.status, 0, used.stdout + used.stderr); assert.equal(JSON.parse(used.stdout).engine.binary.sha256, old.manifest.data.engines[f.target].sha256);
  const later = tree(f.project), refused = run(f, args); assert.equal(refused.status, 3, refused.stdout + refused.stderr); assert.deepEqual(tree(f.project), later);
});

nativeTest('direct real v1 rollback preserves a later same-content new-inode managed file', t => {
  if (!process.env.PHOENIX_DIAGRAM_V1_RELEASE_DIR) { t.skip('Requires retained real v1 release'); return; }
  const f = fixture(t), old = realV1Install(f), skill = path.join(f.project, '.agents/skills/phoenix-diagram/SKILL.md');
  const contents = fs.readFileSync(skill), original = path.join(f.temp, 'original-skill'); fs.renameSync(skill, original); write(skill, contents); const replacement = fs.statSync(skill, { bigint: true }).ino; assert.notEqual(replacement, fs.statSync(original, { bigint: true }).ino);
  const before = tree(f.project), result = run(f, ['migrate', '--rollback', old.receipt.transactionId, '--project', f.project, '--json']); assert.equal(result.status, 3, result.stdout + result.stderr); assert.match(result.stdout, /install\/managed-file-modified/); assert.deepEqual(tree(f.project), before); assert.equal(fs.statSync(skill, { bigint: true }).ino, replacement);
});


async function cliCheckpoint(f, args, phase, name) {
  const directory=path.join(f.temp,name);fs.mkdirSync(directory,{mode:0o700});
  const binary=boot.enginePath(f.manifest,f.target,f.env);
  const child=spawn(binary,args,{cwd:f.project,env:{...f.env,PHOENIX_DIAGRAM_ACCEPTANCE:'1',PHOENIX_DIAGRAM_CHECKPOINT_DIR:directory,PHOENIX_DIAGRAM_CHECKPOINT_PHASE:phase},stdio:['ignore','pipe','pipe']});
  let stdout='',stderr='';child.stdout.on('data',b=>{stdout+=b});child.stderr.on('data',b=>{stderr+=b});
  const exited=new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',(code,signal)=>resolve({code,signal}));});
  try {
    const markerPath=path.join(directory,'checkpoint.json');const deadline=Date.now()+20000;
    while(!fs.existsSync(markerPath) && child.exitCode===null && child.signalCode===null && Date.now()<deadline) await new Promise(resolve=>setTimeout(resolve,25));
    assert(fs.existsSync(markerPath),`real CLI did not reach ${phase}; exit=${child.exitCode} ${stdout} ${stderr}`);
    const marker=JSON.parse(fs.readFileSync(markerPath));assert.equal(marker.pid,child.pid);assert.equal(marker.phase,phase);
    const stat=fs.lstatSync(markerPath);assert(stat.isFile());assert.equal(stat.mode&0o7777,0o600);assert.equal(stat.nlink,1);
    child.kill('SIGKILL');const result=await exited;assert.equal(result.signal,'SIGKILL');
    return {marker,directory};
  } finally {if(child.exitCode===null&&child.signalCode===null)child.kill('SIGKILL');await exited;}
}

nativeTest('real CLI maintenance gates stop at durable legacy deletion and rollback boundaries', async t => {
  const f=fixture(t),old=legacy(f);const args=['migrate','--from-archify','--project',f.project,'--bundle-root',f.bundleRoot,'--providers','codex','--confirm-remove-archify','--json'];
  const paused=await cliCheckpoint(f,args,'migrate-after-legacy-remove','deletion-checkpoint');
  const journals=path.join(f.project,'.phoenix-diagram/install/journals');const name=fs.readdirSync(journals).find(name=>/^[a-f0-9]{32}\.json$/.test(name));assert(name);
  const journal=JSON.parse(fs.readFileSync(path.join(journals,name))),id=journal.transactionId;
  assert.equal(journal.phase,'applying');assert.equal(journal.migration.confirmed,true);assert.equal(journal.migration.roots[0].pending,0);
  assert.equal(paused.marker.recoveryDirectory,path.join(f.project,'.phoenix-diagram/install/transactions',id));
  assert.equal(paused.marker.target,path.join(f.project,journal.migration.roots[0].entries[0].path));assert.equal(fs.existsSync(paused.marker.target),false);assert(fs.existsSync(old));
  const recovered=run(f,['migrate','--recover',id,'--project',f.project,'--json']);assert.equal(recovered.status,0,recovered.stdout+recovered.stderr);assert.equal(JSON.parse(recovered.stdout).status,'recovered');assert.equal(fs.existsSync(old),false);
  const reverse=await cliCheckpoint(f,['migrate','--rollback',id,'--project',f.project,'--json'],'rollback-after-file','rollback-checkpoint');
  const rolling=JSON.parse(fs.readFileSync(path.join(journals,name)));assert.equal(rolling.phase,'rolling-back');assert.equal(reverse.marker.recoveryDirectory,paused.marker.recoveryDirectory);assert.equal(rolling.rollback.completed,0);
  const restored=run(f,['migrate','--recover',id,'--project',f.project,'--json']);assert.equal(restored.status,0,restored.stdout+restored.stderr);assert.equal(JSON.parse(restored.stdout).status,'rolled-back');assert.equal(fs.existsSync(old),false);
});

nativeTest('installer maintenance gate stays inert unless complete and rejects an enabled unknown phase', t => {
  const f=fixture(t),binary=boot.enginePath(f.manifest,f.target,f.env),missing=path.join(f.temp,'must-not-exist');
  const command=args=>spawnSync(binary,args,{cwd:f.project,env:{...f.env,PHOENIX_DIAGRAM_ACCEPTANCE:'0',PHOENIX_DIAGRAM_CHECKPOINT_DIR:missing,PHOENIX_DIAGRAM_CHECKPOINT_PHASE:'unknown'},encoding:'utf8',timeout:600000,maxBuffer:64*1024*1024});
  const installed=command(['install','--project',f.project,'--bundle-root',f.bundleRoot,'--providers','codex','--json']);assert.equal(installed.status,0,installed.stdout+installed.stderr);assert.equal(fs.existsSync(missing),false);
  legacy(f);const directory=path.join(f.temp,'unknown-phase');fs.mkdirSync(directory,{mode:0o700});const before=tree(f.project);
  const refused=spawnSync(binary,['migrate','--from-archify','--project',f.project,'--bundle-root',f.bundleRoot,'--providers','codex','--confirm-remove-archify','--json'],{cwd:f.project,env:{...f.env,PHOENIX_DIAGRAM_ACCEPTANCE:'1',PHOENIX_DIAGRAM_CHECKPOINT_DIR:directory,PHOENIX_DIAGRAM_CHECKPOINT_PHASE:'unknown'},encoding:'utf8',timeout:600000,maxBuffer:64*1024*1024});
  assert.equal(refused.status,2,refused.stdout+refused.stderr);assert.match(refused.stdout,/Unknown acceptance checkpoint phase/);assert.equal(fs.existsSync(path.join(directory,'checkpoint.json')),false);assert.deepEqual(tree(f.project),before);
});
