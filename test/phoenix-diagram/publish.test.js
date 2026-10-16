'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const { pathToFileURL } = require('node:url');
const root = path.resolve(__dirname, '../..');
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const targets = ['darwin-arm64','darwin-x64','linux-x64','linux-arm64'];
function write(dir, name, value, mode = 0o644) {
  const file = path.join(dir, name); fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, value, { mode }); fs.chmodSync(file, mode);
}
function fixture(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram publish guard '));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'))).version;
  write(dir, 'package.json', JSON.stringify({ name:'@xgent-ai/skills',version,bin:{'phoenix-ui':'bin/phoenix-ui.js','phoenix-diagram':'bin/phoenix-diagram.js'},files:['bin','dist/phoenix-ui','dist/phoenix-diagram'] }));
  write(dir, 'dist/phoenix-ui/VERSION.json', JSON.stringify({ distribution:'release',engines:{'linux-x64':{url:'https://download.example.test/ui'}} }));
  const files = [['SKILL.md',Buffer.from('Fixture skill\n'),0o644], ...['phoenix-bootstrap.cjs','phoenix-launcher.cjs','phoenix-diagram'].map(name => [`scripts/${name}`,fs.readFileSync(path.join(root,'skills/phoenix-diagram/scripts',name)),name==='phoenix-bootstrap.cjs'?0o644:0o755])];
  for (const [name,bytes,mode] of files) write(dir,`dist/phoenix-diagram/bundle/${name}`,bytes,mode);
  const manifest = Buffer.from(JSON.stringify({schemaVersion:1,tool:'phoenix-diagram',version:'0.1.0',files:files.map(([name,bytes,mode])=>({path:name,size:bytes.length,sha256:digest(bytes),mode}))})+'\n');
  write(dir,'dist/phoenix-diagram/bundle/manifest.json',manifest);
  // Metadata fixtures exercise the guard, never native-platform execution.
  const data={schemaVersion:1,distribution:'release',tool:'phoenix-diagram',toolVersion:'0.1.0',npmPackageVersion:version,sourceCommit:'a'.repeat(40),sourceInputSha256:'b'.repeat(64),inputContractSchema:1,receiptSchema:1,viewerRevision:'phoenix-v1',bundleSha256:digest(manifest),engines:Object.fromEntries(targets.map(target=>[target,{asset:`phoenix-diagram-${target}`,size:3,sha256:'c'.repeat(64),url:`https://download.example.test/phoenix-diagram-${target}`}]))};
  const seal=()=>{const bytes=JSON.stringify(data)+'\n';write(dir,'dist/phoenix-diagram/VERSION.json',bytes);write(dir,'dist/phoenix-diagram/bundle/ENGINE.json',bytes);};
  seal();
  const license=fs.readFileSync(path.join(root,'LICENSE'));
  for(const name of ['LICENSE','NOTICE.md','THIRD-PARTY-NOTICES.md']) write(dir,`dist/phoenix-diagram/${name}`,license);
  write(dir,'dist/phoenix-diagram/THIRD-PARTY-LICENSES/fixture.txt',license);
  write(dir,'dist/phoenix-diagram/THIRD-PARTY.json',JSON.stringify({licenses:[{path:'THIRD-PARTY-LICENSES/fixture.txt',sha256:digest(license)}]}));
  return {dir,data,seal};
}
const guard=dir=>spawnSync(process.execPath,[path.join(root,'scripts/require-release-package.mjs'),dir],{encoding:'utf8'});

test('diagram resources cannot bypass the publish guard through a valid UI manifest',t=>{
  const f=fixture(t); fs.unlinkSync(path.join(f.dir,'dist/phoenix-diagram/VERSION.json'));
  const result=guard(f.dir); assert.equal(result.status,1); assert.match(result.stderr,/phoenix-diagram/i);
});
test('diagram publish guard rejects candidate, missing platform, wrong package and nonpublic URL',t=>{
  for(const change of [v=>v.distribution='candidate',v=>delete v.engines['darwin-x64'],v=>v.npmPackageVersion='99.0.0',v=>v.engines['linux-x64'].url='http://127.0.0.1:48765/engine']){
    const f=fixture(t);change(f.data);f.seal();assert.equal(guard(f.dir).status,1);
  }
});
test('diagram publish guard verifies the actual bundle and full license resources',t=>{
  for(const file of ['bundle/SKILL.md','THIRD-PARTY-LICENSES/fixture.txt']){
    const f=fixture(t);fs.appendFileSync(path.join(f.dir,'dist/phoenix-diagram',file),'changed');assert.equal(guard(f.dir).status,1,file);
  }
  const f=fixture(t);assert.equal(guard(f.dir).status,0,'This proves the static release contract only, not public hosting or a release authorization');
});
test('UI release blocks a package carrying diagram before any release state or uploads',async t=>{
  const script=fs.readFileSync(path.join(root,'scripts/release-phoenix-ui.mjs'),'utf8');
  const call=script.indexOf('assertUiOnlyRelease(root)');assert.ok(call>script.indexOf("step('Checkout')"));
  assert.ok(call<script.indexOf('fs.mkdirSync(dir'),'gate must precede even local release state');
  const {assertUiOnlyRelease}=await import(pathToFileURL(path.join(root,'scripts/require-release-package.mjs')));
  assert.equal(typeof assertUiOnlyRelease,'function');
  const f=fixture(t);assert.throws(()=>assertUiOnlyRelease(f.dir),/joint|联合/i);
});
