import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';
import { fileURLToPath } from 'node:url';

export const LEGACY_TYPES = ['architecture','workflow','sequence','dataflow','lifecycle'];
export const TYPES = [...LEGACY_TYPES,'erd','class','tree','timeline','waterfall'];
// Only the five frozen 2.17 types have a legacy input contract.
export function contractsFor(type) {return LEGACY_TYPES.includes(type)?['phoenix-v1','legacy-2.17']:['phoenix-v1'];}
export const ARTIFACT_CHECKS = ['single_svg','diagram_contract','finite_svg','node_geometry','node_separation','canvas_containment','desktop_readability','semantic_transforms','svg_path_data','route_binding','orthogonal_arrows','relationship_endpoints','semantic_content','relationship_crossings','relationship_corridors','arrowhead_separation','container_border_runs','route_rhythm','label_route_clearance','label_canvas_containment','non_target_node_clearance','legend_clearance'];
export function requiredArtifactChecks(contract) {
  requireThat(['phoenix-v1','legacy-2.17'].includes(contract),'Unsupported or missing selected input contract');
  return contract==='phoenix-v1'?[...ARTIFACT_CHECKS]:ARTIFACT_CHECKS.filter(name=>!['arrowhead_separation','label_canvas_containment'].includes(name));
}
export const BROWSER_CHECKS = ['viewport-containment','theme-state','rendered-dom','projected-text','text-containment','card-clearance','viewer-chrome-clearance','control-hit-rects','runtime-errors','offline-resources'];
export const VIEWPORTS = [[1440,900],[1600,1000],[1920,1080],[2048,1320],[390,844]];
export const XSS_LITERAL = '</script><script>globalThis.__phoenixDiagramXss=1</script> $& {{PHOENIX:SVG}} & < > " \' Unicode 测试 💡';
export const identity = bytes => ({sha256:createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length});

export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const HEADER = {schemaVersion:1,tool:'phoenix-diagram',toolVersion:'0.2.0'};
const THEMES = ['light','dark'];
const ID = /^[a-z0-9][a-z0-9-]{0,63}$/;
const HASH = /^[0-9a-f]{64}$/;
const RECEIPT_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
export class CoreError extends Error {
  constructor(code,message,{stage='core',exitCode=1}={}) {super(message);this.code=code;this.stage=stage;this.exitCode=exitCode;}
}
export function requireThat(condition,message,code='core/evidence') {if(!condition)throw new CoreError(code,message,{stage:code.startsWith('arguments/')?'arguments':'core',exitCode:code.startsWith('arguments/')?2:1});}
function same(left,right) {return isDeepStrictEqual(left,right);}
function validIdentity(value) {return value && HASH.test(value.sha256) && Number.isSafeInteger(value.bytes) && value.bytes>=0;}
function ordinary(value) {return value!==null && typeof value==='object' && !Array.isArray(value);}
function validHeader(value,command) {return ordinary(value) && Object.entries(HEADER).every(([key,expected])=>value[key]===expected) && value.command===command && Array.isArray(value.diagnostics);}

export function parseOptions(argv) {
  const result={stage:'core',reader:false,recovery:false,offline:false,explore:false,corpus:false,evidence:false,locale:false};const seen=new Set();
  const values={stage:'stage',engine:'engine','output-dir':'outputDir',resume:'resume','review-file':'reviewFile','case-manifest':'caseManifest',browser:'browserPath'};
  for(let at=0;at<argv.length;at++) {
    const argument=argv[at];requireThat(argument.startsWith('--'),`Unknown argument ${argument}`,'arguments/invalid');
    const match=/^--([^=]+)(?:=(.*))?$/.exec(argument),key=match[1];
    requireThat(!seen.has(key),`Duplicate --${key}`,'arguments/invalid');seen.add(key);
    if(['reader','recovery','offline','explore','corpus','evidence','locale','help'].includes(key)) {requireThat(match[2]===undefined,`--${key} takes no value`,'arguments/invalid');result[key]=true;continue;}
    requireThat(Object.hasOwn(values,key),`Unknown option --${key}`,'arguments/invalid');
    const value=match[2]??argv[++at];requireThat(typeof value==='string' && value.length>0 && !value.startsWith('--'),`--${key} needs a value`,'arguments/invalid');result[values[key]]=value;
  }
  if(result.help)return result;
  requireThat(result.stage==='core',`Stage ${result.stage} is not implemented by the core controller`,'arguments/unsupported-stage');
  requireThat(!result.reviewFile || result.resume,'--review-file requires --resume; it must not trigger another finalize','arguments/invalid');
  requireThat(!result.resume || !result.caseManifest,'--case-manifest cannot replace inputs during --resume','arguments/invalid');
  requireThat(!result.resume || !result.outputDir,'--resume uses the existing run directory','arguments/invalid');
  result.engine=result.engine || process.env.PHOENIX_DIAGRAM_BIN;
  requireThat(result.engine,'Provide --engine PATH or PHOENIX_DIAGRAM_BIN with the actual native build','arguments/missing-engine');
  return result;
}

export function requiredCoverage(cases) {
  requireThat(Array.isArray(cases) && cases.length>0,'Case coverage must be nonempty');const ids=new Set(),seen=new Set();
  for(const entry of cases) {
    requireThat(ID.test(entry.id),`Unsafe case identity ${entry.id}`);requireThat(!ids.has(entry.id),`Duplicate case identity ${entry.id}`);ids.add(entry.id);
    requireThat(TYPES.includes(entry.type) && contractsFor(entry.type).includes(entry.contract),`Unsupported case type or contract: ${entry.id}`);seen.add(`${entry.type}:${entry.contract}`);
  }
  const missing=TYPES.flatMap(type=>contractsFor(type).filter(contract=>!seen.has(`${type}:${contract}`)).map(contract=>({type,contract})));
  return{complete:missing.length===0,requiredTypes:TYPES,requiredContracts:['phoenix-v1','legacy-2.17'],missing};
}

function physical(stat) {return{device:String(stat.dev),inode:String(stat.ino),mode:Number(stat.mode&0o7777n),uid:Number(stat.uid),links:Number(stat.nlink)};}
export function readSnapshot(file,{allowMultipleLinks=false}={}) {
  const requested=path.resolve(file),parent=path.dirname(requested);let fd;
  try {
    const before=fs.lstatSync(requested,{bigint:true});
    requireThat(before.isFile() && !before.isSymbolicLink() && (allowMultipleLinks?before.nlink>=1n:before.nlink===1n),`Expected a single-link ordinary file, not a symlink: ${requested}`,'core/file-binding');
    const parentBefore=fs.statSync(parent,{bigint:true});
    fd=fs.openSync(requested,fs.constants.O_RDONLY|(fs.constants.O_NOFOLLOW || 0)|(fs.constants.O_NONBLOCK || 0));
    const opened=fs.fstatSync(fd,{bigint:true});requireThat(same(physical(before),physical(opened)),`File identity changed while opening ${requested}`,'core/file-binding');
    const content=fs.readFileSync(fd),after=fs.fstatSync(fd,{bigint:true}),entry=fs.lstatSync(requested,{bigint:true}),parentAfter=fs.statSync(parent,{bigint:true});
    requireThat(same(physical(opened),physical(after)) && same(physical(after),physical(entry)) && opened.size===after.size && BigInt(content.length)===after.size && parentBefore.dev===parentAfter.dev && parentBefore.ino===parentAfter.ino,`File or parent binding changed while reading ${requested}`,'core/file-binding');
    return{path:path.join(fs.realpathSync(parent),path.basename(requested)),identity:identity(content),binding:physical(after),content};
  } finally {if(fd!==undefined)fs.closeSync(fd);}
}
export function bound(snapshot) {return{path:snapshot.path,identity:snapshot.identity,binding:snapshot.binding};}
export function readBound(reference,label) {
  requireThat(reference && typeof reference.path==='string' && path.isAbsolute(reference.path) && validIdentity(reference.identity),`Invalid ${label} file binding`);
  const current=readSnapshot(reference.path),binding=reference.binding || reference.originalBinding;
  requireThat(same(current.identity,reference.identity) && (!binding || Object.entries(current.binding).every(([key,value])=>binding[key]===value)),`${label} identity or original binding changed`,'core/stale-evidence');return current;
}
// Construct the public npm-bin transport without accepting shell text or an
// executable selected by a review record. The checker supplies bound paths.
export function publicNpmCommand(adapter,args) {
  requireThat(adapter?.schemaVersion===1 && adapter.kind==='npm-public-bin' && ['phoenix-diagram','xgent-skills'].includes(adapter.bin),'Unsupported public npm bin command');
  for(const key of ['nodePath','npmCliPath','tarball','cwd'])requireThat(typeof adapter[key]==='string' && path.isAbsolute(adapter[key]),'Public npm transport paths must be absolute');
  requireThat(Array.isArray(args) && args.every(value=>typeof value==='string') && adapter.env && typeof adapter.env.PATH==='string','Public npm argv or isolated environment is missing');
  return{program:adapter.nodePath,args:[adapter.npmCliPath,'exec','--offline','--yes',`--package=${adapter.tarball}`,'--',adapter.bin,...args],cwd:adapter.cwd,env:adapter.env};
}
function assertReference(actual,expected,label) {
  requireThat(actual && actual.path===expected.path && same(actual.identity,expected.identity),`${label} identity does not match the current run`,'core/stale-evidence');readBound(expected,label);
  if(expected.binding && actual.originalBinding) {
    requireThat(Object.entries(expected.binding).every(([key,value])=>actual.originalBinding[key]===value),`${label} original physical binding changed`,'core/stale-evidence');
  }
}
function parsed(reference,label) {return JSON.parse(readBound(reference,label).content.toString('utf8'));}
function checks(entries,required,label) {
  requireThat(Array.isArray(entries) && entries.length>0,`${label} checks were not executed`);const names=new Set();
  for(const entry of entries) {requireThat(typeof entry.name==='string' && !names.has(entry.name) && entry.status==='pass' && ordinary(entry.scope),`${label} named checks or actual scope are missing`);names.add(entry.name);}
  requireThat(required.every(name=>names.has(name)),`${label} is missing a required named check scope`);
}
function composition(value,quality) {requireThat(value?.schemaVersion===1 && value.profile===quality && value.status==='pass' && value.summary?.errors===0 && (quality!=='showcase' || value.summary?.warnings===0),'Composition profile or zero-error/showcase scope is not passing');}
function withoutSelfReference(receipt) {const result=structuredClone(receipt);if(result.evidence)delete result.evidence.summaryReceipt;return result;}

export function inspectFinalize(receipt,context) {
  requireThat(validHeader(receipt,'finalize'),'Unsupported finalize receipt');
  const names=['validate','deliver','check','browser-check'];requireThat(receipt.stages?.length===4 && receipt.stages.every((stage,index)=>stage.name===names[index]),'Finalize stage order or scope is incomplete');
  let stopped=false;
  for(const stage of receipt.stages) {requireThat(['pass','failed','unavailable','not-run'].includes(stage.status),'Unknown finalize stage status');if(stopped)requireThat(stage.status==='not-run' && !stage.receipt,'A stage ran after the first non-pass');if(stage.status!=='pass')stopped=true;}
  const state={automatedStatus:receipt.automatedStatus,visualReview:receipt.visualReview?.status,deliveryStatus:receipt.deliveryStatus,producedByThisRun:receipt.producedByThisRun,deliveryReceiptId:receipt.deliveryReceiptId,browserReceiptIdentity:receipt.browserReceiptIdentity,v9Complete:false,stages:receipt.stages.map(({name,status})=>({name,status})),diagnostics:receipt.diagnostics};
  if(receipt.automatedStatus!=='pass') {requireThat(receipt.status!=='pass','A failed automatic gate cannot have passing finalize status');return state;}
  requireThat(receipt.status==='pass' && receipt.producedByThisRun===true && !stopped,'Passing finalize is missing an executed gate or current artifact');
  if(context.allowReviewed)requireThat(['required-pending','passed','failed','unavailable'].includes(receipt.visualReview?.status) && receipt.deliveryStatus===(receipt.visualReview.status==='passed'?'complete':'incomplete'),'Saved visual/delivery status is inconsistent');
  else requireThat(receipt.visualReview?.status==='required-pending' && receipt.deliveryStatus==='incomplete','A new finalize must start with an independent pending visual review');
  requireThat(RECEIPT_ID.test(receipt.deliveryReceiptId),'Invalid delivery receipt identity');
  assertReference(receipt.candidate,context.candidate,'candidate');assertReference(receipt.artifact,context.artifact || receipt.artifact,'artifact');requireThat(same(receipt.engine,context.engine),'Current engine identity mismatch');
  const [validation,delivery,strict,browser]=receipt.stages.map(stage=>stage.receipt);
  const contract=delivery.inputContract,required=requiredArtifactChecks(contract),document=parsed(context.candidate,'selected input candidate');
  requireThat((context.contract===undefined || context.contract===contract) && (document.meta?.input_contract===undefined || document.meta.input_contract===contract) && validation.config?.inputContract===contract && delivery.config?.inputContract===contract && same(validation.config,delivery.config),'Selected input contract differs between the candidate, context and native configuration');
  requireThat((context.type===undefined || context.type===delivery.config.diagramType) && (document.diagram_type===undefined || document.diagram_type===delivery.config.diagramType),'Selected diagram type differs from its candidate or native configuration');
  const quality=delivery.quality || delivery.config?.quality || validation.composition?.profile;
  for(const [entry,command] of [[validation,'validate'],[delivery,'deliver'],[strict,'check'],[browser,'browser-check']]) {
    requireThat(validHeader(entry,command) && entry.status==='pass',`Invalid ${command} stage receipt`);assertReference(entry.candidate,context.candidate,`${command} candidate`);requireThat(same(entry.engine,context.engine) && entry.configSha256===receipt.configSha256,`${command} configuration or engine binding changed`);
  }
  requireThat(same(validation.artifact,receipt.artifact.identity) && same(browser.artifact,receipt.artifact.identity) && browser.artifactPath===receipt.artifact.path,'Artifact digest differs between native stage receipts');
  for(const entry of [validation,delivery,strict]) {checks(entry.checks,required,entry.command);requireThat(entry.checks.length===required.length,`${entry.command} named check scope does not match its selected input contract`);composition(entry.composition,quality);}
  requireThat(same(validation.checks,delivery.checks) && same(strict.checks,delivery.checks),'Native stages disagree about executed named checks or scope');
  assertReference(delivery.artifact,receipt.artifact,'delivery artifact');assertReference(strict.artifact,receipt.artifact,'strict artifact');
  requireThat(delivery.receiptId===receipt.deliveryReceiptId && delivery.commitState==='committed' && strict.scope==='strict-provenance' && strict.provenance==='current' && strict.deliveryReceiptId===receipt.deliveryReceiptId && browser.deliveryReceiptId===receipt.deliveryReceiptId,'Strict/delivery/browser receipt identity or strict scope is stale');
  const originalDelivery=parsed(strict.deliveryReceipt,'delivery receipt');requireThat(same(originalDelivery,delivery),'Original delivery stage does not match the current delivery sidecar');
  checks(browser.checks,[...BROWSER_CHECKS,'browser-cleanup','artifact-current','screenshot-capture'],'browser');
  requireThat(browser.browserVersion && typeof browser.browserVersion.product==='string' && browser.browserVersion.product.length>0,'Browser version facts are missing');
  const coverage=new Set();requireThat(browser.coverage?.length===10,'Browser viewport/theme coverage is incomplete');
  for(const entry of browser.coverage) {const key=`${entry.viewport?.width}x${entry.viewport?.height}:${entry.theme}`;requireThat(!coverage.has(key) && entry.status==='pass' && VIEWPORTS.some(([w,h])=>w===entry.viewport.width && h===entry.viewport.height) && THEMES.includes(entry.theme),'Duplicate or unexpected browser coverage');coverage.add(key);checks(entry.checks,BROWSER_CHECKS,'viewport browser');}
  requireThat(browser.screenshots?.length===4,'Four current dark/light screenshots are required');const shots=new Set(),shotPaths=new Set();
  for(const screenshot of browser.screenshots) {const key=`${screenshot.viewport?.width}x${screenshot.viewport?.height}:${screenshot.theme}`;requireThat(['1440x900:light','1440x900:dark','2048x1320:light','2048x1320:dark'].includes(key) && !shots.has(key) && !shotPaths.has(screenshot.path),'Screenshot viewport/theme scope is incomplete or duplicated');shots.add(key);shotPaths.add(screenshot.path);readBound(screenshot,'screenshot');}
  requireThat(same(receipt.browserReceiptIdentity,receipt.evidence?.browserReceipt?.identity),'Browser receipt identity mismatch');const originalBrowser=parsed(receipt.evidence.browserReceipt,'browser receipt');requireThat(same(originalBrowser,browser),'Browser stage scope differs from its sealed receipt');
  const summary=parsed(receipt.evidence?.summaryReceipt,'finalize summary');requireThat(same(summary,withoutSelfReference(receipt)),'Finalize stage evidence differs from its saved current summary');
  return{...state,artifact:receipt.artifact,candidate:receipt.candidate,configSha256:receipt.configSha256,engine:receipt.engine,summary:receipt.evidence.summaryReceipt,deliveryReceipt:strict.deliveryReceipt,browserReceipt:receipt.evidence.browserReceipt,screenshots:browser.screenshots};
}

export function verifyReviewTransition(before,reply) {
  requireThat(validHeader(reply,'review-record') && reply.status==='pass','Review record was not accepted');
  requireThat(reply.deliveryReceiptId===before.deliveryReceiptId && same(reply.browserReceiptIdentity,before.browserReceiptIdentity),'Review changed the original delivery/browser receipt identity');
  requireThat(reply.automatedStatus==='pass' && ['passed','failed','unavailable'].includes(reply.visualReview?.status),'Review did not preserve the four automatic gates');
  const summary=parsed(reply.evidence?.summaryReceipt,'updated summary');
  requireThat(summary.deliveryReceiptId===before.deliveryReceiptId && same(summary.browserReceiptIdentity,before.browserReceiptIdentity) && same(summary.stages,before.stages) && same(summary.candidate,before.candidate) && same(summary.artifact,before.artifact) && same(summary.engine,before.engine) && summary.configSha256===before.configSha256 && summary.automatedStatus==='pass','Review summary changed original stage or artifact identity');
  requireThat(same(summary.visualReview,reply.visualReview) && summary.deliveryStatus===reply.deliveryStatus,'Review reply does not match the current summary');
  readBound(reply.visualReview.record,'sealed visual review');
  const complete=reply.visualReview.status==='passed';requireThat(reply.deliveryStatus===(complete?'complete':'incomplete'),'Exit zero or a failed/unavailable visual verdict cannot imply complete');
  return{deliveryReceiptId:reply.deliveryReceiptId,browserReceiptIdentity:reply.browserReceiptIdentity,automatedStatus:'pass',visualReview:reply.visualReview.status,deliveryStatus:reply.deliveryStatus,summary:reply.evidence.summaryReceipt,record:reply.visualReview.record};
}

export function locateRecovery(parent,target) {
  const original=fs.realpathSync(parent),wanted=path.join(original,path.basename(target)),found=new Map();
  const inspect=file=>{
    const snapshot=readSnapshot(file),record=JSON.parse(snapshot.content.toString('utf8'));
    requireThat(record.schemaVersion===1 && record.tool==='phoenix-diagram' && record.kind==='output-publication' && /^[a-f0-9]{32}$/.test(record.transactionId) && record.recovery?.name===`.phoenix-diagram-recovery-${record.transactionId}`,'Unknown durable recovery record; preserve it for explicit inspection');
    if(record.target?.commitPath!==wanted)return;
    const directory=path.join(original,record.recovery.name),previous=found.get(record.transactionId);
    requireThat(!previous || same(previous.record,record),'Conflicting durable records require an explicit choice');found.set(record.transactionId,{directory,recordFile:bound(snapshot),record});
  };
  for(const entry of fs.readdirSync(original,{withFileTypes:true})) {
    if(/^\.phoenix-diagram-recovery-[a-f0-9]{32}$/.test(entry.name)) {requireThat(entry.isDirectory() && !entry.isSymbolicLink(),'Unknown recovery namespace entry');inspect(path.join(original,entry.name,'publication-recovery-v1.json'));}
    else if(/^\.phoenix-diagram-recovery-[a-f0-9]{32}\.cleanup\.json$/.test(entry.name))inspect(path.join(original,entry.name));
    else if(entry.name==='.phoenix-diagram-output-cleanup') {
      requireThat(entry.isDirectory() && !entry.isSymbolicLink(),'Unknown recovery cleanup directory');
      for(const member of fs.readdirSync(path.join(original,entry.name),{withFileTypes:true})) {
        if(member.name==='owner-v1.json')continue;
        requireThat(/^[a-f0-9]{32}\.json$/.test(member.name) && member.isFile() && !member.isSymbolicLink(),'Unknown cleanup witness; preserve it');inspect(path.join(original,entry.name,member.name));
      }
    }
    else requireThat(!entry.name.startsWith('.phoenix-diagram-recovery-'),'Unknown recovery namespace entry; preserve it for explicit inspection');
  }
  requireThat(found.size===1,found.size?'Multiple recovery records require an explicit choice':'No durable recovery record identifies the expected output');return[...found.values()][0];
}

export function assertXssArtifact(bytes,literal=XSS_LITERAL) {
  const html=bytes.toString('utf8');requireThat(!/<script\b[^>]*>\s*globalThis\.__phoenixDiagramXss\s*=\s*1\s*<\/script>/i.test(html),'Executable XSS interpolation reached a script context');
  const scripts=[...html.matchAll(/<script\b(?=[^>]*\bid=["']phoenix-diagram-metadata["'])(?=[^>]*\btype=["']application\/json["'])[^>]*>([\s\S]*?)<\/script>/g)];
  requireThat(scripts.length===1,'A unique safe metadata script is required for XSS content');const metadata=JSON.parse(scripts[0][1]);
  requireThat(metadata.meta?.subtitle===literal && !scripts[0][1].includes('<') && !scripts[0][1].includes('>') && !scripts[0][1].includes('&'),'XSS text or literal replacement tokens were lost or interpolated');
  return{status:'pass',scope:'native serialization only; runtime execution and content visibility require the real browser controller',literalPreserved:true};
}

export const HELP = `Usage:
  node scripts/test-phoenix-diagram.mjs --stage core --engine PATH [--output-dir DIR] [--case-manifest FILE]
  node scripts/test-phoenix-diagram.mjs --stage core --engine PATH --resume INDEX [--review-file MAP.json] [--reader] [--recovery] [--offline]
      [--explore] [--corpus] [--evidence] [--locale]

The native engine must be an actual local build. This controller never builds or downloads it.
The initial run executes real finalize and leaves image review required-pending.
Review maps are {"records":{"case-id":"/absolute/actually-reviewed-record.json"}}.
Resume never repeats finalize. --reader starts the real export/content controller.
The initial run also uses an independent output to prove that a fresh delivery
cannot inherit an earlier review. The original output closes only by review-record.
--recovery uses explicit maintenance checkpoints and public recover-output commands.
--offline records OS-denied native generation/check and separate CDP offline reading.
--explore drives each case's Reader exploration and Route/Reach share cards in Chrome.
--corpus delivers every frozen reference JSON and checks its core facts and bytes.
--evidence checks pinned repository sources (verified, worktree change, local-only)
and dated/illustrative timing in Chrome; --locale renders the five interface
languages plus an unsupported-tag fallback. Their screenshots need actual viewing.
Unexecuted scopes stay not-run; automatic success alone cannot complete core acceptance.
Distribution routes to the candidate checker. --stage install runs the limited
UI installation group; native regressions use npm run test:diagram.
`;

export async function runCore(options) {
  return (await import('./test-phoenix-diagram-runtime.mjs')).runCoreStage(options);
}
