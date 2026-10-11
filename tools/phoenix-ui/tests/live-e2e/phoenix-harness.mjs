/**
 * Phoenix UI adapter for the imported live-e2e harness.
 *
 * The upstream harness targets the original naming: `.impeccable/live`
 * config, `window.__IMPECCABLE_LIVE_*__` page globals and the Chromium
 * revision pinned by Playwright. Phoenix renamed the config directory and the
 * page globals but kept the `impeccable-*` DOM ids, classes and data
 * attributes. This module bridges only those differences, so the unchanged
 * upstream `ui.mjs` / `agent.mjs` helpers drive the Phoenix engine:
 *
 *   - stage a fixture with `.phoenix-ui/live/config.json`
 *   - alias `window.__IMPECCABLE_LIVE_*__` to `__PHOENIX_UI_LIVE_*__`
 *   - launch an explicit Chrome executable (PHOENIX_E2E_CHROME)
 *   - record browser -> helper requests, helper -> browser SSE messages and
 *     every event the deterministic agent leased
 *
 * Engine: IMPECCABLE_BIN (read by ../lib/engine-bin.mjs). Evidence:
 * PHOENIX_E2E_EVIDENCE_DIR.
 */

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createFakeAgent, runAgentLoop } from './agent.mjs';
import {
  ENGINE_BIN, FIXTURES_DIR, requireEngineBin, runInstall, startDevServer, stopDevServer,
} from './session.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const CHECKOUT_ROOT = resolve(__dirname, '..', '..', '..', '..');

// npm/npx must use the runner's Node even when mise leaves a system Node first.
const runnerNodeDir = dirname(process.execPath);
process.env.PATH = [runnerNodeDir, ...process.env.PATH.split(delimiter).filter(p => p !== runnerNodeDir)].join(delimiter);

export const EVIDENCE_DIR = process.env.PHOENIX_E2E_EVIDENCE_DIR
  || join(CHECKOUT_ROOT, 'local', 'phoenix-ui', 'm3-live-walkthrough');
export const CHROME_PATH = process.env.PHOENIX_E2E_CHROME
  || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
export { ENGINE_BIN };

const GLOBAL_SUFFIXES = [
  'INIT', 'STATE', 'CHROME_CORE', 'ADAPTER', 'UI_SURFACES', 'SESSION', 'DOM',
  'IGNORES', 'MOUNT_CONTRACT', 'BAR_HIDDEN', 'UI_ROOT',
];

/** Page init script: upstream global names read through to the Phoenix ones. */
export function phoenixGlobalAliases(suffixes) {
  for (const suffix of suffixes) {
    const from = `__IMPECCABLE_LIVE_${suffix}__`;
    const to = `__PHOENIX_UI_LIVE_${suffix}__`;
    if (Object.getOwnPropertyDescriptor(window, from)) continue;
    Object.defineProperty(window, from, {
      configurable: true,
      get() { return window[to]; },
      set(value) { window[to] = value; },
    });
  }
}

export function evidencePath(...parts) {
  const file = join(EVIDENCE_DIR, ...parts);
  mkdirSync(dirname(file), { recursive: true });
  return file;
}

export function writeEvidence(relPath, data) {
  const file = evidencePath(relPath);
  writeFileSync(file, typeof data === 'string' ? data : JSON.stringify(data, null, 2) + '\n');
  return file;
}

export function sha256File(file) {
  return createHash('sha256').update(readFileSync(file)).digest('hex');
}

export function engineInfo() {
  const bin = requireEngineBin();
  return {
    path: bin,
    sha256: sha256File(bin),
    version: execFileSync(bin, ['--version'], { encoding: 'utf-8' }).trim(),
  };
}

export async function launchChrome({ headless = true, args = [] } = {}) {
  const { chromium } = await import('playwright');
  const browser = await chromium.launch({ executablePath: CHROME_PATH, headless, args });
  return browser;
}

export function browserInfo(browser) {
  return { executablePath: CHROME_PATH, version: browser.version() };
}

// ---------------------------------------------------------------------------
// Staging
// ---------------------------------------------------------------------------

const nodeModulesTemplates = new Map();

/** Install a fixture's dependencies once per process; sessions copy them. */
export function nodeModulesTemplate(name) {
  if (nodeModulesTemplates.has(name)) return nodeModulesTemplates.get(name);
  const fixture = readFixture(name);
  const dir = mkdtempSync(join(tmpdir(), `phoenix-e2e-deps-${name}-`));
  cpSync(join(FIXTURES_DIR, name, 'files', 'package.json'), join(dir, 'package.json'));
  runInstall(dir, fixture.runtime.install);
  nodeModulesTemplates.set(name, dir);
  return dir;
}

export function removeNodeModulesTemplates() {
  for (const dir of nodeModulesTemplates.values()) rmSync(dir, { recursive: true, force: true });
  nodeModulesTemplates.clear();
}

export function readFixture(name) {
  return JSON.parse(readFileSync(join(FIXTURES_DIR, name, 'fixture.json'), 'utf-8'));
}

/**
 * Copy a framework fixture into a fresh git repo with the Phoenix live
 * config, and give it the shared node_modules copy.
 */
export function stagePhoenixFixture(name, { parent = tmpdir(), prefix = 'phoenix-e2e-' } = {}) {
  const fixture = readFixture(name);
  const fixtureRoot = join(FIXTURES_DIR, name);
  const tmp = mkdtempSync(join(parent, prefix));
  cpSync(join(fixtureRoot, 'files'), tmp, { recursive: true });
  writeFileSync(join(tmp, '.gitignore'), readFileSync(join(fixtureRoot, 'gitignore.txt'), 'utf-8'));
  mkdirSync(join(tmp, '.phoenix-ui', 'live'), { recursive: true });
  writeFileSync(join(tmp, '.phoenix-ui', 'live', 'config.json'), JSON.stringify(fixture.config));
  git(tmp, ['init', '-q']);
  git(tmp, ['config', 'user.email', 'test@example.com']);
  git(tmp, ['config', 'user.name', 'Fixture']);
  git(tmp, ['add', '-A']);
  git(tmp, ['commit', '-qm', 'fixture']);
  const template = nodeModulesTemplate(name);
  cpSync(join(template, 'node_modules'), join(tmp, 'node_modules'), { recursive: true, verbatimSymlinks: true });
  return { tmp, fixture };
}

export function git(cwd, args) {
  return execFileSync('git', args, { cwd, encoding: 'utf-8' });
}

/** Tracked-source diff and untracked files, for before/after evidence. */
export function sourceState(cwd) {
  return {
    diff: git(cwd, ['diff', '--no-color']),
    status: git(cwd, ['status', '--porcelain', '--untracked-files=all']),
  };
}

// ---------------------------------------------------------------------------
// Engine verbs
// ---------------------------------------------------------------------------

/** Run one verb; never throws, returns {code, stdout, stderr, json}. */
export function engine(verb, args, { cwd, env = {} } = {}) {
  const bin = requireEngineBin();
  const childEnv = { ...process.env, ...env };
  delete childEnv.PHOENIX_UI_SKILL_DIR;
  let code = 0;
  let stdout = '';
  let stderr = '';
  try {
    stdout = execFileSync(bin, [verb, ...args], {
      cwd, env: childEnv, encoding: 'utf-8', stdio: ['ignore', 'pipe', 'pipe'], timeout: 60_000,
    });
  } catch (err) {
    code = typeof err.status === 'number' ? err.status : -1;
    stdout = String(err.stdout || '');
    stderr = String(err.stderr || err.message || '');
  }
  let json = null;
  const lines = stdout.trim().split('\n').filter(Boolean);
  for (const candidate of [stdout.trim(), lines[lines.length - 1]]) {
    if (json || !candidate) continue;
    try { json = JSON.parse(candidate); } catch { /* not JSON */ }
  }
  return { verb, args, cwd, code, stdout, stderr, json };
}

export function startHelper(cwd) {
  const res = engine('live-server', ['--background'], { cwd });
  if (res.code !== 0 || !res.json?.port || !res.json?.pid) {
    throw new Error(`live-server --background failed: ${JSON.stringify(res)}`);
  }
  return { pid: res.json.pid, port: res.json.port, token: res.json.token, result: res };
}

export function injectScript(cwd, live) {
  const res = engine('live-inject', ['--port', String(live.port), '--token', String(live.token)], { cwd });
  if (res.code !== 0 || !res.json?.ok) throw new Error(`live-inject failed: ${JSON.stringify(res)}`);
  return res;
}

export function pidAlive(pid) {
  if (!pid) return false;
  try { process.kill(pid, 0); return true; } catch { return false; }
}

/** lsof view of one pid / port, for process evidence. */
export function processRecord(label, { pid, port } = {}) {
  const run = (cmd, args) => {
    try { return execFileSync(cmd, args, { encoding: 'utf-8' }); } catch (err) { return String(err.stdout || ''); }
  };
  return {
    label,
    at: new Date().toISOString(),
    pid: pid ?? null,
    pidAlive: pid ? pidAlive(pid) : null,
    ps: pid ? run('ps', ['-o', 'pid,ppid,stat,etime,command', '-p', String(pid)]).trim() : null,
    port: port ?? null,
    listeners: port ? run('lsof', ['-nP', `-iTCP:${port}`, '-sTCP:LISTEN']).trim() : null,
  };
}

export async function waitFor(fn, { timeout = 10_000, interval = 100, label = 'condition' } = {}) {
  const deadline = Date.now() + timeout;
  let last;
  while (Date.now() < deadline) {
    last = await fn();
    if (last) return last;
    await new Promise((r) => setTimeout(r, interval));
  }
  throw new Error(`${label} not met within ${timeout}ms (last=${JSON.stringify(last)})`);
}

// ---------------------------------------------------------------------------
// Recording
// ---------------------------------------------------------------------------

function redact(value, token) {
  if (value == null) return value;
  let text = String(value);
  if (token) text = text.split(encodeURIComponent(token)).join('<token>').split(token).join('<token>');
  return text;
}

function parseMaybeJson(text) {
  if (text == null) return null;
  try { return JSON.parse(text); } catch { return text; }
}

export function createRecorder(name) {
  const startedAt = Date.now();
  const rec = {
    name,
    livePorts: new Set(),
    tokens: new Set(),
    requests: [],
    sse: [],
    agentEvents: [],
    domEvents: [],
    console: [],
    notes: [],
    t: () => Date.now() - startedAt,
    clean(value) {
      let out = value;
      for (const token of rec.tokens) out = redact(out, token);
      return out;
    },
    note(label, data = {}) { rec.notes.push({ t: rec.t(), label, ...data }); },
    trackLive(live) {
      if (!live) return;
      rec.livePorts.add(String(live.port));
      if (live.token) rec.tokens.add(String(live.token));
    },
    helperRequests(filter = () => true) {
      return rec.requests.filter((r) => r.phase === 'request' && filter(r));
    },
    eventPosts(type) {
      return rec.helperRequests((r) => r.path === '/events' && r.method === 'POST' && r.body?.type === type);
    },
    save(relPath = `events/${name}.json`) {
      return writeEvidence(relPath, {
        name: rec.name,
        requests: rec.requests,
        sse: rec.sse,
        agentEvents: rec.agentEvents,
        domEvents: rec.domEvents,
        console: rec.console,
        notes: rec.notes,
      });
    },
  };
  return rec;
}

export async function attachRecorder(page, rec) {
  page.on('request', (req) => {
    let url;
    try { url = new URL(req.url()); } catch { return; }
    if (!rec.livePorts.has(url.port)) return;
    rec.requests.push({
      t: rec.t(),
      phase: 'request',
      method: req.method(),
      port: url.port,
      path: url.pathname,
      query: rec.clean(url.search),
      body: parseMaybeJson(rec.clean(req.postData())),
    });
  });
  page.on('response', (res) => {
    let url;
    try { url = new URL(res.url()); } catch { return; }
    if (!rec.livePorts.has(url.port)) return;
    rec.requests.push({ t: rec.t(), phase: 'response', method: res.request().method(), port: url.port, path: url.pathname, status: res.status() });
  });
  page.on('requestfailed', (req) => {
    let url;
    try { url = new URL(req.url()); } catch { return; }
    if (!rec.livePorts.has(url.port)) return;
    rec.requests.push({ t: rec.t(), phase: 'failed', method: req.method(), port: url.port, path: url.pathname, error: req.failure()?.errorText || null });
  });
  page.on('console', (msg) => {
    const text = msg.text();
    if (msg.type() === 'error' || /Phoenix UI|impeccable/i.test(text)) {
      rec.console.push({ t: rec.t(), type: msg.type(), text: rec.clean(text) });
    }
  });
  page.on('pageerror', (err) => rec.console.push({ t: rec.t(), type: 'pageerror', text: err.message }));
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Network.enable');
  cdp.on('Network.eventSourceMessageReceived', (e) => {
    rec.sse.push({ t: rec.t(), event: e.eventName, data: parseMaybeJson(rec.clean(e.data)) });
  });
  return cdp;
}

export async function openPhoenixPage(browser, url, rec, { viewport } = {}) {
  const ctx = await browser.newContext(viewport ? { viewport } : {});
  await ctx.addInitScript(phoenixGlobalAliases, GLOBAL_SUFFIXES);
  const page = await ctx.newPage();
  const cdp = rec ? await attachRecorder(page, rec) : null;
  await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30_000 });
  return { ctx, page, cdp };
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

export function startAgent({ cwd, live, rec, agent = createFakeAgent(), wrapTarget = { classes: 'hero-title', tag: 'h1' }, log = () => {} }) {
  const abort = new AbortController();
  const done = runAgentLoop({
    tmp: cwd,
    engineBin: requireEngineBin(),
    port: live.port,
    token: live.token,
    agent,
    wrapTarget,
    signal: abort.signal,
    log,
    trace: (event, data) => rec?.agentEvents.push({ t: rec.t(), event, ...data }),
  }).catch((err) => { log(`agent loop ended: ${err.message}`); });
  return {
    stop: async () => { abort.abort(); await done; },
  };
}

/** Wrap an agent so every generate event it handles is kept verbatim. */
export function recordingAgent(rec, base = createFakeAgent()) {
  return {
    ...base,
    async generateVariants(event, context) {
      rec.agentEvents.push({
        t: rec.t(),
        event: 'agent.generate.payload',
        id: event.id,
        mode: event.mode || 'replace',
        action: event.action,
        count: event.count,
        freeformPrompt: event.freeformPrompt ?? null,
        comments: event.comments ?? null,
      });
      return base.generateVariants(event, context);
    },
  };
}

// ---------------------------------------------------------------------------
// One full session: stage -> helper -> inject -> dev server -> agent -> page
// ---------------------------------------------------------------------------

export async function bootPhoenixSession({ name = 'vite8-react-plain', browser, rec, agent, viewport, parent, log = () => {} }) {
  const { tmp, fixture } = stagePhoenixFixture(name, { parent });
  const handles = { tmp, fixture, live: null, dev: null, agent: null, ctx: null, page: null };
  handles.teardown = async () => {
    try { if (handles.ctx) await handles.ctx.close(); } catch { /* closed */ }
    try { if (handles.agent) await handles.agent.stop(); } catch { /* stopped */ }
    try { if (handles.dev?.child) await stopDevServer(handles.dev.child); } catch { /* gone */ }
    if (handles.live?.pid && pidAlive(handles.live.pid)) {
      engine('live-server', ['stop'], { cwd: handles.tmp });
      if (pidAlive(handles.live.pid)) {
        try { process.kill(handles.live.pid, 'SIGKILL'); } catch { /* gone */ }
      }
    }
  };
  handles.remove = () => rmSync(handles.tmp, { recursive: true, force: true });
  try {
    handles.live = startHelper(tmp);
    rec?.trackLive(handles.live);
    handles.inject = injectScript(tmp, handles.live);
    handles.dev = startDevServer(tmp, fixture.runtime);
    const { port: devPort } = await handles.dev.ready;
    handles.devPort = devPort;
    handles.url = `http://127.0.0.1:${devPort}/`;
    if (agent !== null) {
      handles.agent = startAgent({ cwd: tmp, live: handles.live, rec, agent: agent || recordingAgent(rec), log });
    }
    if (browser) {
      const opened = await openPhoenixPage(browser, handles.url, rec, { viewport });
      handles.ctx = opened.ctx;
      handles.page = opened.page;
    }
    return handles;
  } catch (err) {
    if (handles.dev?.log) err.message += `\n--- dev server tail ---\n${handles.dev.log()}`;
    await handles.teardown();
    throw err;
  }
}
