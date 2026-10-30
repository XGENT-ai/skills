import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, openSync, writeSync, closeSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { normalizeEmbedCsp } from "./embed-csp.mjs";

export const cases = [
  ["omitted", undefined, true], ["clear", null, true], ["empty", {}, true],
  ["all directives", Object.fromEntries(["connectSrc","scriptSrc","styleSrc","fontSrc","imgSrc","mediaSrc"].map(k=>[k,["https://cdn.example.com"]])), true],
  ["websocket connect", {connectSrc:["wss://events.example.com"]}, true],
  ["local schemes connect", {connectSrc:["blob:","data:"]}, true],
  ["local scheme with payload", {connectSrc:["data:,x"]}, false],
  ["alias", {platformSources:{scriptSrc:["jsCdn"],styleSrc:["jsCdn"],fontSrc:["jsCdn"],connectSrc:["jsCdn"]}}, true],
  ["origin normalization", {imgSrc:[" https://EXAMPLE.com:443/ ","https://example.com"]}, true],
  ["empty arrays", {scriptSrc:[],platformSources:{scriptSrc:[]}}, true],
  ["unknown field", {frameAncestors:["https://example.com"]}, false],
  ["unknown alias", {platformSources:{scriptSrc:["other"]}}, false],
  ["unsupported alias directive", {platformSources:{imgSrc:["jsCdn"]}}, false],
  ["wrong collection", {scriptSrc:"https://example.com"}, false],
  ["wrong source", {scriptSrc:[null]}, false],
  ...["https://*.example.com","https://example.com/script.js","https://example.com?x=1","https://example.com#x",
    "https://user:private@localhost","unsafe-inline","'unsafe-eval'","data:","blob:","http://example.com",
    "http://localhost:8080","wss://example.com","https:example.com","https://example.com\n","https://example.com;"].map((value,i)=>[`invalid origin ${i}`,{scriptSrc:[value]},false]),
];

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = mkdtempSync(join(tmpdir(),"release-preflight-fixture-"));
  const preflight = resolve(dirname(fileURLToPath(import.meta.url)),"../scripts/preflight.mjs");
  let passed = 0;
  try {
    mkdirSync(join(root,"dist"));
    writeFileSync(join(root,"dist/index.html"),'<script src="/apps/demo/main.js"></script>');
    writeFileSync(join(root,"dist/main.js"),'console.log("fixture")');
    writeFileSync(join(root,"fixture.env"),"LISTING_KEY=demo\n");
    for (const [label,value,valid] of cases) {
      const manifest = {listingKey:"demo",name:"Demo",version:"1",...(value===undefined?{}:{embedCsp:value})};
      writeFileSync(join(root,"app.manifest.json"),JSON.stringify(manifest));
      const result = spawnSync(process.execPath,[preflight,"--offline","--config","fixture.env","--key","demo","--dist","dist","--version","1","--manifest","app.manifest.json"],{cwd:root,env:{PATH:process.env.PATH},encoding:"utf8",timeout:15000});
      assert.equal(result.status,valid?0:1,label);
      if (value!==undefined) assert.match(result.stdout,/能力门未核验/,label);
      assert.ok(!result.stdout.includes("private@localhost"),"不回显非法来源中的凭据");
      passed++;
    }
    // usageMetrics[].label：门户收平台语言键 { "zh-CN", en?, "zh-TW"? }，旧形状 { zh, en?, tw? } 仍收。
    for (const [label,value,valid] of [
      ["metric label zh-CN", {"zh-CN":"完成任务数",en:"Jobs"}, true],
      ["metric label legacy zh", {zh:"完成任务数"}, true],
      ["metric label en only", {en:"Jobs"}, false],
      ["metric label blank", {"zh-CN":"  "}, false],
      ["metric label string", "完成任务数", false],
    ]) {
      const manifest = {listingKey:"demo",name:"Demo",version:"1",usageMetrics:[{key:"demo.jobs",kind:"counter",unit:"count",label:value}]};
      writeFileSync(join(root,"app.manifest.json"),JSON.stringify(manifest));
      const result = spawnSync(process.execPath,[preflight,"--offline","--config","fixture.env","--key","demo","--dist","dist","--version","1","--manifest","app.manifest.json"],{cwd:root,env:{PATH:process.env.PATH},encoding:"utf8",timeout:15000});
      assert.equal(result.status,valid?0:1,label);
      passed++;
    }
    // requiredEnvNotes 与 Dashboard 组件：与门户提交面同口径（坏的在 CI 里就拦住）。
    for (const [label,extra,valid] of [
      ["env note ok", {requiredEnv:["DEMO_TITLE"],requiredEnvNotes:{DEMO_TITLE:"报表抬头"}}, true],
      ["env note undeclared key", {requiredEnv:["DEMO_TITLE"],requiredEnvNotes:{DEMO_OTHER:"x"}}, false],
      ["env note blank", {requiredEnv:["DEMO_TITLE"],requiredEnvNotes:{DEMO_TITLE:" "}}, false],
      ["env note too long", {requiredEnv:["DEMO_TITLE"],requiredEnvNotes:{DEMO_TITLE:"长".repeat(201)}}, false],
      ["widget ok", {dashboardWidgets:[{key:"w",source:"app",owner:"demo",type:"topic_list",title:"W"}]}, true],
      ["widget nested key", {dashboard:{widgets:[{key:"w",source:"app",owner:"demo",type:"metric",title:"W"}]}}, false],
      ["widget type typo", {dashboardWidgets:[{key:"w",source:"app",owner:"demo",type:"topic-list",title:"W"}]}, false],
      ["widget owner mismatch", {dashboardWidgets:[{key:"w",source:"app",owner:"other",type:"metric",title:"W"}]}, false],
      ["admin page ok", {adminPages:{platform:{route:"/admin/settings",label:{"zh-CN":"设置",en:"Settings"}},tenant:{route:"/settings",label:"设置"}}}, true],
      ["admin page escape", {adminPages:{platform:{route:"//evil.example",label:"x"}}}, false],
      ["admin page no zh-CN", {adminPages:{tenant:{route:"/s",label:{en:"S"}}}}, false],
      ["admin page unknown level", {adminPages:{console:{route:"/s",label:"x"}}}, false],
      ["tenant config ok", {type:"service",scopes:["demo.read"],tenantConfig:{label:"设置",path:"/v2/policy",scopes:["demo.read"],schema:{type:"object",properties:{limits:{type:"object",properties:{wait:{type:"integer",title:"等待",default:10,unit:"秒",scale:1000},mode:{type:"select",title:"模式",options:[{value:"auto",label:"自动"}]},strict:{type:"boolean",title:"严格",default:false}}}}}}}, true],
      ["tenant config bad scope", {type:"service",scopes:["demo.read"],tenantConfig:{label:"设置",path:"/v2/policy",scopes:["demo.write"],schema:{type:"object",properties:{wait:{type:"integer",title:"等待"}}}}}, false],
      ["tenant config escape", {type:"service",scopes:["demo.read"],tenantConfig:{label:"设置",path:"//evil.example",scopes:["demo.read"],schema:{type:"object",properties:{wait:{type:"integer",title:"等待"}}}}}, false],
    ]) {
      writeFileSync(join(root,"app.manifest.json"),JSON.stringify({listingKey:"demo",name:"Demo",version:"1",...extra}));
      const result = spawnSync(process.execPath,[preflight,"--offline","--config","fixture.env","--key","demo","--dist","dist","--version","1","--manifest","app.manifest.json"],{cwd:root,env:{PATH:process.env.PATH},encoding:"utf8",timeout:15000});
      assert.equal(result.status,valid?0:1,label);
      passed++;
    }
    // 单包上限与门户 MAX_DIST_BYTES 同为 200 MiB：用不可压缩内容逼近边界，199 MiB 放行、201 MiB 拒绝。
    {
      const blob = join(root,"dist/blob.bin");
      const grow = (mib) => { const fd = openSync(blob,"a"); for (let i = 0; i < mib; i++) writeSync(fd,randomBytes(1024*1024)); closeSync(fd); };
      const run = () => spawnSync(process.execPath,[preflight,"--offline","--config","fixture.env","--key","demo","--dist","dist","--version","1","--manifest","app.manifest.json"],{cwd:root,env:{PATH:process.env.PATH},encoding:"utf8",timeout:120000});
      writeFileSync(join(root,"app.manifest.json"),JSON.stringify({listingKey:"demo",name:"Demo",version:"1"}));
      grow(199);
      assert.equal(run().status,0,"dist 199 MiB");
      passed++;
      grow(2);
      const over = run();
      assert.equal(over.status,1,"dist 201 MiB");
      assert.match(over.stdout,/单包 200 MiB 上限/,"dist 201 MiB");
      passed++;
      rmSync(blob);
    }
    assert.deepEqual(normalizeEmbedCsp(cases.find(c=>c[0]==="origin normalization")[1]),{imgSrc:["https://example.com"]});passed++;
    assert.throws(()=>normalizeEmbedCsp(Object.defineProperty({},"scriptSrc",{get(){throw new Error("must-not-run");}})),/accessors/);passed++;
    console.log(`exported preflight: ${passed}/0`);
  } finally { rmSync(root,{recursive:true,force:true}); }
}
