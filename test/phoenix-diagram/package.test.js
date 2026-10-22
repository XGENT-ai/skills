'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const root = path.resolve(__dirname, '../..');

test('npm exposes an independent diagram bin and carries its consumer resources', () => {
  const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json')));
  assert.equal(pkg.bin['phoenix-diagram'], 'bin/phoenix-diagram.js');
  for (const file of ['docs/phoenix-diagram.md', 'dist/phoenix-diagram/bundle', 'dist/phoenix-diagram/VERSION.json', 'dist/phoenix-diagram/LICENSE', 'dist/phoenix-diagram/NOTICE.md', 'dist/phoenix-diagram/THIRD-PARTY.json', 'dist/phoenix-diagram/THIRD-PARTY-NOTICES.md', 'dist/phoenix-diagram/THIRD-PARTY-LICENSES']) assert.ok(pkg.files.includes(file), file);
  assert.equal(pkg.bin['phoenix-ui'], 'bin/phoenix-ui.js');
  assert.equal(pkg.engines.node, '>=18');
});

test('installer help distinguishes the two tools and their offline directories', () => {
  const result = spawnSync(process.execPath, [path.join(root, 'bin/xgent-skills.js'), '--help'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  for (const value of ['--no-phoenix-ui', '--no-phoenix-diagram', '--diagram-release-dir=DIR', '--release-dir=DIR', 'Phoenix Diagram']) assert.ok(result.stdout.includes(value), value);
});
