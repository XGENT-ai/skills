import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import net from 'node:net';
import http from 'node:http';
import {execFileSync} from 'node:child_process';
import {setTimeout as delay} from 'node:timers/promises';
import {isDeepStrictEqual as same} from 'node:util';
import {privateDirectory,writeNew,readSnapshot,readBound,bound,identity,jsonBound,proveControllerOwnership} from './p3-state.mjs';

export {proveControllerOwnership};

const OLD_HTML=Buffer.from('The original comparison HTML must survive an uncommitted pair.\n');
const OLD_RECEIPT=Buffer.from('The original comparison receipt must survive an uncommitted pair.\n');
const CLAIMANT=Buffer.from('A later user claimant must remain unchanged.\n');
const PHASES=['compare-before-second-member','compare-before-committed','compare-committed'];
function stopOwned(child,signal){if(child.exitCode!==null||child.signalCode!==null)return;try{process.kill(-child.pid,signal);}catch(error){if(error.code!=='ESRCH')throw error;}}
async function waitFor(operation,label,child,timeoutMs=15000){const deadline=Date.now()+timeoutMs;while(Date.now()<deadline){const value=await operation();if(value)return value;if(child&&(child.exitCode!==null||child.signalCode!==null))throw new Error(label+': owned process exited early');await delay(15);}throw new Error(label+' timed out');}
function ordinaryFile(file){try{return bound(readSnapshot(file));}catch(error){if(error.code==='ENOENT')return null;throw error;}}
function captureProcess(directory,name,args,result){return writeNew(path.join(directory,name+'.json'),{args,exitCode:result.exitCode,signal:result.signal,stdout:writeNew(path.join(directory,name+'.stdout'),result.stdout||''),stderr:writeNew(path.join(directory,name+'.stderr'),result.stderr||''),...(result.actualTransport?{actualTransport:result.actualTransport}:{})});}
async function invokeOwned(invoke,engine,args,settings,directory,name){try{return await invoke(engine,args,settings);}catch(error){if(error.processResult)error.evidence=captureProcess(directory,name+'-failed',args,error.processResult);throw error;}}
function processTable(){return execFileSync('/bin/ps',['-ww','-e','-o','pid=,ppid=,pgid=,args='],{encoding:'utf8',maxBuffer:16*1024*1024}).trim().split('\n').map(line=>{const match=/^\s*(\d+)\s+(\d+)\s+(\d+)\s+(.*)$/.exec(line);return match?{pid:Number(match[1]),ppid:Number(match[2]),pgid:Number(match[3]),command:match[4]}:null;}).filter(Boolean);}

async function ownedController(child,engine,publicNpm,settings={}){
  assert(publicNpm?.enginePath===engine.path,'Public PID observation used another engine');readBound(engine,'public controller executable');
  return waitFor(()=>{const rows=processTable();if(!rows.some(row=>row.pgid===child.pid&&row.command.startsWith(`${engine.path} ${settings.command||'preview'} `)))return null;return proveControllerOwnership(child,engine,rows,settings);},'owned native controller',child);
}
async function terminatePreview(child,engine,publicNpm){
  if(!publicNpm){stopOwned(child,'SIGTERM');return null;}
  const proof=await ownedController(child,engine,publicNpm);process.kill(proof.controllerPid,'SIGTERM');return proof;
}
export async function waitForPairCheckpoint(child,file,{phase,target,timeoutMs=15000,engine,publicNpm}){
  assert(PHASES.includes(phase),'Unknown comparison checkpoint');
  if(publicNpm){assert.equal(publicNpm.enginePath,engine.path);readBound(engine,'checkpoint native executable');}
  return waitFor(()=>{
    let current;try{current=ordinaryFile(file);}catch(error){if(error.code==='core/file-binding')return null;throw error;}if(!current)return null;let marker;try{marker=jsonBound(current,'owned comparison checkpoint');}catch(error){if(error instanceof SyntaxError||['core/file-binding','core/stale-evidence'].includes(error.code))return null;throw error;}
    assert(marker.schemaVersion===1&&Number.isSafeInteger(marker.pid)&&marker.pid>0&&marker.phase===phase&&marker.target===target,'Checkpoint does not identify the owned command/phase/target');
    const ownership=publicNpm?proveControllerOwnership(child,engine,processTable(),{controllerPid:marker.pid,command:'compare'}):null;
    if(!publicNpm)assert.equal(marker.pid,child.pid,'Checkpoint belongs to another native process');
    return{marker:current,pid:marker.pid,phase,target,...(ownership?{ownership}:{})};
  },'comparison checkpoint',child,timeoutMs);
}
export function locatePair(parent,target){
  const candidates=[];
  for(const name of fs.readdirSync(parent).filter(name=>/^\.phoenix-diagram-compare-[a-f0-9]{32}$/.test(name))){
    const directory=path.join(parent,name),stat=fs.lstatSync(directory);assert(stat.isDirectory()&&!stat.isSymbolicLink()&&(stat.mode&0o7777)===0o700&&stat.uid===process.getuid(),'Comparison recovery namespace is not private');
    const names=fs.readdirSync(directory).filter(name=>/^record-\d{8}\.json$/.test(name)).sort();assert(names.length,'Comparison has no durable journal record');
    const record=bound(readSnapshot(path.join(directory,names.at(-1)))),value=jsonBound(record,'durable pair record');
    if(value.schemaVersion===1&&value.tool==='phoenix-diagram'&&value.kind==='architecture-comparison-publication'&&value.targets?.some(entry=>entry.path===target))candidates.push({directory,record,phase:value.phase,targets:value.targets});
  }
  assert.equal(candidates.length,1,'Choose exactly one durable pair record bound to this output; terminal stdout/marker paths are not ownership evidence');return candidates[0];
}

export async function runCompareFaults({engine,runRoot,base,head,invoke,publicNpm}){
  readBound(base,'comparison fault base');readBound(head,'comparison fault head');
  const root=privateDirectory(path.join(runRoot,'compare-faults')),scenarios=[];
  const definitions=[...PHASES.map(phase=>({id:phase,phase})),...['later-claimant','backup-tamper','parent-replacement','same-content-backup-replacement'].map(id=>({id,phase:'compare-before-committed'}))];
  for(const definition of definitions){
    const scenario={...definition,status:'failed'},directory=privateDirectory(path.join(root,definition.id));scenarios.push(scenario);
    const parent=privateDirectory(path.join(directory,'output')),target=path.join(parent,'comparison.html'),receipt=target+'.compare.json',checkpoint=privateDirectory(path.join(directory,'checkpoint'));
    writeNew(target,OLD_HTML);writeNew(receipt,OLD_RECEIPT);const args=['compare','architecture',base.path,head.path,target,'--json'];
    const killed=await invokeOwned(invoke,engine,args,{json:false,env:{PHOENIX_DIAGRAM_ACCEPTANCE:'1',PHOENIX_DIAGRAM_CHECKPOINT_DIR:checkpoint,PHOENIX_DIAGRAM_CHECKPOINT_PHASE:definition.phase},onSpawn:async child=>{
      scenario.checkpoint=await waitForPairCheckpoint(child,path.join(checkpoint,'checkpoint.json'),{phase:definition.phase,target:definition.phase==='compare-before-second-member'?receipt:target,engine,publicNpm});stopOwned(child,'SIGKILL');
    }},directory,'interrupted-process');
    scenario.process=captureProcess(directory,'interrupted-process',args,killed);assert.equal(killed.signal,'SIGKILL','Comparison was not killed at its observed durable checkpoint');
    // Locate from the journal itself after discarding command stdout and the
    // marker's recoveryDirectory. This is also the lost-stdout recovery path.
    let located=locatePair(parent,target);scenario.located={directory:located.directory,record:located.record,terminalOutputDiscarded:true};
    const before=ordinaryFile(target),beforeReceipt=ordinaryFile(receipt);let preserved;
    if(definition.id==='later-claimant'){fs.unlinkSync(target);preserved=writeNew(target,CLAIMANT);}
    if(definition.id==='backup-tamper'){const file=path.join(located.directory,'before-0');fs.appendFileSync(file,'changed backup');preserved=bound(readSnapshot(file));}
    if(definition.id==='same-content-backup-replacement'){
      const file=path.join(located.directory,'before-0'),temporary=path.join(directory,'new-backup');const original=readSnapshot(file);writeNew(temporary,original.content);fs.renameSync(temporary,file);preserved=bound(readSnapshot(file));assert(same(preserved.identity,original.identity)&&preserved.binding.inode!==original.binding.inode,'Same-content replacement must isolate a changed inode');
    }
    if(definition.id==='parent-replacement'){
      const holding=path.join(directory,'original-parent');fs.renameSync(parent,holding);privateDirectory(parent);preserved=writeNew(target,CLAIMANT);located={...located,directory:path.join(holding,path.basename(located.directory))};scenario.located.movedDirectory=located.directory;scenario.located.record=bound(readSnapshot(path.join(located.directory,path.basename(located.record.path))));
    }
    const recovered=await invoke(engine,['recover-compare',located.directory,'--json']);scenario.recovery=captureProcess(directory,'recovery',['recover-compare',located.directory,'--json'],recovered);
    if(preserved){assert.equal(recovered.exitCode,3,'Changed bindings must require recovery instead of overwriting');assert.equal(recovered.receipt?.status,'failed');assert.equal(recovered.receipt?.recoveryStatus,'recovery-required');readBound(preserved,'preserved later claimant or backup');scenario.preserved=preserved;}
    else {
      assert.equal(recovered.exitCode,0,JSON.stringify(recovered.receipt));
      assert.equal(recovered.receipt?.status,'pass');
      if(definition.phase==='compare-committed'){assert.equal(recovered.receipt?.recoveryStatus,'committed');readBound(before,'committed comparison');readBound(beforeReceipt,'committed receipt');}
      else {assert.equal(recovered.receipt?.recoveryStatus,'recovered');assert(same(readSnapshot(target).identity,identity(OLD_HTML)));assert(same(readSnapshot(receipt).identity,identity(OLD_RECEIPT)));}
      const again=await invoke(engine,['recover-compare',located.directory,'--json']);scenario.repeat=captureProcess(directory,'repeat',['recover-compare',located.directory,'--json'],again);assert.equal(again.exitCode,0,'Explicit pair recovery was not idempotent');
    }
    scenario.status='pass';scenario.record=writeNew(path.join(directory,'result.json'),scenario);
  }
  const report={schemaVersion:1,kind:'p3-real-comparison-faults',status:'pass',engine,base,head,scenarios};return{status:'pass',record:writeNew(path.join(root,'index.json'),report),scope:'Real owned SIGKILL at three durable pair windows; lost stdout locator, claimant/backup/parent/inode protections and repeated recovery'};
}

async function portClosed(port){return await new Promise(resolve=>{const socket=net.connect({host:'127.0.0.1',port});socket.once('connect',()=>{socket.destroy();resolve(false);});socket.once('error',error=>{socket.destroy();resolve(error.code==='ECONNREFUSED');});socket.setTimeout(500,()=>{socket.destroy();resolve(false);});});}
async function request(url,{method='GET',headers={}}={}){return await fetch(url,{method,headers,signal:AbortSignal.timeout(3000),redirect:'error',cache:'no-store'});}
export async function runPreviewAcceptance({engine,runRoot,document,invoke,publicNpm}){
  const root=privateDirectory(path.join(runRoot,'preview-acceptance')),cases=[];
  for(const mode of ['graceful','sigkill','output-claimant']){
    const directory=privateDirectory(path.join(root,mode)),candidate=path.join(directory,'candidate.json'),output=path.join(directory,'diagram.html');writeNew(candidate,document);
    const events=[];let started,checkpoint,termination;
    const args=['preview','architecture',candidate,output,'--port','0','--json'];
    const result=await invokeOwned(invoke,engine,args,{json:false,timeoutMs:90000,onSpawn:async child=>{
      let pending='';child.stdout.on('data',chunk=>{try{pending+=chunk.toString('utf8');while(pending.includes('\n')){const at=pending.indexOf('\n'),line=pending.slice(0,at);pending=pending.slice(at+1);if(line.trim())events.push(JSON.parse(line));}}catch{stopOwned(child,'SIGKILL');}});
      started=await waitFor(()=>events.find(event=>event.event==='started'),'preview started',child);
      assert(typeof started.url==='string'&&Number.isInteger(started.port)&&started.port>0,'Started event lacks its actual URL/port');
      const url=new URL(started.url);assert.equal(url.hostname,'127.0.0.1');assert.equal(url.port,String(started.port));assert(/^\/[a-f0-9]{32}\/$/.test(url.pathname),'Preview URL has no owned session path');
      const statusUrl=new URL('status',url),artifactUrl=new URL('artifact',url);
      const current=async()=>{const response=await request(statusUrl);assert.equal(response.status,200);return response.json();};
      const initial=await waitFor(async()=>{const status=await current();return status.status==='ready'&&status.artifact?status:null;},'preview initial publication',child);
      const firstArtifact=await request(artifactUrl);assert.equal(firstArtifact.status,200);assert(same(identity(Buffer.from(await firstArtifact.arrayBuffer())),readSnapshot(output).identity),'HTTP artifact differs from the formal file');
      const formal=readSnapshot(output).content.toString('utf8');for(const forbidden of [started.url,url.pathname.slice(1,-1),candidate])assert(!formal.includes(forbidden),'Formal HTML leaked preview session or source information');
      const statusResponse=await request(statusUrl);assert.equal(statusResponse.headers.get('cache-control'),'no-store');assert.equal(statusResponse.headers.get('referrer-policy'),'no-referrer');
      if(mode==='sigkill'){
        const changed=structuredClone(document);changed.components[1].label='Interrupted service';fs.writeFileSync(candidate,JSON.stringify(changed));
        const checking=await waitFor(()=>events.find(event=>event.event==='checking'&&event.generation>initial.generation),'preview launched replacement worker',child);
        const ownership=publicNpm?await ownedController(child,engine,publicNpm):null;stopOwned(child,'SIGKILL');checkpoint={initial,checking,terminalOutputDiscarded:false,...(ownership?{ownership}:{})};return;
      }
      const changed=structuredClone(document);changed.components[1].label='Updated service';
      if(mode==='output-claimant'){
        const temporary=writeNew(path.join(directory,'later-claimant'),CLAIMANT);fs.renameSync(temporary.path,output);const claimant=bound(readSnapshot(output));
        fs.writeFileSync(candidate,JSON.stringify(changed));
        const failed=await waitFor(async()=>{const status=await current();return status.status==='error'&&status.artifactAvailable===false?status:null;},'preview output claimant refusal',child);
        const hidden=await request(artifactUrl);assert.equal(hidden.status,409,'Preview served a claimant as its last-good artifact');readBound(claimant,'preserved preview output claimant');checkpoint={initial,failed,claimant,artifactHttpStatus:hidden.status};termination=await terminatePreview(child,engine,publicNpm);return;
      }
      fs.writeFileSync(candidate,JSON.stringify(changed));
      const updated=await waitFor(async()=>{const status=await current();return status.status==='ready'&&status.artifact?.generation>initial.artifact.generation?status:null;},'preview valid update',child);
      const last=bound(readSnapshot(output));fs.writeFileSync(candidate,'{"schema_version":');
      const failed=await waitFor(async()=>{const status=await current();return status.status==='error'?status:null;},'preview invalid input',child);readBound(last,'preview last-good file');
      const retained=await request(artifactUrl);assert(same(identity(Buffer.from(await retained.arrayBuffer())),last.identity),'Invalid input replaced the last-good HTTP artifact');assert.equal(failed.artifact.generation,updated.artifact.generation);
      fs.unlinkSync(candidate);const deleted=await waitFor(async()=>{const status=await current();return status.status==='error'&&status.generation>failed.generation?status:null;},'preview deleted input',child);readBound(last,'deleted-input last-good file');
      const rejectedOrigin=await request(statusUrl,{headers:{Origin:'https://outside.example'}});assert(rejectedOrigin.status>=400,'Preview accepted a cross-origin request');
      const rejectedHost=await new Promise((resolve,reject)=>{
        const probe=http.get(statusUrl,{headers:{Host:'outside.example'}},response=>{response.resume();resolve(response.statusCode);});
        probe.on('error',reject);probe.setTimeout(3000,()=>probe.destroy(new Error('Owned Host probe timed out')));
      });assert(rejectedHost>=400,'Preview accepted an unrecognized Host');
      const unknown=await request(new URL('unknown',url));assert.equal(unknown.status,404,'Preview exposed an unrecognized route');
      fs.writeFileSync(candidate,JSON.stringify(changed));await waitFor(async()=>{const status=await current();return status.status==='ready'?status:null;},'preview recovery',child);
      checkpoint={initial,updated,failed,deleted,lastGood:last,rejectedOrigin:rejectedOrigin.status,rejectedHost,unknownRoute:unknown.status};termination=await terminatePreview(child,engine,publicNpm);
    }},directory,'process');
    const processRecord=captureProcess(directory,'process',args,result);assert(started&&await waitFor(()=>portClosed(started.port),'preview released port',null,5000),'Preview left its listener open');
    if(mode!=='sigkill'){assert.equal(result.exitCode,mode==='output-claimant'?3:0);assert.equal(result.signal,null);assert(events.some(event=>event.event==='stopped'&&event.shutdownComplete===true),'Graceful stop did not confirm complete shutdown');}
    else assert.equal(result.signal,'SIGKILL');
    const entry={mode,status:'pass',events,checkpoint,...(termination?{termination}:{}),portClosed:true,process:processRecord,artifact:bound(readSnapshot(output))};cases.push(entry);writeNew(path.join(directory,'result.json'),entry);
  }
  const workerStops=await runPreviewWorkerStops({engine,runRoot:root,document,invoke,publicNpm});
  return{status:'pass',record:writeNew(path.join(root,'index.json'),{schemaVersion:1,kind:'p3-real-preview-processes',status:'pass',engine,cases,workerStops}),scope:'Real public preview, loopback HTTP/formal bytes, valid/invalid/deleted/recovery generations, Host/Origin/routes, output claimant protection, graceful stop, owned SIGKILL, and graceful/forced worker cancellation with closed ports'};
}

export async function runPreviewWorkerStops({engine,runRoot,document,invoke,publicNpm}) {
  const root=privateDirectory(path.join(runRoot,'preview-worker-stops')),cases=[];
  for(const forced of [false,true]) {
    const directory=privateDirectory(path.join(root,forced?'forced':'graceful'));
    const input=path.join(directory,'input.json'),output=path.join(directory,'diagram.html');
    const source=writeNew(input,document),previous=writeNew(output,OLD_HTML);
    const checkpoint=privateDirectory(path.join(directory,'checkpoint'));
    const phase=forced?'preview-worker-ignore-term':'preview-worker-ready';
    const args=['preview','architecture',input,output,'--port','0','--json'];
    let marker,stoppedAt;
    const result=await invokeOwned(invoke,engine,args,{json:false,timeoutMs:90000,
      env:{PHOENIX_DIAGRAM_ACCEPTANCE:'1',PHOENIX_DIAGRAM_CHECKPOINT_DIR:checkpoint,PHOENIX_DIAGRAM_CHECKPOINT_PHASE:phase},
      onSpawn:async child=>{
        marker=await waitFor(()=>{
          let file;try{file=ordinaryFile(path.join(checkpoint,'checkpoint.json'));}catch(error){if(error.code==='core/file-binding')return null;throw error;}if(!file)return null;
          let value;try{value=jsonBound(file,'owned worker checkpoint');}catch(error){if(error instanceof SyntaxError||['core/file-binding','core/stale-evidence'].includes(error.code))return null;throw error;}
          assert(value.schemaVersion===1&&value.kind==='preview-worker-checkpoint'&&value.phase===phase&&value.generation===1&&value.inputSha256===source.identity.sha256&&value.ignoreSigterm===forced,'Worker checkpoint belongs to another snapshot or stop mode');
          assert(Number.isInteger(value.pid)&&value.pid>0&&value.pid!==child.pid,'Worker checkpoint lacks a distinct process');
          const parent=Number(execFileSync('/bin/ps',['-o','ppid=','-p',String(value.pid)],{encoding:'utf8'}).trim());
          const ownership=publicNpm?proveControllerOwnership(child,engine,processTable(),{controllerPid:parent,workerPid:value.pid}):null;
          if(!publicNpm)assert.equal(parent,child.pid,'Only this controller\'s child can be stopped');
          return{file,...value,...(ownership?{ownership}:{})};
        },'owned worker ready',child,60000);
        stoppedAt=Date.now();if(publicNpm){const proof=await ownedController(child,engine,publicNpm,{controllerPid:marker.ownership.controllerPid,workerPid:marker.pid});process.kill(proof.controllerPid,'SIGTERM');}else child.kill('SIGTERM');
      }},directory,'process');
    const stopElapsedMs=Date.now()-stoppedAt;
    const events=result.stdout.trim().split('\n').filter(Boolean).map(line=>JSON.parse(line));
    const processRecord=captureProcess(directory,'process',args,result);
    writeNew(path.join(directory,'observation.json'),{forced,marker,events,stopElapsedMs,process:processRecord});
    const started=events.find(event=>event.event==='started');
    assert(started&&await waitFor(()=>portClosed(started.port),'stopped worker port',null,5000),'Worker stop left the listener open');
    assert.equal(result.exitCode,0);assert.equal(result.signal,null);
    assert(events.some(event=>event.event==='stopped'&&event.shutdownComplete===true&&event.lastAttemptStatus==='cancelled'),'Worker cancellation lacks confirmed shutdown');
    assert(!events.some(event=>event.event==='updated'),'A cancelled snapshot must not be published');
    readBound(previous,'preexisting file after worker cancellation');
    if(forced)assert(stopElapsedMs>=2900,'Stopped worker did not exercise the three-second forced-stop path');
    const entry={forced,status:'pass',marker,events,stopElapsedMs,portClosed:true,previous,process:processRecord};
    cases.push(entry);writeNew(path.join(directory,'result.json'),entry);
  }
  return{status:'pass',record:writeNew(path.join(root,'index.json'),{schemaVersion:1,kind:'p3-real-preview-worker-stops',status:'pass',engine,cases})};
}
