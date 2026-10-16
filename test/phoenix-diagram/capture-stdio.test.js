'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const os=require('node:os');
const {spawnSync}=require('node:child_process');
const {createHash}=require('node:crypto');
const collector=path.resolve(__dirname,'../../tools/phoenix-diagram/tests/support/capture-stdio.mjs');
const identity=bytes=>({sha256:createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length});
// These owned Node launchers test collection mechanics only. They are never
// native engines, browser evidence, provider sessions or candidate acceptance.
function fixture(t) {
  const directory=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'diagram stdio fixture ')));t.after(()=>fs.rmSync(directory,{recursive:true,force:true}));
  const project=path.join(directory,'project'),evidence=path.join(directory,'capture'),program=path.join(project,'.agents/skills/phoenix-diagram/scripts/phoenix-diagram');fs.mkdirSync(path.dirname(program),{recursive:true});fs.mkdirSync(evidence,{mode:0o700});
  const marker=path.join(directory,'spawned'),raw=Buffer.from(JSON.stringify({tool:'phoenix-diagram',fixture:true,payload:'原样💡'.repeat(30000)})+'\n');
  fs.writeFileSync(program,`#!${process.execPath}\nconst fs=require('node:fs');fs.writeFileSync(${JSON.stringify(marker)},'fixture spawn');process.stdout.write(Buffer.from(${JSON.stringify(raw.toString('base64'))},'base64'));process.stderr.write('fixture stderr\\n');process.exitCode=process.argv[2]==='fail'?7:0;\n`,{mode:0o755});
  const stat=fs.statSync(program,{bigint:true}),root=fs.statSync(project,{bigint:true}),active=path.join(project,'.phoenix-diagram/install/active.json');fs.mkdirSync(path.dirname(active),{recursive:true});
  fs.writeFileSync(active,JSON.stringify({schemaVersion:1,tool:'phoenix-diagram',project:{device:String(root.dev),inode:String(root.ino)},ownership:[{path:'.agents/skills/phoenix-diagram/scripts/phoenix-diagram',identity:{...identity(fs.readFileSync(program)),device:String(stat.dev),inode:String(stat.ino),mode:Number(stat.mode&0o7777n),uid:Number(stat.uid),links:Number(stat.nlink)}}]}),{mode:0o600});
  const stdout=path.join(evidence,'stdout.json'),stderr=path.join(evidence,'stderr.txt');
  const run=(argv=['doctor','--json'])=>spawnSync(process.execPath,[collector,'--program',program,'--cwd',project,'--stdout',stdout,'--stderr',stderr,'--',...argv],{encoding:'utf8',maxBuffer:1024*1024});
  return{directory,project,evidence,program,active,stdout,stderr,raw,marker,run};
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
