'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const modulePath='../../tools/phoenix-diagram/tests/p3-acceptance.mjs';
const statePath='../../tools/phoenix-diagram/tests/support/p3-state.mjs';
function temp(t){const directory=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'phoenix-p3-controller-unit-')));t.after(()=>fs.rmSync(directory,{recursive:true,force:true}));return directory;}

test('author-tools options reject unsupported stages and ambiguous resume/review requests',async()=>{
  const {parseOptions}=await import(modulePath);
  assert.deepEqual(parseOptions(['--stage','author-tools','--release-dir','/release','--output-dir','/new']),{stage:'author-tools',releaseDir:'/release',outputDir:'/new'});
  assert.equal(parseOptions(['--stage','author-tools','--release-dir','/release','--resume','/run/state.json','--review-file','/review.json']).resume,'/run/state.json');
  for(const args of [
    ['--stage','all','--release-dir','/r','--output-dir','/new'],
    ['--stage','author-tools','--release-dir','/r','--output-dir','/new','--resume','/run/state.json'],
    ['--stage','author-tools','--release-dir','/r','--output-dir','/new','--review-file','/review.json'],
    ['--stage','author-tools','--release-dir','/r','--output-dir','/new','--release-dir','/other'],
  ])assert.throws(()=>parseOptions(args));
});

test('official resource descriptor requires the fixed commit URL and exact supplied PNG identity',async t=>{
  const {readOfficialResource,OFFICIAL_RESOURCE}=await import(statePath),root=temp(t),file=path.join(root,'official.json');
  const commit='a'.repeat(40),descriptor={schemaVersion:1,sourceCommit:commit,url:`https://raw.githubusercontent.com/XGENT-ai/skills/${commit}/tools/phoenix-diagram/tests/fixtures/brands/official-test.png`,...OFFICIAL_RESOURCE};
  fs.writeFileSync(file,JSON.stringify(descriptor));assert.equal(readOfficialResource(file).descriptor.sourceCommit,commit);
  for(const changed of [{...descriptor,url:descriptor.url.replace(commit,'main')},{...descriptor,sha256:'0'.repeat(64)},{...descriptor,bytes:126},{...descriptor,secret:'not allowed'}]){
    fs.writeFileSync(file,JSON.stringify(changed));assert.throws(()=>readOfficialResource(file));
  }
});

async function comparisonFixture(t){
  // Inert maintenance fixtures exercise controller contracts. These are not
  // native browser receipts, real screenshots, or an actual image review.
  const core=await import('../../scripts/test-phoenix-diagram-core.mjs'),model=await import(statePath),root=temp(t);fs.chmodSync(root,0o700);
  const put=(name,value)=>model.writeNew(path.join(root,name),value);
  const base=put('base.json','{"base":true}'),head=put('head.json','{"head":true}'),artifact=put('comparison.html','fixture comparison only');
  const engineFile=put('engine','inert maintenance fixture'),engine={sourceInputSha256:'a'.repeat(64),binary:engineFile.identity};
  const original={schemaVersion:1,tool:'phoenix-diagram',kind:'architecture-comparison',status:'pass',transaction:{status:'committed'},receiptId:'fixture-this-run',artifact:{path:artifact.path,identity:artifact.identity},comparisonIr:{identity:core.identity(Buffer.from('fixture IR'))}};
  for(const [name,candidate] of [['base',base],['head',head]])original[name]={candidate:{path:candidate.path,identity:candidate.identity},semanticSha256:'b'.repeat(64),configSha256:'c'.repeat(64),engine,config:{brandResources:{}}};
  const receipt=put('comparison.html.compare.json',original),comparison={receipt:{path:receipt.path,identity:receipt.identity},receiptId:original.receiptId,comparisonIr:original.comparisonIr.identity,artifact:original.artifact};
  for(const [name,candidate] of [['base',base],['head',head]])comparison[name]={candidate:{path:candidate.path,identity:candidate.identity},semanticSha256:original[name].semanticSha256,configSha256:original[name].configSha256,engine,resources:{}};
  const checks=core.BROWSER_CHECKS.map(name=>({name,status:'pass',scope:{fixture:true}}));
  const screenshots=['before','delta','after'].flatMap(viewId=>[[1440,900],[2048,1320]].flatMap(([width,height])=>['light','dark'].map(theme=>({...put(`${viewId}-${width}-${theme}.png`,'inert fixture, no actual image'),viewId,viewport:{width,height},theme}))));
  const browser={schemaVersion:1,kind:'architecture-comparison-browser',command:'visual-check',status:'pass',comparison,browserVersion:{product:'inert fixture; not a real browser'},checks,screenshots,coverage:['before','delta','after'].flatMap(viewId=>core.VIEWPORTS.flatMap(([width,height])=>['light','dark'].map(theme=>({viewId,viewport:{width,height},theme,status:'pass',checks}))))};
  const browserRef=put('browser.json',browser),summary={schemaVersion:1,tool:'phoenix-diagram',kind:'architecture-comparison-summary',command:'visual-check',status:'pass',automatedStatus:'pass',comparison,stages:[{name:'check',status:'pass',receipt:{status:'pass',strictProvenance:true,comparison}},{name:'visual-check',status:'pass',receipt:browser}],visualReview:{status:'required-pending',record:null},deliveryStatus:'incomplete',evidence:{browserReceipt:{path:browserRef.path,identity:browserRef.identity}}};
  const summaryRef=put('summary.json',summary),entry={id:'comparison',root,base,head,artifact,receipt,summary:summaryRef};
  const release={engineIdentity:engine,files:{engine:engineFile}};model.rememberCase(entry,release);
  const state={runId:'f'.repeat(32),runRoot:root,cases:{comparison:entry},release};
  return{root,put,core,model,entry,state,summary,browser,release};
}
function observedDraft(f){const draft=f.model.comparisonDraft(f.entry.evidence);draft.reviewer='Maintenance fixture, no actual image review';draft.verdict='passed';draft.findings=[];for(const shot of draft.screenshotIdentities){shot.observed=true;shot.observations='Inert fixture tests the observation binding only.';}return draft;}

test('comparison draft retains twelve current images and never fabricates observations',async t=>{
  const f=await comparisonFixture(t),draft=f.model.comparisonDraft(f.entry.evidence);
  assert.equal(draft.verdict,'unavailable');assert.equal(draft.reviewer,'');assert.equal(draft.screenshotIdentities.length,12);assert(draft.screenshotIdentities.every(shot=>shot.observed===false&&shot.observations===''));
  draft.verdict='passed';assert.throws(()=>f.model.verifyComparisonReview(draft,f.entry.evidence),/observation|reviewer/i);
  const valid=observedDraft(f);f.model.verifyComparisonReview(valid,f.entry.evidence);
  valid.screenshotIdentities[0].viewId='after';assert.throws(()=>f.model.verifyComparisonReview(valid,f.entry.evidence));
});

test('same-run review resume invokes only review-record and keeps compare/HTML/browser bytes',async t=>{
  const f=await comparisonFixture(t),{resumeReviews}=await import(modulePath),record=f.put('actual-review.json',observedDraft(f)),commands=[],before=[f.entry.artifact,f.entry.receipt,f.entry.evidence.browserReceipt];
  await resumeReviews(f.state,{records:{comparison:record.path}},{invoke:async(_engine,args)=>{
    commands.push(args);const sealed=f.put('sealed-review.json',f.model.jsonBound(record,'unit observation record'));
    const summary={...f.summary,visualReview:{status:'passed',record:{path:sealed.path,identity:sealed.identity}},deliveryStatus:'complete',evidence:{...f.summary.evidence,reviewRecord:{path:sealed.path,identity:sealed.identity}}};fs.writeFileSync(f.entry.summary.path,JSON.stringify(summary));
    const receipt={command:'review-record',automatedStatus:'pass',status:'pass',visualReview:summary.visualReview,deliveryStatus:'complete',evidence:{summaryReceipt:f.core.bound(f.core.readSnapshot(f.entry.summary.path)),reviewRecord:sealed}};
    return{exitCode:0,signal:null,stdout:JSON.stringify(receipt),stderr:'',receipt};
  }});
  assert.deepEqual(commands.map(args=>args[0]),['review-record']);assert.equal(f.entry.evidence.visualReview,'passed');assert.equal(f.entry.evidence.deliveryStatus,'complete');for(const file of before)f.core.readBound(file,'unchanged unit original');
});

test('recorded finalize, compare and visual-check steps are adopted without starting native processes',async t=>{
  const {invokeRecordedStep}=await import(modulePath),model=await import(statePath),root=temp(t);fs.chmodSync(root,0o700);model.privateDirectory(path.join(root,'commands'));
  const engine=model.writeNew(path.join(root,'engine'),'inert fixture; not a native engine'),state={runRoot:root,release:{files:{engine}},steps:{},diagnostics:[]};let calls=0;
  for(const command of ['finalize','compare','visual-check']){
    const directory=model.privateDirectory(path.join(root,'commands',command)),args=[command,'explicit unit argv','--json'],receipt={command,status:'pass'};
    const process=model.writeNew(path.join(directory,'process.json'),{schemaVersion:1,kind:'p3-owned-native-process',engine,nativeArgv:args,exitCode:0,signal:null,stdout:model.writeNew(path.join(directory,'stdout'),JSON.stringify(receipt)),stderr:model.writeNew(path.join(directory,'stderr'),'')});
    state.steps[command]={status:command==='visual-check'?'running':'pass',process:command==='visual-check'?undefined:process};
    const result=await invokeRecordedStep(state,command,args,{invoke:async()=>{calls++;throw new Error('Existing commands must not rerun');}});assert.equal(result.receipt.command,command);assert.equal(state.steps[command].status,'pass');
    await assert.rejects(invokeRecordedStep(state,command,[command,'different input'],{invoke:async()=>{calls++;}}),/another command/);
  }
  assert.equal(calls,0);
});

test('cross-run maps, old-image observations and unknown review fields fail before any native invocation',async t=>{
  const f=await comparisonFixture(t),{resumeReviews}=await import(modulePath),map=f.put('map.json',{schemaVersion:1,runId:'0'.repeat(32),records:{comparison:'/record.json'}});
  assert.throws(()=>f.model.readReviewMap(map.path,f.state),/another acceptance run/);
  const draft=observedDraft(f);draft.screenshotIdentities[0].file.identity.sha256='0'.repeat(64);const record=f.put('old-review.json',draft);let calls=0;
  await assert.rejects(resumeReviews(f.state,{records:{comparison:record.path}},{invoke:async()=>{calls++;throw new Error('must not invoke');}}));assert.equal(calls,0);
  const unknown=observedDraft(f);unknown.extra=true;assert.throws(()=>f.model.verifyComparisonReview(unknown,f.entry.evidence),/fields differ/);
});

test('a sealed review survives summary failure and retries only identical observation bytes',async t=>{
  const f=await comparisonFixture(t),{resumeReviews}=await import(modulePath),record=f.put('actual-review.json',observedDraft(f)),bytes=fs.readFileSync(record.path);let calls=0,sealed;
  const invoke=async(_engine,args)=>{
    assert.equal(args[0],'review-record');calls++;
    if(!sealed)sealed=f.put('sealed-review.json',bytes);
    if(calls===1){const receipt={command:'review-record',status:'failed',automatedStatus:'pass',visualReview:{status:'required-pending'},deliveryStatus:'incomplete',diagnostics:[{evidence:{sealedReviewRecord:sealed}}]};return{exitCode:4,signal:null,stdout:JSON.stringify(receipt),stderr:'injected summary update failure',receipt};}
    const summary={...f.summary,visualReview:{status:'passed',record:sealed},deliveryStatus:'complete',evidence:{...f.summary.evidence,reviewRecord:sealed}};fs.writeFileSync(f.entry.summary.path,JSON.stringify(summary));
    const receipt={command:'review-record',status:'pass',automatedStatus:'pass',visualReview:summary.visualReview,deliveryStatus:'complete',evidence:{summaryReceipt:f.core.bound(f.core.readSnapshot(f.entry.summary.path)),reviewRecord:sealed}};return{exitCode:0,signal:null,stdout:JSON.stringify(receipt),stderr:'',receipt};
  };
  await assert.rejects(resumeReviews(f.state,{records:{comparison:record.path}},{invoke}),/preserve and retry/);assert.equal(f.entry.evidence.visualReview,'required-pending');f.core.readBound(sealed,'sealed record after unit failure');
  fs.writeFileSync(record.path,Buffer.concat([bytes,Buffer.from('\n')]));await assert.rejects(resumeReviews(f.state,{records:{comparison:record.path}},{invoke}),/same record bytes/);assert.equal(calls,1);
  fs.writeFileSync(record.path,bytes);await resumeReviews(f.state,{records:{comparison:record.path}},{invoke});assert.equal(calls,2);assert.equal(f.entry.evidence.deliveryStatus,'complete');f.core.readBound(sealed,'same sealed record after retry');
});

test('artifact drift and missing view evidence cannot be adopted on resume',async t=>{
  const f=await comparisonFixture(t);fs.writeFileSync(f.entry.artifact.path,'changed artifact');assert.throws(()=>f.model.refreshCase(f.entry,f.release),/identity|this run|changed/);
  const g=await comparisonFixture(t);const changed=structuredClone(g.summary);changed.stages.pop();fs.writeFileSync(g.entry.summary.path,JSON.stringify(changed));assert.throws(()=>g.model.refreshCase(g.entry,g.release),/immutable automatic evidence/);
});

test('passing compare alone stays incomplete and a later state claimant is not overwritten',async t=>{
  const {recompute,persistState,privateDirectory}=await import(statePath),directory=temp(t);fs.chmodSync(directory,0o700);
  const state={runRoot:directory,cases:{comparison:{status:'pass'}},steps:{compare:{status:'pass'}},preview:{status:'not-run'},compareFaults:{status:'not-run'},brandExports:{status:'not-run'}};
  assert.equal(recompute(state).status,'incomplete');assert.equal(state.visualReview,'required-pending');assert.equal(state.deliveryStatus,'incomplete');
  const initial=persistState(state);const replacement=path.join(directory,'replacement');fs.writeFileSync(replacement,'later claimant');fs.renameSync(replacement,initial.path);assert.throws(()=>persistState(state,initial),/identity|binding/);assert.equal(fs.readFileSync(initial.path,'utf8'),'later claimant');privateDirectory(directory,{existing:true});
});

test('final candidate stages require candidate identity and retain pending review semantics',async()=>{
  const {parseOptions}=await import(modulePath);
  assert.equal(parseOptions(['--stage','all','--candidate-dir','/candidate','--output-dir','/new']).candidateDir,'/candidate');
  assert.equal(parseOptions(['--stage','providers','--candidate-dir','/candidate','--provider-manifest','/sessions.json','--output-dir','/new']).providerManifest,'/sessions.json');
  for(const args of [
    ['--stage','all','--release-dir','/development','--output-dir','/new'],
    ['--stage','providers','--candidate-dir','/candidate','--output-dir','/new'],
    ['--stage','author-tools','--candidate-dir','/candidate','--output-dir','/new'],
  ])assert.throws(()=>parseOptions(args));
});

test('public author-tools transport captures actual npm argv and preserves review/checkpoint settings',async t=>{
  const {createPublicInvoke}=await import(modulePath),{publicNpmCommand}=await import('../../scripts/check-phoenix-diagram-candidate.mjs'),model=await import(statePath),root=temp(t);
  const engine=model.writeNew(path.join(root,'engine'),'inert fixture, never executed'),tarball=model.writeNew(path.join(root,'package.tgz'),'inert package'),calls=[];
  const publicNpm={schemaVersion:1,kind:'npm-public-bin',bin:'phoenix-diagram',nodePath:'/node',npmCliPath:'/npm-cli.js',tarball:tarball.path,cwd:root,env:{PATH:'/isolated'},enginePath:engine.path};
  const invoke=createPublicInvoke({engine,publicNpm},{invoke:async(actual,args,settings)=>{
    calls.push({actual,args,settings});const command=publicNpmCommand(publicNpm,args),prefix=settings.prefix||[];
    await settings.onSpawn({pid:1234,spawnfile:prefix[0]||command.program,spawnargs:prefix.length?[...prefix,command.program,...command.args]:[command.program,...command.args]});
    return{exitCode:0,signal:null,stdout:'{}',stderr:'',receipt:{}};
  }});
  let observed;const result=await invoke(engine,['review-record','/artifact','--json'],{json:false,env:{PHOENIX_DIAGRAM_ACCEPTANCE:'1'},prefix:['/sandbox','rule'],onSpawn:child=>{observed=child.pid;}});
  assert.equal(observed,1234);assert.equal(calls[0].settings.options.publicNpm,publicNpm);assert.equal(calls[0].settings.json,false);assert.equal(calls[0].settings.env.PHOENIX_DIAGRAM_ACCEPTANCE,'1');
  assert.equal(result.actualTransport.pid,1234);assert.deepEqual(result.actualTransport.argv,publicNpmCommand(publicNpm,calls[0].args).args);
  await assert.rejects(invoke({...engine,path:'/different-engine'},['doctor','--json']),/engine|transport/i);
  const missing=createPublicInvoke({engine,publicNpm},{invoke:async()=>({exitCode:0,signal:null})});await assert.rejects(missing(engine,['doctor','--json']),/spawn|transport/i);
});

test('comparison byte verifier shares strict checks with archived originals without recertifying inodes',async t=>{
  const f=await comparisonFixture(t),snapshots=new Map();
  for(const file of [f.entry.summary,f.entry.base,f.entry.head,f.entry.artifact,f.entry.receipt,f.entry.evidence.browserReceipt,...f.entry.evidence.screenshots])snapshots.set(file.path,f.core.readBound(file).content);
  fs.rmSync(f.entry.artifact.path);
  const context={base:f.entry.base,head:f.entry.head,artifact:f.entry.artifact,receipt:f.entry.receipt,engine:f.release.engineIdentity,bytesFor:reference=>snapshots.get(reference.path)};
  assert.equal(f.model.inspectComparison(f.entry.summary,context).automatedStatus,'pass');
  assert.throws(()=>f.model.inspectComparison(f.entry.summary,{...context,bytesFor:reference=>reference.path===f.entry.artifact.path?Buffer.from('forged'):snapshots.get(reference.path)}),/identity|bytes|digest/i);
  assert.throws(()=>f.model.inspectComparison(f.entry.summary,{...context,bytesFor:undefined}),/ENOENT|identity|binding/);
});

test('public process ownership proves the real controller and refuses an unrelated marker PID',async()=>{
  const {proveControllerOwnership}=await import('../../tools/phoenix-diagram/tests/support/p3-processes.mjs');
  const engine={path:'/owned/engine'},child={pid:100},rows=[{pid:100,ppid:1,pgid:100,command:'/node npm-cli exec'},{pid:101,ppid:100,pgid:100,command:'/node public-bin'},{pid:102,ppid:101,pgid:100,command:'/owned/engine preview architecture /input /output --json'},{pid:103,ppid:102,pgid:103,command:'/owned/engine __preview-worker'}];
  const proof=proveControllerOwnership(child,engine,rows,{controllerPid:102,workerPid:103});assert.equal(proof.controllerPid,102);assert.deepEqual(proof.chain.map(row=>row.pid),[102,101,100]);
  assert.throws(()=>proveControllerOwnership(child,engine,[...rows,{pid:999,ppid:1,pgid:999,command:'/owned/engine preview'}],{controllerPid:999}),/owned|ancestor|group/i);
  assert.throws(()=>proveControllerOwnership(child,engine,rows,{controllerPid:101}),/engine|controller/i);
  assert.throws(()=>proveControllerOwnership(child,engine,rows,{controllerPid:102,workerPid:101}),/worker|parent/i);
});

async function inertCandidate(t){
  // Protocol/controller fixture only: no candidate authentication or native
  // acceptance is claimed by these injected dependencies.
  const model=await import(statePath),directory=temp(t),put=(name,bytes)=>{const file=path.join(directory,name);fs.writeFileSync(file,bytes);return model.readSnapshot(file);};
  const inputs=put('source-inputs.json','[["inert",0,"'+ '0'.repeat(64)+'"]]'),tgz=put('package.tgz','inert npm package, never installed'),source={commit:'a'.repeat(40),inputSha256:inputs.identity.sha256};
  const version={distribution:'candidate',sourceCommit:source.commit,sourceInputSha256:source.inputSha256},versionFile=put('VERSION.json',JSON.stringify(version)),snapshot=put('candidate.json',JSON.stringify({source,tarball:{path:'package.tgz',size:tgz.identity.bytes,sha256:tgz.identity.sha256}}));
  return{directory,tgz,snapshot,source,version,files:new Map([['source-inputs.json',inputs],['release-dir/VERSION.json',versionFile]])};
}

test('provider scope archives the supplied manifest and never claims final candidate certification',async t=>{
  const candidate=await inertCandidate(t),{runProviders}=await import(modulePath),model=await import(statePath),outputDir=path.join(temp(t),'providers'),manifest=path.join(temp(t),'sessions.json');fs.writeFileSync(manifest,'{"schemaVersion":1,"kind":"phoenix-diagram-p3-provider-manifest","providers":[]}');
  let called=0;const result=await runProviders({stage:'providers',candidateDir:candidate.directory,providerManifest:manifest,outputDir},{candidateLoader:(_directory,_target,options)=>{assert.equal(options.profile,'p3');return candidate;},validateProviders:async options=>{called++;assert.equal(options.profile,'p3');assert.equal(options.manifest.providers.length,0);return{schemaVersion:1,kind:'phoenix-diagram-p3-provider-report',profile:'p3',status:'incomplete',providers:[],diagnostics:[{message:'No actual sessions in this unit fixture'}]};}});
  assert.equal(called,1);assert.equal(result.exitCode,3);assert.equal(result.providers.status,'incomplete');assert.equal(result.finalCandidate.handoff,'pending-aggregate');assert.equal(path.dirname(result.providers.manifest.path),result.runRoot);model.readBound(result.providers.manifest);
  assert.equal(fs.readFileSync(manifest,'utf8'),'{"schemaVersion":1,"kind":"phoenix-diagram-p3-provider-manifest","providers":[]}');
  const alias=path.join(temp(t),'candidate-alias');fs.symlinkSync(candidate.directory,alias);
  await assert.rejects(runProviders({stage:'providers',candidateDir:candidate.directory,providerManifest:manifest,outputDir:path.join(alias,'must-not-create')},{candidateLoader:()=>candidate}),/candidate inventory/);assert(!fs.existsSync(path.join(candidate.directory,'must-not-create')));
});

test('all preserves a failed child consumer and does not start author-tools or promote the candidate',async t=>{
  const candidate=await inertCandidate(t),{runAll}=await import(modulePath),model=await import(statePath),outputDir=path.join(temp(t),'all');let consumeCalls=0,authorCalls=0;
  const result=await runAll({stage:'all',candidateDir:candidate.directory,outputDir},{candidateLoader:()=>candidate,consume:async options=>{consumeCalls++;assert.equal(options.profile,'p3');assert.equal(options.explore,true);assert.equal(options.locale,true);assert.equal(options.evidence,undefined);model.privateDirectory(options.evidenceDir);const state={schemaVersion:1,kind:'phoenix-diagram-candidate-consumer',profile:'p3',status:'failed',automatedStatus:'not-run',phase:'failed',scopes:['explore','locale']};return{state,stateFile:model.writeNew(path.join(options.evidenceDir,'consumer-state.json'),state)};},authorTools:async()=>{authorCalls++;throw new Error('Failed consumption cannot launch author tools');}});
  assert.equal(consumeCalls,1);assert.equal(authorCalls,0);assert.equal(result.exitCode,1);assert.equal(result.packageConsumer.status,'failed');assert.equal(result.finalCandidate.status,'incomplete');assert.equal(result.consumerStateFile.path,path.join(result.runRoot,'consumer/consumer-state.json'));model.readBound(result.consumerStateFile);
});

test('all routes historical load installation and resume through its descriptor and current harness owner',async t=>{
  // Injected protocol fixtures only; no native, browser or package acceptance.
  const candidate=await inertCandidate(t),old=await inertCandidate(t),model=await import(statePath),{runAll}=await import(modulePath),inputs=(await import('../../scripts/build-phoenix-diagram.mjs')).sourceInputs();
  candidate.sourceInputs=inputs;fs.writeFileSync(candidate.files.get('source-inputs.json').path,inputs.bytes);candidate.files.set('source-inputs.json',model.readSnapshot(candidate.files.get('source-inputs.json').path));
  for(const [item,commit,text] of [[candidate,'a'.repeat(40),'inert current package'],[old,'b'.repeat(40),'inert historical package']]){
    item.source={...item.source,commit,...(item===candidate?{inputSha256:inputs.sha256}:{})};fs.writeFileSync(item.tgz.path,text);item.tgz=model.readSnapshot(item.tgz.path);
    item.version={...item.version,sourceCommit:commit,sourceInputSha256:item.source.inputSha256};fs.writeFileSync(item.files.get('release-dir/VERSION.json').path,JSON.stringify(item.version));item.files.set('release-dir/VERSION.json',model.readSnapshot(item.files.get('release-dir/VERSION.json').path));
    fs.writeFileSync(item.snapshot.path,JSON.stringify({source:item.source,tarball:{path:'package.tgz',size:item.tgz.identity.bytes,sha256:item.tgz.identity.sha256}}));item.snapshot=model.readSnapshot(item.snapshot.path);
  }
  const outputDir=path.join(temp(t),'all'),engine=model.writeNew(path.join(candidate.directory,'engine'),'inert current engine'),oldEngine=model.writeNew(path.join(old.directory,'engine'),'inert historical engine'),engineIdentity={sourceInputSha256:candidate.source.inputSha256};
  const publicNpm={schemaVersion:1,kind:'npm-public-bin',bin:'phoenix-diagram',nodePath:'/node',npmCliPath:'/npm-cli.js',tarball:candidate.tgz.path,cwd:outputDir,env:{PATH:'/isolated'},enginePath:engine.path};
  const consumer={schemaVersion:1,kind:'phoenix-diagram-candidate-consumer',profile:'p3',status:'automated-pass',automatedStatus:'pass',phase:'pending-review',scopes:['explore','locale'],source:candidate.source,candidate:{tarball:model.bound(candidate.tgz)},engine:{...engine,engineIdentity},publicNpm};
  const release={distribution:'candidate',sourceCommit:candidate.source.commit,sourceInputSha256:candidate.source.inputSha256,files:{engine,version:model.bound(candidate.files.get('release-dir/VERSION.json'))},candidate:{record:model.bound(candidate.snapshot),tarball:model.bound(candidate.tgz)},engineIdentity,publicNpm};
  const oldState={schemaVersion:1,kind:'phoenix-diagram-candidate-install-context',profile:'p2',scope:'installation-only',status:'pass',automatedStatus:'not-run',phase:'installed',source:old.source,candidate:{tarball:model.bound(old.tgz),ownedTarball:model.bound(old.tgz)},engine:oldEngine,publicNpm:{...publicNpm,tarball:old.tgz.path,enginePath:oldEngine.path}};
  let phase='initial';const calls=[];
  const routed=options=>{assert.equal(options.profile,'p2');assert.equal(options.harnessCandidateDir,candidate.directory);assert.equal(model.readBound(options.legacyDescriptor).identity.sha256,'193f11ed47b7eecc72662c3d8c594ac768bdcd7565692b9901efb0ce22fb6a95');};
  const dependencies={
    candidateLoader:(directory,_target,options)=>{if(directory===candidate.directory){assert.equal(options.profile,'p3');assert.equal(options.legacyDescriptor,undefined);return candidate;}assert.equal(directory,old.directory);routed(options);calls.push([phase,'load']);return old;},
    consume:async options=>{model.privateDirectory(options.evidenceDir);return{state:consumer,stateFile:model.writeNew(path.join(options.evidenceDir,'consumer-state.json'),consumer)};},consumerContext:()=>({state:consumer}),releaseLoader:()=>release,
    authorTools:async options=>{if(options.resume)return{stateFile:model.bound(model.readSnapshot(options.resume))};model.privateDirectory(options.outputDir);const state=model.recompute({schemaVersion:1,kind:model.KIND,profile:'p3',stage:'author-tools',runId:'e'.repeat(32),runRoot:options.outputDir,release,cases:{},steps:{},preview:{status:'not-run'},compareFaults:{status:'not-run'},brandExports:{status:'not-run'}});return{stateFile:model.persistState(state)};},
    prepareLegacy:async options=>{routed(options);assert.equal(options.candidateDir,old.directory);calls.push([phase,'prepare']);model.privateDirectory(options.evidenceDir);return{state:oldState,stateFile:model.writeNew(path.join(options.evidenceDir,'consumer-state.json'),oldState)};},
    legacyContext:options=>{routed(options);assert.equal(options.candidateDir,old.directory);assert.equal(options.resume,path.join(outputDir,'legacy-v1/consumer-state.json'));calls.push([phase,'context']);return{state:oldState};},
    runMigration:async()=>({status:'incomplete',record:model.writeNew(path.join(outputDir,'migration.json'),{scope:'Inert routing fixture; no migrations ran'})}),verifyMigration:()=>{calls.push([phase,'verify']);}
  };
  const options={stage:'all',candidateDir:candidate.directory,legacyCandidateDir:old.directory,migrationFixture:path.resolve(__dirname,'../../tools/phoenix-diagram/tests/fixtures/migration/legacy-sources.v1.json.gz'),outputDir};
  const initial=await runAll(options,dependencies);assert.equal(initial.failure,undefined);assert.equal(initial.exitCode,3);for(const kind of ['load','prepare','context'])assert(calls.some(row=>row[0]==='initial'&&row[1]===kind));
  phase='resume';const resumeOptions={...options,resume:initial.stateFile.path};delete resumeOptions.outputDir;const resumed=await runAll(resumeOptions,dependencies);assert.equal(resumed.failure,undefined);assert.equal(resumed.exitCode,3);for(const kind of ['load','context','verify'])assert(calls.some(row=>row[0]==='resume'&&row[1]===kind));assert(!calls.some(row=>row[0]==='resume'&&row[1]==='prepare'));
});

test('wrapper automatic gates, local image review and provider evidence stay separate',async()=>{
  const {recomputeAcceptance}=await import(statePath);
  const state={packageConsumer:{status:'automated-pass',automatedStatus:'pass',phase:'pending-review',scopes:['explore','evidence','locale']},authorTools:{status:'incomplete',automatedStatus:'pass'},migration:{status:'not-run'},providers:{status:'not-run'}};
  assert.equal(recomputeAcceptance(state).automatedStatus,'not-run');state.migration.status='pass';assert.equal(recomputeAcceptance(state).automatedStatus,'pass');assert.equal(state.status,'incomplete');assert.equal(state.localStatus,'incomplete');
  Object.assign(state.packageConsumer,{status:'pass',phase:'complete'});state.authorTools.status='pass';assert.equal(recomputeAcceptance(state).localStatus,'complete');assert.equal(state.status,'incomplete');state.providers.status='pass';assert.equal(recomputeAcceptance(state).status,'pass');assert.equal(state.finalCandidate.handoff,'pending-aggregate');
  state.packageConsumer.scopes=['explore','locale'];assert.equal(recomputeAcceptance(state).localStatus,'incomplete');assert.equal(state.status,'incomplete');
});

test('workflow outputs preserve child state keys and reject multiline paths without partial output',async t=>{
  const {writeWorkflowOutputs}=await import(modulePath),file=path.join(temp(t),'github-output');fs.writeFileSync(file,'state_path=/consumer/original.json\nevidence_dir=/consumer\n');
  writeWorkflowOutputs({stateFile:{path:'/all/state.json'},runRoot:'/all'},file);const before=fs.readFileSync(file,'utf8');assert.equal(before,'state_path=/consumer/original.json\nevidence_dir=/consumer\np3_state_path=/all/state.json\np3_evidence_dir=/all\n');
  assert.throws(()=>writeWorkflowOutputs({stateFile:{path:'/all/state.json'},runRoot:'/all\nforged=value'},file),/single absolute path/);assert.equal(fs.readFileSync(file,'utf8'),before);writeWorkflowOutputs({},file);assert.equal(fs.readFileSync(file,'utf8'),before);
});

test('all resume retains its child states and shared archive verification cannot promote an unrun author scope',async t=>{
  const candidate=await inertCandidate(t),api=await import(modulePath),model=await import(statePath),outputDir=path.join(temp(t),'all');let consumes=0,authors=0;
  const deps={candidateLoader:()=>candidate,consume:async options=>{
    consumes++;const root=model.privateDirectory(options.evidenceDir),engine=model.writeNew(path.join(root,'engine'),'inert engine, not executable'),engineIdentity={sourceInputSha256:candidate.source.inputSha256,binary:engine.identity};
    const publicNpm={schemaVersion:1,kind:'npm-public-bin',bin:'phoenix-diagram',nodePath:'/node',npmCliPath:'/npm-cli.js',cwd:root,env:{PATH:'/isolated'},tarball:candidate.tgz.path,enginePath:engine.path};
    const state={schemaVersion:1,kind:'phoenix-diagram-candidate-consumer',profile:'p3',target:'fixture',status:'automated-pass',automatedStatus:'pass',phase:'pending-review',scopes:['explore','locale'],source:candidate.source,candidate:{tarball:model.bound(candidate.tgz),ownedTarball:model.bound(candidate.tgz)},engine:{...engine,engineIdentity},publicNpm};
    return{state,stateFile:model.writeNew(path.join(root,'consumer-state.json'),state)};
  },consumerContext:options=>({state:model.jsonBound(model.bound(model.readSnapshot(options.resume)),'synthetic child state')}),releaseLoader:(_candidate,consumer)=>({distribution:'candidate',sourceInputSha256:candidate.source.inputSha256,sourceCommit:candidate.source.commit,files:{version:model.bound(candidate.files.get('release-dir/VERSION.json')),engine:model.bound(model.readSnapshot(consumer.engine.path))},engineIdentity:consumer.engine.engineIdentity,publicNpm:consumer.publicNpm,candidate:{record:model.bound(candidate.snapshot),tarball:model.bound(candidate.tgz)}}),authorTools:async(options,settings)=>{
    authors++;if(options.resume){const ref=model.bound(model.readSnapshot(options.resume));return{...model.jsonBound(ref,'synthetic original author state'),stateFile:ref};}
    const root=model.privateDirectory(options.outputDir),state={schemaVersion:1,kind:model.KIND,profile:'p3',stage:'author-tools',runId:'b'.repeat(32),runRoot:root,release:settings.releaseLoader(),cases:{},steps:{},preview:{status:'not-run'},compareFaults:{status:'not-run'},brandExports:{status:'not-run'},packageConsumer:{status:'not-applicable'},providers:{status:'not-applicable'},finalCandidate:{status:'not-applicable'}};model.recompute(state);return{...state,stateFile:model.writeNew(path.join(root,'state.json'),state)};
  }};
  const first=await api.runAll({stage:'all',candidateDir:candidate.directory,outputDir},deps);assert.equal(first.exitCode,3);assert.equal(first.automatedStatus,'not-run');assert.equal(first.authorTools.status,'incomplete');
  const next=await api.runAll({stage:'all',candidateDir:candidate.directory,resume:first.stateFile.path},deps);assert.equal(next.exitCode,3);assert.equal(consumes,1);assert.equal(authors,2);assert.deepEqual(next.consumerStateFile,first.consumerStateFile);assert.deepEqual(next.authorToolsStateFile,first.authorToolsStateFile);
  const state=model.jsonBound(next.stateFile,'unit wrapper state'),archive=new Map();
  const capture=directory=>{for(const name of fs.readdirSync(directory)){const file=path.join(directory,name);if(fs.statSync(file).isDirectory())capture(file);else archive.set(file,fs.readFileSync(file));}};capture(candidate.directory);capture(outputDir);
  const bytesFor=ref=>archive.get(ref.path),verifyConsumer=consumer=>assert.equal(consumer.automatedStatus,'pass');fs.unlinkSync(state.authorTools.anchor.release.files.engine.path);
  assert.equal(model.verifyAcceptanceState(state,{bytesFor,verifyConsumer}).automatedStatus,'not-run');assert.throws(()=>model.verifyAcceptanceState(state,{verifyConsumer}),/ENOENT/);
  const forged=structuredClone(state);forged.automatedStatus='pass';assert.throws(()=>model.verifyAcceptanceState(forged,{bytesFor,verifyConsumer}),/promoted/);
  assert.throws(()=>model.verifyAcceptanceState(state,{bytesFor:ref=>ref.path===state.authorTools.state.path?Buffer.from('changed original state'):bytesFor(ref),verifyConsumer}),/bytes|identity/i);
});
