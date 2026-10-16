'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const os=require('node:os');
const {spawnSync}=require('node:child_process');
const {createHash}=require('node:crypto');
const api=()=>import('../../scripts/aggregate-phoenix-diagram-candidate.mjs');
const identity=bytes=>({sha256:createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length});
function temp(t) {const directory=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'diagram-aggregate-unit-')));t.after(()=>fs.rmSync(directory,{recursive:true,force:true}));return directory;}
function cohort() {return{repository:'owner/repo',runId:'123',runAttempt:2,source:{commit:'a'.repeat(40),tree:'b'.repeat(40),inputSha256:'c'.repeat(64)},tgzSha256:'d'.repeat(64),candidateArchiveSha256:'e'.repeat(64),harnessSha256:'f'.repeat(64)};}
// All objects below are inert contract fixtures. They are never full platform,
// provider or candidate acceptance evidence and cannot produce handoff=passed.
test('aggregation requires four distinct targets, the exact Node18 floor and both real provider slots',async()=>{
  const {checkInputSlots}=await api();const targets=['darwin-arm64','darwin-x64','linux-x64','linux-arm64'];const input={platforms:targets.map(target=>({target})),node18:{target:'linux-x64'},providers:[{provider:'claude-code'},{provider:'codex'}]};assert.equal(checkInputSlots(input),undefined);
  assert.throws(()=>checkInputSlots({...input,platforms:input.platforms.slice(1)}),/platform|target/i);assert.throws(()=>checkInputSlots({...input,platforms:[...input.platforms.slice(0,3),input.platforms[0]]}),/platform|target/i);assert.throws(()=>checkInputSlots({...input,providers:[{provider:'codex'},{provider:'codex'}]}),/provider/i);assert.throws(()=>checkInputSlots({...input,node18:{target:'darwin-arm64'}}),/Node18|floor|target/i);
});
test('current cohort rejects another tgz, source, run attempt or harness despite success flags',async()=>{
  const {checkStateCohort}=await api(),expected=cohort(),state={schemaVersion:1,kind:'phoenix-diagram-candidate-consumer',repository:expected.repository,runId:expected.runId,runAttempt:'2',source:expected.source,candidate:{tarball:{identity:{sha256:expected.tgzSha256}},candidateArchiveSha256:expected.candidateArchiveSha256},harnessArchiveSha256:expected.harnessSha256};assert.equal(checkStateCohort(state,expected),undefined);
  for(const change of [{runAttempt:'1'},{runId:'999'},{source:{...expected.source,commit:'0'.repeat(40)}},{candidate:{...state.candidate,tarball:{identity:{sha256:'0'.repeat(64)}}}},{harnessArchiveSha256:'0'.repeat(64)}])assert.throws(()=>checkStateCohort({...state,...change},expected),/cohort|candidate|source|run|harness/i);
});
test('job proof binds the actual successful job to its current workflow and uploaded artifact',async()=>{
  const {checkJobProof}=await api(),expected=cohort(),entry={target:'linux-x64',jobId:45,artifact:{id:67,name:'phoenix-diagram-consume-linux-x64-evidence',digest:'1'.repeat(64)}};
  const job={id:45,run_id:123,run_attempt:2,status:'completed',conclusion:'success',name:'Consume the same tgz on linux-x64',head_sha:expected.source.commit,started_at:'2026-10-16T00:00:00Z',completed_at:'2026-10-16T01:00:00Z',steps:[{name:'Consume the unchanged tgz through its package entry points',conclusion:'success'},{name:'Resume the same delivery after actual image review',conclusion:'success'}]},artifact={id:67,name:entry.artifact.name,digest:`sha256:${entry.artifact.digest}`,expired:false,created_at:'2026-10-16T00:59:00Z',workflow_run:{id:123,head_sha:expected.source.commit}};
  assert.equal(checkJobProof(job,artifact,entry,expected),undefined);assert.throws(()=>checkJobProof({...job,conclusion:'failure'},artifact,entry,expected),/job|success/i);assert.throws(()=>checkJobProof({...job,run_attempt:1},artifact,entry,expected),/attempt|job/i);assert.throws(()=>checkJobProof(job,{...artifact,digest:'sha256:'+'2'.repeat(64)},entry,expected),/artifact|digest/i);assert.throws(()=>checkJobProof({...job,steps:job.steps.slice(0,1)},artifact,entry,expected),/step|review/i);
  assert.throws(()=>checkJobProof(job,{...artifact,created_at:'2026-10-15T00:00:00Z'},entry,expected),/artifact|job|time/i);
});
test('Node18 floor is the actual 18.20.8 in state, workflow and npm child evidence',async()=>{
  const {checkNode18Floor}=await api();const state={runtimeNode:'v18.20.8',environment:{proof:{node:'v18.20.8'},npmChildProof:{node:'v18.20.8'}}},phase={node:'v18.20.8'},proof={node:'v18.20.8'};assert.equal(checkNode18Floor(state,phase,proof),undefined);
  assert.throws(()=>checkNode18Floor({...state,runtimeNode:'v18.20.1'},phase,proof),/18.20.8|floor/i);assert.throws(()=>checkNode18Floor(state,phase,{node:'v25.2.1'}),/18.20.8|floor/i);
});
test('archived references rehash real zip bytes without adopting copied inode bindings',async t=>{
  const {ArchiveEvidence,checkStateSelection}=await api(),directory=temp(t),bytes=Buffer.from('{"scope":"inert archive bytes only"}\n');fs.mkdirSync(path.join(directory,'evidence'));fs.writeFileSync(path.join(directory,'evidence','receipt.json'),bytes);const archive=path.join(directory,'artifact.zip');const zip=spawnSync('zip',['-q',archive,'evidence/receipt.json'],{cwd:directory});assert.equal(zip.status,0);
  const store=new ArchiveEvidence({path:archive,identity:identity(fs.readFileSync(archive))},'evidence','/original/job');const reference={path:'/original/job/receipt.json',identity:identity(bytes),originalBinding:{inode:'a deliberately unavailable original inode'}};assert.deepEqual(store.bytes(reference),bytes);assert.throws(()=>store.bytes({...reference,identity:{...reference.identity,sha256:'0'.repeat(64)}}),/digest|bytes/i);assert.throws(()=>store.bytes({...reference,path:'/other/job/receipt.json'}),/root|path/i);assert.throws(()=>store.bytes({...reference,path:'/original/job/missing.json'}),/member|archive|missing/i);
  const emptyPrefix=new ArchiveEvidence({path:archive,identity:identity(fs.readFileSync(archive))},'','/original');assert.deepEqual(emptyPrefix.bytes({path:'/original/evidence/receipt.json',identity:identity(bytes)}),bytes);
  assert.equal(checkStateSelection(store,{state:reference},'evidence/receipt.json'),undefined);assert.throws(()=>checkStateSelection(store,{state:null},'evidence/receipt.json'),/state|reference/i);assert.throws(()=>checkStateSelection(store,{state:{...reference,identity:{...reference.identity,sha256:'0'.repeat(64)}}},'evidence/receipt.json'),/state|digest|bytes/i);
});
test('provider evidence cannot replace native session captures with arbitrary passed assertions',async()=>{
  const {checkProviderCapture}=await api();assert.throws(()=>checkProviderCapture({provider:'codex',status:'pass',assertions:{allScenarios:'pass',authenticated:true}},()=>Buffer.from('{}')),/capture|session|transcript|provider/i);
});
test('provider scenario coverage maps named plan outcomes and rejects missing or generic claims',async()=>{
  const {PROVIDER_SCENARIOS,checkProviderCoverage}=await api();assert.ok(PROVIDER_SCENARIOS.some(row=>row.id==='missing-condition' && row.kind==='clarification'));assert.ok(PROVIDER_SCENARIOS.some(row=>row.id==='stalled-repair' && row.kind==='stall'));assert.ok(PROVIDER_SCENARIOS.some(row=>row.id==='visual-pending' && row.kind==='visual-pending'));
  assert.throws(()=>checkProviderCoverage([{id:'all',status:'pass'}]),/scenario|coverage|scope/i);assert.throws(()=>checkProviderCoverage(PROVIDER_SCENARIOS.slice(1).map(row=>({id:row.id}))),/scenario|coverage|scope/i);
});
test('native provider reads and PD results require matching tool events, not assistant claims',async()=>{
  const {checkProviderCapture}=await api(),quote='A sufficiently long literal installed skill excerpt used only by this inert parser fixture.';
  const skill='/unit/.agents/skills/phoenix-diagram/SKILL.md',reference='/unit/.agents/skills/phoenix-diagram/references/delivery-contract.md';
  const rows=[{type:'thread.started',thread_id:'unit-thread'},...[[skill,'read-skill'],[reference,'read-reference']].flatMap(([file,id])=>[{type:'item.started',item:{id,type:'command_execution',command:`cat ${file}`}},{type:'item.completed',item:{id,type:'command_execution',command:`cat ${file}`,aggregated_output:quote,exit_code:0,status:'completed'}}]),{type:'item.started',item:{id:'pd',type:'command_execution',command:'/unit/.agents/skills/phoenix-diagram/scripts/phoenix-diagram doctor --json'}},{type:'item.completed',item:{id:'pd',type:'command_execution',command:'PD doctor',aggregated_output:'{"tool":"phoenix-diagram"}',exit_code:0,status:'completed'}},{type:'item.completed',item:{id:'answer',type:'agent_message',text:'{"tool":"phoenix-diagram"}'}},{type:'turn.completed',usage:{input_tokens:1,output_tokens:1}}];
  const lines=rows.map(row=>Buffer.from(JSON.stringify(row)+'\n')),raw=Buffer.concat(lines),anchors=lines.map((line,index)=>({offset:lines.slice(0,index).reduce((sum,previous)=>sum+previous.length,0),length:line.length,sha256:identity(line).sha256}));
  const transcript={path:'/unit/raw.jsonl',identity:identity(raw)},session={id:'unit-thread',format:'codex-exec-json',provider:'codex',capture:{path:'capture'},version:{path:'version'},transcript,installedReceipt:{path:'active'},loadedReads:[{file:{path:skill},quote,call:anchors[1],result:anchors[2]},{file:{path:reference},quote,call:anchors[3],result:anchors[4]}],pdCall:{call:anchors[5],result:anchors[6]}};
  const read=file=>file.path===transcript.path?raw:Buffer.from(file.path==='capture'?JSON.stringify({program:'codex',argv:['exec','--json'],cwd:'/unit',exitCode:0,stdout:transcript}):file.path==='version'?JSON.stringify({program:'codex',argv:['--version'],exitCode:0,stdout:'codex-cli 1.2.3'}):quote);
  assert.equal(checkProviderCapture(session,read).capture.program,'codex');
  assert.throws(()=>checkProviderCapture({...session,pdCall:{...session.pdCall,result:anchors[7]}},read),/tool|result|pair/i);
  assert.throws(()=>checkProviderCapture({...session,loadedReads:[{...session.loadedReads[0],result:anchors[4]},session.loadedReads[1]]},read),/tool|result|pair/i);
  const changed=structuredClone(rows);changed[5].item.command='cat /unit/phoenix-diagram/receipt.json'.padEnd(rows[5].item.command.length,' ');const changedRaw=Buffer.from(changed.map(row=>JSON.stringify(row)+'\n').join('')),changedCall={...anchors[5],sha256:identity(changedRaw.subarray(anchors[5].offset,anchors[5].offset+anchors[5].length)).sha256};
  const readChanged=file=>file.path===transcript.path?changedRaw:read(file);
  assert.throws(()=>checkProviderCapture({...session,pdCall:{call:changedCall,result:anchors[6]}},readChanged),/PD|entry|execution/i);
});
test('Reader aggregation rejects old reports without actual print/PDF scope',async()=>{
  const {checkPrintEvidence}=await api();assert.throws(()=>checkPrintEvidence(undefined,()=>{throw new Error('must not fabricate PDF');}),/print|scope/i);
  const bytes=Buffer.from('%PDF-1.7\n1 0 obj <</Type /Page>> endobj\n%%EOF\n'),rows=['dark','light'].flatMap(theme=>['still','live'].map(mode=>({theme,mode,status:'pass',motionApplicable:mode==='live',before:{mode},during:{mode},after:{mode},motionBefore:{printMedia:false,running:mode==='live'?2:0},motionDuring:{printMedia:true,running:0,staticCount:1,hiddenStatic:0},motionAfter:{printMedia:false,running:mode==='live'?2:0},nativePrint:{events:[{type:'beforeprint'},{type:'afterprint'}],pdf:{path:`/unit/${theme}-${mode}.pdf`,identity:identity(bytes),signature:'PDF',eof:true,pages:1}}})));
  const print={status:'pass',method:'cdp-print-media-and-native-print-to-pdf',observations:rows};assert.equal(checkPrintEvidence(print,()=>bytes),undefined);assert.throws(()=>checkPrintEvidence({...print,observations:rows.slice(1)},()=>bytes),/print|scope/i);assert.throws(()=>checkPrintEvidence(print,()=>Buffer.from('renamed non-PDF file')),/PDF|print|signature/i);
});
test('Reader aggregation requires the shared eight-scope interaction gate instead of status-only claims',async()=>{
  const {checkReaderInteractionEvidence}=await api();assert.equal(typeof checkReaderInteractionEvidence,'function');
  const {REQUIRED_READER_INTERACTIONS,verifyReaderInteractions}=await import('../../tools/phoenix-diagram/tests/support/export-reader.mjs');
  assert.equal(checkReaderInteractionEvidence,verifyReaderInteractions);
  assert.throws(()=>checkReaderInteractionEvidence(undefined));
  const claimed={status:'pass',observations:['camera','search-empty','search-label','search-id','search-no-match','details','keyboard','style'].map(name=>({name,status:'pass',before:{},after:{},result:{}}))};
  assert.deepEqual([...REQUIRED_READER_INTERACTIONS],claimed.observations.map(row=>row.name));assert.throws(()=>checkReaderInteractionEvidence(claimed));
  assert.throws(()=>checkReaderInteractionEvidence({status:'pass',observations:claimed.observations.slice(1)}));
});

test('Reader aggregation uses the shared native motion-suppression gate and rejects old export reports',async()=>{
  const {checkMotionSuppressionEvidence,readerEvidence}=await api(),{verifyMotionSuppression}=await import('../../tools/phoenix-diagram/tests/export-acceptance.mjs');
  assert.equal(typeof checkMotionSuppressionEvidence,'function');assert.equal(checkMotionSuppressionEvidence,verifyMotionSuppression);
  for(const value of [undefined,{status:'not-run'},{status:'pass',observations:[]}])assert.throws(()=>checkMotionSuppressionEvidence(value),/motion suppression/i);
  const artifact={path:'/unit/diagram.html',identity:{sha256:'a'.repeat(64),bytes:10}},report={schemaVersion:1,kind:'real-browser-export',status:'pass',artifact:{path:artifact.path,...artifact.identity},actualBrowserVersion:'Chrome/fixture',offline:{requested:true,status:'pass',configured:true,remoteRequests:[]},resources:{status:'pass',requests:[],loadingFailed:[]},errors:[]};
  assert.throws(()=>readerEvidence(report,{bytes(){throw new Error('must reject the missing motion matrix first');}},artifact),/motion suppression/i);
});

function stdioProviderFixture({provider='codex',shape='function-cmd',exitCode=0,status='pass',command='finalize'}={}) {
  // Synthetic native-event grammar and raw file bytes only, never host proof.
  const directory='/fixture/provider',project=directory+'/project',program=project+'/.agents/skills/phoenix-diagram/scripts/phoenix-diagram',script=directory+'/capture-stdio.mjs',nodePath=fs.realpathSync(process.execPath),quote='An installed skill excerpt long enough for the native read binding fixture.';
  const files=new Map(),put=(name,bytes)=>{bytes=Buffer.isBuffer(bytes)?bytes:Buffer.from(bytes);files.set(name,bytes);return{path:name,identity:identity(bytes)};};
  const template=fs.readFileSync(path.join(__dirname,'../../tools/phoenix-diagram/tests/support/capture-stdio.mjs')),collector=put(script,template),launcher=put(program,fs.readFileSync(path.join(__dirname,'../../skills/phoenix-diagram/scripts/phoenix-diagram'))),installedReceipt=put(project+'/.phoenix-diagram/install/active.json',JSON.stringify({schemaVersion:1,tool:'phoenix-diagram',ownership:[{path:'.agents/skills/phoenix-diagram/scripts/phoenix-diagram',identity:launcher.identity}]}));
  const candidate={path:project+'/candidate.json',identity:identity(Buffer.from('{}'))},artifact={path:project+'/diagram.html',identity:identity(Buffer.from('inert'))},receipt={schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.1.0',command,status,diagnostics:[],candidate,artifact,payload:'x'.repeat(227144)};
  const reply=put(directory+'/stdout.json',JSON.stringify(receipt)+'\n'),stderr=put(directory+'/stderr.txt',''),argv=command==='finalize'?['finalize','workflow',candidate.path,artifact.path,'--json']:['validate','workflow',candidate.path,'--json'];
  const dto={schemaVersion:1,kind:'phoenix-diagram-stdio-capture',tool:'phoenix-diagram',collector,node:{path:nodePath,version:process.version},program,argv,cwd:project,installedReceipt,launcher,exitCode,signal:null,stdout:reply,stderr};
  const commandFor=argv=>[nodePath,script,'--program',program,'--cwd',dto.cwd,'--stdout',reply.path,'--stderr',stderr.path,'--',...argv].map(value=>`'${value}'`).join(' '),cmd=commandFor(argv);
  const call=(id,command)=>provider==='claude-code'?{type:'assistant',message:{role:'assistant',content:[{type:'tool_use',id,name:'Bash',input:{command}}]}}:shape.startsWith('function')?{type:'response_item',payload:{type:'function_call',name:'exec_command',call_id:id,arguments:JSON.stringify({[shape==='function-command'?'command':'cmd']:command})}}:{type:'item.started',item:{id,type:'command_execution',command}};
  const result=(id,output,exit=0)=>provider==='claude-code'?{type:'user',message:{content:[{type:'tool_result',tool_use_id:id,content:output,is_error:exit!==0}]}}:shape.startsWith('function')?{type:'response_item',payload:{type:'function_call_output',call_id:id,output:shape==='function-native-output'?`Chunk ID: a7723f\nWall time: 0.7479 seconds\nProcess exited with code ${exit}\nOriginal token count: 604\nOutput:\n${output}\n`:JSON.stringify({output,exit_code:exit})}}:{type:'item.completed',item:{id,type:'command_execution',command:cmd,aggregated_output:output,exit_code:exit,status:'completed'}};
  const skill=put(project+'/.agents/skills/phoenix-diagram/SKILL.md',quote),reference=put(project+'/.agents/skills/phoenix-diagram/references/delivery-contract.md',quote),rows=[provider==='claude-code'?{type:'system',subtype:'init',session_id:'fixture-session'}:{type:'thread.started',thread_id:'fixture-session'},call('read-skill',`cat '${skill.path}'`),result('read-skill',quote),call('read-ref',`cat '${reference.path}'`),result('read-ref',quote),call('pd',cmd),result('pd',JSON.stringify(dto),exitCode),provider==='claude-code'?{type:'result',session_id:'fixture-session',is_error:false}:{type:'turn.completed'}];
  function build({skillOnly=false,withoutPd=false,skillInjection,beforeSkillInjection}={}) {
    const selected=rows.filter((_,index)=>!(skillOnly && [3,4].includes(index)) && !(withoutPd && [5,6].includes(index)));
    if(skillInjection)selected.splice(selected.indexOf(rows[2])+1,0,...(beforeSkillInjection?[beforeSkillInjection]:[]),skillInjection);
    const lines=selected.map(row=>Buffer.from(JSON.stringify(row)+'\n')),raw=Buffer.concat(lines),anchors=lines.map((line,index)=>({offset:lines.slice(0,index).reduce((sum,row)=>sum+row.length,0),length:line.length,sha256:identity(line).sha256})),transcript=put(directory+'/native.jsonl',raw),version=put(directory+'/version.json',JSON.stringify({program:provider==='codex'?'codex':'claude',argv:['--version'],exitCode:0,stdout:provider==='codex'?'codex-cli fixture':'Claude Code fixture'})),capture=put(directory+'/capture.json',JSON.stringify({program:provider==='codex'?'codex':'claude',argv:provider==='codex'?['exec','--json']:['--output-format','stream-json'],cwd:project,exitCode:0,stdout:transcript})),at=index=>anchors[selected.indexOf(rows[index])];
    const session={id:'fixture-session',provider,format:provider==='codex'?'codex-exec-json':'claude-stream-json',capture,version,transcript,installedReceipt,collector:{script:collector,nodePath},loadedReads:[{file:skill,quote,call:at(1),result:at(2),...(skillInjection?{injection:anchors[selected.indexOf(skillInjection)]}:{})},...(!skillOnly?[{file:reference,quote,call:at(3),result:at(4)}]:[])],pdCall:withoutPd?undefined:{call:at(5),result:at(6)}};
    return{session,proof:session.pdCall,read:ref=>{const bytes=files.get(ref.path);assert.ok(bytes,'fixture file exists: '+ref.path);return bytes;}};
  }
  return{build,rows,dto,receipt,reply,files,templateIdentity:identity(template),collector,cmd,put,replaceArgv(argv){dto.argv=argv;rows[5]=call('pd',commandFor(argv));},replaceResult(value,exit=0){rows[6]=result('pd',JSON.stringify(value),exit);}};
}
function skillInjectionFixture(provider='claude-code') {
  const f=stdioProviderFixture({provider,command:'doctor'}),read=f.build().session.loadedReads[0],body='# Phoenix Diagram\n\n'+read.quote+'\n',source='---\nname: phoenix-diagram\n---\n\n'+body;
  Object.assign(read.file,f.put(read.file.path.replace('/.agents/','/.claude/'),source));
  f.rows[1]={type:'assistant',session_id:'fixture-session',message:{role:'assistant',content:[{type:'tool_use',id:'read-skill',name:'Skill',input:{skill:'phoenix-diagram'}}]}};
  f.rows[2]={type:'user',session_id:'fixture-session',message:{role:'user',content:[{type:'tool_result',tool_use_id:'read-skill',content:'Launching skill: phoenix-diagram'}]},tool_use_result:{success:true,commandName:'phoenix-diagram'}};
  const injection={type:'user',session_id:'fixture-session',isSynthetic:true,message:{role:'user',content:[{type:'text',text:'Base directory for this skill: '+path.posix.dirname(read.file.path)+'\n\n'+body}]}};
  if(provider==='codex'){
    f.rows[1]={type:'response_item',payload:{type:'function_call',name:'Skill',call_id:'read-skill',arguments:JSON.stringify({skill:'phoenix-diagram'})}};
    f.rows[2]={type:'response_item',payload:{type:'function_call_output',call_id:'read-skill',output:JSON.stringify({output:'Launching skill: phoenix-diagram',exit_code:0})},tool_use_result:{success:true,commandName:'phoenix-diagram'}};
  }
  f.replaceArgv(['doctor','--json']);f.replaceResult(f.dto);
  return{...f,body,source,injection,build:options=>f.build({skillOnly:true,withoutPd:true,skillInjection:injection,...options})};
}
test('Claude native Skill loading binds the complete adjacent synthetic owned SKILL body',async()=>{
  const {checkProviderCapture}=await api();
  for(const mode of ['clarification','doctor','delivery']){
    const f=skillInjectionFixture(),b=f.build({skillOnly:mode!=='delivery',withoutPd:mode==='clarification'}),scenarioRows=[{id:mode==='delivery'?'intent-workflow':'missing-condition',sessionId:b.session.id}];
    assert.equal(checkProviderCapture(b.session,b.read,'claude-code',{collectorIdentity:f.templateIdentity,scenarioRows}).capture.exitCode,0,mode);
  }
});
test('native Skill loading rejects claims, altered bodies and unbound or nonadjacent host events',async()=>{
  const {checkProviderCapture}=await api();
  for(const fault of ['no-injection','wrong-skill','launching-only','ordinary-user','assistant-injection','wrong-role','call-session','result-session','injection-session','wrong-directory','other-project-file-and-injection','changed-body','frontmatter-body','intervening-event','failed-result','wrong-command-name','error-result','wrong-id','quote-only','extra-content','codex-skill']){
    const f=skillInjectionFixture(fault==='codex-skill'?'codex':'claude-code'),options={};
    if(fault==='no-injection')options.skillInjection=null;
    else if(fault==='wrong-skill')f.rows[1].message.content[0].input.skill='phoenix-ui';
    else if(fault==='launching-only')delete f.rows[2].tool_use_result;
    else if(fault==='ordinary-user')delete f.injection.isSynthetic;
    else if(fault==='assistant-injection')f.injection.type='assistant';
    else if(fault==='wrong-role')f.injection.message.role='assistant';
    else if(fault==='call-session')f.rows[1].session_id='different-session';
    else if(fault==='result-session')f.rows[2].session_id='different-session';
    else if(fault==='injection-session')f.injection.session_id='different-session';
    else if(fault==='wrong-directory')f.injection.message.content[0].text=f.injection.message.content[0].text.replace('/project/','/other-project/');
    else if(fault==='other-project-file-and-injection'){
      const read=f.build().session.loadedReads[0],previous=read.file.path;Object.assign(read.file,f.put(previous.replace('/project/','/other-project/'),f.source));
      f.injection.message.content[0].text=f.injection.message.content[0].text.replace(path.posix.dirname(previous),path.posix.dirname(read.file.path));
    }
    else if(fault==='changed-body')f.injection.message.content[0].text+='Altered rules.\n';
    else if(fault==='frontmatter-body')f.injection.message.content[0].text=f.injection.message.content[0].text.replace(f.body,f.source);
    else if(fault==='intervening-event')options.beforeSkillInjection={type:'system',subtype:'status',session_id:'fixture-session'};
    else if(fault==='failed-result')f.rows[2].tool_use_result.success=false;
    else if(fault==='wrong-command-name')f.rows[2].tool_use_result.commandName='phoenix-ui';
    else if(fault==='error-result')f.rows[2].message.content[0].is_error=true;
    else if(fault==='wrong-id')f.rows[2].message.content[0].tool_use_id='other-tool';
    else if(fault==='quote-only')f.injection.message.content[0].text=f.injection.message.content[0].text.replace(f.body,f.build().session.loadedReads[0].quote);
    else if(fault==='extra-content')f.injection.message.content.push({type:'text',text:'Unbound extra instructions.'});
    const b=f.build(options);
    assert.throws(()=>checkProviderCapture(b.session,b.read,b.session.provider,{collectorIdentity:f.templateIdentity,scenarioRows:[{id:'missing-condition',sessionId:b.session.id}]}),/skill|read|injection|pair|tool/i,fault);
  }
});
test('native Skill loading preserves nonclarification reference and PD requirements',async()=>{
  const {checkProviderCapture}=await api();
  for(const id of ['intent-workflow','repairable-diagnostic','stalled-repair','browser-unavailable','visual-pending']){
    const f=skillInjectionFixture(),b=f.build();
    assert.throws(()=>checkProviderCapture(b.session,b.read,'claude-code',{collectorIdentity:f.templateIdentity,scenarioRows:[{id,sessionId:b.session.id}]}),/capture|evidence|reference|PD/i,id);
  }
});
test('clarification-only session scope permits real SKILL reads with a doctor or no PD call',async()=>{
  const {checkProviderCapture}=await api();
  for(const provider of ['claude-code','codex'])for(const withDoctor of [false,true]){
    const f=stdioProviderFixture({provider,command:'doctor'});f.replaceArgv(['doctor','--json']);f.replaceResult(f.dto);
    const b=f.build({skillOnly:true,withoutPd:!withDoctor});if(!withDoctor)delete b.session.collector;
    const scenarioRows=['missing-date','missing-amount','missing-condition'].map(id=>({id,sessionId:b.session.id}));
    assert.equal(checkProviderCapture(b.session,b.read,provider,{collectorIdentity:f.templateIdentity,scenarioRows}).capture.exitCode,0);
  }
});
test('clarification-only scope cannot come from session claims, empty or mixed scenario rows',async()=>{
  const {checkProviderCapture}=await api();
  for(const provider of ['claude-code','codex'])for(const scenarioRows of [undefined,[],[{id:'missing-date',sessionId:'other'}],[{id:'unknown',sessionId:'fixture-session',kind:'clarification'}],...['intent-workflow','repairable-diagnostic','stalled-repair','browser-failed','visual-pending'].map(id=>[{id:'missing-date',sessionId:'fixture-session'},{id,sessionId:'fixture-session',kind:'clarification'}])]){
    const f=stdioProviderFixture({provider}),b=f.build({skillOnly:true,withoutPd:true});b.session.clarificationOnly=true;b.session.scenarios=[{id:'missing-date',kind:'clarification'}];
    assert.throws(()=>checkProviderCapture(b.session,b.read,provider,{collectorIdentity:f.templateIdentity,scenarioRows}),/capture|evidence|reference|PD/i);
  }
});
test('clarification-only scope still requires a real SKILL read and strict optional PD evidence',async()=>{
  const {checkProviderCapture}=await api();
  for(const provider of ['claude-code','codex'])for(const fault of ['missing-skill-proof','different-read','doctor-exit']){
    const f=stdioProviderFixture({provider,command:'doctor'});f.replaceArgv(['doctor','--json']);
    if(fault==='different-read'){if(provider==='claude-code')f.rows[1].message.content[0].input.command="cat '/different/NOT-SKILL.md'";else f.rows[1].payload.arguments=JSON.stringify({cmd:"cat '/different/NOT-SKILL.md'"});}
    if(fault==='doctor-exit')f.dto.exitCode=1;f.replaceResult(f.dto);
    const b=f.build({skillOnly:fault!=='missing-skill-proof',withoutPd:fault!=='doctor-exit'});if(fault==='missing-skill-proof')b.session.loadedReads=b.session.loadedReads.slice(1);
    assert.throws(()=>checkProviderCapture(b.session,b.read,provider,{collectorIdentity:f.templateIdentity,scenarioRows:[{id:'missing-date',sessionId:b.session.id}]}),/capture|evidence|skill|read|exit|inconsistent/i,provider+' '+fault);
  }
});
test('provider reply paths resolve exact relative argv from the authenticated capture cwd',async()=>{
  const {checkProviderCapture,checkProviderReply}=await api();
  for(const provider of ['claude-code','codex'])for(const command of ['validate','deliver','finalize','check','browser-check','visual-check','review-record']){
    const f=stdioProviderFixture({provider,command}),argv=['validate','deliver','finalize'].includes(command)?[command,'workflow','candidate.json',...(command==='validate'?[]:['./diagram.html']),'--json']:[command,'diagram.html','--json'];
    f.replaceArgv(argv);f.replaceResult(f.dto);const b=f.build(),capture=checkProviderCapture(b.session,b.read,provider,{collectorIdentity:f.templateIdentity});
    assert.deepEqual(checkProviderReply(capture,b.proof,f.reply,b.read),f.receipt,provider+' '+command);
  }
});
test('relative provider paths reject other projects, wrong argv slots and a different collector cwd',async()=>{
  const {checkProviderCapture,checkProviderReply}=await api();
  for(const provider of ['claude-code','codex'])for(const fault of ['parent-input','parent-output','absolute-other-project','swapped-slots','collector-cwd']){
    const f=stdioProviderFixture({provider}),argv=['finalize','workflow','candidate.json','diagram.html','--json'];
    if(fault==='parent-input')argv[2]='../other-project/candidate.json';
    else if(fault==='parent-output')argv[3]='../other-project/diagram.html';
    else if(fault==='absolute-other-project')argv[2]='/fixture/provider/other-project/candidate.json';
    else if(fault==='swapped-slots')[argv[2],argv[3]]=[argv[3],argv[2]];
    else f.dto.cwd='/fixture/provider/other-project';
    f.replaceArgv(argv);f.replaceResult(f.dto);const b=f.build();
    assert.throws(()=>{const capture=checkProviderCapture(b.session,b.read,provider,{collectorIdentity:f.templateIdentity});checkProviderReply(capture,b.proof,f.reply,b.read);},/argv|input|output|artifact|installation|binding/i,provider+' '+fault);
  }
});
test('Codex typed outputs bind the observed native process wrapper exit to the collector',async()=>{
  const {checkProviderCapture,checkProviderReply}=await api();
  for(const exitCode of [0,1]){
    const f=stdioProviderFixture({shape:'function-native-output',exitCode,status:exitCode?'failed':'pass',command:'validate'}),b=f.build(),capture=checkProviderCapture(b.session,b.read,'codex',{collectorIdentity:f.templateIdentity});
    assert.deepEqual(checkProviderReply(capture,b.proof,f.reply,b.read,{outcome:exitCode?'failure':'success'}),f.receipt);
    if(exitCode)assert.throws(()=>checkProviderReply(capture,b.proof,f.reply,b.read),/exit|success|failure/i);
  }
  for(const [nativeExit,dtoExit] of [[1,0],[0,1]]){
    const f=stdioProviderFixture({shape:'function-native-output',exitCode:dtoExit,status:dtoExit?'failed':'pass'});f.replaceResult(f.dto,nativeExit);const b=f.build();
    assert.throws(()=>checkProviderCapture(b.session,b.read,'codex',{collectorIdentity:f.templateIdentity}),/exit|inconsistent/i);
  }
});
test('Codex native process wrappers reject missing, duplicated or foreign exit prefixes',async()=>{
  const {checkProviderCapture}=await api();
  for(const fault of ['missing-exit','duplicate-exit','foreign-prefix'])for(const claimedProvider of ['codex','claude-code',undefined]){
    const f=stdioProviderFixture({shape:'function-native-output'}),output=f.rows[6].payload.output;
    f.rows[6].payload.output=fault==='missing-exit'?output.replace('Process exited with code 0\n',''):fault==='duplicate-exit'?output.replace('Process exited with code 0\n','Process exited with code 0\nProcess exited with code 1\n'):'Other output\n'+output;
    const b=f.build();b.session.provider=claimedProvider;assert.throws(()=>checkProviderCapture(b.session,b.read,'codex',{collectorIdentity:f.templateIdentity}),/exit|wrapper|inconsistent/i,fault+' '+claimedProvider);
  }
});
test('large native stdout is accepted through authenticated collector files for Claude and Codex event forms',async()=>{
  const {checkProviderCapture,checkProviderReply}=await api();assert.equal(typeof checkProviderReply,'function');
  for(const config of [{provider:'claude-code'},{shape:'function-cmd'},{shape:'function-command'},{shape:'command-execution'}]){
    const f=stdioProviderFixture(config),b=f.build(),capture=checkProviderCapture(b.session,b.read,b.session.provider,{collectorIdentity:f.templateIdentity});
    assert.deepEqual(checkProviderReply(capture,b.proof,f.reply,b.read),f.receipt);assert.ok(f.files.get(f.reply.path).length>227144);assert.ok(Buffer.byteLength(JSON.stringify(f.rows[6]))<6000);
  }
});
test('collector proof rejects modified bytes, producer/template/argv claims, wrong IDs and foreign result payloads',async()=>{
  const {checkProviderCapture,checkProviderReply}=await api();assert.equal(typeof checkProviderReply,'function');
  for(const fault of ['stdout','stdout-path','stdout-hash','argv','command','input','template','native-exit','failed-success','tool-id','tool-name','cat','foreign-result','assistant']){
    const f=stdioProviderFixture({provider:'claude-code'});
    if(fault==='stdout')f.files.set(f.reply.path,Buffer.from(JSON.stringify({...f.receipt,payload:'changed'})));
    else if(fault==='stdout-path')f.dto.stdout={...f.reply,path:'/fixture/provider/different.json'};
    else if(fault==='stdout-hash')f.dto.stdout={...f.reply,identity:{...f.reply.identity,sha256:'0'.repeat(64)}};
    else if(fault==='argv')f.dto.argv=['doctor','--json'];
    else if(fault==='command'){f.dto.argv[0]='validate';f.rows[5].message.content[0].input.command=f.cmd.replace("'finalize'","'validate'");}
    else if(fault==='input'){const previous=f.dto.argv[2];f.dto.argv[2]='/fixture/provider/project/other.json';f.rows[5].message.content[0].input.command=f.cmd.replace(previous,f.dto.argv[2]);}
    else if(fault==='template'){const forged=f.put(f.collector.path,'// untrusted replacement');f.collector.identity=forged.identity;f.dto.collector=f.collector;}
    else if(fault==='native-exit')f.dto.exitCode=7;
    else if(fault==='failed-success')f.dto.exitCode=1;
    else if(fault==='tool-id')f.rows[6].message.content[0].tool_use_id='other-tool';
    else if(fault==='tool-name')f.rows[5].message.content[0].name='Write';
    else if(fault==='cat')f.rows[5].message.content[0].input.command=`cat '${f.reply.path}' # ${f.cmd}`;
    else if(fault==='foreign-result')f.rows[6].message.content.push({type:'tool_result',tool_use_id:'other-tool',content:JSON.stringify(f.dto)});
    else f.rows[6]={type:'assistant',message:{role:'assistant',content:[{type:'text',text:JSON.stringify(f.dto)}]}};
    if(!['tool-id','assistant','foreign-result'].includes(fault))f.replaceResult(f.dto,fault==='failed-success'?1:0);
    if(fault==='foreign-result')f.rows[6].message.content[0].content='{}';
    const b=f.build();assert.throws(()=>{const capture=checkProviderCapture(b.session,b.read,b.session.provider,{collectorIdentity:f.templateIdentity});checkProviderReply(capture,b.proof,f.reply,b.read);},/collector|capture|stdout|reply|execution|native|binding|identity|tool|command|argv|template|shell|entry/i,fault);
  }
});
test('captured nonzero outcomes remain explicit failure evidence and never successful delivery',async()=>{
  const {checkProviderCapture,checkProviderReply}=await api();assert.equal(typeof checkProviderReply,'function');const f=stdioProviderFixture({exitCode:1,status:'failed',command:'validate'}),b=f.build(),capture=checkProviderCapture(b.session,b.read,b.session.provider,{collectorIdentity:f.templateIdentity});
  assert.throws(()=>checkProviderReply(capture,b.proof,f.reply,b.read),/exit|success|native|failure/i);assert.equal(checkProviderReply(capture,b.proof,f.reply,b.read,{outcome:'failure'}).status,'failed');
});
test('collector identity comes from the actual unique authenticated harness tar member',async t=>{
  const {collectorIdentityFromHarness,checkProviderCapture}=await api(),directory=temp(t),member='tools/phoenix-diagram/tests/support/capture-stdio.mjs',bytes=fs.readFileSync(path.join(__dirname,'../../',member));fs.mkdirSync(path.dirname(path.join(directory,member)),{recursive:true});fs.writeFileSync(path.join(directory,member),bytes);
  const tar=names=>{const result=spawnSync('tar',['-cf','-',...names],{cwd:directory,env:{...process.env,COPYFILE_DISABLE:'1'}});assert.equal(result.status,0);return result.stdout;};
  assert.deepEqual(collectorIdentityFromHarness(tar([member])),identity(bytes));assert.throws(()=>collectorIdentityFromHarness(tar([member,member])),/duplicate|collector/i);
  fs.writeFileSync(path.join(directory,'other.txt'),'not a collector');assert.throws(()=>collectorIdentityFromHarness(tar(['other.txt'])),/missing|collector/i);
  const f=stdioProviderFixture(),b=f.build();assert.throws(()=>checkProviderCapture(b.session,b.read,b.session.provider),/authenticated|collector/i);
});
test('inline replies still require complete bytes from the exact direct PD tool result',async()=>{
  const {checkProviderCapture,checkProviderReply}=await api();
  for(const fault of [null,'cat','native-failure','wrong-command','foreign-result']){
    const f=stdioProviderFixture({provider:'claude-code'}),direct=[f.dto.program,...f.dto.argv].map(value=>`'${value}'`).join(' ');f.rows[5].message.content[0].input.command=direct;f.rows[6].message.content[0].content=f.files.get(f.reply.path).toString('utf8');
    if(fault==='cat')f.rows[5].message.content[0].input.command=`cat '${f.reply.path}' # ${direct}`;
    else if(fault==='native-failure')f.rows[6].message.content[0].is_error=true;
    else if(fault==='wrong-command')f.rows[5].message.content[0].input.command=direct.replace("'finalize'","'doctor'");
    else if(fault==='foreign-result'){f.rows[6].message.content.push({type:'tool_result',tool_use_id:'unrelated',content:f.rows[6].message.content[0].content});f.rows[6].message.content[0].content='{}';}
    const b=f.build();delete b.session.collector;const verify=()=>checkProviderReply(checkProviderCapture(b.session,b.read,b.session.provider),b.proof,f.reply,b.read);
    if(fault)assert.throws(verify,/PD|tool|execution|reply|command|shell/i,fault);else assert.deepEqual(verify(),f.receipt);
  }
});
