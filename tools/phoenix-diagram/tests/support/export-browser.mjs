// Maintenance-test adapter derived from the frozen Archify 3.1.0
// archify/bin/visual-check.mjs PipeCdp/ChromeVisualBrowser and
// renderers/shared/owned-temp-directory.mjs. MIT: ../../viewer/LICENSE.txt.
// Copyright (c) 2026 tt-a1i (Archify); Copyright (c) 2025 Cocoon AI.
// Phoenix changes: test-only API, own namespace, strict explicit discovery,
// default sandbox, 90s startup/15s commands, event subscription, no Viewer code.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

export const DEFAULT_TIMEOUTS = Object.freeze({ startupMs: 90000, commandMs: 15000, shutdownMs: 1500, actionMs: 15000, decodeMs: 12000 });

function identity(directory) {
  const stat = fs.lstatSync(directory, { bigint: true });
  if (!stat.isDirectory() || stat.isSymbolicLink() || !stat.ino) throw new Error(`Owned profile identity is unavailable: ${directory}`);
  return { directory, dev: stat.dev, ino: stat.ino, birthtimeNs: stat.birthtimeNs };
}
function verify(expected) {
  const actual = identity(expected.directory);
  if (actual.dev !== expected.dev || actual.ino !== expected.ino || actual.birthtimeNs !== expected.birthtimeNs) throw new Error(`Owned profile identity changed: ${expected.directory}`);
}
export function ownedProfile(parent = os.tmpdir()) {
  const canonical = fs.realpathSync.native(path.resolve(parent)), ancestors = [];
  for (let current = canonical;; current = path.dirname(current)) { ancestors.push(identity(current)); if (path.dirname(current) === current) break; }
  const verifyParents = () => { for (const ancestor of [...ancestors].reverse()) verify(ancestor); };
  verifyParents();
  const directory = fs.mkdtempSync(path.join(canonical, 'phoenix-diagram-export-profile-'));
  verifyParents(); const binding = identity(directory);
  return { path: directory, async cleanup() {
    for (let attempt = 0;; attempt++) {
      try {
        verifyParents();
        try { verify(binding); } catch (error) { if (error.code === 'ENOENT') return; throw error; }
        fs.rmSync(directory, { recursive: true, force: false, maxRetries: 0 }); return;
      } catch (cause) {
        if (['EBUSY', 'EPERM', 'EACCES', 'ENOTEMPTY'].includes(cause.code) && attempt < 3) { await delay(100 * (attempt + 1)); continue; }
        const error = new Error(`Owned export profile retained at ${directory}: ${cause.message}`, { cause });
        error.code = 'browser/profile-cleanup'; error.directory = directory; throw error;
      }
    }
  } };
}

function executable(file) { try { fs.accessSync(file, fs.constants.X_OK); return fs.statSync(file).isFile() ? fs.realpathSync(file) : null; } catch { return null; } }
export function discoverBrowser({ browserPath, env = process.env, platform = process.platform } = {}) {
  const explicit = browserPath ?? env.PHOENIX_DIAGRAM_BROWSER;
  if (explicit !== undefined) {
    const resolved = executable(explicit);
    if (!resolved) { const error = new Error(`Configured PHOENIX_DIAGRAM_BROWSER is not executable: ${explicit}`); error.code = 'browser/configured-path'; throw error; }
    return resolved;
  }
  const candidates = [];
  if (platform === 'darwin') {
    for (const parent of ['/Applications', path.join(os.homedir(), 'Applications')]) for (const bundle of ['Google Chrome.app/Contents/MacOS/Google Chrome', 'Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary', 'Chromium.app/Contents/MacOS/Chromium']) candidates.push(path.join(parent, bundle));
  } else {
    for (const name of ['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser']) for (const directory of String(env.PATH || '').split(path.delimiter).filter(Boolean)) candidates.push(path.join(directory, name));
    candidates.push('/opt/google/chrome/chrome');
  }
  const resolved = candidates.map(executable).find(Boolean);
  if (!resolved) { const error = new Error('No installed Chrome/Chromium was found; set PHOENIX_DIAGRAM_BROWSER. No browser is downloaded by this test.'); error.code = 'browser/unavailable'; throw error; }
  return resolved;
}

export function browserArguments(profile, { noSandbox = false } = {}) {
  return [...(noSandbox ? ['--no-sandbox'] : []), '--headless=new', '--remote-debugging-pipe', '--disable-gpu', '--hide-scrollbars', '--disable-background-networking', '--disable-component-update', '--disable-default-apps', '--disable-sync', '--metrics-recording-only', '--no-first-run', '--use-mock-keychain', '--no-default-browser-check', '--disable-background-timer-throttling', '--disable-backgrounding-occluded-windows', '--disable-renderer-backgrounding', '--force-device-scale-factor=1', `--user-data-dir=${profile}`, 'about:blank'];
}

export class PipeCdp {
  constructor(child, { commandMs = DEFAULT_TIMEOUTS.commandMs, failureDetails = () => '' } = {}) {
    this.child = child; this.commandMs = commandMs; this.failureDetails = failureDetails; this.nextId = 1; this.buffer = ''; this.pending = new Map(); this.waiters = []; this.subscribers = new Set();
    this.writePipe = child.stdio[3]; this.readPipe = child.stdio[4]; this.readPipe.setEncoding('utf8');
    this.readPipe.on('data', chunk => this.consume(chunk));
    for (const [stream, name] of [[this.writePipe, 'write pipe'], [this.readPipe, 'read pipe']]) {
      stream.on('error', error => this.failAll(this.failure(name, error)));
      stream.once('close', () => this.failAll(this.failure(name, new Error('stream closed'))));
    }
    this.readPipe.once('end', () => this.failAll(this.failure('read pipe', new Error('stream ended'))));
    child.once('error', error => this.failAll(this.failure('launch', error)));
    child.once('close', (code, signal) => this.failAll(this.failure('process exit', new Error(signal || `exit ${code}`))));
  }
  failure(stage, cause) { const error = new Error(`Export Chrome ${stage}: ${cause.message}\n${this.failureDetails()}`, { cause }); error.code = cause.code || 'browser/cdp'; return error; }
  subscribe(callback) { this.subscribers.add(callback); return () => this.subscribers.delete(callback); }
  consume(chunk) {
    this.buffer += chunk;
    for (let at; (at = this.buffer.indexOf('\0')) >= 0;) {
      const raw = this.buffer.slice(0, at); this.buffer = this.buffer.slice(at + 1); if (!raw) continue;
      let message; try { message = JSON.parse(raw); } catch (error) { this.failAll(this.failure('JSON', error)); continue; }
      if (message.id) {
        const pending = this.pending.get(message.id); if (!pending) continue;
        clearTimeout(pending.timer); this.pending.delete(message.id);
        if (message.error) pending.reject(this.failure(pending.method, new Error(message.error.message))); else pending.resolve(message.result || {});
      } else {
        for (const callback of this.subscribers) callback(message);
        for (const waiter of [...this.waiters]) {
          if (waiter.method !== message.method || waiter.sessionId && waiter.sessionId !== message.sessionId) continue;
          clearTimeout(waiter.timer); this.waiters.splice(this.waiters.indexOf(waiter), 1); waiter.resolve(message.params || {});
        }
      }
    }
  }
  send(method, params = {}, sessionId, timeoutMs = this.commandMs) {
    const id = this.nextId++, message = { id, method, params }; if (sessionId) message.sessionId = sessionId;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { this.pending.delete(id); const error = new Error(`${method} timed out after ${timeoutMs}ms`); error.code = 'browser/command-timeout'; reject(this.failure('deadline', error)); }, timeoutMs);
      this.pending.set(id, { method, resolve, reject, timer });
      try { this.writePipe.write(JSON.stringify(message) + '\0', error => { if (error) this.failAll(this.failure('write pipe', error)); }); }
      catch (error) { this.failAll(this.failure('write pipe', error)); }
    });
  }
  waitFor(method, sessionId, timeoutMs = this.commandMs) {
    return new Promise((resolve, reject) => {
      const waiter = { method, sessionId, resolve, reject, timer: null };
      waiter.timer = setTimeout(() => { this.waiters.splice(this.waiters.indexOf(waiter), 1); reject(this.failure('event deadline', new Error(`${method} timed out after ${timeoutMs}ms`))); }, timeoutMs);
      this.waiters.push(waiter);
    });
  }
  failAll(error) { for (const pending of this.pending.values()) { clearTimeout(pending.timer); pending.reject(error); } for (const waiter of this.waiters) { clearTimeout(waiter.timer); waiter.reject(error); } this.pending.clear(); this.waiters = []; }
}

export class ExportBrowser {
  constructor(browserPath, { timeouts = {}, noSandbox = false, spawnImpl = spawn } = {}) {
    this.timeouts = { ...DEFAULT_TIMEOUTS, ...timeouts }; this.browserPath = browserPath; this.profile = ownedProfile(); this.profileRoot = this.profile.path; this.stderr = '';
    try {
      this.child = spawnImpl(browserPath, browserArguments(this.profileRoot, { noSandbox }), { stdio: ['ignore', 'ignore', 'pipe', 'pipe', 'pipe'] });
      this.child.stderr.setEncoding('utf8'); this.child.stderr.on('data', chunk => { this.stderr = (this.stderr + chunk).slice(-8000); });
      this.cdp = new PipeCdp(this.child, { commandMs: this.timeouts.commandMs, failureDetails: () => this.stderr });
      this.ready = this.cdp.send('Browser.getVersion', {}, undefined, this.timeouts.startupMs);
      this.ready.catch(() => {});
    } catch (error) { this.ready = Promise.reject(error); this.ready.catch(() => {}); }
  }
  close() { if (!this.closing) this.closing = this.finishClose(); return this.closing; }
  async finishClose() {
    this.cdp?.failAll(new Error('Export acceptance finished'));
    if (this.child) {
      const exited = () => this.child.exitCode !== null || this.child.signalCode !== null;
      const signalAndWait = signal => new Promise((resolve, reject) => {
        let timer;
        const finish = () => { clearTimeout(timer); this.child.removeListener('exit', finish); resolve(true); };
        this.child.once('exit', finish); timer = setTimeout(() => { this.child.removeListener('exit', finish); resolve(exited()); }, this.timeouts.shutdownMs);
        try { this.child.kill(signal); } catch (error) { clearTimeout(timer); this.child.removeListener('exit', finish); reject(error); }
      });
      try {
        if (!exited() && !await signalAndWait('SIGTERM') && !await signalAndWait('SIGKILL')) {
          const error = new Error(`Chrome did not exit; owned profile retained at ${this.profileRoot}`); error.code = 'browser/shutdown'; throw error;
        }
      } finally { for (const stream of [this.child.stderr, this.child.stdio[3], this.child.stdio[4]]) if (stream && !stream.destroyed) stream.destroy(); }
    }
    await this.profile.cleanup();
  }
}
