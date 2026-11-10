/**
 * `capture` paths that never start a browser: help, argument and state-file
 * errors (all reported before launch, exit 1), and an unavailable browser
 * (exit 1, no output directory). Browser runs are covered by
 * crates/cli/tests/capture.rs.
 */
import fs from 'node:fs';
import path from 'node:path';

const page = (ws) => fs.writeFileSync(path.join(ws, 'page.html'), '<!doctype html><title>x</title><p>page</p>\n');

export default [
  { id: 'capture-help', verb: 'capture', args: ['--help'] },
  { id: 'capture-stitch-help', verb: 'capture', args: ['stitch', '--help'] },
  { id: 'capture-unknown-option', verb: 'capture', args: ['page.html', '--viewpor', 'desktop'], setup: page },
  { id: 'capture-bad-viewport', verb: 'capture', args: ['page.html', '--viewport', '50x50'], setup: page },
  {
    id: 'capture-state-file-errors',
    verb: 'capture',
    args: ['page.html', '--states', 'states.json'],
    env: { PHOENIX_UI_BROWSER: '/nonexistent' },
    setup(ws) {
      page(ws);
      fs.writeFileSync(path.join(ws, 'states.json'), JSON.stringify({
        states: [
          { name: 'initial', steps: [{ do: 'clik', target: { css: 'a' } }] },
          { name: 'open', steps: [{ do: 'click', taget: { css: 'a' } }] },
        ],
      }));
    },
  },
  {
    id: 'capture-browser-unavailable',
    verb: 'capture',
    args: ['page.html', '--out', 'shots'],
    env: { PHOENIX_UI_BROWSER: '/nonexistent' },
    setup: page,
    files: ['shots/**'],
  },
];
