/**
 * Plan §6 rows 3 and 4, Chrome: variant accept/discard, workspace move, and
 * generation failure / partial results, driven through the Phoenix live
 * overlay on the vite8-react-plain fixture with the deterministic agent.
 *
 * Row 3 「接受/取消变体；移动 workspace 后恢复」 (one staged project whose
 * index.html carries a CSP meta, so the CSP patch and its revert are real):
 *   - Discard restores src/App.jsx byte-for-byte.
 *   - Accept leaves only the chosen variant's markup (each variant carries a
 *     visible " vN" marker) and no variant/carbonize/scope residue.
 *   - `live-server stop` leaves index.html byte-identical to HEAD: no script,
 *     no CSP patch, no data-impeccable-csp-original.
 *   - The project directory is renamed while nothing runs, live starts from
 *     the new path and runs a session; then the helper is killed (abnormal
 *     exit), the directory is renamed again, and live starts and stops from
 *     the second path. No earlier path may reappear.
 *
 * Row 4 「生成服务失败/部分结果」 (a fresh project per case; the agent is the
 * local stand-in, no model is called):
 *   - error: the agent replies `error` before writing anything.
 *   - partial: the agent writes 1 of 3 variants and replies `done`.
 *   - partial-then-error: the agent writes 1 of 3 variants, then replies
 *     `error` (its `done` POST is rewritten in this process).
 *   Each case: failure is shown, the usable part stays usable, nothing is
 *   accepted on its own, the original markup survives, and the generate
 *   request is neither re-posted by the page nor re-leased by the agent.
 *
 * Run (cwd tools/phoenix-ui):
 *   IMPECCABLE_BIN=<phoenix-ui engine> PHOENIX_E2E_CHROME=<chrome> \
 *   PHOENIX_E2E_EVIDENCE_DIR=<abs dir> node --test --test-concurrency=1 \
 *     tests/live-e2e/phoenix-variants.test.mjs
 */

import { after, before, describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join, relative } from 'node:path';

import { createFakeAgent } from './agent.mjs';
import { startDevServer, stopDevServer } from './session.mjs';
import {
  browserInfo, createRecorder, engine, engineInfo, evidencePath, git, injectScript, launchChrome, openPhoenixPage,
  pidAlive, processRecord, removeNodeModulesTemplates, sourceState, stagePhoenixFixture, startAgent, startHelper,
  waitFor, writeEvidence,
} from './phoenix-harness.mjs';
import {
  clickAccept, clickDiscard, clickGo, cycleToVariant, installLiveQueryHelpers, pickElement, waitForBarHidden,
  waitForCycling, waitForHandshake,
} from './ui.mjs';

const ROW3 = 'row3-accept-discard-move';
const ROW4 = 'row4-generation-failure';
const FIXTURE = 'vite8-react-plain';
const APP = 'src/App.jsx';
const INDEX = 'index.html';
const ORIGINAL_H1 = '<h1 className="hero-title">Vite 8 Fixture</h1>';
const CSP = "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ws:";
const PROVIDER_ERROR = 'generation service unavailable (simulated 503)';
const PARTIAL_ERROR = 'generation service failed after 1 of 3 variants (simulated 503)';
// What the bar says once the reply settles 1 of 3: the rest failed, not "arriving".
const SHORTFALL_BAR = /1 of 3 arrived\b.*\b2 failed\b/;
const RESIDUE = /data-impeccable|impeccable-variants|impeccable-carbonize|@scope|data-impeccable-css/;
const OBSERVE_MS = 8_000;

const meta = {};
let browser;

// ---------------------------------------------------------------------------
// Local helpers (the harness boots one fixed session; these rows need a CSP
// baseline, restarts from a moved directory, and a scripted agent).
// ---------------------------------------------------------------------------

function headBytes(cwd, file) {
  return execFileSync('git', ['show', `HEAD:${file}`], { cwd });
}

function sameAsHead(cwd, file) {
  return Buffer.compare(readFileSync(join(cwd, file)), headBytes(cwd, file)) === 0;
}

function readText(cwd, file) {
  return readFileSync(join(cwd, file), 'utf-8');
}

/** A sibling of `path` that shares no prefix with it. */
function movedPath(path, tag) {
  return join(tmpdir(), `phoenix-e2e-moved-${tag}-${basename(path).slice('phoenix-e2e-'.length)}`);
}

function isOurTmp(path) {
  return typeof path === 'string' && path.startsWith(tmpdir()) && basename(path).startsWith('phoenix-e2e-');
}

function removeTmp(path) {
  if (isOurTmp(path) && existsSync(path)) rmSync(path, { recursive: true, force: true });
}

function listFiles(root, sub) {
  const dir = join(root, sub);
  if (!existsSync(dir)) return [];
  const out = [];
  const walk = (d) => {
    for (const name of readdirSync(d)) {
      const p = join(d, name);
      if (statSync(p).isDirectory()) walk(p);
      else out.push(relative(root, p));
    }
  };
  walk(dir);
  return out.sort();
}

function receipts(cwd) {
  return listFiles(cwd, '.phoenix-ui/live/accept-receipts').map((file) => {
    try { return { file, ...JSON.parse(readText(cwd, file)) }; } catch { return { file, unreadable: true }; }
  });
}

function stage({ csp }) {
  const { tmp, fixture } = stagePhoenixFixture(FIXTURE);
  if (csp) {
    const file = join(tmp, INDEX);
    const html = readFileSync(file, 'utf-8').replace(
      '<meta charset="UTF-8" />',
      `<meta charset="UTF-8" />\n    <meta http-equiv="Content-Security-Policy" content="${CSP}" />`,
    );
    writeFileSync(file, html);
    git(tmp, ['commit', '-qam', 'csp meta']);
  }
  return { tmp, fixture };
}

/** Helper -> inject -> dev server -> agent -> page, all from `tmp`. */
async function bootAt(tmp, fixture, { rec, agent, logs }) {
  const s = { tmp, fixture, live: null, dev: null, agent: null, ctx: null, page: null };
  s.live = startHelper(tmp);
  rec.trackLive(s.live);
  s.inject = injectScript(tmp, s.live);
  s.dev = startDevServer(tmp, fixture.runtime);
  try {
    const { port } = await s.dev.ready;
    s.url = `http://127.0.0.1:${port}/`;
  } catch (err) {
    await shutdown(s, { helper: 'stop' });
    throw err;
  }
  s.agent = startAgent({ cwd: tmp, live: s.live, rec, agent, log: (m) => logs.push({ t: rec.t(), m }) });
  const opened = await openPhoenixPage(browser, s.url, rec);
  s.ctx = opened.ctx;
  s.page = opened.page;
  await waitForHandshake(s.page);
  return s;
}

/** helper: 'stop' (normal `live-server stop`), 'kill' (SIGKILL) or 'none'. */
async function shutdown(s, { helper = 'stop' } = {}) {
  const out = {};
  try { if (s.ctx) await s.ctx.close(); } catch { /* closed */ }
  s.ctx = null;
  s.page = null;
  try { if (s.agent) await s.agent.stop(); } catch { /* stopped */ }
  s.agent = null;
  try { if (s.dev?.child) await stopDevServer(s.dev.child); } catch { /* gone */ }
  if (s.live?.pid && pidAlive(s.live.pid)) {
    if (helper === 'stop') {
      const res = engine('live-server', ['stop'], { cwd: s.tmp });
      out.stop = { code: res.code, stdout: res.stdout, stderr: res.stderr };
      await waitFor(() => !pidAlive(s.live.pid), { label: 'helper exit', timeout: 15_000 }).catch(() => {});
    } else if (helper === 'kill') {
      process.kill(s.live.pid, 'SIGKILL');
      await waitFor(() => !pidAlive(s.live.pid), { label: 'helper killed', timeout: 15_000 }).catch(() => {});
      out.killed = true;
    }
    if (helper !== 'none' && pidAlive(s.live.pid)) {
      try { process.kill(s.live.pid, 'SIGKILL'); } catch { /* gone */ }
    }
  }
  return out;
}

async function liveDebug(page) {
  return page.evaluate(() => {
    const d = window.__PHOENIX_UI_LIVE_CHROME_CORE__?.debugState?.();
    return d ? { state: d.state, sessionId: d.currentSessionId, expected: d.expectedVariants, arrived: d.arrivedVariants,
      visible: d.visibleVariant, barText: d.barText, barConnected: d.barConnected } : null;
  });
}

async function barSnapshot(page) {
  await installLiveQueryHelpers(page);
  return page.evaluate(() => {
    const bar = window.__impeccableLiveQuery('#impeccable-live-bar');
    const accept = bar ? [...bar.querySelectorAll('button')].find((b) => /Accept/.test(b.textContent || '')) : null;
    return {
      visible: !!bar && bar.style.display !== 'none',
      text: bar?.textContent?.replace(/\s+/g, ' ').trim() ?? null,
      acceptEnabled: accept ? accept.style.pointerEvents !== 'none' && !accept.disabled : null,
    };
  });
}

async function toastText(page) {
  await installLiveQueryHelpers(page);
  return page.evaluate(() => {
    const t = window.__impeccableLiveQuery('#impeccable-live-toast');
    return t && t.style.display !== 'none' ? (t.textContent || '').trim() : null;
  });
}

async function domWrappers(page) {
  await installLiveQueryHelpers(page);
  return page.evaluate(() => ({
    wrappers: document.querySelectorAll('[data-impeccable-variants]').length,
    h1: document.querySelector('h1.hero-title')?.textContent ?? null,
  }));
}

/** Variant N keeps the original tag and class, and says " vN" in its text. */
function markVariant(innerHtml, n) {
  return innerHtml.replace(/>([^<]*)</, (_m, text) => `>${text.replace(/ v\d+$/, '')} v${n}<`);
}

/**
 * The deterministic agent with a switchable generation outcome:
 *   ok                all three variants
 *   error             throws before writing (agent replies `error`)
 *   partial           writes variant 1 only, replies `done`
 *   partialThenError  writes variant 1 only, then its `done` becomes `error`
 */
function scriptedAgent(rec, control) {
  const base = createFakeAgent();
  return {
    ...base,
    async generateVariants(event, context) {
      const scenario = control.scenario;
      control.generateCalls.push({ t: rec.t(), id: event.id, scenario, count: event.count });
      rec.agentEvents.push({ t: rec.t(), event: 'agent.generate.payload', id: event.id, count: event.count, scenario });
      if (scenario === 'error') throw new Error(PROVIDER_ERROR);
      const out = await base.generateVariants(event, context);
      const variants = out.variants.map((v, i) => ({ ...v, innerHtml: markVariant(v.innerHtml, i + 1) }));
      if (scenario === 'partial' || scenario === 'partialThenError') {
        if (scenario === 'partialThenError') control.failAfterWrite.add(event.id);
        return { ...out, variants: variants.slice(0, 1) };
      }
      return { ...out, variants };
    },
  };
}

// The agent loop replies through global fetch; for partialThenError its
// generate `done` (sent after the partial write) is turned into `error`.
const realFetch = globalThis.fetch;
const replyRewrites = { control: null, rec: null };
globalThis.fetch = async (input, init = {}) => {
  const { control, rec } = replyRewrites;
  if (control && init?.method === 'POST' && typeof init.body === 'string' && /\/poll$/.test(String(input))) {
    let body = null;
    try { body = JSON.parse(init.body); } catch { /* not JSON */ }
    if (body?.type === 'done' && body.sourceEventType === 'generate' && control.failAfterWrite.has(body.id)) {
      rec?.agentEvents.push({ t: rec.t(), event: 'agent.reply.rewritten', id: body.id, from: 'done', to: 'error', message: PARTIAL_ERROR });
      return realFetch(input, { ...init, body: JSON.stringify({ token: body.token, type: 'error', sourceEventType: 'generate', id: body.id, message: PARTIAL_ERROR }) });
    }
  }
  return realFetch(input, init);
};

function allTrue(checks) {
  return Object.fromEntries(Object.keys(checks).map((k) => [k, true]));
}

before(async () => {
  meta.engine = engineInfo();
  browser = await launchChrome();
  meta.browser = browserInfo(browser);
  meta.startedAt = new Date().toISOString();
});

after(async () => {
  globalThis.fetch = realFetch;
  if (browser) await browser.close();
  removeNodeModulesTemplates();
  writeEvidence(`${ROW3}/meta.json`, meta);
  writeEvidence(`${ROW4}/meta.json`, meta);
});

// ---------------------------------------------------------------------------
// Row 3
// ---------------------------------------------------------------------------

describe('row 3: accept / discard / move workspace', () => {
  const rec = createRecorder('row3');
  const logs = [];
  const control = { scenario: 'ok', generateCalls: [], failAfterWrite: new Set() };
  const results = [];
  const paths = [];
  let s;
  let fixture;
  let acceptedApp = null;

  before(async () => {
    const staged = stage({ csp: true });
    fixture = staged.fixture;
    paths.push(staged.tmp);
    s = await bootAt(staged.tmp, fixture, { rec, agent: scriptedAgent(rec, control), logs });
    results.push({ step: 'boot', tmp: s.tmp, inject: s.inject.json, index: readText(s.tmp, INDEX), helper: processRecord('helper', s.live) });
  });

  after(async () => {
    if (s) {
      const out = await shutdown(s, { helper: 'stop' });
      results.push({ step: 'final teardown', ...out, source: existsSync(s.tmp) ? sourceState(s.tmp) : null });
    }
    rec.save(`${ROW3}/events.json`);
    writeEvidence(`${ROW3}/results.json`, { paths, results, agentLog: logs, generateCalls: control.generateCalls });
    for (const p of paths) removeTmp(p);
  });

  it('discard restores the original source byte-for-byte and leaves no wrapper', async () => {
    const { page, tmp } = s;
    const before = sourceState(tmp);
    await pickElement(page, 'h1.hero-title');
    await clickGo(page);
    await waitForCycling(page, 3, { timeout: 60_000 });
    await cycleToVariant(page, 2, 3);
    const during = { source: readText(tmp, APP), debug: await liveDebug(page) };
    await page.screenshot({ path: evidencePath(ROW3, 'discard-1-cycling-v2.png') });
    await clickDiscard(page);
    await waitForBarHidden(page, { timeout: 15_000 });
    const restored = await waitFor(() => sameAsHead(tmp, APP), { label: 'App.jsx back to HEAD', timeout: 20_000 }).catch(() => false);
    const dom = await waitFor(async () => { const d = await domWrappers(page); return d.wrappers === 0 ? d : null; },
      { label: 'no wrapper in DOM', timeout: 10_000 }).catch(async () => domWrappers(page));
    await page.screenshot({ path: evidencePath(ROW3, 'discard-2-after.png') });
    const sessionId = during.debug?.sessionId;
    const receipt = receipts(tmp).find((r) => r.id === sessionId) || null;
    const checks = {
      cycledThreeVariants: [1, 2, 3].every((n) => during.source.includes(`Vite 8 Fixture v${n}`)),
      appByteIdenticalToHead: restored === true,
      noResidueInApp: !RESIDUE.test(readText(tmp, APP)),
      noWrapperInDom: dom.wrappers === 0,
      originalTextInDom: dom.h1 === 'Vite 8 Fixture',
      discardReceipt: receipt?.operation === 'discard',
      noAcceptPosted: rec.eventPosts('accept').length === 0,
    };
    results.push({ step: 'discard', sessionId, checks, before, after: sourceState(tmp), dom, receipt });
    assert.deepEqual(checks, allTrue(checks));
  });

  it('accept leaves only the chosen variant and no variant, carbonize or scope residue', async () => {
    const { page, tmp } = s;
    await pickElement(page, 'h1.hero-title');
    await clickGo(page);
    await waitForCycling(page, 3, { timeout: 60_000 });
    await cycleToVariant(page, 2, 3);
    const debug = await liveDebug(page);
    await page.screenshot({ path: evidencePath(ROW3, 'accept-1-cycling-v2.png') });
    await clickAccept(page, { expectedVariant: 2 });
    await waitForBarHidden(page, { timeout: 20_000 });
    const sessionId = debug?.sessionId;
    const settled = await waitFor(() => {
      const src = readText(tmp, APP);
      return !RESIDUE.test(src) && src.includes('Vite 8 Fixture v2') ? src : null;
    }, { label: 'accepted source settled', timeout: 30_000 }).catch(() => null);
    const completed = await waitFor(() => logs.some((l) => l.m === `completed carbonize session ${sessionId}`),
      { label: 'live-complete', timeout: 30_000 }).catch(() => false);
    const dom = await waitFor(async () => { const d = await domWrappers(page); return d.wrappers === 0 && d.h1 === 'Vite 8 Fixture v2' ? d : null; },
      { label: 'accepted DOM', timeout: 15_000 }).catch(async () => domWrappers(page));
    await page.screenshot({ path: evidencePath(ROW3, 'accept-2-after.png') });
    const app = readText(tmp, APP);
    const head = headBytes(tmp, APP).toString('utf-8');
    const chosenLine = '<h1 className="hero-title">Vite 8 Fixture v2</h1>';
    const receipt = receipts(tmp).find((r) => r.id === sessionId) || null;
    const status = await engine('live-status', [], { cwd: tmp });
    const checks = {
      sourceSettled: settled !== null,
      liveCompleteRan: completed === true,
      chosenVariantPresent: app.includes(chosenLine),
      otherVariantsAbsent: !/Vite 8 Fixture v[13]\b/.test(app),
      noResidueInApp: !RESIDUE.test(app),
      onlyTheH1Changed: app.replace(chosenLine, ORIGINAL_H1) === head,
      noWrapperInDom: dom.wrappers === 0,
      chosenTextInDom: dom.h1 === 'Vite 8 Fixture v2',
      acceptReceipt: receipt?.operation === 'accept' && String(receipt?.variantId) === '2',
      oneAcceptPosted: rec.eventPosts('accept').length === 1,
    };
    acceptedApp = readFileSync(join(tmp, APP));
    results.push({ step: 'accept', sessionId, checks, after: sourceState(tmp), dom, receipt, liveStatus: status.json ?? status.stdout });
    assert.deepEqual(checks, allTrue(checks));
  });

  it('live-server stop leaves no injected script or CSP residue', async () => {
    const { tmp } = s;
    const helper = s.live;
    const injected = readText(tmp, INDEX);
    const out = await shutdown(s, { helper: 'stop' });
    const index = readText(tmp, INDEX);
    const status = git(tmp, ['status', '--porcelain', '--untracked-files=all']);
    const tracked = status.split('\n').filter((l) => l && !l.startsWith('??'));
    const untracked = status.split('\n').filter((l) => l.startsWith('?? ')).map((l) => l.slice(3));
    const cspViolations = rec.console.filter((c) => /Content Security Policy/i.test(c.text));
    const checks = {
      stopExitedZero: out.stop?.code === 0,
      helperGone: !pidAlive(helper.pid),
      injectHadPatchedCsp: injected.includes('data-impeccable-csp-original') && injected.includes(`http://localhost:${helper.port}`),
      indexByteIdenticalToHead: sameAsHead(tmp, INDEX),
      noCspMarker: !index.includes('data-impeccable-csp-original'),
      noLiveScript: !/live\.js|impeccable-live|phoenix-ui-live/i.test(index),
      onlyAppTracked: tracked.length === 1 && tracked[0] === ` M ${APP}`,
      untrackedOnlyUnderPhoenixDir: untracked.every((f) => f.startsWith('.phoenix-ui/')),
      noCspViolations: cspViolations.length === 0,
    };
    results.push({ step: 'stop', checks, stop: out.stop, injectedIndex: injected, indexAfterStop: index, gitStatus: status, cspViolations,
      gitInfoExcludeAfterStop: readText(tmp, '.git/info/exclude'),
      helper: processRecord('stopped helper', helper) });
    assert.deepEqual(checks, allTrue(checks));
  });

  it('after the project moves, live runs from the new path and never writes the old one', async () => {
    const oldPath = s.tmp;
    const moved = movedPath(oldPath, 'a');
    renameSync(oldPath, moved);
    paths.push(moved);
    s = await bootAt(moved, fixture, { rec, agent: scriptedAgent(rec, control), logs });
    const page = s.page;
    await pickElement(page, 'h1.hero-title');
    await clickGo(page);
    await waitForCycling(page, 3, { timeout: 60_000 });
    await cycleToVariant(page, 3, 3);
    await page.screenshot({ path: evidencePath(ROW3, 'move-1-cycling.png') });
    await clickDiscard(page);
    await waitForBarHidden(page, { timeout: 15_000 });
    const restored = await waitFor(() => Buffer.compare(readFileSync(join(moved, APP)), acceptedApp) === 0,
      { label: 'App.jsx back to the accepted bytes', timeout: 20_000 }).catch(() => false);
    const status = engine('live-status', [], { cwd: moved });
    const serverJson = existsSync(join(moved, '.phoenix-ui/live/server.json')) ? readText(moved, '.phoenix-ui/live/server.json') : null;
    const helper = s.live;
    // Abnormal exit at the new path: the injected script stays behind.
    const out = await shutdown(s, { helper: 'kill' });
    const indexAfterKill = readText(moved, INDEX);
    const checks = {
      oldPathAbsent: !existsSync(oldPath),
      discardRestoredAcceptedBytes: restored === true,
      noResidueInApp: !RESIDUE.test(readText(moved, APP)),
      statusNamesNoOldPath: !String(status.stdout).includes(oldPath),
      killLeftInjection: /data-impeccable-csp-original/.test(indexAfterKill),
    };
    results.push({ step: 'move after clean stop', oldPath, moved, checks, liveStatus: status.json ?? status.stdout, serverJson,
      helper: processRecord('killed helper', helper), out, indexAfterKill, phoenixFiles: listFiles(moved, '.phoenix-ui') });
    assert.deepEqual(checks, allTrue(checks));
  });

  it('after an abnormal exit and another move, live recovers at the new path and stops clean', async () => {
    const [first, second] = paths;
    const moved2 = movedPath(first, 'b');
    renameSync(second, moved2);
    paths.push(moved2);
    const statusBefore = engine('live-status', [], { cwd: moved2 });
    const journal = existsSync(join(moved2, '.phoenix-ui/live/inject-journal.json')) ? readText(moved2, '.phoenix-ui/live/inject-journal.json') : null;
    s = await bootAt(moved2, fixture, { rec, agent: scriptedAgent(rec, control), logs });
    const start = s.live.result.json;
    const inject = s.inject.json;
    await s.page.screenshot({ path: evidencePath(ROW3, 'move-2-recovered.png') });
    const helper = s.live;
    const out = await shutdown(s, { helper: 'stop' });
    const index = readText(moved2, INDEX);
    const phoenixFiles = listFiles(moved2, '.phoenix-ui');
    // macOS tmpdir is a symlink: the engine records realpaths (/private/var/...).
    const oldForms = [first, second].flatMap((p) => [p, `/private${p}`]);
    const mentionsOld = phoenixFiles.filter((f) => {
      try { const t = readText(moved2, f); return oldForms.some((p) => t.includes(p)); } catch { return false; }
    });
    const checks = {
      firstPathAbsent: !existsSync(first),
      secondPathAbsent: !existsSync(second),
      stopExitedZero: out.stop?.code === 0,
      indexByteIdenticalToHead: sameAsHead(moved2, INDEX),
      appKeepsAcceptedBytes: Buffer.compare(readFileSync(join(moved2, APP)), acceptedApp) === 0,
    };
    results.push({ step: 'move after abnormal exit', first, second, moved2, checks, statusBefore: statusBefore.json ?? statusBefore.stdout,
      journalBefore: journal, start, inject, stop: out.stop, helper: processRecord('stopped helper', helper), indexAfterStop: index,
      gitStatus: git(moved2, ['status', '--porcelain', '--untracked-files=all']), phoenixFiles, filesMentioningOldPaths: mentionsOld });
    assert.deepEqual(checks, allTrue(checks));
  });
});

// ---------------------------------------------------------------------------
// Row 4
// ---------------------------------------------------------------------------

describe('row 4: generation failure and partial results', () => {
  const runs = [];

  after(() => {
    replyRewrites.control = null;
    replyRewrites.rec = null;
    writeEvidence(`${ROW4}/results.json`, runs);
  });

  /** One fresh project per case: boot, pick, Go, then hand over. */
  async function withCase(name, scenario, body) {
    const rec = createRecorder(`row4-${name}`);
    const logs = [];
    const control = { scenario, generateCalls: [], failAfterWrite: new Set() };
    replyRewrites.control = control;
    replyRewrites.rec = rec;
    const { tmp, fixture } = stage({ csp: false });
    const run = { case: name, scenario, tmp };
    runs.push(run);
    let s;
    try {
      s = await bootAt(tmp, fixture, { rec, agent: scriptedAgent(rec, control), logs });
      await pickElement(s.page, 'h1.hero-title');
      await clickGo(s.page);
      await body({ s, rec, control, run });
    } finally {
      if (s) run.teardown = await shutdown(s, { helper: 'stop' });
      run.sourceAfterTeardown = existsSync(tmp) ? sourceState(tmp) : null;
      run.agentLog = logs;
      run.generateCalls = control.generateCalls;
      rec.save(`${ROW4}/events-${name}.json`);
      removeTmp(tmp);
    }
  }

  function sseOf(rec, type, id) {
    return rec.sse.filter((e) => e.data?.type === type && (!id || e.data?.id === id));
  }

  /** Watch the toast for `needle` while the page settles. */
  async function catchToast(page, needle, timeout = 20_000) {
    return waitFor(async () => {
      const t = await toastText(page);
      return t && t.includes(needle) ? t : null;
    }, { label: `toast "${needle}"`, timeout, interval: 100 }).catch(() => null);
  }

  it('an error reply is shown, nothing is accepted, the source is untouched and nothing is re-sent', async () => {
    await withCase('error', 'error', async ({ s, rec, control, run }) => {
      const { page, tmp } = s;
      const toast = await catchToast(page, PROVIDER_ERROR);
      await page.screenshot({ path: evidencePath(ROW4, 'error-1-toast.png') });
      await page.waitForTimeout(OBSERVE_MS);
      const debug = await liveDebug(page);
      const bar = await barSnapshot(page);
      await page.screenshot({ path: evidencePath(ROW4, 'error-2-settled.png') });
      const checks = {
        failureShown: toast !== null,
        leftGenerating: debug?.state !== 'GENERATING',
        backToPicking: debug?.state === 'PICKING' && !bar.visible,
        appByteIdenticalToHead: sameAsHead(tmp, APP),
        noAcceptPosted: rec.eventPosts('accept').length === 0,
        // Nothing reached the source, so there is nothing for a discard to undo.
        noDiscardPosted: rec.eventPosts('discard').length === 0,
        oneGeneratePost: rec.eventPosts('generate').length === 1,
        oneAgentGenerate: control.generateCalls.length === 1,
      };
      Object.assign(run, { checks, toast, debug, bar, errorSse: sseOf(rec, 'error'), source: sourceState(tmp), receipts: receipts(tmp) });
      assert.deepEqual(checks, allTrue(checks));
    });
  });

  it('a partial done keeps the usable variant, marks the rest as failed and accepts nothing on its own', async () => {
    await withCase('partial', 'partial', async ({ s, rec, control, run }) => {
      const { page, tmp } = s;
      const sessionId = await waitFor(() => control.generateCalls[0]?.id, { label: 'generate leased', timeout: 20_000 });
      await waitFor(() => sseOf(rec, 'done', sessionId).length > 0, { label: 'done SSE', timeout: 30_000 });
      await waitFor(async () => { const d = await liveDebug(page); return d?.state === 'CYCLING' && d.arrived === 1 ? d : null; },
        { label: 'cycling with 1 variant', timeout: 20_000 }).catch(() => null);
      const toast = await catchToast(page, '1 of 3 variants');
      await page.screenshot({ path: evidencePath(ROW4, 'partial-0-toast.png') });
      await page.waitForTimeout(OBSERVE_MS);
      const debug = await liveDebug(page);
      const bar = await barSnapshot(page);
      const appDuring = readText(tmp, APP);
      await page.screenshot({ path: evidencePath(ROW4, 'partial-1-after-done.png') });
      const checks = {
        usableVariantShown: debug?.state === 'CYCLING' && debug.arrived === 1 && /1\s*\/\s*1/.test(bar.text || ''),
        usableVariantAcceptable: bar.acceptEnabled === true,
        missingVariantsNotStillArriving: !/more arriving/i.test(bar.text || ''),
        shortfallStatedInBar: SHORTFALL_BAR.test(bar.text || ''),
        shortfallToastShown: toast !== null && /failed/.test(toast),
        notAutoAccepted: rec.eventPosts('accept').length === 0 && receipts(tmp).length === 0,
        originalMarkupKept: appDuring.includes(ORIGINAL_H1),
        oneGeneratePost: rec.eventPosts('generate').length === 1,
        oneAgentGenerate: control.generateCalls.length === 1,
      };
      // The user's way out: discard must still restore the original exactly.
      let discardRestored = null;
      if (bar.visible) {
        await clickDiscard(page);
        await waitForBarHidden(page, { timeout: 15_000 }).catch(() => {});
        discardRestored = await waitFor(() => sameAsHead(tmp, APP), { label: 'discard restored', timeout: 20_000 }).catch(() => false);
      }
      checks.discardRestoresOriginal = discardRestored === true;
      Object.assign(run, { sessionId, checks, toast, debug, bar, appDuring, doneSse: sseOf(rec, 'done', sessionId), receipts: receipts(tmp), source: sourceState(tmp) });
      assert.deepEqual(checks, allTrue(checks));
    });
  });

  it('a partial write followed by an error keeps the usable variant and the original, and re-sends nothing', async () => {
    await withCase('partial-then-error', 'partialThenError', async ({ s, rec, control, run }) => {
      const { page, tmp } = s;
      const sessionId = await waitFor(() => control.generateCalls[0]?.id, { label: 'generate leased', timeout: 20_000 });
      const toast = await catchToast(page, PARTIAL_ERROR, 30_000);
      await page.screenshot({ path: evidencePath(ROW4, 'partial-error-1-toast.png') });
      await page.waitForTimeout(OBSERVE_MS);
      const debug = await liveDebug(page);
      const bar = await barSnapshot(page);
      const appDuring = readText(tmp, APP);
      await page.screenshot({ path: evidencePath(ROW4, 'partial-error-2-settled.png') });
      const checks = {
        failureShown: toast !== null,
        usableVariantKept: debug?.state === 'CYCLING' && debug.arrived === 1 && bar.visible,
        usableVariantAcceptable: bar.acceptEnabled === true,
        shortfallStatedInBar: SHORTFALL_BAR.test(bar.text || ''),
        notAutoAccepted: rec.eventPosts('accept').length === 0 && receipts(tmp).length === 0,
        originalMarkupKept: appDuring.includes(ORIGINAL_H1),
        oneGeneratePost: rec.eventPosts('generate').length === 1,
        oneAgentGenerate: control.generateCalls.length === 1,
      };
      // Way out: the bar's discard when it is still there; otherwise the
      // engine discard an agent would run for the orphaned wrapper.
      let restoredBy = null;
      if (bar.visible) {
        await clickDiscard(page);
        await waitForBarHidden(page, { timeout: 15_000 }).catch(() => {});
        restoredBy = 'bar discard';
      } else if (RESIDUE.test(appDuring)) {
        const res = engine('live-accept', ['--id', sessionId, '--discard'], { cwd: tmp });
        run.engineDiscard = { code: res.code, json: res.json, stderr: res.stderr };
        restoredBy = 'live-accept --discard';
      }
      const restored = await waitFor(() => sameAsHead(tmp, APP), { label: 'original restored', timeout: 20_000 }).catch(() => false);
      checks.originalRestorable = restored === true;
      // The page itself must resolve the wrapper; the engine discard is only the fallback evidence.
      checks.restoredByBarDiscard = restoredBy === 'bar discard';
      Object.assign(run, { sessionId, checks, toast, debug, bar, appDuring, restoredBy, errorSse: sseOf(rec, 'error', sessionId),
        receipts: receipts(tmp), source: sourceState(tmp) });
      assert.deepEqual(checks, allTrue(checks));
    });
  });
});
