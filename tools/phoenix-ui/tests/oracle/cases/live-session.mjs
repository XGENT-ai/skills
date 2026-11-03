/**
 * `live-complete`, `live-poll`, `live-server` without a running helper server
 * (or against a server.json whose pid is the recorder).
 */
import { LIVE_FILES, rm, write } from '../live-helpers.mjs';

const F = LIVE_FILES;
const ID = 'ab12cd34';
const S = (id, verb, args, setup, extra = {}) => ({ id, verb, workspace: 'live-html', args, setup, files: F, ...extra });

const staleServerJson = (ws) => write(ws, '.phoenix-ui/live/server.json', '{"pid":2147483646,"port":8497,"token":"stale"}');
const aliveServerJson = (ws) => write(ws, '.phoenix-ui/live/server.json', JSON.stringify({ pid: process.pid, port: 65531, token: 'oracle-token' }));

export default [
  // --- live-complete ---
  S('live-complete-help', 'live-complete', ['--help'], null),
  S('live-complete-no-id', 'live-complete', [], null),
  S('live-complete-no-server', 'live-complete', ['--id', ID], null),
  // --- live-poll (no server, or a server.json whose pid is the recorder) ---
  S('live-poll-help', 'live-poll', ['--help'], null),
  S('live-poll-no-server', 'live-poll', [], null),
  S('live-poll-no-server-stale-record', 'live-poll', [], staleServerJson),
  S('live-poll-reply-missing-id', 'live-poll', ['--reply'], aliveServerJson),
  S('live-poll-reply-status-as-id', 'live-poll', ['--reply', 'done'], aliveServerJson),
  S('live-poll-reply-missing-status', 'live-poll', ['--reply', ID], aliveServerJson),
  S('live-poll-reply-status-flag', 'live-poll', ['--reply', ID, '--file', 'x'], aliveServerJson),
  S('live-poll-reply-bad-data-json', 'live-poll', ['--reply', ID, 'done', '--data', '{nope'], aliveServerJson),
  // Port 65531 on loopback: connection refused (nothing listens there).
  S('live-poll-reply-connection-refused', 'live-poll', ['--reply', ID, 'done'], aliveServerJson),
  S('live-poll-connection-refused', 'live-poll', ['--timeout=200'], aliveServerJson),
  // --- live-server (no process started) ---
  S('live-server-help', 'live-server', ['--help'], null),
  S('live-server-stop-none', 'live-server', ['stop'], null, { files: [...F, 'index.html'] }),
  S('live-server-stop-none-keep-inject', 'live-server', ['stop', '--keep-inject'], null),
  S('live-server-stop-removes-tag', 'live-server', ['stop'], (ws) => write(ws, 'index.html', '<html><body>\n<!-- impeccable-live-start -->\n<script src="http://localhost:8412/live.js?token=t"></script>\n<!-- impeccable-live-end -->\n</body></html>\n'), { files: [...F, 'index.html'] }),
  { id: 'live-server-stop-no-config', verb: 'live-server', workspace: 'live-vite', args: ['stop'], setup: (ws) => rm(ws, '.phoenix-ui/live/config.json'), files: F },
  { id: 'live-server-already-running', verb: 'live-server', workspace: 'live-html', args: [], setup: aliveServerJson, files: F },
  { id: 'live-server-target-missing-value', verb: 'live-server', workspace: 'live-html', args: ['--target'] },
];
