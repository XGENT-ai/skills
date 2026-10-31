import fs from 'node:fs';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { isDeepStrictEqual as same } from 'node:util';
import { randomBytes } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { CoreError,identity,readSnapshot,readBound,bound,requireThat,locateRecovery } from './test-phoenix-diagram-core.mjs';

const PHASES=['after-retire','after-recovery-link','after-witness-quarantine'];
const OLD=Buffer.from('Original output retained by the public CLI recovery acceptance.\n');
const CLAIMANT=Buffer.from('A later user claimant must survive recovery.\n');
function directory(file) {fs.mkdirSync(file,{mode:0o700});return fs.realpathSync(file);}
function writeNew(file,bytes) {fs.writeFileSync(file,Buffer.isBuffer(bytes)?bytes:Buffer.from(`${JSON.stringify(bytes,null,2)}\n`),{flag:'wx',mode:0o600});return bound(readSnapshot(file));}
function exists(file) {try{fs.lstatSync(file);return true;}catch(error){if(error.code==='ENOENT')return false;throw error;}}
function digest(file) {return identity(fs.readFileSync(file));}
function noDeliveryEvidence(parent,target) {
  const files=fs.readdirSync(parent);
  requireThat(!files.some(name=>name.startsWith(path.basename(target).replace(/\.html$/,'')) && /\.(?:delivery|finalize-summary|browser-check|visual-review)\./.test(name)),'Interrupted delivery or recovery invented passing delivery evidence');
}

// This marker is a timing signal only. Its recoveryDirectory is deliberately
// discarded; the public locator scans durable records in the original parent.
export async function waitForCheckpoint(child,file,{phase,target,timeoutMs=12000,pidMatches}) {
  requireThat(PHASES.includes(phase),'Unknown checkpoint phase');
  const deadline=Date.now()+timeoutMs;
  while(Date.now()<deadline) {
    try {
      const value=JSON.parse(readSnapshot(file).content.toString('utf8'));
      requireThat(value.schemaVersion===1 && Number.isSafeInteger(value.pid) && value.pid>0 && (pidMatches?pidMatches(value.pid,child.pid):value.pid===child.pid),'Checkpoint marker does not belong to the owned child PID');
      requireThat(value.phase===phase && value.target===target,'Checkpoint phase or target does not match the requested interruption');
      return{pid:value.pid,phase:value.phase,target:value.target};
    } catch(error) {if(error.code!=='ENOENT' && error.code!=='core/file-binding' && !(error instanceof SyntaxError))throw error;}
    if(child.exitCode!==null || child.signalCode!==null)throw new CoreError('core/checkpoint-unavailable','Owned native child exited before reaching the requested maintenance checkpoint',{exitCode:4});
    await delay(10);
  }
  throw new CoreError('core/checkpoint-timeout','Owned native child did not reach the requested maintenance checkpoint',{exitCode:4});
}

export function verifyRecoveryOutcome(result,{mode,target,expected,publicAbsent=false}) {
  const receipt=result.receipt;
  requireThat(receipt?.schemaVersion===1 && receipt.tool==='phoenix-diagram' && receipt.toolVersion==='0.2.0' && receipt.command==='recover-output' && Array.isArray(receipt.diagnostics),'Public recover-output did not return its typed receipt');
  requireThat(!['deliveryReceiptId','deliveryStatus','automatedStatus','visualReview'].some(key=>Object.hasOwn(receipt,key)),'Recovery must not invent delivery or visual evidence');
  if(mode==='recovered')requireThat(result.exitCode===0 && receipt.status==='pass' && receipt.recoveryStatus==='recovered' && receipt.target===target,'Public recovery did not report the original target as recovered');
  else if(mode==='absent')requireThat(result.exitCode===0 && receipt.status==='pass' && receipt.recoveryStatus==='absent','Repeated public recovery was not idempotent');
  else requireThat(result.exitCode===3 && receipt.status==='failed' && ['preserved','recovery-required'].includes(receipt.recoveryStatus) && receipt.preserved.length>0,'A changed binding or later claimant was not preserved with a conflict exit');
  if(publicAbsent) {
    requireThat(!exists(target),'Recovery created public bytes after a protected binding changed');
    if(receipt.publicState.kind!=='unknown')requireThat(receipt.publicState.kind==='absent','Reported public state differs from the actual absent output');
  } else {
    const actual=readSnapshot(target);requireThat(same(actual.identity,expected),'Public output bytes or identity changed during recovery');
    if(receipt.publicState.kind==='file')requireThat(same({sha256:receipt.publicState.identity.sha256,bytes:receipt.publicState.identity.bytes},actual.identity),'Recovery receipt publicState differs from actual public bytes');
    else requireThat(mode!=='recovered' && receipt.publicState.kind==='unknown','Successful recovery omitted actual public file state');
  }
  return{status:'pass',exitCode:result.exitCode,recoveryStatus:receipt.recoveryStatus,publicState:receipt.publicState};
}

async function interrupt(index,options,invoke,args,scenarioRoot,phase,target) {
  const checkpoint=directory(path.join(scenarioRoot,`checkpoint-${phase}`));let observed;
  const result=await invoke(index.engine,args,{options,json:false,env:{PHOENIX_DIAGRAM_ACCEPTANCE:'1',PHOENIX_DIAGRAM_CHECKPOINT_DIR:checkpoint,PHOENIX_DIAGRAM_CHECKPOINT_PHASE:phase},onSpawn:async child=>{
    const pidMatches=options.publicNpm?(pid,leader)=>{const result=spawnSync('/bin/ps',['-o','pgid=','-p',String(pid)],{encoding:'utf8',timeout:3000});return !result.error && result.status===0 && result.stdout.trim()===String(leader);}:undefined;
    // Public npm startup/preflight precedes the independent native checkpoint hold.
    observed=await waitForCheckpoint(child,path.join(checkpoint,'checkpoint.json'),{phase,target,pidMatches,timeoutMs:options.publicNpm?60000:12000});
    if(phase==='after-retire')requireThat(!exists(target),'The interruption window did not retire the old public file');
    else requireThat(same(digest(target),identity(OLD)),'Recovery interruption did not preserve the original public bytes');
    // invokeNative owns a fresh detached process group; never signal another PID.
    try{process.kill(-child.pid,'SIGKILL');}catch(error){if(error.code!=='ESRCH')throw error;}
  }});
  requireThat(observed && result.signal==='SIGKILL','The owned native process was not killed at the observed checkpoint');
  return{status:'pass',...observed,signal:result.signal,terminalOutputDiscarded:true};
}

async function recover(index,options,invoke,scenarioRoot,locator,label,expected) {
  const result=await invoke(index.engine,['recover-output',locator.directory,'--json'],{options});
  const receipt=writeNew(path.join(scenarioRoot,`${label}.json`),result.receipt || {missingReceipt:true,exitCode:result.exitCode,stderr:result.stderr});
  const outcome=verifyRecoveryOutcome(result,expected);return{...outcome,receipt};
}

async function prepareScenario(index,options,invoke,parent,id,candidate) {
  const scenarioRoot=directory(path.join(parent,id)),outputParent=directory(path.join(scenarioRoot,'output')),target=path.join(outputParent,'diagram.html');
  writeNew(target,OLD);
  const interruption=await interrupt(index,options,invoke,['deliver','architecture',candidate.path,target,'--json'],scenarioRoot,'after-retire',target);
  const locator=locateRecovery(outputParent,target);
  const journal=writeNew(path.join(scenarioRoot,'located-record.json'),readBound(locator.recordFile,'located durable recovery record').content);
  const backup=bound(readSnapshot(path.join(locator.directory,'previous')));
  requireThat(same(backup.identity,identity(OLD)),'Durable backup did not retain the original bytes');noDeliveryEvidence(outputParent,target);
  return{id,scenarioRoot,outputParent,target,locator,backup,journal,interruption};
}

export async function runRecoveryScenarios(index,options,{invoke}) {
  if(index.recovery?.status==='pass')return index.recovery;
  const source=index.cases.find(entry=>entry.type==='architecture' && entry.contract==='phoenix-v1');
  if(!source)return{status:'not-run',reason:'A new architecture source is required for public recovery scenarios'};
  readBound(source.candidate,'recovery candidate');const parent=directory(path.join(index.runRoot,`recovery-${randomBytes(8).toString('hex')}`)),scenarios=[];
  for(const id of ['restore-and-repeat','later-claimant','record-tamper','backup-tamper','parent-replacement','kill-after-recovery-link','kill-after-witness-quarantine','kill-recovery-later-claimant']) {
    const scenarioRoot=path.join(parent,id);let scenario;
    try {
      scenario=await prepareScenario(index,options,invoke,parent,id,source.candidate);
      const {target,locator,scenarioRoot,outputParent}=scenario;
      if(id==='restore-and-repeat') {
        scenario.recovery=await recover(index,options,invoke,scenarioRoot,locator,'recover',{mode:'recovered',target,expected:identity(OLD)});
        scenario.repeat=await recover(index,options,invoke,scenarioRoot,locator,'repeat',{mode:'absent',target,expected:identity(OLD)});
        const strict=await invoke(index.engine,['check',target,'--strict-provenance','--json'],{options});
        requireThat(strict.exitCode!==0 && strict.receipt?.status!=='pass','Restored raw bytes must not receive invented current delivery evidence');
        scenario.strict={status:'pass',exitCode:strict.exitCode,receipt:writeNew(path.join(scenarioRoot,'strict-after-recovery.json'),strict.receipt)};
      } else if(id==='later-claimant') {
        const claimant=writeNew(target,CLAIMANT);
        scenario.recovery=await recover(index,options,invoke,scenarioRoot,locator,'recover',{mode:'preserved',target,expected:claimant.identity});readBound(scenario.backup,'preserved backup');scenario.claimant=claimant;
      } else if(id==='record-tamper' || id==='backup-tamper') {
        const changed=id==='record-tamper'?locator.recordFile.path:scenario.backup.path;
        fs.appendFileSync(changed,id==='record-tamper'?'\nnot a valid recovery record':'changed backup bytes');
        const tampered=bound(readSnapshot(changed));scenario.tampered=tampered;
        scenario.recovery=await recover(index,options,invoke,scenarioRoot,locator,'recover',{mode:'preserved',target,publicAbsent:true});readBound(tampered,'preserved tampered file');
        if(id==='record-tamper')readBound(scenario.backup,'preserved original backup');
      } else if(id==='parent-replacement') {
        const holding=path.join(scenarioRoot,'original-parent');fs.renameSync(outputParent,holding);directory(outputParent);const claimant=writeNew(target,CLAIMANT);
        const movedLocator={directory:path.join(holding,path.basename(locator.directory))};
        scenario.recovery=await recover(index,options,invoke,scenarioRoot,movedLocator,'recover',{mode:'preserved',target,expected:claimant.identity});
        scenario.backup=bound(readSnapshot(path.join(movedLocator.directory,'previous')));requireThat(same(scenario.backup.identity,identity(OLD)),'Parent replacement changed the preserved original backup');scenario.claimant=claimant;
      } else {
        const phase=id==='kill-after-witness-quarantine'?'after-witness-quarantine':'after-recovery-link';
        scenario.recoveryInterruption=await interrupt(index,options,invoke,['recover-output',locator.directory,'--json'],scenarioRoot,phase,target);
        const rediscovered=locateRecovery(outputParent,target);scenario.rediscoveredRecord=writeNew(path.join(scenarioRoot,'rediscovered-record.json'),readBound(rediscovered.recordFile,'rediscovered durable record').content);
        if(id==='kill-recovery-later-claimant') {
          const replacement=writeNew(path.join(outputParent,'claimant.tmp'),CLAIMANT);fs.renameSync(replacement.path,target);const claimant=bound(readSnapshot(target));
          scenario.recovery=await recover(index,options,invoke,scenarioRoot,rediscovered,'recover',{mode:'preserved',target,expected:claimant.identity});scenario.claimant=claimant;
        } else {
          scenario.recovery=await recover(index,options,invoke,scenarioRoot,rediscovered,'recover',{mode:'recovered',target,expected:identity(OLD)});
          scenario.repeat=await recover(index,options,invoke,scenarioRoot,rediscovered,'repeat',{mode:'absent',target,expected:identity(OLD)});
        }
      }
      scenario.retained=[];
      if(['later-claimant','record-tamper','backup-tamper','parent-replacement','kill-recovery-later-claimant'].includes(id)) {
        if(scenario.tampered)scenario.retained.push(scenario.tampered);
        if(id!=='backup-tamper')scenario.retained.push(bound(readSnapshot(scenario.backup.path)));
      }
      scenario.finalPublic=exists(target)?bound(readSnapshot(target)):{path:target,absent:true};
      noDeliveryEvidence(outputParent,target);scenario.status='pass';
      // Recovery may deliberately remove its own live journal and backup. The
      // saved copies plus public CLI receipts remain immutable evidence.
      delete scenario.locator;delete scenario.backup;scenarios.push(scenario);
    } catch(error) {
      const failed={id,status:'failed',exitCode:error.exitCode || 1,diagnostics:[{code:error.code || 'core/recovery',message:error.message}],root:scenarioRoot};
      if(error.processResult)try {
        const metadata=fs.lstatSync(scenarioRoot);
        requireThat(metadata.isDirectory() && !metadata.isSymbolicLink() && fs.realpathSync(scenarioRoot)===scenarioRoot && metadata.uid===process.getuid() && (metadata.mode&0o7777)===0o700,'Failed process evidence requires the existing owned 0700 scenario directory','core/recovery-process-evidence');
        const {stdout,stderr,...details}=error.processResult;requireThat(Buffer.isBuffer(stdout) && Buffer.isBuffer(stderr),'Failed process raw bytes are missing','core/recovery-process-evidence');
        const captured={...details,stdout:writeNew(path.join(scenarioRoot,'failed-process.stdout'),stdout),stderr:writeNew(path.join(scenarioRoot,'failed-process.stderr'),stderr)};
        failed.process=writeNew(path.join(scenarioRoot,'failed-process.json'),captured);
      }catch(cause){failed.diagnostics.push({code:cause.code || 'core/recovery-process-evidence',message:cause.message});}
      scenarios.push(failed);
    }
  }
  return{status:scenarios.every(scenario=>scenario.status==='pass')?'pass':'failed',root:parent,scope:'real native public recover-output and actual filesystem/SIGKILL; no browser or visual evidence',scenarios};
}
