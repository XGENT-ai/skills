'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const script = path.resolve(__dirname, '..', '..', 'hooks', 'claude', 'context-goal-guard.js');
const OPUS = 'claude-opus-5-5';
const WINDOW_ENV = ['CLAUDE_CODE_DISABLE_1M_CONTEXT', 'DISABLE_COMPACT', 'CLAUDE_CODE_MAX_CONTEXT_TOKENS'];

function goal(fields = { met: false, sentinel: true }) {
  return { type: 'attachment', isSidechain: false, attachment: { type: 'goal_status', condition: 'PRIVATE_GOAL', ...fields } };
}

function assistant(used, { model = OPUS, iterations, ...extra } = {}) {
  const usage = { input_tokens: 2, cache_creation_input_tokens: 1000, cache_read_input_tokens: used - 1002, output_tokens: 900000 };
  if (iterations) usage.iterations = iterations;
  return { type: 'assistant', isSidechain: false, message: { model, role: 'assistant', usage }, ...extra };
}

const compact = { type: 'system', subtype: 'compact_boundary', isSidechain: false };

function session(t, records = [goal(), assistant(700001)], tail = '') {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'Claude Guard # '));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const transcript = path.join(dir, 'session.jsonl');
  const write = (rows, end = '') => fs.writeFileSync(transcript, rows.map((r) => JSON.stringify(r) + '\n').join('') + end);
  write(records, tail);
  return { dir, transcript, write };
}

function guard(s, input = {}, { env = {}, raw, args = [] } = {}) {
  const environment = { ...process.env, ...env };
  for (const name of WINDOW_ENV) if (!(name in env)) delete environment[name];
  const data = { session_id: 's', transcript_path: s.transcript, cwd: s.dir, hook_event_name: 'PostToolBatch', ...input };
  const result = spawnSync(process.execPath, [script, ...args], {
    input: raw ?? JSON.stringify(data), encoding: 'utf8', env: environment, cwd: s.dir, timeout: 5000,
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stderr, '');
  return result.stdout === '' ? null : JSON.parse(result.stdout);
}

const stop = (active, extra = {}) => ({ hook_event_name: 'Stop', stop_hook_active: active, ...extra });

function assertReminder(text, threshold = 70) {
  for (const part of [`${threshold}%`, '继续', '后台', '已验证', '不新建文件', '工作未完成', '/clear', '/goal']) {
    assert.ok(text.includes(part), part);
  }
  assert.ok(!text.includes('PRIVATE_GOAL'));
}

function assertUnavailable(out) {
  assert.match(out.systemMessage, /^context goal guard 检测不可用：/);
  assert.ok(out.systemMessage.includes('docs/claude-context-goal-guard.md'));
  assert.ok(out.systemMessage.includes('.claude/settings.local.json'));
  assert.equal(out.decision, undefined);
  assert.equal(out.continue, undefined);
}

test('default threshold is strictly above 70% of the verified window', (t) => {
  const s = session(t);
  for (const [model, used, fires] of [
    [OPUS, 699900, false], [OPUS, 700000, false], [OPUS, 700001, true], [OPUS, 1000, false],
    ['claude-sonnet-5-5', 700001, true],
    ['claude-haiku-4-5-20251001', 140000, false], ['claude-haiku-4-5-20251001', 140001, true],
  ]) {
    s.write([goal(), assistant(used, { model })]);
    const out = guard(s);
    assert.equal(out !== null, fires, `${model} ${used}`);
    if (fires) {
      assert.equal(out.hookSpecificOutput.hookEventName, 'PostToolBatch');
      assert.ok(out.hookSpecificOutput.additionalContext.startsWith('上下文估算使用率 70% 已超过 70%'));
      assertReminder(out.hookSpecificOutput.additionalContext);
    }
  }
});

test('--threshold overrides the default and appears in every message', (t) => {
  const s = session(t);
  const args = ['--threshold', '65'];
  for (const [used, fires] of [[649900, false], [650000, false], [650001, true]]) {
    s.write([goal(), assistant(used)]);
    assert.equal(guard(s, {}, { args }) !== null, fires, String(used));
  }
  assertReminder(guard(s, {}, { args }).hookSpecificOutput.additionalContext, 65);
  assertReminder(guard(s, stop(false), { args }).reason, 65);
  assert.ok(guard(s, stop(true), { args }).stopReason.includes('已超过 65%'));
  s.write([goal(), assistant(990000)]);
  assert.equal(guard(s, {}, { args: ['--threshold', '99'] }), null);
  s.write([goal(), assistant(995000)]);
  assert.ok(guard(s, {}, { args: ['--threshold', '99'] }));
  assert.ok(guard(s, {}, { args: ['--threshold', '1'] }));
});

test('invalid --threshold warns on Stop only and never guesses a value', (t) => {
  const s = session(t, [goal(), assistant(990000)]);
  for (const args of [['--threshold', '0'], ['--threshold', '100'], ['--threshold', '6.5'], ['--threshold', '065'],
    ['--threshold', 'abc'], ['--threshold'], ['--other', '65'], ['70']]) {
    assert.equal(guard(s, {}, { args }), null, args.join(' '));
    const out = guard(s, stop(true), { args });
    assertUnavailable(out);
    assert.ok(out.systemMessage.includes('--threshold'), args.join(' '));
  }
  s.write([assistant(990000)]);
  assert.equal(guard(s, stop(true), { args: ['--threshold', '0'] }), null, 'no active goal means no warning');
});

test('usage takes the last iteration and ignores output, sidechains, synthetic and API error records', (t) => {
  const s = session(t);
  const last = { input_tokens: 1, cache_creation_input_tokens: 0, cache_read_input_tokens: 700000, output_tokens: 5 };
  s.write([goal(), assistant(100, { iterations: [{ input_tokens: 1, cache_read_input_tokens: 10 }, last] })]);
  assert.match(guard(s).hookSpecificOutput.additionalContext, /^上下文估算使用率 70%/);

  s.write([goal(), assistant(800000, { iterations: [{ input_tokens: 1, cache_read_input_tokens: 10 }] })]);
  assert.equal(guard(s), null);

  for (const noise of [
    assistant(900000, { isSidechain: true }),
    assistant(900000, { model: '<synthetic>' }),
    assistant(900000, { isApiErrorMessage: true }),
  ]) {
    s.write([goal(), assistant(100000), noise]);
    assert.equal(guard(s), null, JSON.stringify(noise.message.model));
  }
});

test('only the last goal_status decides whether the goal is active', (t) => {
  const s = session(t);
  const over = assistant(750000);
  for (const [rows, active] of [
    [[over], false],
    [[goal(), over, goal({ met: true, sentinel: true })], false],
    [[goal(), over, goal({ met: false, failed: true })], false],
    [[goal({ met: false, reason: 'not yet' }), over], true],
    [[goal(), goal({ met: true }), over], false],
    [[goal(), goal({ met: true }), goal(), over], true],
    [[goal(), over, { ...goal({ met: true }), isSidechain: true }], true],
  ]) {
    s.write(rows);
    assert.equal(guard(s) !== null, active, JSON.stringify(rows.map((r) => r.attachment)));
    assert.equal(guard(s, stop(true)) !== null, active);
  }
});

test('compaction hides usage until a new statistic arrives', (t) => {
  const s = session(t);
  s.write([goal(), assistant(900000), compact]);
  assert.equal(guard(s), null);
  assert.equal(guard(s, stop(true)), null);
  s.write([goal(), assistant(900000), compact, assistant(760000)]);
  assert.ok(guard(s));
  s.write([goal()]);
  assert.equal(guard(s, stop(true)), null);
});

test('a half-written last line is skipped', (t) => {
  const s = session(t, [goal(), assistant(750000)], '{"type":"assistant","message":{"model":"claude-opus-5-5","usage":{"input');
  assert.ok(guard(s));
});

test('subagent events and other events are ignored', (t) => {
  const s = session(t);
  assert.equal(guard(s, { agent_id: 'a1', agent_type: 'general-purpose' }), null);
  assert.equal(guard(s, stop(true, { agent_id: 'a1' })), null);
  assert.equal(guard(s, { hook_event_name: 'PostToolUse' }), null);
  assert.equal(guard(s, { hook_event_name: 'SubagentStop', stop_hook_active: true }), null);
});

test('Stop asks for wrap-up once, then ends the turn', (t) => {
  const s = session(t);
  const tasks = [
    { id: 'b1', type: 'shell', status: 'running', description: 'sleep', command: 'sleep 150' },
    { id: 'a2', type: 'subagent', status: 'running', description: 'research task' },
  ];
  const block = guard(s, stop(false, { background_tasks: tasks }));
  assert.equal(block.decision, 'block');
  assert.equal(block.continue, undefined);
  assertReminder(block.reason);
  for (const part of ['b1', 'sleep 150', 'a2', 'research task', '只需一句确认后结束']) assert.ok(block.reason.includes(part), part);

  const end = guard(s, stop(true, { background_tasks: tasks }));
  assert.equal(end.continue, false);
  assert.equal(end.decision, undefined);
  for (const part of ['70%', 'guard 已结束本回合', 'goal 暂停', '/clear', '/goal', '补交接', '2 个后台任务']) {
    assert.ok(end.stopReason.includes(part), part);
  }
  assert.ok(!end.stopReason.includes('PRIVATE_GOAL'));

  const quiet = guard(s, stop(true, { background_tasks: [] }));
  assert.ok(!quiet.stopReason.includes('后台任务'));
  assert.ok(!guard(s, stop(false)).reason.includes('仍在运行的后台任务'));
});

test('unknown models and window overrides warn on Stop only and never end the turn', (t) => {
  const s = session(t, [goal(), assistant(990000, { model: 'claude-future-9' })]);
  assert.equal(guard(s), null);
  const out = guard(s, stop(true));
  assertUnavailable(out);
  assert.ok(out.systemMessage.includes('claude-future-9'));

  s.write([goal(), assistant(10000)]);
  for (const name of WINDOW_ENV) {
    assert.equal(guard(s, {}, { env: { [name]: '1' } }), null);
    const warned = guard(s, stop(false), { env: { [name]: '1' } });
    assertUnavailable(warned);
    assert.ok(warned.systemMessage.includes(name));
    assert.equal(guard(s, stop(false), { env: { [name]: '' } }), null);
  }

  s.write([assistant(990000, { model: 'claude-future-9' })]);
  assert.equal(guard(s, stop(true)), null, 'no active goal means no warning');
});

test('unreadable input degrades without blocking', (t) => {
  const s = session(t);
  assert.equal(guard(s, {}, { raw: 'not json' }), null);
  assert.equal(guard(s, {}, { raw: '' }), null);
  assertUnavailable(guard(s, stop(true, { transcript_path: path.join(s.dir, 'missing.jsonl') })));
  assert.equal(guard(s, { transcript_path: path.join(s.dir, 'missing.jsonl') }), null);
  assertUnavailable(guard(s, stop(true, { transcript_path: 42 })));

  s.write([goal(), assistant(700000, { iterations: [{ input_tokens: '7' }] })]);
  assertUnavailable(guard(s, stop(true)));
  assert.equal(guard(s), null);

  if (process.getuid && process.getuid() !== 0) {
    s.write([goal(), assistant(700000)]);
    fs.chmodSync(s.transcript, 0o000);
    try {
      assertUnavailable(guard(s, stop(true)));
    } finally {
      fs.chmodSync(s.transcript, 0o600);
    }
  }
});

test('large transcripts stay well inside the 5 second timeout', (t) => {
  const s = session(t);
  const filler = JSON.stringify({ type: 'user', isSidechain: false, message: { content: 'x'.repeat(4000) } }) + '\n';
  const fd = fs.openSync(s.transcript, 'w');
  fs.writeSync(fd, JSON.stringify(goal()) + '\n');
  for (let i = 0; i < 12000; i += 1) fs.writeSync(fd, filler);
  fs.writeSync(fd, JSON.stringify(assistant(750000)) + '\n');
  fs.closeSync(fd);
  assert.ok(fs.statSync(s.transcript).size > 40 * 1024 * 1024);
  const started = process.hrtime.bigint();
  assert.ok(guard(s));
  const ms = Number(process.hrtime.bigint() - started) / 1e6;
  assert.ok(ms < 2000, `${ms} ms`);
});

test('the hook never writes files', (t) => {
  const s = session(t);
  const snapshot = () => fs.readdirSync(s.dir).sort().map((name) => [name, fs.readFileSync(path.join(s.dir, name), 'utf8')]);
  const before = snapshot();
  guard(s);
  guard(s, stop(false));
  guard(s, stop(true));
  s.write([goal(), assistant(990000, { model: 'claude-future-9' })]);
  const unknown = snapshot();
  guard(s, stop(true));
  assert.deepEqual(snapshot(), unknown);
  s.write([goal(), assistant(700001)]);
  assert.deepEqual(snapshot(), before);
});
