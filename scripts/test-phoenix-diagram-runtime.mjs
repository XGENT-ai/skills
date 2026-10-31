import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';
import { REQUIRED_EXPORT_SCOPE, verifyPrintSample, verifyMotionSuppression } from '../tools/phoenix-diagram/tests/export-acceptance.mjs';
import { verifyReaderInteractions } from '../tools/phoenix-diagram/tests/support/export-reader.mjs';
import { root,TYPES,contractsFor,XSS_LITERAL,CoreError,identity,requiredCoverage,readSnapshot,readBound,bound,requireThat,inspectFinalize,verifyReviewTransition,assertXssArtifact,publicNpmCommand,readEvidenceBytes } from './test-phoenix-diagram-core.mjs';

const INDEX_KIND='phoenix-diagram-core-acceptance';
const LEGACY={architecture:'web-app',workflow:'agent-tool-call',sequence:'async-job-roundtrip',dataflow:'product-analytics',lifecycle:'deployment-release'};
const CHECKPOINT_VARIABLES=['PHOENIX_DIAGRAM_ACCEPTANCE','PHOENIX_DIAGRAM_CHECKPOINT_DIR','PHOENIX_DIAGRAM_CHECKPOINT_PHASE'];
const same=isDeepStrictEqual;
// Acceptance scopes run on resume against the finalized cases; each stays not-run until requested.
// The core stage requires all of them for handoff; a package consumer fixes its own subset at creation.
export const SCOPES=['explore','corpus','evidence','locale'];
export const DEFAULT_CASES=TYPES.flatMap(type=>[
  {id:`new-${type}`,type,contract:'phoenix-v1',source:path.join(root,'skills/phoenix-diagram/examples',`minimal.${type}.json`)},
  ...(contractsFor(type).includes('legacy-2.17')?[{id:`legacy-${type}`,type,contract:'legacy-2.17',source:path.join(root,'tools/phoenix-diagram/upstream/legacy-2.17/examples',`${LEGACY[type]}.${type}.json`)}]:[]),
]);

function diagnostic(error) {return{stage:error.stage || 'core',code:error.code || 'core/execution',message:error.message};}
function header(command) {return{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command};}
function writeNew(file,value,mode=0o600) {const content=Buffer.isBuffer(value)?value:Buffer.from(`${JSON.stringify(value,null,2)}\n`);fs.writeFileSync(file,content,{flag:'wx',mode});return bound(readSnapshot(file));}
function privateDirectory(directory,{existing=false}={}) {
  if(!existing)fs.mkdirSync(directory,{mode:0o700});
  const stat=fs.lstatSync(directory);requireThat(stat.isDirectory() && !stat.isSymbolicLink() && (stat.mode&0o7777)===0o700 && stat.uid===process.getuid(),'Run directory must be an owned ordinary 0700 directory','core/private-directory');return fs.realpathSync(directory);
}
function plainIndex(index) {const copy=structuredClone(index);delete copy.indexFile;delete copy.exitCode;return copy;}
function persist(index,previous) {
  if(previous)readBound(previous,'previous core index');
  const name=previous?`index-${randomBytes(8).toString('hex')}.json`:'index.json';
  const file=writeNew(path.join(index.runRoot,name),plainIndex(index));return{...index,indexFile:file};
}
function safeNative(snapshot) {
  const bytes=snapshot.content;let architecture;
  if(bytes.length>=64 && bytes.subarray(0,4).equals(Buffer.from([0x7f,0x45,0x4c,0x46])) && bytes[4]===2 && bytes[5]===1) architecture=bytes.readUInt16LE(18)===183?'arm64':bytes.readUInt16LE(18)===62?'x64':null;
  else if(bytes.length>=32 && bytes.readUInt32LE(0)===0xfeedfacf) architecture=bytes.readUInt32LE(4)===0x0100000c?'arm64':bytes.readUInt32LE(4)===0x01000007?'x64':null;
  requireThat(['darwin','linux'].includes(process.platform) && architecture===process.arch && (snapshot.binding.mode&0o111)!==0,'--engine must be a real executable native binary for this host','core/native-engine');
}
function runtimeEnvironment(options,extra={}) {
  const env={...(options.publicNpm?.env || process.env)};for(const key of CHECKPOINT_VARIABLES)delete env[key];
  if(options.browserPath)env.PHOENIX_DIAGRAM_BROWSER=path.resolve(options.browserPath);
  return{...env,...extra};
}
function stopOwned(child,signal) {
  if(child.exitCode!==null || child.signalCode!==null)return;
  try{process.kill(-child.pid,signal);}catch(error){if(error.code!=='ESRCH')throw error;}
}

// Only this adapter starts native processes. Unit tests exercise pure bindings
// and inject an adapter into resumeReviews; they are not native acceptance.
export async function invokeNative(engine,args,{options={},env={},prefix=[],timeoutMs=300000,json=true,onSpawn}={}) {
  readBound(engine,'native engine');
  const publicCommand=options.publicNpm?publicNpmCommand(options.publicNpm,args):null;
  const executable=publicCommand?.program || engine.path,argv=publicCommand?.args || args;
  const command=prefix.length?prefix[0]:executable;
  const parameters=prefix.length?[...prefix.slice(1),executable,...argv]:[...argv];
  return await new Promise((resolve,reject)=>{
    const cwd=publicCommand?.cwd || root,startedAt=new Date().toISOString(),started=process.hrtime.bigint();
    const child=spawn(command,parameters,{cwd,env:runtimeEnvironment(options,env),stdio:['ignore','pipe','pipe'],detached:true});
    let stdout=[],stderr=[],size=0,failure,timer;
    const accept=(chunks,chunk)=>{size+=chunk.length;if(size>64*1024*1024){failure=new CoreError('core/native-output-limit','Native output exceeded the maintenance capture limit',{exitCode:4});stopOwned(child,'SIGKILL');}else chunks.push(chunk);};
    child.stdout.on('data',chunk=>accept(stdout,chunk));child.stderr.on('data',chunk=>accept(stderr,chunk));
    child.on('error',error=>{failure=error;});
    timer=setTimeout(()=>{failure=new CoreError('core/native-timeout',`Owned native command timed out: ${args[0]}`,{exitCode:4});stopOwned(child,'SIGKILL');},timeoutMs);
    const checkpoint=onSpawn?Promise.resolve().then(()=>onSpawn(child)).catch(error=>{failure=error;stopOwned(child,'SIGKILL');}):Promise.resolve();
    child.on('close',async(code,signal)=>{
      clearTimeout(timer);
      const processResult={program:command,argv:[...parameters],cwd,pid:child.pid??null,startedAt,closedAt:new Date().toISOString(),elapsedMs:Number(process.hrtime.bigint()-started)/1e6,exitCode:code,signal,stdout:Buffer.concat(stdout),stderr:Buffer.concat(stderr)};
      try{
        await checkpoint;
        readBound(engine,'native engine');if(failure)throw failure;
        const output=processResult.stdout.toString('utf8'),errors=processResult.stderr.toString('utf8');let receipt;
        if(json && output.trim())receipt=JSON.parse(output.trim());
        resolve({exitCode:code??4,signal,stdout:output,stderr:errors,receipt});
      }catch(error){Object.defineProperty(error,'processResult',{value:processResult,configurable:true});reject(error);}
    });
  });
}

async function prepareEngine(options,runRoot,index) {
  const supplied=readSnapshot(options.engine,{allowMultipleLinks:true});safeNative(supplied);
  let engine;
  if(index) {
    requireThat(same(supplied.identity,index.engine.identity),'Resume must explicitly use the original native engine bytes','core/stale-engine');engine=index.engine;readBound(engine,'original native engine');
    requireThat(options.publicNpm?engine.path===supplied.path && engine.path===options.publicNpm.enginePath:engine.path===path.join(runRoot,'engine','phoenix-diagram'),'Index cannot select an arbitrary executable path','core/untrusted-index');
  } else if(options.publicNpm) {
    requireThat(supplied.path===options.publicNpm.enginePath,'Public npm transport must bind the installed native engine');engine=bound(supplied);
  } else {
    const directory=privateDirectory(path.join(runRoot,'engine'));const file=path.join(directory,'phoenix-diagram');writeNew(file,supplied.content,0o700);engine={...bound(readSnapshot(file)),source:bound(supplied)};
  }
  const probe=await invokeNative(engine,['engine-probe'],{options,json:false});
  requireThat(probe.exitCode===0 && probe.stdout.trim()==='phoenix-diagram-engine 0.2.0 input=1 receipt=1 viewer=phoenix-v2','Native engine probe identity did not match the supported contracts','core/engine-probe');
  const doctor=await invokeNative(engine,['doctor','--json'],{options});const declared=doctor.receipt?.engine;
  requireThat(declared && /^[0-9a-f]{64}$/.test(declared.sourceInputSha256) && declared.inputContractSchema===1 && declared.receiptSchema===1 && declared.viewerRevision==='phoenix-v2' && same(declared.binary,engine.identity),'Doctor did not bind the actual native binary and source identity','core/engine-identity');
  if(index)requireThat(same(declared,index.engine.engineIdentity),'Engine identity changed during resume','core/stale-engine');
  return{...engine,engineIdentity:declared,probe:probe.stdout.trim(),doctor:{status:doctor.receipt.status,browser:doctor.receipt.browser,platform:doctor.receipt.platform}};
}

function material(options) {
  const definitions=options.caseManifest?JSON.parse(readSnapshot(options.caseManifest).content.toString('utf8')).cases:DEFAULT_CASES;
  const coverage=requiredCoverage(definitions);
  const cases=definitions.map(entry=>{
    const source=readSnapshot(path.resolve(options.caseManifest?path.dirname(path.resolve(options.caseManifest)):root,entry.source));
    const document=JSON.parse(source.content.toString('utf8'));
    requireThat(document.diagram_type===entry.type,'Source case diagram_type does not match its declared type','core/source-case');
    if(entry.contract==='phoenix-v1')requireThat(document.meta?.input_contract==='phoenix-v1','New source cases must persist meta.input_contract=phoenix-v1','core/source-contract');
    else requireThat(document.meta?.input_contract===undefined || document.meta.input_contract==='legacy-2.17','Legacy source must not silently use the new contract','core/source-contract');
    return{...entry,source,document};
  });
  return{cases,coverage};
}

async function executeCase(definition,index,options) {
  const caseRoot=privateDirectory(path.join(index.runRoot,'cases',definition.id));
  const candidateFile=writeNew(path.join(caseRoot,'candidate.json'),definition.source.content);
  const artifactPath=path.join(caseRoot,'diagram.html');
  const result={id:definition.id,type:definition.type,contract:definition.contract,root:caseRoot,source:bound(definition.source),candidate:candidateFile,artifactPath,reader:{status:'not-run'},review:{status:'required-pending'}};
  try {
    const native=await invokeNative(index.engine,['finalize',definition.type,candidateFile.path,artifactPath,'--out-dir',path.join(caseRoot,'browser'),'--json'],{options});
    result.process=writeNew(path.join(caseRoot,'finalize-process.json'),{exitCode:native.exitCode,signal:native.signal,stdout:native.stdout,stderr:native.stderr});
    result.nativeExitCode=native.exitCode;
    requireThat(native.receipt,'Native finalize did not return a machine receipt','core/native-receipt');
    result.nativeExitCode=native.exitCode;result.state=inspectFinalize(native.receipt,{candidate:candidateFile,engine:index.engine.engineIdentity,contract:definition.contract,type:definition.type});
    if(result.state.automatedStatus==='pass')requireThat(result.state.artifact.path===artifactPath,'Finalize returned a different output path');
    result.finalize=native.receipt.evidence?.summaryReceipt || null;
    requireThat(result.state.automatedStatus!=='pass' || native.exitCode===0,'Automatic pass cannot conceal a native command failure');
    readBound(candidateFile,'unchanged candidate snapshot');readBound(result.source,'unchanged source example');
    if(result.state.automatedStatus==='pass') {
      requireThat(native.receipt.stages[1].receipt.inputContract===definition.contract && native.receipt.stages[1].receipt.compatibility?.coreCompatibility==='pass','Selected source contract or complete core compatibility was not preserved');
      await replayDeterminism(result,index,options,{output:path.join(caseRoot,'determinism.html')});
    } else result.determinism={status:'not-run'};
  } catch(error) {result.error=diagnostic(error);result.nativeExitCode=error.exitCode || 1;result.state={...(result.state || {}),automatedStatus:'failed',visualReview:'required-pending',deliveryStatus:'incomplete',v9Complete:false};}
  return result;
}

async function recheckOriginal(entry,index,options,invoke) {
  const saved=JSON.parse(readBound(entry.finalize,'original finalize summary').content.toString('utf8')),original=saved.stages[2].receipt;
  const actual=await invoke(index.engine,['check',entry.state.artifact.path,'--strict-provenance','--json'],{options}),checked=actual.receipt;
  requireThat(actual.exitCode===0 && checked?.command==='check' && checked.status==='pass' && checked.scope==='strict-provenance' && checked.provenance==='current' && checked.deliveryReceiptId===entry.state.deliveryReceiptId && same(checked.engine,index.engine.engineIdentity) && checked.configSha256===entry.state.configSha256 && same(checked.candidate,entry.state.candidate) && same(checked.artifact,entry.state.artifact) && same(checked.deliveryReceipt,entry.state.deliveryReceipt) && same(checked.checks,original.checks) && same(checked.composition,original.composition),'Original delivery failed the actual public strict identity/configuration check on resume');
  readBound(entry.candidate,'strict candidate');readBound(entry.state.artifact,'strict artifact');readBound(entry.state.deliveryReceipt,'strict original delivery receipt');
  entry.currentCheck=writeNew(path.join(entry.root,`current-strict-${randomBytes(8).toString('hex')}.json`),checked);
}

async function replayDeterminism(entry,index,options,{invoke=invokeNative,output}={}) {
  readBound(entry.candidate,'deterministic candidate');readBound(entry.source,'deterministic source example');
  const summary=JSON.parse(readBound(entry.finalize,'selected replay configuration').content.toString('utf8')),config=summary.stages[1].receipt.config;
  const duplicate=output || path.join(entry.root,`determinism-resume-${randomBytes(8).toString('hex')}.html`);
  requireThat(path.dirname(duplicate)===entry.root && duplicate!==entry.artifactPath && !fs.existsSync(duplicate),'Deterministic replay requires a new independent output');
  const second=await invoke(index.engine,['deliver',entry.type,entry.candidate.path,duplicate,'--input-contract',entry.contract,'--quality',config.quality,'--json'],{options});
  requireThat(second.exitCode===0 && second.receipt?.status==='pass' && second.receipt.commitState==='committed','Deterministic replay failed to deliver the same frozen candidate');
  const original=readBound(entry.state.artifact,'original artifact'),again=readSnapshot(duplicate);
  requireThat(same(original.identity,again.identity) && original.content.equals(again.content),'Identical engine/input/config produced different HTML bytes');
  requireThat(second.receipt.receiptId!==entry.state.deliveryReceiptId && second.receipt.configSha256===entry.state.configSha256 && same(second.receipt.engine,index.engine.engineIdentity) && second.receipt.inputContract===entry.contract,'Replay must preserve input contract/config/engine and create a distinct delivery identity');
  for(const forbidden of [index.runRoot,entry.candidate.path,index.engine.path,entry.state.deliveryReceiptId])requireThat(!original.content.includes(Buffer.from(forbidden)),'A path or random receipt identity leaked into deterministic HTML');
  await recheckOriginal(entry,index,options,invoke);
  entry.determinism={status:'pass',artifact:bound(again),delivery:writeNew(path.join(entry.root,`determinism-delivery-${randomBytes(8).toString('hex')}.json`),second.receipt),originalCheck:entry.currentCheck,sameHtml:true,distinctDeliveryReceiptId:second.receipt.receiptId,inputIdentityPreserved:true};
}

// Repair a controller ledger only from the original bound native process and
// sealed summary. A fresh finalize would invalidate the images being reviewed.
export async function resumeCaseEvidence(index,options,{invoke=invokeNative}={}) {
  refreshEvidence(index);
  for(const entry of index.cases) {
    let recovered,active=entry;
    if(entry.state?.automatedStatus==='failed' && entry.error?.code==='core/evidence') {
      requireThat(entry.process?.path===path.join(entry.root,'finalize-process.json') && entry.process.binding,'Missing original bound native finalize process');
      const captured=JSON.parse(readBound(entry.process,'original native finalize process').content.toString('utf8'));
      requireThat(captured.exitCode===0 && captured.signal===null && typeof captured.stdout==='string' && typeof captured.stderr==='string','A failed native process cannot be recovered as an automatic pass');
      const receipt=JSON.parse(captured.stdout);
      requireThat(receipt.automatedStatus==='pass' && receipt.artifact?.path===entry.artifactPath && (!entry.finalize || same(entry.finalize,receipt.evidence?.summaryReceipt)),'Native finalize did not bind the original passing artifact and summary');
      const state=inspectFinalize(receipt,{candidate:entry.candidate,engine:index.engine.engineIdentity,contract:entry.contract,type:entry.type});
      requireThat(readBound(entry.source,'original source example').content.equals(readBound(entry.candidate,'original candidate').content),'Original input identity changed before ledger recovery');
      recovered={process:entry.process,summary:state.summary,previousFailure:{error:entry.error,state:entry.state,nativeExitCode:entry.nativeExitCode}};
      active={...entry,state,finalize:state.summary};
    }
    if(active.state?.automatedStatus!=='pass')continue;
    await recheckOriginal(active,index,options,invoke);
    if(recovered) {
      await replayDeterminism(active,index,options,{invoke});
      Object.assign(entry,active);
      entry.ledgerRecovery={status:'pass',...recovered};entry.nativeExitCode=0;delete entry.error;
    }
  }
  return index;
}

async function contentArtifact(index,options) {
  const original=index.cases.find(entry=>entry.type==='architecture' && entry.contract==='phoenix-v1');
  if(!original)return{status:'not-run',reason:'No new architecture source case was selected'};
  const directory=privateDirectory(path.join(index.runRoot,'content'));const document=JSON.parse(readBound(original.candidate,'content source candidate').content.toString('utf8'));
  document.meta.subtitle=XSS_LITERAL;const candidate=writeNew(path.join(directory,'candidate.json'),Buffer.from(`${JSON.stringify(document,null,2)}\n`));
  const output=path.join(directory,'malicious-text.html');
  const native=await invokeNative(index.engine,['deliver','architecture',candidate.path,output,'--json'],{options});
  requireThat(native.exitCode===0 && native.receipt?.status==='pass','Native malicious-text content fixture failed serialization or artifact checks');
  const artifact=bound(readSnapshot(output));assertXssArtifact(readBound(artifact,'malicious content artifact').content);
  const strict=await invokeNative(index.engine,['check',output,'--strict-provenance','--json'],{options});requireThat(strict.exitCode===0 && strict.receipt?.provenance==='current','Malicious content artifact is missing strict provenance');
  return{status:'pending-browser',candidate,artifact,serialization:{status:'pass'},browser:{status:'not-run'},expectedLiteral:XSS_LITERAL,sentinelName:'__phoenixDiagramXss'};
}

async function newDeliveryArtifact(index,options) {
  const original=index.cases.find(entry=>entry.type==='architecture' && entry.contract==='phoenix-v1' && entry.state?.automatedStatus==='pass');
  if(!original)return{status:'not-run',reason:'A passing new architecture delivery is required'};
  const directory=privateDirectory(path.join(index.runRoot,'new-delivery')),output=path.join(directory,'diagram.html');
  const native=await invokeNative(index.engine,['finalize','architecture',original.candidate.path,output,'--out-dir',path.join(directory,'browser'),'--json'],{options});
  requireThat(native.exitCode===0,'A fresh independent finalize failed');
  const state=inspectFinalize(native.receipt,{candidate:original.candidate,engine:index.engine.engineIdentity});
  requireThat(state.automatedStatus==='pass' && state.deliveryReceiptId!==original.state.deliveryReceiptId && !same(state.browserReceiptIdentity,original.state.browserReceiptIdentity) && same(state.artifact.identity,original.state.artifact.identity),'A new finalize did not produce a distinct pending delivery/browser identity for identical HTML');
  return{status:'pending-review-proof',caseId:original.id,state,finalize:state.summary};
}

// Mutate only the owned test file and restore only the bytes this controller
// wrote. An unexpected replacement or edit is retained and fails the rehearsal.
export async function withMutatedFile(reference,bytes,operation) {
  const original=readBound(reference,'original mutation fixture'),fd=fs.openSync(reference.path,fs.constants.O_RDWR|(fs.constants.O_NOFOLLOW || 0));
  const metadata=fs.fstatSync(fd),read=()=>{const data=Buffer.alloc(fs.fstatSync(fd).size);let at=0;while(at<data.length){const count=fs.readSync(fd,data,at,data.length-at,at);requireThat(count>0,'Mutation fixture changed during read');at+=count;}return data;};
  const replace=data=>{let at=0;while(at<data.length)at+=fs.writeSync(fd,data,at,data.length-at,at);fs.ftruncateSync(fd,data.length);fs.fsyncSync(fd);};
  let changed=false;
  try {
    requireThat(String(metadata.dev)===original.binding.device && String(metadata.ino)===original.binding.inode && read().equals(original.content),'Mutation fixture physical binding changed');
    replace(bytes);changed=true;return await operation();
  } finally {
    try {
      if(changed) {
        const current=fs.lstatSync(reference.path);
        requireThat(current.isFile() && !current.isSymbolicLink() && current.dev===metadata.dev && current.ino===metadata.ino && read().equals(bytes),'A later claimant changed the known mutation; preserve it for inspection');
        replace(original.content);readBound(reference,'restored original mutation fixture');
      }
    } finally {fs.closeSync(fd);}
  }
}

async function rehearseReviewBindings(index,entry,record,options,invoke) {
  const summary=readBound(entry.finalize,'original summary before negative review'),value=JSON.parse(summary.content.toString('utf8'));
  const directory=privateDirectory(path.join(entry.root,`review-rejections-${randomBytes(8).toString('hex')}`)),checks=[];
  requireThat(!fs.existsSync(value.evidence.reviewPath),'Review rejection scopes must precede sealing the actual review record');
  const refuse=async(name,file)=>{
    const result=await invoke(index.engine,['review-record',entry.state.artifact.path,'--record',file.path,'--json'],{options});
    requireThat([1,3].includes(result.exitCode) && result.receipt?.command==='review-record' && result.receipt.status==='failed' && result.receipt.deliveryStatus==='incomplete' && result.receipt.visualReview.status!=='passed','A changed review binding or scope was accepted');
    requireThat(!fs.existsSync(value.evidence.reviewPath),'A rejected review unexpectedly sealed a record');readBound(bound(summary),'summary unchanged after rejected review');
    checks.push({name,status:'pass',exitCode:result.exitCode,receipt:writeNew(path.join(directory,`${name}.json`),result.receipt)});
  };
  for(const [name,file] of [['candidate',entry.candidate],['html',entry.state.artifact],['browser',entry.state.browserReceipt],['screenshot',entry.state.screenshots[0]]]) {
    const original=readBound(file,`${name} negative fixture`);await withMutatedFile(file,Buffer.concat([original.content,Buffer.from('\ncontrolled binding change\n')]),()=>refuse(name,record));
  }
  const changeHash=value=>(value.startsWith('0')?'1':'0')+value.slice(1);
  for(const [name,mutate] of [
    ['config',value=>{value.configSha256=changeHash(value.configSha256);}],
    ['engine',value=>{value.engine.sourceInputSha256=changeHash(value.engine.sourceInputSha256);}],
    ['browser-record-binding',value=>{value.browserReceiptIdentity.sha256=changeHash(value.browserReceiptIdentity.sha256);}],
    ['delivery-record-binding',value=>{value.deliveryReceiptId='00000000-0000-4000-8000-000000000000';}],
    ['review-scope',value=>{value.scope=[];}],
    ['screenshot-observation',value=>{value.screenshotIdentities[0].observed=false;}],
  ]) {
    const changed=JSON.parse(readBound(record,'original actual review').content.toString('utf8'));mutate(changed);
    const draft=writeNew(path.join(directory,`${name}-input.json`),changed);await refuse(name,draft);
  }
  return{status:'pass',caseId:entry.id,scope:'actual candidate/HTML/browser/screenshot changes and reviewer config/engine/delivery/browser/scope bindings',checks};
}

export async function resumeReviews(index,options,{invoke=invokeNative}={}) {
  if(!options.reviewFile)return index;
  const map=JSON.parse(readSnapshot(options.reviewFile).content.toString('utf8'));requireThat(map && map.records && typeof map.records==='object' && !Array.isArray(map.records),'Review map must contain a records object keyed by case identity');
  const ids=new Set(index.cases.map(entry=>entry.id));for(const id of Object.keys(map.records))requireThat(ids.has(id),`Review map names an unknown case ${id}`);
  for(const entry of index.cases) {
    if(!Object.hasOwn(map.records,entry.id))continue;
    requireThat(entry.state?.automatedStatus==='pass' && entry.finalize,'Review cannot fill a not-run or failed automatic gate');
    readBound(entry.candidate,'candidate');readBound(entry.state.artifact,'artifact');readBound(entry.state.deliveryReceipt,'delivery receipt');readBound(entry.state.browserReceipt,'browser receipt');for(const screenshot of entry.state.screenshots)readBound(screenshot,'screenshot');
    const current=readSnapshot(entry.finalize.path),original=JSON.parse(current.content.toString('utf8'));
    requireThat(original.deliveryReceiptId===entry.state.deliveryReceiptId && same(original.browserReceiptIdentity,entry.state.browserReceiptIdentity) && same(original.engine,entry.state.engine) && original.configSha256===entry.state.configSha256,'Review cannot adopt another delivery after an interrupted index update');
    const recordPath=map.records[entry.id];requireThat(typeof recordPath==='string' && path.isAbsolute(recordPath),'Review records require explicit absolute paths');
    const record=readSnapshot(recordPath);
    if(index.kind===INDEX_KIND && JSON.parse(record.content.toString('utf8')).verdict==='passed') {
      if(index.reviewBindings?.status!=='pass' && original.visualReview.status==='required-pending')index.reviewBindings=await rehearseReviewBindings(index,entry,bound(record),options,invoke);
      if(index.newDelivery?.status==='pending-review-proof' && index.newDelivery.caseId===entry.id) {
        const alternate=index.newDelivery,result=await invoke(index.engine,['review-record',alternate.state.artifact.path,'--record',record.path,'--json'],{options});
        requireThat(result.exitCode===1 && result.receipt?.status==='failed' && result.receipt.deliveryStatus==='incomplete','A new delivery inherited a review for the original delivery/browser identity');
        const current=JSON.parse(readBound(alternate.finalize,'new pending finalize summary').content.toString('utf8'));
        requireThat(current.visualReview.status==='required-pending' && current.deliveryStatus==='incomplete','Rejected old review changed the new delivery pending state');
        alternate.status='pass';alternate.oldReviewRejected={status:'pass',exitCode:result.exitCode,receipt:writeNew(path.join(index.runRoot,'new-delivery','old-review-rejected.json'),result.receipt)};
      }
    }
    if(index.kind===INDEX_KIND && index.reviewRecovery?.status!=='pass' && original.visualReview.status==='required-pending' && JSON.parse(record.content.toString('utf8')).verdict==='passed')index.reviewRecovery={caseId:entry.id,...await rehearseSummaryFailure(index,entry,bound(record),options,{invoke})};
    const result=await invoke(index.engine,['review-record',entry.state.artifact.path,'--record',record.path,'--json'],{options});
    requireThat(result.exitCode===0,'Native review-record did not accept and refresh the current summary');
    const transition=verifyReviewTransition(original,result.receipt);
    entry.review={status:transition.visualReview,record:transition.record};entry.state={...entry.state,...transition};entry.finalize=transition.summary;
    if(index.reviewRecovery?.caseId===entry.id && index.reviewRecovery.status==='pass') {
      requireThat(same(index.reviewRecovery.record,transition.record),'Summary retry changed the sealed record identity');index.reviewRecovery.retryStatus='pass';index.reviewRecovery.sameDeliveryReceiptId=transition.deliveryReceiptId;index.reviewRecovery.sameBrowserReceiptIdentity=transition.browserReceiptIdentity;
    }
    if(index.kind===INDEX_KIND) {
      const repeated=await invoke(index.engine,['review-record',entry.state.artifact.path,'--record',record.path,'--json'],{options});requireThat(repeated.exitCode===0,'The same original review record was not idempotent');
      const idempotent=verifyReviewTransition(original,repeated.receipt);requireThat(same(idempotent.summary,transition.summary) && same(idempotent.record,transition.record),'Idempotent review changed the sealed record or summary bytes');
      entry.review.idempotent={status:'pass',receipt:writeNew(path.join(entry.root,`review-repeat-${randomBytes(8).toString('hex')}.json`),repeated.receipt)};
    }
    readBound(bound(record),'unchanged review input');
  }
  return index;
}

export function verifyReaderReport(report,artifact,{offline=false,content=false,bytesFor}={}) {
  const read=(reference,label)=>({content:readEvidenceBytes(reference,{bytesFor,label})});
  requireThat(report?.schemaVersion===1 && report.kind===(content?'real-browser-content':'real-browser-export') && report.status==='pass' && report.artifact?.path===artifact.path && same({sha256:report.artifact.sha256,bytes:report.artifact.bytes},artifact.identity),'Reader receipt is stale or belongs to another artifact');
  requireThat(/(?:Chrome|Chromium)\//.test(report.actualBrowserVersion || '') && report.resources?.status==='pass' && report.resources.requests?.every(item=>!/^(?:https?|wss?):/i.test(item.url)) && report.resources.loadingFailed?.length===0 && report.errors?.length===0,'Actual browser or complete network/resource scope is missing');
  if(offline)requireThat(report.offline?.requested===true && report.offline.status==='pass' && report.offline.configured===true && report.offline.method==='cdp-network-emulation' && report.offline.remoteRequests.length===0,'Offline browser scope is missing');
  if(content) {requireThat(report.content?.status==='pass' && report.content.literalPresent===true && report.content.sentinelDefined===false,'Actual malicious content was omitted or executed');return;}
  const seen=new Set();requireThat(report.exports?.length===20,'Required export scope is incomplete');
  for(const file of report.exports) {
    const key=`${file.theme}:${file.mode}:${file.format}`;requireThat(['light','dark'].includes(file.theme) && ['still','live'].includes(file.mode) && ['svg','png','jpeg','webp','webm'].includes(file.format) && !seen.has(key) && file.status==='pass','Required export scope is duplicated or not passing');seen.add(key);read(file,'actual export file');
  }
  requireThat(report.stateRestoration?.status==='pass' && ['success','permissionDenied','capabilityUnavailable'].every(key=>report.clipboard?.[key]?.status==='pass'),'Reader state or clipboard success/permission/capability scope is missing');
  const clipboard=report.clipboard.success;read({path:clipboard.path,identity:{sha256:clipboard.sha256,bytes:clipboard.bytes}},'actual clipboard PNG');read(report.clipboard.capabilityUnavailable.fallback,'clipboard capability fallback');
  const print=report.stateRestoration.print,printed=new Set();
  requireThat(print?.status==='pass' && print.method===REQUIRED_EXPORT_SCOPE.print.method && Array.isArray(print.observations) && print.observations.length===4,'Required print scope is incomplete');
  for(const sample of print.observations) {
    const key=`${sample.theme}:${sample.mode}`;
    requireThat(sample.status==='pass' && REQUIRED_EXPORT_SCOPE.print.themes.includes(sample.theme) && REQUIRED_EXPORT_SCOPE.print.modes.includes(sample.mode) && !printed.has(key),'Required print scope is duplicated or not passing');printed.add(key);
    verifyPrintSample(sample);
    const pdf=read(sample.nativePrint.pdf,'actual native print PDF').content;
    requireThat(pdf.subarray(0,5).toString()==='%PDF-' && pdf.subarray(-64).includes(Buffer.from('%%EOF')) && (pdf.toString('latin1').match(/\/Type\s*\/Page\b/g) || []).length===sample.nativePrint.pdf.pages,'Native print PDF bytes differ from the reported scope');
  }
  verifyMotionSuppression(report.motionSuppression);
  verifyReaderInteractions(report.readerInteractions);
}

export async function rehearseSummaryFailure(index,entry,record,options,{invoke=invokeNative}={}) {
  if(process.getuid()===0)return{status:'unavailable',reason:'Permission-denial acceptance requires an unprivileged native process'};
  const original=readBound(entry.finalize,'original finalize summary'),summary=JSON.parse(original.content.toString('utf8'));
  const parent=entry.root,metadata=fs.lstatSync(parent),reviewPath=summary.evidence?.reviewPath;
  requireThat(metadata.isDirectory() && !metadata.isSymbolicLink() && metadata.uid===process.getuid() && (metadata.mode&0o7777)===0o700 && path.dirname(entry.state.artifact.path)===parent,'Summary failure may only change this run owned artifact parent permissions');
  requireThat(typeof reviewPath==='string' && path.dirname(reviewPath)!==parent && path.dirname(path.dirname(reviewPath))===parent,'The sealed record must use the independent owned browser evidence directory');
  readBound(record,'review input');let failed;
  try {
    fs.chmodSync(parent,0o500);
    failed=await invoke(index.engine,['review-record',entry.state.artifact.path,'--record',record.path,'--json'],{options});
  } finally {
    const current=fs.lstatSync(parent);requireThat(current.dev===metadata.dev && current.ino===metadata.ino && current.uid===metadata.uid,'Owned artifact parent changed during the permission rehearsal');fs.chmodSync(parent,0o700);
  }
  requireThat(failed.exitCode===4 && failed.receipt?.status==='failed' && failed.receipt.automatedStatus==='pass' && failed.receipt.diagnostics?.some(item=>/Permission denied|EACCES|os error 13/i.test(item.message)),'The real summary publication did not fail with a permission error after validation');
  const sealed=readBound(failed.receipt.evidence?.visualReviewRecord,'record retained after summary failure');
  requireThat(sealed.path===reviewPath && sealed.content.equals(readBound(record,'same review input').content) && same(failed.receipt.visualReview?.record,{path:sealed.path,identity:sealed.identity}),'Summary failure did not retain the exact independently sealed record');
  readBound(bound(original),'unchanged summary after failed write');
  const receipt=writeNew(path.join(parent,`summary-write-failure-${randomBytes(8).toString('hex')}.json`),failed.receipt);
  return{status:'pass',fault:'owned artifact parent chmod 0500; native EACCES',receipt,record:{path:sealed.path,identity:sealed.identity},summaryUnchanged:true,retryStatus:'not-run'};
}

export function refreshEvidence(index) {
  if(index.ledgerRecoveryCheckpoint)readBound(index.ledgerRecoveryCheckpoint,'immutable recovered ledger checkpoint');
  for(const entry of index.cases) {
    requireThat(entry.root===path.join(index.runRoot,'cases',entry.id) && entry.candidate.path===path.join(entry.root,'candidate.json') && entry.artifactPath===path.join(entry.root,'diagram.html'),'Case ledger cannot select paths outside its owned run directory');
    readBound(entry.candidate,'case candidate');readBound(entry.source,'original source example');
    if(entry.process)readBound(entry.process,'original native finalize process');
    if(entry.state?.automatedStatus==='pass') {
      const current=readSnapshot(entry.finalize.path),summary=JSON.parse(current.content.toString('utf8'));
      requireThat(summary.deliveryReceiptId===entry.state.deliveryReceiptId && same(summary.browserReceiptIdentity,entry.state.browserReceiptIdentity) && same(summary.engine,index.engine.engineIdentity) && summary.configSha256===entry.state.configSha256,'A later finalize cannot replace this run delivery or browser identity');
      const state=inspectFinalize({...summary,evidence:{...summary.evidence,summaryReceipt:bound(current)}},{candidate:entry.candidate,artifact:entry.state.artifact,engine:index.engine.engineIdentity,contract:entry.contract,type:entry.type,allowReviewed:true});
      if(state.visualReview!=='required-pending') {
        const record=readBound(summary.visualReview.record,'current visual review record');
        const value=JSON.parse(record.content.toString('utf8'));
        requireThat(value.deliveryReceiptId===state.deliveryReceiptId && same(value.browserReceiptIdentity,state.browserReceiptIdentity) && value.verdict===state.visualReview,'Saved review no longer proves the current delivery');
      }
      entry.state={...entry.state,...state};entry.finalize=bound(current);
      if(entry.determinism?.status==='pass')readBound(entry.determinism.artifact,'determinism artifact');
    }
    if(entry.reader?.status==='pass') {
      const report=JSON.parse(readBound(entry.reader.receipt,'reader receipt').content.toString('utf8'));verifyReaderReport(report,entry.state.artifact,{offline:true});
    }
    if(entry.review?.idempotent?.status==='pass')readBound(entry.review.idempotent.receipt,'idempotent review receipt');
    if(entry.currentCheck)readBound(entry.currentCheck,'current strict check receipt');
  }
  if(index.content?.artifact) {
    readBound(index.content.candidate,'content candidate');readBound(index.content.artifact,'content artifact');
    assertXssArtifact(readBound(index.content.artifact,'content HTML').content);
    if(index.content.browser?.status==='pass')verifyReaderReport(JSON.parse(readBound(index.content.browser.receipt,'content receipt').content.toString('utf8')),index.content.artifact,{content:true});
  }
  if(index.newDelivery?.state?.automatedStatus==='pass') {
    const alternate=index.newDelivery,stored=readBound(alternate.finalize,'new delivery summary');
    inspectFinalize({...JSON.parse(stored.content.toString('utf8')),evidence:{...JSON.parse(stored.content.toString('utf8')).evidence,summaryReceipt:bound(stored)}},{candidate:alternate.state.candidate,artifact:alternate.state.artifact,engine:index.engine.engineIdentity});
    if(alternate.status==='pass')readBound(alternate.oldReviewRejected.receipt,'old review rejection receipt');
  }
  if(index.reviewBindings?.status==='pass')for(const check of index.reviewBindings.checks)readBound(check.receipt,'review binding rejection receipt');
  if(index.reviewRecovery?.status==='pass') {readBound(index.reviewRecovery.receipt,'summary write failure receipt');readBound(index.reviewRecovery.record,'retained visual record');}
  if(index.recovery?.status==='pass')for(const scenario of index.recovery.scenarios) {
    readBound(scenario.journal,'durable recovery journal copy');
    for(const key of ['recovery','repeat','strict'])if(scenario[key]?.receipt)readBound(scenario[key].receipt,'public recovery command receipt');
    if(scenario.rediscoveredRecord)readBound(scenario.rediscoveredRecord,'rediscovered durable journal copy');
    for(const reference of scenario.retained || [])readBound(reference,'retained conflict evidence');
    if(scenario.finalPublic?.absent)requireThat(!fs.existsSync(scenario.finalPublic.path),'A later claimant changed the expected absent scenario output');else if(scenario.finalPublic)readBound(scenario.finalPublic,'recovery scenario public file');
  }
  // Every file a passed scope bound (path, identity and physical binding) must still be current.
  const rebind=value=>{
    if(Array.isArray(value)){value.forEach(rebind);return;}
    if(!value || typeof value!=='object')return;
    if(typeof value.path==='string' && value.identity && value.binding)readBound(value,'scope evidence');
    Object.values(value).forEach(rebind);
  };
  for(const name of SCOPES)if(index[name]?.status==='pass')rebind(index[name]);
  if(index.offline?.status==='pass') {
    requireThat(index.offline.cases.length===index.cases.length,'Offline case scope is incomplete');
    for(const sample of index.offline.cases) {
      readBound(sample.native.artifact,'OS-denied native artifact');readBound(sample.native.delivery,'OS-denied native delivery receipt');readBound(sample.native.check,'OS-denied strict receipt');
      verifyReaderReport(JSON.parse(readBound(sample.browser.receipt,'offline reader receipt').content.toString('utf8')),sample.native.artifact,{offline:true});
    }
  }
}

async function runReader(index,options) {
  const reader=await import('../tools/phoenix-diagram/tests/export-acceptance.mjs');
  for(const entry of index.cases) {
    if(entry.state?.automatedStatus!=='pass' || entry.reader?.status==='pass')continue;
    readBound(entry.state.artifact,'reader artifact');
    const evidence=privateDirectory(path.join(entry.root,`reader-${randomBytes(8).toString('hex')}`));
    const result=await reader.runExportAcceptance({artifactPath:entry.state.artifact.path,evidenceDir:evidence,browserPath:options.browserPath,offline:true});
    const receipt=bound(readSnapshot(result.receiptPath));requireThat(result.artifact?.path===entry.state.artifact.path && same({sha256:result.artifact.sha256,bytes:result.artifact.bytes},entry.state.artifact.identity),'Reader receipt belongs to another artifact');
    if(result.status==='pass')verifyReaderReport(result,entry.state.artifact,{offline:true});
    entry.reader={status:result.status,receipt,exports:result.exports?.map(item=>({format:item.format,theme:item.theme,mode:item.mode,status:item.status,path:item.path,identity:item.identity})),errors:result.errors};
  }
  if(index.content?.artifact && index.content.browser?.status!=='pass') {
    readBound(index.content.artifact,'content artifact');
    const evidence=privateDirectory(path.join(index.runRoot,'content',`reader-${randomBytes(8).toString('hex')}`));
    const result=await reader.runContentAcceptance({artifactPath:index.content.artifact.path,evidenceDir:evidence,expectedLiteral:index.content.expectedLiteral,sentinelName:index.content.sentinelName,browserPath:options.browserPath,offline:true});
    requireThat(result.artifact?.path===index.content.artifact.path && same({sha256:result.artifact.sha256,bytes:result.artifact.bytes},index.content.artifact.identity),'Content browser receipt belongs to another artifact');
    if(result.status==='pass')verifyReaderReport(result,index.content.artifact,{content:true});
    index.content.browser={status:result.status,receipt:bound(readSnapshot(result.receiptPath)),errors:result.errors};index.content.status=result.status;
  }
}

export function recomputeIndex(index) {
  const states=index.cases.map(entry=>entry.state || {automatedStatus:'not-run',visualReview:'required-pending',deliveryStatus:'incomplete'});
  const automatic=states.every(state=>state.automatedStatus==='pass')?'pass':states.some(state=>state.automatedStatus==='failed')?'failed':states.some(state=>state.automatedStatus==='unavailable')?'unavailable':'not-run';
  const complete=states.every(state=>state.deliveryStatus==='complete' && state.visualReview==='passed');
  const phaseNames=['content','recovery','offline','reader','review','reviewRecovery','reviewBindings','newDelivery',...SCOPES];
  const requestedFailures=index.cases.some(entry=>entry.reader?.status==='failed') || phaseNames.some(key=>index[key]?.status==='failed');
  const unavailable=index.cases.some(entry=>entry.reader?.status==='unavailable') || phaseNames.some(key=>index[key]?.status==='unavailable');
  const allScopes=index.coverage.complete && automatic==='pass' && complete && index.cases.every(entry=>entry.determinism?.status==='pass' && entry.reader?.status==='pass' && entry.review?.idempotent?.status==='pass') && ['content','recovery','offline','reviewBindings','newDelivery',...(index.requiredScopes || SCOPES)].every(key=>index[key]?.status==='pass') && index.reviewRecovery?.status==='pass' && index.reviewRecovery.retryStatus==='pass';
  const status=automatic==='failed' || requestedFailures?'failed':allScopes?'pass':'incomplete';
  const exitCode=status==='failed'?1:automatic==='unavailable'?5:unavailable?4:0;
  return{...index,status,automatedStatus:automatic,visualReview:complete?'passed':states.some(state=>state.visualReview==='failed')?'failed':states.some(state=>state.visualReview==='unavailable')?'unavailable':'required-pending',handoff:allScopes?'passed':'incomplete',exitCode};
}

export async function runCoreStage(options) {
  let index,previous;
  if(options.resume) {
    const snapshot=readSnapshot(options.resume);previous=bound(snapshot);index=JSON.parse(snapshot.content.toString('utf8'));
    requireThat(index.schemaVersion===1 && index.kind===INDEX_KIND && index.stage==='core','Unsupported core index');
    index.runRoot=privateDirectory(index.runRoot,{existing:true});requireThat(path.dirname(snapshot.path)===index.runRoot,'Index must be inside its original run directory');
    index.engine=await prepareEngine(options,index.runRoot,index);index.coverage=requiredCoverage(index.cases);
  } else {
    requireThat(!options.requiredScopes || options.requiredScopes.every(name=>SCOPES.includes(name) && options[name]),'Required acceptance scopes must be known and requested');
    const inputs=material(options);
    const directory=options.outputDir?path.resolve(options.outputDir):fs.mkdtempSync(path.join(os.tmpdir(),'phoenix-diagram-core-'));
    const runRoot=privateDirectory(directory,{existing:!options.outputDir});privateDirectory(path.join(runRoot,'cases'));
    index={...header('test-phoenix-diagram'),kind:INDEX_KIND,stage:'core',runRoot,coverage:inputs.coverage,cases:[],content:{status:'not-run'},recovery:{status:'not-run'},offline:{status:'not-run'},reviewRecovery:{status:'not-run'},reviewBindings:{status:'not-run'},newDelivery:{status:'not-run'},...Object.fromEntries(SCOPES.map(name=>[name,{status:'not-run'}])),...(options.requiredScopes?{requiredScopes:[...options.requiredScopes]}:{}),diagnostics:[]};
    index.engine=await prepareEngine(options,runRoot);
    for(const definition of inputs.cases)index.cases.push(await executeCase(definition,index,options));
    try{index.content=await contentArtifact(index,options);}catch(error){index.content={status:'failed',diagnostics:[diagnostic(error)]};}
    try{index.newDelivery=await newDeliveryArtifact(index,options);}catch(error){index.newDelivery={status:'failed',diagnostics:[diagnostic(error)]};}
  }
  if(options.resume) {
    const priorRecoveries=new Set(index.cases.filter(entry=>entry.ledgerRecovery?.status==='pass').map(entry=>entry.id));
    await resumeCaseEvidence(index,options);
    if(index.cases.some(entry=>entry.ledgerRecovery?.status==='pass' && !priorRecoveries.has(entry.id))) {
      // Preserve a reviewable recovery checkpoint before invoking review-record.
      index=persist(recomputeIndex(index),previous);previous=index.indexFile;index.ledgerRecoveryCheckpoint=previous;
    }
  }
  const phase=async(name,operation)=>{try{await operation();if(['reader','review'].includes(name))index[name]={status:'pass',scope:'Requested phase completed; individual case outcomes remain authoritative'};}catch(error){index[name]={status:'failed',exitCode:error.exitCode || 1,diagnostics:[diagnostic(error)]};}};
  if(options.reviewFile)await phase('review',()=>resumeReviews(index,options));
  if(options.reader)await phase('reader',()=>runReader(index,options));
  if(options.recovery)await phase('recovery',async()=>{index.recovery=await(await import('./test-phoenix-diagram-recovery.mjs')).runRecoveryScenarios(index,options,{invoke:invokeNative});});
  if(options.offline)await phase('offline',async()=>{index.offline=await runOffline(index,options);});
  const scopes=await import('./test-phoenix-diagram-scopes.mjs');
  for(const name of SCOPES)if(options[name])await phase(name,async()=>{index[name]=await scopes.run(name,index,options,{invoke:invokeNative});});
  refreshEvidence(index);index=recomputeIndex(index);return persist(index,previous);
}

async function runOffline(index,options) {
  if(index.offline?.status==='pass')return index.offline;
  if(!index.cases.every(entry=>entry.state?.automatedStatus==='pass'))return{status:'not-run',reason:'All selected cases must pass before offline replay'};
  const directory=privateDirectory(path.join(index.runRoot,`offline-${randomBytes(8).toString('hex')}`));
  let prefix;
  if(process.platform==='darwin')prefix=['/usr/bin/sandbox-exec','-p','(version 1) (allow default) (deny network-outbound (remote ip)) (deny network-inbound (local ip))'];
  // Preserve originalBinding.uid across the namespace boundary. See
  // https://man7.org/linux/man-pages/man1/unshare.1.html (--map-current-user).
  else if(process.platform==='linux')prefix=['/usr/bin/unshare','--user','--map-current-user','--net','--'];
  else return{status:'unavailable',reason:'No verified OS deny-network launcher for this host'};
  const reader=await import('../tools/phoenix-diagram/tests/export-acceptance.mjs'),cases=[];
  for(const sample of index.cases) {
    const caseRoot=privateDirectory(path.join(directory,sample.id)),output=path.join(caseRoot,'diagram.html');
    const result=await invokeNative(index.engine,['deliver',sample.type,sample.candidate.path,output,'--json'],{options,prefix});requireThat(result.exitCode===0 && result.receipt?.status==='pass','Real native generation failed under OS network denial');
    const checked=await invokeNative(index.engine,['check',output,'--strict-provenance','--json'],{options,prefix});requireThat(checked.exitCode===0 && checked.receipt?.provenance==='current','Real native strict checking failed under OS network denial');
    const artifact=bound(readSnapshot(output));requireThat(same(artifact.identity,sample.state.artifact.identity),'OS network denial changed deterministic native HTML');
    const report=await reader.runExportAcceptance({artifactPath:artifact.path,evidenceDir:caseRoot,browserPath:options.browserPath,offline:true});
    const receipt=bound(readSnapshot(report.receiptPath));verifyReaderReport(report,artifact,{offline:true});
    cases.push({id:sample.id,status:'pass',native:{status:'pass',artifact,delivery:writeNew(path.join(caseRoot,'native-delivery.json'),result.receipt),check:writeNew(path.join(caseRoot,'native-strict.json'),checked.receipt)},browser:{status:'pass',receipt,offline:report.offline}});
  }
  return{status:'pass',native:{status:'pass',wrapper:prefix,scope:['all-selected-native-generate','all-selected-native-strict-check']},browser:{status:'pass',method:'cdp-network-emulation',scope:['all-selected-reader','all-required-exports','no-remote-requests']},cases,limitation:process.platform==='darwin'?'OS sandbox denies IP for native generation/check; Chrome retains its own sandbox and separately uses CDP offline emulation with request recording. This is not a claim that Chrome ran under sandbox-exec.': 'Native replay uses a network namespace; the independent Chrome run uses CDP offline emulation and request recording.'};
}
