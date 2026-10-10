/** Pinned oracle adaptation; native output retains only documented machine-identity masks. */
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { nativeInvocation, isolatedCases, offlineNetworkProfile } from './network.mjs';
import { reserveCritiqueCollisions, removeCritiqueCollisions } from './critique-clock.mjs';
import { allCases as fixedCases, diffResults, readGolden, caseRunsHere, ORACLE_DIR, REPO_ROOT } from '../lib.mjs';

export { diffResults, readGolden, caseRunsHere, ORACLE_DIR, REPO_ROOT };
export { nativeInvocation, isolatedCases, offlineNetworkProfile };
export const ROOT = path.resolve(REPO_ROOT, '../..');
export const SKILL_SOURCE = path.join(ROOT, 'skills/phoenix-ui/src');
export const sha256 = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');
export const metadata = JSON.parse(fs.readFileSync(path.join(ORACLE_DIR, 'adapters/phoenix.json'), 'utf8'));

function readPinned(relative) {
  const bytes = fs.readFileSync(path.join(REPO_ROOT, relative));
  const upstream = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'UPSTREAM.json'), 'utf8'));
  if (sha256(bytes) !== upstream.importedFiles[relative]) throw new Error(`fixed oracle source changed: ${relative}`);
  return bytes.toString('utf8');
}

/** Edits have original JavaScript string offsets and exact before/after strings. */
export function applyEdits(source, edits, label) {
  let result = source;
  let previous = source.length;
  for (const edit of [...edits].sort((a, b) => b.offset - a.offset)) {
    if (!Number.isInteger(edit.offset) || edit.offset < 0 || edit.offset + edit.before.length > previous ||
        source.slice(edit.offset, edit.offset + edit.before.length) !== edit.before) {
      throw new Error(`invalid exact edit: ${label}:${edit.offset}`);
    }
    result = result.slice(0, edit.offset) + edit.after + result.slice(edit.offset + edit.before.length);
    previous = edit.offset;
  }
  return result;
}

export function verifyCorpus() {
  const upstream = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, 'UPSTREAM.json'), 'utf8'));
  if (upstream.sourceCommit !== metadata.sourceCommit) throw new Error('oracle source pin differs');
  const files = Object.entries(upstream.importedFiles).filter(([file]) =>
    file.startsWith('tests/oracle/') || file.startsWith('tests/fixtures/'));
  for (const [file, hash] of files) {
    if (sha256(fs.readFileSync(path.join(REPO_ROOT, file))) !== hash) throw new Error(`fixed corpus changed: ${file}`);
  }
  for (const entry of metadata.inputs) {
    if (upstream.importedFiles[entry.path] !== entry.beforeSha256) throw new Error(`unbound input patch: ${entry.path}`);
    const source = readPinned(entry.path);
    if (sha256(applyEdits(source, entry.edits, entry.path)) !== entry.afterSha256) throw new Error(`input patch hash differs: ${entry.path}`);
  }
  for (const entry of metadata.workspaces) {
    if (upstream.importedFiles[entry.path] !== entry.beforeSha256) throw new Error(`unbound workspace patch: ${entry.path}`);
    if (sha256(applyEdits(readPinned(entry.path), entry.edits, entry.path)) !== entry.afterSha256) throw new Error(`workspace patch hash differs: ${entry.path}`);
  }
  for (const entry of metadata.externalProjectFixture.files) {
    if (entry.sourceCommit !== upstream.sourceCommit || sha256(fs.readFileSync(path.join(REPO_ROOT, entry.path))) !== entry.sha256) throw new Error(`external root fixture changed: ${entry.path}`);
  }
  if (upstream.importedFiles[metadata.externalProjectFixture.target] !== metadata.externalProjectFixture.targetSha256) throw new Error('external project target is not pinned');
  if (JSON.stringify(metadata.nativeNetworkPreconditions.ids) !== JSON.stringify(isolatedCases) || metadata.nativeNetworkPreconditions.profile !== offlineNetworkProfile) throw new Error('native network precondition differs from the reviewed four cases');
  for (const entry of metadata.currentEvidenceFiles) if (sha256(fs.readFileSync(path.join(REPO_ROOT,entry.file))) !== entry.sha256) throw new Error(`current expectation source changed: ${entry.file}`);
  return { sourceCommit: upstream.sourceCommit, files: files.length };
}

const dataUrl = (source) => `data:text/javascript;base64,${Buffer.from(source).toString('base64')}`;
const moduleUrl = (file) => pathToFileURL(path.join(ORACLE_DIR, file)).href;

/** The original harness still owns staging, execution, normalization and diffing. */
export async function loadPhoenixCases(bin) {
  const original = await fixedCases();
  const input = new Map(metadata.inputs.map((entry) => [entry.path, entry]));
  const sourceFor = (relative) => {
    const source = readPinned(relative), entry = input.get(relative);
    return entry ? applyEdits(source, entry.edits, relative) : source;
  };
  const replaceOne = (source, before, after) => {
    if (source.split(before).length !== 2) throw new Error(`harness anchor changed: ${before}`);
    return source.replace(before, after);
  };
  let harness = readPinned('tests/oracle/lib.mjs');
  harness = `import { nativeInvocation } from ${JSON.stringify(moduleUrl('adapters/network.mjs'))};\n` + harness;
  harness = replaceOne(harness, "from '../lib/live-servers.mjs'", `from ${JSON.stringify(moduleUrl('../lib/live-servers.mjs'))}`);
  harness = replaceOne(harness, 'path.dirname(fileURLToPath(import.meta.url))', JSON.stringify(ORACLE_DIR));
  harness = replaceOne(harness, "const preview = path.join(full, '.impeccable-live');", "const preview = path.join(full, '.phoenix-ui-live');");
  harness = replaceOne(harness, "{ IMPECCABLE_SKILL_DIR: path.join(REPO_ROOT, 'skill'), IMPECCABLE_SELF: bin }", `{ PHOENIX_UI_SKILL_DIR: ${JSON.stringify(SKILL_SOURCE)}, PHOENIX_UI_SELF: bin, OPENAI_API_KEY: null }`);
  harness = replaceOne(harness, "[wsReal, '<WS>'], [ws, '<WS>'], [REPO_ROOT, '<REPO>'], [home, '<HOME>'],", `[wsReal, '<WS>'], [ws, '<WS>'], [${JSON.stringify(SKILL_SOURCE)}, '<PHOENIX_SKILL>'], [REPO_ROOT, '<REPO>'], [home, '<HOME>'],`);
  harness = `import { pathForms, stdinJson, windowsBinaryMasks, windowsClimbMask } from ${JSON.stringify(moduleUrl('adapters/windows.mjs'))};\n` + harness;
  harness = replaceOne(harness, 'if (needle) out = maskPath(out, needle, tag);', 'if (needle) for (const form of pathForms(needle)) out = maskPath(out, form, tag);');
  harness = replaceOne(harness, 'const bin = process.env.IMPECCABLE_BIN;', 'const bin = process.env.IMPECCABLE_BIN;\n    for (const form of ["\\\'" + bin + "\\\' hooks", "\\\"" + bin + "\\\" hooks", bin + " hooks"]) out = out.split(form).join("<HOOK_ADMIN_CMD>");\n    out = windowsBinaryMasks(out, bin);');
  // Carry the original machine-dependent relative-climb mask to the renamed state directory.
  harness = replaceOne(harness, '(?=\\.impeccable\\/)', '(?=\\.phoenix-ui\\/)');
  harness = replaceOne(harness, "'<UP_TO_ROOT>/');", "'<UP_TO_ROOT>/');\n  out = windowsClimbMask(out);");
  // Substitute inside stdin values; a Windows workspace path is not valid raw JSON text.
  harness = replaceOne(harness, 'sub(JSON.stringify(c.stdin))', 'stdinJson(c.stdin, sub)');
  harness = replaceOne(harness, '  return spawnSync(argv[0], argv.slice(1), {', '  const native = opts.impl === "bin" ? nativeInvocation(c.id, argv) : argv;\n  return spawnSync(native[0], native.slice(1), {');
  const harnessUrl = dataUrl(harness);
  const activeHarness = await import(harnessUrl);
  let helpers = sourceFor('tests/oracle/live-helpers.mjs');
  helpers = replaceOne(helpers, "from './lib.mjs'", `from ${JSON.stringify(harnessUrl)}`);
  const helpersUrl = dataUrl(helpers), adapted = [];
  for (const file of fs.readdirSync(path.join(ORACLE_DIR, 'cases')).sort()) {
    if (!file.endsWith('.mjs')) continue;
    let source = sourceFor(`tests/oracle/cases/${file}`);
    source = source.replaceAll("from '../lib.mjs'", `from ${JSON.stringify(harnessUrl)}`);
    source = source.replaceAll("from '../live-helpers.mjs'", `from ${JSON.stringify(helpersUrl)}`);
    const mod = await import(dataUrl(source));
    const cases = typeof mod.default === 'function' ? await mod.default() : mod.default;
    for (const item of Array.isArray(cases) ? cases : [cases]) {
      const entry = { ...item, sourceFile: file };
      if (entry.id === 'critique-write-then-read') {
        let reservations;
        const beforeWrite = entry.steps[2].setup, beforeRead = entry.steps[3].setup;
        entry.steps = entry.steps.map((step, index) => index === 2 ? { ...step, setup(ws) {
          if (beforeWrite) beforeWrite(ws);
          reservations = reserveCritiqueCollisions(ws, step.timeoutMs || entry.timeoutMs || 60_000);
        } } : index === 3 ? { ...step, setup(ws) {
          removeCritiqueCollisions(reservations);
          if (beforeRead) beforeRead(ws);
        } } : step);
      }
      const setup = entry.setup;
      const external = metadata.externalProjectFixture.ids.includes(entry.id);
      if (external) {
        const fixture = metadata.externalProjectFixture;
        const before = `<REPO>/${fixture.target}`, after = `<WS>/${fixture.directory}/${fixture.target}`;
        if (entry.args.filter((arg) => arg === before).length !== 1) throw new Error(`external fixture target changed: ${entry.id}`);
        entry.args = entry.args.map((arg) => arg === before ? after : arg);
        entry.normalize = [...(entry.normalize || []), [`<WS>/${fixture.directory}`, 'g', '<REPO>'],
          [String.raw`<WS>\\{1,2}` + fixture.directory.replaceAll('.', '\\.'), 'g', '<REPO>']];
      }
      entry.setup = (ws) => {
        adaptWorkspace(ws, entry.sourceFile === 'context.mjs', metadata.installedFixtureCases.includes(entry.id));
        if (setup) setup(ws);
        if (external) stageExternalProject(ws);
        applyPlatformPrecondition(entry.id, ws);
      };
      adapted.push(entry);
    }
  }
  const signature = (cases) => cases.map((c) => ({ id: c.id, sourceFile: c.sourceFile,
    verb: c.verb, workspace: c.workspace, platforms: c.platforms,
    steps: c.steps?.map((s) => ({ verb: s.verb, daemon: s.daemon })) }));
  if (JSON.stringify(signature(original)) !== JSON.stringify(signature(adapted))) throw new Error('adapter changed fixed case inventory or execution steps');
  if (adapted.length !== metadata.caseCount) throw new Error('fixed case count changed');
  // IMPECCABLE_BIN is a test-harness path mask, never a production fallback.
  process.env.IMPECCABLE_BIN = bin;
  return { cases: adapted, runCase: activeHarness.runCase, normalize: activeHarness.normalize };
}

function stageExternalProject(ws) {
  const fixture = metadata.externalProjectFixture;
  const directory = path.join(ws, fixture.directory);
  for (const entry of fixture.files) {
    const destination = path.join(directory, entry.currentPath);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(path.join(REPO_ROOT, entry.path), destination);
  }
  fs.mkdirSync(path.join(directory, '.git'));
  const target = path.join(directory, fixture.target);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(path.join(REPO_ROOT, fixture.target), target);
}

/** Establish the POSIX fixture facts listed in phoenix.json on another platform, or fail the case. */
export function applyPlatformPrecondition(id, ws, platform = process.platform) {
  const { livePid } = metadata.platformPreconditions;
  if (livePid.platform === platform && livePid.ids.includes(id)) {
    const file = path.join(ws, livePid.file);
    const state = JSON.parse(fs.readFileSync(file, 'utf8'));
    if (state.pid !== livePid.before) throw new Error(`live pid fixture changed: ${id}`);
    fs.writeFileSync(file, JSON.stringify({ ...state, pid: process.pid }));
  }
}

export const platformSkipReason = (id, platform = process.platform) =>
  metadata.platformPreconditions.skips.find((skip) => skip.id === id && skip.platform === platform)?.reason;

function adaptWorkspace(ws, preserveLegacyLive, installedFixture) {
  const walk = (directory) => {
    for (const ent of fs.readdirSync(directory, { withFileTypes: true })) {
      const before = path.join(directory, ent.name);
      const rename = ent.name === '.impeccable' || (!preserveLegacyLive && ['.impeccable-live', '.impeccable-live.json'].includes(ent.name));
      const installed = installedFixture && ent.name === 'impeccable' && path.basename(directory) === 'skills';
      const after = installed ? path.join(directory, 'phoenix-ui') : rename ? path.join(directory, ent.name.replace('.impeccable', '.phoenix-ui')) : before;
      if (before !== after) fs.renameSync(before, after);
      if (ent.isDirectory()) walk(after);
      else if (ent.isFile()) {
        const relative = path.relative(ws, after).split(path.sep).join('/');
        const fixture = metadata.workspaces.find((entry) => relative === entry.currentPath &&
          sha256(fs.readFileSync(after)) === entry.beforeSha256);
        if (fixture) fs.writeFileSync(after, applyEdits(fs.readFileSync(after, 'utf8'), fixture.edits, fixture.path));
      }
    }
  };
  walk(ws);
}

/** Exact case/field patches only; unrelated output bytes remain historical. */
export function phoenixExpected(id, golden, platform = process.platform) {
  const result = structuredClone(golden);
  if (id === metadata.italic.id) result.stdout = italicExpected(golden.stdout);
  const entry = metadata.expectations.find((item) => item.id === id);
  if (entry) {
    if (sha256(fs.readFileSync(path.join(ORACLE_DIR, 'golden', `${id}.json`))) !== entry.goldenSha256) throw new Error(`expectation source changed: ${id}`);
    for (const patch of [...entry.fields, ...(entry.platformFields || []).filter((patch) => patch.platform === platform)]) {
      const keys = patch.path;
      let parent = result;
      for (const key of keys.slice(0, -1)) parent = parent[key];
      const key = keys.at(-1), before = parent[key];
      if (sha256(before) !== patch.beforeSha256) throw new Error(`expectation field changed: ${id}/${keys.join('/')}`);
      parent[key] = applyEdits(before, patch.edits, `${id}/${keys.join('/')}`);
      if (sha256(parent[key]) !== patch.afterSha256) throw new Error(`expectation result hash differs: ${id}/${keys.join('/')}`);
    }
    if (entry.fileKeys) result.files = Object.fromEntries(Object.entries(result.files).map(([key, value]) => [entry.fileKeys[key] || key, value]));
    for (const patch of entry.values || []) {
      let parent = result;
      for (const key of patch.path.slice(0, -1)) parent = parent[key];
      const key = patch.path.at(-1);
      if (JSON.stringify(parent[key]) !== JSON.stringify(patch.before)) throw new Error(`expectation value changed: ${id}/${patch.path.join('/')}`);
      parent[key] = structuredClone(patch.after);
    }
    for (const patch of entry.fileRemovals || []) {
      if (sha256(result.files[patch.path]) !== patch.sha256) throw new Error(`removed file proof changed: ${id}/${patch.path}`);
      delete result.files[patch.path];
    }
  }
  return result;
}

export function italicExpected(stdout) {
  const delta = metadata.italic;
  for (const evidence of delta.goldenSources) {
    if (sha256(fs.readFileSync(path.join(REPO_ROOT, evidence.path))) !== evidence.sha256) throw new Error('Italic source golden changed');
  }
  const single = readGolden(delta.singleId);
  const singleFindings = JSON.parse(single.stdout);
  if (JSON.stringify(singleFindings.filter((f) => f.snippet === delta.finding.snippet)) !== JSON.stringify([delta.finding])) throw new Error('Italic finding no longer has exact single-file evidence');
  const findings = JSON.parse(stdout);
  if (findings.some((f) => JSON.stringify(f) === JSON.stringify(delta.finding))) throw new Error('Italic finding is already recorded');
  const subset = findings.filter((f) => f.file === delta.finding.file);
  if (subset.length !== delta.directorySubsetCount || singleFindings.length !== delta.singleFileCount ||
      JSON.stringify(singleFindings.filter((f) => f.snippet !== delta.finding.snippet)) !== JSON.stringify(subset)) throw new Error('Italic delta is not the one missing finding');
  const first = findings.findIndex((f) => f.file === delta.finding.file);
  const last = findings.findLastIndex((f) => f.file === delta.finding.file);
  if (last - first + 1 !== subset.length) throw new Error('Italic file findings are not contiguous');
  findings.splice(first, subset.length, ...singleFindings);
  return JSON.stringify(findings, null, 2) + '\n';
}
