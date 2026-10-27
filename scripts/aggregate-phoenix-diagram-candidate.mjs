#!/usr/bin/env node
// Aggregates archived bytes and authenticated job outcomes. Original physical
// currentness was checked in the live consumer job; copied inodes prove nothing.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual as same } from 'node:util';
import { TARGETS,readTarballInventory } from './pack-phoenix-diagram-candidate.mjs';
import { validateCandidate,targetForHost } from './check-phoenix-diagram-candidate.mjs';
import { TYPES,LEGACY_TYPES,contractsFor,requiredArtifactChecks,BROWSER_CHECKS,VIEWPORTS,identity,readSnapshot,readBound,requireThat,assertXssArtifact } from './test-phoenix-diagram-core.mjs';
import { verifyFileSignature,verifyPrintSample,verifyMotionSuppression } from '../tools/phoenix-diagram/tests/export-acceptance.mjs';
import { verifyReaderInteractions } from '../tools/phoenix-diagram/tests/support/export-reader.mjs';
import { COLLECTOR_MEMBER,parseCaptureArgs } from '../tools/phoenix-diagram/tests/support/capture-stdio.mjs';
export { verifyReaderInteractions as checkReaderInteractionEvidence };
export { verifyMotionSuppression as checkMotionSuppressionEvidence };

const HASH=/^[a-f0-9]{64}$/;
const KINDS=['diagram','labels','cards','relationship-occlusion','hierarchy','controls'];
// Case coverage per profile: P1 pairs the five original types with both input
// contracts; P2 adds the five new types, which only have phoenix-v1.
export const PROFILE_CASES={p1:LEGACY_TYPES.flatMap(type=>['phoenix-v1','legacy-2.17'].map(contract=>`${type}:${contract}`)),p2:TYPES.flatMap(type=>contractsFor(type).map(contract=>`${type}:${contract}`))};
// The local full-review consumption adds these scopes; the corpus stays a source-checkout acceptance.
export const LOCAL_SCOPES=['explore','evidence','locale'];
const sorted=value=>[...value].sort();
const sha=value=>identity(value).sha256;
const object=value=>value && typeof value==='object' && !Array.isArray(value);
function need(value,message) {requireThat(value,message,'aggregate/incomplete');}
function relative(value) {need(typeof value==='string' && value && !value.startsWith('/') && !/[\\:\0-\x1f\x7f]/.test(value) && value.split('/').every(part=>part && part!=='.' && part!=='..'),'Unsafe archive member path');return value;}
function processBytes(program,args,input) {const result=spawnSync(program,args,{input,encoding:null,maxBuffer:512*1024*1024,timeout:120000});need(!result.error && result.status===0,`Missing or invalid archive member (${program}): ${result.error?.message || result.stderr?.toString().trim()}`);return result.stdout;}
function digest(bytes,expected,label) {need(expected && HASH.test(expected.sha256) && Number.isSafeInteger(expected.bytes) && same(identity(bytes),expected),`${label} archived bytes/digest differ`);return bytes;}
function header(value,command) {need(value?.schemaVersion===1 && value.tool==='phoenix-diagram' && value.toolVersion==='0.2.0' && value.command===command && Array.isArray(value.diagnostics),`Missing actual ${command} receipt`);}

export class ArchiveEvidence {
  constructor(archive,prefix,originalRoot) {
    this.archive=readBound(archive,'Actions evidence zip').path;this.prefix=prefix===''?'':relative(prefix);this.originalRoot=originalRoot;
    need(typeof originalRoot==='string' && originalRoot.startsWith('/') && !originalRoot.endsWith('/'),'Original evidence root must be absolute');this.cache=new Map();
    const names=processBytes('unzip',['-Z','-1',this.archive]).toString('utf8').split('\n').filter(Boolean).map(name=>relative(name.replace(/\/$/,'')));this.members=new Set(names);need(this.members.size===names.length,'Duplicate archive members are ambiguous');
  }
  member(name) {relative(name);need(this.members.has(name),`Missing archived member: ${name}`);if(!this.cache.has(name))this.cache.set(name,processBytes('unzip',['-p',this.archive,name]));return this.cache.get(name);}
  bytes(reference) {
    need(reference && typeof reference.path==='string' && reference.path.startsWith(`${this.originalRoot}/`),'Evidence reference leaves its original root/path');
    return digest(this.member(`${this.prefix?`${this.prefix}/`:''}${relative(reference.path.slice(this.originalRoot.length+1))}`),reference.identity,'Referenced evidence');
  }
  json(reference) {return JSON.parse(this.bytes(reference).toString('utf8'));}
  relativeJson(member) {return JSON.parse(this.member(relative(member)).toString('utf8'));}
}
// A local consumption keeps its original filesystem: references are reread and rehashed in place.
export class DirectoryEvidence {
  constructor(root) {const stat=fs.lstatSync(root);need(path.isAbsolute(root) && stat.isDirectory() && !stat.isSymbolicLink() && fs.realpathSync(root)===root,'Local evidence root must be a canonical ordinary directory');this.originalRoot=root;}
  bytes(reference) {
    need(reference && typeof reference.path==='string' && reference.path.startsWith(`${this.originalRoot}/`) && fs.realpathSync(reference.path)===reference.path,'Evidence reference leaves its original root/path');relative(reference.path.slice(this.originalRoot.length+1));
    return digest(readSnapshot(reference.path,{allowMultipleLinks:true}).content,reference.identity,'Referenced evidence');
  }
  json(reference) {return JSON.parse(this.bytes(reference).toString('utf8'));}
}
export function checkStateSelection(store,phase,stateMember) {need(phase.state && store.bytes(phase.state).equals(store.member(stateMember)),'The explicit state member differs from the actual final workflow state reference');}
export function checkInputSlots(input,profile='p1') {
  need(Array.isArray(input.platforms) && same(sorted(input.platforms.map(row=>row.target)),sorted(TARGETS)),'Four distinct native platform target records are required');
  need(input.node18?.target==='linux-x64','Node18 floor requires its separate linux-x64 consumer record');need(Array.isArray(input.providers) && same(sorted(input.providers.map(row=>row.provider)),['claude-code','codex']),'Both distinct real provider records are required');
  if(profile==='p2')need(input.local?.target==='darwin-arm64' && typeof input.local.runRoot==='string' && input.local.state?.path,'P2 requires the local darwin-arm64 full-review consumption record');
}
export function checkStateCohort(state,cohort) {
  need(state?.schemaVersion===1 && state.kind==='phoenix-diagram-candidate-consumer','Unsupported consumer state');
  need(state.repository===cohort.repository && String(state.runId)===String(cohort.runId) && Number(state.runAttempt)===Number(cohort.runAttempt),'Consumer run/attempt cohort differs');
  need(same(state.source,cohort.source) && state.candidate?.tarball?.identity?.sha256===cohort.tgzSha256 && state.candidate.candidateArchiveSha256===cohort.candidateArchiveSha256 && state.harnessArchiveSha256===cohort.harnessSha256,'Candidate/source/harness cohort differs');
  if(cohort.candidateArtifactId)need(Number(state.candidate.artifactId)===cohort.candidateArtifactId && state.candidate.artifactDigest===cohort.candidateArtifactDigest,'Consumer candidate artifact provenance differs');
}
export function checkJobProof(job,artifact,entry,cohort,{node18=false,reviewed=!node18}={}) {
  need(job?.id===entry.jobId && String(job.run_id)===String(cohort.runId) && job.run_attempt===cohort.runAttempt && job.head_sha===cohort.source.commit && job.status==='completed' && job.conclusion==='success','Actual job is not completed/success for this run attempt/source');
  need(job.name===(node18?'Consume the same tgz on Node 18':`Consume the same tgz on ${entry.target}`),'Actual native job target/name differs');
  const required=node18?['Consume only the same tgz on the declared Node floor']:['Consume the unchanged tgz through its package entry points',...(reviewed?['Resume the same delivery after actual image review']:[])];
  need(required.every(name=>job.steps?.some(step=>step.name===name && step.conclusion==='success')),'Actual consume/review job step is missing or not successful');
  need(artifact?.id===entry.artifact.id && artifact.name===entry.artifact.name && artifact.expired===false && artifact.digest===`sha256:${entry.artifact.digest}` && String(artifact.workflow_run?.id)===String(cohort.runId) && artifact.workflow_run.head_sha===cohort.source.commit,'Actual uploaded artifact identity/digest/source differs');
  need(entry.artifact.name===(node18?'phoenix-diagram-consume-node18-evidence':`phoenix-diagram-consume-${entry.target}-evidence`),'Unexpected consumer artifact name');
  const uploaded=Date.parse(artifact.created_at),started=Date.parse(job.started_at),completed=Date.parse(job.completed_at);need(Number.isFinite(uploaded) && Number.isFinite(started) && Number.isFinite(completed) && uploaded>=started && uploaded<=completed,'Artifact upload time is outside this completed job');
}
export function checkNode18Floor(state,phase,proof) {need([state.runtimeNode,state.environment?.proof?.node,state.environment?.npmChildProof?.node,phase.node,proof.node].every(value=>value==='v18.20.8'),'Node18 evidence must all identify the actual 18.20.8 floor');}
function checks(rows,names,label) {need(Array.isArray(rows) && rows.length>=names.length && new Set(rows.map(row=>row.name)).size===rows.length && rows.every(row=>row.status==='pass' && object(row.scope)) && names.every(name=>rows.some(row=>row.name===name)),`${label} named checks/scope are incomplete`);}
function boundFile(left,right,label) {need(left?.path===right?.path && same(left?.identity,right?.identity),`${label} original path/digest changed`);}
function browserEvidence(browser,store,artifact) {
  header(browser,'browser-check');need(browser.status==='pass' && browser.artifactPath===artifact.path && same(browser.artifact,artifact.identity) && browser.browserVersion?.product,'Actual browser artifact/version differs');checks(browser.checks,[...BROWSER_CHECKS,'browser-cleanup','artifact-current','screenshot-capture'],'browser');
  need(browser.coverage?.length===10,'Viewport/theme coverage is incomplete');const seen=new Set();
  for(const row of browser.coverage) {const key=`${row.viewport?.width}x${row.viewport?.height}:${row.theme}`;need(!seen.has(key) && ['light','dark'].includes(row.theme) && VIEWPORTS.some(([width,height])=>row.viewport.width===width && row.viewport.height===height) && row.status==='pass','Viewport/theme coverage differs');seen.add(key);checks(row.checks,BROWSER_CHECKS,'viewport');}
  need(browser.screenshots?.length===4,'Four actual screenshot files are required');const shots=new Set();
  for(const shot of browser.screenshots) {const key=`${shot.viewport?.width}x${shot.viewport?.height}:${shot.theme}`;need(['1440x900:dark','1440x900:light','2048x1320:dark','2048x1320:light'].includes(key) && !shots.has(key),'Screenshot scope differs');shots.add(key);const bytes=store.bytes(shot);need(bytes.length>=24 && bytes.subarray(0,8).toString('hex')==='89504e470d0a1a0a' && bytes.readUInt32BE(16)===shot.width && bytes.readUInt32BE(20)===shot.height,'Screenshot PNG dimensions/signature differ');}
}
function reviewEvidence(record,summary,browser) {
  header(record,'review-record');need(record.status==='pass' && record.verdict==='passed' && record.reviewer?.trim() && same(sorted(record.scope || []),sorted(KINDS)) && record.deliveryReceiptId===summary.deliveryReceiptId && same(record.browserReceiptIdentity,summary.browserReceiptIdentity) && record.configSha256===summary.configSha256 && same(record.engine,summary.engine),'Actual visual review identity/scope is incomplete');boundFile(record.artifact,summary.artifact,'Visual artifact');boundFile(record.candidate,summary.candidate,'Visual candidate');
  need(record.screenshotIdentities?.length===4,'Observed screenshot scope is incomplete');const seen=new Set();for(const observation of record.screenshotIdentities){const shot=browser.screenshots.find(file=>same(observation.file,{path:file.path,identity:file.identity}) && same(observation.viewport,file.viewport) && observation.theme===file.theme);need(shot && !seen.has(shot.path) && observation.observed===true && observation.observations?.trim(),'Screenshot was not actually observed in this exact scope');seen.add(shot.path);}
}
function caseEvidence(entry,store,engine,{reviewed=true,reader=true}={}) {
  const summary=store.json(entry.finalize);header(summary,'finalize');need(summary.status==='pass' && summary.automatedStatus==='pass' && summary.producedByThisRun===true && same(summary.engine,engine),'Current native finalize gates/engine are not passing');boundFile(summary.artifact,entry.state.artifact,'Summary artifact');boundFile(summary.candidate,entry.candidate,'Summary candidate');store.bytes(summary.artifact);store.bytes(summary.candidate);
  const stages=['validate','deliver','check','browser-check'];need(summary.stages?.length===4 && summary.stages.every((stage,index)=>stage.name===stages[index] && stage.status==='pass' && stage.receipt?.status==='pass'),'Four executed automatic stages are required');const [validation,delivery,strict,browser]=summary.stages.map(stage=>stage.receipt);
  need(summary.deliveryReceiptId===entry.state.deliveryReceiptId && same(summary.browserReceiptIdentity,entry.state.browserReceiptIdentity) && summary.configSha256===entry.state.configSha256 && (!entry.contract || delivery.inputContract===entry.contract),'Current case delivery/config/browser/input identity differs');
  const required=requiredArtifactChecks(delivery.inputContract),document=JSON.parse(store.bytes(summary.candidate).toString('utf8'));
  need((document.meta?.input_contract===undefined || document.meta.input_contract===delivery.inputContract) && validation.config?.inputContract===delivery.inputContract && delivery.config?.inputContract===delivery.inputContract && same(validation.config,delivery.config),'Selected input contract differs between candidate and native configuration');
  for(const [index,value] of [validation,delivery,strict,browser].entries()){header(value,stages[index]);need(value.configSha256===summary.configSha256 && same(value.engine,engine),'Stage config/engine changed');boundFile(value.candidate,summary.candidate,'Stage candidate');}
  for(const value of [validation,delivery,strict]){checks(value.checks,required,'artifact');need(value.checks.length===required.length,'Artifact named check scope differs from its selected input contract');need(value.composition?.status==='pass' && value.composition.summary?.errors===0 && (value.composition.profile!=='showcase' || value.composition.summary.warnings===0),'Composition gate is incomplete');}
  need([validation,delivery,strict].every(value=>same(value.candidate,summary.candidate)) && same(delivery.artifact,summary.artifact) && same(strict.artifact,summary.artifact),'Original native physical bindings disagree inside the archived receipts');
  need(delivery.receiptId===summary.deliveryReceiptId && delivery.commitState==='committed' && strict.deliveryReceiptId===summary.deliveryReceiptId && strict.scope==='strict-provenance' && strict.provenance==='current' && browser.deliveryReceiptId===summary.deliveryReceiptId && same(store.json(strict.deliveryReceipt),delivery),'Original delivery/strict receipt identity differs');need(same(summary.browserReceiptIdentity,summary.evidence.browserReceipt.identity) && same(store.json(summary.evidence.browserReceipt),browser),'Sealed browser receipt differs');browserEvidence(browser,store,summary.artifact);
  if(reviewed){need(summary.visualReview?.status==='passed' && summary.deliveryStatus==='complete' && entry.state.visualReview==='passed' && entry.state.deliveryStatus==='complete','Actual image review is pending/unavailable/failed');reviewEvidence(store.json(summary.visualReview.record),summary,browser);const reply=store.json(entry.review.idempotent?.receipt || entry.review.receipt);header(reply,'review-record');need(reply.status==='pass' && reply.deliveryReceiptId===summary.deliveryReceiptId && same(reply.browserReceiptIdentity,summary.browserReceiptIdentity) && reply.visualReview.status==='passed' && reply.deliveryStatus==='complete','Actual public review reply did not complete the original identity');boundFile(reply.evidence.summaryReceipt,entry.finalize,'Review summary');}
  else need(summary.visualReview?.status==='required-pending' && summary.deliveryStatus==='incomplete','Automatic-only Node18 run must retain pending visual review');
  if(reader)readerEvidence(store.json(entry.reader.receipt),store,summary.artifact);return summary;
}
function processReceipt(store,reference,{success=true}={}) {const value=store.json(reference);need(typeof value.stdout==='string' && typeof value.stderr==='string' && (success?value.exitCode===0:value.exitCode!==0),'Actual process outcome is missing or not expected');return value;}
function coreEvidence(core,store,engine,{reviewed=true,cases=PROFILE_CASES.p1,scopes=[]}={}) {
  for(const entry of core.cases || [])need(store.bytes(entry.source).equals(store.bytes(entry.candidate)) && entry.determinism?.distinctDeliveryReceiptId!==entry.state?.deliveryReceiptId,'Original packaged input changed or replay inherited its receipt');
  need(core.kind==='phoenix-diagram-core-acceptance' && core.automatedStatus==='pass' && core.coverage?.complete===true && core.cases?.length===cases.length,'Full profile type/input-contract core coverage is missing');
  need(same(sorted(core.cases.map(row=>`${row.type}:${row.contract}`)),sorted(cases)),'Core type/input-contract coverage differs');
  for(const entry of core.cases){need(entry.state?.automatedStatus==='pass' && entry.reader?.status==='pass' && entry.determinism?.status==='pass','Case has a failed/not-run native, Reader or deterministic scope');const summary=caseEvidence(entry,store,engine,{reviewed});need(same(identity(store.bytes(entry.determinism.artifact)),summary.artifact.identity),'Deterministic replay differs');}
  need(['content','recovery','offline'].every(key=>core[key]?.status==='pass'),'XSS/recovery/offline scope is incomplete');assertXssArtifact(store.bytes(core.content.artifact));readerEvidence(store.json(core.content.browser.receipt),store,core.content.artifact,{content:true});
  const recoveryIds=['restore-and-repeat','later-claimant','record-tamper','backup-tamper','parent-replacement','kill-after-recovery-link','kill-after-witness-quarantine','kill-recovery-later-claimant'];need(same(sorted(core.recovery.scenarios.map(row=>row.id)),sorted(recoveryIds)),'Public recovery fault coverage is incomplete');
  for(const scenario of core.recovery.scenarios){need(scenario.status==='pass' && scenario.interruption?.phase==='after-retire' && scenario.interruption.signal==='SIGKILL' && scenario.interruption.terminalOutputDiscarded===true,'Actual publication interruption evidence is missing');const journal=store.json(scenario.journal);need(journal.kind==='output-publication' && journal.target?.commitPath===scenario.target,'Durably located target differs');const result=store.json(scenario.recovery.receipt);header(result,'recover-output');need(scenario.recovery.exitCode===(scenario.recovery.recoveryStatus==='recovered'?0:3) && result.recoveryStatus===scenario.recovery.recoveryStatus && !result.deliveryStatus && !result.deliveryReceiptId,'Recovery exit/state invented delivery evidence');if(scenario.repeat)need(store.json(scenario.repeat.receipt).recoveryStatus==='absent' && scenario.repeat.exitCode===0,'Recovery repeat was not idempotent');for(const file of scenario.retained || [])store.bytes(file);if(!scenario.finalPublic?.absent)store.bytes(scenario.finalPublic);if(scenario.recoveryInterruption)need(scenario.recoveryInterruption.signal==='SIGKILL' && ['after-recovery-link','after-witness-quarantine'].includes(scenario.recoveryInterruption.phase),'Recovery re-interruption is missing');}
  need(core.offline.cases?.length===cases.length && core.offline.native?.status==='pass' && core.offline.browser?.status==='pass','OS-native/CDP-browser offline scope is incomplete');for(const row of core.offline.cases){const delivered=store.json(row.native.delivery),checked=store.json(row.native.check);header(delivered,'deliver');header(checked,'check');need(delivered.status==='pass' && checked.status==='pass' && checked.provenance==='current' && checked.scope==='strict-provenance','Offline native generate/strict failed');store.bytes(row.native.artifact);readerEvidence(store.json(row.browser.receipt),store,row.native.artifact);}
  if(reviewed){need(core.handoff==='passed' && core.status==='pass' && core.reviewBindings?.status==='pass' && core.reviewRecovery?.status==='pass' && core.reviewRecovery.retryStatus==='pass' && core.newDelivery?.status==='pass','Original review/retry/new-delivery scope is incomplete');for(const check of core.reviewBindings.checks)need(store.json(check.receipt).status==='failed' && check.exitCode!==0,'Changed review binding was not rejected');const failed=store.json(core.reviewRecovery.receipt);need(failed.status==='failed' && failed.automatedStatus==='pass' && failed.visualReview?.record && core.reviewRecovery.summaryUnchanged===true,'Sealed-record/summary-failure evidence is missing');store.bytes(core.reviewRecovery.record);need(store.json(core.newDelivery.oldReviewRejected.receipt).status==='failed','A new delivery inherited old review');}
  scopeEvidence(core,store,scopes);
}
const RELATION_TYPES=new Set([...LEGACY_TYPES,'erd','class','tree']);
function scopedDelivery(row,store) {const summary=store.json(row.summary);header(summary,'finalize');need(summary.automatedStatus==='pass' && same(summary.evidence?.status,row.evidence?.status) && same(summary.locale,row.locale),'Scoped finalize outcome differs from its recorded summary');store.bytes(row.candidate);store.bytes(row.artifact);store.bytes(row.receipt);for(const shot of row.screenshots || [])store.bytes(shot);return summary;}
// Requested acceptance scopes keep their original receipts, artifacts and captured files.
function scopeEvidence(core,store,scopes) {
  for(const name of scopes)need(core[name]?.status==='pass',`Requested ${name} scope did not pass`);
  if(scopes.includes('explore')) {
    need(same(sorted(core.explore.cases.map(row=>row.id)),sorted(core.cases.map(row=>row.id))),'Exploration does not cover every case');
    for(const row of core.explore.cases) {
      const report=store.json(row.receipt);need(report.status==='pass' && report.offline?.remoteRequests?.length===0 && row.steps?.length>0 && row.steps.every(step=>step.status==='pass'),'Exploration receipt or step did not pass');
      need(row.cards.length===(RELATION_TYPES.has(row.type)?2:0),'Share card scope differs');
      for(const card of row.cards){verifyFileSignature(store.bytes(card),'png','image/png');need(card.canonical==='false' && ['route','reach'].includes(card.variant) && card.decode?.valid===true,'Share card is not a decoded non-canonical route/reach export');}
    }
  }
  if(scopes.includes('evidence')) {
    const evidence=core.evidence,cases=evidence.cases || {};for(const row of [...Object.values(cases),evidence.worktreeChange])scopedDelivery(row,store);
    need(['architecture','timeline','tree'].every(type=>cases[type]?.evidence?.status==='verified') && cases.waterfall?.evidence?.status==='none' && evidence.worktreeChange?.evidence?.status==='partial' && evidence.links?.every(link=>link.status!=='failed'),'Repository evidence states differ from verified, local-only, illustrative and worktree-change outcomes');
    need(evidence.passports?.length===3,'Source passport scope is incomplete');for(const passport of evidence.passports){need(store.json(passport.receipt).status==='pass','Passport capture failed');for(const shot of passport.screenshots)verifyFileSignature(store.bytes(shot),'png','image/png');}
  }
  if(scopes.includes('locale')) {
    need(same(sorted(core.locale.cases.map(row=>row.tag)),sorted(['en','zh-CN','zh-TW','es','ko','fr'])),'Interface language scope is incomplete');
    for(const row of core.locale.cases){scopedDelivery(row,store);need(same(row.locale,row.tag==='fr'?{requested:'fr',resolved:'en',fallback:true}:{requested:row.tag,resolved:row.tag,fallback:false}),'Resolved interface language differs');}
  }
}
function phaseEvidence(phase,entry,cohort,{node18=false,reviewed=!node18}={}) {
  need(phase.schemaVersion===1 && phase.tool==='phoenix-diagram' && String(phase.workflowRun)===String(cohort.runId) && phase.sourceCommit===cohort.source.commit && phase.target===entry.target && phase.candidate?.tgzSha256===cohort.tgzSha256 && phase.candidate.sha256===cohort.candidateArchiveSha256 && phase.harness?.sha256===cohort.harnessSha256,'Actual workflow phase receipt differs');need(phase.phases?.prepare==='success' && phase.phases.consume==='success' && (!reviewed || phase.phases.review==='success'),'An actual workflow phase is not successful');if(node18)need(phase.node==='v18.20.8','Node18 phase did not run the exact 18.20.8 floor');
  need(Number(phase.candidate.artifactId)===cohort.candidateArtifactId && phase.candidate.artifactDigest===cohort.candidateArtifactDigest && Number(phase.harness.artifactId)===cohort.harnessArtifactId && phase.harness.artifactDigest===cohort.harnessArtifactDigest,'Workflow candidate/harness artifact IDs or digests differ');
}
function platformEvidence(entry,cohort,api,{node18=false,reviewed=!node18,cases=PROFILE_CASES.p1}={}) {
  need(entry.artifact.archive.identity.sha256===entry.artifact.digest,'Evidence archive SHA differs from its Actions artifact digest');const store=new ArchiveEvidence(entry.artifact.archive,entry.evidencePrefix,'/pending');const state=store.relativeJson(entry.stateMember);store.originalRoot=state.runRoot;checkStateCohort(state,cohort);
  checkJobProof(api(`/repos/${cohort.repository}/actions/jobs/${entry.jobId}`),api(`/repos/${cohort.repository}/actions/artifacts/${entry.artifact.id}`),entry,cohort,{node18,reviewed});const phase=store.relativeJson(entry.phaseMember);phaseEvidence(phase,entry,cohort,{node18,reviewed});checkStateSelection(store,phase,entry.stateMember);
  const proof=consumerEvidence(state,store,entry.target,cohort,{node18,reviewed,cases});if(node18)checkNode18Floor(state,phase,proof);
  return{target:entry.target,status:'pass',runtimeNode:state.runtimeNode,jobId:entry.jobId,artifactId:entry.artifact.id,stateMember:entry.stateMember,scope:'Original live-job validation plus unchanged archived bytes; copied filesystem inodes are not re-certified'};
}
// Checks shared by workflow and local consumers once their run binding is established.
function consumerEvidence(state,store,target,cohort,{node18=false,reviewed=!node18,cases=PROFILE_CASES.p1,scopes=[]}={}) {
  need(state.automatedStatus==='pass' && state.status===(reviewed?'pass':'automated-pass') && state.phase===(reviewed?'complete':'pending-review') && (!reviewed || state.platformHandoff==='passed'),'Consumer state did not complete its required scope');if(node18)need(state.runtimeNode==='v18.20.8','Consumer did not run the exact Node18 floor');
  need(state.environment?.proof?.rustc==='unresolvable' && state.environment.proof.cargo==='unresolvable' && state.environment.npmChildProof.rustc==='unresolvable' && state.environment.npmChildProof.cargo==='unresolvable','Rust-free public npm environment proof is missing');const npmProof=processReceipt(store,state.npmEnvironment);const proof=JSON.parse(npmProof.stdout);need(proof.rustc==='unresolvable' && proof.cargo==='unresolvable' && (!node18 || proof.node==='v18.20.8'),'Actual npm child environment/floor differs');
  need(state.publicNpm?.kind==='npm-public-bin' && state.publicNpm.bin==='phoenix-diagram' && state.publicNpm.enginePath===state.engine.path && state.publicNpm.tarball===state.candidate.ownedTarball.path && state.publicNpm.cwd===state.project,'Public npm transport/native identity differs');need(state.engine.engineIdentity.sourceInputSha256===cohort.source.inputSha256 && state.engine.identity.sha256===cohort.engines[target].sha256,'Installed native source/target differs');store.bytes(state.engine);need(sha(store.bytes(state.candidate.ownedTarball))===cohort.tgzSha256,'Owned npm tarball differs');
  need(state.online?.status==='pass' && state.online.serverClosed===true && state.online.httpRequests.some(row=>row.target===target && !row.fault && row.sha256===state.engine.identity.sha256 && row.bytes===state.engine.identity.bytes),'Real fixed-port HTTP transfer is missing');processReceipt(store,state.online.receipt);need(state.offlineInstall?.status==='pass' && ['sandbox-exec','network-namespace'].includes(state.offlineInstall.network?.method),'OS offline installation is missing');processReceipt(store,state.offlineInstall.receipt);need(state.rejections?.status==='pass' && state.rejections.checks.length>=4,'Bad-resource rejection scope is missing');for(const check of state.rejections.checks){need(check.status==='pass','A bad-resource scope did not pass');processReceipt(store,check.receipt,{success:check.name==='unsupported-platform'});}
  const doctor=processReceipt(store,state.doctor),architecture=target.endsWith('arm64')?'aarch64':'x86_64';need(doctor.receipt?.engine && same(doctor.receipt.engine,state.engine.engineIdentity) && doctor.receipt.platform?.os===(target.startsWith('darwin')?'macos':'linux') && doctor.receipt.platform.architecture===architecture,'Actual npm doctor engine/native host differs');const core=store.json(state.coreIndex);need(same(core.engine.engineIdentity,state.engine.engineIdentity),'Core engine differs');coreEvidence(core,store,state.engine.engineIdentity,{reviewed,cases,scopes});
  return proof;
}
// The darwin-arm64 full-review consumption runs outside a workflow; its original
// filesystem remains, so its final state and every reference are reread in place.
function localEvidence(entry,cohort,{cases=PROFILE_CASES.p2,scopes=LOCAL_SCOPES}={}) {
  const store=new DirectoryEvidence(entry.runRoot),state=store.json(entry.state);
  need(state.schemaVersion===1 && state.kind==='phoenix-diagram-candidate-consumer' && state.runRoot===entry.runRoot && state.target===entry.target && state.repository===null && state.runId===null,'Local consumer state/run binding differs');
  need(same(state.source,cohort.source) && state.candidate?.tarball?.identity?.sha256===cohort.tgzSha256,'Local consumer candidate/source differs');need(same(sorted(state.scopes || []),sorted(scopes)),'Local consumption did not request the full-review scopes');
  consumerEvidence(state,store,entry.target,cohort,{reviewed:true,cases,scopes});
  return{target:entry.target,status:'pass',runtimeNode:state.runtimeNode,state:entry.state,scopes,scope:'Local original filesystem reread in place: full actual-review loop with Reader exploration/share cards, repository evidence and interface languages'};
}

// These names are this implementation's mapping of §9.2 outcomes, not a new
// prescribed session count. One real fresh session may cover several rows.
export const PROVIDER_SCENARIOS=[...LEGACY_TYPES.map(type=>({id:`intent-${type}`,kind:'delivery',types:[type]})),{id:'mermaid-flowchart',kind:'delivery',types:['workflow','architecture']},{id:'mermaid-graph',kind:'delivery',types:['workflow','architecture']},{id:'mermaid-sequence',kind:'delivery',types:['sequence']},{id:'mermaid-state',kind:'delivery',types:['lifecycle']},...LEGACY_TYPES.map(type=>({id:`legacy-${type}`,kind:'delivery',types:[type],contract:'legacy-2.17'})),...['date','amount','condition'].map(field=>({id:`missing-${field}`,kind:'clarification',missing:field})),{id:'repairable-diagnostic',kind:'repair'},{id:'stalled-repair',kind:'stall'},{id:'browser-unavailable',kind:'browser-failure',expected:'unavailable'},{id:'browser-failed',kind:'browser-failure',expected:'failed'},{id:'visual-pending',kind:'visual-pending'}];
// P2 rows: Claude Code runs every row; Codex runs the new rows plus three regression samples.
const P2_NEW_ROWS=[...['erd','class','tree','timeline','waterfall'].map(type=>({id:`intent-${type}`,kind:'delivery',types:[type]})),{id:'mermaid-er',kind:'delivery',types:['erd']},{id:'mermaid-class',kind:'delivery',types:['class']},{id:'repo-verified',kind:'delivery',evidence:'verified'},{id:'repo-dirty',kind:'delivery',evidence:'worktree-differs'},{id:'repo-timeline',kind:'delivery',types:['timeline'],evidence:'verified',timing:'day'},{id:'waterfall-illustrative',kind:'delivery',types:['waterfall'],timing:'illustrative'},{id:'reference-31-json',kind:'delivery',types:['workflow'],contract:'phoenix-v1',unchangedOriginal:true},{id:'locale-zh-tw',kind:'delivery',locale:'zh-TW'}];
const P2_REGRESSION_ROWS=[...PROVIDER_SCENARIOS.filter(row=>!row.id.startsWith('legacy-')),PROVIDER_SCENARIOS.find(row=>row.id==='legacy-lifecycle')];
export const P2_PROVIDER_SCENARIOS={'claude-code':[...P2_REGRESSION_ROWS,...P2_NEW_ROWS],codex:[...P2_REGRESSION_ROWS.filter(row=>['intent-workflow','mermaid-sequence','stalled-repair'].includes(row.id)),...P2_NEW_ROWS]};
// D10: once any Codex row diverges from the same Claude Code row, Codex also runs
// every regression row, so its record covers either the sample or all Claude Code rows.
export function p2ProviderScenarios(provider,rows) {
  if(provider!=='codex')return P2_PROVIDER_SCENARIOS[provider];
  return Array.isArray(rows) && rows.length>P2_PROVIDER_SCENARIOS.codex.length?P2_PROVIDER_SCENARIOS['claude-code']:P2_PROVIDER_SCENARIOS.codex;
}
export function checkProviderCoverage(rows,scenarios=PROVIDER_SCENARIOS) {need(Array.isArray(rows) && new Set(rows.map(row=>row.id)).size===rows.length && scenarios.every(required=>rows.some(row=>row.id===required.id)) && rows.every(row=>scenarios.some(required=>required.id===row.id)),'Provider scenario outcome coverage is incomplete or generic');}
function text(value) {if(typeof value==='string')return value;if(Array.isArray(value))return value.map(text).join('\n');return object(value)?Object.values(value).map(text).join('\n'):'';}
function eventPayload(event) {return event.type==='response_item'?event.payload:event.item || event;}
function calls(event) {
  if(event.type==='assistant')return(event.message?.content || []).filter(row=>row.type==='tool_use').map(row=>({id:row.id,name:row.name,input:row.input}));
  const payload=eventPayload(event);if(payload?.type==='function_call'){let input;try{input=JSON.parse(payload.arguments);}catch{return[];}return[{id:payload.call_id,name:payload.name || '<missing>',input}];}
  return event.type==='item.started' && ['command_execution','mcp_tool_call'].includes(payload?.type)?[{id:payload.id,input:{command:payload.command}}]:[];
}
function commandOf(call) {return call.name!==undefined && !['Bash','bash','exec_command','functions.exec_command'].includes(call.name)?undefined:call.input?.cmd ?? call.input?.command;}
function toolEvent(event) {return calls(event).length>0;}
function toolCallText(event) {return calls(event).map(call=>text(commandOf(call) ?? call.input)).join('\n');}
function pairPayload(call,result,toolUseId) {
  const candidates=calls(call).filter(value=>toolUseId===undefined || value.id===toolUseId);if(candidates.length!==1)return null;const selected=candidates[0];if(typeof selected.id!=='string' || !selected.id)return null;
  if(call.type==='assistant') {
    const found=result.type==='user'?(result.message?.content || []).filter(row=>row.type==='tool_result' && row.tool_use_id===selected.id):[];if(found.length!==1)return null;
    return{...selected,output:text(found[0].content),error:found[0].is_error===true,errorFlag:found[0].is_error};
  }
  const right=eventPayload(result);
  if(eventPayload(call).type==='function_call') {
    if(right?.type!=='function_call_output' || right.call_id!==selected.id)return null;
    let output=right.output,exitCode;
    if(typeof output==='string' && output.startsWith('Chunk ID:')) {
      const wrapped=/^Chunk ID: [a-f0-9]{6}\nWall time: \d+(?:\.\d+)? seconds\nProcess exited with code (\d{1,3})\nOriginal token count: \d+\nOutput:\n([\s\S]*)$/.exec(output);
      need(wrapped && Number(wrapped[1])<=255,'Unsupported native process exit wrapper');exitCode=Number(wrapped[1]);output=wrapped[2];
    } else try{const wrapped=JSON.parse(output);if(object(wrapped) && typeof wrapped.output==='string'){output=wrapped.output;exitCode=wrapped.exit_code??wrapped.exitCode;}}catch{}
    return{...selected,output:text(output),exitCode,error:typeof exitCode==='number' && exitCode!==0};
  }
  if(result.type!=='item.completed' || right?.id!==selected.id || !['command_execution','mcp_tool_call'].includes(right.type))return null;
  return{...selected,output:text(right.aggregated_output ?? right.output ?? ''),exitCode:right.exit_code,error:right.status==='failed' || typeof right.exit_code==='number' && right.exit_code!==0};
}
function literalWords(command) {
  if(Array.isArray(command)){need(command.every(value=>typeof value==='string'),'Unsupported execution argv');return command;}
  need(typeof command==='string' && command && !/[\0\r\n]/.test(command),'Unsupported execution command');const words=[];let word='',quote=null,started=false;
  for(let at=0;at<command.length;at++) {
    const char=command[at];if(quote){if(char===quote)quote=null;else{need(quote!== '"' || !/[$`\\]/.test(char),'Unsupported shell expansion');word+=char;}continue;}
    if(/\s/.test(char)){if(started){words.push(word);word='';started=false;}continue;}
    started=true;if(char==="'" || char==='"')quote=char;
    else if(char==='\\'){need(at+1<command.length,'Incomplete shell escape');word+=command[++at];}
    else{need(!/[;&|<>`$(){}*?~#]/.test(char),'Unsupported shell operation');word+=char;}
  }
  need(!quote,'Unterminated execution quote');if(started)words.push(word);return words;
}
// Codex unified exec records `<shell> -c "<literal>"`; the literal under that one
// layer must name the collector or PD entry. Anything else is checked as recorded.
function executedWords(command) {const words=literalWords(command);return words.length===3 && /^\/bin\/(?:zsh|bash|sh)$/.test(words[0]) && words[1]==='-c'?literalWords(words[2]):words;}
const PD_COMMANDS=['engine','doctor','guide','validate','deliver','check','browser-check','visual-check','finalize','review-record','recover-output'];
function directPd(pair,session,raw) {
  let command=commandOf(pair);
  if(typeof command==='string' && /^\s*(?:"\$PD"|\$PD)(?=\s|$)/.test(command)) {
    const setup=session.pdSetup && toolCallText(anchor(raw,session.pdSetup)),match=/^(?:export\s+)?PD=(?:'([^']+)'|"([^"$`]+)"|(\/[^\s;$`]+))$/.exec(setup || '');need(match,'PD variable lacks a literal setup');const program=match[1]??match[2]??match[3];need(path.isAbsolute(program) && !program.includes("'"),'Unsupported PD setup');command=command.replace(/^\s*(?:"\$PD"|\$PD)/,`'${program}'`);
  }
  const words=executedWords(command);let program=words.shift();if(program==='node' || /\/node$/.test(program || '')){need(/\/bin\/phoenix-diagram\.js$/.test(words[0] || ''),'Wrong PD Node entry');program=words.shift();}
  need(program==='phoenix-diagram' || /\/(?:scripts\/phoenix-diagram|bin\/phoenix-diagram\.js)$/.test(program || ''),'Actual execution is not the PD entry');need(PD_COMMANDS.includes(words[0]),'Wrong PD command');return{program,argv:words};
}
function proofPair(capture,proof) {
  if(Object.hasOwn(proof,'polls'))return asyncCollectorPair(capture,proof);
  const resultRef=proof.result || proof,result=anchor(capture.raw,resultRef);let choices;
  if(proof.call){need(proof.call.offset<resultRef.offset,'Tool result precedes its call');choices=[{event:anchor(capture.raw,proof.call),ref:proof.call}];}
  else choices=capture.events.filter(toolEvent).filter(event=>capture.events.indexOf(event)<capture.events.indexOf(capture.events.find(value=>same(value,result)))).map(event=>({event}));
  const pairs=choices.map(({event})=>pairPayload(event,result,proof.toolUseId)).filter(Boolean);need(pairs.length===1,'Actual tool call/result ID pair is missing or ambiguous');return pairs[0];
}
function asyncCollectorPair(capture,proof) {
  need(capture.provider==='codex' && Array.isArray(proof.polls) && proof.polls.length>0,'Async collector requires explicit Codex process polls');
  const callsById=new Map(),resultsById=new Map();
  for(const event of capture.events)if(event.type==='response_item') {
    const payload=event.payload || {},map=payload.type==='function_call'?callsById:payload.type==='function_call_output'?resultsById:null;
    if(map){need(typeof payload.call_id==='string' && payload.call_id && !map.has(payload.call_id),'Async collector native tool IDs are missing or duplicated');map.set(payload.call_id,event);}
  }
  const execNames=['exec_command','functions.exec_command'],pollNames=['write_stdin','functions.write_stdin'];
  const running=/^Chunk ID: [a-f0-9]{6}\nWall time: \d+(?:\.\d+)? seconds\nProcess running with session ID ([1-9]\d*)\nOriginal token count: \d+\nOutput:\n([\s\S]*)$/;
  const exited=/^Chunk ID: [a-f0-9]{6}\nWall time: \d+(?:\.\d+)? seconds\nProcess exited with code (\d{1,3})\nOriginal token count: \d+\nOutput:\n([\s\S]*)$/;
  function nativePair(value) {
    need(value?.call && value.result && typeof value.toolUseId==='string' && !Object.hasOwn(value,'polls'),'Async collector requires each original call/result ID pair');
    need(value.call.offset<value.result.offset,'Async collector native result precedes its call');
    const call=anchor(capture.raw,value.call),result=anchor(capture.raw,value.result),selected=calls(call);
    need(call.type==='response_item' && result.type==='response_item' && selected.length===1 && selected[0].id===value.toolUseId
      && result.payload?.type==='function_call_output' && result.payload.call_id===value.toolUseId
      && same(callsById.get(value.toolUseId),call) && same(resultsById.get(value.toolUseId),result),'Async collector actual tool call/result ID pair differs');
    const output=result.payload.output,live=typeof output==='string' && running.exec(output),done=typeof output==='string' && exited.exec(output);
    need(live || done && Number(done[1])<=255,'Async collector has an unsupported native process wrapper');
    if(live)need(Number.isSafeInteger(Number(live[1])),'Async collector session ID is not an exact integer');
    return{...selected[0],output:(live || done)[2],running:!!live,sessionId:live?Number(live[1]):null,exitCode:done?Number(done[1]):undefined};
  }
  const {polls:pollProofs,...producerProof}=proof,producer=nativePair(producerProof);
  need(execNames.includes(producer.name) && producer.running && !producer.output.trim(),'Async collector producer must yield one empty running result');
  const collector=capture.session.collector,words=literalWords(commandOf(producer));
  need(collector && words[0]===collector.nodePath && words[1]===collector.script.path,'Async collector requires the fixed literal collector producer');
  const sessionId=producer.sessionId,owners=[],actualPolls=[];
  for(const [id,event] of callsById) {
    const call=calls(event)[0];
    if(execNames.includes(call?.name)) {
      const output=resultsById.get(id)?.payload?.output,match=typeof output==='string' && running.exec(output);
      if(match && Number(match[1])===sessionId)owners.push(id);
    }
    if(pollNames.includes(call?.name) && call.input?.session_id===sessionId)actualPolls.push(id);
  }
  need(same(owners,[producer.id]),'Async collector process session has another producer or a session collision');
  need(same(actualPolls,pollProofs.map(value=>value.toolUseId)),'Async collector polls are missing, duplicated or outside the original process chain');
  const expectedName=producer.name==='exec_command'?'write_stdin':'functions.write_stdin',polls=[];let previous=proof.result;
  for(const [index,value] of pollProofs.entries()) {
    need(value.call?.offset>=previous.offset+previous.length,'Async collector process polls overlap or precede their original result');
    const poll=nativePair(value);
    need(poll.name===expectedName && poll.input?.session_id===sessionId && (!Object.hasOwn(poll.input,'chars') || poll.input.chars===''),'Async collector poll actor/session differs or writes stdin');
    if(index<pollProofs.length-1)need(poll.running && poll.sessionId===sessionId && !poll.output.trim(),'Async collector intermediate poll changed session or emitted output');
    else need(!poll.running,'Async collector terminal poll has no native exit');
    polls.push(poll);previous=value.result;
  }
  const terminal=polls.at(-1);
  // This is a verified producer view, not a fabricated cross-ID native pair:
  // the durable proof retains the producer result and every poll's own ID.
  return{id:producer.id,name:producer.name,input:producer.input,output:terminal.output,exitCode:terminal.exitCode,error:terminal.exitCode!==0,producer,polls};
}
function metadataLine(output) {const rows=output.split('\n').flatMap(line=>{try{const value=JSON.parse(line);return value?.kind==='phoenix-diagram-stdio-capture'?[value]:[];}catch{return[];}});need(rows.length===1,'Missing or ambiguous collector result');return rows[0];}
function capturedReply(capture,proof,bytesFor) {
  const pair=proofPair(capture,proof),dto=metadataLine(pair.output),collector=capture.session.collector;
  need(proof.call && collector && capture.collectorIdentity,'Authenticated collector template and exact call are required');
  need(same(identity(bytesFor(collector.script)),capture.collectorIdentity) && same(collector.script.identity,capture.collectorIdentity),'Collector template differs from authenticated harness');
  const words=executedWords(commandOf(pair));need(words[0]===collector.nodePath && words[1]===collector.script.path,'Wrong collector producer or Node entry');
  const options=parseCaptureArgs(words.slice(2));need(dto.schemaVersion===1 && dto.tool==='phoenix-diagram' && same(dto.collector?.identity,capture.collectorIdentity) && dto.collector.path===collector.script.path && dto.node?.path===collector.nodePath && /^v\d+\./.test(dto.node.version || '') && dto.program===options.program && dto.cwd===options.cwd && same(dto.argv,options.argv) && dto.stdout?.path===options.stdout && dto.stderr?.path===options.stderr,'Collector command/argv/file binding differs');
  need(Number.isInteger(dto.exitCode) && dto.exitCode>=0 && dto.exitCode<=255 && dto.signal===null && (pair.exitCode===undefined?capture.provider==='claude-code':pair.exitCode===dto.exitCode) && (pair.errorFlag===undefined || pair.errorFlag===(dto.exitCode!==0)),'Native capture exit or signal is inconsistent');
  need(dto.cwd===capture.capture.cwd && dto.installedReceipt?.path===capture.session.installedReceipt.path && same(dto.installedReceipt.identity,capture.session.installedReceipt.identity),'Collector installation binding differs');
  const installed=JSON.parse(digest(bytesFor(dto.installedReceipt),dto.installedReceipt.identity,'Installed receipt')),relative=path.posix.relative(dto.cwd,dto.program),owned=installed.ownership?.find(value=>value.path===relative);
  need(/^\.(?:agents|claude)\/skills\/phoenix-diagram\/scripts\/phoenix-diagram$/.test(relative) && dto.launcher?.path===dto.program && owned && owned.identity.sha256===dto.launcher.identity.sha256 && owned.identity.bytes===dto.launcher.identity.bytes,'Collector did not execute the owned installed launcher');
  digest(bytesFor(dto.launcher),dto.launcher.identity,'Owned launcher');digest(bytesFor(dto.stdout),dto.stdout.identity,'Original stdout');digest(bytesFor(dto.stderr),dto.stderr.identity,'Original stderr');
  if(dto.launcher.binding)need(Object.entries(dto.launcher.binding).every(([key,value])=>owned.identity[key]===value),'Executed launcher original ownership binding differs');
  if(capture.tarInventory){const expected=capture.tarInventory.get('skills/phoenix-diagram/scripts/phoenix-diagram');need(expected && dto.launcher.identity.sha256===expected.sha256 && dto.launcher.identity.bytes===expected.size,'Executed launcher differs from the candidate');}
  return{dto,pair};
}
function anchor(bytes,value) {need(Number.isSafeInteger(value?.offset) && Number.isSafeInteger(value.length) && value.offset>=0 && value.length>0 && value.offset+value.length<=bytes.length,'Native transcript event anchor is missing');const selected=bytes.subarray(value.offset,value.offset+value.length);need(sha(selected)===value.sha256 && (value.offset===0 || bytes[value.offset-1]===10) && (value.offset+value.length===bytes.length || bytes[value.offset+value.length-1]===10),'Native transcript event byte anchor differs');return JSON.parse(selected.toString('utf8'));}
function injectedSkillBody(captured,read,source,pair) {
  // The observed host omits YAML and its following blank line, retaining the
  // complete remaining body, including its final newline.
  const body=/^---\n[\s\S]*?\n---\n\n([\s\S]*)$/.exec(source)?.[1];
  // Bind literal tool arguments without treating them as confirmed user facts.
  const args=pair.input?.args,withArgs=typeof args==='string' && args.length>0;
  const expectedInput=withArgs?{skill:'phoenix-diagram',args}:{skill:'phoenix-diagram'},suffix=withArgs?'\n\nARGUMENTS: '+args:'';
  need(captured.provider==='claude-code' && pair.name==='Skill' && same(pair.input,expectedInput) && read.file.path===path.posix.join(captured.capture.cwd,'.claude/skills/phoenix-diagram/SKILL.md') && typeof body==='string' && read.call && read.result && read.injection && read.result.offset+read.result.length===read.injection.offset,'Native Skill injection is missing, outside the current project, or not adjacent to its result');
  const call=anchor(captured.raw,read.call),result=anchor(captured.raw,read.result),injection=anchor(captured.raw,read.injection);
  need(call.type==='assistant' && call.message?.role==='assistant' && result.type==='user' && result.message?.role==='user' && result.message.content?.length===1 && result.tool_use_result?.success===true && result.tool_use_result.commandName==='phoenix-diagram' && !pair.error && pair.output==='Launching skill: phoenix-diagram' && [call,result,injection].every(event=>event.session_id===captured.session.id) && injection.type==='user' && injection.message?.role==='user' && injection.isSynthetic===true && injection.message.content?.length===1 && injection.message.content[0].type==='text' && injection.message.content[0].text===`Base directory for this skill: ${path.posix.dirname(read.file.path)}\n\n${body}${suffix}`,'Native Skill injection does not contain this complete owned SKILL body');
  return body;
}
// P2 skill text sends repair and stall rounds through SKILL.md alone, so under p2 those
// sessions, like clarification-only ones, need only the verified SKILL read.
export function checkProviderCapture(session,bytesFor,provider=session?.provider,{collectorIdentity,tarInventory,scenarioRows,profile='p1'}={}) {
  const scopedTo=kinds=>Array.isArray(scenarioRows) && scenarioRows.length>0 && scenarioRows.every(row=>row.sessionId===session?.id && PROVIDER_SCENARIOS.some(required=>required.id===row.id && kinds.includes(required.kind)));
  const clarificationOnly=scopedTo(['clarification']),skillOnly=clarificationOnly || profile==='p2' && scopedTo(['clarification','stall','repair']);
  need(['claude-code','codex'].includes(provider) && session?.capture && session.transcript && session.version && session.installedReceipt && session.loadedReads?.length>=(skillOnly?1:2) && (clarificationOnly || session.pdCall),'Real provider capture/session/transcript evidence is missing');
  const capture=JSON.parse(bytesFor(session.capture)),version=JSON.parse(bytesFor(session.version)),raw=bytesFor(session.transcript),events=raw.toString('utf8').split('\n').filter(Boolean).map(line=>JSON.parse(line));
  const formats=provider==='claude-code'?['claude-stream-json']:['codex-exec-json','codex-session-json'];need(formats.includes(session.format),'Unknown native provider transcript format');need(session.format==='codex-session-json'?capture.nativeSessionFile===session.transcript.path || capture.transcriptSource?.path===capture.nativeSessionFile && same(capture.transcriptSource.identity,session.transcript.identity):same(capture.stdout,session.transcript),'Provider transcript is not bound to the actual stdout/native session file');
  need(capture.exitCode===0 && version.exitCode===0 && typeof capture.program==='string' && capture.program.trim() && capture.program===version.program && Array.isArray(capture.argv) && capture.argv.every(value=>typeof value==='string') && !capture.argv.some(value=>value==='resume' || (provider==='claude-code'?/^(?:--resume|--continue|-r|-c)(?:=|$)/:/^(?:--resume|--continue)(?:=|$)/).test(value)) && version.argv?.includes('--version'),'Provider CLI/version capture is missing or resumed');need((provider==='codex'?/codex/i:/claude/i).test(version.stdout || ''),'Observed provider version does not identify the requested client');
  const start=provider==='claude-code'?events.find(event=>event.type==='system' && event.subtype==='init'):events.find(event=>event.type==='thread.started' || event.type==='session_meta');const id=start?.session_id || start?.thread_id || start?.payload?.id;
  need(start && typeof session.id==='string' && session.id.trim() && id===session.id && events.some(event=>provider==='claude-code'?event.type==='result' && event.is_error===false && event.session_id===session.id:event.type==='turn.completed' || event.type==='event_msg' && event.payload?.type==='task_complete'),'Real native provider session start/completion is missing');
  need(capture.cwd && typeof capture.cwd==='string' && capture.cwd.startsWith('/') && !events.filter(toolEvent).some(event=>/\/renderers\/|\/tools\/phoenix-diagram\/src\/|\/test\/phoenix-diagram\//.test(toolCallText(event))),'Provider accessed implementation/renderer/test material');
  const captured={session,provider,raw,events,capture,version,collectorIdentity,tarInventory};
  if(session.collector)need(collectorIdentity && same(session.collector.script?.identity,collectorIdentity) && same(identity(bytesFor(session.collector.script)),collectorIdentity) && path.isAbsolute(session.collector.nodePath) && path.basename(session.collector.nodePath)==='node','Collector template lacks authenticated harness identity or an explicit Node entry');
  for(const read of session.loadedReads){const pair=proofPair(captured,read),source=bytesFor(read.file).toString('utf8'),injected=read.injection!==undefined || pair.name==='Skill',output=injected?injectedSkillBody(captured,read,source,pair):pair.output;need(!pair.error && (pair.exitCode===undefined || pair.exitCode===0) && (injected || text(pair.input).includes(read.file.path)) && typeof read.quote==='string' && read.quote.length>=40 && source.includes(read.quote) && output.includes(read.quote),'Installed skill/reference tool call/result pair did not actually read the owned file');}
  need(session.loadedReads.some(read=>/\/skills\/phoenix-diagram\/SKILL.md$/.test(read.file.path)) && (skillOnly || session.loadedReads.some(read=>/\/skills\/phoenix-diagram\/references\/[^/]+\.md$/.test(read.file.path))),'Actual installed SKILL and required reference reads are missing');
  if(session.pdCall) {
    const pair=proofPair(captured,session.pdCall);
    if(pair.output.includes('phoenix-diagram-stdio-capture')){const {dto}=capturedReply(captured,session.pdCall,bytesFor);need(JSON.parse(bytesFor(dto.stdout)).tool==='phoenix-diagram','Collector stdout is not a PD reply');}
    else {directPd(pair,session,raw);need(!pair.error && /"tool"\s*:\s*"phoenix-diagram"/.test(pair.output),'Installed PD entry execution tool call/result pair is missing');}
  }
  return captured;
}
export function checkProviderReply(capture,proof,reference,bytesFor,{outcome='success'}={}) {
  const bytes=digest(bytesFor(reference),reference.identity,'Actual CLI stdout'),receipt=JSON.parse(bytes.toString('utf8')),pair=proofPair(capture,proof);let argv,exitCode;
  if(pair.output.includes('phoenix-diagram-stdio-capture')) {
    const {dto}=capturedReply(capture,proof,bytesFor);need(dto.stdout.path===reference.path && same(dto.stdout.identity,reference.identity),'Collector stdout is not this original reply');argv=dto.argv;exitCode=dto.exitCode;
  } else {argv=directPd(pair,capture.session,capture.raw).argv;need(pair.output.includes(bytes.toString('utf8').trim()),'Actual PD execution/result does not contain this exact CLI reply');exitCode=pair.exitCode??(pair.error?1:0);}
  need(PD_COMMANDS.includes(argv[0]) && receipt.command===argv[0],'Actual PD argv names a different reply command');
  const namesPath=(argument,expected)=>typeof argument==='string' && path.posix.resolve(capture.capture.cwd,argument)===expected;
  if(['validate','deliver','finalize'].includes(argv[0]))need(TYPES.includes(argv[1]) && (!receipt.candidate || namesPath(argv[2],receipt.candidate.path)) && (!['deliver','finalize'].includes(argv[0]) || !receipt.artifact?.path || namesPath(argv[3],receipt.artifact.path)),'Actual PD argv/input/output differs from its reply');
  else if(['check','browser-check','visual-check','review-record'].includes(argv[0]))need(!receipt.artifact?.path || namesPath(argv[1],receipt.artifact.path),'Actual PD argv names another artifact');
  need(outcome==='failure'?exitCode!==0 && receipt.status!=='pass':outcome==='success' && exitCode===0 && !pair.error && receipt.status==='pass','Native process exit does not match the expected success/failure outcome');return receipt;
}
function actualReply(capture,proof,reference,store,outcome='success') {return checkProviderReply(capture,proof,reference,value=>store.bytes(value),{outcome});}
function assistantText(event,provider) {const payload=eventPayload(event);need(provider==='claude-code'?event.type==='assistant' && event.message?.role==='assistant':payload?.type==='agent_message' || payload?.type==='message' && payload.role==='assistant' || event.type==='event_msg' && payload.type==='task_complete','Scenario response is not a native assistant message');return text(event);}
function receiptErrors(receipt) {const errors=receipt.composition?.summary?.errors;return Number.isSafeInteger(errors)?errors:(receipt.diagnostics || []).filter(row=>row.severity==='error').length;}
// P2 host sessions run on the authenticated final candidate itself, or on a development
// package that counts only when its bundle and engine source are the same (bundle
// manifest excludes ENGINE.json).
export function checkDevelopmentPackage(version,cohort,{record,candidate,bytesFor}={}) {
  const sameCandidate=version?.distribution==='candidate' || record && HASH.test(cohort.tgzSha256 || '') && (record.candidateSha256===cohort.tgzSha256 || record.package?.tarball?.identity?.sha256===cohort.tgzSha256);
  if(sameCandidate) {
    // Only aggregateCandidate's authenticated validateCandidate result supplies
    // these snapshots; provider source/bundle assertions cannot substitute them.
    const manifest=candidate?.files?.get('release-dir/VERSION.json'),tgz=candidate?.tgz;
    need(record?.package?.version && record.package.tarball && manifest && tgz && typeof bytesFor==='function' && Buffer.isBuffer(manifest.content) && Buffer.isBuffer(tgz.content),'Same-candidate provider package lacks its authenticated candidate VERSION/tgz');
    need(same(identity(manifest.content),manifest.identity) && same(identity(tgz.content),tgz.identity) && manifest.identity.sha256===cohort.versionSha256 && tgz.identity.sha256===cohort.tgzSha256,'Authenticated candidate VERSION/tgz cohort differs');
    need(version.distribution==='candidate' && same(version,candidate.version) && same(version,JSON.parse(manifest.content.toString('utf8'))) && same(candidate.source,cohort.source) && version.sourceCommit===cohort.source.commit && version.sourceInputSha256===cohort.source.inputSha256 && version.bundleSha256===cohort.bundleSha256 && version.npmPackageVersion===cohort.npmPackageVersion && same(sorted(Object.keys(version.engines || {})),sorted(TARGETS)) && same(version.engines,cohort.engines),'Same-candidate provider VERSION/source/complete engines differ');
    need(record.candidateSha256===cohort.tgzSha256 && same(record.source,cohort.source) && same(record.package.version.identity,manifest.identity) && same(record.package.tarball.identity,tgz.identity),'Same-candidate provider source/VERSION/tgz identity differs');
    need(bytesFor(record.package.version).equals(manifest.content) && bytesFor(record.package.tarball).equals(tgz.content),'Same-candidate provider VERSION/tgz bytes differ');
    return version;
  }
  need(version?.schemaVersion===1 && version.distribution==='development' && version.tool==='phoenix-diagram' && version.toolVersion==='0.2.0' && version.npmPackageVersion===cohort.npmPackageVersion && version.sourceInputSha256===cohort.source.inputSha256 && version.bundleSha256===cohort.bundleSha256 && object(version.engines) && Object.keys(version.engines).length>0,'Development package bundle or engine source differs from the final candidate');
  return version;
}
// §9.3 reuse: rows recorded on an earlier development package of the same bundle
// still count once the final engine is shown to reproduce what that row's engine
// produced: the same diagram SVG for each delivery and the same error codes for
// each failed validation. Rows whose output changed must be rerun.
export function checkReusedPackage(version,cohort) {
  need(version?.schemaVersion===1 && version.distribution==='development' && version.tool==='phoenix-diagram' && version.toolVersion==='0.2.0' && version.npmPackageVersion===cohort.npmPackageVersion && version.bundleSha256===cohort.bundleSha256 && HASH.test(version.sourceInputSha256 || '') && version.sourceInputSha256!==cohort.source.inputSha256 && object(version.engines) && Object.keys(version.engines).length>0,'Reused development package differs from the final bundle or is not an earlier engine');
  return version;
}
export function p2ReuseRows(provider,current,earlier) {
  const scenarios=p2ProviderScenarios(provider,earlier);checkProviderCoverage(earlier,scenarios);
  need(Array.isArray(current) && current.length>0 && new Set(current.map(row=>row.id)).size===current.length && current.every(row=>scenarios.some(required=>required.id===row.id)),'Rerun host rows are missing, duplicated or outside the planned scope');
  return{scenarios,reused:scenarios.map(required=>required.id).filter(id=>!current.some(row=>row.id===id))};
}
export function diagramSvg(html) {const start=html.indexOf('<svg'),end=html.indexOf('</svg>');need(start>=0 && end>start && html.indexOf('<svg',start+1)<0 && html.indexOf('</svg>',end+1)<0,'Artifact has no single diagram SVG');return html.slice(start,end+6);}
const errorCodes=receipt=>sorted((receipt.diagnostics || []).filter(row=>row.severity==='error').map(row=>row.code));
function failedValidation(receipt) {const stage=receipt.command==='finalize'?receipt.stages?.find(value=>value.name==='validate')?.receipt:receipt;need(stage?.command==='validate' && stage.status!=='pass','Only a failed validation can be replayed; rerun this host row');return{command:'validate',candidate:stage.candidate?.identity,codes:errorCodes(stage),config:stage.config ?? null};}
function delivered(receipt) {const stage=receipt.command==='finalize'?receipt.stages?.find(value=>value.name==='deliver')?.receipt:receipt;need(stage?.command==='deliver' && stage.status==='pass','Only a passed delivery can be replayed; rerun this host row');return{command:'deliver',candidate:stage.candidate?.identity,artifact:stage.artifact?.identity,config:stage.config};}
export function checkReplay(expected,row,cohort,store) {
  need(Array.isArray(row?.replays) && row.replays.length===expected.length,`Host row ${row?.id} lacks exactly one final-engine replay per original output`);
  const unused=[...row.replays];
  for(const item of expected) {
    const at=unused.findIndex(replay=>replay.command===item.command && (item.command!=='deliver' || same(replay.original?.identity,item.artifact)));need(at>=0,`Host row ${row.id} lacks a final-engine replay of an original ${item.command}`);
    const replay=unused.splice(at,1)[0],receipt=store.json(replay.receipt);header(receipt,item.command);
    need(receipt.engine?.sourceInputSha256===cohort.source.inputSha256 && Object.values(cohort.engines).some(engine=>same(receipt.engine.binary,{sha256:engine.sha256,bytes:engine.size})) && same(receipt.candidate?.identity,item.candidate),'Replay was not produced by the final engine from the original input');
    need(same(receipt.config ?? null,item.config ?? null),'Replay configuration differs from the original');
    if(item.command==='deliver'){need(receipt.status==='pass' && same(replay.artifact?.identity,receipt.artifact?.identity),'Final-engine delivery replay did not pass');need(diagramSvg(store.bytes(replay.artifact).toString('utf8'))===diagramSvg(store.bytes(replay.original).toString('utf8')),`The final engine's diagram differs for host row ${row.id}; rerun it`);}
    else{need(receipt.status!=='pass' && item.codes.length>0,'Final-engine validation replay no longer fails');need(same(errorCodes(receipt),item.codes),`The final engine's error diagnostics differ for host row ${row.id}; rerun it`);}
  }
}
function providerEvidence(entry,cohort,tarInventory,{profile='p1',candidate}={}) {
  if(profile!=='p2' || entry.reuse===undefined)return recordEvidence(entry,entry.provider,cohort,tarInventory,{profile,candidate});
  const reuse=entry.reuse;need(object(reuse) && object(reuse.replay),'Reused host rows need their earlier record and a final-engine replay');
  const current=new ArchiveEvidence(entry.archive,entry.prefix,entry.originalRoot).relativeJson(entry.recordMember),earlier=new ArchiveEvidence(reuse.archive,reuse.prefix,reuse.originalRoot).relativeJson(reuse.recordMember);
  const {scenarios,reused}=p2ReuseRows(entry.provider,current.scenarios,earlier.scenarios),store=new ArchiveEvidence(reuse.replay.archive,reuse.replay.prefix,reuse.replay.originalRoot),replays=store.relativeJson(reuse.replay.member);
  need(replays?.schemaVersion===1 && replays.kind==='phoenix-diagram-host-replay' && replays.provider===entry.provider && Array.isArray(replays.rows) && new Set(replays.rows.map(row=>row.id)).size===replays.rows.length,'Final-engine host replay record differs');
  const rerun=recordEvidence(entry,entry.provider,cohort,tarInventory,{profile,candidate,scenarios,rows:current.scenarios.map(row=>row.id)}),kept=recordEvidence(reuse,entry.provider,cohort,tarInventory,{profile,candidate,scenarios,rows:reused,replay:{store,rows:replays.rows}});
  return{provider:entry.provider,status:'pass',sessionIds:[...rerun.sessionIds,...kept.sessionIds],scenarioIds:scenarios.map(row=>row.id),rerunScenarioIds:rerun.scenarioIds,reused:{engineSource:kept.engineSource,scenarioIds:kept.scenarioIds},scope:`${rerun.scope}; rows kept from an earlier engine of the same bundle only where the final engine reproduces their diagrams and failed validations`};
}
function recordEvidence(entry,provider,cohort,tarInventory,{profile='p1',scenarios,rows,replay,candidate}={}) {
  const store=new ArchiveEvidence(entry.archive,entry.prefix,entry.originalRoot),record=store.relativeJson(entry.recordMember);scenarios??=profile==='p2'?p2ProviderScenarios(provider,record.scenarios):PROVIDER_SCENARIOS;
  need(record.schemaVersion===1 && record.kind==='phoenix-diagram-provider-acceptance' && record.provider===provider && scenarios,'Provider record kind/provider differs');
  let development;
  if(profile==='p2'){need(record.profile==='p2' && record.package?.version,'P2 provider record lacks its package VERSION');const version=store.json(record.package.version);development={version:replay?checkReusedPackage(version,cohort):checkDevelopmentPackage(version,cohort,{record,candidate,bytesFor:reference=>store.bytes(reference)}),sha256:record.package.version.identity.sha256};}
  else need(record.repository===cohort.repository && String(record.runId)===String(cohort.runId) && Number(record.runAttempt)===cohort.runAttempt && record.candidateSha256===cohort.tgzSha256 && same(record.source,cohort.source),'Provider candidate/source/run binding differs');
  const engines=Object.values(development?.version.engines || cohort.engines),manifestSha256=development?.sha256 || cohort.versionSha256,source=development?.version.sourceInputSha256 ?? cohort.source.inputSha256;
  if(!rows)checkProviderCoverage(record.scenarios,scenarios);need(record.sessions?.length>0 && new Set(record.sessions.map(row=>row.id)).size===record.sessions.length,'Real fresh provider sessions are missing/duplicated');
  const sessions=new Map();for(const session of record.sessions) {
    const scenarioRows=record.scenarios.filter(row=>row.sessionId===session.id);
    const captured=checkProviderCapture(session,reference=>store.bytes(reference),provider,{collectorIdentity:cohort.collectorIdentity,tarInventory,scenarioRows,profile}),installed=store.json(session.installedReceipt);
    need(installed.schemaVersion===1 && installed.tool==='phoenix-diagram' && installed.npmPackageVersion===cohort.npmPackageVersion && installed.engine?.sourceInputSha256===source && engines.some(engine=>same(installed.engine.binary,{sha256:engine.sha256,bytes:engine.size})) && installed.bundleManifestSha256===cohort.bundleSha256 && installed.engineManifestSha256===manifestSha256,'Actual project installation receipt differs from this npm package');
    for(const read of session.loadedReads){const suffix=/\/skills\/phoenix-diagram\/(.+)$/.exec(read.file.path)?.[1],expected=tarInventory.get(`skills/phoenix-diagram/${suffix}`);need(expected && same(identity(store.bytes(read.file)),{sha256:expected.sha256,bytes:expected.size}) && installed.ownership?.some(owned=>read.file.path.endsWith(owned.path) && owned.identity.sha256===expected.sha256 && owned.identity.bytes===expected.size),'Loaded installed resource is not owned by this candidate');}
    sessions.set(session.id,{session,...captured,installedEngine:installed.engine});
  }
  for(const row of record.scenarios.filter(value=>!rows || rows.includes(value.id))) {
    const required=scenarios.find(value=>value.id===row.id),capture=sessions.get(row.sessionId),outputs=[];need(capture,'Scenario has no actual fresh native session');const prompt=store.bytes(row.prompt).toString('utf8'),response=assistantText(anchor(capture.raw,row.response),provider);
    need(prompt.trim() && (capture.capture.stdin && store.bytes(capture.capture.stdin).toString('utf8').includes(prompt) || capture.events.some(event=>(event.type==='user' || eventPayload(event)?.role==='user') && text(event).includes(prompt))),'Authored scenario prompt is absent from actual stdin/native user events');
    need(row.review?.verdict==='passed' && row.review.reviewer?.trim() && row.review.observations?.trim(),'Actual per-scenario semantic review is missing');const evidence=row.evidence;need(evidence?.kind===required.kind,'Scenario outcome does not match its declared plan scope');
    if(['delivery','repair'].includes(required.kind)) {
      const completed=caseEvidence(evidence.case,store,store.json(evidence.case.finalize).engine,{reader:false}),delivery=completed.stages[1].receipt;
      need(completed.engine.sourceInputSha256===source && same(completed.engine.binary,capture.installedEngine.binary) && (required.types || TYPES).includes(delivery.config?.diagramType) && (!required.contract || delivery.inputContract===required.contract),'Provider output type/source/input contract differs');const reply=actualReply(capture,evidence.pdResult,evidence.reply,store);header(reply,'finalize');need(reply.deliveryReceiptId===completed.deliveryReceiptId && same(reply.browserReceiptIdentity,completed.browserReceiptIdentity) && same(reply.stages,completed.stages),'Actual initial finalize reply changed identity at review');
      if(required.kind==='repair'){const before=actualReply(capture,evidence.beforeResult,evidence.before,store,'failure');need(['validate','check','finalize'].includes(before.command) && before.status!=='pass' && receiptErrors(before)>0,'Repair lacks a real earlier diagnostic failure');if(replay)outputs.push(failedValidation(before));}
      if(profile==='p2')p2Outcome(required,completed,evidence,store,response);if(replay)outputs.push(delivered(delivery));
    } else if(required.kind==='clarification') {
      need(evidence.missing?.includes(required.missing) && evidence.question?.trim() && response.includes(evidence.question),'Missing facts were not actually clarified');
    } else if(required.kind==='stall') {
      need(evidence.rounds?.length>=3 && evidence.stoppedQuote?.trim() && response.includes(evidence.stoppedQuote),'Stall lacks actual rounds and stopping response');let best=Infinity,streak=0;
      for(const round of evidence.rounds){const receipt=actualReply(capture,round.pdResult,round.receipt,store,'failure'),count=receiptErrors(receipt);need(['validate','check','finalize'].includes(receipt.command) && receipt.status!=='pass' && count>0 && same(identity(store.bytes(round.candidateSnapshot)),receipt.candidate?.identity),'Stall round lacks actual candidate/diagnostic bytes');if(replay)outputs.push(failedValidation(receipt));if(count<best){best=count;streak=0;}else streak++;}need(streak>=2,'Two consecutive real diagnostic rounds did not stall');
    } else if(required.kind==='browser-failure') {
      const receipt=actualReply(capture,evidence.pdResult,evidence.receipt,store,'failure'),browser=receipt.command==='finalize'?receipt.stages?.find(stage=>stage.name==='browser-check')?.receipt:receipt;need(browser?.status===required.expected && receipt.status!=='pass' && receipt.deliveryStatus!=='complete' && evidence.quote?.trim() && response.includes(evidence.quote),'Actual browser failure/unavailable outcome was hidden or omitted');if(replay)outputs.push(delivered(receipt));
    } else {
      const summary=store.json(evidence.summary),reply=actualReply(capture,evidence.pdResult,evidence.reply,store);header(summary,'finalize');need(reply.deliveryReceiptId===summary.deliveryReceiptId && same(reply.stages,summary.stages) && summary.automatedStatus==='pass' && summary.visualReview?.status==='required-pending' && summary.deliveryStatus==='incomplete' && evidence.quote?.trim() && response.includes(evidence.quote),'Visual-pending case was incorrectly called complete');if(replay)outputs.push(delivered(summary.stages[1].receipt));
    }
    if(outputs.length)checkReplay(outputs,replay.rows.find(value=>value.id===row.id) || {id:row.id},cohort,replay.store);
  }
  const checked=record.scenarios.filter(value=>!rows || rows.includes(value.id));
  return{provider,status:'pass',sessionIds:rows?[...new Set(checked.map(row=>row.sessionId))]:[...sessions.keys()],scenarioIds:checked.map(row=>row.id),...(replay?{engineSource:source}:{}),scope:'Actual native JSONL/tool reads and owned candidate artifacts, with explicit per-scenario recorded reviewer semantic observations'};
}

// Facts the P2 rows add to an ordinary delivery: repository evidence, time basis,
// interface language and a reference JSON selected without rewriting it.
export function p2Outcome(required,summary,evidence,store,response) {
  const delivery=summary.stages[1].receipt,candidate=JSON.parse(store.bytes(summary.candidate).toString('utf8')),html=()=>store.bytes(summary.artifact).toString('utf8'),quoted=()=>typeof evidence.quote==='string' && evidence.quote.trim() && response.includes(evidence.quote);
  if(required.evidence==='verified')need(summary.evidence?.status==='verified' && delivery.evidence?.status==='verified' && delivery.evidence.counts?.verified>0 && delivery.evidence.repository,'Repository sources are not verified at the fixed commit');
  if(required.evidence==='worktree-differs')need(['partial','unverified'].includes(summary.evidence?.status) && delivery.evidence?.unverified?.some(row=>row.reason==='worktree-differs') && quoted(),'A changed cited file was not reported as unverified in the reply');
  if(required.timing==='day')need(/data-timeline-evidence="observed"/.test(html()) && candidate.events?.some(event=>event.precision==='day'),'Observed day-precision timing is not disclosed');
  if(required.timing==='illustrative')need(/data-waterfall-evidence="illustrative"/.test(html()) && quoted(),'Illustrative timing is not disclosed in the diagram and reply');
  if(required.locale)need(candidate.meta?.locale===required.locale && same(summary.locale,{requested:required.locale,resolved:required.locale,fallback:false}),'Interface language differs from the request');
  if(required.unchangedOriginal)need(evidence.original && same(identity(store.bytes(evidence.original)),evidence.original.identity) && same(evidence.original.identity,summary.candidate.identity) && candidate.meta?.input_contract===undefined,'Reference JSON was rewritten instead of selected with an explicit input contract');
}
function liveApi(endpoint) {
  need(process.env.GITHUB_TOKEN,'A read-only GITHUB_TOKEN with actions:read is required');need(/^\/repos\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+\/actions\/(?:runs\/\d+|jobs\/\d+|artifacts\/\d+)$/.test(endpoint),'Unexpected read-only Actions API endpoint');
  const env={HOME:process.env.HOME,PATH:process.env.PATH,GH_TOKEN:process.env.GITHUB_TOKEN,GH_HOST:'github.com',GH_PROMPT_DISABLED:'1'};for(const name of ['HTTP_PROXY','HTTPS_PROXY','NO_PROXY'])if(process.env[name]!==undefined)env[name]=process.env[name];
  const result=spawnSync('gh',['api','--hostname','github.com','--method','GET','-H','Accept: application/vnd.github+json',endpoint],{encoding:'utf8',maxBuffer:16*1024*1024,timeout:30000,env});need(!result.error && result.status===0,`Actual Actions provenance could not be read: ${result.error?.message || result.stderr}`);return JSON.parse(result.stdout);
}
function candidateArtifact(entry,cohort,api,name) {
  need(entry.archive.identity.sha256===entry.digest,'Actions input archive digest differs');const metadata=api(`/repos/${cohort.repository}/actions/artifacts/${entry.id}`);need(metadata.id===entry.id && metadata.name===name && metadata.digest===`sha256:${entry.digest}` && metadata.expired===false && String(metadata.workflow_run?.id)===String(cohort.runId) && metadata.workflow_run.head_sha===cohort.source.commit,'Actual candidate/harness artifact provenance differs');
  return new ArchiveEvidence(entry.archive,'unused','/unused');
}
export function collectorIdentityFromHarness(bytes) {
  const names=processBytes('tar',['-tf','-'],bytes).toString('utf8').split('\n').filter(Boolean);for(const name of names)relative(name.replace(/\/$/,''));
  need(names.filter(name=>name===COLLECTOR_MEMBER).length===1,'Authenticated harness has a missing or duplicate collector member');
  const source=processBytes('tar',['-xOf','-',COLLECTOR_MEMBER],bytes);need(source.length>0,'Authenticated collector template is empty');return identity(source);
}
export function aggregateCandidate(input,{api=liveApi,profile='p1'}={}) {
  const result={schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'aggregate-candidate',kind:'phoenix-diagram-candidate-aggregate',profile,status:'incomplete',handoff:'incomplete',publication:'not-requested',platforms:[],node18:{status:'not-run'},...(profile==='p2'?{local:{status:'not-run'}}:{}),providers:[],diagnostics:[]};
  const check=(scope,operation)=>{try{return operation();}catch(error){result.diagnostics.push({scope,code:error.code || 'aggregate/incomplete',message:error.message});return null;}};
  const setup=check('input',()=>{need(input?.schemaVersion===1 && input.kind==='phoenix-diagram-candidate-aggregate-input' && /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(input.repository) && /^\d+$/.test(String(input.runId)) && Number.isSafeInteger(input.runAttempt) && input.runAttempt>0,'Aggregate input cohort is malformed');need(PROFILE_CASES[profile] && (input.profile ?? 'p1')===profile,'Aggregate input profile differs from the requested profile');checkInputSlots(input,profile);const candidate=validateCandidate(input.candidate.directory,targetForHost()),cohort={repository:input.repository,runId:String(input.runId),runAttempt:input.runAttempt,source:candidate.source,tgzSha256:candidate.tgz.identity.sha256,candidateArchiveSha256:input.candidate.candidateArchiveSha256,harnessSha256:input.harness.harnessSha256,engines:candidate.version.engines,npmPackageVersion:candidate.pkg.version,bundleSha256:candidate.version.bundleSha256,versionSha256:sha(candidate.files.get('release-dir/VERSION.json').content)};need(HASH.test(cohort.candidateArchiveSha256) && HASH.test(cohort.harnessSha256),'Candidate/harness transfer SHA is missing');return{candidate,cohort};});if(!setup)return result;
  const {candidate,cohort}=setup;Object.assign(cohort,{candidateArtifactId:input.candidate.artifact.id,candidateArtifactDigest:input.candidate.artifact.digest,harnessArtifactId:input.harness.artifact.id,harnessArtifactDigest:input.harness.artifact.digest});result.cohort=cohort;
  const authenticated=check('workflow-source',()=>{const run=api(`/repos/${cohort.repository}/actions/runs/${cohort.runId}`);need(String(run.id)===cohort.runId && run.run_attempt===cohort.runAttempt && run.head_sha===cohort.source.commit && run.repository?.full_name===cohort.repository,'Actual workflow run/source/latest attempt differs');const source=candidateArtifact(input.candidate.artifact,cohort,api,'phoenix-diagram-candidate'),tar=readBound(input.candidate.tar,'candidate transfer tar');need(sha(source.member('phoenix-diagram-candidate.tar'))===cohort.candidateArchiveSha256 && sha(tar.content)===cohort.candidateArchiveSha256,'Candidate transfer tar differs');
    for(const [name,snapshot] of [['candidate.json',candidate.snapshot],...candidate.files])need(processBytes('tar',['-xOf',tar.path,`candidate/${relative(name)}`]).equals(snapshot.content),'Extracted candidate bytes differ from authenticated transfer tar');const harness=candidateArtifact(input.harness.artifact,cohort,api,'phoenix-diagram-harness'),harnessBytes=harness.member('phoenix-diagram-harness.tar');need(sha(harnessBytes)===cohort.harnessSha256,'Harness transfer bytes differ');cohort.collectorIdentity=collectorIdentityFromHarness(harnessBytes);return true;});if(!authenticated)return result;
  // P2 platforms are automatic-only (ADR-2); the full review loop is the local darwin-arm64 consumption.
  const cases=PROFILE_CASES[profile],reviewed=profile==='p1';
  for(const entry of input.platforms){const checked=check(`platform:${entry.target}`,()=>platformEvidence(entry,cohort,api,{reviewed,cases}));result.platforms.push(checked || {target:entry.target,status:'incomplete'});}
  result.node18=check('node18',()=>platformEvidence(input.node18,cohort,api,{node18:true,cases})) || {status:'incomplete'};
  if(profile==='p2')result.local=check('local',()=>localEvidence(input.local,cohort,{cases})) || {status:'incomplete'};
  const inventory=readTarballInventory(candidate.tgz.content);for(const entry of input.providers){const checked=check(`provider:${entry.provider}`,()=>providerEvidence(entry,cohort,inventory,{profile,candidate}));result.providers.push(checked || {provider:entry.provider,status:'incomplete'});}
  if(result.diagnostics.length===0 && result.platforms.length===4 && result.platforms.every(row=>row.status==='pass') && result.node18.status==='pass' && (profile!=='p2' || result.local.status==='pass') && result.providers.length===2 && result.providers.every(row=>row.status==='pass')){result.status='pass';result.handoff='passed';}return result;
}
export const HELP='Usage: node scripts/aggregate-phoenix-diagram-candidate.mjs [--profile p1|p2] --input INPUT.json --output NEW.json\nChecks authenticated current workflow/job/artifact outcomes, unchanged archived evidence, exact Node 18.20.8 and actual Claude/Codex session records. p2 checks the four platforms as automatic-only, adds the local darwin-arm64 full-review consumption and binds host sessions to the identical authenticated candidate tgz/VERSION or a development package with the same bundle and engine source; a provider entry may keep rows from an earlier engine of the same bundle (reuse) when a final-engine replay reproduces the diagram and failed validations of each kept row. Missing, pending, unavailable or inconsistent evidence stays incomplete. Never runs providers, Chrome, Cargo, renderers or publication.\n';
export async function main(argv=process.argv.slice(2)) {
  if(argv.length===1 && argv[0]==='--help'){process.stdout.write(HELP);return 0;}let output;
  try{const options={};for(let at=0;at<argv.length;at++){const key=argv[at];need(['--input','--output','--profile'].includes(key) && !options[key] && argv[at+1] && !argv[at+1].startsWith('--'),'Expected unique --input/--output/--profile values');options[key]=argv[++at];}need(!options['--profile'] || Object.hasOwn(PROFILE_CASES,options['--profile']),'Unknown aggregate profile');need(options['--input'] && options['--output'],'--input and --output are required');output=path.resolve(options['--output']);need(!fs.existsSync(output),'Aggregate output already exists; preserve it');const input=JSON.parse(readSnapshot(options['--input']).content.toString('utf8')),result=aggregateCandidate(input,{profile:options['--profile'] || 'p1'});fs.writeFileSync(output,`${JSON.stringify(result,null,2)}\n`,{flag:'wx',mode:0o600});process.stdout.write(`${JSON.stringify(result)}\n`);return result.handoff==='passed'?0:1;}
  catch(error){process.stdout.write(`${JSON.stringify({schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'aggregate-candidate',status:'incomplete',handoff:'incomplete',diagnostics:[{code:'aggregate/incomplete',message:error.message}]})}\n`);return 1;}
}
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url))process.exitCode=await main();
export function readerEvidence(report,store,artifact,{content=false}={}) {
  need(report.status==='pass' && report.kind===(content?'real-browser-content':'real-browser-export') && report.artifact.path===artifact.path && same({sha256:report.artifact.sha256,bytes:report.artifact.bytes},artifact.identity) && /(?:Chrome|Chromium)\//.test(report.actualBrowserVersion || ''),'Actual Reader artifact/version differs');need(report.offline?.requested===true && report.offline.status==='pass' && report.offline.configured===true && report.offline.remoteRequests?.length===0 && report.resources?.status==='pass' && report.resources.loadingFailed?.length===0 && report.resources.requests?.every(row=>!/^(?:https?|wss?):/i.test(row.url)) && report.errors?.length===0,'Actual offline/resource scope is incomplete');
  if(content){need(report.content?.status==='pass' && report.content.literalPresent===true && report.content.sentinelDefined===false,'Actual XSS content was omitted or executed');return;}
  verifyMotionSuppression(report.motionSuppression);
  verifyReaderInteractions(report.readerInteractions);
  checkPrintEvidence(report.stateRestoration?.print,reference=>store.bytes(reference));
  need(report.exports?.length===20 && report.stateRestoration?.status==='pass','Real export/Reader-restoration scope is incomplete');const seen=new Set();for(const file of report.exports){const key=`${file.theme}:${file.mode}:${file.format}`;need(!seen.has(key) && ['dark','light'].includes(file.theme) && ['still','live'].includes(file.mode) && ['svg','png','jpeg','webp','webm'].includes(file.format) && file.status==='pass' && file.decode?.valid===true && file.decode.width>0 && file.decode.height>0,'Actual export decode scope is incomplete');seen.add(key);verifyFileSignature(store.bytes(file),file.format,file.mime);if(file.format==='svg')need(file.canonical?.complete===true && file.canonical.utf8===true,'Canonical SVG is incomplete');}
  need(['success','permissionDenied','capabilityUnavailable'].every(key=>report.clipboard?.[key]?.status==='pass'),'Native clipboard success/permission/capability evidence is incomplete');const copied=report.clipboard.success;verifyFileSignature(store.bytes({path:copied.path,identity:{sha256:copied.sha256,bytes:copied.bytes}}),'png',copied.mime);store.bytes(report.clipboard.capabilityUnavailable.fallback);
}
export function checkPrintEvidence(print,bytesFor) {
  need(print?.status==='pass' && print.method==='cdp-print-media-and-native-print-to-pdf' && print.observations?.length===4,'Actual print/native PDF scope is incomplete');const seen=new Set();
  for(const sample of print.observations){const key=`${sample.theme}:${sample.mode}`;need(['dark:still','dark:live','light:still','light:live'].includes(key) && !seen.has(key) && sample.status==='pass','Print theme/mode scope differs');seen.add(key);verifyPrintSample(sample);const pdf=bytesFor(sample.nativePrint.pdf);need(pdf.subarray(0,5).toString()==='%PDF-' && pdf.subarray(-64).includes(Buffer.from('%%EOF')) && (pdf.toString('latin1').match(/\/Type\s*\/Page\b/g) || []).length===sample.nativePrint.pdf.pages,'Actual print PDF signature/EOF/pages differ');}
}
