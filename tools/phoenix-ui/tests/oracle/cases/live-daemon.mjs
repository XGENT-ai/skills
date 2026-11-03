/**
 * One end-to-end helper-server flow driven through the daemon step support
 * in lib.mjs: start `live-server` detached, then status / poll / reply /
 * complete / stop against it. Everything run-dependent (port, token, pids,
 * lease and phase timestamps, durations) is masked per-case.
 */
import { LIVE_FILES } from '../live-helpers.mjs';

const ID = 'ab12cd34';
const NORM = [
  ['localhost:\\d{4,5}', 'g', 'localhost:<PORT>'],
  ['"(port|serverPort)":(\\s*)\\d{4,5}', 'g', '"$1":$2<PORT>'],
  ['Stopped live server on port \\d+\\.', 'g', 'Stopped live server on port <PORT>.'],
  ['already running on port \\d+ ', 'g', 'already running on port <PORT> '],
  ['"(leaseUntil|generationReadyAt|lastPollAt|generationCompletedAt|generationCanceledAt|at|leasedAt|clientSentAt)":(\\s*)\\d{10,}', 'g', '"$1":$2<EPOCH>'],
  ['"(durationMs|scaffoldDurationMs)":(\\s*)\\d+(?:\\.\\d+)?', 'g', '"$1":$2<N>'],
];

export default [
  {
    id: 'live-daemon-server-poll-complete', workspace: 'live-html', files: [...LIVE_FILES, 'index.html'], normalize: NORM,
    // Sessions live in the helper's memory: with no browser event queued the
    // poll times out, and replies for unknown sessions are rejected.
    steps: [
      { verb: 'live-server', daemon: true, readyFile: '.phoenix-ui/live/server.json', readyTimeoutMs: 15000 },
      { verb: 'live-server', args: [] },
      { verb: 'live-poll', args: ['--timeout=300'] },
      { verb: 'live-poll', args: ['--reply', 'zz99zz99', 'done'] },
      { verb: 'live-complete', args: ['--id', ID] },
      { verb: 'live-server', args: ['stop'] },
    ],
  },
];
