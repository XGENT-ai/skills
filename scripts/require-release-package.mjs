#!/usr/bin/env node
// prepublishOnly guard. npm runs it for `npm publish` from a checkout, never for `npm publish <tarball>`.
// A checkout carries a development Phoenix manifest without public engine URLs; publishing it ships a
// package whose engine cannot be installed. Releases go through `npm run release:phoenix`.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(process.argv[2] || path.join(path.dirname(fileURLToPath(import.meta.url)), '..'));
const manifest = JSON.parse(fs.readFileSync(path.join(root, 'vendor/phoenix-ui/VERSION.json'), 'utf8'));
const engines = Object.values(manifest.engines || {});
if (manifest.distribution !== 'release' || !engines.length || !engines.every(engine => /^https:\/\//.test(engine.url || ''))) {
  console.error('Refusing to publish: vendor/phoenix-ui/VERSION.json is a development manifest without public engine URLs.\n'
    + 'Publish with `npm run release:phoenix`, which uploads the tested engines and publishes the verified tarball.');
  process.exit(1);
}
