'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const os=require('node:os');
const modulePath='../../scripts/test-phoenix-diagram-core.mjs';

function temp(t) {const directory=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'phoenix-core-controller-unit-')));t.after(()=>fs.rmSync(directory,{recursive:true,force:true}));return directory;}

test('core options separate initial execution, review-only resume and unimplemented stages',async()=>{
  const {parseOptions}=await import(modulePath);
  const initial=parseOptions(['--stage','core','--engine','/native/phoenix-diagram','--output-dir','/tmp/core']);
  assert.equal(initial.stage,'core');assert.equal(initial.engine,'/native/phoenix-diagram');assert.equal(initial.reader,false);assert.equal(initial.recovery,false);
  const resume=parseOptions(['--stage','core','--engine','/native/phoenix-diagram','--resume','/tmp/core/index.json','--review-file','/tmp/reviews.json']);
  assert.equal(resume.resume,'/tmp/core/index.json');assert.equal(resume.reviewFile,'/tmp/reviews.json');
  assert.throws(()=>parseOptions(['--stage','install','--engine','/native/phoenix-diagram']),/not implemented/i);
  assert.throws(()=>parseOptions(['--stage','core','--engine','/native/phoenix-diagram','--review-file','/tmp/reviews.json']),/resume/i);
  assert.throws(()=>parseOptions(['--stage','core','--engine','/native/phoenix-diagram','--engine','/other']),/duplicate/i);
});

test('coverage requires each type in both contracts and rejects duplicate case identities',async()=>{
  const {requiredCoverage,TYPES}=await import(modulePath);
  const cases=TYPES.flatMap(type=>['phoenix-v1','legacy-2.17'].map(contract=>({id:`${contract==='phoenix-v1'?'new':'legacy'}-${type}`,type,contract})));
  assert.equal(requiredCoverage(cases).complete,true);
  const missing=requiredCoverage(cases.slice(1));assert.equal(missing.complete,false);assert.equal(missing.missing.length,1);
  assert.throws(()=>requiredCoverage([...cases,cases[0]]),/duplicate/i);
  assert.throws(()=>requiredCoverage([{id:'../escape',type:'architecture',contract:'phoenix-v1'}]),/case|identity|unsafe/i);
});

test('controller snapshots reject symlinks and same-byte physical replacements',async t=>{
  const {readSnapshot}=await import(modulePath);const directory=temp(t),file=path.join(directory,'input.json');fs.writeFileSync(file,'{"facts":true}\n');
  const before=readSnapshot(file);assert.equal(before.identity.bytes,15);assert.ok(before.identity.sha256.match(/^[0-9a-f]{64}$/));
  const replacement=path.join(directory,'replacement');fs.writeFileSync(replacement,fs.readFileSync(file));fs.renameSync(replacement,file);
  const after=readSnapshot(file);assert.deepEqual(before.identity,after.identity);assert.notDeepEqual(before.binding,after.binding);
  const alias=path.join(directory,'alias');fs.symlinkSync(file,alias);assert.throws(()=>readSnapshot(alias),/symlink|ordinary|symbolic|loop/i);
});

async function finalizeFixture(t,{contract='phoenix-v1',directory=temp(t),marked=true}={}) {
  const {identity,ARTIFACT_CHECKS,BROWSER_CHECKS,VIEWPORTS}=await import(modulePath);
  const write=(name,bytes)=>{const file=path.join(directory,name);fs.writeFileSync(file,bytes);return{path:file,identity:identity(Buffer.from(bytes))};};
  const candidate=write('candidate.json',JSON.stringify({diagram_type:'architecture',meta:marked?{input_contract:contract}:{}})),artifact=write('diagram.html','unit fixture; not a rendered acceptance artifact');
  const engine={sourceInputSha256:'a'.repeat(64),inputContractSchema:1,receiptSchema:1,viewerRevision:'phoenix-v1',binary:identity(Buffer.from('unit fixture'))};
  // The actual legacy native receipt executes these 20 scopes. These inert
  // files exercise the ledger protocol, never native/browser acceptance.
  const checks=ARTIFACT_CHECKS.filter(name=>contract!=='legacy-2.17' || !['arrowhead_separation','label_canvas_containment'].includes(name)).map(name=>({name,status:'pass',scope:{unitFixture:true}}));const composition={schemaVersion:1,profile:'showcase',status:'pass',summary:{errors:0,warnings:0}};
  const config={diagramType:'architecture',inputContract:contract,quality:'showcase',viewerRevision:'phoenix-v1'};
  const common={schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.1.0',status:'pass',diagnostics:[],candidate,engine,configSha256:'c'.repeat(64)};
  const validate={...common,command:'validate',artifact:artifact.identity,config,checks,composition};
  const deliver={...common,command:'deliver',artifact,inputContract:contract,config,receiptId:'11111111-1111-4111-8111-111111111111',commitState:'committed',checks,composition};
  const delivery=write('diagram.delivery.json',JSON.stringify(deliver));
  const checked={...common,command:'check',artifact,scope:'strict-provenance',provenance:'current',deliveryReceiptId:deliver.receiptId,deliveryReceipt:delivery,checks,composition};
  const screenshots=[[1440,900],[2048,1320]].flatMap(([width,height])=>['light','dark'].map(theme=>({...write(`${width}-${theme}.png`,'unit screenshot identity fixture'),viewport:{width,height},width,height,theme,fullPage:false})));
  const browser={...common,command:'browser-check',artifactPath:artifact.path,artifact:artifact.identity,deliveryReceiptId:deliver.receiptId,browserVersion:{product:'Unit protocol fixture'},coverage:VIEWPORTS.flatMap(([width,height])=>['light','dark'].map(theme=>({viewport:{width,height},theme,status:'pass',checks:BROWSER_CHECKS.map(name=>({name,status:'pass',scope:{unitFixture:true}}))}))),checks:[...BROWSER_CHECKS,'browser-cleanup','artifact-current','screenshot-capture'].map(name=>({name,status:'pass',scope:{unitFixture:true}})),screenshots};
  const browserFile=write('diagram.browser-check.json',JSON.stringify(browser));
  const summary={...common,command:'finalize',artifact,deliveryReceiptId:deliver.receiptId,browserReceiptIdentity:browserFile.identity,automatedStatus:'pass',visualReview:{status:'required-pending',record:null},deliveryStatus:'incomplete',producedByThisRun:true,stages:[validate,deliver,checked,browser].map(receipt=>({name:receipt.command,status:'pass',receipt})),evidence:{browserReceipt:browserFile}};
  const summaryFile=write('diagram.finalize-summary.json',JSON.stringify(summary));summary.evidence.summaryReceipt=summaryFile;
  return{summary,context:{candidate,artifact,engine,contract,type:'architecture'},screenshots,browserFile,summaryFile};
}

test('declared legacy native scope accepts its 20 checks while phoenix-v1 requires 22',async t=>{
  const {inspectFinalize,requiredArtifactChecks}=await import(modulePath),legacy=await finalizeFixture(t,{contract:'legacy-2.17'});
  assert.equal(inspectFinalize(legacy.summary,legacy.context).automatedStatus,'pass');
  assert.equal(requiredArtifactChecks('legacy-2.17').length,20);assert.equal(requiredArtifactChecks('phoenix-v1').length,22);
  assert.throws(()=>requiredArtifactChecks(undefined),/contract/i);assert.throws(()=>requiredArtifactChecks('future-v2'),/contract/i);
  for(const mutate of [s=>s.stages[1].receipt.inputContract='future-v2',s=>s.stages[0].receipt.config={...s.stages[0].receipt.config,inputContract:'phoenix-v1'},s=>s.stages[0].receipt.checks=s.stages[0].receipt.checks.slice(1)]){
    const changed=structuredClone(legacy.summary);mutate(changed);assert.throws(()=>inspectFinalize(changed,legacy.context),/contract|check|scope/i);
  }
  const current=await finalizeFixture(t);for(const stage of current.summary.stages.slice(0,3))stage.receipt.checks=stage.receipt.checks.filter(row=>!['arrowhead_separation','label_canvas_containment'].includes(row.name));
  assert.throws(()=>inspectFinalize(current.summary,current.context),/check|scope/i);
  const explicit=await finalizeFixture(t,{marked:false});assert.equal(inspectFinalize(explicit.summary,explicit.context).automatedStatus,'pass');
  assert.throws(()=>inspectFinalize(explicit.summary,{...explicit.context,contract:'legacy-2.17'}),/contract/i);
});

async function failedLedgerFixture(t) {
  const {readSnapshot,bound}=await import(modulePath),runRoot=temp(t),directory=path.join(runRoot,'cases','legacy-architecture');fs.mkdirSync(directory,{recursive:true,mode:0o700});
  const f=await finalizeFixture(t,{contract:'legacy-2.17',directory}),processPath=path.join(directory,'finalize-process.json');
  fs.writeFileSync(processPath,JSON.stringify({exitCode:0,signal:null,stdout:JSON.stringify(f.summary),stderr:''}));
  const entry={id:'legacy-architecture',type:'architecture',contract:'legacy-2.17',root:directory,candidate:bound(readSnapshot(f.context.candidate.path)),source:bound(readSnapshot(f.context.candidate.path)),artifactPath:f.context.artifact.path,process:bound(readSnapshot(processPath)),nativeExitCode:1,error:{code:'core/evidence',message:'validate is missing a required named check scope'},state:{automatedStatus:'failed',visualReview:'required-pending',deliveryStatus:'incomplete'}};
  return{f,entry,index:{runRoot,engine:{path:'/unit/native-engine-fixture',engineIdentity:f.context.engine},cases:[entry]}};
}

test('resume restores only a bound successful native finalize and replays into a distinct output',async t=>{
  const {resumeCaseEvidence}=await import('../../scripts/test-phoenix-diagram-runtime.mjs'),{readSnapshot,bound}=await import(modulePath),{f,index,entry}=await failedLedgerFixture(t),calls=[];
  const originalSummary=fs.readFileSync(f.summaryFile.path),originalHtml=fs.readFileSync(f.context.artifact.path);
  entry.determinism={status:'pass',sameHtml:true}; // An old ledger claim supplies no evidence.
  await resumeCaseEvidence(index,{}, {invoke:async(_engine,args)=>{
    calls.push(args);assert.notEqual(args[0],'finalize');
    if(args[0]==='check')return{exitCode:0,receipt:f.summary.stages[2].receipt};
    assert.equal(args[0],'deliver');assert.notEqual(args[3],entry.artifactPath);assert.deepEqual(args.slice(4),['--input-contract','legacy-2.17','--quality','showcase','--json']);fs.writeFileSync(args[3],originalHtml);
    return{exitCode:0,receipt:{...f.summary.stages[1].receipt,artifact:bound(readSnapshot(args[3])),receiptId:'22222222-2222-4222-8222-222222222222'}};
  }});
  assert.equal(entry.state.automatedStatus,'pass');assert.equal(entry.state.deliveryReceiptId,f.summary.deliveryReceiptId);assert.deepEqual(entry.state.browserReceiptIdentity,f.summary.browserReceiptIdentity);assert.equal(entry.determinism.status,'pass');assert.equal(entry.ledgerRecovery.status,'pass');assert.equal(entry.error,undefined);assert.deepEqual(fs.readFileSync(f.summaryFile.path),originalSummary);assert.deepEqual(fs.readFileSync(f.context.artifact.path),originalHtml);assert.deepEqual(calls.map(row=>row[0]),['check','deliver','check']);
  const replay=entry.determinism.artifact;calls.length=0;
  await resumeCaseEvidence(index,{}, {invoke:async(_engine,args)=>{calls.push(args);return{exitCode:0,receipt:f.summary.stages[2].receipt};}});
  assert.deepEqual(calls.map(row=>row[0]),['check']);assert.deepEqual(entry.determinism.artifact,replay);
});

test('resume cannot promote a forged outcome, changed original summary, replaced process or stale strict check',async t=>{
  const {resumeCaseEvidence}=await import('../../scripts/test-phoenix-diagram-runtime.mjs'),{readSnapshot,bound}=await import(modulePath);
  for(const fault of ['native-failure','stage-failure','summary-successor','process-successor','candidate-changed','artifact-changed','browser-changed','screenshot-changed','engine-changed','strict-config']){
    const {f,index,entry}=await failedLedgerFixture(t);let calls=0;
    if(fault==='native-failure' || fault==='stage-failure'){
      const captured=JSON.parse(fs.readFileSync(entry.process.path));if(fault==='native-failure')captured.exitCode=1;else {const receipt=JSON.parse(captured.stdout);receipt.stages[0].status='failed';captured.stdout=JSON.stringify(receipt);}fs.writeFileSync(entry.process.path,JSON.stringify(captured));entry.process=bound(readSnapshot(entry.process.path));
    }else if(fault==='summary-successor'){fs.writeFileSync(f.summaryFile.path,JSON.stringify({...JSON.parse(fs.readFileSync(f.summaryFile.path)),deliveryReceiptId:'33333333-3333-4333-8333-333333333333'}));}
    else if(fault==='process-successor'){const replacement=entry.process.path+'.replacement';fs.writeFileSync(replacement,fs.readFileSync(entry.process.path));fs.renameSync(replacement,entry.process.path);}
    else if(fault==='candidate-changed')fs.writeFileSync(entry.candidate.path,'changed candidate');
    else if(fault==='artifact-changed')fs.writeFileSync(entry.artifactPath,'changed HTML');
    else if(fault==='browser-changed')fs.writeFileSync(f.browserFile.path,'changed browser evidence');
    else if(fault==='screenshot-changed')fs.writeFileSync(f.screenshots[0].path,'changed screenshot');
    else if(fault==='engine-changed')index.engine.engineIdentity={...f.context.engine,sourceInputSha256:'f'.repeat(64)};
    await assert.rejects(()=>resumeCaseEvidence(index,{}, {invoke:async()=>{calls++;return{exitCode:0,receipt:{...f.summary.stages[2].receipt,configSha256:'d'.repeat(64)}};}}),/native|stage|gate|summary|binding|identity|strict|configuration/i);
    assert.equal(calls,fault==='strict-config'?1:0);assert.notEqual(entry.ledgerRecovery?.status,'pass');assert.notEqual(entry.state.automatedStatus,'pass');assert.equal(entry.determinism,undefined);
  }
});

test('a complete automatic fixture remains pending review and never becomes V9 acceptance',async t=>{
  const {inspectFinalize}=await import(modulePath);const f=await finalizeFixture(t);const state=inspectFinalize(f.summary,f.context);
  assert.equal(state.automatedStatus,'pass');assert.equal(state.visualReview,'required-pending');assert.equal(state.deliveryStatus,'incomplete');assert.equal(state.v9Complete,false);
});

test('finalize evidence rejects basic checks, missing stages, identity drift and omitted screenshot scope',async t=>{
  const {inspectFinalize}=await import(modulePath);const f=await finalizeFixture(t);
  for(const mutate of [s=>s.stages.pop(),s=>s.stages[0].receipt.checks.splice(1),s=>s.stages[2].receipt.scope='artifact-only',s=>s.stages[3].receipt.coverage.pop(),s=>s.stages[3].receipt.screenshots[1]=s.stages[3].receipt.screenshots[0]]){
    const modified=structuredClone(f.summary);mutate(modified);assert.throws(()=>inspectFinalize(modified,f.context),/stage|scope|check|screenshot|browser/i);
  }
  fs.writeFileSync(f.context.artifact.path,'later claimant');assert.throws(()=>inspectFinalize(f.summary,f.context),/artifact|binding|identity/i);
});

test('review continuation requires original receipt identity and current complete summary',async t=>{
  const {verifyReviewTransition,identity}=await import(modulePath);const f=await finalizeFixture(t);
  const record=path.join(path.dirname(f.summaryFile.path),'review.json');fs.writeFileSync(record,'unit fixture record');
  const reply={schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.1.0',command:'review-record',status:'pass',diagnostics:[],automatedStatus:'pass',deliveryReceiptId:f.summary.deliveryReceiptId,browserReceiptIdentity:f.summary.browserReceiptIdentity,visualReview:{status:'passed',record:{path:record,identity:identity(fs.readFileSync(record))}},deliveryStatus:'complete',evidence:{}};
  const stored=structuredClone(f.summary);delete stored.evidence.summaryReceipt;stored.visualReview=reply.visualReview;stored.deliveryStatus='complete';fs.writeFileSync(f.summaryFile.path,JSON.stringify(stored));reply.evidence.summaryReceipt={path:f.summaryFile.path,identity:identity(fs.readFileSync(f.summaryFile.path))};
  assert.equal(verifyReviewTransition(f.summary,reply).deliveryStatus,'complete');
  const different=structuredClone(reply);different.deliveryReceiptId='22222222-2222-4222-8222-222222222222';assert.throws(()=>verifyReviewTransition(f.summary,different),/identity|delivery|receipt/i);
  const incomplete=structuredClone(reply);incomplete.visualReview.status='failed';incomplete.deliveryStatus='incomplete';assert.throws(()=>verifyReviewTransition(f.summary,incomplete),/summary/i);
  stored.visualReview=incomplete.visualReview;stored.deliveryStatus='incomplete';fs.writeFileSync(f.summaryFile.path,JSON.stringify(stored));incomplete.evidence.summaryReceipt.identity=identity(fs.readFileSync(f.summaryFile.path));assert.equal(verifyReviewTransition(f.summary,incomplete).deliveryStatus,'incomplete');
});

test('recovery discovery uses durable parent records, including quarantined witnesses, and ignores terminal hints',async t=>{
  const {locateRecovery}=await import(modulePath);const directory=temp(t),target=path.join(directory,'diagram.html'),id='a'.repeat(32),name=`.phoenix-diagram-recovery-${id}`;
  const record={schemaVersion:1,tool:'phoenix-diagram',kind:'output-publication',transactionId:id,target:{commitPath:target},recovery:{name}};
  fs.mkdirSync(path.join(directory,name));fs.writeFileSync(path.join(directory,name,'publication-recovery-v1.json'),JSON.stringify(record));
  assert.equal(locateRecovery(directory,target).directory,path.join(directory,name));
  fs.rmSync(path.join(directory,name),{recursive:true});fs.mkdirSync(path.join(directory,'.phoenix-diagram-output-cleanup'));fs.writeFileSync(path.join(directory,'.phoenix-diagram-output-cleanup',`${id}.json`),JSON.stringify(record));
  assert.equal(locateRecovery(directory,target).directory,path.join(directory,name));
  fs.writeFileSync(path.join(directory,'.phoenix-diagram-output-cleanup',`${'b'.repeat(32)}.json`),JSON.stringify({...record,transactionId:'b'.repeat(32),recovery:{name:`.phoenix-diagram-recovery-${'b'.repeat(32)}`}}));
  assert.throws(()=>locateRecovery(directory,target),/multiple|explicit|ambiguous/i);
});

test('XSS content checks reject executable interpolation and preserve literal replacement tokens',async()=>{
  const {assertXssArtifact,XSS_LITERAL}=await import(modulePath);
  const json=JSON.stringify({meta:{subtitle:XSS_LITERAL}}).replace(/</g,'\\u003c').replace(/>/g,'\\u003e').replace(/&/g,'\\u0026');
  const safe=`<script id="phoenix-diagram-metadata" type="application/json">${json}</script><p>$&amp; {{PHOENIX:SVG}}</p>`;
  assert.equal(assertXssArtifact(Buffer.from(safe),XSS_LITERAL).status,'pass');
  assert.throws(()=>assertXssArtifact(Buffer.from(safe+'<script>globalThis.__phoenixDiagramXss=1</script>'),XSS_LITERAL),/execut|script|XSS/i);
});

test('review orchestration invokes only review-record and keeps original delivery/browser identity',async t=>{
  const {inspectFinalize,identity}=await import(modulePath);const {resumeReviews}=await import('../../scripts/test-phoenix-diagram-runtime.mjs');
  const f=await finalizeFixture(t),directory=path.dirname(f.summaryFile.path),record=path.join(directory,'actually-reviewed-record.json');fs.writeFileSync(record,'unit scope fixture; not actual image review');
  const mapping=path.join(directory,'reviews.json');fs.writeFileSync(mapping,JSON.stringify({records:{'new-architecture':record}}));
  const index={engine:{},cases:[{id:'new-architecture',candidate:f.context.candidate,finalize:f.summaryFile,state:inspectFinalize(f.summary,f.context)}]};const calls=[];
  await resumeReviews(index,{reviewFile:mapping},{invoke:async(_engine,args)=>{
    calls.push(args);assert.equal(args[0],'review-record');
    const stored=JSON.parse(fs.readFileSync(f.summaryFile.path));stored.visualReview={status:'passed',record:{path:record,identity:identity(fs.readFileSync(record))}};stored.deliveryStatus='complete';fs.writeFileSync(f.summaryFile.path,JSON.stringify(stored));
    return{exitCode:0,receipt:{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.1.0',command:'review-record',status:'pass',diagnostics:[],automatedStatus:'pass',visualReview:stored.visualReview,deliveryStatus:'complete',deliveryReceiptId:stored.deliveryReceiptId,browserReceiptIdentity:stored.browserReceiptIdentity,evidence:{summaryReceipt:{path:f.summaryFile.path,identity:identity(fs.readFileSync(f.summaryFile.path))}}}};
  }});
  assert.equal(calls.length,1);assert.equal(index.cases[0].state.deliveryStatus,'complete');assert.equal(index.cases[0].state.deliveryReceiptId,f.summary.deliveryReceiptId);assert.deepEqual(index.cases[0].state.browserReceiptIdentity,f.summary.browserReceiptIdentity);
  fs.writeFileSync(f.context.candidate.path,'changed candidate');
  await assert.rejects(()=>resumeReviews(index,{reviewFile:mapping},{invoke:async()=>{throw new Error('Stale candidate must stop before another command');}}),/candidate.*(?:identity|binding)|stale/i);
});

test('controller rollup cannot ignore a failed requested reader or review phase',async()=>{
  const {recomputeIndex}=await import('../../scripts/test-phoenix-diagram-runtime.mjs');
  const index={coverage:{complete:true},cases:[{state:{automatedStatus:'pass',visualReview:'required-pending',deliveryStatus:'incomplete'},determinism:{status:'pass'},reader:{status:'not-run'}}],content:{status:'not-run'},recovery:{status:'not-run'},offline:{status:'not-run'},reader:{status:'failed',diagnostics:[{code:'unit/reader-failure'}]}};
  assert.equal(recomputeIndex(index).status,'failed');index.reader={status:'not-run'};index.review={status:'failed',diagnostics:[{code:'unit/review-failure'}]};assert.equal(recomputeIndex(index).status,'failed');
  index.review={status:'not-run'};assert.equal(recomputeIndex(index).status,'incomplete');assert.equal(recomputeIndex(index).handoff,'incomplete');
});

test('maintenance markers bind the owned child and phase without authorizing a recovery path',async t=>{
  const {waitForCheckpoint}=await import('../../scripts/test-phoenix-diagram-recovery.mjs');
  const directory=temp(t),file=path.join(directory,'checkpoint.json'),target=path.join(directory,'diagram.html');
  fs.writeFileSync(file,JSON.stringify({schemaVersion:1,pid:12345,phase:'after-retire',target,recoveryDirectory:'/untrusted/hint'}));
  const child={pid:12345,exitCode:null,signalCode:null};
  const marker=await waitForCheckpoint(child,file,{phase:'after-retire',target,timeoutMs:50});
  assert.deepEqual(marker,{pid:12345,phase:'after-retire',target});assert.equal(marker.recoveryDirectory,undefined);
  await assert.rejects(()=>waitForCheckpoint({...child,pid:54321},file,{phase:'after-retire',target,timeoutMs:50}),/owned|pid|child/i);
  await assert.rejects(()=>waitForCheckpoint(child,file,{phase:'after-recovery-link',target,timeoutMs:50}),/phase|checkpoint/i);
  fs.rmSync(file);await assert.rejects(()=>waitForCheckpoint({...child,exitCode:0},file,{phase:'after-retire',target,timeoutMs:50}),/exit|checkpoint/i);
});

test('recovery outcomes require public bytes, truthful state and no invented delivery evidence',async t=>{
  const {verifyRecoveryOutcome}=await import('../../scripts/test-phoenix-diagram-recovery.mjs');
  const {identity}=await import(modulePath);const directory=temp(t),target=path.join(directory,'diagram.html');fs.writeFileSync(target,'old bytes');
  const restored={exitCode:0,receipt:{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.1.0',command:'recover-output',status:'pass',recoveryStatus:'recovered',target,publicState:{kind:'file',identity:identity(Buffer.from('old bytes'))},preserved:[],diagnostics:[]}};
  assert.equal(verifyRecoveryOutcome(restored,{mode:'recovered',target,expected:identity(Buffer.from('old bytes'))}).status,'pass');
  fs.writeFileSync(target,'later claimant');assert.throws(()=>verifyRecoveryOutcome(restored,{mode:'recovered',target,expected:identity(Buffer.from('old bytes'))}),/bytes|identity|public/i);
  const refused={...restored,exitCode:3,receipt:{...restored.receipt,status:'failed',recoveryStatus:'preserved',publicState:{kind:'file',identity:identity(Buffer.from('later claimant'))},preserved:['retained recovery']}};
  assert.equal(verifyRecoveryOutcome(refused,{mode:'preserved',target,expected:identity(Buffer.from('later claimant'))}).status,'pass');
  assert.throws(()=>verifyRecoveryOutcome({...refused,receipt:{...refused.receipt,deliveryStatus:'complete'}},{mode:'preserved',target,expected:identity(Buffer.from('later claimant'))}),/delivery|evidence/i);
});

test('summary failure rehearsal restores its owned parent and retains the exact record for retry',async t=>{
  const {rehearseSummaryFailure}=await import('../../scripts/test-phoenix-diagram-runtime.mjs');
  const {inspectFinalize,identity}=await import(modulePath);const f=await finalizeFixture(t),directory=path.dirname(f.summaryFile.path);
  fs.chmodSync(directory,0o700);const evidence=path.join(directory,'browser');fs.mkdirSync(evidence,{mode:0o700});
  const record=path.join(directory,'review-input.json'),sealed=path.join(evidence,'diagram.visual-review.json');fs.writeFileSync(record,'unit scope record');
  const stored=JSON.parse(fs.readFileSync(f.summaryFile.path));stored.evidence.reviewPath=sealed;fs.writeFileSync(f.summaryFile.path,JSON.stringify(stored));
  const entry={root:directory,finalize:{path:f.summaryFile.path,identity:identity(fs.readFileSync(f.summaryFile.path))},state:inspectFinalize({...stored,evidence:{...stored.evidence,summaryReceipt:{path:f.summaryFile.path,identity:identity(fs.readFileSync(f.summaryFile.path))}}},f.context)};
  let calls=0;
  const result=await rehearseSummaryFailure({engine:{}},entry,{path:record,identity:identity(fs.readFileSync(record))},{},{invoke:async(_engine,args)=>{
    calls++;assert.equal(args[0],'review-record');assert.equal(fs.statSync(directory).mode&0o777,0o500);fs.writeFileSync(sealed,fs.readFileSync(record));
    return{exitCode:4,receipt:{status:'failed',automatedStatus:'pass',visualReview:{status:'passed',record:{path:sealed,identity:identity(fs.readFileSync(sealed))}},evidence:{visualReviewRecord:{path:sealed,identity:identity(fs.readFileSync(sealed))}},diagnostics:[{code:'output/private-directory',message:'Permission denied'}]}};
  }});
  assert.equal(calls,1);assert.equal(result.status,'pass');assert.equal(fs.statSync(directory).mode&0o777,0o700);assert.equal(result.record.path,sealed);assert.equal(result.summaryUnchanged,true);
  fs.rmSync(sealed);
  await assert.rejects(()=>rehearseSummaryFailure({engine:{}},entry,{path:record,identity:identity(fs.readFileSync(record))},{},{invoke:async()=>{throw new Error('unit injected early failure');}}),/early failure/);
  assert.equal(fs.statSync(directory).mode&0o777,0o700);
});

test('reader evidence cannot pass with omitted export or clipboard scope',async()=>{
  const {verifyReaderReport}=await import('../../scripts/test-phoenix-diagram-runtime.mjs');
  const artifact={path:'/unit/diagram.html',identity:{sha256:'a'.repeat(64),bytes:10}};
  assert.throws(()=>verifyReaderReport({schemaVersion:1,kind:'real-browser-export',status:'pass',artifact:{path:artifact.path,...artifact.identity},actualBrowserVersion:'Chrome/1',exports:[],clipboard:{},resources:{status:'pass',requests:[],loadingFailed:[]},stateRestoration:{status:'pass'},errors:[]},artifact),/export scope/i);
});

test('reader evidence requires all four print observations after otherwise complete scopes',async t=>{
  const {verifyReaderReport}=await import('../../scripts/test-phoenix-diagram-runtime.mjs');
  const {identity}=await import(modulePath),directory=temp(t),file=path.join(directory,'identity-only.txt');
  // Protocol fixture only; these bytes never stand in for a real browser export.
  fs.writeFileSync(file,'unit identity fixture');const reference={path:file,identity:identity(fs.readFileSync(file))};
  const artifact={path:'/unit/diagram.html',identity:{sha256:'a'.repeat(64),bytes:10}};
  const report={schemaVersion:1,kind:'real-browser-export',status:'pass',artifact:{path:artifact.path,...artifact.identity},actualBrowserVersion:'Chrome/fixture',
    resources:{status:'pass',requests:[],loadingFailed:[]},errors:[],stateRestoration:{status:'pass'},
    exports:['light','dark'].flatMap(theme=>['still','live'].flatMap(mode=>['svg','png','jpeg','webp','webm'].map(format=>({...reference,theme,mode,format,status:'pass'})))),
    clipboard:{success:{status:'pass',path:file,...reference.identity},permissionDenied:{status:'pass'},capabilityUnavailable:{status:'pass',fallback:reference}}};
  for(const print of [undefined,{status:'not-run'}, {status:'pass',method:'cdp-print-media-and-native-print-to-pdf',observations:[]}]){
    report.stateRestoration.print=print;assert.throws(()=>verifyReaderReport(report,artifact),/print scope/i);
  }
});

test('reader evidence rejects legacy reports and status-only claims without actual UI interaction scope',async t=>{
  const {verifyReaderReport}=await import('../../scripts/test-phoenix-diagram-runtime.mjs'),{identity}=await import(modulePath),directory=temp(t),file=path.join(directory,'inert-protocol.pdf');
  // Inert protocol bytes reach the interaction gate; they never prove exports.
  fs.writeFileSync(file,'%PDF-1.7\n1 0 obj <</Type /Page>> endobj\n%%EOF\n');const reference={path:file,identity:identity(fs.readFileSync(file))};
  const artifact={path:'/unit/diagram.html',identity:{sha256:'a'.repeat(64),bytes:10}},observations=['dark','light'].flatMap(theme=>['still','live'].map(mode=>({theme,mode,status:'pass',motionApplicable:mode==='live',before:{mode},during:{mode},after:{mode},motionBefore:{printMedia:false,running:mode==='live'?2:0},motionDuring:{printMedia:true,running:0,staticCount:1,hiddenStatic:0},motionAfter:{printMedia:false,running:mode==='live'?2:0},nativePrint:{events:[{type:'beforeprint'},{type:'afterprint'}],pdf:{...reference,signature:'PDF',eof:true,pages:1}}})));
  const report={schemaVersion:1,kind:'real-browser-export',status:'pass',artifact:{path:artifact.path,...artifact.identity},actualBrowserVersion:'Chrome/fixture',resources:{status:'pass',requests:[],loadingFailed:[]},errors:[],stateRestoration:{status:'pass',print:{status:'pass',method:'cdp-print-media-and-native-print-to-pdf',observations}},exports:['light','dark'].flatMap(theme=>['still','live'].flatMap(mode=>['svg','png','jpeg','webp','webm'].map(format=>({...reference,theme,mode,format,status:'pass'})))),clipboard:{success:{status:'pass',path:file,...reference.identity},permissionDenied:{status:'pass'},capabilityUnavailable:{status:'pass',fallback:reference}}};
  for(const readerInteractions of [undefined,{status:'not-run',observations:[]},{status:'pass',observations:['camera','search-empty','search-label','search-id','search-no-match','details','keyboard','style'].map(name=>({name,status:'pass',before:{},after:{},result:{}}))}]){
    report.readerInteractions=readerInteractions;assert.throws(()=>verifyReaderReport(report,artifact),/Reader interaction/i);
  }
});

test('binding rejection rehearsal restores only its own mutation and preserves a later claimant',async t=>{
  const {withMutatedFile}=await import('../../scripts/test-phoenix-diagram-runtime.mjs');const {readSnapshot,bound}=await import(modulePath);
  const directory=temp(t),file=path.join(directory,'candidate.json');fs.writeFileSync(file,'original');const original=bound(readSnapshot(file));
  await assert.rejects(()=>withMutatedFile(original,Buffer.from('known mutation'),async()=>{assert.equal(fs.readFileSync(file,'utf8'),'known mutation');throw new Error('unit command rejection');}),/unit command rejection/);
  assert.equal(fs.readFileSync(file,'utf8'),'original');
  await assert.rejects(()=>withMutatedFile(original,Buffer.from('known mutation'),async()=>{fs.writeFileSync(file,'a later user edit');}),/claimant|changed|mutation/i);
  assert.equal(fs.readFileSync(file,'utf8'),'a later user edit');
});
