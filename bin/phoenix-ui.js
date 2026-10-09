#!/usr/bin/env node
'use strict';

const path = require('node:path');
const { main, shellQuote } = require('../skills/phoenix-ui/scripts/phoenix-bootstrap.cjs');
const root = path.resolve(__dirname, '..');
main(process.argv.slice(2), {
  manifestPath: path.join(root, 'vendor/phoenix-ui/VERSION.json'),
  bundleRoot: path.join(root, 'vendor/phoenix-ui/bundle'),
  skillDir: path.join(root, 'skills/phoenix-ui'),
  self: `node ${shellQuote(__filename)}`,
}).then(code => { process.exitCode = code; }).catch(error => {
  process.stderr.write(`Phoenix UI: ${error.message}\n`);
  process.exitCode = 4;
});
