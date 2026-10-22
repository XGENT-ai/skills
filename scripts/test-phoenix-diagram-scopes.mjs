// Acceptance scopes that run on resume against an existing core run: the frozen
// reference corpus, repository evidence, interface languages and exploration.
import fs from 'node:fs';
import path from 'node:path';
import { randomBytes } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { root,LEGACY_TYPES,TYPES,identity,readSnapshot,readBound,bound,requireThat } from './test-phoenix-diagram-core.mjs';

const REFERENCE=path.join(root,'local/phoenix-diagram/reference/archify');
const MANIFEST=path.join(root,'tools/phoenix-diagram/upstream/archify-3.1.0-manifest.json');
const COLLECTIONS={architecture:['components','connections'],workflow:['nodes','edges'],sequence:['participants','messages'],dataflow:['nodes','flows'],lifecycle:['states','transitions'],erd:['entities','relationships'],class:['types','relationships'],tree:['nodes',null],timeline:['events',null],waterfall:['spans',null]};
const DEFERRED=/(?:\/brand|^\/meta\/views|^\/meta\/animation)$/;

function directory(file) {fs.mkdirSync(file,{mode:0o700});return fs.realpathSync(file);}
function writeNew(file,value) {fs.writeFileSync(file,Buffer.isBuffer(value)?value:Buffer.from(`${JSON.stringify(value,null,2)}\n`),{flag:'wx',mode:0o600});return bound(readSnapshot(file));}
function typeOf(name) {return TYPES.find(type=>name.endsWith(`.${type}.json`));}
function metadataOf(html) {
  const scripts=[...html.matchAll(/<script\b(?=[^>]*\bid="phoenix-diagram-metadata")[^>]*>([\s\S]*?)<\/script>/g)];
  requireThat(scripts.length===1,'Artifact needs exactly one metadata script');return JSON.parse(scripts[0][1]);
}
const attribute=value=>String(value).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;').replaceAll("'",'&#39;');

/** The pinned reference examples, checked against the frozen manifest before and after. */
export function referenceFiles(reference=REFERENCE) {
  const manifest=JSON.parse(fs.readFileSync(MANIFEST,'utf8'));
  const files=manifest.filter(([name])=>name.startsWith('archify/examples/') && !name.slice('archify/examples/'.length).includes('/') && (typeOf(name) || name.endsWith('.html')));
  return files.map(([name,size,sha256])=>{
    const file=path.join(reference,name.slice('archify/'.length)),actual=identity(fs.readFileSync(file));
    requireThat(actual.bytes===size && actual.sha256===sha256,`Frozen reference example differs from its manifest: ${name}`,'core/frozen-reference');
    return{name:path.basename(file),file,identity:actual};
  });
}

// Every authored entity appears once as a node and every authored relation keeps
// its direction; explicit geometry and labels are bound by the strict metadata check.
export function verifyCoreFacts(document,type,html) {
  const [nodes,relations]=COLLECTIONS[type],metadata=metadataOf(html),authored=document[nodes] || [];
  // Metadata lists nodes in drawing order; each authored node must appear unchanged.
  const byId=list=>JSON.stringify([...(list || [])].sort((a,b)=>a.id<b.id?-1:a.id>b.id?1:0));
  requireThat(byId(metadata.graph?.nodes)===byId(authored),'Metadata nodes differ from the authored facts');
  for(const node of authored)requireThat(html.split(`data-node-id="${attribute(node.id)}"`).length===2,`Authored node ${node.id} is not rendered exactly once`);
  const expected=relations?(document[relations] || []):type==='tree'?authored.filter(node=>node.parent!==undefined).map(node=>({from:node.parent,to:node.id})):[];
  for(const relation of expected)requireThat(html.includes(`data-edge-from="${attribute(relation.from)}" data-edge-to="${attribute(relation.to)}"`) || html.includes(`data-edge-to="${attribute(relation.to)}" data-edge-from="${attribute(relation.from)}"`),`Relation ${relation.from}→${relation.to} lost its direction`);
  return{nodes:authored.length,relations:expected.length};
}

const DETAIL_VIEWPORTS=[{width:1440,height:900},{width:390,height:844}];
async function corpus(index,options,{invoke,passports}) {
  if(index.corpus?.status==='pass')return index.corpus;
  passports??=(await import('../tools/phoenix-diagram/tests/export-acceptance.mjs')).runPassportAcceptance;
  const scopeRoot=directory(path.join(index.runRoot,`corpus-${randomBytes(8).toString('hex')}`));
  const before=referenceFiles(),cases=[];
  const legacyLifecycle=path.join(root,'tools/phoenix-diagram/upstream/legacy-2.17/examples/deployment-release.lifecycle.json');
  const inputs=[...before.filter(entry=>typeOf(entry.name)).map(entry=>({...entry,contract:'phoenix-v1'})),{name:'deployment-release.lifecycle.legacy-v1.json',file:legacyLifecycle,identity:identity(fs.readFileSync(legacyLifecycle)),contract:'legacy-2.17'}];
  for(const entry of inputs) {
    const type=typeOf(entry.name.replace('.legacy-v1',''))||'lifecycle',caseRoot=directory(path.join(scopeRoot,entry.name.replace(/\.json$/,'')));
    const candidate=writeNew(path.join(caseRoot,'candidate.json'),fs.readFileSync(entry.file)),output=path.join(caseRoot,'diagram.html');
    const explicit=entry.contract==='phoenix-v1' && LEGACY_TYPES.includes(type)?['--input-contract','phoenix-v1']:[];
    const delivered=await invoke(index.engine,['deliver',type,candidate.path,output,...explicit,'--json'],{options});
    requireThat(delivered.exitCode===0 && delivered.receipt?.status==='pass' && delivered.receipt.inputContract===entry.contract,`Reference ${entry.name} did not deliver under ${entry.contract}: ${JSON.stringify(delivered.receipt?.diagnostics || delivered.stderr)}`);
    const strict=await invoke(index.engine,['check',output,'--strict-provenance','--json'],{options});
    requireThat(strict.exitCode===0 && strict.receipt?.provenance==='current','Reference delivery failed strict provenance');
    const deferred=delivered.receipt.compatibility?.deferredFields || [];
    requireThat(delivered.receipt.compatibility?.coreCompatibility==='pass' && deferred.every(field=>DEFERRED.test(field.path) && field.phase==='P3'),`Unexpected deferred field in ${entry.name}`);
    const document=JSON.parse(readBound(candidate,'corpus candidate').content.toString('utf8'));
    const artifact=bound(readSnapshot(output)),facts=verifyCoreFacts(document,type,readBound(artifact,'corpus artifact').content.toString('utf8'));
    const record={name:entry.name,type,contract:entry.contract,schemaVersion:document.schema_version,candidate,artifact,facts,deferred:deferred.map(field=>field.path),delivery:writeNew(path.join(caseRoot,'delivery.json'),delivered.receipt),strict:writeNew(path.join(caseRoot,'strict.json'),strict.receipt)};
    // Every 3.1.0 reference passes the browser gate as delivered; the 2.17
    // lifecycle only proves the old contract's core facts.
    if(entry.contract==='phoenix-v1') {
      const visual=await invoke(index.engine,['visual-check',output,'--out-dir',path.join(caseRoot,'browser'),'--json'],{options});
      requireThat(visual.exitCode===0 && visual.receipt?.status==='pass',`Reference ${entry.name} failed browser checks`);
      record.screenshots=visual.receipt.screenshots.filter(shot=>shot.viewport.width===1440 && shot.viewport.height===900).map(shot=>({path:shot.path,identity:shot.identity,theme:shot.theme,viewing:'required'}));
      requireThat(record.screenshots.length===2,'Dark and light 1440x900 screenshots are required for actual viewing');
      record.visual=writeNew(path.join(caseRoot,'visual-check.json'),visual.receipt);
      // A table with several foreign keys names every target in its details, on desktop and narrow screens.
      const tables=type==='erd'?document.entities.filter(entity=>(entity.attributes || []).filter(attribute=>attribute.references).length>1):[];
      if(tables.length) {
        const report=await passports({artifactPath:output,evidenceDir:directory(path.join(caseRoot,'passports')),browserPath:options.browserPath,offline:true,evidence:false,nodes:tables.flatMap(entity=>DETAIL_VIEWPORTS.map(viewport=>({id:entity.id,theme:'light',viewport})))});
        requireThat(report.status==='pass',`Reference ${entry.name} details failed: ${JSON.stringify(report.errors)}`);
        for(const passport of report.passports)requireThat(tables.find(entity=>entity.id===passport.id).attributes.filter(attribute=>attribute.references).every(attribute=>passport.context.includes(`${attribute.name} → ${attribute.references}`)),`Reference ${entry.name} details omit a foreign key target of ${passport.id}`);
        record.passports={receipt:bound(readSnapshot(report.receiptPath)),screenshots:report.passports.map(passport=>({id:passport.id,viewport:passport.viewport,path:passport.screenshot.path,identity:passport.screenshot.identity,viewing:'required'}))};
      }
    }
    cases.push(record);
  }
  const after=referenceFiles();
  requireThat(JSON.stringify(after)===JSON.stringify(before),'A frozen reference JSON or HTML changed during the corpus run');
  const lifecycle=cases.filter(entry=>entry.name.startsWith('deployment-release.lifecycle'));
  requireThat(lifecycle.length===2 && lifecycle.some(entry=>entry.contract==='legacy-2.17' && entry.schemaVersion===1) && lifecycle.some(entry=>entry.contract==='phoenix-v1' && entry.schemaVersion===3),'Legacy v1 and reference v3 lifecycle are not both checked');
  return{status:'pass',references:before.filter(entry=>typeOf(entry.name)).length,html:before.filter(entry=>entry.name.endsWith('.html')).length,unchanged:after.map(entry=>({name:entry.name,identity:entry.identity})),cases};
}

function git(cwd,args) {
  const env=Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.startsWith('GIT_')));
  const result=spawnSync('git',['-c','core.hooksPath=/dev/null',...args],{cwd,env,encoding:'utf8',timeout:120000});
  requireThat(result.status===0,`git ${args[0]} failed: ${result.stderr}`,'core/git');return result.stdout.trim();
}

// Read-only link existence through the GitHub API; absent network is reported, not hidden.
function linkExists(href) {
  const match=/^https:\/\/github\.com\/([^/]+)\/([^/]+)\/blob\/([0-9a-f]{40})\/([^#?]+)/.exec(href);
  requireThat(match,`Unexpected GitHub source link ${href}`);
  const result=spawnSync('gh',['api',`repos/${match[1]}/${match[2]}/contents/${match[4]}?ref=${match[3]}`,'--silent'],{encoding:'utf8',timeout:30000});
  if(result.error || /could not resolve|connect|timeout|network/i.test(result.stderr || ''))return{href,status:'not-run',reason:(result.error?.message || result.stderr).trim()};
  return{href,status:result.status===0?'pass':'failed',...(result.status===0?{}:{reason:result.stderr.trim()})};
}

function evidenceDocuments(revision) {
  const repository={url:'https://github.com/XGENT-ai/skills',revision};
  const output=name=>({title:name,output:`${name}.html`,input_contract:'phoenix-v1',quality_profile:'showcase'});
  return{
    architecture:{schema_version:1,diagram_type:'architecture',meta:{...output('Phoenix Diagram input gate'),repository},layout:{mode:'grid',cols:2},
      components:[
        {id:'gate',type:'backend',label:'Input gate',sublabel:'contracts and semantics',row:0,col:1,sources:[{path:'tools/phoenix-diagram/src/input.rs',line:1,end_line:20,label:'parse_input'}]},
        {id:'schemas',type:'database',label:'Schema registry',sublabel:'embedded, offline',row:1,col:1,sources:[{path:'tools/phoenix-diagram/src/schema.rs',line:1,end_line:30}]},
        {id:'skill',type:'frontend',label:'Skill text',row:0,col:0,sources:[{path:'skills/phoenix-diagram/SKILL.md'}]},
      ],
      connections:[{from:'skill',to:'gate',label:'candidate'},{from:'gate',to:'schemas',label:'lookup'}]},
    timeline:{schema_version:1,diagram_type:'timeline',meta:{...output('Phoenix Diagram P1 acceptance'),repository,evidence:'observed',timezone:'Asia/Shanghai'},
      events:[
        {id:'definition',at:'2026-10-19T12:00+08:00',precision:'day',title:'M7 definition preserved',kind:'change',sources:[{path:'docs/plan/PHOENIX-DIAGRAM-P1.md',line:1,end_line:5}]},
        {id:'accepted',at:'2026-10-20T13:52:22+08:00',title:'M7 candidate accepted',kind:'action',sources:[{path:'docs/plan/PHOENIX-DIAGRAM-P1.records/M7.md',line:1}]},
        {id:'recorded',at:'2026-10-20T13:52:49+08:00',title:'Final commit recorded',kind:'recovery',sources:[{path:'docs/plan/PHOENIX-DIAGRAM-P1.md',line:30,end_line:40}]},
      ]},
    tree:{schema_version:1,diagram_type:'tree',meta:{...output('Phoenix Diagram skill files'),repository:{...repository,link_mode:'local-only'}},
      nodes:[
        {id:'skill',label:'skills/phoenix-diagram',sources:[{path:'skills/phoenix-diagram/SKILL.md',line:1,end_line:8}]},
        {id:'schemas',label:'schemas',parent:'skill',sources:[{path:'skills/phoenix-diagram/schemas/README.md'}]},
        {id:'scripts',label:'scripts',parent:'skill',sources:[{path:'skills/phoenix-diagram/scripts/phoenix-launcher.cjs',line:1}]},
      ]},
    waterfall:{schema_version:1,diagram_type:'waterfall',meta:{...output('Illustrative request timing'),evidence:'illustrative',unit:'ms'},
      spans:[{id:'request',name:'GET /orders',start:0,end:120},{id:'auth',name:'Authorize',start:4,end:22,parent:'request'},{id:'query',name:'Query orders',start:25,duration:70,parent:'request',service:'db'},{id:'render',name:'Render',start:96,status:'incomplete',parent:'request'}]},
  };
}

async function finalizeDocument(index,options,invoke,caseRoot,type,document,repoRoot) {
  const candidate=writeNew(path.join(caseRoot,'candidate.json'),Buffer.from(`${JSON.stringify(document,null,2)}\n`)),output=path.join(caseRoot,'diagram.html');
  const native=await invoke(index.engine,['finalize',type,candidate.path,output,...(repoRoot?['--repo-root',repoRoot]:[]),'--out-dir',path.join(caseRoot,'browser'),'--json'],{options});
  requireThat(native.receipt?.automatedStatus==='pass' && native.exitCode===0,`${type} finalize did not pass its automatic gates: ${JSON.stringify(native.receipt?.diagnostics || native.stderr)}`);
  const html=readSnapshot(output).content.toString('utf8'),metadata=metadataOf(html);
  const screenshots=JSON.parse(readBound(native.receipt.evidence.browserReceipt,'browser receipt').content.toString('utf8')).screenshots.filter(shot=>shot.viewport.width===1440).map(shot=>({path:shot.path,identity:shot.identity,theme:shot.theme,viewing:'required'}));
  return{type,candidate,artifact:bound(readSnapshot(output)),receipt:writeNew(path.join(caseRoot,'finalize.json'),native.receipt),summary:native.receipt.evidence.summaryReceipt,evidence:native.receipt.evidence,locale:native.receipt.locale,sourceEvidence:metadata.sourceEvidence,html,screenshots};
}

async function evidence(index,options,{invoke}) {
  if(index.evidence?.status==='pass')return index.evidence;
  const scopeRoot=directory(path.join(index.runRoot,`evidence-${randomBytes(8).toString('hex')}`));
  // A temporary clone of the pushed commit keeps the working tree clean by construction.
  // A package consumer has no checkout of its own and names the repository to cite.
  const repository=options.evidenceRepository || root,revision=git(repository,['rev-parse','origin/main']),clone=path.join(scopeRoot,'checkout');
  git(scopeRoot,['clone','--quiet','--shared','--no-checkout',repository,clone]);git(clone,['checkout','--quiet','--detach',revision]);
  git(clone,['remote','set-url','origin','https://github.com/XGENT-ai/skills.git']);
  const documents=evidenceDocuments(revision),cases={};
  for(const type of ['architecture','timeline','tree','waterfall'])cases[type]=await finalizeDocument(index,options,invoke,directory(path.join(scopeRoot,type)),type,documents[type],type==='waterfall'?null:clone);
  for(const type of ['architecture','timeline'])requireThat(cases[type].evidence.status==='verified' && cases[type].sourceEvidence.references.every(reference=>reference.state==='verified' && reference.href),`${type} sources are not verified with links`);
  requireThat(cases.tree.evidence.status==='verified' && cases.tree.sourceEvidence.repository.linkMode==='local-only' && cases.tree.sourceEvidence.references.every(reference=>!reference.href),'Local-only tree evidence is not verified without links');
  requireThat(cases.waterfall.evidence.status==='none' && /data-waterfall-evidence="illustrative"/.test(cases.waterfall.html),'Illustrative waterfall timing is not disclosed');
  requireThat(cases.timeline.html.includes('data-timeline-evidence="observed"') && JSON.stringify(cases.timeline.sourceEvidence).length>0 && /"precision":"day"/.test(cases.timeline.html),'Day-precision observed timeline is not disclosed');
  fs.appendFileSync(path.join(clone,'tools/phoenix-diagram/src/schema.rs'),'\n// uncommitted change\n');
  // The changed variant also leaves one component uncited to show all three states.
  const changedDocument=structuredClone(documents.architecture);delete changedDocument.components[2].sources;
  const changedRoot=directory(path.join(scopeRoot,'architecture-worktree-change'));
  const changed=await finalizeDocument(index,options,invoke,changedRoot,'architecture',changedDocument,clone);
  const reasons=changed.sourceEvidence.references.map(reference=>reference.reason||null);
  requireThat(changed.evidence.status==='partial' && reasons.filter(Boolean).length===1 && reasons.includes('worktree-differs') && JSON.stringify(changed.sourceEvidence.unsourced)==='["skill"]','A changed cited file or an uncited element was not reported');
  const links=['architecture','timeline'].flatMap(type=>cases[type].sourceEvidence.references.map(reference=>linkExists(reference.href)));
  requireThat(links.every(link=>link.status!=='failed'),`A verified source link does not exist on GitHub: ${JSON.stringify(links.filter(link=>link.status==='failed'))}`);
  const strip=entry=>{const {html,...rest}=entry;return rest;};
  const reader=await import('../tools/phoenix-diagram/tests/export-acceptance.mjs'),passports=[];
  for(const [label,entry,ids,states] of [['verified',cases.architecture,['gate'],['verified']],['changed',changed,['schemas','skill'],['unverified','unsourced']],['local-only',cases.tree,['schemas'],['verified']]]) {
    const report=await reader.runPassportAcceptance({artifactPath:entry.artifact.path,evidenceDir:directory(path.join(scopeRoot,`passports-${label}`)),browserPath:options.browserPath,offline:true,nodes:ids.flatMap(id=>['dark','light'].map(theme=>({id,theme})))});
    requireThat(report.status==='pass',`Passport capture failed: ${JSON.stringify(report.errors)}`);
    for(const passport of report.passports)requireThat(passport.state===states[ids.indexOf(passport.id)],`Passport ${passport.id} shows ${passport.state}`);
    if(label==='local-only')requireThat(report.passports.every(passport=>passport.rows.every(row=>!row.href) && !passport.repositoryHref),'Local-only evidence shows links');
    passports.push({label,receipt:bound(readSnapshot(report.receiptPath)),screenshots:report.passports.map(passport=>({id:passport.id,theme:passport.theme,state:passport.state,status:passport.status,path:passport.screenshot.path,identity:passport.screenshot.identity,viewing:'required'}))});
  }
  return{status:'pass',revision,links,linkExistence:links.every(link=>link.status==='pass')?'pass':'not-run',cases:Object.fromEntries(Object.entries(cases).map(([key,value])=>[key,strip(value)])),worktreeChange:strip(changed),passports};
}

const LOCALES=['en','zh-CN','zh-TW','es','ko','fr'];
async function locale(index,options,{invoke}) {
  if(index.locale?.status==='pass')return index.locale;
  const scopeRoot=directory(path.join(index.runRoot,`locale-${randomBytes(8).toString('hex')}`)),cases=[];
  for(const tag of LOCALES) {
    const document={schema_version:1,diagram_type:'timeline',meta:{title:`Release timeline (${tag})`,output:'timeline.html',input_contract:'phoenix-v1',locale:tag,evidence:'observed',timezone:'Europe/Berlin'},
      lanes:[{id:'build',label:'Build'},{id:'ops',label:'Operations'}],
      events:[{id:'cut',at:'2026-03-28T23:30+00:00',title:'Release cut',lane:'build',kind:'change'},{id:'dst',at:'2026-03-29T03:15+02:00',title:'Deploy after DST change',lane:'ops',kind:'action'},{id:'alert',at:'2026-03-30T12:00+02:00',precision:'day',title:'Error budget alert',lane:'ops',kind:'alert'},{id:'fixed',at:'2026-04-02T09:00+02:00',title:'Recovered',lane:'ops',kind:'recovery'}]};
    const result=await finalizeDocument(index,options,invoke,directory(path.join(scopeRoot,tag)),'timeline',document,null);
    const expected=tag==='fr'?{requested:'fr',resolved:'en',fallback:true}:{requested:tag,resolved:tag,fallback:false};
    requireThat(JSON.stringify(result.locale)===JSON.stringify(expected),`Receipt locale differs for ${tag}: ${JSON.stringify(result.locale)}`);
    requireThat(result.html.includes(`<html lang="${expected.resolved}"`) && result.html.includes(` lang="${expected.resolved}" data-diagram-type="timeline"`),`Resolved interface language is not written for ${tag}`);
    const {html,...rest}=result;cases.push({tag,...rest});
  }
  return{status:'pass',cases};
}

const RELATION_TYPES=new Set(['architecture','workflow','sequence','dataflow','lifecycle','erd','class','tree']);
async function explore(index,options) {
  if(index.explore?.status==='pass')return index.explore;
  const reader=await import('../tools/phoenix-diagram/tests/export-acceptance.mjs'),scopeRoot=directory(path.join(index.runRoot,`explore-${randomBytes(8).toString('hex')}`)),cases=[];
  requireThat(index.cases.every(entry=>entry.state?.automatedStatus==='pass'),'Every case must pass its automatic gates before exploration');
  for(const entry of index.cases) {
    readBound(entry.state.artifact,'explored artifact');
    const report=await reader.runExploreAcceptance({artifactPath:entry.state.artifact.path,evidenceDir:directory(path.join(scopeRoot,entry.id)),browserPath:options.browserPath,offline:true});
    requireThat(report.status==='pass',`Exploration failed for ${entry.id}: ${JSON.stringify(report.errors)}`);
    requireThat(report.offline?.remoteRequests?.length===0,'Exploration made a remote request');
    requireThat(RELATION_TYPES.has(entry.type)?report.cards.length===2:report.cards.length===0,`Share card scope is wrong for ${entry.id}`);
    readBound(entry.source,'author source after exploration');readBound(entry.candidate,'candidate after exploration');
    cases.push({id:entry.id,type:entry.type,receipt:bound(readSnapshot(report.receiptPath)),steps:report.explore.observations.map(sample=>({name:sample.name,status:sample.status})),cards:report.cards.map(card=>({action:card.action,name:card.download.name,path:card.path,identity:card.identity,decode:card.decode,canonical:card.receipt['data-last-export-canonical'],variant:card.receipt['data-last-export-variant']}))});
  }
  return{status:'pass',cases};
}

const RUNNERS={corpus,evidence,locale,explore};

export async function run(name,index,options,context) {
  requireThat(Object.hasOwn(RUNNERS,name),`Scope ${name} has no runner yet`,'core/scope-unavailable');
  return await RUNNERS[name](index,options,context);
}
