#!/usr/bin/env node
// Candidate maintenance only. Consumer commands execute the unchanged npm
// tarball. A source-less harness supplies drivers; native/assembly receipts
// bind the canonical source provenance without shipping Rust or renderers.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import { gunzipSync } from 'node:zlib';
import { randomBytes } from 'node:crypto';
import { spawn,spawnSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual as same } from 'node:util';
import { TARGETS,DEFAULT_PORT,readTarballInventory,assertArtifactAgreement,assertRuntimeSources } from './pack-phoenix-diagram-candidate.mjs';
import { readSourceInputManifest } from './build-phoenix-diagram.mjs';
import { root,TYPES,contractsFor,identity,readSnapshot,readBound,bound,requireThat,publicNpmCommand } from './test-phoenix-diagram-core.mjs';
import { runCoreStage,invokeNative,refreshEvidence } from './test-phoenix-diagram-runtime.mjs';
import { inspectNpm } from './prepare-phoenix-diagram.mjs';
import { MIGRATION_FIXTURE_MEMBER,MIGRATION_FIXTURE_IDENTITY,readMigrationFixture } from '../tools/phoenix-diagram/tests/support/p3-migration.mjs';
export { publicNpmCommand };

const KIND='phoenix-diagram-candidate-consumer';
const REVIEW_KIND='phoenix-diagram-candidate-review';
const HASH=/^[a-f0-9]{64}$/;
const LEGACY_DESCRIPTOR_IDENTITY={bytes:1468,sha256:'193f11ed47b7eecc72662c3d8c594ac768bdcd7565692b9901efb0ce22fb6a95'};
const MAX_REVIEW=2*1024*1024;
const WAIT_MS=90*60*1000;
const POLL_BUDGET_MS=85*60*1000; // Leave the 90-minute step time for review-record.
const OVERRIDES=['dist/phoenix-diagram/VERSION.json','dist/phoenix-diagram/bundle/ENGINE.json','skills/phoenix-diagram/ENGINE.json'];
const NPM_ENV_PROBE=`const{spawnSync}=require('node:child_process');const r=spawnSync('sh',['-c','command -v rustc; command -v cargo; exit 0'],{encoding:'utf8'});if(r.error||r.status!==0||r.stdout.trim())throw new Error('Rust is resolvable inside npm exec');process.stdout.write(JSON.stringify({rustc:'unresolvable',cargo:'unresolvable',path:process.env.PATH,node:process.version}));`;
const LEGACY={
  architecture:{file:'web-app.architecture.json',sha256:'483350f5297df682aba4e4a0fa491307ce3d3abd725ae4df07fab177490752cf'},
  workflow:{file:'agent-tool-call.workflow.json',sha256:'4d3f6c23de2ca7d306e05d0041a9506833bb5a164a2bddc2d1984cf4da3ee90c'},
  sequence:{file:'async-job-roundtrip.sequence.json',sha256:'545f20920c89b69c2bb1f15508d5543fddbeb15cffba61e102206e04bd5a5cd9'},
  dataflow:{file:'product-analytics.dataflow.json',sha256:'91411404fb4828a3b7ea358f3a7b9877781c9e9dc9c04b42b6eb4a757281b59f'},
  lifecycle:{file:'deployment-release.lifecycle.json',sha256:'735c2c5819266e6ea2a71775b6eefce98b53030ce90cb721dc8e1d022ce5be71'},
};
const sorted=values=>[...values].sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b)));
const pretty=value=>Buffer.from(`${JSON.stringify(value,null,2)}\n`);
const descriptor=(name,bytes)=>({path:name,size:bytes.length,sha256:identity(bytes).sha256});
function relative(name) {requireThat(typeof name==='string' && name && !path.isAbsolute(name) && !/[\\:\0-\x1f\x7f]/.test(name) && name.split('/').every(part=>part && part!=='.' && part!=='..' && !/[. ]$/.test(part)),'Unsafe candidate relative path');return name;}
function ordinaryDirectory(directory) {const stat=fs.lstatSync(directory);requireThat(stat.isDirectory() && !stat.isSymbolicLink(),'Expected an ordinary directory, not a symbolic link');return fs.realpathSync(directory);}
function mkdir(directory) {fs.mkdirSync(directory,{mode:0o700});return ordinaryDirectory(directory);}
function writeNew(file,bytes,mode=0o600) {fs.writeFileSync(file,Buffer.isBuffer(bytes)?bytes:pretty(bytes),{flag:'wx',mode});fs.chmodSync(file,mode);return bound(readSnapshot(file));}
function errorRecord(error) {return{code:error.code || 'consumer/failure',message:error.message};}
function hostHeader() {return{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'check-candidate',kind:KIND};}
export function targetForHost(platform=process.platform,arch=process.arch) {const target=`${platform}-${arch}`;requireThat(TARGETS.includes(target),'Unsupported native consumer host');return target;}
// Scopes a local full-review consumption may add; the corpus needs the source checkout's frozen reference.
export const CONSUMER_SCOPES=['explore','evidence','locale'];
export const P3_HARNESS_MEMBERS=Object.freeze([
  'scripts/aggregate-phoenix-diagram-candidate.mjs', 'scripts/check-phoenix-diagram-candidate.mjs',
  'scripts/test-phoenix-diagram.mjs', 'scripts/test-phoenix-diagram-core.mjs', 'scripts/test-phoenix-diagram-runtime.mjs',
  'scripts/test-phoenix-diagram-scopes.mjs', 'scripts/test-phoenix-diagram-recovery.mjs',
  'scripts/pack-phoenix-diagram-candidate.mjs', 'scripts/build-phoenix-diagram.mjs',
  'scripts/check-phoenix-diagram-source.mjs', 'scripts/prepare-phoenix-diagram.mjs',
  'skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs', 'tools/phoenix-diagram/tests/export-acceptance.mjs',
  ...['export-browser','export-page','export-reader','explore-reader','capture-stdio','p3-state','p3-processes',
    'p3-migration','p3-migration-run','p3-provider','capture-p3-stdio','capture-p3-provider'].map(name=>`tools/phoenix-diagram/tests/support/${name}.mjs`),
  'tools/phoenix-diagram/tests/p3-acceptance.mjs', 'tools/phoenix-diagram/tests/provider/p3-scenarios.json',
  'tools/phoenix-diagram/tests/fixtures/brands/official-resource.json', MIGRATION_FIXTURE_MEMBER,
  'tools/phoenix-diagram/tests/fixtures/migration/legacy-candidate.v1.json',
  'tools/phoenix-ui/build-tool-assets.json', 'tools/phoenix-diagram/build-tools.lock.json',
  'LICENSE', 'tools/phoenix-ui/LICENSE', 'tools/phoenix-ui/NOTICE.md', 'tools/phoenix-diagram/viewer/LICENSE.txt',
]);
export function parseOptions(argv) {
  const options={},fields={'candidate-dir':'candidateDir',target:'target',profile:'profile','evidence-dir':'evidenceDir',resume:'resume','review-ref':'reviewRef','review-file':'reviewFile','evidence-repo':'evidenceRepository','npm-cli':'npmCliPath'};
  for(let at=0;at<argv.length;at++) {
    if(argv[at]==='--help' && argv.length===1)return{help:true};
    const match=/^--([^=]+)(?:=(.*))?$/.exec(argv[at]);
    if(match && CONSUMER_SCOPES.includes(match[1])){requireThat(match[2]===undefined && !options[match[1]],`--${match[1]} takes no value and is not repeatable`,'arguments/invalid');options[match[1]]=true;continue;}
    requireThat(match && Object.hasOwn(fields,match[1]),'Unknown candidate checker option','arguments/invalid');const key=fields[match[1]];
    requireThat(!Object.hasOwn(options,key),'Duplicate candidate checker option','arguments/invalid');const value=match[2]??argv[++at];requireThat(value && !value.startsWith('--'),'Missing candidate checker option value','arguments/invalid');options[key]=value;
  }
  requireThat(options.candidateDir && options.target,'--candidate-dir DIR and --target PLATFORM are required','arguments/invalid');
  requireThat(options.target===targetForHost(),'--target must equal the actual native host platform','arguments/invalid');
  requireThat(options.profile===undefined || ['p1','p2','p3'].includes(options.profile),'Unknown candidate profile','arguments/invalid');
  requireThat(!options.npmCliPath || path.isAbsolute(options.npmCliPath),'--npm-cli requires an absolute prepared CLI path','arguments/invalid');
  requireThat(options.resume?Boolean(options.reviewRef)!==Boolean(options.reviewFile):!options.reviewRef && !options.reviewFile,'--resume needs exactly one of --review-ref or --review-file, and neither applies without it','arguments/invalid');requireThat(!options.resume || !options.evidenceDir,'Resume retains its original evidence directory','arguments/invalid');
  requireThat(!options.resume || CONSUMER_SCOPES.every(name=>!options[name]),'Acceptance scopes run with the initial consumption','arguments/invalid');
  requireThat(Boolean(options.evidence)===Boolean(options.evidenceRepository) && (!options.evidenceRepository || path.isAbsolute(options.evidenceRepository)),'--evidence requires --evidence-repo with an absolute git checkout path, and vice versa','arguments/invalid');return options;
}
export function verifyCandidateInventory(directory) {
  directory=ordinaryDirectory(path.resolve(directory));const snapshot=readSnapshot(path.join(directory,'candidate.json')),record=JSON.parse(snapshot.content.toString('utf8'));
  requireThat(record.schemaVersion===1 && record.tool==='phoenix-diagram' && record.status==='assembled' && record.handoff==='pending-consumer' && record.releaseDir==='release-dir' && Array.isArray(record.files),'Unsupported candidate inventory');
  const expected=new Map();for(const file of record.files) {relative(file.path);requireThat(file.path!=='candidate.json' && !expected.has(file.path) && Number.isSafeInteger(file.size) && file.size>=0 && HASH.test(file.sha256),'Invalid or duplicate candidate inventory row');expected.set(file.path,file);}
  const files=new Map();
  const visit=(parent,prefix='')=>{for(const name of fs.readdirSync(parent)) {
    const member=prefix?`${prefix}/${name}`:name;relative(member);if(member==='candidate.json')continue;
    const full=path.join(parent,name),stat=fs.lstatSync(full);
    if(stat.isDirectory() && !stat.isSymbolicLink()) {requireThat([...expected.keys()].some(key=>key.startsWith(`${member}/`)),'Extra candidate inventory directory');visit(full,member);}
    else {const actual=readSnapshot(full),wanted=expected.get(member);requireThat(wanted && same(actual.identity,{sha256:wanted.sha256,bytes:wanted.size}),`Candidate inventory digest or bytes differ: ${member}`);files.set(member,actual);}
  }};visit(directory);requireThat(same(sorted(files.keys()),sorted(expected.keys())),'Candidate inventory has missing or extra files');return{directory,record,snapshot,files};
}
function tarMembers(bytes) {
  const inventory=readTarballInventory(bytes),data=gunzipSync(bytes,{maxOutputLength:512*1024*1024}),members=new Map();let nextPath,ended=false;
  for(let offset=0;offset+512<=data.length;) {
    const header=data.subarray(offset,offset+512);if(header.every(byte=>byte===0)){requireThat(offset+1024<=data.length && nextPath===undefined,'Missing npm tar footer');ended=true;break;}
    const string=(from,to)=>header.subarray(from,to).toString().replace(/\0.*$/,'');const prefix=string(345,500),name=nextPath??`${prefix?`${prefix}/`:''}${string(0,100)}`;
    const size=parseInt(string(124,136).trim(),8),body=data.subarray(offset+512,offset+512+size),type=header[156];
    if([0,48].includes(type)) {const key=name.slice(8),entry=inventory.get(key);requireThat(entry && entry.size===body.length && entry.sha256===identity(body).sha256 && !members.has(key),'Tar member bytes differ from the checked inventory');members.set(key,{...entry,bytes:body});nextPath=undefined;}
    else if(type===76)nextPath=body.toString().replace(/\0.*$/,'');
    else if(type===120)for(let at=0;at<body.length;){const space=body.indexOf(32,at),length=Number(body.subarray(at,space).toString()),record=body.subarray(space+1,at+length-1).toString();if(record.startsWith('path='))nextPath=record.slice(5);at+=length;}
    else nextPath=undefined;
    offset+=512+Math.ceil(size/512)*512;
  }
  requireThat(ended && members.size===inventory.size,'Incomplete npm tar byte inventory');return members;
}
function checkedFile(candidate,name) {const file=candidate.files.get(relative(name));requireThat(file,`Missing candidate file ${name}`);return file;}
function execText(program,args,options={}) {const result=spawnSync(program,args,{cwd:root,encoding:'utf8',maxBuffer:256*1024*1024,...options});requireThat(!result.error && result.status===0,`${path.basename(program)} failed: ${result.error?.message || result.stderr}`);return result.stdout;}
function verifySourceTuples(members,inputs,versionBytes) {
  const tuples=new Map(inputs.entries.map(([name,size,sha256])=>[name,{size,sha256}]));
  for(const [name,file] of members)if(tuples.has(name) && !OVERRIDES.includes(name))requireThat(file.size===tuples.get(name).size && file.sha256===tuples.get(name).sha256,`Packed native source-input tuple differs: ${name}`);
  for(const name of OVERRIDES)requireThat(members.get(name)?.bytes.equals(versionBytes),`Candidate override differs: ${name}`);
}
// Only the authenticated tar members and their original source tuples select
// a layout. Historical engines did not include examples in their input roots.
export function selectLegacySamples(members,sourceInputs,{profile}={}) {
  requireThat(members instanceof Map && Array.isArray(sourceInputs?.entries),'Legacy sample members/source identity is unavailable');
  const oldPaths=Object.fromEntries(Object.entries(LEGACY).map(([type,sample])=>[type,`skills/archify/examples/${sample.file}`]));
  const newPaths=Object.fromEntries(Object.entries(LEGACY).map(([type,sample])=>[type,`skills/phoenix-diagram/examples/legacy.${sample.file}`]));
  const oldCount=Object.values(oldPaths).filter(name=>members.has(name)).length,newCount=Object.values(newPaths).filter(name=>members.has(name)).length;
  requireThat(oldCount===5 && newCount===0 || newCount===5 && oldCount===0,'Legacy sample layout is mixed, missing or unknown');
  const layout=oldCount===5?'archify':'phoenix-flat',paths=layout==='archify'?oldPaths:newPaths;
  requireThat(profile!=='p3' || layout==='phoenix-flat','P3 requires the new flat legacy sample layout');
  if(layout==='phoenix-flat')requireThat(![...members.keys()].some(name=>name.toLowerCase().startsWith('skills/archify/')),'New legacy sample layout retains an active Archify directory');
  const known=new Set([...Object.values(oldPaths),...Object.values(newPaths)]),rows=sourceInputs.entries.filter(row=>known.has(row[0])),tuples=new Map(rows.map(row=>[row[0],row]));
  requireThat(rows.length===tuples.size && (layout==='archify' && rows.length===0 || rows.length===5 && Object.values(paths).every(name=>tuples.has(name))),'Legacy sample source tuples do not identify the selected layout');
  for(const [type,name] of Object.entries(paths)) {
    const file=members.get(name),expected=LEGACY[type].sha256;
    requireThat(Buffer.isBuffer(file?.bytes) && file.size===file.bytes.length && file.sha256===expected && identity(file.bytes).sha256===expected,`Legacy sample bytes/digest differ: ${name}`);
    const tuple=tuples.get(name);
    if(tuple)requireThat(tuple.length===3 && tuple[1]===file.size && tuple[2]===expected,`Legacy sample source tuple differs: ${name}`);
  }
  return{layout,paths};
}
export function verifyCandidateCohort(candidate,{legacyDescriptor}={}) {
  if(legacyDescriptor){
    requireThat(same(identity(legacyDescriptor),LEGACY_DESCRIPTOR_IDENTITY),'Historical v1 descriptor bytes differ');
    const descriptor=JSON.parse(legacyDescriptor);requireThat(descriptor.schemaVersion===1 && descriptor.kind==='phoenix-diagram-v1-candidate-fixture','Historical v1 descriptor contract differs');
    requireThat(same(descriptor.source,candidate.source),'Historical package source differs from authenticated v1 input');
    requireThat(same(descriptor.tarball,candidate.tgz.identity),'Historical package tgz differs');
    for(const tuple of descriptor.writerSources)requireThat(candidate.sourceInputs.entries.some(row=>same(row,tuple)),'Historical writer source tuple differs');
    return descriptor;
  }
  if(process.env.PHOENIX_DIAGRAM_TGZ_SHA256)requireThat(candidate.tgz.identity.sha256===process.env.PHOENIX_DIAGRAM_TGZ_SHA256,'Workflow tgz SHA differs');if(process.env.PHOENIX_DIAGRAM_SOURCE_COMMIT)requireThat(candidate.source.commit===process.env.PHOENIX_DIAGRAM_SOURCE_COMMIT,'Workflow source commit differs');
}
export function validateCandidate(directory,target,{profile,legacyDescriptor}={}) {
  requireThat(target===targetForHost(),'Candidate target must be the current native host');
  requireThat(!legacyDescriptor || profile==='p2','Historical v1 validation requires its original P2 contract');
  const candidate=verifyCandidateInventory(directory),record=candidate.record;
  requireThat(record.port===DEFAULT_PORT && same(record.targets,TARGETS) && record.toolVersion==='0.2.0' && HASH.test(record.source?.inputSha256),'Candidate target/port/source contracts differ');
  const tgz=checkedFile(candidate,record.tarball?.path);requireThat(record.tarball.path.endsWith('.tgz') && same(descriptor(record.tarball.path,tgz.content),record.tarball),'Candidate tarball digest differs');
  requireThat(candidate.files.has('release-dir/VERSION.json') && candidate.files.has('release-dir/bundle/manifest.json'),'Candidate release resources are missing');
  const versionBytes=checkedFile(candidate,'release-dir/VERSION.json').content,version=JSON.parse(versionBytes),members=tarMembers(tgz.content),pkg=JSON.parse(members.get('package.json')?.bytes.toString('utf8'));
  requireThat(pkg.name==='@xgent-ai/skills' && pkg.version===record.npmPackageVersion && same(pkg.bin,{skills:'bin/xgent-skills.js','xgent-skills':'bin/xgent-skills.js','phoenix-ui':'bin/phoenix-ui.js','phoenix-diagram':'bin/phoenix-diagram.js'}) && !pkg.dependencies && !pkg.optionalDependencies,'Unsupported npm identity, public bins or runtime dependency boundary');
  requireThat(version.distribution==='candidate' && version.npmPackageVersion===pkg.version && version.sourceCommit===record.source.commit && version.sourceInputSha256===record.source.inputSha256 && same(Object.keys(version.engines),TARGETS),'Candidate VERSION source/platform identity differs');
  requireThat(same(record.derivation?.version,descriptor('release-dir/VERSION.json',versionBytes)) && same(record.derivation?.bundleManifest,descriptor('release-dir/bundle/manifest.json',checkedFile(candidate,'release-dir/bundle/manifest.json').content)),'Candidate derivation bytes differ');
  const inputs=readSourceInputManifest(checkedFile(candidate,'source-inputs.json').content);requireThat(inputs.sha256===record.source.inputSha256,'Actual canonical source-input bytes differ from the candidate source identity');
  const frozenIdentity=value=>{const copy=structuredClone(value);delete copy.reference.localStatus;return copy;};
  requireThat(record.derivation.parentVersions?.length===4 && same(sorted(record.derivation.parentVersions.map(row=>row.target)),sorted(TARGETS)) && same(record.derivation.stagedOverrides,OVERRIDES.map(name=>descriptor(name,versionBytes))),'Candidate derivation scope is incomplete');
  const bundleBytes=checkedFile(candidate,'release-dir/bundle/manifest.json').content,bundleManifest=JSON.parse(bundleBytes),bundleFiles=new Map(bundleManifest.files.map(file=>{const actual=checkedFile(candidate,`release-dir/bundle/${relative(file.path)}`);requireThat(actual.identity.sha256===file.sha256 && actual.identity.bytes===file.size && actual.binding.mode===file.mode,'Release bundle bytes/mode differ');return[file.path,actual.content];}));
  const receipts=new Map();requireThat(record.buildReceipts?.length===4,'Exactly four build receipts are required');
  const artifacts=record.buildReceipts.map(binding=>{
    requireThat(TARGETS.includes(binding.target) && !receipts.has(binding.target),'Unknown or duplicate build receipt target');const receiptBytes=checkedFile(candidate,binding.path).content;requireThat(same(descriptor(binding.path,receiptBytes),{path:binding.path,size:binding.size,sha256:binding.sha256}),'Build receipt digest differs');const receipt=JSON.parse(receiptBytes);receipts.set(binding.target,receipt);
    const binary=checkedFile(candidate,`release-dir/phoenix-diagram-${binding.target}`),meta=version.engines[binding.target];requireThat(meta.url===`http://127.0.0.1:${DEFAULT_PORT}/${meta.asset}` && binary.binding.mode===0o755 && meta.asset===`phoenix-diagram-${binding.target}` && meta.sha256===binary.identity.sha256 && meta.size===binary.identity.bytes,'Actual engine bytes, mode or frozen loopback URL differ');
    const development={...version,distribution:'development',engines:{[binding.target]:{asset:meta.asset,size:meta.size,sha256:meta.sha256}}};requireThat(record.derivation.parentVersions?.some(row=>row.target===binding.target && row.size===pretty(development).length && row.sha256===identity(pretty(development)).sha256),'Parent development VERSION derivation differs');
    requireThat(same(receipt.sourceInputs,descriptor('source-inputs.json',inputs.bytes)) && receipt.frozenSources?.reference?.archiveSha256==='6831ba7f7d9d5c0fd473e317bd9b513f2581b9d4d63bd157d1de4680af3b32a4' && same(frozenIdentity(receipt.frozenSources),frozenIdentity(receipts.values().next().value.frozenSources)),'Build source tuples or frozen archive attestations differ');
    for(const field of ['template','sourceManifest']) {const value=receipt.viewer?.[field],tuple=inputs.entries.find(row=>row[0]===value?.path);requireThat(tuple && tuple[1]===value.size && tuple[2]===value.sha256,'Reader source identity differs from canonical native tuples');}
    const licenses=new Map(receipt.legal.files.map(file=>{const actual=checkedFile(candidate,`release-dir/${relative(file.path)}`);requireThat(actual.identity.sha256===file.sha256 && actual.identity.bytes===file.size,'Full legal bytes differ');return[file.path,actual.content];}));
    return{target:binding.target,receipt,version:development,sourceInputs:inputs,binary:{asset:meta.asset,size:meta.size,sha256:meta.sha256,bytes:binary.content},bundle:{manifest:bundleManifest,bytes:bundleBytes,sha256:identity(bundleBytes).sha256,files:bundleFiles},licenses,licenseSha256:receipt.legal.sha256};
  });
  const agreement=assertArtifactAgreement(artifacts,{sourceInputs:inputs,npmPackageVersion:pkg.version,requireBrands:profile==='p3'});requireThat(same(agreement.source,record.source),'Candidate source commit/tree differs from the four native build receipts');verifySourceTuples(members,inputs,versionBytes);assertRuntimeSources(new Map([...members].map(([name,file])=>[name,{bytes:file.bytes,mode:file.mode}])),artifacts[0]);
  const expected=[record.tarball.path,'source-inputs.json',...TARGETS.map(name=>`build-receipts/${name}.json`),'release-dir/VERSION.json','release-dir/bundle/ENGINE.json','release-dir/bundle/manifest.json',...bundleManifest.files.map(file=>`release-dir/bundle/${file.path}`),...[...artifacts[0].licenses.keys()].map(name=>`release-dir/${name}`)];
  expected.push(...TARGETS.map(name=>`release-dir/phoenix-diagram-${name}`));requireThat(same(sorted(candidate.files.keys()),sorted(expected)),'Candidate contains extra or missing release/receipt resources');
  const legacySamples=selectLegacySamples(members,inputs,{profile});
  for(const name of ['docs/phoenix-diagram.md','bin/phoenix-diagram.js','bin/xgent-skills.js',...TYPES.map(type=>`skills/phoenix-diagram/examples/minimal.${type}.json`)])requireThat(members.has(name),`Consumption package is missing ${name}`);
  verifyCandidateCohort({tgz,source:record.source,sourceInputs:inputs},{legacyDescriptor:legacyDescriptor?readBound(legacyDescriptor,'fixed historical v1 descriptor').content:undefined});
  return{...candidate,tgz,version,pkg,members,sourceInputs:inputs,legacySamples,source:record.source,engine:checkedFile(candidate,`release-dir/phoenix-diagram-${target}`),frozenSources:artifacts[0].receipt.frozenSources,sourceVerification:{scope:'Actual canonical tuples and four checksum-bound native/assembly attestations; full source/archive reconstruction belongs to native and assemble jobs',status:'pass'}};
}

export function validateReviewEnvelope(value,state,stateIdentity) {
  requireThat(value?.schemaVersion===1 && value.kind===REVIEW_KIND,'Unsupported review envelope');
  for(const [key,expected] of Object.entries({repository:state.repository,runId:state.runId,target:state.target,nonce:state.nonce,stateSha256:stateIdentity.sha256,candidateSha256:state.candidate.tarball.identity.sha256,sourceCommit:state.source.commit}))requireThat(value[key]===expected,`Review envelope ${key} binding differs`);
  requireThat(value.records && typeof value.records==='object' && !Array.isArray(value.records) && same(sorted(Object.keys(value.records)),sorted(state.cases.map(entry=>entry.id))),'Review record case scope is incomplete or unexpected');
  for(const entry of state.cases) {
    const record=value.records[entry.id],current=entry.state;
    requireThat(record?.schemaVersion===1 && record.tool==='phoenix-diagram' && record.toolVersion==='0.2.0' && record.command==='review-record' && record.status==='pass' && record.deliveryReceiptId===current.deliveryReceiptId && same(record.browserReceiptIdentity,current.browserReceiptIdentity) && record.configSha256===current.configSha256 && same(record.engine,current.engine) && same(record.artifact,{path:current.artifact.path,identity:current.artifact.identity}) && same(record.candidate,{path:current.candidate.path,identity:current.candidate.identity}),'Visual review record original identity differs');
    requireThat(typeof record.reviewer==='string' && record.reviewer.trim() && same(sorted(record.scope || []),sorted(['diagram','labels','cards','relationship-occlusion','hierarchy','controls'])) && ['passed','failed','unavailable'].includes(record.verdict),'Visual review reviewer or observed scope is missing');
    requireThat(record.screenshotIdentities?.length===4,'Four actual screenshot observations are required');const seen=new Set();
    for(const observation of record.screenshotIdentities) {const screenshot=current.screenshots.find(file=>same(observation.file,{path:file.path,identity:file.identity}) && same(observation.viewport,file.viewport) && observation.theme===file.theme);requireThat(screenshot && !seen.has(screenshot.path),'Screenshot review binding or viewport scope differs');seen.add(screenshot.path);if(record.verdict==='passed')requireThat(observation.observed===true && typeof observation.observations==='string' && observation.observations.trim(),'An unobserved screenshot cannot pass');}
  }
  return value.records;
}
export function decodeGitHubReview(response,target) {
  requireThat(response?.type==='file' && response.path===`reviews/${target}.json` && response.encoding==='base64' && Number.isSafeInteger(response.size) && response.size>0 && response.size<=MAX_REVIEW,'Unexpected GitHub review path, file, encoding or size');
  requireThat(typeof response.content==='string','Missing base64 review content');const encoded=response.content.replace(/\r?\n/g,'');requireThat(/^[A-Za-z0-9+/]*={0,2}$/.test(encoded),'Invalid base64 review content');const bytes=Buffer.from(encoded,'base64');requireThat(bytes.length===response.size && bytes.toString('base64')===encoded,'GitHub review content size/base64 differs');return JSON.parse(bytes.toString('utf8'));
}

function executable(name) {
  for(const directory of (process.env.PATH || '').split(path.delimiter))try{const file=fs.realpathSync(path.join(directory,name)),stat=fs.statSync(file);if(stat.isFile() && stat.mode&0o111)return file;}catch{}
  throw new Error(`Required maintenance executable is unavailable: ${name}`);
}
export function verifySourceLessHarness(directory=root,{profile,sourceInputs}={}) {
  directory=ordinaryDirectory(directory);const files=[],seen=new Set(),allowed=new Set(P3_HARNESS_MEMBERS);let migrationFixture;
  const visit=parent=>{for(const name of fs.readdirSync(parent)){
    const file=path.join(parent,name),member=path.relative(directory,file).split(path.sep).join('/'),stat=fs.lstatSync(file);
    requireThat(!['.git','node_modules','target','upstream','renderers'].includes(name) && !/\.rs$|(?:^|\/)Cargo\.(?:toml|lock)$/.test(member),'Consumer harness contains Rust, renderer or source archives');
    if(stat.isDirectory() && !stat.isSymbolicLink()){visit(file);continue;}
    requireThat(stat.isFile() && !stat.isSymbolicLink(),'Consumer harness contains a nonordinary file');
    if(profile==='p3')requireThat(allowed.has(member),'Unknown file outside the fixed P3 harness allowlist');
    const snapshot=readSnapshot(file);seen.add(member);files.push(bound(snapshot));
    if(member===MIGRATION_FIXTURE_MEMBER){
      requireThat(same(snapshot.identity,MIGRATION_FIXTURE_IDENTITY),'Fixed inert migration fixture bytes differ');
      if(profile==='p3')readMigrationFixture(snapshot.content,{sourceInputs});
      migrationFixture={file:bound(snapshot),identity:snapshot.identity,manifestBound:profile==='p3',execution:'never; isolated old target data only'};
    }else requireThat(!/\.(?:crate|tgz|tar|gz|zip|xz|bz2)$/i.test(name),'Consumer harness contains a source archive');
  }};
  visit(directory);
  if(profile==='p3')requireThat(same(sorted(seen),sorted(allowed)) && migrationFixture?.manifestBound===true,'P3 harness is missing a required driver or its candidate-bound inert dataset');
  return{status:'pass',scope:'No checkout/Rust, Cargo, target, upstream archives or active Node renderer; the sole fixed inert migration dataset is never executed',files,...(migrationFixture?{migrationFixture}:{})};
}
export function makeEnvironment(runRoot,{npmCliPath}={}) {
  const npmTool=npmCliPath?inspectNpm(npmCliPath):undefined;
  const home=mkdir(path.join(runRoot,'home')),bin=mkdir(path.join(runRoot,'bin')),npmCache=mkdir(path.join(runRoot,'npm-cache')),engineHome=mkdir(path.join(runRoot,'engine-cache')),uiHome=mkdir(path.join(runRoot,'ui-cache'));
  const programs={node:fs.realpathSync(process.execPath),npm:npmTool?.cli.path || executable('npm'),npx:npmTool?.npx.path || executable('npx')};
  for(const name of ['git','sh','bash','env','dirname','readlink','uname','grep','sed','cat','mkdir','rm','cut','tr'])try{programs[name]=executable(name);}catch{}
  for(const [name,file] of Object.entries(programs))fs.symlinkSync(file,path.join(bin,name));
  requireThat(!fs.existsSync(path.join(bin,'cargo')) && !fs.existsSync(path.join(bin,'rustc')),'Rust executables must not enter the consumer PATH');
  const userConfig=path.join(home,'user.npmrc'),globalConfig=path.join(home,'global.npmrc');writeNew(userConfig,Buffer.alloc(0));writeNew(globalConfig,Buffer.alloc(0));
  // Chrome's Unix sockets need a short parent even when evidence paths are deep.
  // Browsers still create their own private temporary directories below it.
  const env={HOME:home,USER:os.userInfo().username,LOGNAME:os.userInfo().username,PATH:bin,TMPDIR:fs.realpathSync('/tmp'),LANG:process.env.LANG || 'en_US.UTF-8',LC_ALL:process.env.LC_ALL || 'en_US.UTF-8',PHOENIX_DIAGRAM_HOME:engineHome,PHOENIX_HOME:uiHome,npm_config_cache:npmCache,npm_config_userconfig:userConfig,npm_config_globalconfig:globalConfig,npm_config_update_notifier:'false',npm_config_audit:'false',npm_config_fund:'false',npm_config_ignore_scripts:'true',npm_config_logs_max:'10',NO_PROXY:'127.0.0.1,localhost'};
  const browserPath=process.env.PHOENIX_DIAGRAM_BROWSER;if(browserPath)env.PHOENIX_DIAGRAM_BROWSER=path.resolve(browserPath);
  const npmCli=npmTool?.cli.path || fs.realpathSync(path.resolve(path.dirname(process.execPath),'../lib/node_modules/npm/bin/npm-cli.js'));readSnapshot(npmCli);
  const proof=execText(programs.node,['-e',`const f=require('node:fs'),p=require('node:path');for(const n of ['rustc','cargo'])for(const d of process.env.PATH.split(p.delimiter))if(f.existsSync(p.join(d,n)))process.exit(1);process.stdout.write(JSON.stringify({rustc:'unresolvable',cargo:'unresolvable',path:process.env.PATH,node:process.version}));`],{env});
  return{env,nodePath:programs.node,npmCliPath:npmCli,npmCli:bound(readSnapshot(npmCli)),...(npmTool?{npmTool}:{}),npmCache,proof:JSON.parse(proof),programs:Object.fromEntries(Object.entries(programs).map(([name,file])=>[name,bound(readSnapshot(file,{allowMultipleLinks:true}))])),sandbox:{status:'not-applicable',prefix:[],reason:'Consumption has no source checkout; Chrome retains its own sandbox'},browserPath};
}
export function verifyNpmSelection(environment,selectedPath) {
  requireThat(!selectedPath || selectedPath===environment.npmCliPath,'The selected npm CLI differs from the original transport');
  requireThat(environment.npmCli.path===environment.npmCliPath,'Original npm CLI binding/path differs');readBound(environment.npmCli,'same actual npm CLI');
  if(environment.npmTool) {readBound(environment.npmTool.archive,'same pinned npm archive');requireThat(same(inspectNpm(environment.npmCliPath),environment.npmTool),'Prepared npm tool changed');}
}
async function withEnvironment(env,operation) {const original={...process.env};for(const name of Object.keys(process.env))delete process.env[name];Object.assign(process.env,env);try{return await operation();}finally{for(const name of Object.keys(process.env))delete process.env[name];Object.assign(process.env,original);}}
async function runPublic(adapter,args,{prefix=adapter.sandboxPrefix || [],timeoutMs=600000,maintenanceProbe=false}={}) {
  const command=publicNpmCommand(adapter,args);if(maintenanceProbe)command.args.splice(command.args.length-1,1,'node','-e',NPM_ENV_PROBE);
  const program=prefix[0] || command.program,argv=prefix.length?[...prefix.slice(1),command.program,...command.args]:command.args;
  return await new Promise((resolve,reject)=>{
    const child=spawn(program,argv,{cwd:command.cwd,env:command.env,stdio:['ignore','pipe','pipe'],detached:true});let stdout=[],stderr=[],length=0,failure;
    const capture=chunks=>data=>{length+=data.length;if(length>64*1024*1024){failure=new Error('Consumer command output exceeded its bound');try{process.kill(-child.pid,'SIGKILL');}catch{}}else chunks.push(data);};child.stdout.on('data',capture(stdout));child.stderr.on('data',capture(stderr));child.on('error',error=>{failure=error;});
    const timer=setTimeout(()=>{failure=new Error('Owned public npm command timed out');try{process.kill(-child.pid,'SIGKILL');}catch{}},timeoutMs);
    child.on('close',(exitCode,signal)=>{clearTimeout(timer);if(failure)reject(failure);else resolve({exitCode:exitCode??4,signal,stdout:Buffer.concat(stdout).toString('utf8'),stderr:Buffer.concat(stderr).toString('utf8')});});
  });
}
function copyCandidate(candidate,runRoot) {
  const assets=mkdir(path.join(runRoot,'assets'));const copy=(name,snapshot)=>{const file=path.join(assets,name);fs.mkdirSync(path.dirname(file),{recursive:true,mode:0o700});writeNew(file,snapshot.content,snapshot.binding.mode);};
  copy('candidate.json',candidate.snapshot);for(const [name,snapshot] of candidate.files)copy(name,snapshot);
  const checked=verifyCandidateInventory(assets);requireThat(same(checked.snapshot.identity,candidate.snapshot.identity),'Owned candidate copy changed');return checked;
}
export async function startCandidateServer(candidate,{port=DEFAULT_PORT}={}) {
  requireThat(port===DEFAULT_PORT,'The candidate HTTP port is frozen');const requests=[],fault={enabled:false};
  const server=http.createServer((request,response)=>{
    const target=TARGETS.find(name=>request.url===`/phoenix-diagram-${name}`),snapshot=target && candidate.files.get(`release-dir/phoenix-diagram-${target}`);
    if(request.method!=='GET' || !snapshot){response.writeHead(404,{Connection:'close'});response.end();return;}
    try{const current=readBound(bound(snapshot),'served engine bytes'),bytes=fault.enabled?Buffer.from('controlled corrupt engine response'):current.content;requests.push({target,url:request.url,method:request.method,bytes:bytes.length,sha256:identity(bytes).sha256,fault:fault.enabled});response.writeHead(200,{'Content-Type':'application/octet-stream','Content-Length':bytes.length,Connection:'close'});response.end(bytes);}catch(error){response.writeHead(500,{Connection:'close'});response.end(error.message);}
  });
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(port,'127.0.0.1',resolve);});return{requests,fault,close:()=>new Promise((resolve,reject)=>server.close(error=>error?reject(error):resolve()))};
}
function packageRootIn(cache) {
  const npx=path.join(cache,'_npx'),found=[];if(fs.existsSync(npx))for(const name of fs.readdirSync(npx)){const file=path.join(npx,name,'node_modules/@xgent-ai/skills');if(fs.existsSync(file))found.push(ordinaryDirectory(file));}
  requireThat(found.length===1,'The isolated npm cache must resolve exactly one candidate package');return found[0];
}
function verifyPackage(directory,members) {
  const files=new Map();const visit=(parent,prefix='')=>{ordinaryDirectory(parent);for(const name of fs.readdirSync(parent)){const member=prefix?`${prefix}/${name}`:name,file=path.join(parent,name),stat=fs.lstatSync(file);if(stat.isDirectory() && !stat.isSymbolicLink())visit(file,member);else{const snapshot=readSnapshot(file),expected=members.get(member);requireThat(expected && expected.sha256===snapshot.identity.sha256 && expected.size===snapshot.identity.bytes,'Installed npm member differs from the unique tarball');files.set(member,bound(snapshot));}}};visit(directory);requireThat(same(sorted(files.keys()),sorted(members.keys())),'Installed npm package has omitted or extra files');return Object.fromEntries(files);
}
function networkPrefix() {if(process.platform==='darwin')return['/usr/bin/sandbox-exec','-p','(version 1) (allow default) (deny network-outbound (remote ip)) (deny network-inbound (local ip))'];if(process.platform==='linux')return['/usr/bin/unshare','--user','--map-current-user','--net','--'];throw new Error('No OS network-denial launcher is configured');}
export function caseManifest(packageRoot,candidate,options={}) {
  const selected=selectLegacySamples(candidate.members,candidate.sourceInputs,options);
  return{cases:TYPES.flatMap(type=>contractsFor(type).map(contract=>contract==='phoenix-v1'?{id:`new-${type}`,type,contract,source:path.join(packageRoot,'skills/phoenix-diagram/examples',`minimal.${type}.json`)}:{id:`legacy-${type}`,type,contract,source:path.join(packageRoot,selected.paths[type])}))};
}

const ENVIRONMENT_KEYS=new Set(['HOME','USER','LOGNAME','PATH','TMPDIR','LANG','LC_ALL','PHOENIX_DIAGRAM_HOME','PHOENIX_HOME','PHOENIX_DIAGRAM_BROWSER','npm_config_cache','npm_config_userconfig','npm_config_globalconfig','npm_config_update_notifier','npm_config_audit','npm_config_fund','npm_config_ignore_scripts','npm_config_logs_max','NO_PROXY']);
export function checkedEnvironment(env) {requireThat(env && Object.keys(env).every(key=>ENVIRONMENT_KEYS.has(key)) && typeof env.PATH==='string' && !Object.keys(env).some(key=>/token|auth|credential/i.test(key)),'Consumer state contains an unapproved environment key');return env;}
function adapterFor(environment,tarball,project) {return{schemaVersion:1,kind:'npm-public-bin',nodePath:environment.nodePath,npmCliPath:environment.npmCliPath,tarball,cwd:project,env:checkedEnvironment(environment.env),bin:'phoenix-diagram'};}
function engineCachePath(environment,version,target) {return path.join(environment.env.PHOENIX_DIAGRAM_HOME,'bin',version.toolVersion,target,version.engines[target].sha256,'phoenix-diagram');}
function processEvidence(directory,label,result) {return writeNew(path.join(directory,`${label}.json`),result);}
async function installProject(adapter,project,releaseDir,prefix=[]) {
  return await runPublic({...adapter,bin:'xgent-skills',cwd:project},['install',project,'--no-phoenix-ui','--no-xgent-init',...(releaseDir?[`--diagram-release-dir=${releaseDir}`]:[])],{prefix});
}
async function negativeResources(state,candidate,environment,adapter,server) {
  const directory=mkdir(path.join(state.runRoot,'rejections')),checks=[];
  for(const kind of ['missing-release','corrupt-http']) {
    const project=mkdir(path.join(directory,`${kind}-project`)),cache=mkdir(path.join(directory,`${kind}-engine-cache`)),negative={...adapter,cwd:project,env:{...adapter.env,PHOENIX_DIAGRAM_HOME:cache}};
    let result;try{server.fault.enabled=kind==='corrupt-http';result=await installProject(negative,project,kind==='missing-release'?path.join(directory,'missing-release'):undefined);}finally{server.fault.enabled=false;}
    requireThat(result.exitCode!==0 && fs.readdirSync(project).length===0,`${kind} preflight wrote to the project or accepted a bad resource`);checks.push({name:kind,status:'pass',projectUnchanged:true,receipt:processEvidence(directory,kind,result)});
  }
  const badCache=mkdir(path.join(directory,'bad-cache')),badEnvironment={...environment,env:{...environment.env,PHOENIX_DIAGRAM_HOME:badCache}},badPath=engineCachePath(badEnvironment,candidate.version,state.target);
  fs.mkdirSync(path.dirname(badPath),{recursive:true,mode:0o700});writeNew(badPath,Buffer.concat([candidate.engine.content,Buffer.from('controlled cache corruption')]),0o700);
  const active=bound(readSnapshot(path.join(adapter.cwd,'.phoenix-diagram/install/active.json'))),bad=await runPublic({...adapter,env:badEnvironment.env},['doctor','--json']);
  requireThat(bad.exitCode!==0 && /digest|binary|VERSION/.test(bad.stdout+bad.stderr),'A changed cached engine was accepted');readBound(active,'unchanged native installation after bad cache');checks.push({name:'cached-engine-bytes',status:'pass',receipt:processEvidence(directory,'bad-cache',bad)});
  const bootstrap=path.join(state.packageRoot,'skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs');
  const unsupported=spawnSync(environment.nodePath,['-e',`try{require(process.argv[1]).platformTarget('win32','x64');process.exit(1)}catch(e){process.stdout.write(e.message)}`,bootstrap],{cwd:adapter.cwd,env:adapter.env,encoding:'utf8'});
  requireThat(!unsupported.error && unsupported.status===0 && /unsupported|support/i.test(unsupported.stdout),'Packaged unsupported-platform guard did not reject');checks.push({name:'unsupported-platform',status:'pass',scope:'Actual packaged pure bootstrap guard; no unsupported-host native execution is claimed',receipt:processEvidence(directory,'unsupported-platform',{exitCode:unsupported.status,stdout:unsupported.stdout,stderr:unsupported.stderr})});
  return{status:'pass',checks};
}
function context(candidate,target,profile,historical) {
  const repository=process.env.GITHUB_REPOSITORY || null,runId=process.env.GITHUB_RUN_ID || null;
  if(repository)requireThat(/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository),'Invalid workflow repository');if(runId)requireThat(/^\d+$/.test(runId),'Invalid workflow run ID');
  if(process.env.GITHUB_ACTIONS==='true')requireThat(HASH.test(process.env.PHOENIX_DIAGRAM_TGZ_SHA256 || '') && HASH.test(process.env.PHOENIX_DIAGRAM_CANDIDATE_SHA256 || '') && /^[a-f0-9]{40}$/.test(process.env.PHOENIX_DIAGRAM_SOURCE_COMMIT || '') && HASH.test(process.env.PHOENIX_DIAGRAM_HARNESS_SHA256 || ''),'Workflow transfer identities are required');
  return{...hostHeader(),...(profile?{profile}:{}),target,repository,runId,runAttempt:process.env.GITHUB_RUN_ATTEMPT || '1',job:process.env.GITHUB_JOB || null,nonce:randomBytes(16).toString('hex'),startedAt:new Date().toISOString(),source:candidate.source,candidate:{record:bound(candidate.snapshot),tarball:bound(candidate.tgz),candidateArchiveSha256:historical?historical.candidateArchive.sha256:process.env.PHOENIX_DIAGRAM_CANDIDATE_SHA256 || null,artifactId:historical?String(historical.artifact.id):process.env.PHOENIX_DIAGRAM_ARTIFACT_ID || null,artifactDigest:historical?historical.artifact.digest.slice('sha256:'.length):process.env.PHOENIX_DIAGRAM_ARTIFACT_DIGEST || null},harnessArchiveSha256:process.env.PHOENIX_DIAGRAM_HARNESS_SHA256 || null,sourceVerification:candidate.sourceVerification,runtimeNode:process.version,phase:'starting',automatedStatus:'not-run',platformHandoff:'incomplete',handoff:'pending-review',candidateHandoff:'pending-all-platforms-and-provider',diagnostics:[]};
}
function reviewRequest(state,stateFile) {
  const records=Object.fromEntries(state.cases.map(entry=>[entry.id,{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command:'review-record',status:'pass',diagnostics:[],artifact:{path:entry.state.artifact.path,identity:entry.state.artifact.identity},candidate:{path:entry.state.candidate.path,identity:entry.state.candidate.identity},configSha256:entry.state.configSha256,engine:entry.state.engine,deliveryReceiptId:entry.state.deliveryReceiptId,browserReceiptIdentity:entry.state.browserReceiptIdentity,screenshotIdentities:entry.state.screenshots.map(file=>({file:{path:file.path,identity:file.identity},viewport:file.viewport,theme:file.theme,observed:false,observations:''})),reviewer:'',scope:['diagram','labels','cards','relationship-occlusion','hierarchy','controls'],verdict:'unavailable',findings:['Actual image review has not been performed.']}]))
  return{schemaVersion:1,kind:'phoenix-diagram-review-request',state:stateFile,reviewRef:state.runId?`phoenix-diagram-review/${state.runId}`:null,reviewPath:`reviews/${state.target}.json`,instructions:'Read every actual current screenshot. Fill reviewer/observed/observations/verdict/findings; retain all identities and paths. This template is not an accepted visual review.',envelope:{schemaVersion:1,kind:REVIEW_KIND,repository:state.repository,runId:state.runId,target:state.target,nonce:state.nonce,stateSha256:stateFile.identity.sha256,candidateSha256:state.candidate.tarball.identity.sha256,sourceCommit:state.source.commit,records}};
}
function githubOutputs(stateFile,runRoot,outputFile=process.env.GITHUB_OUTPUT) {
  if(!outputFile)return;for(const [key,value] of [['evidence_dir',runRoot],['state_path',stateFile.path]]){const delimiter=`PHOENIX_${randomBytes(12).toString('hex')}`;fs.appendFileSync(outputFile,`${key}<<${delimiter}\n${value}\n${delimiter}\n`);}
}
export function persistState(state,{initial=false,outputFile=process.env.GITHUB_OUTPUT}={}) {
  const stateFile=writeNew(path.join(state.runRoot,initial?'consumer-state.json':`consumer-state-${randomBytes(8).toString('hex')}.json`),state);
  if(state.kind===KIND){
    if(initial && state.phase==='pending-review')writeNew(path.join(state.runRoot,'review-request.json'),reviewRequest(state,stateFile));
    githubOutputs(stateFile,state.runRoot,outputFile);
  }
  return{state,stateFile};
}
export async function initialConsume(options) {return consumeCandidate(options);}
// Explicit old-package precondition: installation evidence only, never a core pass.
export async function prepareCandidateTransport(options) {return consumeCandidate(options,{installationOnly:true});}
async function consumeCandidate(options,{installationOnly=false}={}) {
  requireThat(!options.legacyDescriptor && !options.harnessCandidateDir || installationOnly && options.legacyDescriptor && options.harnessCandidateDir,'Historical installation needs its descriptor and the current P3 harness owner');
  const candidate=validateCandidate(options.candidateDir,options.target,{profile:options.profile,legacyDescriptor:options.legacyDescriptor});
  const harnessCandidate=options.harnessCandidateDir?validateCandidate(options.harnessCandidateDir,options.target,{profile:'p3'}):candidate;
  const harness=verifySourceLessHarness(root,{profile:options.harnessCandidateDir?'p3':options.profile,sourceInputs:harnessCandidate.sourceInputs.entries});
  const historical=options.legacyDescriptor?verifyCandidateCohort(candidate,{legacyDescriptor:readBound(options.legacyDescriptor,'fixed historical v1 descriptor').content}):undefined,state=context(candidate,options.target,options.profile,historical);
  if(options.harnessCandidateDir)state.harnessSource=harnessCandidate.source;
  if(installationOnly){state.kind='phoenix-diagram-candidate-install-context';state.scope='installation-only';}
  const directory=options.evidenceDir?mkdir(path.resolve(options.evidenceDir)):fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'phoenix-diagram-consumer-')));fs.chmodSync(directory,0o700);state.runRoot=directory;const directoryStat=fs.statSync(directory);state.directoryBinding={device:String(directoryStat.dev),inode:String(directoryStat.ino),uid:directoryStat.uid};state.harness=harness;let server;
  try {
    const owned=copyCandidate(candidate,directory),environment=makeEnvironment(directory,{npmCliPath:options.npmCliPath});state.environment=environment;state.candidate.ownedRecord=bound(owned.snapshot);state.candidate.ownedTarball=bound(checkedFile(owned,candidate.record.tarball.path));
    const onlineProject=mkdir(path.join(directory,'online-project')),adapter=adapterFor(environment,state.candidate.ownedTarball.path,onlineProject);
    const npmEnvironment=await runPublic(adapter,[],{maintenanceProbe:true});requireThat(npmEnvironment.exitCode===0,'npm exec exposes Rust through its effective child PATH');environment.npmChildProof=JSON.parse(npmEnvironment.stdout);state.npmEnvironment=processEvidence(directory,'npm-child-environment',npmEnvironment);
    const uiDirectory=path.join(onlineProject,'.phoenix-ui/install');fs.mkdirSync(uiDirectory,{recursive:true,mode:0o700});const uiClaimant=writeNew(path.join(uiDirectory,'active.json'),{kind:'unclaimed-preservation-probe',note:'This is not a successful UI installation receipt.'});
    server=await startCandidateServer(owned);const installed=await installProject(adapter,onlineProject);state.online={status:'failed',receipt:processEvidence(directory,'online-install',installed)};requireThat(installed.exitCode===0,'Public npm online installation failed');
    state.packageRoot=packageRootIn(environment.npmCache);state.packageFiles=verifyPackage(state.packageRoot,candidate.members);readBound(uiClaimant,'preserved UI state claimant');
    requireThat(server.requests.some(request=>!request.fault && request.target===options.target && request.sha256===candidate.engine.identity.sha256 && request.bytes===candidate.engine.identity.bytes),'No actual fixed-port HTTP transfer supplied the expected native engine');state.online.status='pass';
    state.rejections=await negativeResources(state,candidate,environment,adapter,server);state.online.httpRequests=server.requests;await server.close();server=undefined;state.online.serverClosed=true;
    const offlineProject=mkdir(path.join(directory,'offline-project')),offlineHome=mkdir(path.join(directory,'offline-engine-cache')),offlineEnvironment={...environment,env:{...environment.env,PHOENIX_DIAGRAM_HOME:offlineHome}},offlineAdapter=adapterFor(offlineEnvironment,state.candidate.ownedTarball.path,offlineProject);
    const offline=await installProject(offlineAdapter,offlineProject,path.join(owned.directory,'release-dir'),networkPrefix());state.offlineInstall={status:'failed',receipt:processEvidence(directory,'offline-install',offline),network:{method:process.platform==='darwin'?'sandbox-exec':'network-namespace',prefix:networkPrefix()}};requireThat(offline.exitCode===0,'Public npm offline installation under OS network denial failed');state.offlineInstall.status='pass';
    state.environment=offlineEnvironment;state.project=offlineProject;state.engine=bound(readSnapshot(engineCachePath(offlineEnvironment,candidate.version,options.target)));requireThat(same(state.engine.identity,candidate.engine.identity),'Installed native bytes differ from the unique candidate target');offlineAdapter.enginePath=state.engine.path;
    const doctor=await invokeNative(state.engine,['doctor','--json'],{options:{publicNpm:offlineAdapter,browserPath:environment.browserPath}});requireThat(doctor.exitCode===0 && doctor.receipt.engine.sourceInputSha256===candidate.source.inputSha256 && same(doctor.receipt.engine.binary,state.engine.identity),'Public npm doctor does not bind the candidate source/native engine');state.doctor=processEvidence(directory,'public-doctor',doctor);
    state.publicNpm=offlineAdapter;
    if(installationOnly){state.engine={...state.engine,engineIdentity:doctor.receipt.engine};verifyNpmSelection(environment,options.npmCliPath);verifyPackage(state.packageRoot,candidate.members);verifyCandidateInventory(owned.directory);state.phase='installed';state.status='pass';}
    else {
      const manifest=writeNew(path.join(directory,'cases.json'),caseManifest(state.packageRoot,candidate,{profile:options.profile}));state.caseManifest=manifest;state.publicNpm=offlineAdapter;
      state.scopes=CONSUMER_SCOPES.filter(name=>options[name]);
      const scopes=Object.fromEntries(state.scopes.map(name=>[name,true]));
      const core=await withEnvironment(offlineEnvironment.env,()=>runCoreStage({stage:'core',engine:state.engine.path,outputDir:path.join(directory,'core'),caseManifest:manifest.path,publicNpm:offlineAdapter,browserPath:environment.browserPath,reader:true,recovery:true,offline:true,...scopes,requiredScopes:state.scopes,evidenceRepository:options.evidenceRepository}));
      state.coreIndex=core.indexFile;state.cases=core.cases;state.engine={...state.engine,engineIdentity:core.engine.engineIdentity};
      requireThat(core.automatedStatus==='pass' && core.status!=='failed' && core.coverage.complete && core.cases.every(entry=>entry.reader.status==='pass' && entry.determinism.status==='pass') && ['content','recovery','offline',...state.scopes].every(key=>core[key]?.status==='pass'),'Automatic candidate consumption has failed or unexecuted scopes');
      verifyNpmSelection(environment,options.npmCliPath);verifyPackage(state.packageRoot,candidate.members);verifyCandidateInventory(owned.directory);requireThat(fs.readdirSync(environment.env.PHOENIX_HOME).length===0,'Diagram consumption changed the independent UI cache');readBound(uiClaimant,'unchanged UI state claimant');state.isolation={status:'pass',uiClaimant,uiCacheUnchanged:true,scope:'UI claimant bytes/cache preserved by explicit UI skip; diagram ownership verified by the public bootstrap. Full joint UI installation remains a separate gate.'};state.automatedStatus='pass';state.phase='pending-review';state.status='automated-pass';
    }
  } catch(error) {state.status='failed';state.phase='failed';state.diagnostics.push(errorRecord(error));}
  finally {if(server)await server.close();}
  return persistState(state,{initial:true});
}

export async function pollForReview(state,stateIdentity,{fetchReview,timeoutMs=WAIT_MS,pollMs=20000}={}) {
  requireThat(Number.isSafeInteger(timeoutMs) && timeoutMs>0 && timeoutMs<=WAIT_MS,'Review wait cannot exceed 90 minutes');requireThat(state.repository && /^\d+$/.test(state.runId || ''),'Live job review requires the original workflow repository/run ID');const deadline=Date.now()+timeoutMs;let last='No matching review file';
  while(Date.now()<deadline) {
    try{const response=await fetchReview({remainingMs:Math.max(1,deadline-Date.now())});if(response){const value=decodeGitHubReview(response,state.target);validateReviewEnvelope(value,state,stateIdentity);return value;}}catch(error){last=error.message;}
    await delay(Math.min(pollMs,Math.max(1,deadline-Date.now())));
  }
  throw new Error(`Actual review remains pending after the bounded wait: ${last}`);
}
export function readCandidateTransport(options,candidate) {return readConsumerContext(options,candidate,{installationOnly:true});}
export function readConsumerContext(options,candidate,{installationOnly=false}={}) {
  const file=readSnapshot(options.resume),state=JSON.parse(file.content.toString('utf8'));requireThat(state.schemaVersion===1 && state.target===options.target && (installationOnly?state.kind==='phoenix-diagram-candidate-install-context' && state.scope==='installation-only' && state.phase==='installed' && state.status==='pass' && state.automatedStatus==='not-run':state.kind===KIND && state.automatedStatus==='pass' && ['pending-review','complete','failed-review'].includes(state.phase)),'State does not prove the required consumption/installation scope for this native target');
  requireThat((state.profile??null)===(options.profile??null),'Resume profile differs from the original consumption');
  if(options.legacyDescriptor){
    requireThat(installationOnly && options.harnessCandidateDir,'Historical resume needs its current P3 harness owner');
    const historical=verifyCandidateCohort(candidate,{legacyDescriptor:readBound(options.legacyDescriptor,'fixed historical v1 descriptor').content}),owner=validateCandidate(options.harnessCandidateDir,options.target,{profile:'p3'});
    requireThat(same(state.harnessSource,owner.source) && state.harness.migrationFixture?.manifestBound===true,'Historical transport harness belongs to another current candidate');
    requireThat(state.candidate.candidateArchiveSha256===historical.candidateArchive.sha256 && String(state.candidate.artifactId)===String(historical.artifact.id) && state.candidate.artifactDigest===historical.artifact.digest.slice('sha256:'.length),'Historical transport archive provenance differs');
  }
  const directory=ordinaryDirectory(state.runRoot),stat=fs.statSync(directory);requireThat(path.dirname(file.path)===directory && stat.uid===process.getuid() && (stat.mode&0o7777)===0o700 && same(state.directoryBinding,{device:String(stat.dev),inode:String(stat.ino),uid:stat.uid}),'Resume requires its original owned 0700 live-job directory');
  requireThat(state.source.commit===candidate.source.commit && same(state.source,candidate.source) && same(state.candidate.tarball.identity,candidate.tgz.identity) && same(state.candidate.record.identity,candidate.snapshot.identity),'Resume candidate/source bytes differ');
  checkedEnvironment(state.environment.env);requireThat(state.runtimeNode===process.version && same(state.publicNpm.env,state.environment.env) && state.publicNpm.bin==='phoenix-diagram' && state.publicNpm.nodePath===state.environment.nodePath && state.publicNpm.npmCliPath===state.environment.npmCliPath && state.publicNpm.tarball===state.candidate.ownedTarball.path && state.publicNpm.cwd===state.project && state.publicNpm.enginePath===state.engine.path && state.engine.path===engineCachePath(state.environment,candidate.version,state.target),'Public npm resume transport changed');
  requireThat(state.environment.env.PATH===path.join(directory,'bin') && same(sorted(fs.readdirSync(state.environment.env.PATH)),sorted(Object.keys(state.environment.programs))),'Rust-free PATH inventory changed');
  for(const [name,reference] of Object.entries(state.environment.programs)) {const current=readSnapshot(reference.path,{allowMultipleLinks:true});requireThat(same(current.identity,reference.identity) && same(current.binding,reference.binding) && fs.realpathSync(path.join(state.environment.env.PATH,name))===reference.path,'A whitelisted executable changed');}verifyNpmSelection(state.environment,options.npmCliPath);
  readBound(state.candidate.ownedRecord,'owned candidate record');readBound(state.candidate.ownedTarball,'owned unchanged tgz');readBound(state.engine,'same native engine');if(!installationOnly)readBound(state.caseManifest,'same package case manifest');verifyPackage(state.packageRoot,candidate.members);for(const file of state.harness.files)readBound(file,'same maintenance harness');
  if(installationOnly){const doctor=JSON.parse(readBound(state.doctor,'public doctor process').content);requireThat(doctor.receipt?.engine && same(doctor.receipt.engine,state.engine.engineIdentity),'Installation doctor engine identity changed');requireThat(doctor.exitCode===0 && doctor.signal===null && JSON.parse(doctor.stdout).engine && same(JSON.parse(doctor.stdout).engine,state.engine.engineIdentity),'Installation public doctor capture differs');return{state,file};}
  const core=JSON.parse(readBound(state.coreIndex,'core index').content.toString('utf8'));refreshEvidence(core);requireThat(same(core.engine.engineIdentity,state.engine.engineIdentity),'Core current engine identity changed');state.cases=core.cases;return{state,file,core};
}
async function githubReview(options,state,stateIdentity) {
  const gh=executable('gh'),endpoint=`repos/${state.repository}/contents/reviews/${state.target}.json?ref=${encodeURIComponent(options.reviewRef)}`;
  requireThat(process.env.GITHUB_TOKEN,'A read-only GITHUB_TOKEN is required for the dedicated review channel');const ghEnv={HOME:process.env.HOME,PATH:process.env.PATH,GH_TOKEN:process.env.GITHUB_TOKEN,GH_HOST:'github.com',GH_PROMPT_DISABLED:'1'};
  const fetchReview=async({remainingMs})=>{const result=spawnSync(gh,['api','--hostname','github.com','--method','GET','-H','Accept: application/vnd.github+json',endpoint],{encoding:'utf8',maxBuffer:4*MAX_REVIEW,timeout:Math.min(30000,remainingMs),env:ghEnv});if(result.error)throw result.error;if(result.status!==0){if(/404|Not Found/.test(result.stderr))return null;throw new Error(`Read-only GitHub review API failed: ${result.stderr}`);}return JSON.parse(result.stdout);};
  state.reviewPollBudgetMs=POLL_BUDGET_MS;return await pollForReview(state,stateIdentity,{fetchReview,timeoutMs:POLL_BUDGET_MS});
}
// The local channel carries the same bound envelope as the workflow review branch.
function localReview(file,state,stateIdentity) {
  const snapshot=readSnapshot(path.resolve(file));requireThat(snapshot.identity.bytes>0 && snapshot.identity.bytes<=MAX_REVIEW,'Local review file size is out of bounds');
  const value=JSON.parse(snapshot.content.toString('utf8'));validateReviewEnvelope(value,state,stateIdentity);return value;
}
export async function resumeConsume(options) {
  const candidate=validateCandidate(options.candidateDir,options.target,{profile:options.profile});verifySourceLessHarness(root,{profile:options.profile,sourceInputs:candidate.sourceInputs.entries});const {state,file}=readConsumerContext(options,candidate),origin=state.reviewOrigin || bound(file);state.reviewOrigin=origin;
  if(options.reviewFile)requireThat(state.repository===null && state.runId===null && !process.env.GITHUB_RUN_ID,'A local review file resumes only a local consumption outside workflow runs');
  else requireThat(state.repository===process.env.GITHUB_REPOSITORY && state.runId===process.env.GITHUB_RUN_ID && state.runAttempt===(process.env.GITHUB_RUN_ATTEMPT || '1') && state.job===(process.env.GITHUB_JOB || null) && options.reviewRef===`phoenix-diagram-review/${state.runId}`,'Resume must use this same repository/run/job attempt and dedicated review branch');
  try {
    const npmEnvironment=await runPublic(state.publicNpm,[],{maintenanceProbe:true});requireThat(npmEnvironment.exitCode===0,'Rust became resolvable in the public npm child environment');state.resumeNpmEnvironment=processEvidence(state.runRoot,`npm-child-environment-${randomBytes(8).toString('hex')}`,npmEnvironment);
    const envelope=options.reviewFile?localReview(options.reviewFile,state,origin.identity):await githubReview(options,state,origin.identity);const directory=mkdir(path.join(state.runRoot,`review-${randomBytes(8).toString('hex')}`)),records={};
    for(const [id,record] of Object.entries(envelope.records))records[id]=writeNew(path.join(directory,`${id}.json`),record).path;const mapping=writeNew(path.join(directory,'records.json'),{records});
    const core=await withEnvironment(state.environment.env,()=>runCoreStage({stage:'core',engine:state.engine.path,resume:state.coreIndex.path,reviewFile:mapping.path,publicNpm:state.publicNpm,browserPath:state.environment.browserPath}));
    state.coreIndex=core.indexFile;state.cases=core.cases;state.reviewEnvelope=writeNew(path.join(directory,'envelope.json'),envelope);state.phase=core.handoff==='passed'?'complete':'failed-review';state.status=core.handoff==='passed'?'pass':'failed';state.platformHandoff=core.handoff==='passed'?'passed':'incomplete';state.handoff='pending-aggregate';
    requireThat(core.handoff==='passed' && core.cases.every(entry=>entry.state.visualReview==='passed' && entry.state.deliveryStatus==='complete'),'Actual review did not complete every original delivery identity');verifyPackage(state.packageRoot,candidate.members);
  } catch(error) {state.phase='failed-review';state.status='failed';state.platformHandoff='incomplete';state.diagnostics.push(errorRecord(error));}
  return persistState(state);
}

export const HELP=`Usage: node scripts/check-phoenix-diagram-candidate.mjs --candidate-dir DIR --target PLATFORM [--evidence-dir NEW_DIR] [--npm-cli PATH] [--profile p1|p2|p3] [--explore] [--locale] [--evidence --evidence-repo GIT_CHECKOUT]\n       node scripts/check-phoenix-diagram-candidate.mjs --candidate-dir DIR --target PLATFORM --resume STATE --review-ref phoenix-diagram-review/RUN_ID\n       node scripts/check-phoenix-diagram-candidate.mjs --candidate-dir DIR --target PLATFORM --resume STATE --review-file ENVELOPE\n\nScope flags add Reader exploration/share cards, interface languages and repository evidence to the initial consumption; they become required for its handoff. --review-file resumes a local (non-workflow) consumption from the same bound review envelope.\nThe initial run uses the exact tgz/public npm bins in an isolated project/cache and Rust-free PATH.\n--npm-cli selects only the pinned tool produced by explicit --prepare-consumer; resume retains the original tool.\nConsumption requires the separately transferred source-less Node harness, canonical source-inputs and all four native build receipts.\nAutomatic success exits zero with handoff=pending-review. Read-only GitHub polling defaults to 85 minutes (hard ceiling 90), leaving time for actual review-record commands in the same live job/filesystem.\nReview JSON binds the unique npm tgz SHA (not the outer candidate archive), state SHA, source, run, target and nonce.\nNo publishing, remote writes, new checkout or automatic visual verdict. A platform result never replaces the four-platform/provider aggregate gate.\n`;
export async function main(argv=process.argv.slice(2)) {
  try{const options=parseOptions(argv);if(options.help){process.stdout.write(HELP);return 0;}const result=options.resume?await resumeConsume(options):await initialConsume(options),state=result.state;process.stdout.write(`${JSON.stringify({...hostHeader(),status:state.status,automatedStatus:state.automatedStatus,phase:state.phase,platformHandoff:state.platformHandoff,handoff:state.handoff,evidenceDir:state.runRoot,statePath:result.stateFile.path,stateIdentity:result.stateFile.identity,diagnostics:state.diagnostics})}\n`);return state.status==='failed'?1:0;}
  catch(error){process.stdout.write(`${JSON.stringify({...hostHeader(),status:'failed',handoff:'incomplete',diagnostics:[errorRecord(error)]})}\n`);return error.exitCode || 1;}
}
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url))process.exitCode=await main();
