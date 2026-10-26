#!/usr/bin/env node
// Maintenance-only stdout collection. This is not a product CLI or a renderer.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { isDeepStrictEqual as same } from 'node:util';
import { fileURLToPath } from 'node:url';

export const COLLECTOR_MEMBER='tools/phoenix-diagram/tests/support/capture-stdio.mjs';
export const HELP='Usage: node capture-stdio.mjs --program ABSOLUTE_INSTALLED_PD --cwd ABSOLUTE_PROJECT --stdout NEW_FILE --stderr NEW_FILE [--inspection-dir NEW_PROJECT_DIRECTORY] [--previous-inspection ABSOLUTE_INSPECTION_JSON] -- PD_ARGV...\n       node capture-stdio.mjs inspect --capture SEALED_CAPTURE_JSON --inspection-dir NEW_PROJECT_DIRECTORY\nMaintenance capture only. Output parents must be owned ordinary 0700 directories. Optional inspection uses the existing project stdio/inspection-chain directory. inspect rebuilds derived files without executing a program.\n';
const hash=bytes=>({sha256:createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length});
function need(value,message) {if(!value)throw new Error(`stdio capture: ${message}`);}
function physical(stat) {return{device:String(stat.dev),inode:String(stat.ino),links:Number(stat.nlink),mode:Number(stat.mode&0o7777n),uid:Number(stat.uid)};}
function snapshot(file) {
  const before=fs.lstatSync(file,{bigint:true});need(before.isFile() && !before.isSymbolicLink() && before.nlink===1n && Number(before.uid)===process.getuid(),'expected an owned single-link ordinary file');
  need(fs.realpathSync(file)===file,'file path must be canonical');
  const fd=fs.openSync(file,fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW|fs.constants.O_NONBLOCK);
  try {
    const opened=fs.fstatSync(fd,{bigint:true});need(same(physical(before),physical(opened)),'file changed while opening');
    const content=fs.readFileSync(fd),after=fs.fstatSync(fd,{bigint:true}),current=fs.lstatSync(file,{bigint:true});
    need(same(physical(opened),physical(after)) && same(physical(after),physical(current)) && opened.mtimeNs===after.mtimeNs && opened.ctimeNs===after.ctimeNs && after.size===BigInt(content.length),'file changed during capture');
    return{path:file,identity:hash(content),binding:physical(after),content};
  } finally {fs.closeSync(fd);}
}
function reference(value) {const {content,...file}=value;return file;}
function unchanged(value) {const current=snapshot(value.path);need(same(reference(value),reference(current)),'original input or capture file changed');return current;}
function directory(file,privateOnly=false) {
  const stat=fs.lstatSync(file,{bigint:true});need(stat.isDirectory() && !stat.isSymbolicLink() && Number(stat.uid)===process.getuid() && fs.realpathSync(file)===file && (!privateOnly || Number(stat.mode&0o7777n)===0o700),'directory must be canonical, ordinary and owned; capture parent requires 0700');const {links,...binding}=physical(stat);return binding;
}
export function parseCaptureArgs(argv) {
  const options={},keys={'--program':'program','--cwd':'cwd','--stdout':'stdout','--stderr':'stderr','--inspection-dir':'inspectionDir','--previous-inspection':'previousInspection'};let at=0;
  for(;at<argv.length && argv[at]!=='--';at+=2) {
    const key=keys[argv[at]],value=argv[at+1];need(key && !Object.hasOwn(options,key) && typeof value==='string' && path.isAbsolute(value) && path.normalize(value)===value && !/[\0\r\n]/.test(value),'expected unique absolute program/cwd/stdout/stderr paths');options[key]=value;
  }
  need(['program','cwd','stdout','stderr'].every(key=>options[key]) && (!options.previousInspection || options.inspectionDir) && argv[at]==='--' && argv.length>at+1,'four capture options and an explicit PD argv are required; previous inspection needs inspection-dir');
  options.argv=argv.slice(at+1);need(options.argv.every(value=>typeof value==='string' && !/[\0\r\n]/.test(value)),'invalid PD argv');return options;
}
export async function captureStdio(options) {
  const project=directory(options.cwd),program=snapshot(options.program),installed=snapshot(path.join(options.cwd,'.phoenix-diagram/install/active.json')),active=JSON.parse(installed.content),relative=path.relative(options.cwd,options.program);
  need(/^\.(?:agents|claude)\/skills\/phoenix-diagram\/scripts\/phoenix-diagram$/.test(relative) && (program.binding.mode&0o111)!==0,'program is not an installed PD launcher');
  const owner=active.ownership?.find(value=>value.path===relative);
  need(active.schemaVersion===1 && active.tool==='phoenix-diagram' && active.project?.device===project.device && active.project.inode===project.inode && owner && Object.entries({...program.binding,...program.identity}).every(([key,value])=>owner.identity[key]===value),'installed launcher ownership or original project binding differs');
  const self=snapshot(fileURLToPath(import.meta.url)),parents=new Map(),descriptors=[];
  for(const file of [options.stdout,options.stderr]){const parent=path.dirname(file);parents.set(parent,directory(parent,true));need(!fs.existsSync(file),'capture slot is already claimed');}
  need(options.stdout!==options.stderr,'stdout and stderr require distinct new files');
  let child,nativeResult=null;
  try {
    for(const file of [options.stdout,options.stderr])descriptors.push(fs.openSync(file,fs.constants.O_RDWR|fs.constants.O_CREAT|fs.constants.O_EXCL|fs.constants.O_NOFOLLOW,0o600));
    const created=descriptors.map(fd=>physical(fs.fstatSync(fd,{bigint:true})));
    const result=await new Promise((resolve,reject)=>{
      child=spawn(options.program,options.argv,{cwd:options.cwd,stdio:['ignore',...descriptors],detached:true});
      const kill=signal=>{if(child.exitCode===null && child.signalCode===null)try{process.kill(-child.pid,signal);}catch(error){if(error.code!=='ESRCH')reject(error);}};
      const interrupt=()=>kill('SIGTERM'),timer=setTimeout(()=>kill('SIGKILL'),300000),cleanup=()=>{clearTimeout(timer);process.removeListener('SIGTERM',interrupt);process.removeListener('SIGINT',interrupt);};process.once('SIGTERM',interrupt);process.once('SIGINT',interrupt);
      child.once('error',error=>{cleanup();reject(error);});child.once('close',(exitCode,signal)=>{cleanup();resolve({exitCode,signal});});
    });
    nativeResult=result;
    for(const fd of descriptors)fs.fsyncSync(fd);
    const files=[snapshot(options.stdout),snapshot(options.stderr)];need(files.every((file,index)=>same(file.binding,created[index])),'capture slot changed after native execution');
    for(const [parent,original] of parents)need(same(directory(parent,true),original),'capture parent changed');
    need(same(directory(options.cwd),project),'project directory changed');unchanged(program);unchanged(installed);unchanged(self);
    return{schemaVersion:1,kind:'phoenix-diagram-stdio-capture',tool:'phoenix-diagram',collector:reference(self),node:{path:fs.realpathSync(process.execPath),version:process.version},program:options.program,argv:options.argv,cwd:options.cwd,installedReceipt:reference(installed),launcher:reference(program),...result,stdout:reference(files[0]),stderr:reference(files[1])};
  } catch(error){error.nativeStarted=Boolean(child);error.nativeResult=nativeResult;throw error;}
  finally {for(const fd of descriptors)fs.closeSync(fd);}
}
const PAGE_LIMIT=8192;
const encode=value=>Buffer.from(JSON.stringify(value)+'\n');
function canonicalJson(value) {
  if(Array.isArray(value))return '['+value.map(canonicalJson).join(',')+']';
  if(value && typeof value==='object')return '{'+Object.entries(value).map(([key,child])=>JSON.stringify(key)+':'+canonicalJson(child)).join(',')+'}';
  return Object.is(value,-0)?'-0':JSON.stringify(value);
}
const inside=(parent,file)=>path.isAbsolute(file) && path.normalize(file)===file && file.startsWith(parent+path.sep);
function exists(file) {try{fs.lstatSync(file);return true;}catch(error){if(error.code==='ENOENT')return false;throw error;}}
function readBound(file) {need(file?.path && file.identity && file.binding,'missing file reference');const value=snapshot(file.path);need(same(reference(value),file),'file identity or binding differs');return value;}
function json(file) {return JSON.parse(file.content.toString('utf8'));}
function write(file,value) {
  const bytes=Buffer.isBuffer(value)?value:encode(value),fd=fs.openSync(file,fs.constants.O_WRONLY|fs.constants.O_CREAT|fs.constants.O_EXCL|fs.constants.O_NOFOLLOW,0o600);
  try{fs.writeFileSync(fd,bytes);fs.fsyncSync(fd);}finally{fs.closeSync(fd);}return reference(snapshot(file));
}
function removeOwned(file) {if(same(reference(snapshot(file.path)),file))fs.unlinkSync(file.path);}
function outsideChain(project,file) {const root=path.join(project,'stdio/inspection-chain');return file!==root && !inside(root,file);}
function checkNewDirectory(project,file) {need(inside(project,file) && outsideChain(project,file) && !exists(file),'inspection directory must be new and inside the original project, outside its reserved chain');directory(path.dirname(file),true);}
function checkChainSlots(root,sequence) {const names=fs.readdirSync(root);need(names.length===sequence+2 && names.every(name=>name==='.lock' || name==='latest.json' || /^capture-[1-9][0-9]*\.json$/.test(name) && Number(name.slice(8,-5))<=sequence),'inspection chain contains an uncommitted capture or head transaction; Root recovery is required');}
function locked(project,run) {
  const root=path.join(project,'stdio/inspection-chain'),binding=directory(root,true),lock=write(path.join(root,'.lock'),{pid:process.pid,project});
  const verify=()=>need(same(directory(root,true),binding),'inspection chain directory changed');
  return Promise.resolve().then(()=>run(root,verify)).finally(()=>{verify();removeOwned(lock);});
}
function head(root) {const file=path.join(root,'latest.json');return exists(file)?snapshot(file):null;}
function advance(root,old,value,verify) {
  verify();if(old)unchanged(old);else need(!exists(path.join(root,'latest.json')),'inspection head already exists');
  const temporary=write(path.join(root,`.latest-${process.pid}.json`),value);
  try{verify();if(old)unchanged(old);fs.renameSync(temporary.path,path.join(root,'latest.json'));}finally{if(exists(temporary.path))removeOwned(temporary);}
  return snapshot(path.join(root,'latest.json'));
}
function validateCapture(file) {
  const capture=json(file),chain=capture.inspectionChain;
  need(capture.kind==='phoenix-diagram-stdio-capture' && capture.schemaVersion===1 && chain && Number.isSafeInteger(chain.sequence) && chain.sequence>0,'not a sealed complete capture');
  need(file.path===path.join(capture.cwd,'stdio/inspection-chain',`capture-${chain.sequence}.json`) && same(directory(capture.cwd),chain.project),'capture project or sequence binding differs');
  need(chain.sequence===1?!chain.previousCapture && !chain.previousInspection:chain.previousCapture && chain.previousInspection,'capture predecessor is missing');
  for(const key of ['stdout','stderr','installedReceipt','launcher','collector'])readBound(capture[key]);
  need(same(capture.collector.identity,snapshot(fileURLToPath(import.meta.url)).identity),'collector identity differs from the sealed capture');
  const installed=json(readBound(capture.installedReceipt)),relative=path.relative(capture.cwd,capture.program),owner=installed.ownership?.find(value=>value.path===relative);
  need(capture.installedReceipt.path===path.join(capture.cwd,'.phoenix-diagram/install/active.json') && capture.launcher.path===capture.program && /^\.(?:agents|claude)\/skills\/phoenix-diagram\/scripts\/phoenix-diagram$/.test(relative) && owner && installed.project?.device===chain.project.device && installed.project.inode===chain.project.inode && Object.entries({...capture.launcher.binding,...capture.launcher.identity}).every(([key,value])=>owner.identity[key]===value),'sealed capture installation binding differs');
  return capture;
}
function splitText(text) {
  const chunks=[];let chunk='',cost=0;
  // Budget the encoded JSON string, not just the original UTF-8. This also
  // preserves code points when long strings contain emoji or escaped controls.
  for(const character of text){const size=Buffer.byteLength(JSON.stringify(character))-2;if(cost+size>4096){chunks.push(chunk);chunk='';cost=0;}chunk+=character;cost+=size;}
  if(chunk || !chunks.length)chunks.push(chunk);return chunks;
}
function pageChain(directory,prefix,text,pages) {
  const chunks=splitText(text),ranges=[];let offset=0,next=null;
  for(const chunk of chunks){const end=offset+Buffer.byteLength(chunk);ranges.push({start:offset,end});offset=end;}
  for(let i=chunks.length-1;i>=0;i--){const bytes=encode({schemaVersion:1,kind:'phoenix-diagram-inspection-page',sequence:i+1,range:ranges[i],content:chunks[i],next});need(bytes.length<=PAGE_LIMIT,'encoded page exceeds 8KiB');next=write(path.join(directory,`${prefix}-${String(i+1).padStart(5,'0')}.json`),bytes);pages.push(next);}
  return{firstPage:next,pages:chunks.length,identity:hash(Buffer.from(text))};
}
function readPages(stream,pages) {
  need(stream?.firstPage && Number.isSafeInteger(stream.pages) && stream.pages>0,'missing page stream');let next=stream.firstPage,sequence=0,offset=0,text='';const seen=new Set();
  while(next){need(!seen.has(next.path),'page chain cycle');seen.add(next.path);const file=readBound(next),page=json(file);need(file.identity.bytes<=PAGE_LIMIT && page.schemaVersion===1 && page.kind==='phoenix-diagram-inspection-page' && page.sequence===++sequence && page.range?.start===offset && typeof page.content==='string','page order, size or range differs');offset+=Buffer.byteLength(page.content);need(page.range.end===offset,'page content range differs');text+=page.content;pages.push(next);next=page.next;}
  need(sequence===stream.pages && same(hash(Buffer.from(text)),stream.identity),'page stream identity differs');return text;
}
function shareValues(document) {
  const values=new Map(),references=[],shared=new Map();
  const count=value=>{if(!value || typeof value!=='object')return;for(const [key,child] of Object.entries(value)){if(key==='measuredScene' || key==='source'){const bytes=canonicalJson(child),id=hash(Buffer.from(bytes)).sha256;const seen=values.get(id);need(!seen || seen.bytes===bytes,'shared value digest collision');values.set(id,{bytes,count:(seen?.count || 0)+1});}count(child);}};count(document);
  const copy=(value,location=[])=>{if(!value || typeof value!=='object')return value;const result=Array.isArray(value)?[]:{};for(const [key,child] of Object.entries(value)){const id=(key==='measuredScene' || key==='source')?hash(Buffer.from(canonicalJson(child))).sha256:null;if(id && values.get(id).count>1){shared.set(id,values.get(id).bytes);references.push({path:[...location,key],id});Object.defineProperty(result,key,{value:null,enumerable:true,writable:true,configurable:true});}else Object.defineProperty(result,key,{value:copy(child,[...location,key]),enumerable:true,writable:true,configurable:true});}return result;};
  return{document:copy(document),shared,references};
}
const qualityCommands=new Set(['validate','deliver','check','browser-check','visual-check','finalize']);
function progressFor(capture,captureFile,document,previous) {
  const command=capture.argv[0],gates=structuredClone(previous?.progress.gates || {}),observations=[];
  const progress={schemaVersion:1,kind:'phoenix-diagram-inspection-progress',sequence:capture.inspectionChain.sequence,capture:captureFile,previousInspection:capture.inspectionChain.previousInspection,previousProgress:previous?.entry.progress || null,command,category:qualityCommands.has(command)?'quality':'auxiliary',status:'derived',observations,gates,stopRequired:Boolean(previous?.progress.stopRequired)};
  if(!qualityCommands.has(command)){progress.status='not-applicable';return progress;}
  const valid=value=>value?.schemaVersion===1 && value.tool==='phoenix-diagram' && ['pass','failed','unavailable','not-run'].includes(value.status) && Array.isArray(value.diagnostics) && value.diagnostics.every(row=>row && ['error','warning'].includes(row.severity) && typeof row.stage==='string');
  if(!valid(document) || document.command!==command){progress.status='not-derivable';progress.reason='No verifiable DTO for the executed quality command';return progress;}
  const observe=(gate,status,receipt)=>{
    const errors=receipt?.diagnostics?.filter(value=>value.severity==='error'),stages=[...new Set((errors || []).map(value=>value.stage))];
    const phase=gate==='validate'?(status==='pass'?'layout':stages.includes('input')?'input':stages.includes('layout')?'layout':stages.includes('artifact')?'artifact':'unknown'):stages.join(',') || gate;
    const row={gate,phase,status,errors:null,best:null,stall:null};observations.push(row);if(status==='not-run')return;
    let count=null;
    if(valid(receipt) && (receipt.command===gate || gate==='browser-check' && receipt.command==='visual-check') && receipt.status===status){if(status==='pass')count=0;else{const summary=receipt.composition?.summary?.errors;count=Number.isSafeInteger(summary) && summary>=0?summary:errors.length;if(count===0)count=null;}}
    if(count===null || phase==='unknown'){progress.status='not-derivable';return;}
    const key=gate==='validate'?`${gate}/${phase==='input'?'input':'layout'}`:gate,prior=gates[key],failures=(prior?.failures || 0)+(status==='pass'?0:1);
    const best=status==='pass'?0:!prior?.failures?count:Math.min(prior.best,count),stall=status==='pass'?0:!prior?.failures || count<prior.best?0:prior.stall+1;
    Object.assign(row,{errors:count,best,stall});gates[key]={best,stall,failures};if(stall>=2)progress.stopRequired=true;
  };
  if(command==='finalize'){
    const expected=['validate','deliver','check','browser-check'];if(!Array.isArray(document.stages) || !same(document.stages.map(value=>value.name),expected)){progress.status='not-derivable';return progress;}
    for(const stage of document.stages)observe(stage.name,stage.status,stage.receipt);
    if(document.status==='pass'?!document.stages.every(stage=>stage.status==='pass'):['failed','unavailable'].includes(document.status) && !document.stages.some(stage=>['failed','unavailable'].includes(stage.status)))progress.status='not-derivable';
  }else observe(command==='visual-check'?'browser-check':command,document.status,document);
  return progress;
}
function derive(captureFile,directoryName) {
  const capture=validateCapture(readBound(captureFile)),previous=capture.inspectionChain.previousInspection?readInspection(capture.inspectionChain.previousInspection.path):null;
  if(previous)need(same(reference(snapshot(capture.inspectionChain.previousInspection.path)),capture.inspectionChain.previousInspection),'previous inspection identity differs');
  const raw=readBound(capture.stdout),document=JSON.parse(raw.content.toString('utf8')),shared=shareValues(document),pages=[];
  checkNewDirectory(capture.cwd,directoryName);fs.mkdirSync(directoryName,{mode:0o700});directory(directoryName,true);
  const index={schemaVersion:1,kind:'phoenix-diagram-diagnostic-index',capture:captureFile,stdout:capture.stdout,stderr:capture.stderr,normalizedIdentity:hash(Buffer.from(canonicalJson(document))),document:pageChain(directoryName,'document',canonicalJson(shared.document),pages),references:shared.references,sharedValues:[]};
  for(const [id,text] of shared.shared)index.sharedValues.push({id,...pageChain(directoryName,`shared-${index.sharedValues.length+1}`,text,pages)});
  const indexFile=write(path.join(directoryName,'index.json'),index),indexStream=pageChain(directoryName,'index',JSON.stringify(index),pages),progress=write(path.join(directoryName,'progress.json'),progressFor(capture,captureFile,document,previous));
  const entry=write(path.join(directoryName,'inspection.json'),{schemaVersion:1,kind:'phoenix-diagram-inspection',capture:captureFile,stdout:capture.stdout,stderr:capture.stderr,index:indexFile,indexStream,totalPages:pages.length,totalBytes:pages.reduce((sum,file)=>sum+file.identity.bytes,0),progress});
  need(entry.identity.bytes<=PAGE_LIMIT,'inspection entry exceeds 8KiB');readInspection(entry.path);return entry;
}
export function readInspection(file,seen=new Set()) {
  need(!seen.has(file),'inspection predecessor cycle');seen.add(file);
  const entryFile=snapshot(file);need(entryFile.identity.bytes<=PAGE_LIMIT,'inspection entry exceeds 8KiB');const entry=json(entryFile),pages=[];
  need(entry.schemaVersion===1 && entry.kind==='phoenix-diagram-inspection','invalid inspection entry');const capture=validateCapture(readBound(entry.capture));
  need(inside(capture.cwd,file) && same(entry.stdout,capture.stdout) && same(entry.stderr,capture.stderr),'inspection source binding differs');
  const index=JSON.parse(readPages(entry.indexStream,pages));need(same(json(readBound(entry.index)),index) && same(index.capture,entry.capture) && same(index.stdout,entry.stdout) && same(index.stderr,entry.stderr),'index binding differs');
  const document=JSON.parse(readPages(index.document,pages)),values=new Map();
  for(const value of index.sharedValues){const text=readPages(value,pages);need(!values.has(value.id) && hash(Buffer.from(text)).sha256===value.id,'shared value identity differs');values.set(value.id,JSON.parse(text));}
  for(const ref of index.references){need(Array.isArray(ref.path) && ref.path.length && values.has(ref.id),'shared reference is missing');let parent=document;for(const key of ref.path.slice(0,-1)){need(parent && Object.hasOwn(parent,key),'shared reference path is missing');parent=parent[key];}const key=ref.path.at(-1);need(parent && Object.hasOwn(parent,key) && parent[key]===null,'shared reference placeholder differs');Object.defineProperty(parent,key,{value:structuredClone(values.get(ref.id)),enumerable:true,writable:true,configurable:true});}
  need(same(document,JSON.parse(readBound(entry.stdout).content.toString('utf8'))) && same(hash(Buffer.from(canonicalJson(document))),index.normalizedIdentity),'expanded JSON differs from original stdout');
  need(pages.length===entry.totalPages && new Set(pages.map(value=>value.path)).size===pages.length && pages.reduce((sum,value)=>sum+value.identity.bytes,0)===entry.totalBytes,'inspection page totals differ');
  const predecessor=capture.inspectionChain.previousInspection,previous=predecessor?readInspection(readBound(predecessor).path,seen):null;
  if(previous)need(previous.progress.sequence+1===capture.inspectionChain.sequence && same(previous.entry.capture,capture.inspectionChain.previousCapture) && same(validateCapture(readBound(previous.entry.capture)).cwd,capture.cwd),'inspection predecessor sequence or project differs');
  const progress=json(readBound(entry.progress));need(same(progress,progressFor(capture,entry.capture,document,previous)),'mechanical progress or predecessor differs');
  return{entry,index,document,progress,pages};
}
async function captureWithInspection(options) {
  checkNewDirectory(options.cwd,options.inspectionDir);
  need([options.stdout,options.stderr].every(file=>inside(options.cwd,file) && outsideChain(options.cwd,file)),'inspection captures must stay in the project, outside its reserved chain');
  return locked(options.cwd,async(root,verify)=>{
    let old=head(root),previous=null;
    if(old){const latest=json(old);need(latest.status==='complete' && latest.inspection && options.previousInspection===latest.inspection.path,'latest inspection is required; an unfinished capture can only be recovered with inspect');previous=readInspection(options.previousInspection);readBound(latest.inspection);need(same(previous.entry.capture,latest.capture) && previous.progress.sequence===latest.sequence && ['derived','not-applicable'].includes(previous.progress.status),'latest view or progress is incomplete');}
    else need(!options.previousInspection && fs.readdirSync(root).every(name=>name==='.lock'),'cannot start another chain or omit an existing head');
    if(old)checkChainSlots(root,json(old).sequence);
    const inspectionChain={sequence:old?json(old).sequence+1:1,project:directory(options.cwd),previousCapture:old?json(old).capture:null,previousInspection:old?json(old).inspection:null};
    let capture;
    try{capture=await captureStdio(options);}catch(error){if(!error.nativeStarted)throw error;const failed={schemaVersion:1,kind:'phoenix-diagram-stdio-capture-failed',cwd:options.cwd,inspectionChain,nativeResult:error.nativeResult,error:error.message,stdout:options.stdout,stderr:options.stderr},captureFile=write(path.join(root,`capture-${inspectionChain.sequence}.json`),failed);advance(root,old,{schemaVersion:1,sequence:inspectionChain.sequence,status:'capture.failed',capture:captureFile,inspection:null},verify);return{...failed,captureStatus:'failed',captureFile};}
    capture.inspectionChain=inspectionChain;const captureFile=write(path.join(root,`capture-${inspectionChain.sequence}.json`),capture);
    old=advance(root,old,{schemaVersion:1,sequence:inspectionChain.sequence,status:'inspection.failed',capture:captureFile,inspection:null},verify);
    let inspection;
    try{const entry=derive(captureFile,options.inspectionDir);advance(root,old,{schemaVersion:1,sequence:inspectionChain.sequence,status:'complete',capture:captureFile,inspection:entry},verify);inspection={status:'complete',entry};}
    catch(error){inspection={status:'failed',error:error.message,directory:options.inspectionDir};}
    return{...capture,captureFile,inspection};
  });
}
async function inspect(argv) {
  need(argv.length===4 && argv[0]==='--capture' && argv[2]==='--inspection-dir','inspect accepts only --capture and --inspection-dir; it never accepts a program or PD argv');
  const sealed=snapshot(argv[1]),capture=validateCapture(sealed);checkNewDirectory(capture.cwd,argv[3]);
  return locked(capture.cwd,async(root,verify)=>{const old=head(root),latest=old && json(old);need(latest && latest.sequence===capture.inspectionChain.sequence && same(latest.capture,reference(sealed)),'inspect can rebuild only the latest sealed capture');checkChainSlots(root,latest.sequence);const entry=derive(reference(sealed),argv[3]);advance(root,old,{...latest,status:'complete',inspection:entry},verify);return{schemaVersion:1,kind:'phoenix-diagram-inspection-rebuild',captureFile:reference(sealed),inspection:{status:'complete',entry}};});
}
export async function main(argv=process.argv.slice(2)) {
  if(argv.length===1 && argv[0]==='--help'){process.stdout.write(HELP);return 0;}
  try {if(argv[0]==='inspect'){process.stdout.write(`${JSON.stringify(await inspect(argv.slice(1)))}\n`);return 0;}const options=parseCaptureArgs(argv),result=await(options.inspectionDir?captureWithInspection(options):captureStdio(options));process.stdout.write(`${JSON.stringify(result)}\n`);return result.exitCode??4;}
  catch(error){process.stderr.write(`${error.message}\n`);return 4;}
}
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url))process.exitCode=await main();
