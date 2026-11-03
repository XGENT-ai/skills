#!/usr/bin/env node
// prepublishOnly guard: a package is publishable only with both Phoenix tools' release manifests.
// `npm publish <tgz>` skips this hook, so release:phoenix also runs it on the staged package.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const targets = ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64'];
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8'));

function included(pkg, file) {
  return (pkg.files || []).some(entry => entry === file || file.startsWith(`${entry.replace(/\/$/, '')}/`));
}
function publicHttps(value) {
  if (!/^https:\/\//.test(value || '')) return false;
  const url = new URL(value);
  return !url.username && !url.password
    && !['localhost', '[::1]'].includes(url.hostname) && !/^127\./.test(url.hostname)
    && !url.hostname.endsWith('.localhost');
}
export function assertReleaseBranch(branch) {
  if (branch !== 'main') throw new Error('Release only from main; release branches and detached HEAD are not supported.');
}
export function assertPublishablePackage(root) {
  const pkg = readJson(path.join(root, 'package.json'));
  for (const tool of ['phoenix-ui', 'phoenix-diagram']) {
    const manifest = readJson(path.join(root, `dist/${tool}/VERSION.json`));
    if (manifest.distribution !== 'release' || manifest.npmPackageVersion !== pkg.version || !targets.every(target => publicHttps(manifest.engines?.[target]?.url))) {
      throw new Error(`dist/${tool}/VERSION.json is not a release manifest of ${pkg.version} with four public HTTPS engine URLs`);
    }
    // What bin/<tool>.js loads: its skill bootstrap, the manifest and the bundle.
    for (const file of [pkg.bin?.[tool], `skills/${tool}/scripts/phoenix-bootstrap.cjs`, `dist/${tool}/VERSION.json`, `dist/${tool}/bundle/manifest.json`]) {
      if (!file || !included(pkg, file) || !fs.existsSync(path.join(root, file))) throw new Error(`Package omits ${file || `the ${tool} bin`}`);
    }
  }
}
if (process.argv[1] && fs.realpathSync(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    assertPublishablePackage(path.resolve(process.argv[2] || path.join(path.dirname(fileURLToPath(import.meta.url)), '..')));
  } catch (error) {
    console.error(`Refusing to publish: ${error.message}.\nUse npm run release:phoenix from main; development manifests cannot be published.`);
    process.exitCode = 1;
  }
}
