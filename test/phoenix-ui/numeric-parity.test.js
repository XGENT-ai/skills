const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { spawnSync } = require('node:child_process');
const test = require('node:test');

const workspace = path.resolve(__dirname, '../../tools/phoenix-ui');
const read = (file) => fs.readFileSync(path.join(workspace, file), 'utf8');
const sha256 = (value) => crypto.createHash('sha256').update(value).digest('hex');
const evidence = JSON.parse(read('patches/numeric-parity.json'));
const upstream = JSON.parse(read('UPSTREAM.json'));
const bits = (value) => {
  const buffer = Buffer.alloc(8);
  buffer.writeDoubleBE(value);
  return `0x${buffer.toString('hex')}`;
};

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (!value || typeof value !== 'object') return value;
  if (value.$negzero === true && Object.keys(value).length === 1) return -0;
  return Object.fromEntries(Object.entries(value).filter(([, v]) =>
    !(v && typeof v === 'object' && v.$undef === true && Object.keys(v).length === 1))
    .map(([key, value]) => [key, canonical(value)]));
}

function constant(file, name) {
  const body = read(file).match(new RegExp(`pub const ${name}:[\\s\\S]*?=\\s*&?\\[([\\s\\S]*?)\\];`));
  assert.ok(body, `${file}: ${name}`);
  return [...body[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
}

function recordedCalls() {
  const calls = [];
  for (const { module, functions } of evidence.recordedFunctions) {
    for (const name of functions) {
      const file = `tests/oracle/vectors/calls/${module}/${name}.jsonl`;
      if (!Object.hasOwn(evidence.recordedSources, file)) continue;
      read(file).split('\n').forEach((line, i) => {
        if (line.trim()) calls.push({ module, name, file, line: i + 1, ...JSON.parse(line) });
      });
    }
  }
  return calls;
}

function reversePatch(after, patch) {
  const input = after.trimEnd().split('\n'), output = [], lines = patch.trimEnd().split('\n');
  let cursor = 0;
  assert.equal(lines[0], `--- a/${evidence.candidate.file}`);
  assert.equal(lines[1], `+++ b/${evidence.candidate.file}`);
  for (let i = 2; i < lines.length;) {
    const hunk = lines[i++].match(/^@@ -(\d+),(\d+) \+(\d+),(\d+) @@/);
    assert.ok(hunk);
    const target = Number(hunk[3]) - 1;
    output.push(...input.slice(cursor, target));
    cursor = target;
    assert.equal(output.length, Number(hunk[1]) - 1);
    let oldCount = 0, newCount = 0;
    while (i < lines.length && !lines[i].startsWith('@@')) {
      const line = lines[i++];
      assert.ok([' ', '+', '-'].includes(line[0]));
      if (line[0] !== '-') { assert.equal(input[cursor++], line.slice(1)); newCount++; }
      if (line[0] !== '+') { output.push(line.slice(1)); oldCount++; }
    }
    assert.equal(oldCount, Number(hunk[2]));
    assert.equal(newCount, Number(hunk[4]));
  }
  return [...output, ...input.slice(cursor)].join('\n') + '\n';
}

test('the pow patch reconstructs the pinned source and leaves all ECMAScript guards intact', () => {
  const candidate = evidence.candidate, after = read(candidate.file);
  const patch = read(`patches/${candidate.patch}`), before = reversePatch(after, patch);
  assert.equal(evidence.baselineCommit, upstream.sourceCommit);
  assert.equal(sha256(patch), candidate.patchSha256);
  assert.equal(sha256(before), candidate.beforeSha256);
  assert.equal(candidate.beforeSha256, upstream.importedFiles[candidate.file]);
  assert.equal(sha256(after), candidate.afterSha256);
  const body = (source) => source.match(/pub fn math_pow\([\s\S]*?\n\}/)[0];
  assert.deepEqual(body(after), body(before).replace('x.powf(y)', 'libm::pow(x, y)'));
});

test('the shared software kernel has an exact version and package checksum', () => {
  const dependency = evidence.dependency;
  assert.equal(dependency.version, '0.2.16');
  assert.equal(dependency.license, 'MIT');
  assert.equal(dependency.defaultFeatures, false);
  assert.match(read('crates/foundation/Cargo.toml'), /^libm = \{ version = "=0\.2\.16", default-features = false \}$/m);
  const locked = read('Cargo.lock').split('[[package]]').find((entry) => /^name = "libm"$/m.test(entry));
  assert.ok(locked);
  assert.match(locked, /^version = "0\.2\.16"$/m);
  assert.equal(locked.match(/^checksum = "([a-f0-9]+)"$/m)[1], dependency.packageSha256);
});

test('every exact Phoenix override refers to an unchanged recorded call in the actual dispatcher', () => {
  const tables = [
    ['shared.color', 'crates/foundation/src/vectors/mod.rs', 'COLOR_FNS'],
    ['shared.inline-ignores', 'crates/foundation/src/vectors/mod.rs', 'INLINE_IGNORE_FNS'],
    ['rules.checks', 'crates/foundation/src/vectors/checks_a.rs', 'KNOWN_FNS'],
    ['rules.checks', 'crates/foundation/src/vectors/checks_b.rs', 'KNOWN_FNS'],
    ['rules.checks', 'crates/core/src/checks/vectors_a.rs', 'KNOWN'],
    ['rules.checks', 'crates/core/src/checks/vectors_b.rs', 'KNOWN'],
  ];
  assert.deepEqual(evidence.recordedFunctions, tables.map(([module, file, name]) => {
    const functions = constant(file, name);
    return { module, functions: name === 'KNOWN' ? functions.slice(1) : functions };
  }));
  const expectedSources = evidence.recordedFunctions.flatMap(({ module, functions }) =>
    functions.map((name) => `tests/oracle/vectors/calls/${module}/${name}.jsonl`))
    .filter((file) => fs.existsSync(path.join(workspace, file)));
  assert.deepEqual(Object.keys(evidence.recordedSources).sort(), expectedSources.sort());
  for (const [file, hash] of Object.entries(evidence.recordedSources)) {
    assert.equal(hash, upstream.importedFiles[file], file);
    assert.equal(sha256(read(file)), hash, file);
  }
  const calls = recordedCalls(), used = new Set(), counts = {};
  assert.equal(calls.length, 8321);
  assert.equal(evidence.overrides.length, 27);
  for (const override of evidence.overrides) {
    const source = calls[override.index];
    assert.ok(source);
    for (const key of ['module', 'name', 'file', 'line', 'args']) assert.deepEqual(override[key], source[key]);
    assert.equal(override.argsSha256, sha256(JSON.stringify(source.args)));
    assert.equal(override.sourceSha256, evidence.recordedSources[source.file]);
    assert.deepEqual(override.oldResult, source.result);
    assert.equal(override.module, 'shared.color');
    assert.ok(['relativeLuminance', 'contrastRatio'].includes(override.name));
    assert.ok(Number.isFinite(override.newResult));
    assert.notEqual(override.newResult, override.oldResult);
    assert.equal(override.oldBits, bits(override.oldResult));
    assert.equal(override.newBits, bits(override.newResult));
    assert.equal(used.has(override.index), false);
    used.add(override.index);
    counts[override.name] = (counts[override.name] || 0) + 1;
  }
  assert.deepEqual(counts, { relativeLuminance: 13, contrastRatio: 14 });
});

test('the 34 Rust edge cases include exact NaN, infinities, signed zero and subnormal behavior', () => {
  const scalar = (text) => {
    const constants = { 'f64::NAN': NaN, 'f64::INFINITY': Infinity, 'f64::NEG_INFINITY': -Infinity,
      'f64::MAX': Number.MAX_VALUE, 'f64::from_bits(1)': Number.MIN_VALUE };
    if (Object.hasOwn(constants, text)) return constants[text];
    return text.split('/').map(Number).reduce((value, divisor) => value / divisor);
  };
  const decode = (value) => value?.$nan ? NaN : value?.$inf ? value.$inf * Infinity : value?.$negzero ? -0 : value;
  const array = read(evidence.candidate.file).match(/fn math_pow_preserves_ecmascript_special_cases\(\)[\s\S]*?= &\[([\s\S]*?)\n        \];/)[1];
  const cases = [...array.matchAll(/^\s*\((.+), (.+), (.+)\),$/gm)].map((match) => match.slice(1).map(scalar));
  assert.equal(cases.length, 34);
  assert.equal(evidence.candidate.specialCases.length, cases.length);
  evidence.candidate.specialCases.forEach(({ args, expected }, i) => {
    const decoded = [...args, expected].map(decode);
    assert.deepEqual(decoded, cases[i]);
    assert.deepEqual(Math.pow(...decoded.slice(0, 2)), decoded[2]);
  });
});

test('the 99 probes cross the actual contrast, dark-background, gamma and RGB rounding boundaries', () => {
  const probes = evidence.boundaries.probes;
  assert.equal(probes.length, 99);
  const classify = (probe, value) => probe.name === 'contrastRatio'
    ? value < Number(probe.id.match(/^contrast-([^/]+)/)[1])
    : probe.name === 'relativeLuminance' ? (probe.id.startsWith('dark-luminance/') ? value < 0.1 : null) : canonical(value);
  let scalarChanges = 0;
  for (const probe of probes) {
    assert.deepEqual(canonical(probe.baselineWasm), canonical(probe.candidateNative), probe.id);
    assert.deepEqual(classify(probe, probe.baselineNative), classify(probe, probe.candidateNative), probe.id);
    assert.equal(probe.classificationUnchanged, true);
    if (typeof probe.baselineNative === 'number' && !Object.is(probe.baselineNative, probe.candidateNative)) scalarChanges++;
  }
  assert.equal(scalarChanges, 1);
  assert.deepEqual(evidence.boundaries.baselineClassificationChanges, []);
  for (const name of ['checkColors', 'checkHoverContrast', 'checkGlow', 'cssTextHasDarkRootBg']) {
    const decisions = new Set(probes.filter((probe) => probe.name === name).map((probe) =>
      typeof probe.candidateNative === 'boolean' ? probe.candidateNative : probe.candidateNative.length > 0));
    assert.deepEqual(decisions, new Set([true, false]), name);
  }
  for (const threshold of [3, 4.5]) assert.deepEqual(new Set(probes.filter((p) => p.id.startsWith(`contrast-${threshold}/`))
    .map((p) => p.candidateNative < threshold)), new Set([true, false]));
  assert.deepEqual(new Set(probes.filter((p) => p.id.startsWith('gamma-branch/')).map((p) => p.args[0].r / 255 <= 0.03928)), new Set([true, false]));
  assert.deepEqual(new Set(probes.filter((p) => p.name === 'oklabToRgb').map((p) => p.candidateNative.r)), new Set([127, 128]));
});

test('the sole Italic delta is a finding missing from the directory golden but present in the single-file golden', () => {
  const italic = evidence.italic;
  const findings = (file) => JSON.parse(JSON.parse(read(file)).stdout);
  const single = findings(italic.singleFile), directory = findings(italic.directoryFile)
    .filter((finding) => finding.file === '<REPO>/tests/fixtures/antipatterns/italic-serif-display.html');
  for (const [file, hash] of [[italic.singleFile, italic.singleFileSha256], [italic.directoryFile, italic.directoryFileSha256]]) {
    assert.equal(sha256(read(file)), hash);
    assert.equal(upstream.importedFiles[file], hash);
  }
  const missing = single.filter((finding) => !directory.some((other) => JSON.stringify(other) === JSON.stringify(finding)));
  assert.equal(single.length, 8);
  assert.equal(directory.length, 7);
  assert.deepEqual(missing, italic.missingFromDirectory);
  assert.deepEqual(missing, italic.directoryActualOnly);
  assert.equal(missing.length, 1);
  assert.equal(missing[0].snippet, 'italic serif h1 (fraunces) at 72px "Inline Em Inside Roman"');
  assert.equal(italic.classification, 'stale-directory-golden');
});

test('independent specification vectors cover exactly all ten interfaces missing historical recordings', () => {
  const supplemental = evidence.supplemental;
  assert.equal(supplemental.kind, 'phoenix-specification-vectors');
  assert.equal(supplemental.probes.length, 54);
  assert.equal(supplemental.cases, supplemental.probes.length);
  const keys = supplemental.interfaces.map(({ module, name }) => `${module}/${name}`).sort();
  assert.deepEqual(keys, [...evidence.recordedFunctionsWithoutVectors].sort());
  const seen = new Set();
  for (const entry of supplemental.interfaces) {
    assert.equal(entry.baselineSha256, upstream.importedFiles[entry.file]);
    const probes = supplemental.probes.filter((probe) => probe.module === entry.module && probe.name === entry.name);
    assert.ok(probes.length >= 5, `${entry.module}/${entry.name}`);
    for (const probe of probes) {
      assert.ok(Array.isArray(probe.args));
      assert.ok(Object.hasOwn(probe, 'expected'));
      assert.equal(seen.has(probe.id), false);
      seen.add(probe.id);
    }
  }
  assert.equal(seen.size, supplemental.probes.length);
  const covered = new Set(recordedCalls().map(({ module, name }) => `${module}/${name}`));
  for (const key of keys) covered.add(key);
  const exported = evidence.recordedFunctions.flatMap(({ module, functions }) => functions.map((name) => `${module}/${name}`));
  assert.equal(exported.length, 72);
  assert.deepEqual([...covered].sort(), exported.sort());
});

const runtimeNames = ['PHOENIX_UI_NUMERIC_VECTORS_BIN', 'PHOENIX_UI_NUMERIC_WASM_BINDINGS', 'PHOENIX_UI_NUMERIC_WASM_MODULE'];
if (runtimeNames.some((name) => process.env[name])) {
  test('actual native and WASM results match every exact Phoenix expectation and all boundary probes', () => {
    for (const name of runtimeNames) assert.ok(process.env[name], `Set ${name}; all three runtime artifacts are required`);
    const context = vm.createContext({ TextEncoder, TextDecoder, WebAssembly, console });
    vm.runInContext(fs.readFileSync(process.env[runtimeNames[1]], 'utf8'), context);
    context.bytes = fs.readFileSync(process.env[runtimeNames[2]]);
    vm.runInContext('wasm_bindgen.initSync({module:bytes})', context);
    const api = vm.runInContext('wasm_bindgen', context);
    assert.deepEqual(JSON.parse(api.pure_functions()), evidence.recordedFunctions);
    const overrides = new Map(evidence.overrides.map((entry) => [entry.index, entry.newResult]));
    const calls = recordedCalls().map((call, index) => ({ ...call, expected: overrides.has(index) ? overrides.get(index) : call.result ?? null }));
    calls.push(...evidence.boundaries.probes.map((probe) => ({ ...probe, expected: probe.candidateNative })));
    calls.push(...evidence.supplemental.probes);
    const native = spawnSync(process.env[runtimeNames[0]], [], {
      input: calls.map(({ module, name, args }) => JSON.stringify({ module, name, args: JSON.stringify(args) })).join('\n') + '\n',
      encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
    });
    if (native.error) throw native.error;
    assert.equal(native.status, 0, native.stderr);
    const results = native.stdout.trim().split('\n').map((line) => JSON.parse(JSON.parse(line).result));
    assert.equal(results.length, calls.length);
    calls.forEach((call, i) => {
      const wasm = JSON.parse(api.pure_call(call.module, call.name, JSON.stringify(call.args)));
      const label = call.id || `${call.file}:${call.line}`;
      assert.deepEqual(canonical(results[i]), canonical(call.expected), `native ${label}`);
      assert.deepEqual(canonical(wasm), canonical(call.expected), `WASM ${label}`);
    });
  });
}
