'use strict';
// Real Node installer -> both native engines. Other-target host-byte copies
// below exercise offline inventory only; they are not platform/run evidence.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');
const { spawn, spawnSync } = require('node:child_process');
const { pathToFileURL } = require('node:url');
const root=path.resolve(__dirname,'../..');
const targets=['darwin-arm64','darwin-x64','linux-x64','linux-arm64'];
const target=`${process.platform}-${process.arch}`;
const skip=!process.env.PHOENIX_UI_BIN||!process.env.PHOENIX_DIAGRAM_BIN?'Requires explicit PHOENIX_UI_BIN and PHOENIX_DIAGRAM_BIN; fresh npm test does not build or download':!targets.includes(target)?'Joint native test supports the four Unix targets':false;
const nativeTest=(name,fn)=>test(name,{skip},fn);
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const encoded=value=>JSON.stringify(value,null,2)+'\n';
function write(file,bytes,mode=0o644){fs.mkdirSync(path.dirname(file),{recursive:true});fs.writeFileSync(file,bytes,{mode});fs.chmodSync(file,mode);}
function copy(from,to){fs.cpSync(from,to,{recursive:true,mode:fs.constants.COPYFILE_FICLONE});}
let engines;
test.after(()=>{if(engines?.warmUiCache)fs.rmSync(engines.warmUiCache,{recursive:true,force:true});});
function realEngines(){
  if(engines)return engines;
  const ui=process.env.PHOENIX_UI_BIN,diagram=process.env.PHOENIX_DIAGRAM_BIN;
  for(const [file,expected] of [[ui,'phoenix-ui-engine 0.1.0\n'],[diagram,'phoenix-diagram-engine 0.2.0 input=1 receipt=1 viewer=phoenix-v2\n']]){
    const probe=spawnSync(file,['engine-probe'],{encoding:'utf8',timeout:3000,maxBuffer:4096});
    assert.equal(probe.status,0,probe.error?.message||probe.stderr);assert.equal(probe.stdout,expected);
  }
  const doctor=spawnSync(diagram,['doctor','--json'],{encoding:'utf8'});
  const identity=JSON.parse(doctor.stdout).engine;
  const uiBytes=fs.readFileSync(ui),diagramBytes=fs.readFileSync(diagram);
  assert.equal(sha(diagramBytes),identity.binary.sha256,'Provide immutable native snapshots');
  return engines={ui,diagram,uiBytes,diagramBytes,identity};
}
async function fixture(t,{warmUi=true}={}){
  const temp=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'phoenix joint install ')));
  const cleanups=new Set();
  t.after(async()=>{for(const cleanup of cleanups)await cleanup();fs.rmSync(temp,{recursive:true,force:true});});
  const pkg=path.join(temp,'package'),home=path.join(temp,'home'),diagramCache=path.join(temp,'diagram-cache');
  let uiCache=path.join(temp,'ui-cache');
  fs.mkdirSync(home,{recursive:true});
  copy(path.join(root,'package.json'),path.join(pkg,'package.json'));
  for(const name of ['xgent-skills.js','phoenix-ui.js','phoenix-diagram.js'])copy(path.join(root,'bin',name),path.join(pkg,'bin',name));
  copy(path.join(root,'.claude/hooks'),path.join(pkg,'.claude/hooks'));
  for(const name of ['apply-ui-review','design-md','design-reference','review-ui','ui-pattern-research'])copy(path.join(root,'skills',name),path.join(pkg,'skills',name));
  copy(path.join(root,'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs'),path.join(pkg,'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs'));
  const engine=realEngines();
  const uiDist=path.join(pkg,'dist/phoenix-ui'),uiRelease=path.join(temp,'ui-release');
  copy(path.join(root,'dist/phoenix-ui'),uiDist);
  const uiVersion=JSON.parse(fs.readFileSync(path.join(uiDist,'VERSION.json')));
  uiVersion.distribution='development';
  uiVersion.engines=Object.fromEntries(targets.map(key=>[key,{asset:`phoenix-ui-${key}`,size:engine.uiBytes.length,sha256:sha(engine.uiBytes)}]));
  write(path.join(uiDist,'VERSION.json'),encoded(uiVersion));
  copy(uiDist,uiRelease);
  for(const meta of Object.values(uiVersion.engines))fs.copyFileSync(engine.ui,path.join(uiRelease,meta.asset),fs.constants.COPYFILE_FICLONE);
  const build=await import(pathToFileURL(path.join(root,'scripts/build-phoenix-diagram.mjs')));
  const bundle=build.bundleFiles(root),diagramDist=path.join(pkg,'dist/phoenix-diagram'),diagramRelease=path.join(temp,'diagram-release');
  for(const [name,bytes] of bundle.files){
    const mode=bundle.manifest.files.find(entry=>entry.path===name).mode;
    write(path.join(diagramDist,'bundle',name),bytes,mode);write(path.join(pkg,'skills/phoenix-diagram',name),bytes,mode);
  }
  write(path.join(diagramDist,'bundle/manifest.json'),bundle.bytes);
  const npmPackageVersion=JSON.parse(fs.readFileSync(path.join(pkg,'package.json'))).version;
  const diagramVersion={schemaVersion:1,distribution:'development',tool:'phoenix-diagram',toolVersion:'0.2.0',npmPackageVersion,sourceCommit:'a'.repeat(40),sourceInputSha256:engine.identity.sourceInputSha256,inputContractSchema:1,receiptSchema:1,viewerRevision:'phoenix-v2',bundleSha256:bundle.sha256,engines:Object.fromEntries(targets.map(key=>[key,{asset:`phoenix-diagram-${key}`,size:engine.diagramBytes.length,sha256:sha(engine.diagramBytes)}]))};
  for(const file of [path.join(diagramDist,'VERSION.json'),path.join(diagramDist,'bundle/ENGINE.json'),path.join(pkg,'skills/phoenix-diagram/ENGINE.json')])write(file,encoded(diagramVersion));
  // Declared legal inventory is complete for this format fixture. M6 separately
  // verifies the actual complete Rust/font/upstream release license closure.
  const apache=fs.readFileSync(path.join(root,'tools/phoenix-ui/LICENSE'));
  write(path.join(diagramDist,'LICENSE'),fs.readFileSync(path.join(root,'LICENSE')));
  write(path.join(diagramDist,'NOTICE.md'),fs.readFileSync(path.join(root,'tools/phoenix-diagram/NOTICE.md')));
  write(path.join(diagramDist,'THIRD-PARTY-LICENSES/apache-2.0.txt'),apache);
  write(path.join(diagramDist,'THIRD-PARTY.json'),encoded({schemaVersion:1,resources:[{name:'joint-installer-format-fixture',licenses:[{path:'THIRD-PARTY-LICENSES/apache-2.0.txt',sha256:sha(apache)}]}]}));
  write(path.join(diagramDist,'THIRD-PARTY-NOTICES.md'),'Full local license bytes for bootstrap format testing; not final release legal acceptance.\n');
  copy(diagramDist,diagramRelease);
  for(const meta of Object.values(diagramVersion.engines))fs.copyFileSync(engine.diagram,path.join(diagramRelease,meta.asset),fs.constants.COPYFILE_FICLONE);
  const env={...process.env,HOME:home,USERPROFILE:home,XDG_CONFIG_HOME:path.join(home,'config'),XDG_CACHE_HOME:path.join(home,'cache'),XDG_DATA_HOME:path.join(home,'data'),PHOENIX_UI_HOME:uiCache,PHOENIX_DIAGRAM_HOME:diagramCache,HTTP_PROXY:'',HTTPS_PROXY:'',ALL_PROXY:'',NO_PROXY:'',http_proxy:'',https_proxy:'',all_proxy:'',no_proxy:''};
  if(warmUi){
    // This lane tests joint transactions with a ready real UI cache. Actual
    // cold acquisition remains in the UI suite and final-package acceptance.
    const ui=require(path.join(pkg,'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs'));
    const manifest=ui.manifestAt(path.join(uiDist,'VERSION.json'));
    if(!engine.warmUiCache){
      engine.warmUiCache=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'phoenix joint verified UI cache ')));
      const owned=engine.warmUiCache;
      const destination=ui.enginePath(manifest,target,{...env,PHOENIX_UI_HOME:owned});
      fs.mkdirSync(path.dirname(destination),{recursive:true,mode:0o700});fs.copyFileSync(engine.ui,destination,fs.constants.COPYFILE_FICLONE);fs.chmodSync(destination,0o700);
      assert.equal(sha(fs.readFileSync(destination)),manifest.data.engines[target].sha256);
      // Explicit fixture preparation loads the copied Mach-O before the joint
      // transaction lane. The production probe below still has its 3s/4KiB cap.
      const loaded=spawnSync(destination,['--version'],{env:{...env,PHOENIX_UI_HOME:owned},encoding:'utf8',timeout:30000,maxBuffer:4096});
      assert.equal(loaded.status,0,loaded.error?.message||loaded.stderr);assert.equal(loaded.stdout,'0.1.0\n');
    }
    uiCache=engine.warmUiCache;env.PHOENIX_UI_HOME=uiCache;
    ui.verifyEngine(ui.enginePath(manifest,target,env),manifest,target,env);
  }
  const project=name=>{const dir=path.join(temp,name);fs.mkdirSync(path.join(dir,'.git'),{recursive:true});write(path.join(dir,'business.txt'),'preserve business bytes\n');return dir;};
  return {temp,pkg,env,uiCache,diagramCache,uiRelease,diagramRelease,uiDist,diagramDist,project,cleanups};
}
function tree(directory,includeDirectories=false){
  const result={};
  function visit(prefix){for(const name of fs.readdirSync(path.join(directory,prefix))){
    const relative=prefix?`${prefix}/${name}`:name,file=path.join(directory,relative),stat=fs.lstatSync(file);
    if(stat.isDirectory()&&!stat.isSymbolicLink()){if(includeDirectories)result[relative]={directory:true,mode:stat.mode&0o777};visit(relative);}
    else result[relative]=stat.isSymbolicLink()?{link:fs.readlinkSync(file)}:{sha256:sha(fs.readFileSync(file)),mode:stat.mode&0o777};
  }}
  visit('');return result;
}
function physical(file){const stat=fs.statSync(file,{bigint:true});return {device:stat.dev.toString(),inode:stat.ino.toString()};}
async function install(f,project,flags=[],onUiActive){
  const args=[path.join(f.pkg,'bin/xgent-skills.js'),'install',project,'--no-xgent-init',...flags];
  const watchers=new Map();let child,running=false,barrier=false,failure,stdout='',stderr='';
  const kill=()=>{if(running&&child?.pid){try{process.kill(-child.pid,'SIGKILL');}catch(error){if(error.code!=='ESRCH')throw error;}}};
  const fail=error=>{failure||=error;kill();};
  function inspect(){
    if(barrier||failure)return;
    const directories=process.platform==='darwin'?[project]:[project,path.join(project,'.phoenix-ui'),path.join(project,'.phoenix-ui/install')];
    for(const directory of directories){
      if(watchers.has(directory)||!fs.existsSync(directory))continue;
      try{const watcher=fs.watch(directory,{recursive:process.platform==='darwin'},inspect);watcher.on('error',fail);watchers.set(directory,watcher);}catch(error){if(error.code!=='ENOENT')fail(error);}
    }
    const active=path.join(project,'.phoenix-ui/install/active.json');
    if(!fs.existsSync(active))return;
    try{const bytes=fs.readFileSync(active),receipt=JSON.parse(bytes);assert.equal(receipt.schema_version,1);assert.ok(receipt.transaction_id);barrier=true;onUiActive({path:active,bytes,identity:physical(active)});}catch(error){fail(error);}
  }
  if(onUiActive)inspect();
  try{child=spawn(process.execPath,args,{cwd:project,env:f.env,stdio:['ignore','pipe','pipe'],detached:true});running=true;}catch(error){for(const watcher of watchers.values())watcher.close();throw error;}
  const done=new Promise(resolve=>{
    child.stdout.on('data',bytes=>{stdout+=bytes;if(stdout.length>16*1024*1024)fail(new Error('Installer stdout exceeded bound'));if(onUiActive)inspect();});
    child.stderr.on('data',bytes=>{stderr+=bytes;if(stderr.length>16*1024*1024)fail(new Error('Installer stderr exceeded bound'));});
    child.once('error',error=>{failure||=error;});
    child.once('close',(code,signal)=>{running=false;resolve({code,signal,stdout,stderr});});
  });
  const timeout=setTimeout(()=>fail(new Error('Real installer exceeded its ten-minute test deadline')),600000);
  const exitKill=()=>kill();process.once('exit',exitKill);
  const cleanup=async()=>{clearTimeout(timeout);for(const watcher of watchers.values())watcher.close();kill();await done;process.removeListener('exit',exitKill);};
  f.cleanups.add(cleanup);
  try{const result=await done;if(failure)throw failure;if(onUiActive)assert(barrier,`The real UI active.json barrier must be observed (exit ${result.code}, signal ${result.signal})\n${stdout.slice(-4000)}\n${stderr.slice(-2000)}`);return result;}finally{await cleanup();f.cleanups.delete(cleanup);}
}
const both=f=>[`--release-dir=${f.uiRelease}`,`--diagram-release-dir=${f.diagramRelease}`];
function assertBoth(project,provider){for(const name of ['phoenix-ui','phoenix-diagram'])assert(fs.existsSync(path.join(project,provider,'skills',name,'SKILL.md')),`${provider}/${name}`);}

nativeTest('default installer really coinstalls both tools, is idempotent, and skip flags preserve both installations',async t=>{
  const f=await fixture(t),project=f.project('default project');
  const first=await install(f,project,both(f));assert.equal(first.code,0,first.stdout+first.stderr);
  assertBoth(project,'.claude');assert.match(first.stdout,/Phoenix UI.*已安装/);assert.match(first.stdout,/Phoenix Diagram.*已安装/);
  const before=tree(project),activeUi=path.join(project,'.phoenix-ui/install/active.json'),activeDiagram=path.join(project,'.phoenix-diagram/install/active.json');
  const identities=[physical(activeUi),physical(activeDiagram)];
  const second=await install(f,project,both(f));assert.equal(second.code,0,second.stdout+second.stderr);
  assert.match(second.stdout,/Phoenix UI.*未变/);assert.match(second.stdout,/Phoenix Diagram.*未变/);
  assert.deepEqual(tree(project),before);assert.deepEqual([physical(activeUi),physical(activeDiagram)],identities);
  const skipped=await install(f,project,['--no-phoenix-ui','--no-phoenix-diagram','--release-dir=/missing-ui','--diagram-release-dir=/missing-diagram']);
  assert.equal(skipped.code,0,skipped.stdout+skipped.stderr);assert.deepEqual(tree(project),before);
});
nativeTest('selected Codex providers isolate both skill roots from unselected harnesses',async t=>{
  const f=await fixture(t),project=f.project('selected project');
  write(path.join(project,'.cursor/settings.user.json'),'unselected provider bytes\n');
  const result=await install(f,project,[...both(f),'--providers=codex']);assert.equal(result.code,0,result.stdout+result.stderr);
  assertBoth(project,'.agents');
  for(const provider of ['.claude','.cursor','.codex'])for(const name of ['phoenix-ui','phoenix-diagram'])assert.equal(fs.existsSync(path.join(project,provider,'skills',name)),false,`${provider}/${name} must remain absent`);
  assert.equal(fs.readFileSync(path.join(project,'.cursor/settings.user.json'),'utf8'),'unselected provider bytes\n');
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(project,'.phoenix-diagram/install/active.json'))).providers,['.agents','.codex']);
});
nativeTest('a ready real UI engine plus missing diagram resources causes zero XGENT or project writes',async t=>{
  const f=await fixture(t),project=f.project('preflight failure project');
  const ui=require(path.join(f.pkg,'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs'));
  await ui.installEngine(ui.manifestAt(path.join(f.uiDist,'VERSION.json')),target,project,f.uiRelease,f.env);
  const cacheBefore=tree(f.uiCache,true),before=tree(project,true);
  fs.unlinkSync(path.join(f.diagramDist,'VERSION.json'));
  const result=await install(f,project,both(f));
  assert.deepEqual(tree(project,true),before);assert.deepEqual(tree(f.uiCache,true),cacheBefore);
  assert.equal(result.code,4,result.stdout+result.stderr);assert.match(result.stderr,/Phoenix Diagram.*未修改项目/s);
});
nativeTest('UI and diagram release directories are never borrowed for the other tool',async t=>{
  const f=await fixture(t,{warmUi:false});
  const scenarios=[
    {flags:[`--release-dir=${f.diagramRelease}`,`--diagram-release-dir=${f.diagramRelease}`],tool:/Phoenix UI engine.*未就绪/},
    {flags:[`--release-dir=${f.uiRelease}`,`--diagram-release-dir=${f.uiRelease}`],tool:/Phoenix Diagram.*未就绪/},
    {flags:[`--release-dir=${f.uiRelease}`],tool:/Phoenix Diagram.*未就绪/},
    {flags:[`--diagram-release-dir=${f.diagramRelease}`],tool:/Phoenix UI engine.*未就绪/},
  ];
  for(const [index,scenario] of scenarios.entries()){
    const project=f.project(`separate release ${index}`),before=tree(project,true);
    const local={...f,env:{...f.env,PHOENIX_UI_HOME:path.join(f.temp,`ui-cache-${index}`),PHOENIX_DIAGRAM_HOME:path.join(f.temp,`diagram-cache-${index}`)}};
    const result=await install(local,project,scenario.flags);assert.equal(result.code,4,result.stdout+result.stderr);assert.match(result.stderr,scenario.tool);assert.doesNotMatch(result.stderr,/ETIMEDOUT|Probe:/);assert.deepEqual(tree(project,true),before);
  }
});
nativeTest('a claimant created after the real UI active barrier survives diagram apply failure and the completed prefix remains',async t=>{
  const f=await fixture(t),project=f.project('later claimant project');let barrier,claim;
  const claimant=path.join(project,'.claude/skills/phoenix-diagram/SKILL.md'),content='Later user claimant: preserve exact bytes.\n';
  const result=await install(f,project,both(f),observed=>{
    assert.equal(fs.existsSync(path.join(project,'.phoenix-diagram/install/active.json')),false);
    assert.equal(fs.existsSync(claimant),false);
    fs.mkdirSync(path.dirname(claimant),{recursive:true});fs.writeFileSync(claimant,content,{flag:'wx',mode:0o644});
    barrier=observed;claim=physical(claimant);
  });
  assert.equal(result.code,3,result.stdout+result.stderr);assert.match(result.stdout,/XGENT 已安装,Phoenix UI 已完成/);assert.match(result.stdout,/保留前段结果/);
  assert(fs.existsSync(path.join(project,'.claude/hooks/xgent-statusline.js')));assert(fs.existsSync(path.join(project,'.claude/skills/phoenix-ui/SKILL.md')));
  assert.deepEqual(fs.readFileSync(barrier.path),barrier.bytes);assert.deepEqual(physical(barrier.path),barrier.identity);
  assert.equal(fs.readFileSync(claimant,'utf8'),content);assert.deepEqual(physical(claimant),claim);
  assert.equal(fs.existsSync(path.join(project,'.phoenix-diagram/install/active.json')),false);
});
