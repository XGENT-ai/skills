#!/usr/bin/env node
// Maintenance-only stdout collection. This is not a product CLI or a renderer.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { isDeepStrictEqual as same } from 'node:util';
import { fileURLToPath } from 'node:url';

export const COLLECTOR_MEMBER='tools/phoenix-diagram/tests/support/capture-stdio.mjs';
export const HELP='Usage: node capture-stdio.mjs --program ABSOLUTE_INSTALLED_PD --cwd ABSOLUTE_PROJECT --stdout NEW_FILE --stderr NEW_FILE -- PD_ARGV...\nMaintenance capture only. Output parents must be owned ordinary 0700 directories. No download, rendering or installation is performed by the collector.\n';
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
  const options={},keys={'--program':'program','--cwd':'cwd','--stdout':'stdout','--stderr':'stderr'};let at=0;
  for(;at<argv.length && argv[at]!=='--';at+=2) {
    const key=keys[argv[at]],value=argv[at+1];need(key && !Object.hasOwn(options,key) && typeof value==='string' && path.isAbsolute(value) && path.normalize(value)===value && !/[\0\r\n]/.test(value),'expected unique absolute program/cwd/stdout/stderr paths');options[key]=value;
  }
  need(Object.keys(options).length===4 && argv[at]==='--' && argv.length>at+1,'four capture options and an explicit PD argv are required');
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
  let child;
  try {
    for(const file of [options.stdout,options.stderr])descriptors.push(fs.openSync(file,fs.constants.O_RDWR|fs.constants.O_CREAT|fs.constants.O_EXCL|fs.constants.O_NOFOLLOW,0o600));
    const created=descriptors.map(fd=>physical(fs.fstatSync(fd,{bigint:true})));
    const result=await new Promise((resolve,reject)=>{
      child=spawn(options.program,options.argv,{cwd:options.cwd,stdio:['ignore',...descriptors],detached:true});
      const kill=signal=>{if(child.exitCode===null && child.signalCode===null)try{process.kill(-child.pid,signal);}catch(error){if(error.code!=='ESRCH')reject(error);}};
      const interrupt=()=>kill('SIGTERM'),timer=setTimeout(()=>kill('SIGKILL'),300000),cleanup=()=>{clearTimeout(timer);process.removeListener('SIGTERM',interrupt);process.removeListener('SIGINT',interrupt);};process.once('SIGTERM',interrupt);process.once('SIGINT',interrupt);
      child.once('error',error=>{cleanup();reject(error);});child.once('close',(exitCode,signal)=>{cleanup();resolve({exitCode,signal});});
    });
    for(const fd of descriptors)fs.fsyncSync(fd);
    const files=[snapshot(options.stdout),snapshot(options.stderr)];need(files.every((file,index)=>same(file.binding,created[index])),'capture slot changed after native execution');
    for(const [parent,original] of parents)need(same(directory(parent,true),original),'capture parent changed');
    need(same(directory(options.cwd),project),'project directory changed');unchanged(program);unchanged(installed);unchanged(self);
    return{schemaVersion:1,kind:'phoenix-diagram-stdio-capture',tool:'phoenix-diagram',collector:reference(self),node:{path:fs.realpathSync(process.execPath),version:process.version},program:options.program,argv:options.argv,cwd:options.cwd,installedReceipt:reference(installed),launcher:reference(program),...result,stdout:reference(files[0]),stderr:reference(files[1])};
  } finally {for(const fd of descriptors)fs.closeSync(fd);}
}
export async function main(argv=process.argv.slice(2)) {
  if(argv.length===1 && argv[0]==='--help'){process.stdout.write(HELP);return 0;}
  try {const result=await captureStdio(parseCaptureArgs(argv));process.stdout.write(`${JSON.stringify(result)}\n`);return result.exitCode??4;}
  catch(error){process.stderr.write(`${error.message}\n`);return 4;}
}
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url))process.exitCode=await main();
