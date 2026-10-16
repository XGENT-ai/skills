// Maintenance-only, pinned Node oracle for cross-edge pin causality.
import assert from 'node:assert/strict';
import {readFile, writeFile, rename} from 'node:fs/promises';
import {fileURLToPath, pathToFileURL} from 'node:url';
import path from 'node:path';
import {verifyFrozenSources} from '../../../../../scripts/check-phoenix-diagram-source.mjs';
import documents from './pin-cases.mjs';
const root=fileURLToPath(new URL('../../../../../',import.meta.url));
verifyFrozenSources({localReference:path.join(root,'local/phoenix-diagram/reference')});

function repairChanges(message, document) {
  const replacement=message.match(/^set labelAt on edge "([^"]+)" to (\[[^\]]+\])$/);
  if (replacement) return [{op:'set',path:`/edges/${document.edges.findIndex(edge=>edge.id===replacement[1])}/labelAt`,value:JSON.parse(replacement[2])}];
  const segment=message.match(/^set labelSegment on edge "([^"]+)" to (\d+)$/);
  if (segment) return [{op:'set',path:`/edges/${document.edges.findIndex(edge=>edge.id===segment[1])}/labelSegment`,value:Number(segment[2])}];
  const preset=message.match(/^set edge "([^"]+)" route to verified preset "([^"]+)"$/);
  if (preset) return [{op:'set',path:`/edges/${document.edges.findIndex(edge=>edge.id===preset[1])}/route`,value:preset[2]}];
  const removals=[...message.matchAll(/remove (\w+(?: and \w+)*) from edge "([^"]+)"/g)];
  if (!removals.length) return null;
  return removals.flatMap(([,fields,id])=>fields.split(' and ').map(field=>({op:'remove',path:`/edges/${document.edges.findIndex(edge=>edge.id===id)}/${field}`})));
}
function apply(document, changes) {
  const candidate=structuredClone(document);
  for (const change of changes) {
    const [, , index, field]=change.path.split('/');
    assert.ok(Number(index)>=0);
    if(change.op==='set')candidate.edges[index][field]=change.value;
    else delete candidate.edges[index][field];
  }
  return candidate;
}
const cases=[];
for(const [contract,relative] of [['legacy-2.17','tools/phoenix-diagram/upstream/legacy-2.17'],['phoenix-v1','local/phoenix-diagram/reference/archify']]) {
  const source=`${relative}/renderers/workflow/workflow-compiler.mjs`;
  const {compileWorkflow}=await import(pathToFileURL(path.join(root,source)));
  const schemaValidator=`${relative}/renderers/shared/generated-validators.mjs`;
  const {workflow:validateSchema}=await import(pathToFileURL(path.join(root,schemaValidator)));
  for(const specimen of documents) {
    if(specimen.contracts&&!specimen.contracts.includes(contract))continue;
    const document=structuredClone(specimen.document);
    const sourceSchemaValid=validateSchema(document);
    const sourceSchemaErrors=sourceSchemaValid?[]:validateSchema.errors.map(({instancePath,keyword,params})=>({path:instancePath,keyword,params}));
    const schemaEvidence={schemaValidator,sourceSchemaValid,sourceSchemaErrors};
    const before=JSON.stringify(document);
    let result;
    try {result=compileWorkflow({workflow:document,qualityProfile:specimen.quality});}
    catch(error) {
      assert.equal(JSON.stringify(document),before,'Source must preserve authored pins');
      cases.push({...specimen,contract,source,...schemaEvidence,sourceException:{name:error.name,message:error.message}});
      continue;
    }
    assert.equal(JSON.stringify(document),before,'Source must preserve authored pins');
    const diagnostic=result.diagnostics?.find(d=>d.severity==='error');
    const repairs=[];
    let rejectedProperSubsets=0;
    for(const message of ['workflow/explicit-pin-conflict','workflow/route-preset-conflict'].includes(diagnostic?.code)?diagnostic.supportedFixes||[]:[]) {
      const changes=repairChanges(message,document);
      assert.ok(changes,`${specimen.name}: unparsed source repair ${message}`);
      assert.equal(compileWorkflow({workflow:apply(document,changes),qualityProfile:specimen.quality}).ok,true,`Source repair did not recompile: ${message}`);
      if(changes.length>1&&changes.every(change=>change.op==='remove')) {
        for(let mask=1;mask<(1<<changes.length)-1;mask+=1) {
          const subset=changes.filter((_,index)=>mask&(1<<index));
          assert.equal(compileWorkflow({workflow:apply(document,subset),qualityProfile:specimen.quality}).ok,false,`Source removal set was not minimal: ${message}`);
          rejectedProperSubsets+=1;
        }
      }
      repairs.push(changes);
    }
    cases.push({...specimen,contract,source,...schemaEvidence,ok:result.ok,diagnostic:diagnostic||null,repairs,rejectedProperSubsets,receipt:result.ok?result.receipt:null});
  }
}
const output=new URL('./pin-oracle.json',import.meta.url);
const serialized=`${JSON.stringify({format:1,cases},null,2)}\n`;
if(process.argv.includes('--check'))assert.equal(await readFile(output,'utf8'),serialized,'Pin oracle differs; regenerate using the pinned Node toolchain');
else {const temporary=new URL('./.pin-oracle-next.json',import.meta.url);await writeFile(temporary,serialized);await rename(temporary,output);}
console.log(`${process.argv.includes('--check')?'Verified':'Wrote'} ${cases.length} cross-edge pin cases; ${cases.filter(c=>c.ok).length} accepted; ${cases.filter(c=>c.sourceException).length} source exceptions; ${cases.reduce((count,c)=>count+(c.repairs?.length||0),0)} complete source-verified repairs.`);
