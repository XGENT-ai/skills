/**
 * Protocol tests for agent-initiated element targeting (the `generate`
 * command), driven against the engine binary: POST /agent-target held-open
 * pairing with POST /agent-target-result, validation, the first-wins claim
 * and its declines, the no-browser and timeout verdicts, and the live-generate verb's
 * local failure modes. Skips cleanly without a binary (tests/lib/engine-bin.mjs).
 *
 * Run with: node --test tests/live-agent-target.test.mjs
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { execFile, execFileSync, spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { ENGINE_MISSING_MESSAGE, engineEnv, findEngineBinary } from './lib/engine-bin.mjs';
import { armLiveServerReaper, trackServerChild } from './lib/live-servers.mjs';

// Resolve the repo from this file, not from cwd: the runner may be invoked
// from tests/ or anywhere else.
const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const ENGINE_BIN = findEngineBinary();
// Every live server this file starts dies with it, whichever way it exits.
armLiveServerReaper();

// The action vocabulary lives in the engine (crates/live/src/vocabulary.rs);
// read it from the Rust source so the matrix below can never drift from what
// the live server accepts.
function readVisualActions() {
  const rust = readFileSync(join(REPO_ROOT, 'crates/live/src/vocabulary.rs'), 'utf-8');
  const block = rust.match(/pub const VISUAL_ACTIONS: \[&str; (\d+)\] = \[([\s\S]*?)\];/);
  if (!block) throw new Error('VISUAL_ACTIONS not found in crates/live/src/vocabulary.rs');
  return [...block[2].matchAll(/"([a-z]+)"/g)].map((m) => m[1]);
}
const VISUAL_ACTIONS = readVisualActions();

function liveServerPath(cwd) {
  return join(cwd, '.phoenix-ui/live/server.json');
}

/** Run the live-generate verb; the JSON verdict is on stdout on every exit code. */
function runGenerate(cwd, args) {
  return execFileSync(ENGINE_BIN, ['live-generate', ...args], {
    cwd,
    encoding: 'utf-8',
    env: engineEnv(ENGINE_BIN, {}),
  });
}

function startServer(port, { cwd, env = {} } = {}) {
  return new Promise((resolve, reject) => {
    const proc = trackServerChild(spawn(ENGINE_BIN, ['live-server', '--port=' + port], {
      cwd,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: engineEnv(ENGINE_BIN, { PHOENIX_UI_LIVE_COPY_AGENT: 'off', ...env }),
    }));
    let output = '';
    proc.stdout.on('data', (d) => { output += d.toString(); });
    proc.stderr.on('data', (d) => { output += d.toString(); });
    proc.on('error', reject);
    // The server writes server.json on listen; poll for it rather than
    // parsing the banner, so a slow first start still resolves.
    const deadline = Date.now() + 10_000;
    const tick = () => {
      try {
        const info = JSON.parse(readFileSync(liveServerPath(cwd), 'utf-8'));
        if (info.port && info.token) {
          resolve({ proc, port: info.port, token: info.token, cwd });
          return;
        }
      } catch { /* not yet */ }
      if (Date.now() > deadline) {
        reject(new Error('Server start timeout. Output: ' + output));
        return;
      }
      setTimeout(tick, 50);
    };
    tick();
  });
}

async function stopServer(server) {
  try {
    await fetch(`http://localhost:${server.port}/stop?token=${server.token}`);
  } catch { /* already gone */ }
}

function postJson(server, path, body) {
  return fetch(`http://localhost:${server.port}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}

/**
 * A minimal fake overlay: holds the SSE stream open and resolves pushed
 * messages so a test can await the next one matching a predicate.
 */
async function openSseClient(server) {
  const controller = new AbortController();
  const res = await fetch(
    `http://localhost:${server.port}/events?token=${server.token}`,
    { signal: controller.signal },
  );
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  const messages = [];
  const waiters = [];
  let buffer = '';
  (async () => {
    try {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        let idx;
        while ((idx = buffer.indexOf('\n\n')) !== -1) {
          const frame = buffer.slice(0, idx);
          buffer = buffer.slice(idx + 2);
          const dataLine = frame.split('\n').find((l) => l.startsWith('data: '));
          if (!dataLine) continue;
          let msg;
          try { msg = JSON.parse(dataLine.slice(6)); } catch { continue; }
          messages.push(msg);
          for (let i = waiters.length - 1; i >= 0; i -= 1) {
            if (waiters[i].match(msg)) {
              waiters[i].resolve(msg);
              waiters.splice(i, 1);
            }
          }
        }
      }
    } catch { /* stream closed */ }
  })();
  return {
    messages,
    next(match, timeoutMs = 5000) {
      const found = messages.find(match);
      if (found) return Promise.resolve(found);
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('SSE message timeout')), timeoutMs);
        waiters.push({ match, resolve: (m) => { clearTimeout(timer); resolve(m); } });
      });
    },
    close() { controller.abort(); },
  };
}

describe('POST /agent-target', { skip: ENGINE_BIN ? false : ENGINE_MISSING_MESSAGE }, () => {
  let tmp;
  let server;

  before(async () => {
    tmp = mkdtempSync(join(tmpdir(), 'impeccable-agent-target-'));
    mkdirSync(join(tmp, '.phoenix-ui/live'), { recursive: true });
    writeFileSync(join(tmp, 'index.html'), '<html><body><h1>t</h1></body></html>');
    // A short timeout keeps the browser_timeout case fast; the env override
    // exists exactly for this.
    server = await startServer(8497, {
      cwd: tmp,
      env: { PHOENIX_UI_AGENT_TARGET_TIMEOUT_MS: '400' },
    });
  });

  after(async () => {
    await stopServer(server);
    rmSync(tmp, { recursive: true, force: true });
  });

  it('rejects a wrong token with 401', async () => {
    const res = await postJson(server, '/agent-target', {
      token: 'nope', selector: 'h1', action: 'bolder', count: 3,
    });
    assert.equal(res.status, 401);
  });

  it('rejects an invalid action with 400 naming the vocabulary', async () => {
    const res = await postJson(server, '/agent-target', {
      token: server.token, selector: 'h1', action: 'bold', count: 3,
    });
    assert.equal(res.status, 400);
    const body = await res.json();
    assert.match(body.error, /invalid action/);
    assert.match(body.error, /bolder/);
  });

  it('rejects an out-of-range count with 400', async () => {
    const res = await postJson(server, '/agent-target', {
      token: server.token, selector: 'h1', action: 'bolder', count: 9,
    });
    assert.equal(res.status, 400);
    const body = await res.json();
    assert.match(body.error, /count must be 1-8/);
  });

  it('rejects a missing selector with 400', async () => {
    const res = await postJson(server, '/agent-target', {
      token: server.token, action: 'bolder', count: 3,
    });
    assert.equal(res.status, 400);
    const body = await res.json();
    assert.match(body.error, /selector is required/);
  });

  it('answers no_browser_connected when no SSE client is attached', async () => {
    const res = await postJson(server, '/agent-target', {
      token: server.token, selector: 'h1', action: 'bolder', count: 3,
    });
    assert.equal(res.status, 200);
    const body = await res.json();
    assert.equal(body.ok, false);
    assert.equal(body.error, 'no_browser_connected');
  });

  it('broadcasts agent_target and resolves the held request with the browser result', async () => {
    const sse = await openSseClient(server);
    try {
      await sse.next((m) => m.type === 'connected');
      const held = postJson(server, '/agent-target', {
        token: server.token,
        selector: 'section.pricing',
        text: 'Studio',
        index: 2,
        action: 'bolder',
        count: 3,
        prompt: 'warmer',
        dryRun: true,
      });
      const pushed = await sse.next((m) => m.type === 'agent_target');
      assert.equal(pushed.selector, 'section.pricing');
      assert.equal(pushed.text, 'Studio');
      assert.equal(pushed.index, 2);
      assert.equal(pushed.action, 'bolder');
      assert.equal(pushed.count, 3);
      assert.equal(pushed.prompt, 'warmer');
      assert.equal(pushed.dryRun, true);
      assert.match(pushed.targetId, /^[0-9a-f]{8}$/);

      const claimed = await (await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-a', eligible: true,
      })).json();
      assert.equal(claimed.granted, true);
      const resultRes = await postJson(server, '/agent-target-result', {
        token: server.token,
        targetId: pushed.targetId,
        clientId: 'tab-a',
        ok: true,
        matchCount: 1,
        sessionId: 'aabbccdd',
        element: { tag: 'section', id: null, classes: ['pricing'], text: 'Three ways' },
      });
      assert.deepEqual(await resultRes.json(), { ok: true, delivered: true });

      const verdict = await (await held).json();
      assert.equal(verdict.ok, true);
      assert.equal(verdict.targetId, pushed.targetId);
      assert.equal(verdict.matchCount, 1);
      assert.equal(verdict.sessionId, 'aabbccdd');
      assert.equal(verdict.element.tag, 'section');
      // The browser's own token must never leak back into the verdict.
      assert.equal('token' in verdict, false);
    } finally {
      sse.close();
      // Give the server's 8s SSE-drop exit timer no chance to fire between
      // tests: reconnecting tests open their own client immediately.
    }
  });

  it('grants only the first agent-target claim, so one tab owns the request', async () => {
    const sse = await openSseClient(server);
    try {
      await sse.next((m) => m.type === 'connected');
      const held = postJson(server, '/agent-target', {
        token: server.token, selector: 'h1', action: 'bolder', count: 3,
      });
      const pushed = await sse.next((m) => m.type === 'agent_target');
      const first = await (await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-a', eligible: true,
      })).json();
      const second = await (await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-b', eligible: true,
      })).json();
      assert.deepEqual(first, { ok: true, granted: true });
      assert.deepEqual(second, { ok: true, granted: false });
      // Settle the held request so the suite never waits out the timeout.
      await postJson(server, '/agent-target-result', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-a', ok: true, matchCount: 1, sessionId: 'aabbccdd',
      });
      await (await held).json();
    } finally {
      sse.close();
    }
  });

  it('denies a claim for an unknown or already-resolved targetId', async () => {
    const res = await postJson(server, '/agent-target-claim', {
      token: server.token, targetId: 'deadbeef', clientId: 'tab-x', eligible: true,
    });
    assert.deepEqual(await res.json(), { ok: true, granted: false });
  });

  it('answers busy as soon as every connected overlay has reported busy', async () => {
    // Two tabs, both mid-session. Neither claims; each declines, and the
    // second decline answers the held request busy well inside the 400ms
    // target timeout.
    const tabA = await openSseClient(server);
    const tabB = await openSseClient(server);
    try {
      await tabA.next((m) => m.type === 'connected');
      await tabB.next((m) => m.type === 'connected');
      const startedAt = Date.now();
      const held = postJson(server, '/agent-target', {
        token: server.token, selector: 'h1', action: 'bolder', count: 3,
      });
      const pushed = await tabA.next((m) => m.type === 'agent_target');
      for (const clientId of ['tab-a', 'tab-b']) {
        const report = await (await postJson(server, '/agent-target-claim', {
          token: server.token, targetId: pushed.targetId, clientId, eligible: false, state: 'CYCLING', reason: 'session_active',
        })).json();
        assert.deepEqual(report, { ok: true, granted: false });
      }
      const verdict = await (await held).json();
      assert.equal(verdict.error, 'busy');
      assert.equal(verdict.state, 'CYCLING');
      assert.equal(verdict.reason, 'session_active');
      assert.ok(Date.now() - startedAt < 350, 'the busy verdict did not wait for the timeout');
    } finally {
      tabA.close();
      tabB.close();
    }
  });

  it('lets an eligible tab serve the request while another tab reports busy', async () => {
    const tabA = await openSseClient(server);
    const tabB = await openSseClient(server);
    try {
      await tabA.next((m) => m.type === 'connected');
      await tabB.next((m) => m.type === 'connected');
      const held = postJson(server, '/agent-target', {
        token: server.token, selector: 'h1', action: 'bolder', count: 3,
      });
      const pushed = await tabA.next((m) => m.type === 'agent_target');
      await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-a', eligible: false, state: 'CYCLING', reason: 'session_active',
      });
      const claim = await (await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-b', eligible: true,
      })).json();
      assert.deepEqual(claim, { ok: true, granted: true }, 'one busy report does not answer while an idle tab is left');
      await postJson(server, '/agent-target-result', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-b', ok: true, matchCount: 1, sessionId: 'aabbccdd',
      });
      const verdict = await (await held).json();
      assert.equal(verdict.ok, true);
      assert.equal(verdict.sessionId, 'aabbccdd');
    } finally {
      tabA.close();
      tabB.close();
    }
  });

  it('never answers busy once a tab has won the claim', async () => {
    // Tab A reported busy, then freed up and claimed before tab B reported.
    // Every tab has now declined at some point, but A won the claim, so B's
    // busy report must not answer the request; A's result does.
    const tabA = await openSseClient(server);
    const tabB = await openSseClient(server);
    try {
      await tabA.next((m) => m.type === 'connected');
      await tabB.next((m) => m.type === 'connected');
      const held = postJson(server, '/agent-target', {
        token: server.token, selector: 'h1', action: 'bolder', count: 3,
      });
      const pushed = await tabA.next((m) => m.type === 'agent_target');
      await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-a', eligible: false, state: 'CYCLING', reason: 'session_active',
      });
      const reclaim = await (await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-a', eligible: true,
      })).json();
      assert.deepEqual(reclaim, { ok: true, granted: true }, 'the freed tab wins the claim');
      await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-b', eligible: false, state: 'CYCLING', reason: 'session_active',
      });
      // Still pending: A won, so the declines do not answer. A's result does.
      const resultRes = await postJson(server, '/agent-target-result', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-a', ok: true, matchCount: 1, sessionId: 'aabbccdd',
      });
      assert.deepEqual(await resultRes.json(), { ok: true, delivered: true });
      const verdict = await (await held).json();
      assert.equal(verdict.ok, true);
      assert.equal(verdict.sessionId, 'aabbccdd');
    } finally {
      tabA.close();
      tabB.close();
    }
  });

  it('answers the resolution verdict when every idle page declined as unable to resolve', async () => {
    // Two idle tabs on pages that lack the element decline with their
    // resolution verdicts; the request resolves as no_match, not busy.
    const tabA = await openSseClient(server);
    const tabB = await openSseClient(server);
    try {
      await tabA.next((m) => m.type === 'connected');
      await tabB.next((m) => m.type === 'connected');
      const startedAt = Date.now();
      const held = postJson(server, '/agent-target', {
        token: server.token, selector: 'h1', action: 'bolder', count: 3,
      });
      const pushed = await tabA.next((m) => m.type === 'agent_target');
      for (const [clientId, raw] of [['tab-a', 0], ['tab-b', 3]]) {
        const report = await (await postJson(server, '/agent-target-claim', {
          token: server.token, targetId: pushed.targetId, clientId, eligible: false, state: 'IDLE', reason: 'no_match',
          result: { ok: false, error: 'no_match', selector: 'h1', matchCount: 0, rawMatchCount: raw },
        })).json();
        assert.deepEqual(report, { ok: true, granted: false });
      }
      const verdict = await (await held).json();
      assert.equal(verdict.error, 'no_match');
      assert.equal(verdict.ok, false);
      assert.equal(verdict.targetId, pushed.targetId);
      assert.ok(Date.now() - startedAt < 350, 'the verdict did not wait for the timeout');
    } finally {
      tabA.close();
      tabB.close();
    }
  });

  it('forwards the hidden-bar request to the overlay, and refuses a non-boolean', async () => {
    const tabA = await openSseClient(server);
    try {
      await tabA.next((m) => m.type === 'connected');
      const held = postJson(server, '/agent-target', {
        token: server.token, selector: 'h1', action: 'bolder', count: 3, hideLiveBar: true,
      });
      assert.equal((await tabA.next((m) => m.type === 'live_bar')).hidden, true, 'every connected tab hears the helper-wide preference first');
      const pushed = await tabA.next((m) => m.type === 'agent_target');
      assert.equal(pushed.hideLiveBar, true, 'the overlay is told to keep the bottom bar out of the way');
      const tabB = await openSseClient(server);
      try {
        assert.equal((await tabB.next((m) => m.type === 'connected')).hideLiveBar, true, 'a later connection learns it on connect');
      } finally { tabB.close(); }
      const status = await (await fetch(`http://127.0.0.1:${server.port}/status?token=${server.token}`)).json();
      assert.equal(status.hideLiveBar, true, '/status carries it');
      await postJson(server, '/agent-target-result', { token: server.token, targetId: pushed.targetId, clientId: 'tab-a', ok: true, sessionId: 'aabbccdd' });
      await (await held).json();
      const refused = await postJson(server, '/agent-target', {
        token: server.token, selector: 'h1', action: 'bolder', count: 3, hideLiveBar: 'yes',
      });
      assert.equal(refused.status, 400);
      assert.equal((await refused.json()).error, 'agent_target: hideLiveBar must be a boolean');
    } finally {
      tabA.close();
    }
  });

  it('honors a result only from the claim winner', async () => {
    const tabA = await openSseClient(server);
    try {
      await tabA.next((m) => m.type === 'connected');
      const held = postJson(server, '/agent-target', { token: server.token, selector: 'h1', action: 'bolder', count: 3 });
      const pushed = await tabA.next((m) => m.type === 'agent_target');
      const claim = await (await postJson(server, '/agent-target-claim', { token: server.token, targetId: pushed.targetId, clientId: 'tab-a', eligible: true })).json();
      assert.equal(claim.granted, true);
      const bystander = await postJson(server, '/agent-target-result', { token: server.token, targetId: pushed.targetId, clientId: 'tab-b', ok: true, sessionId: 'b0b0b0b0' });
      assert.equal(bystander.status, 409, 'a tab that knows the id and the token still cannot answer for the holder');
      assert.equal((await bystander.json()).reason, 'not_holder');
      const anonymous = await postJson(server, '/agent-target-result', { token: server.token, targetId: pushed.targetId, ok: true });
      assert.equal(anonymous.status, 400, 'a result without a client id is malformed');
      const holder = await postJson(server, '/agent-target-result', { token: server.token, targetId: pushed.targetId, clientId: 'tab-a', ok: true, sessionId: 'aabbccdd' });
      assert.equal(holder.status, 200);
      assert.equal((await (await held).json()).sessionId, 'aabbccdd', 'the holder answers');
    } finally {
      tabA.close();
    }
  });

  it('prefers busy over no_match, so the agent retries when the right page is mid-session', async () => {
    const tabA = await openSseClient(server);
    const tabB = await openSseClient(server);
    try {
      await tabA.next((m) => m.type === 'connected');
      await tabB.next((m) => m.type === 'connected');
      const held = postJson(server, '/agent-target', {
        token: server.token, selector: 'h1', action: 'bolder', count: 3,
      });
      const pushed = await tabA.next((m) => m.type === 'agent_target');
      await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-b', eligible: false, state: 'IDLE', reason: 'no_match',
        result: { ok: false, error: 'no_match', matchCount: 0, rawMatchCount: 0 },
      });
      await postJson(server, '/agent-target-claim', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-a', eligible: false, state: 'CYCLING', reason: 'session_active',
      });
      const verdict = await (await held).json();
      assert.equal(verdict.error, 'busy');
      assert.equal(verdict.reason, 'session_active');
    } finally {
      tabA.close();
      tabB.close();
    }
  });

  it('carries every action in the live vocabulary from the CLI through the push', async () => {
    // The generate command promises the whole action picker, Freeform through
    // Overdrive. Drive each value through the CLI and the server, and read it
    // back off the broadcast the overlay would act on.
    const sse = await openSseClient(server);
    try {
      await sse.next((m) => m.type === 'connected');
      for (const action of VISUAL_ACTIONS) {
        const cli = new Promise((resolve) => {
          execFile(
            ENGINE_BIN,
            ['live-generate', '--selector', 'h1', '--action', action, '--dry-run'],
            { cwd: tmp, encoding: 'utf-8', env: engineEnv(ENGINE_BIN, {}) },
            (err, stdout) => resolve({ code: err ? err.code : 0, stdout }),
          );
        });
        const pushed = await sse.next(
          (m) => m.type === 'agent_target' && m.action === action && m.dryRun === true,
          10_000,
        );
        await postJson(server, '/agent-target-claim', {
          token: server.token, targetId: pushed.targetId, clientId: 'tab-vocab', eligible: true,
        });
        await postJson(server, '/agent-target-result', {
          token: server.token,
          targetId: pushed.targetId,
          clientId: 'tab-vocab',
          ok: true,
          dryRun: true,
          matchCount: 1,
          element: { tag: 'h1', id: null, classes: [], text: 't' },
        });
        const { code, stdout } = await cli;
        const verdict = JSON.parse(stdout);
        assert.equal(code, 0, `${action}: the CLI exits 0`);
        assert.equal(verdict.ok, true, `${action}: the verdict is ok`);
      }
    } finally {
      sse.close();
    }
  });

  it('times out into browser_timeout when the overlay never answers, and a late result reports delivered:false', async () => {
    const sse = await openSseClient(server);
    try {
      await sse.next((m) => m.type === 'connected');
      const held = postJson(server, '/agent-target', {
        token: server.token, selector: 'h1', action: 'quieter', count: 2,
      });
      const pushed = await sse.next((m) => m.type === 'agent_target');
      const verdict = await (await held).json();
      assert.equal(verdict.ok, false);
      assert.equal(verdict.error, 'browser_timeout');
      assert.equal(verdict.timeoutMs, 400);

      const late = await postJson(server, '/agent-target-result', {
        token: server.token, targetId: pushed.targetId, clientId: 'tab-a', ok: true,
      });
      assert.deepEqual(await late.json(), { ok: true, delivered: false });
    } finally {
      sse.close();
    }
  });

  it('rejects an agent-target result without a targetId', async () => {
    const res = await postJson(server, '/agent-target-result', {
      token: server.token, ok: true,
    });
    assert.equal(res.status, 400);
    const body = await res.json();
    assert.match(body.error, /missing targetId/);
  });
});

describe('live-generate CLI --wait-for-browser', { skip: ENGINE_BIN ? false : ENGINE_MISSING_MESSAGE }, () => {
  let tmp;
  let server;

  before(async () => {
    tmp = mkdtempSync(join(tmpdir(), 'impeccable-generate-wait-'));
    mkdirSync(join(tmp, '.phoenix-ui/live'), { recursive: true });
    writeFileSync(join(tmp, 'index.html'), '<html><body><h1>t</h1></body></html>');
    server = await startServer(8496, {
      cwd: tmp,
      env: { PHOENIX_UI_AGENT_TARGET_TIMEOUT_MS: '400' },
    });
  });

  after(async () => {
    await stopServer(server);
    rmSync(tmp, { recursive: true, force: true });
  });

  function runCli(cwd, args) {
    try {
      const stdout = runGenerate(cwd, args);
      return { code: 0, json: JSON.parse(stdout) };
    } catch (err) {
      return { code: err.status, json: JSON.parse(err.stdout) };
    }
  }

  it('rejects a malformed wait budget locally', () => {
    const { code, json } = runCli(tmp, ['--selector', 'h1', '--wait-for-browser', 'soon']);
    assert.equal(code, 1);
    assert.equal(json.error, 'invalid_wait');
  });

  it('gives up with no_browser_connected after the wait budget with no page attached', () => {
    const startedAt = Date.now();
    const { code, json } = runCli(tmp, ['--selector', 'h1', '--action', 'bolder', '--wait-for-browser', '1500']);
    assert.equal(code, 1);
    assert.equal(json.error, 'no_browser_connected');
    assert.equal(json.waitedMs, 1500);
    assert.ok(Date.now() - startedAt >= 1400, 'the CLI actually waited out the budget');
  });

  it('proceeds to target as soon as a page connects during the wait', async () => {
    // Connect the fake overlay 1.2s into the CLI's wait window. The CLI must
    // then send the target; the silent overlay lets it resolve as
    // browser_timeout, which proves the wait detected the connection and the
    // request went out (no_browser_connected would mean it never did). The
    // child runs async: execFileSync would block this process's event loop
    // and the delayed connect would never happen.
    const child = new Promise((resolve) => {
      execFile(
        ENGINE_BIN,
        ['live-generate', '--selector', 'h1', '--action', 'bolder', '--wait-for-browser', '10000'],
        { cwd: tmp, encoding: 'utf-8', env: engineEnv(ENGINE_BIN, {}) },
        (err, stdout) => resolve({ code: err ? err.code : 0, stdout }),
      );
    });
    await new Promise((r) => setTimeout(r, 1200));
    const sse = await openSseClient(server);
    const res = await child;
    sse.close();
    assert.equal(res.code, 1);
    const json = JSON.parse(res.stdout);
    assert.equal(json.error, 'browser_timeout');
  });
});

describe('live-generate CLI local failure modes', { skip: ENGINE_BIN ? false : ENGINE_MISSING_MESSAGE }, () => {
  function runCli(cwd, args) {
    try {
      const stdout = runGenerate(cwd, args);
      return { code: 0, json: JSON.parse(stdout) };
    } catch (err) {
      return { code: err.status, json: JSON.parse(err.stdout) };
    }
  }

  it('accepts --no-live-bar as a boolean flag', () => {
    const tmp = mkdtempSync(join(tmpdir(), 'impeccable-generate-cli-'));
    try {
      const { code, json } = runCli(tmp, ['--selector', 'h1', '--action', 'bolder', '--no-live-bar']);
      assert.equal(code, 1);
      assert.equal(json.error, 'server_not_running', 'the flag parses; the verdict is about the missing helper, not the flag');
    } finally {
      rmSync(tmp, { recursive: true, force: true });
    }
  });

  it('fails with server_not_running when no live server is recorded', () => {
    const tmp = mkdtempSync(join(tmpdir(), 'impeccable-generate-cli-'));
    try {
      const { code, json } = runCli(tmp, ['--selector', 'h1', '--action', 'bolder']);
      assert.equal(code, 1);
      assert.equal(json.error, 'server_not_running');
      assert.match(json._instructions, / live\)/, 'names the boot verb');
    } finally {
      rmSync(tmp, { recursive: true, force: true });
    }
  });

  it('fails with invalid_action locally, listing the vocabulary and the mapping hint', () => {
    const tmp = mkdtempSync(join(tmpdir(), 'impeccable-generate-cli-'));
    try {
      const { code, json } = runCli(tmp, ['--selector', 'h1', '--action', 'bold']);
      assert.equal(code, 1);
      assert.equal(json.error, 'invalid_action');
      assert.ok(json.validActions.includes('bolder'));
      assert.match(json._instructions, /bold -> bolder/);
    } finally {
      rmSync(tmp, { recursive: true, force: true });
    }
  });

  it('fails with selector_required when --selector is missing', () => {
    const tmp = mkdtempSync(join(tmpdir(), 'impeccable-generate-cli-'));
    try {
      const { code, json } = runCli(tmp, ['--action', 'bolder']);
      assert.equal(code, 1);
      assert.equal(json.error, 'selector_required');
    } finally {
      rmSync(tmp, { recursive: true, force: true });
    }
  });

  it('fails with invalid_count on a non-integer count', () => {
    const tmp = mkdtempSync(join(tmpdir(), 'impeccable-generate-cli-'));
    try {
      const { code, json } = runCli(tmp, ['--selector', 'h1', '--count', 'many']);
      assert.equal(code, 1);
      assert.equal(json.error, 'invalid_count');
    } finally {
      rmSync(tmp, { recursive: true, force: true });
    }
  });
});
