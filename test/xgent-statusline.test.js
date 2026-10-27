'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const path = require('node:path');

const script = path.resolve(__dirname, '..', '.claude', 'hooks', 'xgent-statusline.js');

function statusline(contextWindow, env = {}) {
  const childEnv = { ...process.env, ...env };
  if (!('CLAUDE_CODE_AUTO_COMPACT_WINDOW' in env)) delete childEnv.CLAUDE_CODE_AUTO_COMPACT_WINDOW;
  const result = spawnSync(process.execPath, [script], {
    input: JSON.stringify({ context_window: contextWindow }),
    env: childEnv,
    encoding: 'utf8',
    timeout: 10000,
  });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.replace(/\x1b\[[\d;]*m/g, '');
}

// Shaped like Claude Code's statusLine context_window: occupancy is the last
// call's input + cache creation + cache read tokens.
function usage(tokens, size) {
  const used = Math.round((tokens / size) * 100);
  return {
    total_input_tokens: tokens,
    total_output_tokens: 1000,
    context_window_size: size,
    current_usage: { input_tokens: 2000, output_tokens: 1000, cache_creation_input_tokens: 3000, cache_read_input_tokens: tokens - 5000 },
    used_percentage: used,
    remaining_percentage: 100 - used,
  };
}

test('counts down to the auto-compact trigger of a 1M window', () => {
  assert.equal(statusline(usage(835_000, 1_000_000)), '💀 ████████░░ 86% 132K available');
  assert.equal(statusline(usage(950_000, 1_000_000)), '💀 █████████░ 98% 17K available');
  assert.equal(statusline(usage(970_000, 1_000_000)), '💀 ██████████ 100% 0K available');
});

test('reserves the same tokens on a 200K window', () => {
  assert.equal(statusline(usage(100_000, 200_000)), '██████░░░░ 60% 67K available');
});

test('treats CLAUDE_CODE_AUTO_COMPACT_WINDOW as the window size', () => {
  assert.equal(statusline(usage(300_000, 1_000_000), { CLAUDE_CODE_AUTO_COMPACT_WINDOW: '400k' }),
    '💀 ████████░░ 82% 67K available');
  assert.equal(statusline(usage(300_000, 1_000_000), { CLAUDE_CODE_AUTO_COMPACT_WINDOW: '400000' }),
    '💀 ████████░░ 82% 67K available');
  // Claude Code clamps to the model window and ignores unusable values.
  assert.equal(statusline(usage(100_000, 200_000), { CLAUDE_CODE_AUTO_COMPACT_WINDOW: '1M' }),
    '██████░░░░ 60% 67K available');
  assert.equal(statusline(usage(100_000, 200_000), { CLAUDE_CODE_AUTO_COMPACT_WINDOW: 'abc' }),
    '██████░░░░ 60% 67K available');
});

test('prints nothing before the first response', () => {
  assert.equal(statusline({ ...usage(0, 1_000_000), current_usage: null, used_percentage: null, remaining_percentage: null }), '');
});
