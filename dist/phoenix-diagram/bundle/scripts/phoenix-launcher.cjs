#!/usr/bin/env node
'use strict';
// Apache-2.0: adapted from Phoenix UI launcher, engine 0.1.0.
// Phoenix modification: the installed diagram skill and its ENGINE identity.
const path = require('node:path');
const { main } = require('./phoenix-bootstrap.cjs');
const skillDir = path.resolve(__dirname, '..');
main(process.argv.slice(2), { skillDir, manifestPath: path.join(skillDir, 'ENGINE.json') })
  .then(code => { process.exitCode = code; })
  .catch(error => { process.stderr.write(`Phoenix Diagram: ${error.message}\n`); process.exitCode = 4; });
