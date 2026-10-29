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
