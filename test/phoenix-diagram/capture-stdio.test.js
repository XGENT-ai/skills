'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const os=require('node:os');
const {spawnSync}=require('node:child_process');
const {createHash}=require('node:crypto');
const collector=path.resolve(__dirname,'../../tools/phoenix-diagram/tests/support/capture-stdio.mjs');
const progressReader=path.resolve(__dirname,'../../tools/phoenix-diagram/tests/support/read-current-progress.mjs');
const identity=bytes=>({sha256:createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length});
// These owned Node launchers test collection mechanics only. They are never
// native engines, browser evidence, provider sessions or candidate acceptance.
function fixture(t) {
  const directory=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'diagram stdio fixture ')));t.after(()=>fs.rmSync(directory,{recursive:true,force:true}));
  const project=path.join(directory,'project'),evidence=path.join(directory,'capture'),program=path.join(project,'.agents/skills/phoenix-diagram/scripts/phoenix-diagram');fs.mkdirSync(path.dirname(program),{recursive:true});fs.mkdirSync(evidence,{mode:0o700});
  const marker=path.join(directory,'spawned'),response=path.join(project,'response.json'),raw=Buffer.from(JSON.stringify({tool:'phoenix-diagram',fixture:true,payload:'原样💡'.repeat(30000)})+'\n');fs.writeFileSync(response,raw);
  fs.writeFileSync(program,`#!${process.execPath}\nconst fs=require('node:fs');fs.appendFileSync(${JSON.stringify(marker)},'fixture spawn\\n');for(const arg of process.argv.slice(2)){if(arg.startsWith('--fixture-directory='))fs.mkdirSync(arg.slice(20),{mode:0o700});if(arg.startsWith('--fixture-receipt='))fs.appendFileSync(arg.slice(18),' ');}const bytes=fs.readFileSync(${JSON.stringify(response)});process.stdout.write(bytes);process.stderr.write('fixture stderr\\n');process.exitCode=process.argv[2]==='fail'?7:JSON.parse(bytes).status==='failed'?3:0;\n`,{mode:0o755});
  const stat=fs.statSync(program,{bigint:true}),root=fs.statSync(project,{bigint:true}),active=path.join(project,'.phoenix-diagram/install/active.json');fs.mkdirSync(path.dirname(active),{recursive:true});
  fs.writeFileSync(active,JSON.stringify({schemaVersion:1,tool:'phoenix-diagram',project:{device:String(root.dev),inode:String(root.ino)},ownership:[{path:'.agents/skills/phoenix-diagram/scripts/phoenix-diagram',identity:{...identity(fs.readFileSync(program)),device:String(stat.dev),inode:String(stat.ino),mode:Number(stat.mode&0o7777n),uid:Number(stat.uid),links:Number(stat.nlink)}}]}),{mode:0o600});
  const stdout=path.join(evidence,'stdout.json'),stderr=path.join(evidence,'stderr.txt');
  const run=(argv=['doctor','--json'],extra=[],number=0)=>spawnSync(process.execPath,[collector,'--program',program,'--cwd',project,'--stdout',number?path.join(project,'stdio',`call-${number}.stdout`):stdout,'--stderr',number?path.join(project,'stdio',`call-${number}.stderr`):stderr,...extra,'--',...argv],{encoding:'utf8',maxBuffer:1024*1024});
  return{directory,project,evidence,program,active,stdout,stderr,raw,marker,response,run};
}
test('collector keeps a large original stdout in exclusive files and emits only a short bound result',t=>{
  const f=fixture(t),r=f.run();assert.equal(r.status,0,r.stderr);const dto=JSON.parse(r.stdout);
  assert.ok(f.raw.length>227144);assert.ok(Buffer.byteLength(r.stdout)<6000);assert.deepEqual(fs.readFileSync(f.stdout),f.raw);assert.deepEqual(dto.stdout.identity,identity(f.raw));assert.equal(dto.stdout.path,f.stdout);assert.equal(fs.statSync(f.stdout).mode&0o777,0o600);assert.equal(fs.statSync(f.stderr).mode&0o777,0o600);assert.equal(dto.exitCode,0);assert.equal(dto.signal,null);assert.equal(dto.program,f.program);assert.deepEqual(dto.argv,['doctor','--json']);assert.equal(dto.collector.path,collector);assert.equal(dto.node.path,fs.realpathSync(process.execPath));assert.equal(dto.installedReceipt.path,f.active);
});
test('collector preserves actual nonzero child exits without calling them successful',t=>{
  const f=fixture(t),r=f.run(['fail']);assert.equal(r.status,7,r.stderr);const dto=JSON.parse(r.stdout);assert.equal(dto.exitCode,7);assert.equal(dto.signal,null);assert.deepEqual(fs.readFileSync(f.stdout),f.raw);assert.equal(fs.readFileSync(f.stderr,'utf8'),'fixture stderr\n');
});
test('collector refuses claimed output slots and changed installation ownership before spawning',t=>{
  for(const fault of ['existing','symlink','hardlink','launcher','receipt','parent-mode']){
    const f=fixture(t),held=path.join(f.directory,'claimant');fs.writeFileSync(held,'preserve claimant');
    if(fault==='existing')fs.writeFileSync(f.stdout,'preserve claimant');else if(fault==='symlink')fs.symlinkSync(held,f.stdout);else if(fault==='hardlink')fs.linkSync(held,f.stdout);else if(fault==='launcher')fs.appendFileSync(f.program,'// changed\n');else if(fault==='receipt'){const active=JSON.parse(fs.readFileSync(f.active));active.ownership=[];fs.writeFileSync(f.active,JSON.stringify(active));}else fs.chmodSync(f.evidence,0o755);
    const r=f.run();assert.notEqual(r.status,0);assert.equal(fs.existsSync(f.marker),false);assert.equal(fs.readFileSync(held,'utf8'),'preserve claimant');
  }
});

function receipt(errors,{stage='layout',command='validate',scene,composition=true}={}) {
  return{schemaVersion:1,tool:'phoenix-diagram',command,status:errors?'failed':'pass',diagnostics:Array.from({length:errors},(_,i)=>({stage,severity:'error',code:`fixture/${i}`,subject:{path:`/nodes/${i}`,identity:'原💡'},evidence:{measuredScene:scene??{nodes:['保持原值']}},supportedFixes:[`完整修法${i}：`+'甲\n💡'.repeat(20)]})),...(composition?{composition:{summary:{errors}}}:{})};
}
function inspectionFixture(t) {
  const f=fixture(t),stdio=path.join(f.project,'stdio'),chain=path.join(stdio,'inspection-chain');fs.mkdirSync(stdio,{mode:0o700});fs.mkdirSync(chain,{mode:0o700});
  let calls=0,previous=null;
  const run=(dto,argv=[dto.command,'--json'],{predecessor=previous,advance=true}={})=>{
    fs.writeFileSync(f.response,typeof dto==='string'?dto:JSON.stringify(dto)+'\n');const dir=path.join(stdio,`inspection-${++calls}`);
    const result=f.run(argv,['--inspection-dir',dir,...(predecessor?['--previous-inspection',predecessor]:[])],calls);
    let capture;try{capture=JSON.parse(result.stdout);}catch{}
    if(advance && capture?.inspection?.status==='complete')previous=capture.inspection.entry.path;
    return{...result,capture,dir};
  };
  return{...f,stdio,chain,run,get previous(){return previous;},spawns:()=>fs.existsSync(f.marker)?fs.readFileSync(f.marker,'utf8').trim().split('\n').length:0};
}
test('inspection preserves every diagnostic and repair with bounded Unicode pages and value-only scene sharing',async t=>{
  const f=inspectionFixture(t),scene={text:'大场景💡\n\t"'.repeat(7000)},dto=receipt(13,{scene});dto.diagnostics[12].evidence.measuredScene={...scene,text:scene.text+'不同'};dto.source={long:'源💡'.repeat(10000)};dto.diagnostics[0].evidence.source=dto.source;
  const r=f.run(dto);assert.equal(r.status,3,r.stderr);assert.equal(r.capture?.exitCode,3);assert.equal(r.capture.inspection.status,'complete');
  const {readInspection}=await import(collector),view=readInspection(r.capture.inspection.entry.path);
  assert.deepEqual(view.document,dto);assert.deepEqual(fs.readFileSync(r.capture.stdout.path),Buffer.from(JSON.stringify(dto)+'\n'));assert.equal(f.spawns(),1);
  assert.ok(view.entry.totalPages>10);assert.ok(fs.statSync(r.capture.inspection.entry.path).size<=8192);assert.equal(view.index.sharedValues.length,2);
  assert.deepEqual(view.document.diagnostics[0].supportedFixes,dto.diagnostics[0].supportedFixes);
  for(const page of view.pages){assert.ok(page.identity.bytes<=8192);assert.equal(fs.statSync(page.path).mode&0o777,0o600);}
  const changed=view.pages.at(-1);fs.appendFileSync(changed.path,' ');assert.throws(()=>readInspection(r.capture.inspection.entry.path),/changed|differs|identity/);
});
test('inspection counts real inner gates once, preserves best/stall through candidates and separates input, layout and later gates',async t=>{
  const f=inspectionFixture(t),{readInspection}=await import(collector),progress=r=>readInspection(r.capture.inspection.entry.path).progress;
  const first=progress(f.run(receipt(13)));assert.equal(first.observations[0].errors,13);assert.equal(first.observations[0].stall,0);
  const second=progress(f.run(receipt(9)));assert.equal(second.observations[0].best,9);assert.equal(second.observations[0].stall,0);
  const third=progress(f.run(receipt(0)));assert.equal(third.observations[0].errors,0);assert.equal(third.observations[0].best,0);
  const g=inspectionFixture(t);let r=g.run(receipt(1,{stage:'input'}));assert.equal(progress(r).observations[0].phase,'input');
  r=g.run(receipt(1));assert.equal(progress(r).observations[0].stall,0);r=g.run(receipt(1));assert.equal(progress(r).observations[0].stall,1);r=g.run(receipt(1));assert.equal(progress(r).observations[0].stall,2);assert.equal(progress(r).stopRequired,true);
  const h=inspectionFixture(t),inner=receipt(1);inner.diagnostics.push({...inner.diagnostics[0],severity:'warning'});
  const final={schemaVersion:1,tool:'phoenix-diagram',command:'finalize',status:'failed',diagnostics:[...inner.diagnostics,...inner.diagnostics],stages:[{name:'validate',status:'failed',receipt:inner},{name:'deliver',status:'not-run',receipt:null},{name:'check',status:'not-run',receipt:null},{name:'browser-check',status:'not-run',receipt:null}]};
  r=h.run(final);assert.equal(progress(r).observations[0].errors,1);assert.equal(progress(r).observations[1].errors,null);
  const browser=receipt(1,{command:'browser-check',stage:'browser',composition:false});final.stages=final.stages.map((s,i)=>({name:s.name,status:i===3?'failed':'pass',receipt:i===3?browser:receipt(0,{command:s.name})}));
  r=h.run(final);const p=progress(r);assert.equal(p.observations[3].gate,'browser-check');assert.equal(p.observations[3].stall,0);
});
test('chain refuses missing/stale predecessors, forks, unfinished views and unrelated captures before another native call',async t=>{
  const f=inspectionFixture(t),r=f.run(receipt(1)),first=f.previous;assert.equal(r.status,3,r.stderr);
  assert.notEqual(f.run(receipt(1),undefined,{predecessor:null,advance:false}).status,0);assert.equal(f.spawns(),1);
  f.run(receipt(1));assert.equal(f.spawns(),2);const oldHead=fs.readFileSync(path.join(f.chain,'latest.json'));
  const stale=f.run(receipt(1),undefined,{predecessor:first,advance:false});assert.equal(stale.status,4);assert.equal(f.spawns(),2);assert.deepEqual(fs.readFileSync(path.join(f.chain,'latest.json')),oldHead);
  const foreign=inspectionFixture(t);foreign.run(receipt(1));assert.equal(f.run(receipt(1),undefined,{predecessor:foreign.previous,advance:false}).status,4);assert.equal(f.spawns(),2);
  fs.writeFileSync(path.join(f.chain,'.lock'),'held',{flag:'wx',mode:0o600});assert.equal(f.run(receipt(1),undefined,{advance:false}).status,4);assert.equal(f.spawns(),2);assert.equal(fs.readFileSync(path.join(f.chain,'.lock'),'utf8'),'held');
});
test('inspect repairs only derived data for the latest sealed capture and never executes its program',async t=>{
  const f=inspectionFixture(t),r=f.run(receipt(1));assert.equal(r.status,3,r.stderr);const capture=r.capture.captureFile.path,latestBefore=JSON.parse(fs.readFileSync(path.join(f.chain,'latest.json')));
  const dir=path.join(f.stdio,'recovered'),result=spawnSync(process.execPath,[collector,'inspect','--capture',capture,'--inspection-dir',dir],{encoding:'utf8'});assert.equal(result.status,0,result.stderr);assert.equal(f.spawns(),1);
  const {readInspection}=await import(collector),view=readInspection(JSON.parse(result.stdout).inspection.entry.path);assert.deepEqual(view.document,receipt(1));assert.equal(view.progress.sequence,1);
  const latest=JSON.parse(fs.readFileSync(path.join(f.chain,'latest.json')));assert.equal(latest.sequence,latestBefore.sequence);assert.deepEqual(latest.capture,latestBefore.capture);assert.notDeepEqual(latest.inspection,latestBefore.inspection);
  assert.ok(fs.existsSync(r.capture.inspection.entry.path));assert.equal(spawnSync(process.execPath,[collector,'inspect','--capture',capture,'--inspection-dir',path.join(f.stdio,'bad'),'--program',f.program],{encoding:'utf8'}).status,4);assert.equal(f.spawns(),1);
  fs.appendFileSync(r.capture.stdout.path,' ');assert.equal(spawnSync(process.execPath,[collector,'inspect','--capture',capture,'--inspection-dir',path.join(f.stdio,'changed')],{encoding:'utf8'}).status,4);assert.equal(f.spawns(),1);
});
test('unusable DTOs and derived failures retain the original native exit and do not reset history',async t=>{
  const f=inspectionFixture(t),bad={schemaVersion:1,tool:'other',command:'validate',status:'failed',diagnostics:[]},r=f.run(bad);
  assert.equal(r.status,3,r.stderr);assert.equal(r.capture.exitCode,3);const {readInspection}=await import(collector);assert.equal(readInspection(r.capture.inspection.entry.path).progress.status,'not-derivable');
  assert.equal(f.run(receipt(1),undefined,{advance:false}).status,4);assert.equal(f.spawns(),1);
  const g=inspectionFixture(t),broken=g.run('{"not valid"');assert.equal(broken.status,1);assert.equal(broken.capture.exitCode,1);assert.equal(broken.capture.inspection.status,'failed');assert.equal(g.spawns(),1);
  assert.equal(JSON.parse(fs.readFileSync(path.join(g.chain,'latest.json'))).status,'inspection.failed');assert.ok(fs.existsSync(broken.capture.captureFile.path));assert.equal(g.run(receipt(1),undefined,{advance:false}).status,4);assert.equal(g.spawns(),1);
  const attempt=spawnSync(process.execPath,[collector,'inspect','--capture',broken.capture.captureFile.path,'--inspection-dir',path.join(g.stdio,'rebuild')],{encoding:'utf8'});assert.equal(attempt.status,4);assert.equal(g.spawns(),1);assert.equal(fs.readFileSync(broken.capture.stdout.path,'utf8'),'{"not valid"');
});
test('index metadata also paginates, including many distinct scenes and very long repair strings',async t=>{
  const f=inspectionFixture(t),dto=receipt(60);for(let i=0;i<30;i++){const value={nodes:[{label:'不同💡'+i}]};dto.diagnostics[2*i].evidence.measuredScene=value;dto.diagnostics[2*i+1].evidence.measuredScene=value;}
  dto.diagnostics[0].supportedFixes=['含控制字符\u0000💡'.repeat(20000)];const r=f.run(dto);assert.equal(r.capture.inspection.status,'complete');const {readInspection}=await import(collector),view=readInspection(f.previous);
  assert.ok(view.entry.index.identity.bytes>8192);assert.ok(view.entry.indexStream.pages>1);assert.equal(view.index.sharedValues.length,30);assert.deepEqual(view.document,dto);
  for(const page of view.pages)assert.ok(page.identity.bytes<=8192);
});
test('counter fallback, failed zero counts, not-run and review registration never invent quality progress',async t=>{
  const {readInspection}=await import(collector),f=inspectionFixture(t),dto=receipt(2,{composition:false});dto.diagnostics.push({...dto.diagnostics[0],severity:'warning'});let r=f.run(dto);let p=readInspection(f.previous).progress;assert.equal(p.observations[0].errors,2);
  r=f.run({schemaVersion:1,tool:'phoenix-diagram',command:'review-record',status:'failed',diagnostics:receipt(9).diagnostics});p=readInspection(f.previous).progress;assert.equal(p.category,'auxiliary');assert.deepEqual(p.observations,[]);assert.equal(p.gates['validate/layout'].best,2);
  r=f.run(receipt(2));p=readInspection(f.previous).progress;assert.equal(p.observations[0].stall,1);
  const g=inspectionFixture(t),zero=receipt(0);zero.status='failed';r=g.run(zero);p=readInspection(g.previous).progress;assert.equal(p.status,'not-derivable');assert.equal(p.observations[0].errors,null);assert.deepEqual(p.gates,{});
  const h=inspectionFixture(t),notRun=receipt(0);notRun.status='not-run';r=h.run(notRun);p=readInspection(h.previous).progress;assert.equal(p.observations[0].errors,null);assert.deepEqual(p.gates,{});
});
test('successful native output survives a view failure and inspect recovers from that exact capture once',async t=>{
  const f=inspectionFixture(t),dir=path.join(f.stdio,'inspection-1'),r=f.run(receipt(1),['validate','--json',`--fixture-directory=${dir}`]);
  assert.equal(r.status,3,r.stderr);assert.equal(r.capture.inspection.status,'failed');assert.equal(f.spawns(),1);assert.equal(JSON.parse(fs.readFileSync(path.join(f.chain,'latest.json'))).sequence,1);
  const recovered=spawnSync(process.execPath,[collector,'inspect','--capture',r.capture.captureFile.path,'--inspection-dir',path.join(f.stdio,'recovered')],{encoding:'utf8'});assert.equal(recovered.status,0,recovered.stderr);assert.equal(f.spawns(),1);
  const {readInspection}=await import(collector),view=readInspection(JSON.parse(recovered.stdout).inspection.entry.path);assert.equal(view.progress.observations[0].stall,0);assert.equal(view.progress.sequence,1);assert.deepEqual(view.document,receipt(1));
});
test('incomplete raw collection is recorded separately from the observed native result',t=>{
  const f=inspectionFixture(t),r=f.run(receipt(1),['validate','--json',`--fixture-receipt=${f.active}`]);
  assert.equal(r.status,4);assert.equal(r.capture.captureStatus,'failed');assert.equal(r.capture.nativeResult.exitCode,3);assert.equal(r.capture.nativeResult.signal,null);assert.equal(JSON.parse(fs.readFileSync(path.join(f.chain,'latest.json'))).status,'capture.failed');assert.equal(f.spawns(),1);assert.deepEqual(JSON.parse(fs.readFileSync(path.join(f.stdio,'call-1.stdout'))),receipt(1));
});
test('artifact diagnostics retain their actual phase without restarting validation layout history',async t=>{
  const f=inspectionFixture(t),{readInspection}=await import(collector);f.run(receipt(1));f.run(receipt(1,{stage:'artifact'}));let p=readInspection(f.previous).progress;assert.equal(p.status,'derived');assert.equal(p.observations[0].phase,'artifact');assert.equal(p.observations[0].stall,1);
  f.run(receipt(1,{stage:'artifact'}));p=readInspection(f.previous).progress;assert.equal(p.stopRequired,true);
  f.run({schemaVersion:1,tool:'phoenix-diagram',command:'doctor',status:'pass',diagnostics:[]});assert.equal(readInspection(f.previous).progress.stopRequired,true);
});
test('finalize cannot derive success from passed or unrun children when its actual outer status failed',async t=>{
  const {readInspection}=await import(collector);
  for(const status of ['pass','not-run']){const f=inspectionFixture(t),dto={schemaVersion:1,tool:'phoenix-diagram',command:'finalize',status:'failed',diagnostics:receipt(1).diagnostics,stages:['validate','deliver','check','browser-check'].map(name=>({name,status,receipt:status==='pass'?receipt(0,{command:name}):null}))};const r=f.run(dto);assert.equal(r.capture.inspection.status,'complete');assert.equal(readInspection(f.previous).progress.status,'not-derivable');}
});
test('a sealed next-capture slot or abandoned head transaction refuses another native execution',t=>{
  for(const slot of ['capture-2.json','.latest-abandoned.json']){const f=inspectionFixture(t);f.run(receipt(1));const claimed=path.join(f.chain,slot);fs.writeFileSync(claimed,'retain original',{flag:'wx',mode:0o600});const head=fs.readFileSync(path.join(f.chain,'latest.json'));const r=f.run(receipt(1),undefined,{advance:false});assert.equal(r.status,4);assert.equal(f.spawns(),1);assert.equal(fs.readFileSync(claimed,'utf8'),'retain original');assert.deepEqual(fs.readFileSync(path.join(f.chain,'latest.json')),head);}
});
test('normalization also preserves negative zero and literal prototype-looking fields',async t=>{
  const f=inspectionFixture(t),raw='{"schemaVersion":1,"tool":"phoenix-diagram","command":"validate","status":"failed","composition":{"summary":{"errors":1}},"diagnostics":[{"stage":"layout","severity":"error","subject":{"path":"/x"},"evidence":{"measuredScene":{"x":-0}},"supportedFixes":["完整"]}],"__proto__":{"source":{"x":-0}}}\n';const r=f.run(raw,['validate','--json']);assert.equal(r.capture.inspection.status,'complete');const {readInspection}=await import(collector),view=readInspection(f.previous);assert.deepEqual(view.document,JSON.parse(raw));assert.ok(Object.is(view.document.diagnostics[0].evidence.measuredScene.x,-0));assert.deepEqual(fs.readFileSync(r.capture.stdout.path),Buffer.from(raw));
});
test('authenticated byte-identical collector copies can be inspected by the repository harness',async t=>{
  const f=inspectionFixture(t),copy=path.join(f.directory,'collector.mjs');fs.copyFileSync(collector,copy);fs.writeFileSync(f.response,JSON.stringify(receipt(1)));
  const result=spawnSync(process.execPath,[copy,'--program',f.program,'--cwd',f.project,'--stdout',path.join(f.stdio,'copied.stdout'),'--stderr',path.join(f.stdio,'copied.stderr'),'--inspection-dir',path.join(f.stdio,'copied-view'),'--','validate','--json'],{encoding:'utf8'});assert.equal(result.status,3,result.stderr);const dto=JSON.parse(result.stdout),{readInspection}=await import(collector);assert.deepEqual(readInspection(dto.inspection.entry.path).document,receipt(1));assert.equal(dto.collector.path,copy);
});
test('raw captures and derived directories cannot consume the reserved project chain namespace',t=>{
  const f=inspectionFixture(t);fs.writeFileSync(f.response,JSON.stringify(receipt(1)));
  for(const target of ['view','stdout']){const dir=target==='view'?path.join(f.chain,'view'):path.join(f.stdio,'view');const r=spawnSync(process.execPath,[collector,'--program',f.program,'--cwd',f.project,'--stdout',path.join(target==='stdout'?f.chain:f.stdio,'reserved.stdout'),'--stderr',path.join(f.stdio,'reserved.stderr'),'--inspection-dir',dir,'--','validate','--json'],{encoding:'utf8'});assert.equal(r.status,4);assert.equal(f.spawns(),0);}
  const r=f.run(receipt(1));assert.equal(r.status,3,r.stderr);const inspect=spawnSync(process.execPath,[collector,'inspect','--capture',r.capture.captureFile.path,'--inspection-dir',path.join(f.chain,'rebuild')],{encoding:'utf8'});assert.equal(inspect.status,4);assert.equal(f.spawns(),1);assert.equal(fs.existsSync(path.join(f.chain,'rebuild')),false);
});

function readCurrent(project) {
  return spawnSync(process.execPath,[progressReader,'--project',project,'--json'],{encoding:'utf8'});
}
function projectState(project) {
  const rows=[];
  function walk(directory){for(const name of fs.readdirSync(directory).sort()){
    const file=path.join(directory,name),stat=fs.lstatSync(file,{bigint:true});
    rows.push([path.relative(project,file),String(stat.ino),String(stat.mtimeNs),String(stat.ctimeNs),Number(stat.mode),stat.isFile()?identity(fs.readFileSync(file)):null]);
    if(stat.isDirectory())walk(file);
  }}
  walk(project);return rows;
}
test('current progress reader returns the latest mechanical values without writes or another native call',async t=>{
  const f=inspectionFixture(t);f.run(receipt(2));f.run(receipt(2));
  fs.writeFileSync(path.join(f.project,'progress.json'),JSON.stringify({best:99,stall:0}));
  const before=projectState(f.project),spawns=f.spawns(),{readInspection}=await import(collector);
  const result=readCurrent(f.project);assert.equal(result.status,0,result.stderr);
  const view=JSON.parse(result.stdout);assert.equal(view.kind,'phoenix-diagram-current-progress-view');
  assert.equal(view.inspection.path,f.previous);assert.deepEqual(view.data,readInspection(f.previous).progress);
  assert.equal(view.data.sequence,2);assert.equal(view.data.observations[0].best,2);assert.equal(view.data.observations[0].stall,1);
  assert.equal(view.reader.path,progressReader);assert.equal(view.progress.path,path.join(path.dirname(f.previous),'progress.json'));
  assert.equal(f.spawns(),spawns);assert.deepEqual(projectState(f.project),before);
});
test('current progress reader refuses live producers, rewound heads and incomplete chain slots',t=>{
  for(const fault of ['lock','rewound','uncommitted','unfinished']){
    const f=inspectionFixture(t);f.run(receipt(1));const first=fs.readFileSync(path.join(f.chain,'latest.json'));f.run(receipt(1));
    if(fault==='lock')fs.writeFileSync(path.join(f.chain,'.lock'),'preserve live producer');
    else if(fault==='rewound')fs.writeFileSync(path.join(f.chain,'latest.json'),first);
    else if(fault==='uncommitted')fs.writeFileSync(path.join(f.chain,'capture-3.json'),'preserve uncommitted capture');
    else {const head=JSON.parse(fs.readFileSync(path.join(f.chain,'latest.json')));head.status='inspection.failed';fs.writeFileSync(path.join(f.chain,'latest.json'),JSON.stringify(head));}
    const before=projectState(f.project),result=readCurrent(f.project);assert.equal(result.status,4,result.stderr);assert.equal(result.stdout,'');assert.equal(f.spawns(),2);assert.deepEqual(projectState(f.project),before);
  }
});
test('current progress reader authenticates pages, raw stdout and progress before returning any values',t=>{
  for(const fault of ['stdout','progress','page','project-alias']){
    const f=inspectionFixture(t),captured=f.run(receipt(1));let project=f.project;
    if(fault==='stdout')fs.appendFileSync(captured.capture.stdout.path,' ');
    else if(fault==='progress')fs.appendFileSync(path.join(captured.dir,'progress.json'),' ');
    else if(fault==='page'){
      const index=JSON.parse(fs.readFileSync(path.join(captured.dir,'index.json')));fs.appendFileSync(index.document.firstPage.path,' ');
    }else {project=path.join(f.directory,'alias');fs.symlinkSync(f.project,project);}
    const before=projectState(f.project),result=readCurrent(project);assert.equal(result.status,4,result.stderr);assert.equal(result.stdout,'');assert.equal(f.spawns(),1);assert.deepEqual(projectState(f.project),before);
  }
});
test('current progress reader preserves stopRequired and not-derivable without declaring quality success',t=>{
  const stopped=inspectionFixture(t);stopped.run(receipt(1));stopped.run(receipt(1));stopped.run(receipt(1));
  let result=readCurrent(stopped.project);assert.equal(result.status,0,result.stderr);let view=JSON.parse(result.stdout);
  assert.equal(view.data.stopRequired,true);assert.equal(view.data.observations[0].stall,2);assert.equal(stopped.spawns(),3);assert.equal(view.status,undefined);
  const unknown=inspectionFixture(t);unknown.run({schemaVersion:1,tool:'other',command:'validate',status:'failed',diagnostics:[]});
  result=readCurrent(unknown.project);assert.equal(result.status,0,result.stderr);view=JSON.parse(result.stdout);
  assert.equal(view.data.status,'not-derivable');assert.equal(unknown.spawns(),1);assert.equal(view.status,undefined);
});
test('current progress reader rejects a foreign head before opening the foreign inspection',async t=>{
  const f=inspectionFixture(t),foreign=inspectionFixture(t);f.run(receipt(1));foreign.run(receipt(1));
  const head=JSON.parse(fs.readFileSync(path.join(f.chain,'latest.json'))),other=JSON.parse(fs.readFileSync(path.join(foreign.chain,'latest.json')));
  head.inspection=other.inspection;head.capture=other.capture;fs.writeFileSync(path.join(f.chain,'latest.json'),JSON.stringify(head));
  const {readCurrentProgress}=await import(progressReader),open=fs.openSync,opened=[];
  fs.openSync=function(file,...args){if(typeof file==='string' && file.startsWith(foreign.project+path.sep))opened.push(file);return open.call(this,file,...args);};
  try{assert.throws(()=>readCurrentProgress(f.project));}finally{fs.openSync=open;}
  assert.deepEqual(opened,[]);assert.equal(f.spawns(),1);assert.equal(foreign.spawns(),1);
});
