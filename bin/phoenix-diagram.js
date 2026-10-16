#!/usr/bin/env node
'use strict';
// Apache-2.0: Phoenix UI npm-launcher pattern, engine 0.1.0.
// Phoenix modification: diagram-only package identity and bundle resources.
const path = require('node:path');
const { main } = require('../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs');
const root = path.resolve(__dirname, '..');
main(process.argv.slice(2), {
  manifestPath: path.join(root, 'dist/phoenix-diagram/VERSION.json'),
  bundleRoot: path.join(root, 'dist/phoenix-diagram/bundle'),
  skillDir: path.join(root, 'skills/phoenix-diagram'),
  packageJsonPath: path.join(root, 'package.json'),
}).then(code => { process.exitCode = code; })
  .catch(error => { process.stderr.write(`Phoenix Diagram: ${error.message}\n`); process.exitCode = 4; });
