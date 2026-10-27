#!/usr/bin/env node
// Read-only maintenance view; never executes Native, repairs a chain or judges quality.
import fs from 'node:fs';
import path from 'node:path';
import {isDeepStrictEqual as same} from 'node:util';
import {fileURLToPath} from 'node:url';
import {readInspection} from './capture-stdio.mjs';
import {bound,readBound,readSnapshot} from '../../../../scripts/test-phoenix-diagram-core.mjs';

export const HELP='Usage: node read-current-progress.mjs --project ABSOLUTE_PROJECT --json\nRead the latest sealed mechanical progress without running Native or writing files.\nRead data.status, data.observations, data.gates and data.stopRequired; exit 0 only means the view was authenticated.\n';
const need=(value,message)=>{if(!value)throw new Error(`current progress: ${message}`);};
const inside=(parent,file)=>typeof file==='string' && path.isAbsolute(file) && path.normalize(file)===file && file.startsWith(parent+path.sep);
function directory(file,privateOnly=false) {
  const stat=fs.lstatSync(file,{bigint:true});
  need(stat.isDirectory() && !stat.isSymbolicLink() && Number(stat.uid)===process.getuid()
    && fs.realpathSync(file)===file && (!privateOnly || (stat.mode&0o7777n)===0o700n),'project/chain must be canonical, ordinary and owned; chain requires 0700');
  return{device:String(stat.dev),inode:String(stat.ino),uid:Number(stat.uid),mode:Number(stat.mode&0o7777n)};
}
function slots(root,sequence) {
  const names=fs.readdirSync(root);
  need(!names.includes('.lock'),'producer or recovery lock exists; do not use earlier progress');
  need(Number.isSafeInteger(sequence) && sequence>0 && names.length===sequence+1
    && names.every(name=>name==='latest.json' || /^capture-[1-9][0-9]*\.json$/.test(name)
      && Number(name.slice(8,-5))<=sequence),'chain has an uncommitted capture/transaction or a rewound head; Root recovery is required');
}
export function readCurrentProgress(project) {
  need(typeof project==='string' && path.isAbsolute(project) && path.normalize(project)===project,'an absolute project is required');
  const projectBinding=directory(project),root=path.join(project,'stdio/inspection-chain'),chainBinding=directory(root,true);
  need(!fs.existsSync(path.join(root,'.lock')),'producer or recovery lock exists; do not use earlier progress');
  const headFile=bound(readSnapshot(path.join(root,'latest.json'))),head=JSON.parse(readBound(headFile).content);
  need(head.schemaVersion===1 && head.status==='complete' && head.inspection && head.capture,'latest capture/view is incomplete; retain it and stop dependent work');
  slots(root,head.sequence);
  need(inside(project,head.inspection.path) && !inside(root,head.inspection.path)
    && head.capture.path===path.join(root,`capture-${head.sequence}.json`),'head references another project or reserved chain entry');
  const entry=JSON.parse(readBound(head.inspection).content);
  need(same(entry.capture,head.capture) && inside(project,entry.progress?.path),'inspection references another capture or project');
  const capture=JSON.parse(readBound(head.capture).content);
  need(capture.cwd===project && inside(project,capture.stdout?.path) && inside(project,capture.stderr?.path)
    && capture.installedReceipt?.path===path.join(project,'.phoenix-diagram/install/active.json')
    && inside(project,capture.program) && capture.launcher?.path===capture.program,'capture references another project');
  const view=readInspection(head.inspection.path);
  need(capture.cwd===project && same(head.capture,view.entry.capture) && view.progress.sequence===head.sequence,'head, inspection and project do not identify the same current capture');
  const progress=JSON.parse(readBound(view.entry.progress).content);
  need(same(progress,view.progress),'progress changed after reconstruction');
  const reader=bound(readSnapshot(fileURLToPath(import.meta.url)));
  // A new producer must not turn this into a view of its previous best capture.
  readBound(headFile);readBound(head.inspection);readBound(view.entry.progress);slots(root,head.sequence);
  need(same(directory(project),projectBinding) && same(directory(root,true),chainBinding),'project or chain binding changed while reading');
  return{schemaVersion:1,kind:'phoenix-diagram-current-progress-view',project,reader,head:headFile,inspection:head.inspection,progress:view.entry.progress,data:progress};
}
export function main(argv=process.argv.slice(2)) {
  if(argv.length===1 && argv[0]==='--help'){process.stdout.write(HELP);return 0;}
  try{
    need(argv.length===3 && argv[0]==='--project' && argv[2]==='--json','expected --project ABSOLUTE_PROJECT --json');
    process.stdout.write(JSON.stringify(readCurrentProgress(argv[1]))+'\n');return 0;
  }catch(error){process.stderr.write(error.message+'\n');return 4;}
}
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url))process.exitCode=main();
