#!/usr/bin/env node
// Ignored maintenance draft. Never an acceptance stage; execute only after root review.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawn,spawnSync } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual as same } from 'node:util';

const AUTH={
  repository:'XGENT-ai/skills',runId:38907214275,
  sourceCommit:'55ba9908221d72863d1ff82b36cca7b406515dd1',
  sourceInputSha256:'6ca1da238af5e88722bd36bca6bef0111f10b54c6d1e7e86ec2b1b369b2529c4',
  candidateArtifactId:12050117772,harnessArtifactId:12050207785,
  candidateArchive:{sha256:'c40572c5db86a4813e1f69910d78c40896f624363b445cfdf1a3a35da35fe9e2',bytes:65187840},
  harnessArchive:{sha256:'c00ecaa80494b541bc09f20077b376260efcf9a5bea4441be507c0d835d0a4f7',bytes:450560},
  tarball:{sha256:'5a63722aaa680993d86007ae629b89b3aecc67d8ff8a35e5429d985039003035',bytes:7144832},
  binary:{sha256:'b2f4077e97e1c50feb07f4102fdc37db167d33b1f6f56fc854de8cd4c5dac86a',bytes:13917240},
  candidate:{sha256:'af2adbcf0f7a8ed531236a10489938063dfd33d497085219481a83fe8a7113d7',bytes:552},
};
const HELP='Usage: node diagnose-diagram-checkpoint.mjs --run-diagnostic --candidate-dir DIR --harness-dir DIR --candidate-archive TAR --harness-archive TAR --output-parent OWNED_0700_DIR\nOne darwin-x64 after-retire observation with 60000ms observer budget; no acceptance verdict or threshold change.\n';
const OLD=Buffer.from('Original output retained by the public CLI recovery acceptance.\n');
const digest=bytes=>({sha256:createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length});
const need=(value,message)=>{if(!value)throw new Error(message);};
const exists=file=>{try{fs.lstatSync(file);return true;}catch(error){if(error.code==='ENOENT')return false;throw error;}};
const errorValue=error=>({name:error.name,code:error.code || null,message:error.message,stack:error.stack});
function writeNew(file,value,mode=0o600){fs.writeFileSync(file,Buffer.isBuffer(value)?value:Buffer.from(JSON.stringify(value,null,2)+'\n'),{flag:'wx',mode});return file;}
function mkdir(file){fs.mkdirSync(file,{mode:0o700});return fs.realpathSync(file);}
function executable(name){for(const dir of (process.env.PATH || '').split(path.delimiter)){try{const file=fs.realpathSync(path.join(dir,name)),stat=fs.statSync(file);if(stat.isFile() && stat.mode&0o111)return file;}catch{}}throw new Error('Missing maintenance program '+name);}
function requireIdentity(file,expected){need(same(digest(fs.readFileSync(file)),expected),'Unexpected original bytes: '+file);}

function environment(runRoot){
  const home=mkdir(path.join(runRoot,'home')),bin=mkdir(path.join(runRoot,'bin'));
  const programs={node:fs.realpathSync(process.execPath),npm:executable('npm'),npx:executable('npx')};
  for(const name of ['git','sh','bash','env','dirname','readlink','uname','grep','sed','cat','mkdir','rm','cut','tr'])programs[name]=executable(name);
  for(const [name,file] of Object.entries(programs))fs.symlinkSync(file,path.join(bin,name));
  const user=writeNew(path.join(home,'user.npmrc'),Buffer.alloc(0)),global=writeNew(path.join(home,'global.npmrc'),Buffer.alloc(0));
  const env={HOME:home,USER:os.userInfo().username,LOGNAME:os.userInfo().username,PATH:bin,TMPDIR:mkdir(path.join(runRoot,'tmp')),LANG:'en_US.UTF-8',LC_ALL:'en_US.UTF-8',PHOENIX_DIAGRAM_HOME:mkdir(path.join(runRoot,'engine-cache')),PHOENIX_HOME:mkdir(path.join(runRoot,'ui-cache')),npm_config_cache:mkdir(path.join(runRoot,'npm-cache')),npm_config_userconfig:user,npm_config_globalconfig:global,npm_config_update_notifier:'false',npm_config_audit:'false',npm_config_fund:'false',npm_config_ignore_scripts:'true',npm_config_logs_max:'0',NO_PROXY:'127.0.0.1,localhost'};
  const npmCliPath=fs.realpathSync(path.resolve(path.dirname(programs.node),'../lib/node_modules/npm/bin/npm-cli.js'));
  need(!exists(path.join(bin,'cargo')) && !exists(path.join(bin,'rustc')),'Rust entered the owned child PATH');
  return{env,programs,npmCliPath};
}

// Purposefully do not use invokeNative: it throws an onSpawn failure before
// returning its buffered output. Pipe every chunk straight to exclusive files,
// retaining its original pipe stdio and every byte even when marker waiting fails.
async function capture(logs,label,command,env,{onSpawn,timeoutMs=300000}={}){
  const stdout=path.join(logs,label+'.stdout'),stderr=path.join(logs,label+'.stderr'),events=path.join(logs,label+'.events.ndjson');
  const out=fs.openSync(stdout,'wx',0o600),err=fs.openSync(stderr,'wx',0o600),evt=fs.openSync(events,'wx',0o600);
  const started=performance.now(),startWall=Date.now(),record={command,startedAt:new Date(startWall).toISOString(),startWallMs:startWall,timeoutMs,code:null,signal:null,spawnError:null,hookError:null,timedOut:false};
  const event=(name,details={})=>{const value={name,wallMs:Date.now(),elapsedMs:performance.now()-started,...details};fs.writeSync(evt,JSON.stringify(value)+'\n');return value;};
  let child,timer,interrupt,terminate;
  try{
    event('spawn-call');
    child=spawn(command.program,command.args,{cwd:command.cwd,env,stdio:['ignore','pipe','pipe'],detached:true});record.pid=child.pid || null;
    child.stdout.on('data',bytes=>fs.writeSync(out,bytes));child.stderr.on('data',bytes=>fs.writeSync(err,bytes));
    const killGroup=reason=>{if(child.exitCode!==null || child.signalCode!==null)return;event('owned-pgid-sigkill',{reason,pgid:child.pid});try{process.kill(-child.pid,'SIGKILL');}catch(error){if(error.code!=='ESRCH')throw error;}};
    interrupt=()=>killGroup('controller-SIGINT');terminate=()=>killGroup('controller-SIGTERM');process.once('SIGINT',interrupt);process.once('SIGTERM',terminate);
    child.once('spawn',()=>event('spawned',{pid:child.pid}));child.once('error',error=>{record.spawnError=errorValue(error);event('spawn-error',record.spawnError);});
    timer=setTimeout(()=>{record.timedOut=true;killGroup('diagnostic-command-deadline');},timeoutMs);
    const hook=onSpawn?Promise.resolve().then(()=>onSpawn(child,{event,killGroup})).catch(error=>{record.hookError=errorValue(error);event('marker-hook-error',record.hookError);killGroup('marker-observation-failed');}):Promise.resolve();
    await new Promise(resolve=>child.once('close',(code,signal)=>{record.code=code;record.signal=signal;record.closed=event('close',{code,signal});resolve();}));
    await hook;record.elapsedMs=performance.now()-started;
    for(const fd of [out,err,evt])fs.fsyncSync(fd);
    record.stdout={path:stdout,identity:digest(fs.readFileSync(stdout))};record.stderr={path:stderr,identity:digest(fs.readFileSync(stderr))};record.events={path:events,identity:digest(fs.readFileSync(events))};
    writeNew(path.join(logs,label+'.json'),record);return record;
  }finally{clearTimeout(timer);if(interrupt)process.removeListener('SIGINT',interrupt);if(terminate)process.removeListener('SIGTERM',terminate);for(const fd of [out,err,evt])fs.closeSync(fd);}
}

async function run(options){
  need(process.platform==='darwin' && process.arch==='x64','This diagnostic must run on an actual Intel Mac');
  need(process.version==='v25.2.1','Keep the actual consumer Node version pinned to 25.2.1');
  const parent=fs.realpathSync(options.outputParent),stat=fs.lstatSync(parent);need(stat.isDirectory() && !stat.isSymbolicLink() && stat.uid===process.getuid() && (stat.mode&0o7777)===0o700,'Output parent must be an owned ordinary 0700 directory');
  const runRoot=fs.realpathSync(fs.mkdtempSync(path.join(parent,'checkpoint-')));fs.chmodSync(runRoot,0o700);
  if(process.env.GITHUB_OUTPUT)fs.appendFileSync(process.env.GITHUB_OUTPUT,'evidence_dir='+runRoot+'\n');
  const report={schemaVersion:1,kind:'phoenix-diagram-mac-intel-checkpoint-diagnostic',status:'starting',acceptance:'not-run',scope:'One public npm after-retire timing observation and public recovery; no browser, Reader, visual, provider or publication acceptance',auth:AUTH,runRoot,host:{platform:process.platform,arch:process.arch,node:process.version,release:os.release()},diagnosticWorkflow:{runId:process.env.GITHUB_RUN_ID || null,sourceCommit:process.env.GITHUB_SHA || null},originalAcceptanceBudgetMs:12000,observationBudgetMs:60000,nativeIndependentWaitStatus:'unavailable: the native PID is not the controller direct child; only npm leader close is waitable',diagnostics:[]};
  const logs=mkdir(path.join(runRoot,'logs'));let engine,core;
  try{
    requireIdentity(options.candidateArchive,AUTH.candidateArchive);requireIdentity(options.harnessArchive,AUTH.harnessArchive);
    const imported=await import(pathToFileURL(path.join(options.harnessDir,'scripts/check-phoenix-diagram-candidate.mjs')));
    core=await import(pathToFileURL(path.join(options.harnessDir,'scripts/test-phoenix-diagram-core.mjs')));
    const recovery=await import(pathToFileURL(path.join(options.harnessDir,'scripts/test-phoenix-diagram-recovery.mjs')));
    imported.verifySourceLessHarness(options.harnessDir);
    const candidate=imported.validateCandidate(options.candidateDir,'darwin-x64');
    need(candidate.source.commit===AUTH.sourceCommit && candidate.source.inputSha256===AUTH.sourceInputSha256,'Candidate source differs from authenticated final55');requireIdentity(candidate.tgz.path,AUTH.tarball);need(same(candidate.engine.identity,AUTH.binary),'Candidate Intel engine differs');
    const e=environment(runRoot),project=mkdir(path.join(runRoot,'project'));
    const adapter={schemaVersion:1,kind:'npm-public-bin',nodePath:e.programs.node,npmCliPath:e.npmCliPath,tarball:candidate.tgz.path,cwd:project,env:e.env,bin:'phoenix-diagram'};
    const invoke=(label,args,extra={})=>capture(logs,label,core.publicNpmCommand(adapter,args),{...e.env,...extra.env},{onSpawn:extra.onSpawn,timeoutMs:extra.timeoutMs});
    writeNew(path.join(runRoot,'environment.json'),{...e,diagnosticNotes:'No inherited token, NODE_OPTIONS, Rust tool PATH, user HOME or global npm configuration'});
    const installation=await capture(logs,'install',core.publicNpmCommand({...adapter,bin:'xgent-skills'},['install',project,'--no-phoenix-ui','--no-xgent-init','--diagram-release-dir='+path.join(candidate.directory,'release-dir')]),e.env,{timeoutMs:600000});
    report.installation=installation;need(installation.code===0 && !installation.spawnError,'Public npm installation failed; inspect complete raw output');
    const proof=core.publicNpmCommand({...adapter,bin:'xgent-skills'},[]);proof.args.splice(proof.args.length-1,1,'node','-e',"const{spawnSync}=require('node:child_process');const r=spawnSync('sh',['-c','command -v rustc; command -v cargo; exit 0'],{encoding:'utf8'});if(r.error||r.status!==0||r.stdout.trim())throw new Error('Rust resolved inside npm exec');process.stdout.write(JSON.stringify({rustc:'unresolvable',cargo:'unresolvable',path:process.env.PATH,node:process.version}));");
    report.npmEnvironment=await capture(logs,'npm-environment',proof,e.env);need(report.npmEnvironment.code===0,'Rust-free npm environment proof failed');
    const enginePath=path.join(e.env.PHOENIX_DIAGRAM_HOME,'bin',candidate.version.toolVersion,'darwin-x64',AUTH.binary.sha256,'phoenix-diagram');engine=core.bound(core.readSnapshot(enginePath));need(same(engine.identity,AUTH.binary),'Installed Intel engine bytes differ');adapter.enginePath=enginePath;report.engine=engine;
    report.doctor=await invoke('doctor',['doctor','--json']);need([0,4].includes(report.doctor.code),'Public npm doctor process failed unexpectedly');const doctor=JSON.parse(fs.readFileSync(report.doctor.stdout.path,'utf8'));need(doctor.engine.sourceInputSha256===AUTH.sourceInputSha256 && same(doctor.engine.binary,AUTH.binary),'Public doctor engine identity differs');
    const npx=path.join(e.env.npm_config_cache,'_npx'),packages=fs.readdirSync(npx).map(name=>path.join(npx,name,'node_modules/@xgent-ai/skills')).filter(exists);need(packages.length===1,'Expected exactly one owned npm candidate package');
    const source=path.join(packages[0],'skills/phoenix-diagram/examples/minimal.architecture.json');requireIdentity(source,AUTH.candidate);const input=writeNew(path.join(runRoot,'candidate.json'),fs.readFileSync(source));report.candidate=core.bound(core.readSnapshot(input));
    const output=mkdir(path.join(runRoot,'output')),target=writeNew(path.join(output,'diagram.html'),OLD),checkpoint=mkdir(path.join(runRoot,'checkpoint-after-retire'));let observed;
    report.measurement=await invoke('after-retire',['deliver','architecture',input,target,'--json'],{timeoutMs:75000,env:{PHOENIX_DIAGRAM_ACCEPTANCE:'1',PHOENIX_DIAGRAM_CHECKPOINT_DIR:checkpoint,PHOENIX_DIAGRAM_CHECKPOINT_PHASE:'after-retire'},onSpawn:async(child,{event,killGroup})=>{
      event('marker-wait-start',{observationBudgetMs:60000,acceptanceBudgetMs:12000});let firstMarkerRead;
      const pidMatches=(pid,leader)=>{firstMarkerRead={wallMs:Date.now(),elapsedMs:performance.now()};const ps=spawnSync('/bin/ps',['-o','pgid=','-p',String(pid)],{encoding:'utf8',timeout:3000});event('native-pgid-proof',{nativePid:pid,leaderPid:leader,code:ps.status,signal:ps.signal,stdout:ps.stdout,stderr:ps.stderr,error:ps.error?errorValue(ps.error):null});return !ps.error && ps.status===0 && ps.stdout.trim()===String(leader);};
      const waitStarted={wallMs:Date.now(),elapsedMs:performance.now()};
      observed=await recovery.waitForCheckpoint(child,path.join(checkpoint,'checkpoint.json'),{phase:'after-retire',target,timeoutMs:60000,pidMatches});
      const reached=event('marker-observed',observed),marker=core.readSnapshot(path.join(checkpoint,'checkpoint.json')),mtime=fs.statSync(marker.path).mtimeMs;
      report.marker={...observed,observedAfterSpawnMs:reached.elapsedMs,firstMarkerReadAfterWaitMs:firstMarkerRead.elapsedMs-waitStarted.elapsedMs,validatedAfterWaitMs:reached.wallMs-waitStarted.wallMs,markerMtimeMs:mtime,markerMtimeAfterWaitMs:mtime-waitStarted.wallMs,waitStartedWallMs:waitStarted.wallMs,observedWallMs:reached.wallMs,file:core.bound(marker),copy:writeNew(path.join(runRoot,'observed-checkpoint.json'),marker.content)};
      need(!exists(target),'after-retire marker did not coincide with an absent original public target');killGroup('observed-after-retire');
    }});
    if(observed){const ps=spawnSync('/bin/ps',['-o','pid=,ppid=,pgid=,stat=,comm=','-p',String(observed.pid)],{encoding:'utf8',timeout:3000});report.nativeAfterOwnedGroupKill={pid:observed.pid,psCode:ps.status,psSignal:ps.signal,stdout:ps.stdout,stderr:ps.stderr,error:ps.error?errorValue(ps.error):null,independentWaitStatus:'unavailable'};}
    // Discard the marker recoveryDirectory. Discover only durable records under
    // this newly owned original output parent, as the existing acceptance does.
    const journalNames=fs.readdirSync(output).filter(name=>name.startsWith('.phoenix-diagram-recovery-') || name==='.phoenix-diagram-output-cleanup');
    if(journalNames.length){
      const locator=core.locateRecovery(output,target);writeNew(path.join(runRoot,'located-record.json'),core.readBound(locator.recordFile,'durable diagnostic recovery record').content);
      report.recovery=await invoke('recover',['recover-output',locator.directory,'--json']);
      const result={exitCode:report.recovery.code,signal:report.recovery.signal,receipt:JSON.parse(fs.readFileSync(report.recovery.stdout.path,'utf8'))};
      report.recovery.outcome=recovery.verifyRecoveryOutcome(result,{mode:'recovered',target,expected:digest(OLD)});
    }else{need(exists(target) && same(digest(fs.readFileSync(target)),digest(OLD)),'No recovery record and original public bytes are absent or changed');report.recovery={status:'not-required',reason:'Original bytes were never retired; no durable recovery record'};}
    core.readBound(engine,'unchanged original diagnostic native engine');report.finalPublic=core.bound(core.readSnapshot(target));need(same(report.finalPublic.identity,digest(OLD)),'Original public bytes were not restored');
    need(observed && report.measurement.signal==='SIGKILL' && !report.measurement.hookError,'The marker/owned-PGID interruption was not observed; inspect preserved stdout/stderr/events');
    report.status='observed';report.observation=report.marker.firstMarkerReadAfterWaitMs>12000?'marker-observed-after-12s':'marker-observed-within-12s';
    report.interpretation='One instrument-free public npm observation only. Polling/ps scheduling affects observed elapsed; compare marker mtime and event clocks. This cannot by itself establish a universal slow-host cause or change acceptance.';
  }catch(error){report.status='failed';report.diagnostics.push(errorValue(error));}
  finally{if(engine && core)try{core.readBound(engine,'final unchanged native engine');}catch(error){report.diagnostics.push(errorValue(error));report.status='failed';}writeNew(path.join(runRoot,'diagnostic.json'),report);}
  process.stdout.write(JSON.stringify({status:report.status,acceptance:'not-run',evidenceDir:runRoot,observation:report.observation || 'marker-not-observed'})+'\n');return report.status==='observed'?0:1;
}

if(process.argv.length===3 && process.argv[2]==='--help'){process.stdout.write(HELP);}
else{
  const argv=process.argv.slice(2),options={};need(argv.shift()==='--run-diagnostic','Explicit --run-diagnostic required; use --help');
  const keys={'--candidate-dir':'candidateDir','--harness-dir':'harnessDir','--candidate-archive':'candidateArchive','--harness-archive':'harnessArchive','--output-parent':'outputParent'};
  for(let at=0;at<argv.length;at+=2){const key=keys[argv[at]],value=argv[at+1];need(key && !options[key] && typeof value==='string' && path.isAbsolute(value),'Provide each unique absolute input/output path');options[key]=value;}
  need(Object.keys(options).length===5,'All five diagnostic paths are required');process.exitCode=await run(options);
}
