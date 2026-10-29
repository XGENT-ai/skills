import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import {isDeepStrictEqual as same} from 'node:util';
import {verifyReaderReport} from '../../../../scripts/test-phoenix-diagram-runtime.mjs';
import {bound,readSnapshot,readBound,identity,inspectFinalize,VIEWPORTS,BROWSER_CHECKS,readEvidenceBytes,publicNpmCommand} from '../../../../scripts/test-phoenix-diagram-core.mjs';

export const OFFICIAL_RESOURCE=Object.freeze({mediaType:'image/png',bytes:125,sha256:'8a1035d9172bf23b4444d8409418141e7bb39271deb17786c3860ef1defd494e'});
export const REVIEW_SCOPE=['diagram','labels','cards','relationship-occlusion','hierarchy','controls'];
export const CASE_IDS=['brand-builtin','brand-capture','comparison'];
const REQUIRED_STEPS=['brands-list','validate-brand-builtin','finalize-brand-builtin','brands-capture','validate-brand-capture','finalize-brand-capture','compare','compare-check','compare-visual'];
export const KIND='phoenix-diagram-p3-author-tools';
const HASH=/^[a-f0-9]{64}$/;
const THEMES=['light','dark'];

export function keys(value,names,label){assert(value && typeof value==='object' && !Array.isArray(value),`${label} must be an object`);assert.deepEqual(Object.keys(value).sort(),[...names].sort(),`${label} fields differ`);}
export function privateDirectory(directory,{existing=false}={}){
  if(!existing)fs.mkdirSync(directory,{mode:0o700});
  const stat=fs.lstatSync(directory);assert(stat.isDirectory()&&!stat.isSymbolicLink()&&(stat.mode&0o7777)===0o700&&stat.uid===process.getuid(),'Use an owned ordinary 0700 run directory');return fs.realpathSync(directory);
}
export function writeNew(file,value){fs.writeFileSync(file,Buffer.isBuffer(value)||typeof value==='string'?value:JSON.stringify(value,null,2)+'\n',{flag:'wx',mode:0o600});return bound(readSnapshot(file));}
export function jsonBound(reference,label){return JSON.parse(readBound(reference,label).content.toString('utf8'));}
export function readOfficialResource(file){
  const snapshot=readSnapshot(file),descriptor=JSON.parse(snapshot.content);
  keys(descriptor,['schemaVersion','sourceCommit','url','mediaType','bytes','sha256'],'Official resource descriptor');
  assert.equal(descriptor.schemaVersion,1);assert(/^[a-f0-9]{40}$/.test(descriptor.sourceCommit),'Official resource requires its full fixed commit');
  assert.equal(descriptor.url,`https://raw.githubusercontent.com/XGENT-ai/skills/${descriptor.sourceCommit}/tools/phoenix-diagram/tests/fixtures/brands/official-test.png`,'Official resource URL must name the fixed repository, file and commit');
  for(const [key,value] of Object.entries(OFFICIAL_RESOURCE))assert.equal(descriptor[key],value,`Official resource ${key} differs`);
  return{descriptor,file:bound(snapshot),readiness:'not-claimed-by-descriptor'};
}
export function bindOfficialResource(state,file){
  if(file){const next=readOfficialResource(file);if(state.officialResource)assert(same(next.file.identity,state.officialResource.file.identity),'Resume cannot replace the original official resource descriptor');else state.officialResource=next;}
  if(state.officialResource)readBound(state.officialResource.file,'original official descriptor');
}
function reference(actual,expected,label,bytesFor){assert(actual&&actual.path===expected.path&&same(actual.identity,expected.identity),`${label} differs from this run`);if(expected.binding&&actual.originalBinding)assert(same(expected.binding,actual.originalBinding),`${label} original binding differs`);readEvidenceBytes(expected,{bytesFor,label});}
function safeReference(referenceValue,label,bytesFor){readEvidenceBytes(referenceValue,{bytesFor,label});return bytesFor?{...referenceValue}:bound(readBound(referenceValue,label));}
function evidenceJson(referenceValue,label,bytesFor){return JSON.parse(readEvidenceBytes(referenceValue,{bytesFor,label}).toString('utf8'));}
function requireChecks(checks,label){assert(Array.isArray(checks)&&checks.length&&checks.every(check=>check.status==='pass'&&typeof check.name==='string'),`${label} checks did not pass`);assert.equal(new Set(checks.map(check=>check.name)).size,checks.length,`${label} checks are duplicated`);}
function coverageKey(value){return`${value.viewId}/${value.viewport?.width}x${value.viewport?.height}/${value.theme}`;}

export function inspectComparison(summaryReference,{base,head,artifact,receipt,engine,allowReviewed=false,bytesFor}){
  const summary=evidenceJson(summaryReference,'comparison summary',bytesFor);
  assert.equal(summary.schemaVersion,1);assert.equal(summary.tool,'phoenix-diagram');assert.equal(summary.kind,'architecture-comparison-summary');assert.equal(summary.command,'visual-check');assert.equal(summary.automatedStatus,'pass');
  assert.deepEqual(summary.stages.map(stage=>[stage.name,stage.status]),[['check','pass'],['visual-check','pass']]);
  const binding=summary.comparison;
  reference(binding.base.candidate,base,'comparison base',bytesFor);reference(binding.head.candidate,head,'comparison head',bytesFor);reference(binding.artifact,artifact,'comparison HTML',bytesFor);reference(binding.receipt,receipt,'immutable compare receipt',bytesFor);
  const original=evidenceJson(receipt,'immutable compare receipt',bytesFor);
  assert.equal(original.kind,'architecture-comparison');assert.equal(original.status,'pass');assert.equal(original.transaction?.status,'committed');assert.equal(original.receiptId,binding.receiptId);
  assert(same(original.artifact,binding.artifact),'Comparison HTML identity drifted');assert(same(original.comparisonIr?.identity,binding.comparisonIr),'Comparison IR identity drifted');
  for(const side of ['base','head']){
    assert(same(binding[side].engine,engine)&&same(original[side].engine,engine),'Comparison engine is not this run engine');
    assert.equal(binding[side].semanticSha256,original[side].semanticSha256);assert.equal(binding[side].configSha256,original[side].configSha256);
    assert(same(binding[side].resources,original[side].config.brandResources||{}),'Comparison resource identity drifted');
  }
  const browserRef=summary.evidence.browserReceipt,browser=evidenceJson(browserRef,'comparison browser receipt',bytesFor);
  assert.equal(browser.kind,'architecture-comparison-browser');assert.equal(browser.command,'visual-check');assert.equal(browser.status,'pass');assert(same(browser.comparison,binding),'Comparison browser belongs to another comparison');
  assert(browser.browserVersion&&typeof browser.browserVersion.product==='string','Comparison lacks an actual browser version');
  requireChecks(browser.checks,'comparison browser');
  const expectedCoverage=new Set(['before','delta','after'].flatMap(view=>VIEWPORTS.flatMap(([width,height])=>THEMES.map(theme=>`${view}/${width}x${height}/${theme}`))));
  assert.equal(browser.coverage?.length,expectedCoverage.size,'Each comparison view needs all current viewport/theme measurements');
  for(const entry of browser.coverage){assert(expectedCoverage.delete(coverageKey(entry)),'Unexpected or duplicate comparison view coverage');assert.equal(entry.status,'pass');requireChecks(entry.checks,'comparison view');for(const name of BROWSER_CHECKS)assert(entry.checks.some(check=>check.name===name),'Comparison view lacks '+name);}
  const expectedShots=new Set(['before','delta','after'].flatMap(view=>[[1440,900],[2048,1320]].flatMap(([width,height])=>THEMES.map(theme=>`${view}/${width}x${height}/${theme}`))));
  assert.equal(browser.screenshots?.length,12,'All twelve original comparison screenshots are required');
  const screenshots=browser.screenshots.map(shot=>{assert(expectedShots.delete(coverageKey(shot)),'Unexpected or duplicate screenshot view/viewport/theme');return{...shot,...safeReference(shot,'comparison screenshot',bytesFor)};});
  for(const stage of summary.stages){assert(same(stage.receipt.comparison,binding),'Comparison stage binding differs');assert.equal(stage.receipt.status,'pass');}
  assert(same(summary.stages[1].receipt,browser),'Comparison browser stage differs from its original receipt');assert.equal(summary.stages[0].receipt.strictProvenance,true);
  const visual=summary.visualReview?.status;
  assert((allowReviewed?['required-pending','passed','failed','unavailable']:['required-pending']).includes(visual),'Unexpected comparison image-review state');
  assert.equal(summary.deliveryStatus,visual==='passed'?'complete':'incomplete');
  if(visual!=='required-pending')verifyComparisonReview(evidenceJson(summary.visualReview.record,'sealed comparison review',bytesFor),{comparison:binding,browserReceipt:browserRef,screenshots},{bytesFor});
  return{summary:safeReference(summaryReference,'comparison summary',bytesFor),comparison:binding,browserReceipt:safeReference(browserRef,'comparison browser receipt',bytesFor),screenshots,automatedStatus:'pass',visualReview:visual,deliveryStatus:summary.deliveryStatus};
}

export function comparisonDraft(evidence){return{schemaVersion:1,kind:'architecture-comparison-review',command:'review-record',comparison:evidence.comparison,browserReceipt:{path:evidence.browserReceipt.path,identity:evidence.browserReceipt.identity},screenshotIdentities:evidence.screenshots.map(shot=>({viewId:shot.viewId,file:{path:shot.path,identity:shot.identity},viewport:shot.viewport,theme:shot.theme,observed:false,observations:''})),reviewer:'',scope:[...REVIEW_SCOPE],verdict:'unavailable',findings:['Image review has not been performed.']};}
export function verifyComparisonReview(record,evidence,{bytesFor}={}){
  keys(record,['schemaVersion','kind','command','comparison','browserReceipt','screenshotIdentities','reviewer','scope','verdict','findings'],'Comparison review record');
  assert.equal(record.schemaVersion,1);assert.equal(record.kind,'architecture-comparison-review');assert.equal(record.command,'review-record');assert(same(record.comparison,evidence.comparison),'Review belongs to another comparison');
  reference(record.browserReceipt,evidence.browserReceipt,'review browser',bytesFor);assert.deepEqual([...record.scope].sort(),[...REVIEW_SCOPE].sort());assert.equal(record.screenshotIdentities.length,12);
  const shots=new Map(evidence.screenshots.map(shot=>[coverageKey(shot),shot]));
  for(const observed of record.screenshotIdentities){keys(observed,['viewId','file','viewport','theme','observed','observations'],'Comparison screenshot observation');const key=coverageKey(observed),expected=shots.get(key);assert(expected,'Unknown or duplicate observed screenshot');shots.delete(key);reference(observed.file,expected,'observed comparison screenshot',bytesFor);assert.equal(typeof observed.observed,'boolean');assert.equal(typeof observed.observations,'string');if(record.verdict==='passed')assert(observed.observed&&observed.observations.trim(),'Every screenshot requires actual observations');}
  assert(['passed','failed','unavailable'].includes(record.verdict));assert(Array.isArray(record.findings));if(record.verdict==='passed'){assert(record.reviewer.trim(),'Actual reviewer is required');assert.equal(record.findings.length,0);}else assert(record.findings.length>0,'A non-passing observation needs its actual findings/limitation');
}
export function singleDraft(evidence){return{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'review-record',status:'unavailable',diagnostics:[],artifact:{path:evidence.artifact.path,identity:evidence.artifact.identity},candidate:{path:evidence.candidate.path,identity:evidence.candidate.identity},configSha256:evidence.configSha256,engine:evidence.engine,deliveryReceiptId:evidence.deliveryReceiptId,browserReceiptIdentity:evidence.browserReceiptIdentity,screenshotIdentities:evidence.screenshots.map(shot=>({file:{path:shot.path,identity:shot.identity},viewport:shot.viewport,theme:shot.theme,observed:false,observations:''})),reviewer:'',scope:[...REVIEW_SCOPE],verdict:'unavailable',findings:['Image review has not been performed.']};}

export function inspectCase(entry,release,{allowReviewed=false,bytesFor}={}){
  assert(CASE_IDS.includes(entry.id));assert(path.dirname(entry.artifact.path)===entry.root,'Artifact escaped its original case directory');
  if(entry.id==='comparison')return inspectComparison(entry.summary,{base:entry.base,head:entry.head,artifact:entry.artifact,receipt:entry.receipt,engine:release.engineIdentity,allowReviewed,bytesFor});
  const summary=evidenceJson(entry.summary,'single diagram summary',bytesFor);
  return inspectFinalize({...summary,evidence:{...summary.evidence,summaryReceipt:entry.summary}},{candidate:entry.candidate,artifact:entry.artifact,engine:release.engineIdentity,contract:'phoenix-v1',type:'architecture',allowReviewed,bytesFor});
}
function stableSummary(value){const copy=structuredClone(value);delete copy.visualReview;delete copy.deliveryStatus;delete copy.diagnostics;if(copy.kind==='architecture-comparison-summary')delete copy.status;if(copy.evidence){delete copy.evidence.reviewRecord;delete copy.evidence.visualReviewRecord;}return copy;}
export function refreshCase(entry,release){
  const current=readSnapshot(entry.summary.path),value=JSON.parse(current.content);
  assert(same(stableSummary(value),entry.summaryAnchor),'Summary changed its immutable automatic evidence');
  entry.summary=bound(current);entry.evidence=inspectCase(entry,release,{allowReviewed:true});return entry;
}
export function rememberCase(entry,release){entry.summaryAnchor=stableSummary(jsonBound(entry.summary,'new summary'));entry.evidence=inspectCase(entry,release);return entry;}

export function readReviewMap(file,state){
  const snapshot=readSnapshot(file),map=JSON.parse(snapshot.content);keys(map,['schemaVersion','runId','records'],'Review map');assert.equal(map.schemaVersion,1);assert.equal(map.runId,state.runId,'Review map belongs to another acceptance run');
  assert(map.records&&typeof map.records==='object'&&!Array.isArray(map.records));for(const [id,file] of Object.entries(map.records)){assert(CASE_IDS.includes(id),'Unknown review case '+id);assert(typeof file==='string'&&path.isAbsolute(file),'Review record paths must be explicit absolute paths');}
  return{file:bound(snapshot),records:map.records};
}

export function recompute(state){
  const cases=CASE_IDS.map(id=>state.cases[id]);
  const checks=[...REQUIRED_STEPS.map(id=>state.steps[id]||{status:'not-run'}),state.preview,state.compareFaults,state.brandExports];
  const automatic=cases.every(entry=>entry?.evidence?.automatedStatus==='pass')&&checks.every(entry=>entry?.status==='pass');
  const reviewed=cases.every(entry=>entry?.evidence?.visualReview==='passed'&&entry.evidence.deliveryStatus==='complete');
  const failed=checks.some(entry=>entry?.status==='failed')||cases.some(entry=>entry?.evidence?.visualReview==='failed');
  state.automatedStatus=automatic?'pass':failed?'failed':'not-run';state.visualReview=reviewed?'passed':cases.some(entry=>entry?.evidence?.visualReview==='failed')?'failed':'required-pending';
  state.status=automatic&&reviewed?'pass':failed?'failed':'incomplete';state.deliveryStatus=state.status==='pass'?'complete':'incomplete';state.updatedAt=new Date().toISOString();return state;
}
export function persistState(state,previous){
  privateDirectory(state.runRoot,{existing:true});const file=path.join(state.runRoot,'state.json');
  if(previous){assert.equal(previous.path,file);readBound(previous,'previous author-tools state');}
  else assert(!fs.existsSync(file),'A previous state must never be replaced implicitly');
  const bytes=Buffer.from(JSON.stringify(state,null,2)+'\n');
  if(previous){const temporary=path.join(state.runRoot,`state-${randomBytes(8).toString('hex')}.tmp`);writeNew(temporary,bytes);readBound(previous,'state before commit');fs.renameSync(temporary,file);}else writeNew(file,bytes);
  return bound(readSnapshot(file));
}
export function immutableCaseReferences(entry){return[entry.candidate,entry.base,entry.head,entry.artifact,entry.receipt,entry.sealedReviewRecord,entry.evidence?.deliveryReceipt,entry.evidence?.browserReceipt,...entry.evidence?.screenshots||[]].filter(Boolean);}
export function verifySavedState(state,stateFile){
  assert.equal(state.schemaVersion,1);assert.equal(state.kind,KIND);assert.equal(state.profile,'p3');assert.equal(state.stage,'author-tools');assert(/^[a-f0-9]{32}$/.test(state.runId));
  assert.equal(path.dirname(stateFile.path),state.runRoot);privateDirectory(state.runRoot,{existing:true});
  const stat=fs.statSync(state.runRoot);assert(same(state.runDirectory,{device:String(stat.dev),inode:String(stat.ino)}),'The original run directory was replaced');
  assert.equal(state.packageConsumer.status,'not-applicable');assert.equal(state.providers.status,'not-applicable');assert.equal(state.finalCandidate.status,'not-applicable');
  for(const reference of Object.values(state.release.files))readBound(reference,'original release identity');
  for(const entry of Object.values(state.cases)){assert.equal(entry.root,path.join(state.runRoot,entry.id));for(const reference of immutableCaseReferences(entry))readBound(reference,'original author-tools artifact');}
  for(const [id,step] of Object.entries(state.steps))if(step.status==='pass'){
    assert.equal(step.process.path,authorProcessPath(state,id,step));const process=jsonBound(step.process,'original completed process');
    assert(process.schemaVersion===1&&process.kind==='p3-owned-native-process'&&same(process.engine,state.release.files.engine)&&same(process.nativeArgv,step.argv),'Saved process command/engine does not belong to this run');
    assert.equal(process.exitCode,0);assert.equal(process.signal,null);assert.equal(jsonBound(process.stdout,'original completed stdout').status,'pass');readBound(process.stderr,'original completed stderr');
  }
  if(state.officialResource)readBound(state.officialResource.file,'original official descriptor');
  if(state.capturedResource)readBound(state.capturedResource.resource,'original captured brand envelope');
  verifyAuthorToolsState(state);
}

export function proveControllerOwnership(child,engine,rows,{controllerPid,workerPid,command='preview'}={}){
  const table=new Map(rows.map(row=>[row.pid,row])),root=table.get(child.pid);assert.equal(table.size,rows.length,'Duplicated process identity');assert(root&&root.pgid===child.pid,'Owned public process group is missing');
  const candidates=rows.filter(row=>row.command===`${engine.path} ${command}`||row.command.startsWith(`${engine.path} ${command} `));
  if(controllerPid===undefined){const owned=candidates.filter(row=>row.pgid===root.pgid);assert.equal(owned.length,1,'Expected one owned native controller');controllerPid=owned[0].pid;}
  assert.notEqual(controllerPid,child.pid,'Public npm needs a distinct native controller descendant');
  const controller=table.get(controllerPid);assert(controller&&candidates.includes(controller),'Marker does not identify the bound native engine/controller');
  const chain=[],seen=new Set();let current=controller;
  while(current){assert(!seen.has(current.pid)&&current.pgid===root.pgid,'Controller has no owned process-group ancestry');seen.add(current.pid);chain.push(current);if(current.pid===child.pid)break;current=table.get(current.ppid);}
  assert(chain.at(-1)?.pid===child.pid,'Controller is not a descendant of this owned public invocation');
  if(workerPid!==undefined)assert(table.get(workerPid)?.ppid===controllerPid&&table.get(workerPid)?.pgid===workerPid&&table.get(workerPid)?.command===`${engine.path} __preview-worker`,'Worker has another controller parent');
  return{schemaVersion:1,kind:'p3-owned-public-controller',outerPid:child.pid,controllerPid,enginePath:engine.path,processGroup:root.pgid,chain,command,...(workerPid===undefined?{}:{workerPid,worker:table.get(workerPid)})};
}
export function verifyActualTransport(actual,publicNpm,args,prefix=[],{complete=true}={}){
  const expected=publicNpmCommand(publicNpm,args);
  assert(actual?.schemaVersion===1&&actual.kind==='p3-public-npm-process','Actual public npm transport evidence is required');
  assert.equal(actual.program,expected.program);assert(same(actual.argv,expected.args),'Actual npm argv differs');assert.equal(actual.cwd,expected.cwd);assert(same(actual.prefix,prefix),'Actual launcher prefix differs');
  assert.equal(actual.spawnedProgram,prefix[0]||expected.program);
  assert(same(actual.spawnedArgv,prefix.length?[...prefix,expected.program,...expected.args]:[expected.program,...expected.args]),'Actual spawned argv differs from the public npm command');
  assert(Number.isSafeInteger(actual.pid)&&actual.pid>0,'Actual public npm PID is missing');assert(Number.isFinite(Date.parse(actual.startedAt)),'Actual process start time is missing');
  if(complete)assert(Number.isFinite(Date.parse(actual.closedAt))&&Date.parse(actual.closedAt)>=Date.parse(actual.startedAt),'Actual process completion time is missing');
}

// One explicit capture retry may retain this failure; no other command gains an
// attempt history, and the original exit/transport evidence stays independently bound.
export function verifyBrandCaptureFailure(state,step,{bytesFor}={}){
  const read=(ref,label)=>readEvidenceBytes(ref,{bytesFor,label}),json=(ref,label)=>evidenceJson(ref,label,bytesFor),release=state.release;
  assert(step?.status==='failed'&&!step.retryOf&&step.error?.message&&step.error?.code,'Retry requires the original failed brand capture');
  assert(release.distribution==='candidate'&&release.publicNpm,'Brand capture retry requires the same public candidate transport');
  for(const ref of Object.values(release.files))read(ref,'retry release');read(release.candidate.tarball,'retry candidate tgz');
  const record=json(release.candidate.record,'retry candidate record'),version=json(release.files.version,'retry candidate VERSION');
  assert(version.distribution==='candidate'&&version.sourceCommit===release.sourceCommit&&version.sourceInputSha256===release.sourceInputSha256&&version.bundleSha256===release.bundleSha256&&record.source.commit===release.sourceCommit&&record.source.inputSha256===release.sourceInputSha256&&record.tarball.sha256===release.candidate.tarball.identity.sha256&&record.tarball.size===release.candidate.tarball.identity.bytes,'Retry candidate/source changed');
  assert(release.publicNpm.enginePath===release.files.engine.path&&release.publicNpm.tarball===release.candidate.tarball.path,'Retry public transport changed');
  const descriptor=json(state.officialResource?.file,'retry official descriptor');keys(descriptor,['schemaVersion','sourceCommit','url','mediaType','bytes','sha256'],'Retry official descriptor');
  assert(same(descriptor,state.officialResource.descriptor)&&descriptor.schemaVersion===1&&/^[a-f0-9]{40}$/.test(descriptor.sourceCommit)&&descriptor.url===`https://raw.githubusercontent.com/XGENT-ai/skills/${descriptor.sourceCommit}/tools/phoenix-diagram/tests/fixtures/brands/official-test.png`,'Retry official resource changed');
  for(const[key,value]of Object.entries(OFFICIAL_RESOURCE))assert.equal(descriptor[key],value,'Retry official resource identity changed');
  const directory=path.join(state.runRoot,'commands','brands-capture'),args=['brands','capture',descriptor.url,'--brand-cache',path.join(state.runRoot,'brand-cache'),'--json'];
  assert(same(step.argv,args)&&same(step.prefix,[])&&step.process?.path===path.join(directory,'process.json'),'Retry original command/path changed');
  const process=json(step.process,'original failed capture'),receipt=json(process.stdout,'original failed capture stdout');read(process.stderr,'original failed capture stderr');
  assert(process.schemaVersion===1&&process.kind==='p3-owned-native-process'&&same(process.engine,release.files.engine)&&same(process.nativeArgv,args)&&process.exitCode===5&&process.signal===null&&process.stdout.path===path.join(directory,'stdout.txt')&&process.stderr.path===path.join(directory,'stderr.txt'),'Retry requires the original native exit 5 capture');
  assert(receipt.schemaVersion===1&&receipt.tool==='phoenix-diagram'&&receipt.toolVersion==='0.2.0'&&receipt.command==='brands capture'&&receipt.status==='failed'&&Array.isArray(receipt.diagnostics)&&receipt.diagnostics.some(row=>row.stage==='brands'&&row.code==='brand/capture-unavailable'&&row.severity==='error')&&receipt.diagnostics.every(row=>row.severity!=='error'||row.code==='brand/capture-unavailable'),'Retry requires brand/capture-unavailable');
  verifyActualTransport(process.actualTransport,release.publicNpm,args,[]);return step;
}
export function verifyBrandCaptureRetry(state,{bytesFor}={}){
  const step=state.steps['brands-capture'];if(!step?.retryOf)return null;
  assert.equal(step.retryOf.path,path.join(state.runRoot,'commands','brands-capture','failed-attempt.json'),'Retry failure history moved');
  const history=evidenceJson(step.retryOf,'original capture failure history',bytesFor);
  assert(history.schemaVersion===1&&history.kind==='p3-brand-capture-failed-attempt'&&history.runId===state.runId&&history.runRoot===state.runRoot&&same(history.release,state.release)&&same(history.officialResource,state.officialResource),'Retry history belongs to another run/release/resource');
  verifyBrandCaptureFailure(state,history.step,{bytesFor});
  for(const key of ['argv','prefix','role','networkScope'])assert(same(step[key],history.step[key]),'Retry changed the original command scope');
  assert(['running','failed','pass'].includes(step.status)&&Array.isArray(history.diagnostics)&&same(state.diagnostics.slice(0,history.diagnostics.length),history.diagnostics),'Retry dropped original diagnostics');
  if(step.process){const directory=path.join(state.runRoot,'commands','brands-capture','retry-1');assert.equal(step.process.path,path.join(directory,'process.json'),'Retry process moved');const process=evidenceJson(step.process,'original retry process',bytesFor);assert(process.schemaVersion===1&&process.kind==='p3-owned-native-process'&&same(process.engine,state.release.files.engine)&&same(process.nativeArgv,step.argv)&&process.stdout.path===path.join(directory,'stdout.txt')&&process.stderr.path===path.join(directory,'stderr.txt'),'Retry process command/engine/output paths changed');readEvidenceBytes(process.stdout,{bytesFor,label:'retry stdout'});readEvidenceBytes(process.stderr,{bytesFor,label:'retry stderr'});verifyActualTransport(process.actualTransport,state.release.publicNpm,step.argv,step.prefix);}
  return history.step;
}
function authorProcessPath(state,id,step){assert(!step.retryOf||id==='brands-capture','Only brand capture has a retry attempt');return path.join(state.runRoot,'commands',id,...(step.retryOf?['retry-1']:[]),'process.json');}

// A single verifier reads live bindings by default, or exact authenticated
// archive bytes through bytesFor. Archive mode does not certify copied inodes.
export function verifyAuthorToolsState(state,{bytesFor,requirePublic=Boolean(state.release?.publicNpm)}={}){
  assert.equal(state.schemaVersion,1);assert.equal(state.kind,KIND);assert.equal(state.profile,'p3');assert.equal(state.stage,'author-tools');assert(/^[a-f0-9]{32}$/.test(state.runId));
  const read=(ref,label)=>readEvidenceBytes(ref,{bytesFor,label}),json=(ref,label)=>evidenceJson(ref,label,bytesFor);
  for(const ref of Object.values(state.release.files))read(ref,'author-tools release');
  if(requirePublic){assert.equal(state.release.distribution,'candidate','Final author-tools must use a candidate, not a development artifact');assert(state.release.publicNpm,'Final author-tools has no public npm transport');assert.equal(state.release.publicNpm.enginePath,state.release.files.engine.path);read(state.release.candidate.tarball,'author-tools candidate tgz');assert.equal(state.release.publicNpm.tarball,state.release.candidate.tarball.path);const record=json(state.release.candidate.record,'owned candidate record'),version=json(state.release.files.version,'installed candidate VERSION');assert.equal(version.distribution,'candidate');assert.equal(version.sourceInputSha256,state.release.sourceInputSha256);assert.equal(version.bundleSha256,state.release.bundleSha256);assert.equal(version.sourceCommit,state.release.sourceCommit);assert.equal(record.source.inputSha256,state.release.sourceInputSha256);assert.equal(record.tarball.sha256,state.release.candidate.tarball.identity.sha256);assert.equal(record.tarball.size,state.release.candidate.tarball.identity.bytes);}
  for(const [id,entry] of Object.entries(state.cases)){
    assert(CASE_IDS.includes(id)&&entry.id===id&&entry.root===path.join(state.runRoot,id),'Author-tools case escaped its run');
    for(const ref of immutableCaseReferences(entry))read(ref,'author-tools immutable artifact');
    if(entry.summary){const summary=json(entry.summary,'author-tools summary');assert(same(stableSummary(summary),entry.summaryAnchor),'Author-tools immutable automatic evidence changed');const actual=inspectCase(entry,state.release,{allowReviewed:true,bytesFor});assert(same(actual,entry.evidence),'Saved author-tools evidence differs from original bytes');
      if(id!=='comparison'&&actual.visualReview!=='required-pending'){
        const record=json(summary.visualReview.record,'sealed single review');assert.equal(record.deliveryReceiptId,actual.deliveryReceiptId);assert(same(record.engine,actual.engine)&&same(record.browserReceiptIdentity,actual.browserReceiptIdentity),'Single review engine/browser differs');reference(record.artifact,actual.artifact,'review artifact',bytesFor);reference(record.candidate,actual.candidate,'review candidate',bytesFor);assert.equal(record.configSha256,actual.configSha256);
        const expected=new Map(actual.screenshots.map(shot=>[`${shot.viewport.width}x${shot.viewport.height}/${shot.theme}`,shot]));assert.equal(record.screenshotIdentities.length,4);
        for(const shot of record.screenshotIdentities){const key=`${shot.viewport?.width}x${shot.viewport?.height}/${shot.theme}`,original=expected.get(key);assert(original,'Single review has duplicate/unknown screenshot');expected.delete(key);reference(shot.file,original,'single reviewed screenshot',bytesFor);if(record.verdict==='passed')assert(shot.observed===true&&shot.observations?.trim(),'Every original screenshot requires actual observations');}
        assert.equal(record.verdict,actual.visualReview);assert(same([...record.scope].sort(),[...REVIEW_SCOPE].sort()),'Single review scope differs');if(record.verdict==='passed')assert(record.reviewer?.trim()&&record.findings?.length===0,'Single review lacks actual reviewer/clear findings');
      }
    }
    if(entry.reviewProcess){const process=json(entry.reviewProcess,'original review process');read(process.stdout,'review stdout');read(process.stderr,'review stderr');if(requirePublic)verifyActualTransport(process.actualTransport,state.release.publicNpm,process.nativeArgv,[]);}
  }
  verifyBrandCaptureRetry(state,{bytesFor});
  for(const [id,step] of Object.entries(state.steps))if(step.status==='pass'){
    assert.equal(step.process.path,authorProcessPath(state,id,step));const process=json(step.process,'original author-tools process');if(REQUIRED_STEPS.includes(id))assert.equal(step.argv[0],id.startsWith('brands-')?'brands':id.startsWith('validate-')?'validate':id.startsWith('finalize-')?'finalize':id==='compare-check'?'check':id==='compare-visual'?'visual-check':'compare','Required author command was replaced');assert(same(process.engine,state.release.files.engine)&&same(process.nativeArgv,step.argv),'Author-tools process command/engine changed');assert.equal(process.exitCode,0);assert.equal(process.signal,null);assert.equal(json(process.stdout,'native stdout').status,'pass');read(process.stderr,'native stderr');if(requirePublic)verifyActualTransport(process.actualTransport,state.release.publicNpm,step.argv,step.prefix);
  }
  const verifyProcess=ref=>{const process=json(ref,'original public process');read(process.stdout,'original process stdout');read(process.stderr,'original process stderr');if(requirePublic)verifyActualTransport(process.actualTransport,state.release.publicNpm,process.args,[]);return process;};
  const verifyOwnership=(proof,process,command)=>{assert(proof?.schemaVersion===1&&proof.kind==='p3-owned-public-controller','Actual native ownership proof is missing');assert.equal(proof.outerPid,process.actualTransport.pid);const actual=proveControllerOwnership({pid:proof.outerPid},state.release.files.engine,[...proof.chain,...(proof.worker?[proof.worker]:[])],{controllerPid:proof.controllerPid,workerPid:proof.workerPid,command});assert(same(actual,proof),'Recorded native ownership chain differs');};
  for(const name of ['preview','compareFaults','brandExports'])if(state[name]?.status==='pass'){
    const report=json(state[name].record,'original '+name);assert.equal(report.status,'pass');if(name==='brandExports')verifyReaderReport(report,state.cases['brand-capture'].artifact,{offline:true,bytesFor});else assert(same(report.engine,state.release.files.engine),'Scope used another engine');
    if(name==='compareFaults')for(const scenario of report.scenarios){assert.equal(scenario.status,'pass');for(const key of ['process','recovery','repeat'])if(scenario[key]){const process=verifyProcess(scenario[key]);if(requirePublic&&key==='process')verifyOwnership(scenario.checkpoint.ownership,process,'compare');}}
    if(name==='preview'){for(const entry of report.cases){assert.equal(entry.status,'pass');assert.equal(entry.portClosed,true);const process=verifyProcess(entry.process);if(requirePublic)verifyOwnership(entry.mode==='sigkill'?entry.checkpoint.ownership:entry.termination,process,'preview');}if(report.workerStops){const stops=json(report.workerStops.record,'original worker stops');assert.equal(stops.status,'pass');for(const entry of stops.cases){assert.equal(entry.status,'pass');assert.equal(entry.portClosed,true);const process=verifyProcess(entry.process);if(requirePublic)verifyOwnership(entry.marker.ownership,process,'preview');}}}
  }
  if(state.officialResource)read(state.officialResource.file,'original official descriptor');if(state.capturedResource)read(state.capturedResource.resource,'original captured resource');
  const expected=recompute(structuredClone(state));for(const key of ['automatedStatus','visualReview','status','deliveryStatus'])assert.equal(state[key],expected[key],'Author-tools '+key+' was promoted without evidence');
  return{status:state.status,automatedStatus:state.automatedStatus,visualReview:state.visualReview,deliveryStatus:state.deliveryStatus,scope:bytesFor?'Authenticated archive bytes; no live filesystem currentness claim':'Original live filesystem bindings'};
}

export function authorToolsAnchor(state){const capture=state.steps['brands-capture'];return{runId:state.runId,release:state.release,cases:Object.fromEntries(Object.entries(state.cases).map(([id,entry])=>[id,Object.fromEntries(['candidate','base','head','artifact','receipt','summaryAnchor','sealedReviewRecord'].filter(key=>entry[key]!==undefined).map(key=>[key,entry[key]]))])),processes:Object.fromEntries(Object.entries(state.steps).filter(([,step])=>step.status==='pass').map(([id,step])=>[id,step.process])),failedBrandCapture:capture?.retryOf?jsonBound(capture.retryOf,'capture retry history').step:capture?.status==='failed'?capture:null,brandCaptureRetry:capture?.retryOf||null,scopes:Object.fromEntries(['preview','compareFaults','brandExports'].filter(name=>state[name]?.status==='pass').map(name=>[name,state[name]])),diagnostics:state.diagnostics||[]};}
export function verifyAuthorToolsAnchor(state,anchor,{bytesFor}={}){
  if(!anchor)return;assert.equal(state.runId,anchor.runId,'Child author-tools run was replaced');assert(same(state.release,anchor.release),'Child author-tools release was replaced');
  for(const [id,expected] of Object.entries(anchor.cases))for(const [key,value] of Object.entries(expected))assert(same(state.cases[id]?.[key],value),'Child author-tools changed original '+id+'/'+key);
  for(const [id,ref] of Object.entries(anchor.processes))assert(state.steps[id]?.status==='pass'&&same(state.steps[id].process,ref),'Child author-tools reran a completed process');
  if(anchor.failedBrandCapture){const capture=state.steps['brands-capture'],original=capture?.retryOf?evidenceJson(capture.retryOf,'anchored capture failure',bytesFor).step:capture;assert(same(original,anchor.failedBrandCapture),'Child author-tools dropped or replaced the original failed capture');}
  if(anchor.brandCaptureRetry)assert(same(state.steps['brands-capture']?.retryOf,anchor.brandCaptureRetry),'Child author-tools dropped or replaced capture retry history');
  for(const[name,scope]of Object.entries(anchor.scopes||{}))assert(same(state[name],scope),'Child author-tools changed a completed scope');
  if(anchor.diagnostics)assert(same((state.diagnostics||[]).slice(0,anchor.diagnostics.length),anchor.diagnostics),'Child author-tools dropped original diagnostics');
}

export const ACCEPTANCE_KIND='phoenix-diagram-p3-acceptance';
export function recomputeAcceptance(state){
  const automatic=state.packageConsumer?.automatedStatus==='pass'&&state.authorTools?.automatedStatus==='pass'&&state.migration?.status==='pass';
  const local=automatic&&['explore','evidence','locale'].every(scope=>state.packageConsumer?.scopes?.includes(scope))&&state.packageConsumer?.phase==='complete'&&state.packageConsumer?.status==='pass'&&state.authorTools?.status==='pass';
  const failed=Boolean(state.failure)||[state.packageConsumer,state.authorTools,state.migration,state.providers].some(scope=>scope?.status==='failed');
  state.automatedStatus=automatic?'pass':failed?'failed':'not-run';state.localStatus=local?'complete':'incomplete';state.visualReview=local?'passed':'required-pending';state.status=local&&state.providers?.status==='pass'?'pass':failed?'failed':'incomplete';state.deliveryStatus=state.status==='pass'?'complete':'incomplete';state.finalCandidate={status:'incomplete',handoff:'pending-aggregate',reason:'Four-platform/Node18/run/job/artifact certification belongs to the final aggregate'};state.updatedAt=new Date().toISOString();return state;
}
export function verifyAcceptanceState(state,{bytesFor,verifyConsumer,verifyMigration,verifyProviders}={}){
  assert.equal(state.schemaVersion,1);assert.equal(state.kind,ACCEPTANCE_KIND);assert.equal(state.profile,'p3');assert(['all','providers'].includes(state.stage));assert(/^[a-f0-9]{32}$/.test(state.runId));
  const read=(ref,label)=>readEvidenceBytes(ref,{bytesFor,label}),json=(ref,label)=>evidenceJson(ref,label,bytesFor);
  for(const ref of [state.candidate.record,state.candidate.tarball,state.candidate.version,state.candidate.sourceInputs])read(ref,'original candidate');
  const candidate=json(state.candidate.record,'candidate record'),version=json(state.candidate.version,'candidate VERSION');assert(same(candidate.source,state.source),'Wrapper source differs from candidate');assert.equal(version.distribution,'candidate');assert.equal(version.sourceInputSha256,state.source.inputSha256);assert.equal(version.sourceCommit,state.source.commit);assert.equal(candidate.tarball.sha256,state.candidate.tarball.identity.sha256);assert.equal(candidate.tarball.size,state.candidate.tarball.identity.bytes);assert.equal(state.candidate.sourceInputs.identity.sha256,state.source.inputSha256);
  if(state.legacyV1){for(const ref of Object.values(state.legacyV1.candidate))read(ref,'explicit historical candidate input');const old=json(state.legacyV1.state,'historical install-only context');assert.equal(old.kind,'phoenix-diagram-candidate-install-context');assert.equal(old.scope,'installation-only');assert.equal(old.status,'pass');assert.equal(old.automatedStatus,'not-run');assert(same(old.candidate.tarball.identity,state.legacyV1.candidate.tarball.identity),'Historical install context belongs to another old package');}
  let consumer;
  if(state.packageConsumer?.state){consumer=json(state.packageConsumer.state,'child consumer state');assert.equal(consumer.kind,'phoenix-diagram-candidate-consumer');assert.equal(consumer.profile,'p3');assert(same(consumer.source,state.source)&&same(consumer.candidate.tarball.identity,state.candidate.tarball.identity),'Child consumer belongs to another candidate');for(const key of ['status','automatedStatus','phase'])assert.equal(state.packageConsumer[key],consumer[key]);assert(same(state.packageConsumer.scopes,consumer.scopes),'Wrapper consumer scopes were changed');assert(verifyConsumer,'Consumer scope needs its established core verifier');verifyConsumer(consumer,{bytesFor:read,state});}
  if(state.authorTools?.state){const author=json(state.authorTools.state,'child author-tools state');assert.equal(author.release.distribution,'candidate');assert(same(author.release.candidate.tarball.identity,state.candidate.tarball.identity)&&same(author.release.candidate.record.identity,state.candidate.record.identity),'Author-tools belongs to another candidate');assert(consumer&&same(author.release.engineIdentity,consumer.engine.engineIdentity)&&same(author.release.publicNpm,consumer.publicNpm),'Author-tools changed the consumed engine/public transport');verifyAuthorToolsAnchor(author,state.authorTools.anchor,{bytesFor});verifyAuthorToolsState(author,{bytesFor,requirePublic:true});for(const key of ['status','automatedStatus','visualReview','deliveryStatus'])assert.equal(state.authorTools[key],author[key]);}
  if(state.migration?.record||state.migration?.status==='pass'){assert(verifyMigration,'Migration needs its same original evidence verifier');assert(state.migration.record,'Passing migration has no original record');verifyMigration(state.migration,{bytesFor:read,state});}
  if(state.providers?.status==='pass'){assert(verifyProviders,'Provider originals need their authenticated verifier');read(state.providers.manifest,'original provider manifest');verifyProviders(json(state.providers.record,'provider report'),{bytesFor:read,state});}
  if(state.stage==='all'){const expected=recomputeAcceptance(structuredClone(state));for(const key of ['status','automatedStatus','localStatus','visualReview','deliveryStatus'])assert.equal(state[key],expected[key],'Wrapper '+key+' was promoted without its original scopes');}
  assert.equal(state.finalCandidate.status,'incomplete');assert.equal(state.finalCandidate.handoff,'pending-aggregate');return{status:state.status,automatedStatus:state.automatedStatus,localStatus:state.localStatus,scope:bytesFor?'Authenticated original archive bytes':'Original live bindings'};
}
export function assertIdentity(actual,expected,label){assert(same(actual,expected),`${label} identity differs`);}
export {bound,readSnapshot,readBound,identity,inspectFinalize};
