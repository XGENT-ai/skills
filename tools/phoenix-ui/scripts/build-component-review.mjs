#!/usr/bin/env bun
// Builds ui/component-review into the bundle the engine embeds. `--check`
// writes nothing and fails when the committed bundle no longer matches the
// source. Another Bun version emits different bytes, so the pinned one is required.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const workspace = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const asset = path.join(workspace, 'crates/context/assets/component-review.js');
const args = process.argv.slice(2);
if (args.some(arg => arg !== '--check')) {
  console.error('usage: bun scripts/build-component-review.mjs [--check]');
  process.exit(2);
}
const pinned = JSON.parse(fs.readFileSync(path.join(workspace, 'build-tools.lock.json'), 'utf8')).bun;
assert.equal(process.versions.bun, pinned, `Build with the pinned Bun ${pinned} (mise exec --no-deps -- bun ...)`);

const result = await Bun.build({
  entrypoints: [path.join(workspace, 'ui/component-review/entry.ts')],
  target: 'browser', format: 'iife', minify: true,
});
if (!result.success) {
  for (const log of result.logs) console.error(log);
  process.exit(1);
}
const bundle = await result.outputs[0].text();
const relative = path.relative(workspace, asset);
if (args.includes('--check')) {
  if (fs.readFileSync(asset, 'utf8') !== bundle) {
    console.error(`${relative} is stale; run mise exec --no-deps -- bun scripts/build-component-review.mjs`);
    process.exit(1);
  }
  console.log(`${relative} is current`);
} else {
  fs.writeFileSync(asset, bundle);
  console.log(`Wrote ${relative}`);
}
