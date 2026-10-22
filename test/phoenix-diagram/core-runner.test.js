'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const os=require('node:os');
const modulePath='../../scripts/test-phoenix-diagram-core.mjs';

function temp(t) {const directory=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'phoenix-core-controller-unit-')));t.after(()=>fs.rmSync(directory,{recursive:true,force:true}));return directory;}

async function failedOwnedNode(t) {
  // A real owned Node process exercises maintenance capture, not native acceptance.
  const {readSnapshot,bound,CoreError,root}=await import(modulePath),{invokeNative}=await import('../../scripts/test-phoenix-diagram-runtime.mjs');
  const directory=temp(t);fs.chmodSync(directory,0o700);const program=path.join(directory,'node');fs.copyFileSync(fs.realpathSync(process.execPath),program,fs.constants.COPYFILE_FICLONE);fs.chmodSync(program,0o700);
  const engine=bound(readSnapshot(program)),ready=path.join(directory,'ready'),stdout=Buffer.from('756e6974207374646f757400ff0a','hex'),stderr=Buffer.from('756e69742073746465727200fe0a','hex');
  const args=['-e',`const fs=require('node:fs');fs.writeSync(1,Buffer.from('${stdout.toString('hex')}','hex'));fs.writeSync(2,Buffer.from('${stderr.toString('hex')}','hex'));fs.writeFileSync(process.argv[1],'ready');setInterval(()=>{},1000);`,ready];
  let child,closed,error;
  try {
    await invokeNative(engine,args,{json:false,timeoutMs:5000,onSpawn:async owned=>{
      child=owned;closed=new Promise(resolve=>child.once('close',resolve));const deadline=Date.now()+3000;
      while(!fs.existsSync(ready)){assert(Date.now()<deadline,'Node did not publish its real stdout/stderr barrier');assert.equal(child.exitCode,null);assert.equal(child.signalCode,null);await new Promise(resolve=>setTimeout(resolve,5));}
      throw new CoreError('core/checkpoint-timeout','Maintenance unit injected checkpoint rejection',{exitCode:4});
    }});
  }catch(cause){error=cause;}finally{
    if(child && child.exitCode===null && child.signalCode===null)try{process.kill(-child.pid,'SIGKILL');}catch(cause){if(cause.code!=='ESRCH')throw cause;}
    if(closed)await closed;
  }
  assert(error);assert.equal(error.code,'core/checkpoint-timeout');assert.equal(child.signalCode,'SIGKILL');assert.throws(()=>process.kill(child.pid,0),{code:'ESRCH'});
  return{error,engine,program,args,cwd:root,pid:child.pid,stdout,stderr};
}

test('invokeNative preserves real process evidence when onSpawn rejects after stdout and stderr',async t=>{
  const f=await failedOwnedNode(t),result=f.error.processResult;assert(result,'Failed process evidence must survive rejection');
  assert.equal(result.program,f.program);assert.deepEqual(result.argv,f.args);assert.equal(result.cwd,f.cwd);assert.equal(result.pid,f.pid);assert.equal(result.exitCode,null);assert.equal(result.signal,'SIGKILL');
  assert(Number.isFinite(Date.parse(result.startedAt)));assert(Date.parse(result.closedAt)>=Date.parse(result.startedAt));assert(Number.isFinite(result.elapsedMs)&&result.elapsedMs>=0);
  assert.deepEqual(result.stdout,f.stdout);assert.deepEqual(result.stderr,f.stderr);assert.equal(Object.hasOwn(result,'env'),false);assert.equal(Object.keys(f.error).includes('processResult'),false);
  const {invokeNative}=await import('../../scripts/test-phoenix-diagram-runtime.mjs'),success=await invokeNative(f.engine,['-e','process.stdout.write("{}")']);
  assert.deepEqual(Object.keys(success).sort(),['exitCode','signal','stdout','stderr','receipt'].sort());assert.equal(success.exitCode,0);assert.equal(success.signal,null);assert.deepEqual(success.receipt,{});
});

test('failed recovery scenarios retain original process bytes as owned file references',async t=>{
  const f=await failedOwnedNode(t),{runRecoveryScenarios}=await import('../../scripts/test-phoenix-diagram-recovery.mjs'),{readSnapshot,readBound,bound}=await import(modulePath),runRoot=temp(t);fs.chmodSync(runRoot,0o700);
  const candidate=path.join(runRoot,'candidate.json');fs.writeFileSync(candidate,'{}');
  // Inject the captured maintenance failure; no native recovery acceptance runs.
  const report=await runRecoveryScenarios({runRoot,engine:f.engine,cases:[{type:'architecture',contract:'phoenix-v1',candidate:bound(readSnapshot(candidate))}]},{},{invoke:async()=>{throw f.error;}});
  assert.equal(report.status,'failed');assert.equal(report.scenarios.length,8);
  for(const scenario of report.scenarios){
    assert.equal(scenario.status,'failed');assert.equal(scenario.exitCode,4);assert.equal(scenario.diagnostics[0].code,'core/checkpoint-timeout');assert(scenario.process,'Failed scenario must point to persisted process evidence');
    const processRecord=JSON.parse(readBound(scenario.process,'unit failed process record').content);
    assert.equal(processRecord.program,f.program);assert.deepEqual(processRecord.argv,f.args);assert.equal(processRecord.cwd,f.cwd);assert.equal(processRecord.pid,f.pid);assert.equal(processRecord.exitCode,null);assert.equal(processRecord.signal,'SIGKILL');assert.equal(processRecord.elapsedMs,f.error.processResult.elapsedMs);
    assert.deepEqual(readBound(processRecord.stdout,'unit raw stdout').content,f.stdout);assert.deepEqual(readBound(processRecord.stderr,'unit raw stderr').content,f.stderr);
    for(const reference of [scenario.process,processRecord.stdout,processRecord.stderr]){const metadata=fs.lstatSync(reference.path);assert.equal(path.dirname(reference.path),scenario.root);assert(metadata.isFile()&&!metadata.isSymbolicLink());assert.equal(metadata.nlink,1);assert.equal(metadata.mode&0o7777,0o600);assert.equal(metadata.uid,process.getuid());}
    assert.equal(fs.lstatSync(scenario.root).mode&0o7777,0o700);
  }
  assert(!JSON.stringify(report).includes(f.stdout.toString('utf8')));assert(!JSON.stringify(report).includes(f.stderr.toString('utf8')));
  for(const fault of ['claimed-file','nonprivate-directory']){
    const refused=await runRecoveryScenarios({runRoot,engine:f.engine,cases:[{type:'architecture',contract:'phoenix-v1',candidate:bound(readSnapshot(candidate))}]},{},{invoke:async(_engine,_args,settings)=>{
      const directory=path.dirname(settings.env.PHOENIX_DIAGRAM_CHECKPOINT_DIR);
      if(fault==='claimed-file')fs.writeFileSync(path.join(directory,'failed-process.stdout'),'later claimant');else fs.chmodSync(directory,0o755);
      throw f.error;
    }});
    assert.equal(refused.status,'failed');
    for(const scenario of refused.scenarios){assert.equal(scenario.diagnostics[0].code,'core/checkpoint-timeout');assert.equal(scenario.diagnostics.length,2);assert.equal(scenario.process,undefined);assert(!fs.existsSync(path.join(scenario.root,'failed-process.json')));if(fault==='claimed-file')assert.equal(fs.readFileSync(path.join(scenario.root,'failed-process.stdout'),'utf8'),'later claimant');else assert(!fs.existsSync(path.join(scenario.root,'failed-process.stdout')));}
  }
});

test('public npm recovery observes a delayed owned marker without inventing native recovery success',{timeout:25000},async t=>{
  const {spawn}=require('node:child_process'),{runRecoveryScenarios}=await import('../../scripts/test-phoenix-diagram-recovery.mjs'),{readSnapshot,bound,CoreError}=await import(modulePath);
  const runRoot=temp(t);fs.chmodSync(runRoot,0o700);const candidate=path.join(runRoot,'candidate.json');fs.writeFileSync(candidate,'{}');
  let calls=0,child,markerObserved=false,elapsedMs,closedResult,markerFile;
  // Inert Node process and injected adapter only; public npm selects the wait
  // branch, but no npm/native command or real recovery journal is supplied.
  const report=await runRecoveryScenarios({runRoot,engine:{},cases:[{type:'architecture',contract:'phoenix-v1',candidate:bound(readSnapshot(candidate))}]},{publicNpm:{}},{invoke:async(_engine,args,settings)=>{
    if(++calls!==1)throw new CoreError('unit/no-native-acceptance','No native recovery command runs in this maintenance fixture');
    markerFile=path.join(settings.env.PHOENIX_DIAGRAM_CHECKPOINT_DIR,'checkpoint.json');const phase=settings.env.PHOENIX_DIAGRAM_CHECKPOINT_PHASE,target=args[3];
    const script="const fs=require('node:fs');const [marker,target,phase]=process.argv.slice(1);setTimeout(()=>{fs.unlinkSync(target);fs.writeFileSync(marker,JSON.stringify({schemaVersion:1,pid:process.pid,phase,target}),{flag:'wx',mode:0o600});},13000);setTimeout(()=>process.exit(2),18000);";
    const started=process.hrtime.bigint();child=spawn(fs.realpathSync(process.execPath),['-e',script,markerFile,target,phase],{cwd:runRoot,stdio:'ignore',detached:true});
    let spawnError;child.once('error',error=>{spawnError=error;});const closed=new Promise(resolve=>child.once('close',(exitCode,signal)=>resolve({exitCode,signal})));
    try{await settings.onSpawn(child);markerObserved=true;}finally{
      if(child.pid && child.exitCode===null && child.signalCode===null)try{process.kill(-child.pid,'SIGKILL');}catch(error){if(error.code!=='ESRCH')throw error;}
      closedResult=await closed;elapsedMs=Number(process.hrtime.bigint()-started)/1e6;
    }
    if(spawnError)throw spawnError;return{...closedResult,stdout:'',stderr:''};
  }});
  assert.equal(markerObserved,true,JSON.stringify(report.scenarios[0].diagnostics));assert(elapsedMs>=13000);assert.equal(closedResult.exitCode,null);assert.equal(closedResult.signal,'SIGKILL');assert.throws(()=>process.kill(child.pid,0),{code:'ESRCH'});
  assert.equal(JSON.parse(fs.readFileSync(markerFile,'utf8')).pid,child.pid);assert.equal(calls,8);assert.equal(report.status,'failed');assert(report.scenarios.every(scenario=>scenario.status==='failed'));
  assert.match(report.scenarios[0].diagnostics[0].message,/No durable recovery record/);assert(report.scenarios.slice(1).every(scenario=>scenario.diagnostics[0].code==='unit/no-native-acceptance'));
  t.diagnostic(`Maintenance Node marker observed after ${elapsedMs.toFixed(2)}ms; owned child reaped with SIGKILL; no native acceptance.`);
});

test('core options separate initial execution, review-only resume and unimplemented stages',async()=>{
  const {parseOptions}=await import(modulePath);
  const initial=parseOptions(['--stage','core','--engine','/native/phoenix-diagram','--output-dir','/tmp/core']);
  assert.equal(initial.stage,'core');assert.equal(initial.engine,'/native/phoenix-diagram');assert.equal(initial.reader,false);assert.equal(initial.recovery,false);
  const resume=parseOptions(['--stage','core','--engine','/native/phoenix-diagram','--resume','/tmp/core/index.json','--review-file','/tmp/reviews.json']);
  assert.equal(resume.resume,'/tmp/core/index.json');assert.equal(resume.reviewFile,'/tmp/reviews.json');
  assert.throws(()=>parseOptions(['--stage','install','--engine','/native/phoenix-diagram']),/not implemented/i);
  assert.throws(()=>parseOptions(['--stage','core','--engine','/native/phoenix-diagram','--review-file','/tmp/reviews.json']),/resume/i);
  assert.throws(()=>parseOptions(['--stage','core','--engine','/native/phoenix-diagram','--engine','/other']),/duplicate/i);
  for(const scope of ['explore','corpus','evidence','locale']) {
    assert.equal(initial[scope],false,`--${scope} defaults to not requested`);
    assert.equal(parseOptions(['--stage','core','--engine','/native/phoenix-diagram','--resume','/tmp/core/index.json',`--${scope}`])[scope],true);
    assert.throws(()=>parseOptions(['--stage','core','--engine','/native/phoenix-diagram',`--${scope}=yes`]),/takes no value/);
  }
});

test('unrequested acceptance scopes stay not-run and keep the handoff incomplete',async()=>{
  const {recomputeIndex,SCOPES}=await import('../../scripts/test-phoenix-diagram-runtime.mjs');
  const passed={status:'pass'},state={automatedStatus:'pass',visualReview:'passed',deliveryStatus:'complete'};
  const index={coverage:{complete:true},cases:[{state,determinism:passed,reader:passed,review:{idempotent:passed}}],content:passed,recovery:passed,offline:passed,reviewBindings:passed,newDelivery:passed,reviewRecovery:{status:'pass',retryStatus:'pass'},...Object.fromEntries(SCOPES.map(name=>[name,{status:'not-run'}]))};
  assert.equal(recomputeIndex(index).handoff,'incomplete');
  assert.equal(recomputeIndex({...index,...Object.fromEntries(SCOPES.map(name=>[name,passed]))}).handoff,'passed');
  assert.equal(recomputeIndex({...index,...Object.fromEntries(SCOPES.map(name=>[name,passed])),corpus:{status:'failed'}}).status,'failed');
  // A package consumer fixes its required subset; the remaining scopes stay not-run.
  const consumer={...index,requiredScopes:['explore','evidence','locale']};
  assert.equal(recomputeIndex(consumer).handoff,'incomplete');
  assert.equal(recomputeIndex({...consumer,explore:passed,evidence:passed,locale:passed}).handoff,'passed');
  assert.equal(recomputeIndex({...consumer,explore:passed,evidence:passed,locale:{status:'failed'}}).handoff,'incomplete');
});

test('coverage requires the five legacy types in both contracts and the five new types in phoenix-v1 only',async()=>{
  const {requiredCoverage,TYPES,contractsFor}=await import(modulePath),{DEFAULT_CASES,SCOPES}=await import('../../scripts/test-phoenix-diagram-runtime.mjs');
  assert.deepEqual(TYPES,['architecture','workflow','sequence','dataflow','lifecycle','erd','class','tree','timeline','waterfall']);
  const cases=TYPES.flatMap(type=>contractsFor(type).map(contract=>({id:`${contract==='phoenix-v1'?'new':'legacy'}-${type}`,type,contract})));
  assert.equal(cases.length,15);assert.equal(requiredCoverage(cases).complete,true);
  assert.deepEqual(DEFAULT_CASES.map(entry=>entry.id),cases.map(entry=>entry.id));
  assert.ok(DEFAULT_CASES.every(entry=>entry.contract==='legacy-2.17'?entry.source.includes('upstream/legacy-2.17/examples'):entry.source.endsWith(`examples/minimal.${entry.type}.json`)));
  assert.throws(()=>requiredCoverage([...cases,{id:'legacy-erd',type:'erd',contract:'legacy-2.17'}]),/unsupported case type or contract/i);
  assert.deepEqual(requiredCoverage(cases.filter(entry=>entry.id!=='new-timeline')).missing,[{type:'timeline',contract:'phoenix-v1'}]);
  assert.deepEqual(SCOPES,['explore','corpus','evidence','locale']);
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
  const engine={sourceInputSha256:'a'.repeat(64),inputContractSchema:1,receiptSchema:1,viewerRevision:'phoenix-v2',binary:identity(Buffer.from('unit fixture'))};
  // The actual legacy native receipt executes these 20 scopes. These inert
  // files exercise the ledger protocol, never native/browser acceptance.
  const checks=ARTIFACT_CHECKS.filter(name=>contract!=='legacy-2.17' || !['arrowhead_separation','label_canvas_containment'].includes(name)).map(name=>({name,status:'pass',scope:{unitFixture:true}}));const composition={schemaVersion:1,profile:'showcase',status:'pass',summary:{errors:0,warnings:0}};
  const config={diagramType:'architecture',inputContract:contract,quality:'showcase',viewerRevision:'phoenix-v2'};
  const common={schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',status:'pass',diagnostics:[],candidate,engine,configSha256:'c'.repeat(64)};
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
  const reply={schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'review-record',status:'pass',diagnostics:[],automatedStatus:'pass',deliveryReceiptId:f.summary.deliveryReceiptId,browserReceiptIdentity:f.summary.browserReceiptIdentity,visualReview:{status:'passed',record:{path:record,identity:identity(fs.readFileSync(record))}},deliveryStatus:'complete',evidence:{}};
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
    return{exitCode:0,receipt:{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'review-record',status:'pass',diagnostics:[],automatedStatus:'pass',visualReview:stored.visualReview,deliveryStatus:'complete',deliveryReceiptId:stored.deliveryReceiptId,browserReceiptIdentity:stored.browserReceiptIdentity,evidence:{summaryReceipt:{path:f.summaryFile.path,identity:identity(fs.readFileSync(f.summaryFile.path))}}}};
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
  const restored={exitCode:0,receipt:{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'recover-output',status:'pass',recoveryStatus:'recovered',target,publicState:{kind:'file',identity:identity(Buffer.from('old bytes'))},preserved:[],diagnostics:[]}};
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

function motionScopeFixture() {
  return {status:'pass',observations:['dark','light'].flatMap(theme=>['still','live'].flatMap(mode=>['reduced-motion','page-hidden'].map(condition=>{
    const state=suppressed=>({reader:{mode:suppressed?'still':mode,owner:'',focus:null,view:null,viewBox:'0 0 500 300',preset:'classic',transform:'',clip:'',nodes:[{id:'a',label:'A',text:['A']}],edges:[],text:['A'],eligibleMotion:true},capable:true,preference:mode==='live'?'live':null,reducedMotion:suppressed && condition==='reduced-motion',hidden:suppressed && condition==='page-hidden',visibilityState:suppressed && condition==='page-hidden'?'hidden':'visible',printMedia:false,documentHidden:suppressed && condition==='page-hidden'?'true':null,button:{disabled:suppressed && condition==='reduced-motion',pressed:!suppressed && mode==='live'?'true':'false'},animationCount:2,running:!suppressed && mode==='live'?2:0,meaning:{total:1,visible:[0]}});
    return {theme,mode,condition,status:'pass',method:condition==='reduced-motion'?'cdp-prefers-reduced-motion':'cdp-owned-target-visibility',motionApplicable:mode==='live',before:state(false),during:state(true),after:state(false),inputs:[{type:'click',trusted:true,target:{id:'btn-motion'}}],ownedTargets:condition==='page-hidden'?{subject:'subject',cover:'cover',browserContext:'context'}:undefined,events:condition==='reduced-motion'?[{type:'reduced-motion',trusted:true,matches:true},{type:'reduced-motion',trusted:true,matches:false}]:[{type:'visibilitychange',trusted:true,hidden:true,visibilityState:'hidden'},{type:'visibilitychange',trusted:true,hidden:false,visibilityState:'visible'}]};
  })))};
}

test('reader evidence requires the native motion-suppression matrix before accepting an old report',async t=>{
  const {verifyReaderReport}=await import('../../scripts/test-phoenix-diagram-runtime.mjs'),{identity}=await import(modulePath),directory=temp(t),file=path.join(directory,'inert-motion-protocol.pdf');
  // Inert protocol bytes exercise only the gate, never real Reader acceptance.
  fs.writeFileSync(file,'%PDF-1.7\n1 0 obj <</Type /Page>> endobj\n%%EOF\n');const reference={path:file,identity:identity(fs.readFileSync(file))};
  const artifact={path:'/unit/diagram.html',identity:{sha256:'a'.repeat(64),bytes:10}},observations=['dark','light'].flatMap(theme=>['still','live'].map(mode=>({theme,mode,status:'pass',motionApplicable:mode==='live',before:{mode},during:{mode},after:{mode},motionBefore:{printMedia:false,running:mode==='live'?2:0},motionDuring:{printMedia:true,running:0,staticCount:1,hiddenStatic:0},motionAfter:{printMedia:false,running:mode==='live'?2:0},nativePrint:{events:[{type:'beforeprint'},{type:'afterprint'}],pdf:{...reference,signature:'PDF',eof:true,pages:1}}})));
  const report={schemaVersion:1,kind:'real-browser-export',status:'pass',artifact:{path:artifact.path,...artifact.identity},actualBrowserVersion:'Chrome/fixture',resources:{status:'pass',requests:[],loadingFailed:[]},errors:[],stateRestoration:{status:'pass',print:{status:'pass',method:'cdp-print-media-and-native-print-to-pdf',observations}},exports:['light','dark'].flatMap(theme=>['still','live'].flatMap(mode=>['svg','png','jpeg','webp','webm'].map(format=>({...reference,theme,mode,format,status:'pass'})))),clipboard:{success:{status:'pass',path:file,...reference.identity},permissionDenied:{status:'pass'},capabilityUnavailable:{status:'pass',fallback:reference}}};
  for(const motionSuppression of [undefined,{status:'not-run',observations:[]},{status:'pass',observations:[]}])assert.throws(()=>verifyReaderReport({...report,motionSuppression},artifact),/motion suppression/i);
  assert.throws(()=>verifyReaderReport({...report,motionSuppression:motionScopeFixture()},artifact),/Reader interaction/i,'the new scope must preserve the existing UI gate');
});

test('reader evidence rejects legacy reports and status-only claims without actual UI interaction scope',async t=>{
  const {verifyReaderReport}=await import('../../scripts/test-phoenix-diagram-runtime.mjs'),{identity}=await import(modulePath),directory=temp(t),file=path.join(directory,'inert-protocol.pdf');
  // Inert protocol bytes reach the interaction gate; they never prove exports.
  fs.writeFileSync(file,'%PDF-1.7\n1 0 obj <</Type /Page>> endobj\n%%EOF\n');const reference={path:file,identity:identity(fs.readFileSync(file))};
  const artifact={path:'/unit/diagram.html',identity:{sha256:'a'.repeat(64),bytes:10}},observations=['dark','light'].flatMap(theme=>['still','live'].map(mode=>({theme,mode,status:'pass',motionApplicable:mode==='live',before:{mode},during:{mode},after:{mode},motionBefore:{printMedia:false,running:mode==='live'?2:0},motionDuring:{printMedia:true,running:0,staticCount:1,hiddenStatic:0},motionAfter:{printMedia:false,running:mode==='live'?2:0},nativePrint:{events:[{type:'beforeprint'},{type:'afterprint'}],pdf:{...reference,signature:'PDF',eof:true,pages:1}}})));
  const report={schemaVersion:1,kind:'real-browser-export',status:'pass',artifact:{path:artifact.path,...artifact.identity},actualBrowserVersion:'Chrome/fixture',resources:{status:'pass',requests:[],loadingFailed:[]},errors:[],stateRestoration:{status:'pass',print:{status:'pass',method:'cdp-print-media-and-native-print-to-pdf',observations}},exports:['light','dark'].flatMap(theme=>['still','live'].flatMap(mode=>['svg','png','jpeg','webp','webm'].map(format=>({...reference,theme,mode,format,status:'pass'})))),clipboard:{success:{status:'pass',path:file,...reference.identity},permissionDenied:{status:'pass'},capabilityUnavailable:{status:'pass',fallback:reference}}};
  report.motionSuppression=motionScopeFixture();
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

test('the corpus browser-checks every 3.1.0 reference JSON, original five types included',{skip:!fs.existsSync(path.join(__dirname,'../../local/phoenix-diagram/reference/archify/examples'))?'Requires the gitignored verified reference unpack':false},async t=>{
  const {run}=await import('../../scripts/test-phoenix-diagram-scopes.mjs'),{LEGACY_TYPES}=await import(modulePath),runRoot=temp(t);
  const collections={architecture:['components','connections'],workflow:['nodes','edges'],sequence:['participants','messages'],dataflow:['nodes','flows'],lifecycle:['states','transitions'],erd:['entities','relationships'],class:['types','relationships'],tree:['nodes',null],timeline:['events',null],waterfall:['spans',null]};
  const attr=value=>String(value).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;').replaceAll("'",'&#39;'),checked=[];
  // A fake engine renders only what the corpus inspects: metadata nodes, one element per node and directed relations.
  const invoke=async(_engine,[command,...args])=>{
    if(command==='deliver'){
      const [type,candidate,output]=args,document=JSON.parse(fs.readFileSync(candidate,'utf8')),[nodes,relations]=collections[type],authored=document[nodes] || [];
      const edges=relations?document[relations] || []:type==='tree'?authored.filter(node=>node.parent!==undefined).map(node=>({from:node.parent,to:node.id})):[];
      fs.writeFileSync(output,`<script id="phoenix-diagram-metadata" type="application/json">${JSON.stringify({graph:{nodes:authored}})}</script>${authored.map(node=>`<g data-node-id="${attr(node.id)}"></g>`).join('')}${edges.map(edge=>`<g data-edge-from="${attr(edge.from)}" data-edge-to="${attr(edge.to)}"></g>`).join('')}`);
      return{exitCode:0,receipt:{status:'pass',inputContract:args.includes('--input-contract') || !LEGACY_TYPES.includes(type) || document.schema_version===3?'phoenix-v1':'legacy-2.17',compatibility:{coreCompatibility:'pass',deferredFields:[]}}};
    }
    if(command==='check')return{exitCode:0,receipt:{provenance:'current'}};
    const directory=args[args.indexOf('--out-dir')+1];fs.mkdirSync(directory);checked.push(args[0]);
    return{exitCode:0,receipt:{status:'pass',screenshots:['light','dark'].map(theme=>{const file=path.join(directory,`${theme}.png`);fs.writeFileSync(file,theme);return{path:file,identity:{sha256:theme,bytes:theme.length},theme,viewport:{width:1440,height:900}};})}};
  };
  // Fake details show what the given context function writes; the real Reader runs in the actual corpus scope.
  const details=[];
  const passportsWith=(context,reach={status:'pass',direction:'downstream',input:'touch'})=>async({artifactPath,evidenceDir,nodes,evidence})=>{
    const caseRoot=path.dirname(artifactPath),document=JSON.parse(fs.readFileSync(path.join(caseRoot,'candidate.json'),'utf8')),receiptPath=path.join(evidenceDir,'index.json');
    details.push({name:path.basename(caseRoot),evidence,nodes});fs.writeFileSync(receiptPath,'{}');
    return{status:'pass',receiptPath,passports:nodes.map(({id,theme,viewport})=>{
      const file=path.join(evidenceDir,`${id}-${theme}-${viewport.width}.png`),keys=document.entities.find(entity=>entity.id===id).attributes.filter(attribute=>attribute.references);fs.writeFileSync(file,id);
      return{id,theme,viewport,context:context(keys),reach,screenshot:{path:file,identity:{sha256:id,bytes:id.length}}};
    })};
  };
  const complete=keys=>'Entity › '+keys.map(key=>`${key.name} → ${key.references}`).join(', ');
  const report=await run('corpus',{runRoot,engine:{path:'/fake/engine'}},{},{invoke,passports:passportsWith(complete)});
  const references=report.cases.filter(entry=>entry.contract==='phoenix-v1');
  assert.equal(report.references,25);assert.equal(references.length,25);assert.equal(checked.length,25);
  assert.ok(LEGACY_TYPES.every(type=>references.some(entry=>entry.type===type && entry.screenshots?.length===2)),'each original type has actual-viewing screenshots');
  assert.ok(references.every(entry=>entry.screenshots?.length===2 && entry.visual));
  assert.ok(report.cases.filter(entry=>entry.contract==='legacy-2.17').every(entry=>entry.screenshots===undefined));
  // Only ERD tables with several foreign keys are opened, each at 1440x900 and 390x844, without requiring source evidence.
  assert.deepEqual(details.filter(entry=>entry.name!=='erd-long-foreign-keys').map(entry=>[entry.name,entry.evidence,entry.nodes.map(node=>`${node.id}@${node.viewport.width}x${node.viewport.height}`)]),[
    ['orders.erd',false,['order_item@1440x900','order_item@390x844']],
    ['subscription-billing.erd',false,['membership@1440x900','membership@390x844','subscription@1440x900','subscription@390x844','payment@1440x900','payment@390x844']],
  ]);
  assert.ok(references.filter(entry=>entry.type==='erd').every(entry=>entry.passports.screenshots.length>0 && entry.passports.screenshots.every(shot=>shot.viewing==='required')));
  // An ERD with three long foreign keys is delivered and its details opened at 390x844 in both themes; Reach must take a touch.
  const long=details.find(entry=>entry.name==='erd-long-foreign-keys'),candidate=JSON.parse(fs.readFileSync(report.narrowDetails.candidate.path,'utf8'));
  assert.deepEqual(long.nodes.map(node=>`${node.id}@${node.viewport.width}x${node.viewport.height}:${node.theme}`),['orders@390x844:light','orders@390x844:dark']);
  assert.equal(long.evidence,false);assert.equal(candidate.entities.find(entity=>entity.id==='orders').attributes.filter(attribute=>attribute.references).length,3);
  assert.deepEqual(report.narrowDetails.reach.map(reach=>[reach.theme,reach.status,reach.input]),[['light','pass','touch'],['dark','pass','touch']]);
  assert.ok(report.narrowDetails.screenshots.length===2 && report.narrowDetails.screenshots.every(shot=>shot.viewing==='required'));
  // Details that omit the last foreign key target, or whose Reach was not operated, fail the corpus.
  await assert.rejects(run('corpus',{runRoot:temp(t),engine:{path:'/fake/engine'}},{},{invoke,passports:passportsWith(keys=>complete(keys.slice(0,-1)))}),/omit a foreign key target of order_item/);
  await assert.rejects(run('corpus',{runRoot:temp(t),engine:{path:'/fake/engine'}},{},{invoke,passports:passportsWith(complete,{status:'not-applicable'})}),/Long foreign key ERD details leave Reach unusable by touch/);
});
