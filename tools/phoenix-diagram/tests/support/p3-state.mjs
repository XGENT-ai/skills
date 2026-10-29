import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import {isDeepStrictEqual as same} from 'node:util';
import {bound,readSnapshot,readBound,identity,inspectFinalize,VIEWPORTS,BROWSER_CHECKS} from '../../../../scripts/test-phoenix-diagram-core.mjs';

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
function reference(actual,expected,label){assert(actual&&actual.path===expected.path&&same(actual.identity,expected.identity),`${label} differs from this run`);readBound(expected,label);}
function safeReference(referenceValue,label){const current=readBound(referenceValue,label);return bound(current);}
function requireChecks(checks,label){assert(Array.isArray(checks)&&checks.length&&checks.every(check=>check.status==='pass'&&typeof check.name==='string'),`${label} checks did not pass`);assert.equal(new Set(checks.map(check=>check.name)).size,checks.length,`${label} checks are duplicated`);}
function coverageKey(value){return`${value.viewId}/${value.viewport?.width}x${value.viewport?.height}/${value.theme}`;}

export function inspectComparison(summaryReference,{base,head,artifact,receipt,engine,allowReviewed=false}){
  const summary=jsonBound(summaryReference,'comparison summary');
  assert.equal(summary.schemaVersion,1);assert.equal(summary.tool,'phoenix-diagram');assert.equal(summary.kind,'architecture-comparison-summary');assert.equal(summary.command,'visual-check');assert.equal(summary.automatedStatus,'pass');
  assert.deepEqual(summary.stages.map(stage=>[stage.name,stage.status]),[['check','pass'],['visual-check','pass']]);
  const binding=summary.comparison;
  reference(binding.base.candidate,base,'comparison base');reference(binding.head.candidate,head,'comparison head');reference(binding.artifact,artifact,'comparison HTML');reference(binding.receipt,receipt,'immutable compare receipt');
  const original=jsonBound(receipt,'immutable compare receipt');
  assert.equal(original.kind,'architecture-comparison');assert.equal(original.status,'pass');assert.equal(original.transaction?.status,'committed');assert.equal(original.receiptId,binding.receiptId);
  assert(same(original.artifact,binding.artifact),'Comparison HTML identity drifted');assert(same(original.comparisonIr?.identity,binding.comparisonIr),'Comparison IR identity drifted');
  for(const side of ['base','head']){
    assert(same(binding[side].engine,engine)&&same(original[side].engine,engine),'Comparison engine is not this run engine');
    assert.equal(binding[side].semanticSha256,original[side].semanticSha256);assert.equal(binding[side].configSha256,original[side].configSha256);
    assert(same(binding[side].resources,original[side].config.brandResources||{}),'Comparison resource identity drifted');
  }
  const browserRef=summary.evidence.browserReceipt,browser=jsonBound(browserRef,'comparison browser receipt');
  assert.equal(browser.kind,'architecture-comparison-browser');assert.equal(browser.command,'visual-check');assert.equal(browser.status,'pass');assert(same(browser.comparison,binding),'Comparison browser belongs to another comparison');
  assert(browser.browserVersion&&typeof browser.browserVersion.product==='string','Comparison lacks an actual browser version');
  requireChecks(browser.checks,'comparison browser');
  const expectedCoverage=new Set(['before','delta','after'].flatMap(view=>VIEWPORTS.flatMap(([width,height])=>THEMES.map(theme=>`${view}/${width}x${height}/${theme}`))));
  assert.equal(browser.coverage?.length,expectedCoverage.size,'Each comparison view needs all current viewport/theme measurements');
  for(const entry of browser.coverage){assert(expectedCoverage.delete(coverageKey(entry)),'Unexpected or duplicate comparison view coverage');assert.equal(entry.status,'pass');requireChecks(entry.checks,'comparison view');for(const name of BROWSER_CHECKS)assert(entry.checks.some(check=>check.name===name),'Comparison view lacks '+name);}
  const expectedShots=new Set(['before','delta','after'].flatMap(view=>[[1440,900],[2048,1320]].flatMap(([width,height])=>THEMES.map(theme=>`${view}/${width}x${height}/${theme}`))));
  assert.equal(browser.screenshots?.length,12,'All twelve original comparison screenshots are required');
  const screenshots=browser.screenshots.map(shot=>{assert(expectedShots.delete(coverageKey(shot)),'Unexpected or duplicate screenshot view/viewport/theme');return{...shot,...safeReference(shot,'comparison screenshot')};});
  for(const stage of summary.stages){assert(same(stage.receipt.comparison,binding),'Comparison stage binding differs');assert.equal(stage.receipt.status,'pass');}
  assert(same(summary.stages[1].receipt,browser),'Comparison browser stage differs from its original receipt');assert.equal(summary.stages[0].receipt.strictProvenance,true);
  const visual=summary.visualReview?.status;
  assert((allowReviewed?['required-pending','passed','failed','unavailable']:['required-pending']).includes(visual),'Unexpected comparison image-review state');
  assert.equal(summary.deliveryStatus,visual==='passed'?'complete':'incomplete');
  if(visual!=='required-pending')verifyComparisonReview(jsonBound(summary.visualReview.record,'sealed comparison review'),{comparison:binding,browserReceipt:browserRef,screenshots});
  return{summary:safeReference(summaryReference,'comparison summary'),comparison:binding,browserReceipt:safeReference(browserRef,'comparison browser receipt'),screenshots,automatedStatus:'pass',visualReview:visual,deliveryStatus:summary.deliveryStatus};
}

export function comparisonDraft(evidence){return{schemaVersion:1,kind:'architecture-comparison-review',command:'review-record',comparison:evidence.comparison,browserReceipt:{path:evidence.browserReceipt.path,identity:evidence.browserReceipt.identity},screenshotIdentities:evidence.screenshots.map(shot=>({viewId:shot.viewId,file:{path:shot.path,identity:shot.identity},viewport:shot.viewport,theme:shot.theme,observed:false,observations:''})),reviewer:'',scope:[...REVIEW_SCOPE],verdict:'unavailable',findings:['Image review has not been performed.']};}
export function verifyComparisonReview(record,evidence){
  keys(record,['schemaVersion','kind','command','comparison','browserReceipt','screenshotIdentities','reviewer','scope','verdict','findings'],'Comparison review record');
  assert.equal(record.schemaVersion,1);assert.equal(record.kind,'architecture-comparison-review');assert.equal(record.command,'review-record');assert(same(record.comparison,evidence.comparison),'Review belongs to another comparison');
  reference(record.browserReceipt,evidence.browserReceipt,'review browser');assert.deepEqual([...record.scope].sort(),[...REVIEW_SCOPE].sort());assert.equal(record.screenshotIdentities.length,12);
  const shots=new Map(evidence.screenshots.map(shot=>[coverageKey(shot),shot]));
  for(const observed of record.screenshotIdentities){keys(observed,['viewId','file','viewport','theme','observed','observations'],'Comparison screenshot observation');const key=coverageKey(observed),expected=shots.get(key);assert(expected,'Unknown or duplicate observed screenshot');shots.delete(key);reference(observed.file,expected,'observed comparison screenshot');assert.equal(typeof observed.observed,'boolean');assert.equal(typeof observed.observations,'string');if(record.verdict==='passed')assert(observed.observed&&observed.observations.trim(),'Every screenshot requires actual observations');}
  assert(['passed','failed','unavailable'].includes(record.verdict));assert(Array.isArray(record.findings));if(record.verdict==='passed'){assert(record.reviewer.trim(),'Actual reviewer is required');assert.equal(record.findings.length,0);}else assert(record.findings.length>0,'A non-passing observation needs its actual findings/limitation');
}
export function singleDraft(evidence){return{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'review-record',status:'unavailable',diagnostics:[],artifact:{path:evidence.artifact.path,identity:evidence.artifact.identity},candidate:{path:evidence.candidate.path,identity:evidence.candidate.identity},configSha256:evidence.configSha256,engine:evidence.engine,deliveryReceiptId:evidence.deliveryReceiptId,browserReceiptIdentity:evidence.browserReceiptIdentity,screenshotIdentities:evidence.screenshots.map(shot=>({file:{path:shot.path,identity:shot.identity},viewport:shot.viewport,theme:shot.theme,observed:false,observations:''})),reviewer:'',scope:[...REVIEW_SCOPE],verdict:'unavailable',findings:['Image review has not been performed.']};}

export function inspectCase(entry,release,{allowReviewed=false}={}){
  assert(CASE_IDS.includes(entry.id));assert(path.dirname(entry.artifact.path)===entry.root,'Artifact escaped its original case directory');
  if(entry.id==='comparison')return inspectComparison(entry.summary,{base:entry.base,head:entry.head,artifact:entry.artifact,receipt:entry.receipt,engine:release.engineIdentity,allowReviewed});
  const summary=jsonBound(entry.summary,'single diagram summary');
  return inspectFinalize({...summary,evidence:{...summary.evidence,summaryReceipt:entry.summary}},{candidate:entry.candidate,artifact:entry.artifact,engine:release.engineIdentity,contract:'phoenix-v1',type:'architecture',allowReviewed});
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
    assert.equal(step.process.path,path.join(state.runRoot,'commands',id,'process.json'));const process=jsonBound(step.process,'original completed process');
    assert(process.schemaVersion===1&&process.kind==='p3-owned-native-process'&&same(process.engine,state.release.files.engine)&&same(process.nativeArgv,step.argv),'Saved process command/engine does not belong to this run');
    assert.equal(process.exitCode,0);assert.equal(process.signal,null);assert.equal(jsonBound(process.stdout,'original completed stdout').status,'pass');readBound(process.stderr,'original completed stderr');
  }
  if(state.officialResource)readBound(state.officialResource.file,'original official descriptor');
  if(state.capturedResource)readBound(state.capturedResource.resource,'original captured brand envelope');
}
export function assertIdentity(actual,expected,label){assert(same(actual,expected),`${label} identity differs`);}
export {bound,readSnapshot,readBound,identity,inspectFinalize};
