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
  for(const [file,expected] of [[ui,'phoenix-ui-engine 0.1.0\n'],[diagram,'phoenix-diagram-engine 0.2.0 receipt=1 viewer=phoenix-v2\n']]){
    const probe=spawnSync(file,['engine-probe'],{encoding:'utf8',timeout:3000,maxBuffer:4096});
    assert.equal(probe.status,0,probe.error?.message||probe.stderr);assert.equal(probe.stdout,expected);
  }
  const doctor=spawnSync(diagram,['doctor','--json'],{encoding:'utf8'});
  const identity=JSON.parse(doctor.stdout).engine;
  const uiBytes=fs.readFileSync(ui),diagramBytes=fs.readFileSync(diagram);
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
  copy(path.join(root,'agents'),path.join(pkg,'agents'));
  for(const name of ['apply-ui-review','design-md','design-reference','review-ui','ui-pattern-research'])copy(path.join(root,'skills',name),path.join(pkg,'skills',name));
  copy(path.join(root,'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs'),path.join(pkg,'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs'));
  const engine=realEngines();
  const uiDist=path.join(pkg,'dist/phoenix-ui'),uiRelease=path.join(temp,'ui-release');
  copy(path.join(root,'dist/phoenix-ui'),uiDist);
  const uiVersion={schemaVersion:1,distribution:'development',toolVersion:fs.readFileSync(path.join(root,'skills/phoenix-ui/src/scripts/VERSION'),'utf8').trim(),
    npmPackageVersion:JSON.parse(fs.readFileSync(path.join(root,'package.json'))).version,sourceCommit:'a'.repeat(40),
    bundleSchema:1,bundleSha256:sha(fs.readFileSync(path.join(uiDist,'bundle/manifest.json'))),reviewSchema:1,
    engines:Object.fromEntries(targets.map(key=>[key,{asset:`phoenix-ui-${key}`,size:engine.uiBytes.length,sha256:sha(engine.uiBytes)}]))};
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
  const diagramVersion={schemaVersion:1,distribution:'development',tool:'phoenix-diagram',toolVersion:'0.2.0',npmPackageVersion,sourceCommit:'a'.repeat(40),sourceInputSha256:engine.identity.sourceInputSha256,receiptSchema:1,viewerRevision:'phoenix-v2',bundleSha256:bundle.sha256,engines:Object.fromEntries(targets.map(key=>[key,{asset:`phoenix-diagram-${key}`,size:engine.diagramBytes.length,sha256:sha(engine.diagramBytes)}]))};
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
async function install(f,project,flags=[]){
  const args=[path.join(f.pkg,'bin/xgent-skills.js'),'install',project,...flags];
  let child,running=false,failure,stdout='',stderr='';
  const kill=()=>{if(running&&child?.pid){try{process.kill(-child.pid,'SIGKILL');}catch(error){if(error.code!=='ESRCH')throw error;}}};
  const fail=error=>{failure||=error;kill();};
  child=spawn(process.execPath,args,{cwd:project,env:f.env,stdio:['ignore','pipe','pipe'],detached:true});running=true;
  const done=new Promise(resolve=>{
    child.stdout.on('data',bytes=>{stdout+=bytes;if(stdout.length>16*1024*1024)fail(new Error('Installer stdout exceeded bound'));});
    child.stderr.on('data',bytes=>{stderr+=bytes;if(stderr.length>16*1024*1024)fail(new Error('Installer stderr exceeded bound'));});
    child.once('error',error=>{failure||=error;});
    child.once('close',(code,signal)=>{running=false;resolve({code,signal,stdout,stderr});});
  });
  const timeout=setTimeout(()=>fail(new Error('Real installer exceeded its ten-minute test deadline')),600000);
  const exitKill=()=>kill();process.once('exit',exitKill);
  const cleanup=async()=>{clearTimeout(timeout);kill();await done;process.removeListener('exit',exitKill);};
  f.cleanups.add(cleanup);
  try{const result=await done;if(failure)throw failure;return result;}finally{await cleanup();f.cleanups.delete(cleanup);}
}
const both=f=>[`--release-dir=${f.uiRelease}`,`--diagram-release-dir=${f.diagramRelease}`];
function assertBoth(project,provider){for(const name of ['phoenix-ui','phoenix-diagram'])assert(fs.existsSync(path.join(project,provider,'skills',name,'SKILL.md')),`${provider}/${name}`);}

for(const linked of [false,true])nativeTest(`existing diagram skill ${linked?'symlinks':'copies'} are reinstalled through the npm installer`,async t=>{
  const f=await fixture(t),project=f.project('existing diagram project');
  const source=path.join(f.temp,'old diagram skill');copy(path.join(f.pkg,'skills/phoenix-diagram'),source);
  write(path.join(source,'old-source-marker'),'old channel\n');const sourceBefore=tree(source);
  for(const provider of ['.agents','.claude']){
    const dest=path.join(project,provider,'skills/phoenix-diagram');
    fs.mkdirSync(path.dirname(dest),{recursive:true});
    if(linked)fs.symlinkSync(source,dest);else copy(source,dest);
  }
  const result=await install(f,project,both(f));assert.equal(result.code,0,result.stdout+result.stderr);
  for(const provider of ['.agents','.claude']){
    assertBoth(project,provider);
    const dest=path.join(project,provider,'skills/phoenix-diagram');
    assert(!fs.lstatSync(dest).isSymbolicLink());assert(!fs.existsSync(path.join(dest,'old-source-marker')));
  }
  assert.deepEqual(tree(source),sourceBefore);
  const before=tree(project),again=await install(f,project,both(f));assert.equal(again.code,0,again.stdout+again.stderr);assert.deepEqual(tree(project),before);
});

nativeTest('default installer really coinstalls both tools, is idempotent, and skip flags preserve both installations',async t=>{
  const f=await fixture(t),project=f.project('default project');
  const first=await install(f,project,both(f));assert.equal(first.code,0,first.stdout+first.stderr);
  assertBoth(project,'.claude');assert.match(first.stdout,/Phoenix UI.*已安装/);assert.match(first.stdout,/Phoenix Diagram.*已安装/);
  const before=tree(project);
  const second=await install(f,project,both(f));assert.equal(second.code,0,second.stdout+second.stderr);
  assert.match(second.stdout,/Phoenix UI.*未变/);assert.match(second.stdout,/Phoenix Diagram.*未变/);
  assert.deepEqual(tree(project),before);
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
});
function archifyFixture(f,name){
  const project=f.project(name),home=f.env.HOME,lock={version:1,skills:{
    archify:{source:'XGENT-ai/skills',skillPath:'skills/archify/SKILL.md'},
    'design-md':{source:'XGENT-ai/skills',skillPath:'skills/design-md/SKILL.md'}}};
  const skills=[path.join(project,'.claude/skills/archify'),path.join(home,'.claude/skills/archify'),path.join(home,'.agents/skills/archify')];
  for(const dir of skills)write(path.join(dir,'SKILL.md'),'old archify\n');
  const locks=[path.join(project,'skills-lock.json'),path.join(home,'.agents/.skill-lock.json')];
  for(const file of locks)write(file,encoded(lock));
  // A diagram Archify already produced stays where the user keeps it.
  write(path.join(project,'docs/old.archify.html'),'<html>old output</html>\n');
  return {project,skills,locks};
}
nativeTest('a non-interactive install removes project and personal Archify skills and their lock entries',async t=>{
  const f=await fixture(t),a=archifyFixture(f,'archify project');
  const result=await install(f,a.project,[...both(f),'--providers=claude,codex']);assert.equal(result.code,0,result.stdout+result.stderr);
  for(const dir of a.skills)assert.equal(fs.existsSync(dir),false,dir);
  for(const file of a.locks)assert.deepEqual(Object.keys(JSON.parse(fs.readFileSync(file)).skills),['design-md'],file);
  assert.equal(fs.readFileSync(path.join(a.project,'docs/old.archify.html'),'utf8'),'<html>old output</html>\n');
  assert.match(result.stdout,/无交互环境默认删除/);assertBoth(a.project,'.claude');
});
nativeTest('--keep-archify, a declined prompt and --no-phoenix-diagram leave Archify installed',async t=>{
  const f=await fixture(t);
  const runs=[
    ['keep flag',a=>install(f,a.project,[...both(f),'--providers=claude,codex','--keep-archify'])],
    ['declined prompt',async a=>{
      // Simulated terminal; the [Y/n] prompt defaults to removal, so only an explicit "n" keeps it.
      const r=spawnSync(process.execPath,['-e','process.stdin.isTTY = true; require(process.argv[1]);',path.join(f.pkg,'bin/xgent-skills.js'),'install',a.project,...both(f),'--providers=claude,codex'],{cwd:a.project,env:f.env,input:'n\n',encoding:'utf8',timeout:600000});
      assert.match(r.stdout,/发现旧 archify.*\[Y\/n\]/);return {code:r.status,stdout:r.stdout,stderr:r.stderr};
    }],
    ['skipped diagram',a=>install(f,a.project,['--release-dir='+f.uiRelease,'--no-phoenix-diagram','--providers=claude,codex'])],
  ];
  for(const [name,run] of runs){
    for(const dir of [path.join(f.env.HOME,'.claude'),path.join(f.env.HOME,'.agents')])fs.rmSync(dir,{recursive:true,force:true});
    const a=archifyFixture(f,`archify ${name}`),locks=a.locks.map(file=>fs.readFileSync(file,'utf8'));
    const result=await run(a);assert.equal(result.code,0,name+result.stdout+result.stderr);
    for(const dir of a.skills)assert.equal(fs.readFileSync(path.join(dir,'SKILL.md'),'utf8'),'old archify\n',`${name}: ${dir}`);
    assert.deepEqual(a.locks.map(file=>fs.readFileSync(file,'utf8')),locks,name);
  }
});
nativeTest('an existing Codex project keeps both tools out of harnesses it does not use across reinstalls',async t=>{
  const f=await fixture(t),project=f.project('codex only project');
  fs.mkdirSync(path.join(project,'.agents'));
  for(let attempt=0;attempt<2;attempt+=1){
    const result=await install(f,project,both(f));assert.equal(result.code,0,result.stdout+result.stderr);
    assertBoth(project,'.agents');
    assert.equal(fs.existsSync(path.join(project,'.agents/agents/agi-agent.md')),true);
    assert.equal(fs.existsSync(path.join(project,'.claude')),false);
  }
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
