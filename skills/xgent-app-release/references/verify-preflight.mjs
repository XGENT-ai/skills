import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
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
    assert.deepEqual(normalizeEmbedCsp(cases.find(c=>c[0]==="origin normalization")[1]),{imgSrc:["https://example.com"]});passed++;
    assert.throws(()=>normalizeEmbedCsp(Object.defineProperty({},"scriptSrc",{get(){throw new Error("must-not-run");}})),/accessors/);passed++;
    console.log(`exported preflight: ${passed}/0`);
  } finally { rmSync(root,{recursive:true,force:true}); }
}
