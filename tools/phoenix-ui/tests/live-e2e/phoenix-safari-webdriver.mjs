/**
 * Minimal W3C WebDriver client for real Safari through `safaridriver -p`.
 *
 * Playwright's WebKit is not Safari, so the Safari walkthrough drives the
 * system browser over plain HTTP: new session, navigate, execute script
 * (sync/async), element lookup, Actions (pointer/keyboard), window rect,
 * screenshot, delete session. Node fetch only; no extra dependencies.
 *
 * Page functions passed to exec/execAsync are serialized with
 * Function#toString, so they must be self-contained (no closures), the same
 * rule as Playwright's page.evaluate. A session can carry a `prelude` of
 * function declarations that every script can call.
 */

import { spawn } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { createServer } from 'node:net';

export const SAFARIDRIVER = process.env.PHOENIX_SAFARIDRIVER || '/usr/bin/safaridriver';
export const ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';
export const KEYS = {
  TAB: '', RETURN: '', ENTER: '', SHIFT: '', ALT: '', ESCAPE: '',
};
/** safaridriver's answer while Safari > Develop > Allow Remote Automation is off. */
export const AUTOMATION_OFF = /Allow remote automation/i;

export class WebDriverError extends Error {
  constructor(status, error, message, command) {
    super(`${command}: ${error}: ${message}`);
    this.status = status;
    this.error = error;
    this.wdMessage = message;
    this.command = command;
  }
}

export async function freePort() {
  const srv = createServer();
  await new Promise((r) => srv.listen(0, '127.0.0.1', r));
  const { port } = srv.address();
  await new Promise((r) => srv.close(r));
  return port;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function http(base, method, path, body, { timeout = 120_000 } = {}) {
  const command = `${method} ${path.replace(/\/session\/[^/]+/, '/session/:id')}`;
  const res = await fetch(base + path, {
    method,
    headers: body === undefined ? {} : { 'content-type': 'application/json; charset=utf-8' },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(timeout),
  });
  const text = await res.text();
  let json = null;
  try { json = JSON.parse(text); } catch { /* not JSON */ }
  const value = json && 'value' in json ? json.value : json;
  if (!res.ok || (value && typeof value === 'object' && typeof value.error === 'string')) {
    throw new WebDriverError(res.status, value?.error || `http ${res.status}`, value?.message || text.slice(0, 500), command);
  }
  return value;
}

/**
 * Start `safaridriver -p <port>` as a direct child and wait for /status.
 * stop() ends only this child.
 */
export async function startSafariDriver({ port } = {}) {
  const usePort = port || await freePort();
  const child = spawn(SAFARIDRIVER, ['-p', String(usePort)], { stdio: ['ignore', 'pipe', 'pipe'] });
  const lines = [];
  const capture = (stream) => (chunk) => lines.push(`[${new Date().toISOString()}] ${stream}: ${chunk.toString().trimEnd()}`);
  child.stdout.on('data', capture('stdout'));
  child.stderr.on('data', capture('stderr'));
  let exited = null;
  child.on('exit', (code, signal) => { exited = { code, signal }; lines.push(`[${new Date().toISOString()}] exit code=${code} signal=${signal}`); });
  const base = `http://127.0.0.1:${usePort}`;
  const deadline = Date.now() + 15_000;
  let status = null;
  while (Date.now() < deadline) {
    if (exited) throw new Error(`safaridriver exited before ready: ${JSON.stringify(exited)}\n${lines.join('\n')}`);
    try { status = await http(base, 'GET', '/status', undefined, { timeout: 2_000 }); break; } catch { await sleep(200); }
  }
  if (!status) {
    child.kill('SIGTERM');
    throw new Error(`safaridriver did not answer /status on ${usePort}\n${lines.join('\n')}`);
  }
  return {
    port: usePort,
    base,
    pid: child.pid,
    status,
    log: () => lines.join('\n'),
    get exited() { return exited; },
    async stop() {
      if (exited) return exited;
      child.kill('SIGTERM');
      const until = Date.now() + 5_000;
      while (!exited && Date.now() < until) await sleep(100);
      if (!exited) child.kill('SIGKILL');
      return exited;
    },
  };
}

/** POST /session for Safari. Throws WebDriverError (AUTOMATION_OFF when automation is disabled). */
export async function newSafariSession(driver, { prelude = '', capabilities = {} } = {}) {
  const value = await http(driver.base, 'POST', '/session', {
    capabilities: { alwaysMatch: { browserName: 'safari', ...capabilities } },
  });
  const session = new Session(driver, value.sessionId, value.capabilities, prelude);
  await session.cmd('POST', '/timeouts', { script: 120_000, pageLoad: 60_000, implicit: 0 });
  return session;
}

/** Try one session; returns {available, message}. Deletes the session when it was created. */
export async function probeAutomation(driver) {
  try {
    const s = await newSafariSession(driver);
    const caps = s.capabilities;
    await s.delete();
    return { available: true, capabilities: caps };
  } catch (err) {
    return { available: false, automationOff: AUTOMATION_OFF.test(err.message), message: err.message };
  }
}

export class Session {
  constructor(driver, id, capabilities, prelude) {
    this.driver = driver;
    this.id = id;
    this.capabilities = capabilities;
    this.prelude = prelude;
    this.deleted = false;
  }

  cmd(method, path, body, opts) {
    return http(this.driver.base, method, `/session/${this.id}${path}`, body, opts);
  }

  navigate(url) { return this.cmd('POST', '/url', { url }); }
  refresh() { return this.cmd('POST', '/refresh', {}); }
  currentUrl() { return this.cmd('GET', '/url'); }

  script(fn) {
    const body = typeof fn === 'function' ? `return (${fn.toString()}).apply(null, arguments);` : fn;
    return `${this.prelude}\n${body}`;
  }

  /** Run fn(...args) in the page, return its JSON-able result. */
  exec(fn, ...args) {
    return this.cmd('POST', '/execute/sync', { script: this.script(fn), args });
  }

  /** Run an async fn(...args) in the page and await its promise. */
  async execAsync(fn, ...args) {
    const wrapper = `${this.prelude}
const done = arguments[arguments.length - 1];
const args = Array.prototype.slice.call(arguments, 0, -1);
Promise.resolve().then(() => (${fn.toString()}).apply(null, args))
  .then((v) => done({ ok: true, v: v === undefined ? null : v }), (e) => done({ ok: false, e: String((e && e.stack) || e) }));`;
    const out = await this.cmd('POST', '/execute/async', { script: wrapper, args });
    if (!out?.ok) throw new Error(`page async script failed: ${out?.e}`);
    return out.v;
  }

  /** Document-level CSS lookup; returns an element id or null. */
  async find(css) {
    try {
      const v = await this.cmd('POST', '/element', { using: 'css selector', value: css });
      return v[ELEMENT_KEY];
    } catch (err) {
      if (err.error === 'no such element') return null;
      throw err;
    }
  }

  clickElement(elementId) { return this.cmd('POST', `/element/${elementId}/click`, {}); }

  actions(actions) { return this.cmd('POST', '/actions', { actions }); }
  releaseActions() { return this.cmd('DELETE', '/actions'); }

  /** Real pointer move to viewport coordinates. */
  pointerMove(x, y, { pointerType = 'mouse' } = {}) {
    return this.actions([{
      type: 'pointer', id: pointerType === 'mouse' ? 'mouse' : pointerType, parameters: { pointerType },
      actions: [{ type: 'pointerMove', duration: 0, origin: 'viewport', x: Math.round(x), y: Math.round(y) }],
    }]);
  }

  /** Real pointer move + press + release at viewport coordinates. */
  pointerClick(x, y, { pointerType = 'mouse', hoverMs = 50 } = {}) {
    return this.actions([{
      type: 'pointer', id: pointerType === 'mouse' ? 'mouse' : pointerType, parameters: { pointerType },
      actions: [
        { type: 'pointerMove', duration: 0, origin: 'viewport', x: Math.round(x), y: Math.round(y) },
        { type: 'pause', duration: hoverMs },
        { type: 'pointerDown', button: 0 },
        { type: 'pause', duration: 30 },
        { type: 'pointerUp', button: 0 },
      ],
    }]);
  }

  /** Press one key, optionally with held modifiers (KEYS values). */
  press(key, { modifiers = [] } = {}) {
    const actions = [
      ...modifiers.map((m) => ({ type: 'keyDown', value: m })),
      { type: 'keyDown', value: key },
      { type: 'keyUp', value: key },
      ...modifiers.slice().reverse().map((m) => ({ type: 'keyUp', value: m })),
    ];
    return this.actions([{ type: 'key', id: 'keyboard', actions }]);
  }

  getWindowRect() { return this.cmd('GET', '/window/rect'); }
  setWindowRect(rect) { return this.cmd('POST', '/window/rect', rect); }

  /** Size the window so the page viewport is width x height (as far as Safari allows). */
  async setViewport(width, height) {
    const before = await this.getWindowRect();
    const inner = await this.exec(() => ({ w: innerWidth, h: innerHeight }));
    const dw = before.width - inner.w;
    const dh = before.height - inner.h;
    await this.setWindowRect({ x: 0, y: 0, width: width + dw, height: height + dh });
    const after = await this.exec(() => ({ w: innerWidth, h: innerHeight, dpr: devicePixelRatio, sw: screen.availWidth, sh: screen.availHeight }));
    return { requested: { width, height }, window: await this.getWindowRect(), inner: after };
  }

  async screenshot(file) {
    const b64 = await this.cmd('GET', '/screenshot');
    writeFileSync(file, Buffer.from(b64, 'base64'));
    return file;
  }

  async delete() {
    if (this.deleted) return;
    this.deleted = true;
    await this.cmd('DELETE', '', undefined, { timeout: 30_000 });
  }
}
