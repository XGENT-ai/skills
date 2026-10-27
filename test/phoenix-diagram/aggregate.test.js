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
  const candidate={path:project+'/candidate.json',identity:identity(Buffer.from('{}'))},artifact={path:project+'/diagram.html',identity:identity(Buffer.from('inert'))},receipt={schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command,status,diagnostics:[],candidate,artifact,payload:'x'.repeat(227144)};
  const reply=put(directory+'/stdout.json',JSON.stringify(receipt)+'\n'),stderr=put(directory+'/stderr.txt',''),argv=command==='finalize'?['finalize','workflow',candidate.path,artifact.path,'--json']:['validate','workflow',candidate.path,'--json'];
  const dto={schemaVersion:1,kind:'phoenix-diagram-stdio-capture',tool:'phoenix-diagram',collector,node:{path:nodePath,version:process.version},program,argv,cwd:project,installedReceipt,launcher,exitCode,signal:null,stdout:reply,stderr};
  const commandFor=argv=>[nodePath,script,'--program',program,'--cwd',dto.cwd,'--stdout',reply.path,'--stderr',stderr.path,'--',...argv].map(value=>`'${value}'`).join(' '),cmd=commandFor(argv);
  // Codex unified exec records each command as `/bin/zsh -c "<literal>"`.
  const shell=command=>shape==='command-execution-shell'?`/bin/zsh -c "${command}"`:command;
  const call=(id,command)=>provider==='claude-code'?{type:'assistant',message:{role:'assistant',content:[{type:'tool_use',id,name:'Bash',input:{command}}]}}:shape.startsWith('function')?{type:'response_item',payload:{type:'function_call',name:'exec_command',call_id:id,arguments:JSON.stringify({[shape==='function-command'?'command':'cmd']:command})}}:{type:'item.started',item:{id,type:'command_execution',command:shell(command)}};
  const result=(id,output,exit=0)=>provider==='claude-code'?{type:'user',message:{content:[{type:'tool_result',tool_use_id:id,content:output,is_error:exit!==0}]}}:shape.startsWith('function')?{type:'response_item',payload:{type:'function_call_output',call_id:id,output:shape==='function-native-output'?`Chunk ID: a7723f\nWall time: 0.7479 seconds\nProcess exited with code ${exit}\nOriginal token count: 604\nOutput:\n${output}\n`:JSON.stringify({output,exit_code:exit})}}:{type:'item.completed',item:{id,type:'command_execution',command:shell(cmd),aggregated_output:output,exit_code:exit,status:'completed'}};
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
function asyncStdioProviderFixture({pollCount=1,exitCode=0,namespaced=false,omitChars=false}={}) {
  const f=stdioProviderFixture({shape:'function-native-output',exitCode,status:exitCode?'failed':'pass'}),sessionId=46683;
  const running=()=>`Chunk ID: 0069c8\nWall time: 10.0026 seconds\nProcess running with session ID ${sessionId}\nOriginal token count: 0\nOutput:\n`;
  const terminal=structuredClone(f.rows[6]),polls=[];
  f.rows[0]={type:'session_meta',payload:{id:'fixture-session'}};
  f.rows[5].payload.name=namespaced?'functions.exec_command':'exec_command';
  f.rows[6].payload.output=running();
  for(let index=0;index<pollCount;index++) {
    const id=`poll-${index}`,call={type:'response_item',payload:{type:'function_call',name:namespaced?'functions.write_stdin':'write_stdin',call_id:id,arguments:JSON.stringify({session_id:sessionId,...(!omitChars?{chars:''}:{}),yield_time_ms:30000})}},result=structuredClone(terminal);
    result.payload.call_id=id;if(index<pollCount-1)result.payload.output=running();
    polls.push({call,result});f.rows.splice(f.rows.length-1,0,call,result);
  }
  f.rows[f.rows.length-1]={type:'event_msg',payload:{type:'task_complete'}};
  return{...f,polls,sessionId,build(){
    const b=f.build(),raw=b.read(b.session.transcript),lines=raw.toString('utf8').trimEnd().split('\n').map(line=>Buffer.from(line+'\n')),at=row=>{const index=f.rows.indexOf(row);return{offset:lines.slice(0,index).reduce((sum,line)=>sum+line.length,0),length:lines[index].length,sha256:identity(lines[index]).sha256};};
    b.proof.polls=polls.map(({call,result})=>({call:at(call),result:at(result),toolUseId:call.payload.call_id}));
    b.proof.toolUseId='pd';b.session.format='codex-session-json';
    const capture=JSON.parse(b.read(b.session.capture));capture.nativeSessionFile=b.session.transcript.path;
    b.session.capture=f.put(b.session.capture.path,JSON.stringify(capture));return b;
  }};
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
test('Claude Skill args bind only the exact native argument suffix after the complete owned body',async()=>{
  const {checkProviderCapture}=await api();
  for(const args of ['画账号恢复流程，部分条件未说明。','原题的压缩改写\n保留原参数尾换行\n']) {
    const f=skillInjectionFixture();f.rows[1].message.content[0].input.args=args;
    f.injection.message.content[0].text+='\n\nARGUMENTS: '+args;
    const b=f.build();assert.equal(checkProviderCapture(b.session,b.read,'claude-code',{collectorIdentity:f.templateIdentity,scenarioRows:[{id:'missing-condition',sessionId:b.session.id}]}).capture.exitCode,0);
  }
});
test('Claude Skill args reject invalid input shapes, altered suffixes and trimmed owned bodies',async t=>{
  const {checkProviderCapture}=await api();
  for(const fault of ['empty','null','number','array','object','extra-key','different-args','missing-suffix','extra-suffix','newline-count','trim-body'])await t.test(fault,()=>{
    const f=skillInjectionFixture(),input=f.rows[1].message.content[0].input,args='原条件仍需澄清';input.args=args;
    f.injection.message.content[0].text+='\n\nARGUMENTS: '+args;
    if(fault==='empty')input.args='';
    else if(fault==='null')input.args=null;
    else if(fault==='number')input.args=1;
    else if(fault==='array')input.args=[args];
    else if(fault==='object')input.args={text:args};
    else if(fault==='extra-key')input.extra='unbound input';
    else if(fault==='different-args')input.args='another original tool argument';
    else if(fault==='missing-suffix')f.injection.message.content[0].text=f.injection.message.content[0].text.replace('\n\nARGUMENTS: '+args,'');
    else if(fault==='extra-suffix')f.injection.message.content[0].text+=' unbound addition';
    else if(fault==='newline-count')f.injection.message.content[0].text=f.injection.message.content[0].text.replace('\n\nARGUMENTS: ','\nARGUMENTS: ');
    else if(fault==='trim-body')f.injection.message.content[0].text=f.injection.message.content[0].text.replace(f.body+'\n\nARGUMENTS: ',f.body.trimEnd()+'\n\nARGUMENTS: ');
    const b=f.build();assert.throws(()=>checkProviderCapture(b.session,b.read,'claude-code',{collectorIdentity:f.templateIdentity,scenarioRows:[{id:'missing-condition',sessionId:b.session.id}]}),/Skill|injection|body|read/i);
  });
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
test('P2 repair and stall sessions need only the real SKILL read but keep their PD call',async()=>{
  const {checkProviderCapture}=await api();
  for(const provider of ['claude-code','codex'])for(const ids of [['stalled-repair'],['repairable-diagnostic'],['missing-date','stalled-repair']]){
    const f=stdioProviderFixture({provider}),b=f.build({skillOnly:true}),scenarioRows=ids.map(id=>({id,sessionId:b.session.id}));
    assert.equal(checkProviderCapture(b.session,b.read,provider,{collectorIdentity:f.templateIdentity,scenarioRows,profile:'p2'}).capture.exitCode,0,ids.join());
    assert.throws(()=>checkProviderCapture(b.session,b.read,provider,{collectorIdentity:f.templateIdentity,scenarioRows}),/evidence|reference/i,'p1 '+ids.join());
    const noPd=f.build({skillOnly:true,withoutPd:true});delete noPd.session.collector;
    assert.throws(()=>checkProviderCapture(noPd.session,noPd.read,provider,{collectorIdentity:f.templateIdentity,scenarioRows:ids.map(id=>({id,sessionId:noPd.session.id})),profile:'p2'}),/capture|evidence/i,'no PD '+ids.join());
  }
  for(const id of ['intent-workflow','browser-failed','visual-pending']){
    const f=stdioProviderFixture(),b=f.build({skillOnly:true});
    assert.throws(()=>checkProviderCapture(b.session,b.read,'codex',{collectorIdentity:f.templateIdentity,scenarioRows:[{id:'stalled-repair',sessionId:b.session.id},{id,sessionId:b.session.id}],profile:'p2'}),/evidence|reference/i,id);
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
test('async collector proof preserves the producer and every read-only poll ID through terminal DTO binding',async()=>{
  const {checkProviderCapture,checkProviderReply}=await api();
  for(const config of [{pollCount:1},{pollCount:2},{pollCount:2,namespaced:true},{pollCount:2,omitChars:true},{exitCode:1}]){
    const f=asyncStdioProviderFixture(config),b=f.build(),capture=checkProviderCapture(b.session,b.read,'codex',{collectorIdentity:f.templateIdentity});
    assert.deepEqual(checkProviderReply(capture,b.proof,f.reply,b.read,{outcome:config.exitCode?'failure':'success'}),f.receipt);
    assert.equal(b.proof.toolUseId,'pd');assert.equal(b.proof.polls.length,config.pollCount || 1);
    if(config.exitCode)assert.throws(()=>checkProviderReply(capture,b.proof,f.reply,b.read),/exit|success|failure/i);
  }
});
test('async collector proof rejects foreign, ambiguous, mutating and omitted process continuations',async t=>{
  const {checkProviderCapture,checkProviderReply}=await api();
  for(const fault of ['unknown-session','session-collision','duplicate-call','duplicate-result','missing-poll','extra-poll','stdin','null-chars','number-chars','actor','wrong-result-id','cross-id','running-dto','terminal-session','duplicate-dto','other-command','other-collector','stdout','original-bytes'])await t.test(fault,()=>{
    const f=asyncStdioProviderFixture({pollCount:2}),first=f.polls[0],last=f.polls[1];
    if(fault==='unknown-session')first.call.payload.arguments=JSON.stringify({session_id:46684,chars:''});
    else if(fault==='session-collision'){const call=structuredClone(f.rows[5]),result=structuredClone(f.rows[6]);call.payload.call_id=result.payload.call_id='another-producer';f.rows.splice(7,0,call,result);}
    else if(fault==='duplicate-call')f.rows.splice(7,0,structuredClone(first.call));
    else if(fault==='duplicate-result')f.rows.splice(7,0,structuredClone(first.result));
    else if(fault==='extra-poll'){const call=structuredClone(first.call),result=structuredClone(first.result);call.payload.call_id=result.payload.call_id='unclaimed-poll';f.rows.splice(7,0,call,result);}
    else if(fault==='stdin')first.call.payload.arguments=JSON.stringify({session_id:46683,chars:'write to the running process\n'});
    else if(fault==='null-chars')first.call.payload.arguments=JSON.stringify({session_id:46683,chars:null});
    else if(fault==='number-chars')first.call.payload.arguments=JSON.stringify({session_id:46683,chars:0});
    else if(fault==='actor')first.call.payload.name='functions.write_stdin';
    else if(fault==='wrong-result-id')last.result.payload.call_id='another-native-tool-id';
    else if(fault==='running-dto')f.rows[6].payload.output+=JSON.stringify(f.dto)+'\n';
    else if(fault==='terminal-session')first.result.payload.output=first.result.payload.output.replace('session ID 46683','session ID 46684');
    else if(fault==='duplicate-dto')last.result.payload.output+=JSON.stringify(f.dto)+'\n';
    else if(fault==='other-command'){f.dto.argv[0]='validate';last.result.payload.output=last.result.payload.output.replace("\"finalize\"","\"validate\"");}
    else if(fault==='other-collector'){f.dto.collector={...f.collector,path:'/fixture/other-collector.mjs'};last.result.payload.output=last.result.payload.output.replace(f.collector.path,f.dto.collector.path);}
    else if(fault==='stdout')f.files.set(f.reply.path,Buffer.from('changed original CLI stdout'));
    const b=f.build();
    if(fault==='missing-poll')b.proof.polls.shift();
    if(fault==='cross-id'){b.proof.result=b.proof.polls.at(-1).result;delete b.proof.polls;}
    if(fault==='original-bytes')b.proof.polls[0].result.sha256='0'.repeat(64);
    assert.throws(()=>{const capture=checkProviderCapture(b.session,b.read,'codex',{collectorIdentity:f.templateIdentity});checkProviderReply(capture,b.proof,f.reply,b.read);},/async|native|collector|pair|tool|anchor|stdout|digest|bytes|command|argv|binding/i);
  });
});
test('async collector support does not admit interrupted or incomplete provider sessions',async()=>{
  const {checkProviderCapture}=await api(),f=asyncStdioProviderFixture(),b=f.build(),capture=JSON.parse(b.read(b.session.capture));
  capture.exitCode=null;capture.signal='SIGKILL';capture.timedOut=false;b.session.capture=f.put(b.session.capture.path,JSON.stringify(capture));
  assert.throws(()=>checkProviderCapture(b.session,b.read,'codex',{collectorIdentity:f.templateIdentity}),/CLI\/version|missing|session/i);
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
  for(const config of [{provider:'claude-code'},{shape:'function-cmd'},{shape:'function-command'},{shape:'command-execution'},{shape:'command-execution-shell'}]){
    const f=stdioProviderFixture(config),b=f.build(),capture=checkProviderCapture(b.session,b.read,b.session.provider,{collectorIdentity:f.templateIdentity});
    assert.deepEqual(checkProviderReply(capture,b.proof,f.reply,b.read),f.receipt);assert.ok(f.files.get(f.reply.path).length>227144);assert.ok(Buffer.byteLength(JSON.stringify(f.rows[6]))<6000);
  }
});
test('a shell-wrapped Codex command counts only when its one literal argument is the collector call',async()=>{
  const {checkProviderCapture}=await api();
  for(const wrap of [cmd=>`/bin/zsh -c "${cmd}; rm -f x"`,cmd=>`/bin/zsh -c "${cmd}" extra`,cmd=>`/bin/zsh -lc "${cmd}"`,cmd=>`/usr/bin/env -c "${cmd}"`]){
    const f=stdioProviderFixture({shape:'command-execution-shell'});f.rows[5].item.command=wrap(f.cmd);f.rows[6].item.command=wrap(f.cmd);
    const b=f.build();assert.throws(()=>checkProviderCapture(b.session,b.read,'codex',{collectorIdentity:f.templateIdentity}),/collector|PD|shell|execution/i,wrap('x'));
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
test('P2 profile covers fifteen cases and per-provider host rows instead of the P1 matrix',async()=>{
  const {PROFILE_CASES,PROVIDER_SCENARIOS,P2_PROVIDER_SCENARIOS,checkProviderCoverage,checkInputSlots}=await api();
  assert.equal(PROFILE_CASES.p1.length,10);assert.equal(PROFILE_CASES.p2.length,15);assert.ok(PROFILE_CASES.p2.includes('erd:phoenix-v1'));assert.ok(!PROFILE_CASES.p2.includes('erd:legacy-2.17'));
  assert.equal(PROVIDER_SCENARIOS.length,22);assert.ok(!PROVIDER_SCENARIOS.some(row=>row.id==='intent-erd'));
  const claude=P2_PROVIDER_SCENARIOS['claude-code'],codex=P2_PROVIDER_SCENARIOS.codex;assert.equal(claude.length,31);assert.equal(codex.length,16);
  assert.deepEqual(claude.filter(row=>row.id.startsWith('legacy-')).map(row=>row.id),['legacy-lifecycle']);
  for(const id of ['intent-workflow','mermaid-sequence','stalled-repair','repo-dirty','locale-zh-tw','reference-31-json'])assert.ok(codex.some(row=>row.id===id),id);assert.ok(!codex.some(row=>row.id==='missing-date'));
  assert.equal(checkProviderCoverage(codex.map(row=>({id:row.id})),codex),undefined);assert.throws(()=>checkProviderCoverage(PROVIDER_SCENARIOS.map(row=>({id:row.id})),codex),/coverage/);assert.throws(()=>checkProviderCoverage(codex.slice(1).map(row=>({id:row.id})),codex),/coverage/);
  const targets=['darwin-arm64','darwin-x64','linux-x64','linux-arm64'],input={platforms:targets.map(target=>({target})),node18:{target:'linux-x64'},providers:[{provider:'claude-code'},{provider:'codex'}]};
  assert.throws(()=>checkInputSlots(input,'p2'),/local darwin-arm64/);assert.equal(checkInputSlots({...input,local:{target:'darwin-arm64',runRoot:'/run',state:{path:'/run/consumer-state.json'}}},'p2'),undefined);
});
test('D10: Codex covers either its sampled rows or, after a divergence, every Claude Code row',async()=>{
  const {P2_PROVIDER_SCENARIOS,p2ProviderScenarios,checkProviderCoverage}=await api(),claude=P2_PROVIDER_SCENARIOS['claude-code'],codex=P2_PROVIDER_SCENARIOS.codex,ids=rows=>rows.map(row=>({id:row.id}));
  assert.equal(p2ProviderScenarios('codex',ids(codex)),codex);assert.equal(checkProviderCoverage(ids(codex),p2ProviderScenarios('codex',ids(codex))),undefined);
  assert.equal(p2ProviderScenarios('codex',ids(claude)),claude);assert.equal(checkProviderCoverage(ids(claude),p2ProviderScenarios('codex',ids(claude))),undefined);
  // A partial expansion is neither set: it must still fail coverage.
  const partial=ids([...codex,claude.find(row=>row.id==='intent-architecture')]);assert.throws(()=>checkProviderCoverage(partial,p2ProviderScenarios('codex',partial)),/coverage/);
  assert.equal(p2ProviderScenarios('claude-code',ids(codex)),claude);
});
test('automatic-only platform jobs need the consume step but not the review step',async()=>{
  const {checkJobProof}=await api(),expected=cohort(),entry={target:'linux-x64',jobId:45,artifact:{id:67,name:'phoenix-diagram-consume-linux-x64-evidence',digest:'1'.repeat(64)}};
  const job={id:45,run_id:123,run_attempt:2,status:'completed',conclusion:'success',name:'Consume the same tgz on linux-x64',head_sha:expected.source.commit,started_at:'2026-10-16T00:00:00Z',completed_at:'2026-10-16T01:00:00Z',steps:[{name:'Consume the unchanged tgz through its package entry points',conclusion:'success'},{name:'Resume the same delivery after actual image review',conclusion:'skipped'}]},artifact={id:67,name:entry.artifact.name,digest:`sha256:${entry.artifact.digest}`,expired:false,created_at:'2026-10-16T00:59:00Z',workflow_run:{id:123,head_sha:expected.source.commit}};
  assert.equal(checkJobProof(job,artifact,entry,expected,{reviewed:false}),undefined);assert.throws(()=>checkJobProof(job,artifact,entry,expected),/step|review/i);
  assert.throws(()=>checkJobProof({...job,steps:[{...job.steps[0],conclusion:'failure'}]},artifact,entry,expected,{reviewed:false}),/step/i);
});
test('local directory evidence rehashes original files in place and rejects foreign, changed or linked paths',async t=>{
  const {DirectoryEvidence}=await api(),directory=temp(t),bytes=Buffer.from('{"scope":"inert local bytes only"}\n'),file=path.join(directory,'state.json');fs.writeFileSync(file,bytes);
  const store=new DirectoryEvidence(directory),reference={path:file,identity:identity(bytes)};assert.deepEqual(store.json(reference),{scope:'inert local bytes only'});
  assert.throws(()=>store.bytes({...reference,identity:{...reference.identity,sha256:'0'.repeat(64)}}),/digest|bytes/i);assert.throws(()=>store.bytes({path:'/etc/hosts',identity:reference.identity}),/root|path/i);
  fs.symlinkSync(file,path.join(directory,'alias.json'));assert.throws(()=>store.bytes({path:path.join(directory,'alias.json'),identity:reference.identity}),/root|path|symlink/i);
  assert.throws(()=>new DirectoryEvidence('relative/run'),/canonical|ENOENT/i);
});
test('host sessions on a development package count only with the final bundle and engine source',async()=>{
  const {checkDevelopmentPackage}=await api(),expected={...cohort(),npmPackageVersion:'0.7.4',bundleSha256:'9'.repeat(64)};
  const version={schemaVersion:1,distribution:'development',tool:'phoenix-diagram',toolVersion:'0.2.0',npmPackageVersion:'0.7.4',sourceInputSha256:expected.source.inputSha256,bundleSha256:expected.bundleSha256,engines:{'darwin-arm64':{asset:'phoenix-diagram-darwin-arm64',size:1,sha256:'8'.repeat(64)}}};
  assert.equal(checkDevelopmentPackage(version,expected),version);
  for(const change of [{bundleSha256:'0'.repeat(64)},{sourceInputSha256:'0'.repeat(64)},{distribution:'release'},{toolVersion:'0.1.0'},{engines:{}}])assert.throws(()=>checkDevelopmentPackage({...version,...change},expected),/bundle|engine source/,JSON.stringify(change));
});
function sameCandidatePackageFixture() {
  const expected={...cohort(),npmPackageVersion:'0.7.4',bundleSha256:'9'.repeat(64)};
  const engines=Object.fromEntries(
    ['darwin-arm64','darwin-x64','linux-x64','linux-arm64'].map((target,index)=>[target,{asset:`phoenix-diagram-${target}`,size:index+1,sha256:String(index+1).repeat(64),url:`http://127.0.0.1:48765/phoenix-diagram-${target}`}])
  );
  const version={schemaVersion:1,distribution:'candidate',tool:'phoenix-diagram',toolVersion:'0.2.0',npmPackageVersion:expected.npmPackageVersion,sourceCommit:expected.source.commit,sourceInputSha256:expected.source.inputSha256,bundleSha256:expected.bundleSha256,inputContractSchema:1,receiptSchema:1,viewerRevision:'phoenix-v2',engines};
  const versionBytes=Buffer.from(JSON.stringify(version,null,2)+'\n'),tgzBytes=Buffer.from('Inert unit archive bytes; this fixture is not native/candidate acceptance evidence.');
  Object.assign(expected,{engines,tgzSha256:identity(tgzBytes).sha256,versionSha256:identity(versionBytes).sha256});
  const files=new Map(),put=(name,bytes)=>{files.set(name,bytes);return{path:name,identity:identity(bytes)};};
  const record={source:structuredClone(expected.source),candidateSha256:expected.tgzSha256,package:{version:put('/archive/VERSION.json',versionBytes),tarball:put('/archive/package.tgz',tgzBytes)}};
  // The aggregate supplies this view only from its successful validateCandidate,
  // never from fields of the provider record. Here it is an inert boundary fixture.
  const candidate={source:structuredClone(expected.source),version:structuredClone(version),tgz:{identity:identity(tgzBytes),content:tgzBytes},files:new Map([['release-dir/VERSION.json',{identity:identity(versionBytes),content:versionBytes}]])};
  const context={record,candidate,bytesFor:reference=>{const bytes=files.get(reference.path);assert.ok(bytes);assert.deepEqual(identity(bytes),reference.identity);return bytes;}};
  return{expected,version,context,files,put};
}
test('same-candidate provider packages retain their actual candidate VERSION and exact authenticated tgz',async()=>{
  const {checkDevelopmentPackage}=await api(),f=sameCandidatePackageFixture();
  assert.equal(checkDevelopmentPackage(f.version,f.expected,f.context),f.version);
  assert.equal(f.version.distribution,'candidate');
  assert.throws(()=>checkDevelopmentPackage(f.version,f.expected),/candidate|authenticated/i,'source/bundle claims without the authenticated candidate are insufficient');
});
test('same-candidate provider packages reject side archives, relabeling and altered VERSION bytes',async()=>{
  const {checkDevelopmentPackage}=await api();
  for(const fault of ['other-tgz','other-tgz-and-claim','missing-tgz','changed-version','reformatted-version','development-label','foreign-source','foreign-record-sha']) {
    const f=sameCandidatePackageFixture(),{record}=f.context;
    if(fault==='other-tgz' || fault==='other-tgz-and-claim') {
      record.package.tarball=f.put('/archive/other.tgz',Buffer.from('Another archive with the same source, bundle and VERSION claims.'));
      if(fault==='other-tgz-and-claim')record.candidateSha256=record.package.tarball.identity.sha256;
    } else if(fault==='missing-tgz')delete record.package.tarball;
    else if(fault==='changed-version') {f.version.extra='Unbound VERSION field';record.package.version=f.put('/archive/changed-VERSION.json',Buffer.from(JSON.stringify(f.version)));}
    else if(fault==='reformatted-version')record.package.version=f.put('/archive/reformatted-VERSION.json',Buffer.from(JSON.stringify(f.version)));
    else if(fault==='development-label') {f.version.distribution='development';record.package.version=f.put('/archive/relabeled-VERSION.json',Buffer.from(JSON.stringify(f.version)));}
    else if(fault==='foreign-source')record.source.tree='0'.repeat(40);
    else record.candidateSha256='0'.repeat(64);
    assert.throws(()=>checkDevelopmentPackage(f.version,f.expected,f.context),/candidate|VERSION|tgz|source/i,fault);
  }
});
test('same-candidate provider packages bind all four engines and the validated cohort snapshots',async()=>{
  const {checkDevelopmentPackage}=await api();
  for(const fault of ['non-host-engine','missing-engine','extra-engine','cohort-engine','cohort-version','cohort-source','changed-candidate-tgz']) {
    const f=sameCandidatePackageFixture();
    if(fault==='non-host-engine')f.version.engines['linux-arm64'].sha256='0'.repeat(64);
    else if(fault==='missing-engine')delete f.version.engines['linux-arm64'];
    else if(fault==='extra-engine')f.version.engines['win32-x64']={...f.version.engines['linux-x64']};
    else if(fault==='cohort-engine') {f.expected.engines=structuredClone(f.expected.engines);f.expected.engines['linux-arm64'].size++;}
    else if(fault==='cohort-version')f.expected.versionSha256='0'.repeat(64);
    else if(fault==='cohort-source')f.context.candidate.source.commit='0'.repeat(40);
    else f.context.candidate.tgz.content=Buffer.from('Changed after the validated candidate view was formed.');
    assert.throws(()=>checkDevelopmentPackage(f.version,f.expected,f.context),/candidate|VERSION|engine|cohort|tgz/i,fault);
  }
});
test('P2 delivery rows require their evidence, timing, language and unchanged-reference facts',async()=>{
  const {p2Outcome}=await api(),files=new Map(),ref=(name,bytes)=>{files.set(name,bytes);return{path:name,identity:identity(bytes)};},store={bytes:reference=>files.get(reference.path)};
  const make=({candidate,html='<svg></svg>',evidence={status:'none'},delivery={status:'none'},locale={requested:'en',resolved:'en',fallback:false}})=>({evidence,locale,candidate:ref(`/c-${files.size}`,Buffer.from(JSON.stringify(candidate))),artifact:ref(`/a-${files.size}`,Buffer.from(html)),stages:[{},{receipt:{evidence:delivery}}]});
  const verified=make({candidate:{meta:{}},evidence:{status:'verified'},delivery:{status:'verified',counts:{verified:2},repository:{url:'https://github.com/o/r'}}});
  assert.equal(p2Outcome({evidence:'verified'},verified,{},store,''),undefined);assert.throws(()=>p2Outcome({evidence:'verified'},make({candidate:{meta:{}},evidence:{status:'partial'}}),{},store,''),/verified/);
  const dirty=make({candidate:{meta:{}},evidence:{status:'partial'},delivery:{status:'partial',unverified:[{pointer:'/components/0/sources/0',nodeId:'gate',reason:'worktree-differs'}]}});
  assert.equal(p2Outcome({evidence:'worktree-differs'},dirty,{quote:'src/input.rs 有未提交改动'},store,'注意：src/input.rs 有未提交改动，来源待核实。'),undefined);assert.throws(()=>p2Outcome({evidence:'worktree-differs'},dirty,{quote:'src/input.rs 有未提交改动'},store,'已全部核对。'),/reply/);
  const day=make({candidate:{events:[{precision:'day'}]},html:'<svg data-timeline-evidence="observed"></svg>'});assert.equal(p2Outcome({timing:'day'},day,{},store,''),undefined);assert.throws(()=>p2Outcome({timing:'day'},make({candidate:{events:[{}]},html:'<svg data-timeline-evidence="observed"></svg>'}),{},store,''),/day-precision/);
  const sketch=make({candidate:{},html:'<svg data-waterfall-evidence="illustrative"></svg>'});assert.equal(p2Outcome({timing:'illustrative'},sketch,{quote:'示意'},store,'耗时为示意'),undefined);assert.throws(()=>p2Outcome({timing:'illustrative'},sketch,{quote:'示意'},store,'实测耗时'),/Illustrative/);
  const zh=make({candidate:{meta:{locale:'zh-TW'}},locale:{requested:'zh-TW',resolved:'zh-TW',fallback:false}});assert.equal(p2Outcome({locale:'zh-TW'},zh,{},store,''),undefined);assert.throws(()=>p2Outcome({locale:'zh-TW'},make({candidate:{meta:{locale:'zh-TW'}},locale:{requested:'zh-TW',resolved:'en',fallback:true}}),{},store,''),/language/);
  const original={schema_version:1,diagram_type:'workflow',meta:{title:'Reference'}},reference=make({candidate:original}),kept={original:ref('/original.json',Buffer.from(JSON.stringify(original)))};
  assert.equal(p2Outcome({unchangedOriginal:true},reference,kept,store,''),undefined);assert.throws(()=>p2Outcome({unchangedOriginal:true},make({candidate:{...original,meta:{...original.meta,input_contract:'phoenix-v1'}}}),kept,store,''),/rewritten/);
});
test('a host record on an earlier engine of the same bundle is only a reuse source, never a strict binding',async()=>{
  const {checkReusedPackage}=await api(),expected={...cohort(),npmPackageVersion:'0.7.4',bundleSha256:'9'.repeat(64)};
  const version={schemaVersion:1,distribution:'development',tool:'phoenix-diagram',toolVersion:'0.2.0',npmPackageVersion:'0.7.4',sourceInputSha256:'7'.repeat(64),bundleSha256:expected.bundleSha256,engines:{'darwin-arm64':{asset:'phoenix-diagram-darwin-arm64',size:1,sha256:'8'.repeat(64)}}};
  assert.equal(checkReusedPackage(version,expected),version);
  for(const change of [{sourceInputSha256:expected.source.inputSha256},{sourceInputSha256:'x'},{bundleSha256:'0'.repeat(64)},{npmPackageVersion:'0.7.5'},{distribution:'release'},{toolVersion:'0.1.0'},{engines:{}}])assert.throws(()=>checkReusedPackage({...version,...change},expected),/Reused development package/,JSON.stringify(change));
});
test('rerun host rows replace only themselves; every other planned row comes from the complete earlier record',async()=>{
  const {P2_PROVIDER_SCENARIOS,p2ReuseRows}=await api(),claude=P2_PROVIDER_SCENARIOS['claude-code'],codex=P2_PROVIDER_SCENARIOS.codex,ids=rows=>rows.map(row=>({id:row.id}));
  const rerun=['intent-erd','mermaid-er','intent-timeline','repo-timeline'].map(id=>({id}));
  const result=p2ReuseRows('claude-code',rerun,ids(claude));
  assert.equal(result.scenarios,claude);assert.deepEqual(result.reused,claude.map(row=>row.id).filter(id=>!rerun.some(row=>row.id===id)));
  // D10: an earlier Codex record that already covers every Claude Code row keeps that scope.
  assert.equal(p2ReuseRows('codex',rerun,ids(claude)).scenarios,claude);assert.equal(p2ReuseRows('codex',rerun,ids(codex)).reused.length,codex.length-4);
  assert.throws(()=>p2ReuseRows('claude-code',[],ids(claude)),/Rerun host rows/);
  assert.throws(()=>p2ReuseRows('claude-code',[...rerun,{id:'intent-erd'}],ids(claude)),/Rerun host rows/);
  assert.throws(()=>p2ReuseRows('codex',[{id:'intent-architecture'}],ids(codex)),/Rerun host rows/);
  assert.throws(()=>p2ReuseRows('claude-code',rerun,ids(claude.slice(1))),/coverage/);
});
test('the diagram SVG is the single svg element of a delivered page',async()=>{
  const {diagramSvg}=await api();
  assert.equal(diagramSvg('<style>svg{}</style><div><svg viewBox="0 0 1 1"><g/></svg></div><script>{}</script>'),'<svg viewBox="0 0 1 1"><g/></svg>');
  for(const html of ['<div></div>','<svg></svg><svg></svg>','<svg>'])assert.throws(()=>diagramSvg(html),/single diagram SVG/,html);
});
test('a reused host row needs the final engine to reproduce its diagram and failed validations',async()=>{
  const {checkReplay}=await api(),files=new Map(),ref=(name,value)=>{const bytes=Buffer.from(typeof value==='string'?value:JSON.stringify(value));files.set(name,bytes);return{path:name,identity:identity(bytes)};},store={bytes:reference=>{const bytes=files.get(reference.path);assert.ok(bytes && identity(bytes).sha256===reference.identity.sha256,'store digest');return bytes;},json(reference){return JSON.parse(this.bytes(reference).toString('utf8'));}};
  const final={...cohort(),engines:{'darwin-arm64':{asset:'phoenix-diagram-darwin-arm64',size:9,sha256:'8'.repeat(64)}}},engine={sourceInputSha256:final.source.inputSha256,binary:{sha256:'8'.repeat(64),bytes:9}};
  const config={diagramType:'architecture',inputContract:'phoenix-v1',quality:'showcase',viewerRevision:'phoenix-v2'},candidate=identity(Buffer.from('candidate')),snapshot=identity(Buffer.from('snapshot'));
  const original=ref('original.html','<style>.a{}</style><svg id="d"><g/></svg><script>{"engine":"old"}</script>');
  const receipt=(command,fields)=>({schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0',command,diagnostics:[],engine,config,...fields});
  const replayed=ref('replay.html','<style>.b{}</style><svg id="d"><g/></svg><script>{"engine":"new"}</script>');
  const deliver=fields=>({command:'deliver',original,artifact:replayed,receipt:ref(`deliver-${files.size}.json`,receipt('deliver',{status:'pass',candidate:{path:'/c',identity:candidate},artifact:{path:'/o',identity:replayed.identity},...fields}))});
  const validate=fields=>({command:'validate',receipt:ref(`validate-${files.size}.json`,receipt('validate',{status:'failed',candidate:{path:'/s',identity:snapshot},diagnostics:[{code:'architecture/grid-cell',severity:'error'},{code:'layout/x',severity:'warning'}],...fields}))});
  const expected=[{command:'deliver',candidate,artifact:original.identity,config},{command:'validate',candidate:snapshot,codes:['architecture/grid-cell'],config}];
  assert.equal(checkReplay(expected,{id:'repairable-diagnostic',replays:[deliver(),validate()]},final,store),undefined);
  const rejects=(replays,pattern,label)=>assert.throws(()=>checkReplay(expected,{id:'repairable-diagnostic',replays},final,store),pattern,label);
  const changed=ref('changed.html','<svg id="d"><g transform="x"/></svg>');
  rejects([{...deliver({artifact:{path:'/o',identity:changed.identity}}),artifact:changed},validate()],/diagram differs/,'svg');
  rejects([deliver({engine:{...engine,sourceInputSha256:'7'.repeat(64)}}),validate()],/final engine/,'old engine');
  rejects([deliver({engine:{...engine,binary:{sha256:'0'.repeat(64),bytes:9}}}),validate()],/final engine/,'foreign binary');
  rejects([deliver({candidate:{path:'/c',identity:snapshot}}),validate()],/final engine from the original input/,'other input');
  rejects([deliver({config:{...config,quality:'standard'}}),validate()],/configuration/,'config');
  rejects([deliver({status:'failed'}),validate()],/replay/i,'failed delivery');
  rejects([deliver(),validate({diagnostics:[{code:'layout/other',severity:'error'}]})],/diagnostics differ/,'codes');
  rejects([deliver(),validate({status:'pass'})],/replay/i,'validation passes now');
  rejects([deliver()],/replay/i,'missing validation');
  rejects([deliver(),validate(),validate()],/replay/i,'extra replay');
  rejects([{...deliver(),original:ref('other.html','<svg></svg>')},validate()],/replay/i,'other original');
});
