'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const {createHash}=require('node:crypto');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const api=()=>import('../../scripts/check-phoenix-diagram-candidate.mjs');
function temporary(t) {const dir=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'diagram-consumer-unit-')));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));return dir;}
function inventoryFixture(t) {
  const dir=temporary(t),bytes=Buffer.from('inert inventory fixture, never native evidence');fs.writeFileSync(path.join(dir,'fixture.tgz'),bytes);
  const record={schemaVersion:1,tool:'phoenix-diagram',status:'assembled',handoff:'pending-consumer',releaseDir:'release-dir',files:[{path:'fixture.tgz',size:bytes.length,sha256:sha(bytes)}]};fs.writeFileSync(path.join(dir,'candidate.json'),JSON.stringify(record));return{dir,record};
}
test('consumer options bind a real host target and keep review resume explicit',async()=>{
  const {parseOptions,targetForHost}=await api();assert.equal(targetForHost('linux','arm64'),'linux-arm64');assert.throws(()=>targetForHost('win32','x64'),/unsupported/i);
  const options=parseOptions(['--candidate-dir','/candidate','--target',`${process.platform}-${process.arch}`]);assert.equal(options.candidateDir,'/candidate');
  assert.throws(()=>parseOptions(['--candidate-dir','/candidate','--target',`${process.platform}-${process.arch}`,'--review-ref','phoenix-diagram-review/123']),/resume/i);
  assert.throws(()=>parseOptions(['--candidate-dir','/candidate','--target','not-a-target']),/target|platform/i);
  assert.throws(()=>parseOptions(['--candidate-dir','a','--candidate-dir','b','--target','linux-x64']),/duplicate/i);
});
test('consumer accepts an explicit absolute npm CLI without changing its native Node target',async()=>{
  const {parseOptions}=await api(),args=['--candidate-dir','/candidate','--target',`${process.platform}-${process.arch}`];
  assert.equal(parseOptions([...args,'--npm-cli','/owned/npm/package/bin/npm-cli.js']).npmCliPath,'/owned/npm/package/bin/npm-cli.js');
  assert.throws(()=>parseOptions([...args,'--npm-cli','relative/npm-cli.js']),/absolute/i);
  assert.throws(()=>parseOptions([...args,'--npm-cli','/one','--npm-cli','/two']),/duplicate/i);
});
test('consumer environment retains npm diagnostics and never falls back from a bad selected CLI',async t=>{
  const {makeEnvironment}=await api(),directory=temporary(t),environment=makeEnvironment(directory);
  assert.equal(environment.nodePath,fs.realpathSync(process.execPath));assert.equal(environment.env.npm_config_logs_max,'10');
  const bad=path.join(temporary(t),'missing-cli.js');
  assert.throws(()=>makeEnvironment(temporary(t),{npmCliPath:bad}),/missing-cli|ENOENT/i);
});
test('resume binds the original selected npm CLI and rejects changed bytes or another path',async t=>{
  const {verifyNpmSelection}=await api(),{readSnapshot,bound}=await import('../../scripts/test-phoenix-diagram-core.mjs');
  const file=path.join(temporary(t),'npm-cli.js');fs.writeFileSync(file,'// inert binding fixture, never an npm installation\n');
  const environment={npmCliPath:file,npmCli:bound(readSnapshot(file))};
  assert.doesNotThrow(()=>verifyNpmSelection(environment,file));
  assert.throws(()=>verifyNpmSelection(environment,path.join(path.dirname(file),'other-cli.js')),/npm|selected|transport/i);
  fs.appendFileSync(file,'// changed\n');assert.throws(()=>verifyNpmSelection(environment,file),/changed|identity|binding/i);
});
test('candidate inventory rehashes every byte and rejects additions, aliases and traversal before writes',async t=>{
  const {verifyCandidateInventory}=await api();const f=inventoryFixture(t);assert.equal(verifyCandidateInventory(f.dir).files.size,1);
  fs.writeFileSync(path.join(f.dir,'fixture.tgz'),'tampered');assert.throws(()=>verifyCandidateInventory(f.dir),/digest|byte|size|inventory/i);
  fs.writeFileSync(path.join(f.dir,'fixture.tgz'),'inert inventory fixture, never native evidence');fs.writeFileSync(path.join(f.dir,'unknown'),'claimant');assert.throws(()=>verifyCandidateInventory(f.dir),/extra|inventory/i);fs.rmSync(path.join(f.dir,'unknown'));
  fs.symlinkSync(path.join(f.dir,'fixture.tgz'),path.join(f.dir,'unknown'));assert.throws(()=>verifyCandidateInventory(f.dir),/symbolic|symlink|ordinary|inventory/i);fs.rmSync(path.join(f.dir,'unknown'));
  f.record.files[0].path='../escape';fs.writeFileSync(path.join(f.dir,'candidate.json'),JSON.stringify(f.record));assert.throws(()=>verifyCandidateInventory(f.dir),/safe|path|relative/i);
});
test('candidate inventory does not follow hardlinked or symbolic metadata',async t=>{
  const {verifyCandidateInventory}=await api();const f=inventoryFixture(t),outside=path.join(temporary(t),'alias');fs.linkSync(path.join(f.dir,'fixture.tgz'),outside);assert.throws(()=>verifyCandidateInventory(f.dir),/link|ordinary/i);fs.rmSync(outside);
  fs.renameSync(path.join(f.dir,'candidate.json'),path.join(f.dir,'original'));fs.symlinkSync(path.join(f.dir,'original'),path.join(f.dir,'candidate.json'));assert.throws(()=>verifyCandidateInventory(f.dir),/link|ordinary/i);
});
test('review envelope rejects another tgz, source, nonce, run, state, target or case before writing local records',async()=>{
  const {validateReviewEnvelope}=await api();const state={repository:'owner/repo',runId:'123',target:'linux-x64',nonce:'a'.repeat(32),source:{commit:'b'.repeat(40)},candidate:{tarball:{identity:{sha256:'c'.repeat(64)}}},cases:[{id:'new-architecture'}]};
  const envelope={schemaVersion:1,kind:'phoenix-diagram-candidate-review',repository:state.repository,runId:state.runId,target:state.target,nonce:state.nonce,stateSha256:'d'.repeat(64),candidateSha256:'c'.repeat(64),sourceCommit:state.source.commit,records:{}};
  for(const [field,value] of [['repository','different/repo'],['runId','124'],['nonce','e'.repeat(32)],['target','darwin-arm64'],['stateSha256','f'.repeat(64)],['candidateSha256','f'.repeat(64)],['sourceCommit','f'.repeat(40)]])assert.throws(()=>validateReviewEnvelope({...envelope,[field]:value},state,{sha256:'d'.repeat(64)}),/binding|identity|nonce|run|source|state|candidate|target|repository/i,field);
  assert.throws(()=>validateReviewEnvelope(envelope,state,{sha256:'d'.repeat(64)}),/case|scope|record/i);
});
test('GitHub review decoding treats content as bounded data and never accepts another path or encoding',async()=>{
  const {decodeGitHubReview}=await api();const bytes=Buffer.from('{"schemaVersion":1}');const response={type:'file',path:'reviews/linux-x64.json',encoding:'base64',size:bytes.length,content:bytes.toString('base64')};assert.deepEqual(decodeGitHubReview(response,'linux-x64'),{schemaVersion:1});
  for(const change of [{path:'scripts/run.js'},{encoding:'utf-8'},{size:999},{content:'!!!!'},{type:'symlink'}])assert.throws(()=>decodeGitHubReview({...response,...change},'linux-x64'),/path|encoding|size|content|file|base64/i);
});
test('public npm adapter preserves argv, isolation and the exact package spec without invoking a copied native path',async()=>{
  const {publicNpmCommand}=await api();const adapter={schemaVersion:1,kind:'npm-public-bin',nodePath:'/trusted/node',npmCliPath:'/trusted/npm-cli.js',tarball:'/owned/candidate.tgz',cwd:'/owned/project',env:{PATH:'/owned/bin'},bin:'phoenix-diagram'};
  const command=publicNpmCommand(adapter,['review-record','/owned/a b.html','--record','/owned/review.json','--json']);assert.equal(command.program,adapter.nodePath);assert.deepEqual(command.args.slice(-6),['phoenix-diagram','review-record','/owned/a b.html','--record','/owned/review.json','--json']);assert.ok(command.args.includes(`--package=${adapter.tarball}`));assert.equal(command.cwd,adapter.cwd);assert.equal(command.env.PATH,'/owned/bin');assert.equal(command.args.includes('/copied/native'),false);
  assert.throws(()=>publicNpmCommand({...adapter,bin:'arbitrary-command'},[]),/bin|command/i);
});
test('source-less harness checks real files and rejects Rust, Cargo, archives and renderers',async t=>{
  const {verifySourceLessHarness}=await api();const directory=temporary(t);fs.writeFileSync(path.join(directory,'controller.mjs'),'// maintenance only\n');assert.equal(verifySourceLessHarness(directory).status,'pass');
  for(const name of ['source.rs','Cargo.lock','source.tar.gz']) {fs.writeFileSync(path.join(directory,name),'inert');assert.throws(()=>verifySourceLessHarness(directory),/Rust|Cargo|archive|source/i);fs.rmSync(path.join(directory,name));}
  fs.mkdirSync(path.join(directory,'renderers'));assert.throws(()=>verifySourceLessHarness(directory),/renderer/i);
});
test('review polling never applies a stale envelope and has a strict upper bound',async()=>{
  const {pollForReview}=await api();const state={repository:'owner/repo',runId:'123',target:'linux-x64',nonce:'a'.repeat(32),source:{commit:'b'.repeat(40)},candidate:{tarball:{identity:{sha256:'c'.repeat(64)}}},cases:[{id:'new-architecture'}]};let calls=0;
  const value={schemaVersion:1,kind:'phoenix-diagram-candidate-review',repository:state.repository,runId:state.runId,target:state.target,nonce:'f'.repeat(32),stateSha256:'d'.repeat(64),candidateSha256:'c'.repeat(64),sourceCommit:state.source.commit,records:{}},content=Buffer.from(JSON.stringify(value));
  await assert.rejects(()=>pollForReview(state,{sha256:'d'.repeat(64)},{timeoutMs:15,pollMs:1,fetchReview:async()=>{calls++;return{type:'file',path:'reviews/linux-x64.json',encoding:'base64',size:content.length,content:content.toString('base64')};}}),/pending.*nonce/i);assert.ok(calls>0);
  await assert.rejects(()=>pollForReview(state,{sha256:'d'.repeat(64)},{timeoutMs:5400001,fetchReview:async()=>{throw new Error('must not request');}}),/90 minutes/i);
});
test('consumer environments cannot serialize tokens or inherit arbitrary process settings',async()=>{
  const {checkedEnvironment}=await api();assert.deepEqual(checkedEnvironment({HOME:'/owned/home',PATH:'/owned/bin',PHOENIX_DIAGRAM_HOME:'/owned/cache'}),{HOME:'/owned/home',PATH:'/owned/bin',PHOENIX_DIAGRAM_HOME:'/owned/cache'});
  for(const key of ['GITHUB_TOKEN','GH_TOKEN','NODE_OPTIONS','npm_config__authToken'])assert.throws(()=>checkedEnvironment({HOME:'/owned/home',PATH:'/owned/bin',[key]:'never-write-this-secret'}),/environment|unapproved/i);
});
