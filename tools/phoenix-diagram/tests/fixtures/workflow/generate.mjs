// Maintenance oracle: execute only the two verified source compilers.
// Use the pinned Node toolchain; --check verifies without replacing fixtures.
import assert from 'node:assert/strict';
import { readFile, readdir, rename, writeFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

const root = fileURLToPath(new URL('../../../../../', import.meta.url));
verifyFrozenSources({ localReference: path.join(root, 'local/phoenix-diagram/reference') });
const sourceRoots = [
  ['legacy-2.17', 'tools/phoenix-diagram/upstream/legacy-2.17'],
  ['phoenix-v1', 'local/phoenix-diagram/reference/archify'],
];
const base = version => ({
  schema_version: version,
  diagram_type: 'workflow',
  meta: { title: 'Workflow corpus', output: 'workflow.html', quality_profile: 'standard' },
  lanes: [{ id: 'work', label: 'Work' }],
  nodes: [
    { id: 'a', lane: 'work', col: 0, type: 'frontend', label: 'A' },
    { id: 'b', lane: 'work', col: 1, type: 'backend', label: 'B' },
  ],
  edges: [{ id: 'ab', from: 'a', to: 'b', label: 'go' }],
  mainPath: ['a', 'b'],
});
const documents = [];
const add = (name, document) => documents.push({ name, document });
function nodeText(svg='') {
  let node;
  const rows=[];
  for (const line of svg.split('\n')) {
    node=line.match(/<g\b[^>]*\bdata-node-id="([^"]+)"/)?.[1]||node;
    if (!node||!line.includes('<text ')) continue;
    const field=line.includes('data-node-label=""')?'label':line.includes('data-detail="context"')?'sublabel':line.includes('data-detail="fine"')?'tag':null;
    if (!field) continue;
    const attributes=Object.fromEntries([...line.matchAll(/([\w-]+)="([^"]*)"/g)].map(([,name,value])=>[name,value]));
    rows.push({node,field,x:Number(attributes.x),y:Number(attributes.y),fontSize:Number(attributes['font-size'])});
  }
  return rows;
}
function nodeSigils(svg='') {
  let node;
  const sigils=[];
  for (const line of svg.split('\n')) {
    node=line.match(/<g\b[^>]*\bdata-node-id="([^"]+)"/)?.[1]||node;
    const match=line.match(/data-semantic-sigil="([^"]+)"[^>]*transform="translate\(([^ ]+) ([^)]+)\) scale\(([^)]+)\)"/);
    if (node&&match) sigils.push({node,kind:match[1],x:Number(match[2]),y:Number(match[3]),size:Number(match[4])*16});
  }
  return sigils;
}
function legendEntries(svg='') {
  const decode=text=>text.replace(/&(?:amp|lt|gt|quot|#39);/g,entity=>({'&amp;':'&','&lt;':'<','&gt;':'>','&quot;':'"','&#39;':"'"}[entity]));
  let attributes;
  const entries=[];
  for (const line of svg.split('\n')) {
    if (line.includes('data-legend-semantic-kind=')) attributes=Object.fromEntries([...line.matchAll(/([\w-]+)="([^"]*)"/g)].map(([,name,value])=>[name,value]));
    else if (attributes&&line.includes('<text ')) {
      const label=line.match(/>([^<]*)<\/text>/)?.[1];
      if (label!==undefined) entries.push({kind:attributes['data-legend-semantic-kind'],label:decode(label),x:Number(attributes['data-legend-x']),y:Number(attributes['data-legend-baseline']),width:Number(attributes['data-legend-width'])});
      attributes=undefined;
    }
  }
  return entries;
}
for (const version of [1, 2]) add(`v${version}-simple`, base(version));
let document = base(2);
document.nodes[0].label = 'Prepare deployment request';
add('v2-automatic-width', document);
document = base(2);
document.nodes[0].tag = '01';
document.groups = [{ id: 'group', label: 'Preparation', lane: 'work', fromCol: 0, toCol: 1 }];
document.phases = [{ id: 'phase', label: 'Intake', fromCol: 0, toCol: 1 }];
add('v2-group-frame', document);
document = base(2);
document.lanes.push({ id: 'service', label: 'Service', variant: 'exception' });
document.nodes.push({ id: 'c', lane: 'work', col: 0, type: 'security', label: 'C', yOffset: 90 });
document.nodes.push({ id: 'd', lane: 'service', col: 1, type: 'database', label: 'D' });
document.edges.push({ id: 'bd', from: 'b', to: 'd', label: 'accept', role: 'branch' });
document.edges.push({ id: 'da', from: 'd', to: 'a', label: 'retry', role: 'return' });
add('v2-independent-lanes-and-return', document);
for (const version of [1, 2]) {
  document = base(version);
  document.meta.viewBox = [800, 360];
  document.edges[0] = { ...document.edges[0], fromSide: 'bottom', toSide: 'bottom', via: [[88, 170], [220, 170]], labelAt: [154, 170], labelDx: 2, labelDy: -3 };
  add(`v${version}-absolute-pins`, document);
}
document = base(2);
document.nodes[1].col = 0;
delete document.mainPath;
add('v2-overlapping-nodes', document);
document = base(2);
document.meta.viewBox = [720, 240];
document.nodes[1].col = 5;
add('v2-insufficient-authored-canvas', document);
document = base(1);
document.nodes[1].width = 140;
document.nodes[1].col = 2;
document.nodes[0].col = 1;
add('v1-capacity-overflow', document);
for (let col=0;col<5;col+=1) {
  for (const width of [92,132,200]) {
    for (const [labelName,label] of [['none',''],['cjk','条件：同意'],['channel','x'.repeat(65)]]) {
      document=base(2);
      document.nodes[0].col=col;document.nodes[1].col=col+1;
      document.nodes[0].width=width;document.nodes[1].width=292-width;
      document.edges[0].label=label;
      add(`adjacent-${col}-${width}-${labelName}`,document);
    }
  }
}
for (const route of ['straight','drop','outside-right','return-left','bottom-channel','up-channel']) {
  document=base(2);document.nodes[1].col=2;document.edges[0].route=route;
  add(`preset-${route}`,document);
}
for (const [name,fields] of [
  ['duplicate-via',{fromSide:'right',toSide:'left',via:[[160,119],[160,119]]}],
  ['diagonal-via',{fromSide:'right',toSide:'left',via:[[160,150]]}],
  ['negative-via',{via:[[-16,119],[-16,180]]}],
  ['contradictory-channel',{route:'straight',channelX:160}],
  ['negative-label',{labelAt:[1,1]}],
  ['node-label',{labelAt:[94,119]}],
  ['valid-label',{labelAt:[154,175]}],
]) {
  document=base(2);Object.assign(document.edges[0],fields);add(`pin-${name}`,document);
}
document=base(2);document.meta.viewBox=[1600,600];add('v2-capacity-does-not-stretch-ranks',document);
document=base(2);document.meta.legend={mode:'all',entries:Object.fromEntries(['frontend','backend','security','messagebus','database','cloud','external'].map(kind=>[kind,{label:'Legend label '.repeat(5)}]))};add('v2-multirow-legend',document);
document=base(1);document.meta.legend={mode:'all',entries:Object.fromEntries(['frontend','backend','security','messagebus','database','cloud','external'].map(kind=>[kind,{label:'Legend label '.repeat(5)}]))};add('v1-legend-vertical-overflow',document);
document=base(1);document.meta.legend={entries:{frontend:{label:'字'.repeat(80)}}};add('v1-legend-label-too-wide',document);
document=base(2);document.edges=[{from:'a',to:'b',label:'\uE000'},{from:'a',to:'b',label:'🌳'}];add('unicode-canonical-edge-order',document);
document=base(2);document.nodes[0].label='Prepare deployment request';Object.assign(document.edges[0],{fromSide:'bottom',toSide:'bottom',via:[[88,170],[220,170]]});add('pinned-route-failure-precedes-intrinsic-text-sizing',document);

const cases = [];
for (const [contract, relative] of sourceRoots) {
  const source = `${relative}/renderers/workflow/workflow-compiler.mjs`;
  const { compileWorkflow } = await import(pathToFileURL(path.join(root, source)).href);
  const registerLocale=contract==='phoenix-v1'?(await import(pathToFileURL(path.join(root,relative,'renderers/shared/i18n.mjs')).href)).registerLocale:null;
  const sourceDocuments = [...documents];
  if (contract==='phoenix-v1') {
    for (const locale of ['en','zh-CN','zh-TW','es','ko','fr']) {
      const document=base(2);document.meta.locale=locale;document.meta.legend={mode:'all'};sourceDocuments.push({name:`locale-${locale}`,document});
    }
    for (const [name,translations] of [
      ['override',{'legend.workflow.frontend':'Interfaz especial extensa','legend.title':'Claves'}],
      ['invalid-placeholder',{'legend.workflow.frontend':'Interfaz {actor}'}],
    ]) {
      const document=base(2);document.meta.locale='es';document.meta.translations=translations;sourceDocuments.push({name:`locale-${name}`,document});
    }
    for (const icon of ['none','clock']) {const document=base(2);document.nodes[0].icon=icon;sourceDocuments.push({name:`icon-${icon}`,document});}
    const document=base(2);Object.assign(document.nodes[0],{label:'Some text',width:64,height:32});sourceDocuments.push({name:'small-node-sigil-rail',document});
    const rows=[['submit','buyer',0,0],['retry','buyer',3,0],['validate','order',0,-50],['pending','order',1,50],['rejected','order',3,-50],['reserve','stock',0,-50],['release','stock',4,50],['pay','payment',1,-50],['result','payment',2,-50],['failed','payment',3,-50],['deadline','payment',4,50],['paid','confirmation',2,0],['event','confirmation',3,0],['close','confirmation',4,0],['pick','warehouse',3,0],['ship','warehouse',4,0]];
    const links=[['submit','validate',''],['validate','reserve','Valid'],['reserve','pending','Held'],['reserve','rejected','Empty','error'],['pending','pay',''],['pay','result',''],['result','paid','Yes'],['result','failed','No','error'],['failed','retry','Retry','return'],['retry','pay','In time','return'],['pending','deadline','Wait','branch'],['deadline','close','Yes','error'],['paid','event',''],['event','pick','Paid','async'],['pick','ship',''],['close','release','Release']];
    sourceDocuments.push({name:'optional-placement-advice',documentSource:'local/phoenix-diagram/reference/test/workflow-placement-repair.test.mjs#fixture',document:{schema_version:2,diagram_type:'workflow',meta:{title:'Retry and expiry',output:'retry-expiry.html',quality_profile:'showcase'},lanes:['buyer','order','stock','payment','confirmation','warehouse'].map(id=>({id,label:id})),nodes:rows.map(([id,lane,col,yOffset])=>({id,lane,col,yOffset,type:'backend',label:id,width:152,height:60})),edges:links.map(([from,to,label,role])=>({id:`${from}-${to}`,from,to,label,...(role?{role}:{})})),mainPath:['submit','validate','reserve','pending','pay','result','paid','event','pick','ship'],semanticChecks:{allowedRoots:['submit'],allowedTerminals:['rejected','release','ship'],requiredPaths:[{from:'failed',to:'pay'},{from:'pending',to:'release'}]}}});
  }
  for (const name of (await readdir(path.join(root, relative, 'examples'))).sort().filter(name => name.endsWith('.workflow.json'))) {
    const source = `${relative}/examples/${name}`;
    sourceDocuments.push({ name:`example:${name}`, document:JSON.parse(await readFile(path.join(root,source))), documentSource:source });
  }
  const fixtureRoot = contract === 'phoenix-v1' ? 'local/phoenix-diagram/reference/test/fixtures' : `${relative}/test/fixtures`;
  const names = ['v1-workflow-700x400.workflow.json','v1-workflow-explicit-coordinates.workflow.json',...(contract === 'phoenix-v1' ? ['workflow-free-space.json','workflow-return-arrival.json','workflow-dense-automatic.json','workflow-shared-trunks.workflow.json','workflow-fanin-label-recovery.json','workflow-route-text-recovery.json','workflow-first-draft/automatic-port-slot.workflow.json'] : [])];
  for (const name of names) {
    const source=`${fixtureRoot}/${name}`;
    sourceDocuments.push({ name:`fixture:${name}`, document:JSON.parse(await readFile(path.join(root,source))), documentSource:source });
  }
  for (const { name, document, documentSource } of sourceDocuments) {
    const workflow = structuredClone(document);
    const before = JSON.stringify(workflow);
    const quality=workflow.meta.quality_profile || 'standard';
    registerLocale?.(workflow.meta.locale,workflow.meta.translations);
    const result = compileWorkflow({ workflow, qualityProfile: quality });
    assert.equal(JSON.stringify(workflow), before, `${contract}/${name}: source input mutated`);
    cases.push({ contract, name, source, documentSource, document, quality, ok: result.ok, receipt: result.receipt, nodeText:nodeText(result.svg), sigils:nodeSigils(result.svg), legends:legendEntries(result.svg), readerFit:result.svg?.match(/data-reader-fit="([^"]+)"/)?.[1]||'contain', diagnostics: result.diagnostics || result.receipt?.diagnostics || [] });
  }
}
const output=new URL('./oracle.json',import.meta.url);
const serialized=`${JSON.stringify({ format: 1, cases }, null, 2)}\n`;
if (process.argv.includes('--check')) {
  assert.ok(await readFile(output,'utf8')===serialized,'Workflow oracle differs; regenerate with the pinned Node toolchain');
} else {
  const temporary=new URL('./.oracle-next.json',import.meta.url);
  await writeFile(temporary,serialized);
  await rename(temporary,output);
}
console.log(`${process.argv.includes('--check')?'Verified':'Wrote'} ${cases.length} workflow compiler oracle cases; ${cases.filter(testCase => testCase.ok).length} accepted.`);
