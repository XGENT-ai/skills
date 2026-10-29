#!/usr/bin/env node
// Author-tools and final-candidate acceptance. Human observations are supplied only
// through the original run's native review-record protocol.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual as same} from 'node:util';
import {validateDevelopmentArtifact} from '../../../scripts/build-phoenix-diagram.mjs';
import {invokeNative,verifyReaderReport} from '../../../scripts/test-phoenix-diagram-runtime.mjs';
import {runExportAcceptance} from './export-acceptance.mjs';
import {discoverBrowser} from './support/export-browser.mjs';
import {validateCandidate,initialConsume,resumeConsume,readConsumerContext,prepareCandidateTransport,readCandidateTransport,targetForHost,publicNpmCommand} from '../../../scripts/check-phoenix-diagram-candidate.mjs';
import {KIND,ACCEPTANCE_KIND,CASE_IDS,OFFICIAL_RESOURCE,privateDirectory,writeNew,jsonBound,bindOfficialResource,readReviewMap,comparisonDraft,singleDraft,rememberCase,refreshCase,verifyComparisonReview,recompute,persistState,verifySavedState,verifyActualTransport,verifyAuthorToolsState,authorToolsAnchor,verifyAuthorToolsAnchor,verifyAcceptanceState,recomputeAcceptance,bound,readSnapshot,readBound,identity} from './support/p3-state.mjs';
import {runCompareFaults,runPreviewAcceptance} from './support/p3-processes.mjs';

export function parseOptions(argv){
  const names={'--stage':'stage','--release-dir':'releaseDir','--candidate-dir':'candidateDir','--legacy-candidate-dir':'legacyCandidateDir','--provider-manifest':'providerManifest','--migration-fixture':'migrationFixture','--evidence-repo':'evidenceRepository','--npm-cli':'npmCliPath','--output-dir':'outputDir','--resume':'resume','--review-file':'reviewFile','--official-resource':'officialResource'},result={};
  for(let at=0;at<argv.length;at+=2){const key=names[argv[at]],value=argv[at+1];assert(key&&!Object.hasOwn(result,key),'Unknown or duplicate option '+argv[at]);assert(value&&!value.startsWith('--'),'Option requires a value');result[key]=key==='stage'?value:path.resolve(value);}
  assert(['author-tools','all','providers'].includes(result.stage),'Unknown acceptance stage');
  if(result.stage==='author-tools'){assert(result.releaseDir,'Provide --release-dir');assert(!result.candidateDir&&!result.legacyCandidateDir&&!result.providerManifest&&!result.migrationFixture&&!result.evidenceRepository&&!result.npmCliPath,'Candidate options require all/providers');}
  else {assert(result.candidateDir&&!result.releaseDir,'Final acceptance requires --candidate-dir, not a development release');if(result.stage==='providers')assert(result.providerManifest&&!result.reviewFile&&!result.resume,'providers requires its manifest and a new output directory');}
  assert(Boolean(result.outputDir)!==Boolean(result.resume),'Provide one new --output-dir or the original --resume state.json');assert(!result.reviewFile||result.resume,'--review-file requires --resume');return result;
}

export function createPublicInvoke(consumer,{invoke=invokeNative}={}){
  const {publicNpm,engine}=consumer;assert(publicNpm?.enginePath===engine.path,'Public transport engine differs');
  return async(actual,args,settings={})=>{
    assert(actual.path===engine.path&&same(actual.identity,engine.identity),'Public transport cannot select another engine');readBound(engine,'same installed public engine');
    if(consumer.candidate?.ownedTarball)readBound(consumer.candidate.ownedTarball,'same public npm package');
    assert(!settings.options?.publicNpm||same(settings.options.publicNpm,publicNpm),'Public transport cannot be replaced');
    const command=publicNpmCommand(publicNpm,args),prefix=settings.prefix||[];let actualTransport;
    const configuration={...settings,options:{...settings.options,publicNpm},onSpawn:async child=>{
      actualTransport={schemaVersion:1,kind:'p3-public-npm-process',program:command.program,argv:command.args,cwd:command.cwd,prefix:[...prefix],spawnedProgram:child.spawnfile,spawnedArgv:[...child.spawnargs],pid:child.pid,startedAt:new Date().toISOString()};
      verifyActualTransport(actualTransport,publicNpm,args,prefix,{complete:false});await settings.onSpawn?.(child);
    }};
    try{const result=await invoke(actual,args,configuration);assert(actualTransport,'Public npm invocation has no actual spawn evidence');actualTransport.closedAt=new Date().toISOString();return{...result,actualTransport};}
    catch(error){if(error.processResult&&actualTransport){actualTransport.closedAt=new Date().toISOString();Object.defineProperty(error,'processResult',{value:{...error.processResult,actualTransport},configurable:true});}throw error;}
  };
}
export function loadRelease(directory){
  const artifact=validateDevelopmentArtifact(directory),target=`${process.platform}-${process.arch}`;
  assert.equal(artifact.target,target,'Stage A must run the actual native engine for this host');
  const files=Object.fromEntries([['version','VERSION.json'],['buildReceipt','build-receipt.json'],['sourceInputs','source-inputs.json'],['bundle','bundle/manifest.json'],['engine',artifact.binary.asset]].map(([name,file])=>[name,bound(readSnapshot(path.join(artifact.directory,file)))]));
  const engineIdentity=artifact.receipt.native.engineDoctor.engine;assert(same(engineIdentity.binary,files.engine.identity),'Engine probe differs from its actual executable');
  return{root:artifact.directory,target,sourceCommit:artifact.version.sourceCommit,sourceState:artifact.receipt.sourceState,sourceInputSha256:artifact.version.sourceInputSha256,bundleSha256:artifact.version.bundleSha256,files,engineIdentity};
}
export function offlinePrefix({browser=false}={}){
  // Chrome must initialize its own macOS sandbox. Browser receipts prove
  // CDP offline mode and resource requests separately from native OS denial.
  if(browser)return[];
  if(process.platform==='darwin'){fs.accessSync('/usr/bin/sandbox-exec',fs.constants.X_OK);return['/usr/bin/sandbox-exec','-p','(version 1) (allow default) (deny network-outbound (remote ip)) (deny network-inbound (local ip))'];}
  assert.equal(process.platform,'linux','No verified OS deny-network launcher for this host');fs.accessSync('/usr/bin/unshare',fs.constants.X_OK);return['/usr/bin/unshare','--user','--map-current-user','--net','--'];
}
function currentCase(state,id){const entry=state.cases[id];if(entry?.summary)refreshCase(entry,state.release);return entry;}
function processResult(file,args,engine){
  const record=jsonBound(file,'original native process');assert(same(record.nativeArgv,args)&&same(record.engine,engine),'Original process belongs to another command or engine');
  assert(record.schemaVersion===1&&record.kind==='p3-owned-native-process','Unsupported saved process evidence');
  const stdout=readBound(record.stdout,'original native stdout').content.toString('utf8');readBound(record.stderr,'original native stderr');return{...record,stdout,receipt:stdout.trim()?JSON.parse(stdout):null};
}
function processEvidence(directory,args,engine,result,startedAt,pid){
  const raw=result.processResult||result;
  return writeNew(path.join(directory,'process.json'),{schemaVersion:1,kind:'p3-owned-native-process',engine,nativeArgv:args,startedAt,closedAt:new Date().toISOString(),pid:pid??raw.pid??null,exitCode:raw.exitCode??null,signal:raw.signal??null,
    stdout:writeNew(path.join(directory,'stdout.txt'),raw.stdout||''),stderr:writeNew(path.join(directory,'stderr.txt'),raw.stderr||''),...(raw.actualTransport?{actualTransport:raw.actualTransport}:{})});
}

export async function invokeRecordedStep(state,id,args,{invoke=invokeNative,save=()=>{},browserPath,offline=false,role='automatic'}={}){
  const release=state.release;
    let step=state.steps[id];
    if(step?.status==='running'&&!step.process){const file=path.join(state.runRoot,'commands',id,'process.json');if(fs.existsSync(file)){step.process=bound(readSnapshot(file));save();}}
    if(step?.process){const original=processResult(step.process,args,release.files.engine);if(release.publicNpm)verifyActualTransport(original.actualTransport,release.publicNpm,args,step.prefix);if(['pass','running'].includes(step.status)){assert.equal(original.exitCode,0,'Original process did not pass');assert.equal(original.signal,null);assert.equal(original.receipt?.status,'pass');if(step.status==='running'){step.status='pass';save();}return original;}return null;}
    if(step?.status==='running'){state.diagnostics.push({stage:id,message:'The original process has no complete captured receipt; preserve outputs and use its explicit recovery path. It was not rerun.'});save();return null;}
    if(step?.status==='failed')return null;
    const prefix=offline?offlinePrefix({browser:['finalize','visual-check'].includes(args[0])}):[];
    const directory=privateDirectory(path.join(state.runRoot,'commands',id));step=state.steps[id]={status:'running',role,argv:args,prefix,networkScope:offline?(prefix.length?'OS denies external IP':'Native browser offline-resources measurements; no OS network isolation claim'):'Explicit operation; no offline claim'};save();
    let result,pid;const startedAt=new Date().toISOString();
    try{
      result=await invoke(release.files.engine,args,{options:{browserPath},prefix,onSpawn:child=>{pid=child.pid;}});
      step.process=processEvidence(directory,args,release.files.engine,result,startedAt,pid);save();
      assert.equal(result.exitCode,0,`${id}: ${result.stderr||result.stdout}`);assert.equal(result.signal,null);assert.equal(result.receipt?.status,'pass',`${id}: native stage is not passing`);
      step.status='pass';save();return result;
    }catch(error){
      if(!step.process&&error.processResult)step.process=processEvidence(directory,args,release.files.engine,error,startedAt,pid);
      step.status='failed';step.error={message:error.message,code:error.code||'acceptance/native'};state.diagnostics.push({stage:id,...step.error});save();return null;
    }
}

export async function resumeReviews(state,map,{invoke=invokeNative,save=()=>{}}={}){
  for(const [id,recordPath] of Object.entries(map.records)){
    const entry=currentCase(state,id);assert(entry?.evidence?.automatedStatus==='pass','Review cannot fill an unrun automatic gate');
    const original=bound(readSnapshot(recordPath)),value=jsonBound(original,'actual observation record');
    if(id==='comparison')verifyComparisonReview(value,entry.evidence);
    else {assert.equal(value.deliveryReceiptId,entry.evidence.deliveryReceiptId,'Review belongs to another single delivery');assert(same(value.browserReceiptIdentity,entry.evidence.browserReceiptIdentity),'Review belongs to another browser receipt');}
    if(entry.sealedReviewInput)assert(same(entry.sealedReviewInput.identity,original.identity),'A sealed review must be retried with the same record bytes');
    entry.reviewInput=original;save();
    const result=await invoke(state.release.files.engine,['review-record',entry.artifact.path,'--record',original.path,'--json']);
    entry.reviewAttempts=(entry.reviewAttempts||0)+1;
    const directory=privateDirectory(path.join(entry.root,`review-attempt-${entry.reviewAttempts}`));
    entry.reviewProcess=processEvidence(directory,['review-record',entry.artifact.path,'--record',original.path,'--json'],state.release.files.engine,result,new Date().toISOString());save();
    assert(result.receipt?.command==='review-record','Native review-record returned no typed receipt');
    readBound(original,'unchanged actual observation record');
    const sealed=result.receipt.evidence?.reviewRecord||result.receipt.evidence?.visualReviewRecord||result.receipt.visualReview?.record||result.receipt.diagnostics?.map(diagnostic=>diagnostic.evidence?.sealedReviewRecord).find(Boolean);
    if(sealed){entry.sealedReviewRecord=bound(readSnapshot(sealed.path));assert(same(entry.sealedReviewRecord.identity,original.identity),'Sealed review bytes differ from actual supplied observations');entry.sealedReviewInput=original;save();}
    const reference=result.receipt.evidence?.summaryReceipt;
    if(reference){assert.equal(reference.path,entry.summary.path,'Review moved the original summary');entry.summary=bound(readSnapshot(reference.path));}
    refreshCase(entry,state.release);save();
    if(result.exitCode!==0&&entry.evidence.visualReview==='required-pending')throw new Error('Native review-record did not seal/update this comparison; preserve and retry the same record');
  }
  return state;
}

export async function runAuthorTools(options,{releaseLoader=loadRelease,invoke=invokeNative,exports=runExportAcceptance,preview=runPreviewAcceptance,faults=runCompareFaults,browser=discoverBrowser}={}){
  const release=releaseLoader(options.releaseDir);let state,previous;
  if(options.resume){
    previous=bound(readSnapshot(options.resume));state=jsonBound(previous,'author-tools state');verifySavedState(state,previous);assert(same(release,state.release),'Resume must use the same source, bundle, VERSION and physical engine');
    for(const id of CASE_IDS)currentCase(state,id);
  }else{
    const runRoot=privateDirectory(options.outputDir);privateDirectory(path.join(runRoot,'commands'));
    const stat=fs.statSync(runRoot);
    state={schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',kind:KIND,profile:'p3',stage:'author-tools',runId:randomBytes(16).toString('hex'),runRoot,runDirectory:{device:String(stat.dev),inode:String(stat.ino)},release,startedAt:new Date().toISOString(),status:'incomplete',automatedStatus:'not-run',visualReview:'required-pending',deliveryStatus:'incomplete',cases:{},steps:{},preview:{status:'not-run'},compareFaults:{status:'not-run'},brandExports:{status:'not-run'},packageConsumer:{status:'not-applicable',reason:release.distribution==='candidate'?'Public package consumption is recorded by the owning all wrapper':'Stage A does not consume a certified npm candidate'},providers:{status:'not-applicable',reason:'Provider sessions belong to final candidate acceptance'},finalCandidate:{status:'not-applicable',reason:'No final candidate certification was run'},diagnostics:[]};
  }
  bindOfficialResource(state,options.officialResource);
  const save=()=>{previous=persistState(recompute(state),previous);};save();
  let browserPath;try{browserPath=browser();}catch(error){state.diagnostics.push({stage:'browser',message:error.message});save();}
  const call=(id,args,settings)=>invokeRecordedStep(state,id,args,{...settings,invoke,save,browserPath});
  const stage=async(name,operation)=>{
    if(state[name].status==='pass'){
      const report=jsonBound(state[name].record,'original '+name);assert.equal(report.status,'pass');
      if(name==='brandExports')verifyReaderReport(report,state.cases['brand-capture'].artifact,{offline:true});
      else {assert(same(report.engine,state.release.files.engine),'Saved process scope used another engine');assert.equal(state[name].record.path,path.join(state.runRoot,name==='preview'?'preview-acceptance':'compare-faults','index.json'));}
      return;
    }
    if(state[name].status==='failed')return;
    state[name].status='running';save();
    try{const result=await operation();state[name]=result;assert.equal(result.status,'pass',name+' is incomplete');}
    catch(error){state[name]={...state[name],status:'failed',error:{message:error.message,code:error.code||'acceptance/stage',...(error.evidence?{evidence:error.evidence}:{})}};state.diagnostics.push({stage:name,...state[name].error});}save();
  };
  const prepareCase=(id,document)=>{
    if(state.cases[id])return state.cases[id];
    const root=privateDirectory(path.join(state.runRoot,id)),entry={id,root,status:'not-run',candidate:writeNew(path.join(root,'candidate.architecture.json'),document)};state.cases[id]=entry;save();return entry;
  };
  const source=bound(readSnapshot(path.join(release.root,'bundle/examples/minimal.architecture.json')));
  const document=JSON.parse(readBound(source,'public minimal architecture input').content);document.layout.cellW=200;document.layout.gapX=48;
  const finalize=async entry=>{
    if(entry.summary){refreshCase(entry,release);return;}
    const valid=await call(`validate-${entry.id}`,['validate','architecture',entry.candidate.path,'--brand-cache',path.join(state.runRoot,'brand-cache'),'--json'],{offline:true});if(!valid)return;
    const output=path.join(entry.root,'diagram.html'),args=['finalize','architecture',entry.candidate.path,output,'--brand-cache',path.join(state.runRoot,'brand-cache'),'--out-dir',path.join(entry.root,'browser'),'--json'];
    const result=await call(`finalize-${entry.id}`,args,{offline:true});if(!result)return;
    entry.artifact=bound(readSnapshot(output));entry.summary=bound(readSnapshot(result.receipt.evidence.summaryReceipt.path));rememberCase(entry,release);
    entry.reviewDraft=writeNew(path.join(entry.root,'review-draft.json'),singleDraft(entry.evidence));entry.status='pass';save();
  };
  const builtin=structuredClone(document);builtin.components[0].brand='github';builtin.components[1].brand='postgresql';
  const selected=await call('brands-list',['brands','github','--json'],{offline:true});if(selected)assert(selected.receipt.matches?.some(mark=>mark.id==='github'),'Built-in brand query did not return its exact catalog ID');
  await finalize(prepareCase('brand-builtin',builtin));
  if(state.officialResource){
    const cache=path.join(state.runRoot,'brand-cache');if(!fs.existsSync(cache))privateDirectory(cache);
    const captured=await call('brands-capture',['brands','capture',state.officialResource.descriptor.url,'--brand-cache',cache,'--json']);
    if(captured){
      for(const [key,value] of Object.entries(OFFICIAL_RESOURCE))assert.equal(captured.receipt.resource?.[key],value,'Captured official PNG '+key+' differs');
      assert(captured.receipt.brand?.url===state.officialResource.descriptor.url&&captured.receipt.brand.sha256===OFFICIAL_RESOURCE.sha256,'Capture did not return the explicit digest-pinned URL');assert.equal(path.dirname(captured.receipt.cachePath),cache,'Capture cache escaped this run');
      const resource=bound(readSnapshot(captured.receipt.cachePath)),envelope=jsonBound(resource,'captured resource envelope'),image=Buffer.from(envelope.dataBase64,'base64');assert(same(identity(image),{bytes:125,sha256:OFFICIAL_RESOURCE.sha256})&&image.subarray(0,8).equals(Buffer.from('89504e470d0a1a0a','hex')),'Captured envelope is not the supplied PNG');
      state.capturedResource={status:'pass',descriptor:state.officialResource.file,resource,brand:captured.receipt.brand};
      const remote=structuredClone(document);remote.components[0].brand=captured.receipt.brand;await finalize(prepareCase('brand-capture',remote));
    }
  }
  let comparison=state.cases.comparison;
  if(!comparison){
    const root=privateDirectory(path.join(state.runRoot,'comparison')),head=structuredClone(document);head.components[1].label='Worker service';head.components[1].row=1;head.connections[0].label='Dispatch';
    comparison=state.cases.comparison={id:'comparison',root,status:'not-run',base:writeNew(path.join(root,'base.architecture.json'),document),head:writeNew(path.join(root,'head.architecture.json'),head)};save();
  }
  const output=path.join(comparison.root,'comparison.html');
  if(!comparison.receipt){const result=await call('compare',['compare','architecture',comparison.base.path,comparison.head.path,output,'--json'],{offline:true});if(result){assert.equal(result.receipt.kind,'architecture-comparison');assert.equal(result.receipt.browserStatus,'not-run');assert.equal(result.receipt.deliveryStatus,'incomplete');assert.equal(result.receipt.visualReview.status,'required-pending');comparison.artifact=bound(readSnapshot(output));comparison.receipt=bound(readSnapshot(output+'.compare.json'));save();}}
  if(comparison.receipt&&!comparison.summary){
    const strict=await call('compare-check',['check',output,'--strict-provenance','--json'],{offline:true});
    if(strict){assert.equal(strict.receipt.strictProvenance,true);const visual=await call('compare-visual',['visual-check',output,'--out-dir',path.join(comparison.root,'browser'),'--json'],{offline:true});if(visual){comparison.summary=bound(readSnapshot(visual.receipt.evidence.summaryReceipt.path));rememberCase(comparison,release);comparison.reviewDraft=writeNew(path.join(comparison.root,'review-draft.json'),comparisonDraft(comparison.evidence));comparison.status='pass';save();}}
  }
  if(state.cases['brand-capture']?.summary&&browserPath)await stage('brandExports',async()=>{
    const entry=state.cases['brand-capture'],result=await exports({artifactPath:entry.artifact.path,evidenceDir:entry.root,browserPath,offline:true});verifyReaderReport(result,entry.artifact,{offline:true});return{status:'pass',record:bound(readSnapshot(result.receiptPath)),scope:'Actual captured-brand Reader exports with CDP network emulation; no image-review verdict inferred'};
  });
  if(comparison.receipt)await stage('compareFaults',()=>faults({engine:release.files.engine,runRoot:state.runRoot,base:comparison.base,head:comparison.head,invoke,publicNpm:release.publicNpm}));
  await stage('preview',()=>preview({engine:release.files.engine,runRoot:state.runRoot,document,invoke,publicNpm:release.publicNpm}));
  if(options.reviewFile){const map=readReviewMap(options.reviewFile,state);state.reviewMap=map.file;save();await resumeReviews(state,map,{invoke,save});}
  state.networkScopes={native:'Built-in query, both brand validates, comparison generation and strict comparison check use OS deny-IP; these are distinct from the browser commands.',browser:'Finalize/visual-check retain the browser sandbox and use CDP offline mode plus native offline-resources measurements; no OS network isolation claim.',exports:'Actual export browser CDP offline emulation and its recorded resource requests; distinct from OS network isolation.'};
  state.scopeLimitations=['Preview shell visual walkthrough remains a separate actual-browser/image observation; HTTP/process assertions do not supply that verdict.',release.distribution==='candidate'?'This child records author-tools only; the all wrapper retains separate consumer, migration and provider scopes, and final certification belongs to the aggregate.':'No final npm-candidate, four-platform certification or provider-session acceptance is performed by this Stage-A controller.'];
  state.reviewInstructions={runId:state.runId,records:Object.fromEntries(Object.values(state.cases).filter(entry=>entry.reviewDraft).map(entry=>[entry.id,entry.reviewDraft.path])),next:'Open each original screenshot, fill the existing observation drafts, then supply --resume <this state.json> --review-file <map with this runId>. No new finalize/compare/visual-check is needed.'};
  if(!state.officialResource&&!state.diagnostics.some(diagnostic=>diagnostic.code==='acceptance/official-resource-not-ready'))state.diagnostics.push({stage:'brand-capture',code:'acceptance/official-resource-not-ready',message:'No explicit fixed-commit official resource descriptor was supplied; HTTPS capture and captured-brand delivery remain not-run.'});
  save();return{...state,stateFile:previous,exitCode:state.status==='pass'?0:state.status==='failed'?1:3};
}

export function loadCandidateRelease(candidate,consumer){
  assert.equal(consumer.profile,'p3');assert.equal(consumer.automatedStatus,'pass');assert(same(consumer.source,candidate.source)&&same(consumer.candidate.tarball.identity,candidate.tgz.identity),'Consumed candidate differs');
  const root=path.join(consumer.packageRoot,'dist/phoenix-diagram'),owned=path.dirname(consumer.candidate.ownedRecord.path),target=consumer.target;
  const receipt=candidate.record.buildReceipts.find(row=>row.target===target);assert(receipt,'Candidate lacks this native build receipt');
  const files={version:bound(readSnapshot(path.join(root,'VERSION.json'))),buildReceipt:bound(readSnapshot(path.join(owned,receipt.path))),sourceInputs:bound(readSnapshot(path.join(owned,'source-inputs.json'))),bundle:bound(readSnapshot(path.join(root,'bundle/manifest.json'))),engine:bound(readSnapshot(consumer.engine.path))};
  assert(same(files.version.identity,candidate.files.get('release-dir/VERSION.json').identity)&&same(files.bundle.identity,candidate.files.get('release-dir/bundle/manifest.json').identity)&&same(files.engine.identity,candidate.engine.identity),'Installed public package differs from the candidate');
  const engineIdentity=consumer.engine.engineIdentity;assert(engineIdentity&&same(engineIdentity.binary,files.engine.identity)&&engineIdentity.sourceInputSha256===candidate.source.inputSha256,'Installed engine identity differs');
  const release={distribution:'candidate',root,target,sourceCommit:candidate.source.commit,sourceInputSha256:candidate.source.inputSha256,bundleSha256:candidate.version.bundleSha256,files,engineIdentity,publicNpm:consumer.publicNpm,candidate:{record:consumer.candidate.ownedRecord,tarball:consumer.candidate.ownedTarball}};
  const build=jsonBound(files.buildReceipt,'candidate native receipt');if(build.sourceState!==undefined)release.sourceState=build.sourceState;return release;
}

function candidateBinding(candidate){return{record:bound(candidate.snapshot),tarball:bound(candidate.tgz),version:bound(candidate.files.get('release-dir/VERSION.json')),sourceInputs:bound(candidate.files.get('source-inputs.json'))};}
function ownedInput(state,key,file,name){
  if(state[key]){readBound(state[key],'original owned '+key);if(file){const source=bound(readSnapshot(file));assert(same(source.identity,state[key].identity),'Resume cannot replace '+key);readBound(source,'unchanged explicit '+key);}return state[key];}
  if(!file)return null;const source=bound(readSnapshot(file)),owned=writeNew(path.join(state.runRoot,name),readBound(source,'explicit '+key).content);readBound(source,'unchanged source '+key);state[key]=owned;return owned;
}
function newAcceptance(options,candidate){
  const destination=path.join(fs.realpathSync(path.dirname(options.outputDir)),path.basename(options.outputDir)),relative=path.relative(candidate.directory,destination);assert(relative==='..'||relative.startsWith('..'+path.sep)||path.isAbsolute(relative),'Acceptance output must not alter the candidate inventory');
  const runRoot=privateDirectory(options.outputDir),stat=fs.statSync(runRoot);
  return{schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',kind:ACCEPTANCE_KIND,profile:'p3',stage:options.stage,runId:randomBytes(16).toString('hex'),runRoot,runDirectory:{device:String(stat.dev),inode:String(stat.ino)},target:targetForHost(),candidate:candidateBinding(candidate),source:candidate.source,startedAt:new Date().toISOString(),status:'incomplete',automatedStatus:'not-run',localStatus:'incomplete',visualReview:'required-pending',deliveryStatus:'incomplete',packageConsumer:{status:'not-run',automatedStatus:'not-run'},authorTools:{status:'not-run',automatedStatus:'not-run'},migration:{status:'not-run'},providers:{status:'not-run'},finalCandidate:{status:'incomplete',handoff:'pending-aggregate'},diagnostics:[]};
}
function wrapperReview(file,state){
  if(!file)return{};const ref=bound(readSnapshot(file)),value=jsonBound(ref,'same-run wrapper review map');assert(value.schemaVersion===1&&value.runId===state.runId,'Review map belongs to another acceptance run');assert(Object.keys(value).every(key=>['schemaVersion','runId','consumer','authorTools'].includes(key)),'Unknown wrapper review scope');
  for(const key of ['consumer','authorTools'])if(value[key])assert(typeof value[key]==='string'&&path.isAbsolute(value[key]),'Child review maps require explicit paths');return{...value,file:ref};
}
const missingProviders=async()=>({schemaVersion:1,kind:'phoenix-diagram-p3-provider-report',profile:'p3',status:'incomplete',providers:[],diagnostics:[{code:'acceptance/provider-verifier-not-ready',message:'No authenticated P3 provider verifier/context was supplied; no sessions were started.'}]});
async function providerScope(options,state,candidate,validateProviders){
  const manifest=ownedInput(state,'providerManifestInput',options.providerManifest,'provider-manifest.json');if(!manifest)return;
  if(state.providers.manifest)assert(same(state.providers.manifest,manifest),'Provider manifest changed across this run');
  state.providers={...state.providers,manifest};const report=await validateProviders({...options,profile:'p3',candidate,manifest:jsonBound(manifest,'provider manifest'),manifestFile:manifest});
  readBound(manifest,'unchanged provider originals manifest');assert(report?.schemaVersion===1&&report.kind==='phoenix-diagram-p3-provider-report'&&report.profile==='p3'&&['pass','incomplete'].includes(report.status),'Unsupported provider report');
  const record=writeNew(path.join(state.runRoot,`providers-${randomBytes(8).toString('hex')}.json`),report);state.providers={status:report.status,manifest,record,providers:report.providers,diagnostics:report.diagnostics};
}

export async function runProviders(options,{candidateLoader=validateCandidate,validateProviders=missingProviders}={}){
  const candidate=candidateLoader(options.candidateDir,targetForHost(),{profile:'p3'}),state=newAcceptance(options,candidate);let previous=persistState(state);
  try{await providerScope(options,state,candidate,validateProviders);state.status=state.providers.status==='pass'?'pass':'incomplete';state.automatedStatus=state.status==='pass'?'pass':'not-run';}
  catch(error){state.status='failed';state.diagnostics.push({code:error.code||'acceptance/providers',message:error.message});}
  previous=persistState(state,previous);return{...state,stateFile:previous,exitCode:state.status==='pass'?0:state.status==='failed'?1:3};
}

export async function runAll(options,{candidateLoader=validateCandidate,consume=initialConsume,resumeConsumer=resumeConsume,consumerContext=readConsumerContext,prepareLegacy=prepareCandidateTransport,legacyContext=readCandidateTransport,authorTools=runAuthorTools,releaseLoader=loadCandidateRelease,invoke=invokeNative,runMigration,verifyMigration,validateProviders=missingProviders,verifyProviders}={}){
  const target=targetForHost(),candidate=candidateLoader(options.candidateDir,target,{profile:'p3'}),consumerOptions={candidateDir:options.candidateDir,target,profile:'p3',...(options.npmCliPath?{npmCliPath:options.npmCliPath}:{})};let state,previous;
  const legacyOptions={target,profile:'p2',legacyDescriptor:bound(readSnapshot(fileURLToPath(new URL('./fixtures/migration/legacy-candidate.v1.json',import.meta.url)))),harnessCandidateDir:options.candidateDir,...(options.npmCliPath?{npmCliPath:options.npmCliPath}:{})};
  const verifyConsumer=(consumer)=>{const original=consumerContext({...consumerOptions,resume:state.packageConsumer.state.path},candidate).state;assert(same(original,consumer),'Consumer state differs from its original live context');};
  const migrationVerifier=verifyMigration?(scope,context)=>{const consumer=consumerContext({...consumerOptions,resume:state.packageConsumer.state.path},candidate).state;let legacyV1;if(state.legacyV1){const old=candidateLoader(options.legacyCandidateDir||state.legacyV1.directory,target,legacyOptions);assert(same(state.legacyV1.candidate,candidateBinding(old)),'Legacy precondition candidate changed');const installed=legacyContext({...legacyOptions,candidateDir:old.directory,resume:state.legacyV1.state.path},old).state;legacyV1={candidate:old,consumer:installed};}return verifyMigration(scope,{...context,candidate,consumer,legacyV1});}:undefined;
  const adoptAuthor=ref=>{const child=jsonBound(ref,'original child author-tools state');verifyAuthorToolsAnchor(child,state.authorTools.anchor);verifyAuthorToolsState(child,{requirePublic:true});assert(same(child.release.candidate.tarball.identity,state.candidate.tarball.identity),'Child author-tools belongs to another candidate');state.authorTools={status:child.status,automatedStatus:child.automatedStatus,visualReview:child.visualReview,deliveryStatus:child.deliveryStatus,state:ref,anchor:authorToolsAnchor(child)};};
  if(options.resume){previous=bound(readSnapshot(options.resume));state=jsonBound(previous,'original all state');assert.equal(state.stage,'all');assert.equal(previous.path,path.join(state.runRoot,'state.json'));privateDirectory(state.runRoot,{existing:true});const stat=fs.statSync(state.runRoot);assert(same(state.runDirectory,{device:String(stat.dev),inode:String(stat.ino)}),'Original wrapper directory was replaced');assert(same(state.candidate,candidateBinding(candidate))&&same(state.source,candidate.source),'Resume must retain its same candidate bytes and physical bindings');if(state.authorTools.status==='running'){const file=path.join(state.runRoot,'author-tools/state.json');if(fs.existsSync(file))adoptAuthor(bound(readSnapshot(file)));recomputeAcceptance(state);}verifyAcceptanceState(state,{verifyConsumer,verifyMigration:migrationVerifier,verifyProviders});}
  else {state=newAcceptance(options,candidate);previous=persistState(state);}
  const save=()=>{previous=persistState(recomputeAcceptance(state),previous);};
  try{
    delete state.failure;
    const review=wrapperReview(options.reviewFile,state);if(review.file){state.reviewMap=review.file;save();}
    const official=ownedInput(state,'officialResourceInput',options.officialResource,'official-resource.json');save();
    if(state.packageConsumer.status==='running'&&!state.packageConsumer.state){const file=path.join(state.runRoot,'consumer/consumer-state.json');if(fs.existsSync(file)){const ref=bound(readSnapshot(file)),original=jsonBound(ref,'interrupted consumer state');state.packageConsumer={status:original.status,automatedStatus:original.automatedStatus,phase:original.phase,scopes:original.scopes||[],state:ref,initialState:ref};save();}else {state.diagnostics.push({code:'acceptance/interrupted-consumer',message:'The original consumer has no complete captured state; it was not rerun.'});save();return{...state,stateFile:previous,exitCode:3};}}
    if(!state.packageConsumer.state){
      state.packageConsumer.status='running';save();const result=await consume({...consumerOptions,evidenceDir:path.join(state.runRoot,'consumer'),explore:true,locale:true,...(options.evidenceRepository?{evidence:true,evidenceRepository:options.evidenceRepository}:{})});
      state.packageConsumer={status:result.state.status,automatedStatus:result.state.automatedStatus,phase:result.state.phase,scopes:result.state.scopes||[],state:result.stateFile,initialState:result.stateFile};save();
    }
    if(state.packageConsumer.automatedStatus!=='pass')return{...state,stateFile:previous,consumerStateFile:state.packageConsumer.state,exitCode:state.status==='failed'?1:3};
    if(review.consumer&&state.packageConsumer.phase!=='complete'){
      const result=await resumeConsumer({...consumerOptions,resume:state.packageConsumer.state.path,reviewFile:review.consumer});state.packageConsumer={...state.packageConsumer,status:result.state.status,automatedStatus:result.state.automatedStatus,phase:result.state.phase,state:result.stateFile};save();
    }
    const consumer=consumerContext({...consumerOptions,resume:state.packageConsumer.state.path},candidate).state,release=releaseLoader(candidate,consumer),publicInvoke=createPublicInvoke(consumer,{invoke});
    if(!state.authorTools.state||state.authorTools.status!=='pass'){
      const childOptions={stage:'author-tools',releaseDir:release.root,...(state.authorTools.state?{resume:state.authorTools.state.path}:{outputDir:path.join(state.runRoot,'author-tools')}),...(official?{officialResource:official.path}:{}),...(review.authorTools?{reviewFile:review.authorTools}:{})};
      state.authorTools.status='running';save();
      try{const result=await authorTools(childOptions,{releaseLoader:()=>release,invoke:publicInvoke});adoptAuthor(result.stateFile);save();}
      catch(error){const file=path.join(state.runRoot,'author-tools/state.json');if(fs.existsSync(file)){adoptAuthor(bound(readSnapshot(file)));save();}throw error;}
    }
    const fixture=ownedInput(state,'migrationFixtureInput',options.migrationFixture,'migration-fixture.json.gz');
    if(state.migration.record){assert(migrationVerifier,'Original migration scope needs its shared verifier');migrationVerifier(state.migration,{state});}
    else if(state.migration.status!=='pass'&&fixture){
      if(state.migration.fixture)assert(same(state.migration.fixture,fixture),'Migration fixture changed across this run');
      const {readMigrationFixture}=await import('./support/p3-migration.mjs');readMigrationFixture(readBound(fixture,'fixed migration target data').content,{sourceInputs:candidate.sourceInputs.entries});
      state.migration={status:'not-run',fixture,diagnostics:[]};save();
      if(runMigration){
        assert(verifyMigration,'Migration pass requires its shared original-evidence verifier');let legacyV1;
        if(options.legacyCandidateDir){const old=candidateLoader(options.legacyCandidateDir,target,legacyOptions),oldOptions={...legacyOptions,candidateDir:options.legacyCandidateDir};
          if(state.legacyV1){assert(same(state.legacyV1.candidate,candidateBinding(old)),'Legacy precondition candidate changed');}
          else {const installed=await prepareLegacy({...oldOptions,evidenceDir:path.join(state.runRoot,'legacy-v1')});assert.equal(installed.state.status,'pass','Legacy precondition public installation did not pass');state.legacyV1={directory:old.directory,candidate:candidateBinding(old),state:installed.stateFile,status:'installed'};save();}
          const installed=legacyContext({...oldOptions,resume:state.legacyV1.state.path},old).state;legacyV1={candidate:old,consumer:installed,invoke:createPublicInvoke(installed,{invoke})};
        }
        const result=await runMigration({candidate,consumer,runRoot:state.runRoot,outputDir:path.join(state.runRoot,'migration'),invoke:publicInvoke,fixture,...(legacyV1?{legacyV1}:{})});assert(['pass','failed','not-run','incomplete'].includes(result?.status),'Unsupported migration scope');state.migration={...result,fixture};if(result.status==='pass')verifyMigration(state.migration,{state,candidate,consumer,legacyV1});save();
      }
    }
    if((options.providerManifest||state.providerManifestInput)&&state.providers.status!=='pass'){await providerScope(options,state,candidate,validateProviders);save();}
    if(!state.packageConsumer.scopes?.includes('evidence')&&!state.diagnostics.some(row=>row.code==='acceptance/local-evidence-not-run'))state.diagnostics.push({code:'acceptance/local-evidence-not-run',message:'Full local completion requires the existing evidence scope and an explicit --evidence-repo checkout; no worktree fallback was used.'});
    save();
  }catch(error){state.failure={code:error.code||'acceptance/all',message:error.message};state.diagnostics.push(state.failure);save();}
  return{...state,stateFile:previous,consumerStateFile:state.packageConsumer.state,authorToolsStateFile:state.authorTools.state,exitCode:state.status==='pass'?0:state.status==='failed'?1:3};
}

export function writeWorkflowOutputs(result,outputFile=process.env.GITHUB_OUTPUT){
  if(!outputFile||!result.stateFile)return;
  const rows=[['p3_state_path',result.stateFile.path],['p3_evidence_dir',result.runRoot]];
  for(const [,value] of rows)assert(typeof value==='string'&&path.isAbsolute(value)&&!/[\r\n\0]/.test(value),'Workflow output requires a single absolute path');
  fs.appendFileSync(outputFile,rows.map(([key,value])=>`${key}=${value}\n`).join(''));
}
export async function main(argv=process.argv.slice(2)){
  let options;
  try{
    options=parseOptions(argv);let dependencies={};
    if(options.stage==='all'){
      options.officialResource??=fileURLToPath(new URL('./fixtures/brands/official-resource.json',import.meta.url));
      options.migrationFixture??=fileURLToPath(new URL('./fixtures/migration/legacy-sources.v1.json.gz',import.meta.url));
    }
    if(options.stage!=='author-tools'){const {createP3ProviderDependencies}=await import('../../../scripts/aggregate-phoenix-diagram-candidate.mjs');dependencies=createP3ProviderDependencies({candidateDir:options.candidateDir});}
    if(options.stage==='all'){const {runMigrationAcceptance,verifyMigrationAcceptance}=await import('./support/p3-migration-run.mjs');Object.assign(dependencies,{runMigration:runMigrationAcceptance,verifyMigration:verifyMigrationAcceptance});}
    const result=await(options.stage==='all'?runAll(options,dependencies):options.stage==='providers'?runProviders(options,dependencies):runAuthorTools(options));
    if(options.stage!=='author-tools')writeWorkflowOutputs(result);
    process.stdout.write(JSON.stringify(result,null,2)+'\n');process.exitCode=result.exitCode;return result;
  }catch(error){const result={schemaVersion:1,tool:'phoenix-diagram',kind:options?.stage==='author-tools'||!options?KIND:ACCEPTANCE_KIND,profile:'p3',stage:options?.stage||'author-tools',status:'incomplete',deliveryStatus:'incomplete',diagnostics:[{code:error.code||'acceptance/incomplete',message:error.message}]};process.stdout.write(JSON.stringify(result,null,2)+'\n');process.exitCode=1;return result;}
}

if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))await main();
