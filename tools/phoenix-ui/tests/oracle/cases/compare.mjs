/**
 * `compare` paths that never open a browser or leave a server listening:
 * help, schema, argument and payload errors, the headless refusal, and the
 * `--wait` outcomes staged through state and answer files. The served page
 * and its isolation are covered by crates/cli/tests/compare*.rs.
 */
import fs from 'node:fs';
import path from 'node:path';

const write = (ws, rel, text) => {
  const file = path.join(ws, rel);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text);
};

// `--start` decides "no browser" from these; pin them so a recording made on
// a desktop and a replay in CI take the same branch.
const env = (extra = {}) => ({
  CI: null,
  SSH_CONNECTION: null,
  PHOENIX_UI_QUESTION_DISABLED: null,
  PHOENIX_UI_QUESTION_FORCE: '1',
  ...extra,
});

const validRound = (ws) => {
  write(ws, 'variants/a/index.html', '<!doctype html><title>a</title><p>a</p>\n');
  write(ws, 'variants/b/index.html', '<!doctype html><title>b</title><p>b</p>\n');
  write(ws, 'round.json', JSON.stringify({
    proposals: [
      { id: 'a', name: '方案 A', source: 'variants/a/index.html' },
      { id: 'b', name: '方案 B', source: 'variants/b/index.html' },
    ],
  }));
};

const STATE = '.phoenix-ui/compare/k1.state.json';
const ANSWER = '.phoenix-ui/compare/k1.answer.json';
const served = (extra = {}) => JSON.stringify({ pid: 1, port: 1, url: 'http://127.0.0.1:1/', ...extra });
const FILES = ['.phoenix-ui/compare/**'];

export default [
  { id: 'compare-help', verb: 'compare', workspace: 'ctx-empty', args: ['--help'] },
  { id: 'compare-schema', verb: 'compare', workspace: 'ctx-empty', args: ['--schema'] },
  { id: 'compare-start-missing-payload', verb: 'compare', workspace: 'ctx-empty', args: ['--start'], env: env(), files: FILES },
  { id: 'compare-unknown-option', verb: 'compare', workspace: 'ctx-empty', args: ['--start', '--payload', 'round.json', '--opn'], setup: validRound, env: env(), files: FILES },
  {
    id: 'compare-invalid-payload',
    verb: 'compare',
    workspace: 'ctx-empty',
    args: ['--start', '--payload', 'round.json'],
    env: env(),
    files: FILES,
    setup(ws) {
      write(ws, 'variants/a/index.html', '<p>a</p>\n');
      write(ws, 'round.json', JSON.stringify({
        lang: 'fr',
        proposals: [
          { id: 'now', name: '现状', url: 'http://example.com/', baseline: true },
          { id: 'a', name: '方案 A', source: 'variants/a/index.html', baseline: true, pixelRatio: 2 },
          { id: 'a', name: '', source: 'variants/missing.html' },
          { id: 'out', name: '越界', source: '../outside.html', url: 'http://localhost:5173/' },
        ],
      }));
    },
  },
  {
    id: 'compare-single-proposal',
    verb: 'compare',
    workspace: 'ctx-empty',
    args: ['--start', '--payload', 'round.json'],
    env: env(),
    files: FILES,
    setup(ws) {
      write(ws, 'variants/a/index.html', '<p>a</p>\n');
      write(ws, 'round.json', JSON.stringify({ proposals: [{ id: 'a', name: '方案 A', source: 'variants/a/index.html' }] }));
    },
  },
  { id: 'compare-headless-ci-start', verb: 'compare', workspace: 'ctx-empty', args: ['--start', '--payload', 'round.json'], setup: validRound, env: env({ CI: '1', PHOENIX_UI_QUESTION_FORCE: null }), files: FILES },
  { id: 'compare-disabled-start', verb: 'compare', workspace: 'ctx-empty', args: ['--start', '--payload', 'round.json'], setup: validRound, env: env({ PHOENIX_UI_QUESTION_DISABLED: '1' }), files: FILES },
  // An uncollected pick from the previous round on the same key blocks a new round.
  { id: 'compare-start-uncollected-answer', verb: 'compare', workspace: 'ctx-empty', args: ['--start', '--key', 'k1', '--payload', 'round.json'], setup: (ws) => { validRound(ws); write(ws, ANSWER, JSON.stringify({ proposalId: 'b', name: '方案 B', baseline: false, liked: '', disliked: '' })); }, env: env(), files: FILES },
  // A live pid on the key (pid 1 always exists) blocks a second server.
  { id: 'compare-start-key-in-use', verb: 'compare', workspace: 'ctx-empty', args: ['--start', '--key', 'k1', '--payload', 'round.json'], setup: (ws) => { validRound(ws); write(ws, STATE, served()); }, env: env(), files: FILES },
  { id: 'compare-wait-no-key', verb: 'compare', workspace: 'ctx-empty', args: ['--wait'], env: env() },
  { id: 'compare-wait-no-server', verb: 'compare', workspace: 'ctx-empty', args: ['--wait', '--key', 'k1', '--poll', '2'], env: env(), files: FILES },
  { id: 'compare-wait-dead-pid', verb: 'compare', workspace: 'ctx-empty', setup: (ws) => write(ws, STATE, served({ pid: 2147483000, lastBeat: 1000 })), args: ['--wait', '--key', 'k1', '--poll', '2'], env: env(), files: FILES },
  { id: 'compare-wait-answer-ready', verb: 'compare', workspace: 'ctx-empty', setup: (ws) => { write(ws, STATE, served()); write(ws, ANSWER, `${JSON.stringify({ proposalId: 'b', name: '方案 B', baseline: false, liked: '列表清楚', disliked: '' })}\n`); }, args: ['--wait', '--key', 'k1', '--poll', '2'], env: env(), files: FILES },
  { id: 'compare-wait-answer-baseline', verb: 'compare', workspace: 'ctx-empty', setup: (ws) => { write(ws, STATE, served()); write(ws, ANSWER, `${JSON.stringify({ proposalId: 'now', name: '现状', baseline: true, liked: '', disliked: '新方案都太挤' })}\n`); }, args: ['--wait', '--key', 'k1', '--poll', '2'], env: env(), files: FILES },
  { id: 'compare-wait-page-closed', verb: 'compare', workspace: 'ctx-empty', setup: (ws) => write(ws, STATE, served({ lastBeat: 1000 })), args: ['--wait', '--key', 'k1', '--poll', '2'], env: env(), files: FILES },
  { id: 'compare-wait-still-waiting', verb: 'compare', workspace: 'ctx-empty', setup: (ws) => write(ws, STATE, served()), args: ['--wait', '--key', 'k1', '--poll', '1'], env: env(), files: FILES },
  { id: 'compare-stop', verb: 'compare', workspace: 'ctx-empty', setup: (ws) => { write(ws, STATE, served({ pid: 2147483000 })); write(ws, ANSWER, '{}\n'); }, args: ['--stop', '--key', 'k1'], env: env(), files: FILES },
];
