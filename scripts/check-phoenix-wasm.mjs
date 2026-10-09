#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { workspace } from './phoenix-build-tools.mjs';

const pkg = process.env.PHOENIX_UI_WASM_DIR || path.join(workspace, 'target/wasm-verification');
const wasm = fs.readFileSync(path.join(pkg, 'impeccable_bg.wasm'));
for (const prefix of [workspace, os.homedir(), process.env.CARGO_HOME].filter(Boolean)) {
  assert(!wasm.includes(Buffer.from(prefix)), 'WASM contains an absolute build-machine path');
}
const context = vm.createContext({ TextEncoder, TextDecoder, WebAssembly, console });
vm.runInContext(fs.readFileSync(path.join(pkg, 'impeccable.js'), 'utf8'), context);
const api = vm.runInContext('wasm_bindgen', context);
context.__wasmBytes = wasm;
vm.runInContext('wasm_bindgen.initSync({ module: __wasmBytes })', context);
assert.equal(typeof api.pure_call, 'function', 'Build the verification bundle with --pure');
const parity = JSON.parse(fs.readFileSync(path.join(workspace, 'patches/numeric-parity.json')));
const overrides = new Map(parity.overrides.map(entry => [`${entry.module}/${entry.name}:${entry.line}`, entry]));
const functions = JSON.parse(api.pure_functions());
assert.deepEqual(functions, parity.recordedFunctions, 'Exported interfaces differ from the recorded registry');

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (!value || typeof value !== 'object') return value;
  if (value.$negzero === true && Object.keys(value).length === 1) return -0;
  return Object.fromEntries(Object.entries(value).filter(([, v]) =>
    !(v && typeof v === 'object' && v.$undef === true && Object.keys(v).length === 1))
    .map(([key, value]) => [key, canonical(value)]));
}

let vectors = 0, failures = 0;
const missing = [];
const covered = new Set(), usedOverrides = new Set();
const calls = [], results = [], expectedResults = [], deltas = [], deltaByIndex = new Map();
for (const { module, functions: names } of functions) {
  for (const fn of names) {
    const file = path.join(workspace, 'tests/oracle/vectors/calls', module, `${fn}.jsonl`);
    if (!fs.existsSync(file)) { missing.push(`${module}/${fn}`); continue; }
    const lines = fs.readFileSync(file, 'utf8').split('\n');
    const sourceSha = crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
    for (let line = 0; line < lines.length; line++) {
      if (!lines[line].trim()) continue;
      const record = JSON.parse(lines[line]);
      const override = overrides.get(`${module}/${fn}:${line + 1}`);
      if (override) {
        assert.equal(override.sourceSha256, sourceSha, 'Numeric expectation refers to a different source file');
        assert.deepEqual(record.args, override.args, 'Numeric expectation refers to different arguments');
        assert.equal(record.result, override.oldResult, 'Original numeric golden changed');
        usedOverrides.add(`${module}/${fn}:${line + 1}`);
      }
      const expected = override ? override.newResult : record.result ?? null;
      const args = JSON.stringify(record.args || []);
      const actual = JSON.parse(api.pure_call(module, fn, args));
      calls.push({ module, name: fn, args });
      results.push(actual);
      expectedResults.push(expected);
      vectors++;
      covered.add(`${module}/${fn}`);
      try { assert.deepEqual(canonical(actual), canonical(expected)); }
      catch (error) {
        failures++;
        const delta = { module, name: fn, line: line + 1, expected, wasm: actual };
        deltas.push(delta);
        deltaByIndex.set(results.length - 1, delta);
        if (failures <= 5) console.error(`${module}/${fn}:${line + 1}: ${error.message}`);
      }
    }
  }
}
assert.equal(vectors, 8321, 'Historical vector coverage changed');
assert.equal(usedOverrides.size, overrides.size, 'An exact numeric override was not exercised');
const boundaryProbes = parity.boundaries.probes.map(probe => ({ ...probe, expected: probe.baselineWasm }));
for (const probe of [...boundaryProbes, ...parity.supplemental.probes]) {
  const args = JSON.stringify(probe.args);
  const actual = JSON.parse(api.pure_call(probe.module, probe.name, args));
  calls.push({ module: probe.module, name: probe.name, args });
  results.push(actual);
  expectedResults.push(probe.expected);
  covered.add(`${probe.module}/${probe.name}`);
  vectors++;
  try { assert.deepEqual(canonical(actual), canonical(probe.expected)); }
  catch (error) {
    failures++;
    const delta = { module: probe.module, name: probe.name, id: probe.id, expected: probe.expected, wasm: actual };
    deltas.push(delta);
    deltaByIndex.set(results.length - 1, delta);
    if (failures <= 5) console.error(`${probe.id}: ${error.message}`);
  }
}
const uncovered = functions.flatMap(({ module, functions: names }) => names.map(name => `${module}/${name}`))
  .filter(name => !covered.has(name));
assert.deepEqual(uncovered, [], 'A pure interface has no actual verification vectors');
assert.equal(vectors, 8474, 'Historical, boundary and supplemental vector coverage changed');
assert(process.env.PHOENIX_UI_VECTORS_BIN, 'Set PHOENIX_UI_VECTORS_BIN from the replay_vectors compiler-artifact JSON');
const native = spawnSync(process.env.PHOENIX_UI_VECTORS_BIN, [], {
  input: calls.map(c => JSON.stringify(c)).join('\n') + '\n', encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
});
if (native.error || native.status !== 0) throw native.error || new Error(native.stderr);
const nativeResults = native.stdout.trim().split('\n').map(line => JSON.parse(JSON.parse(line).result));
assert.equal(nativeResults.length, results.length, 'Native vector result count differs');
let parityFailures = 0, nativeGoldenFailures = 0;
for (let i = 0; i < results.length; i++) {
  try { assert.deepEqual(canonical(nativeResults[i]), canonical(expectedResults[i])); }
  catch { nativeGoldenFailures++; }
  try { assert.deepEqual(canonical(results[i]), canonical(nativeResults[i])); }
  catch {
    parityFailures++;
    const delta = deltaByIndex.get(i);
    if (delta) delta.native = nativeResults[i];
  }
}
console.log(JSON.stringify({ vectors, goldenFailures: failures, nativeGoldenFailures, parityFailures, deltas,
  interfaces: covered.size, functionsWithoutRecordedVectors: missing, functionsWithoutVerification: uncovered,
  absolutePathScan: 'passed' }));
process.exit(failures || nativeGoldenFailures || parityFailures ? 1 : 0);
