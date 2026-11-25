#!/usr/bin/env node
// prepublishOnly guard: a package is publishable only with both Phoenix tools' release manifests.
// `npm publish <tgz>` skips this hook, so release:phoenix also runs it on the staged package.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const releaseTargets = ['darwin-arm64', 'linux-arm64', 'linux-x64'];
export const releaseManifestPaths = {
  'phoenix-ui': ['dist/phoenix-ui/VERSION.json', 'skills/phoenix-ui/scripts/ENGINE.json'],
  'phoenix-diagram': ['dist/phoenix-diagram/VERSION.json', 'dist/phoenix-diagram/bundle/ENGINE.json', 'skills/phoenix-diagram/ENGINE.json'],
};
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
  for (const [tool, [versionFile, ...copies]] of Object.entries(releaseManifestPaths)) {
    const bytes = fs.readFileSync(path.join(root, versionFile));
    const manifest = JSON.parse(bytes);
    if (manifest.distribution !== 'release' || manifest.npmPackageVersion !== pkg.version
        || Object.keys(manifest.engines || {}).length !== releaseTargets.length
        || !releaseTargets.every(target => publicHttps(manifest.engines?.[target]?.url))) {
      throw new Error(`${versionFile} must be a release manifest of ${pkg.version} with public HTTPS engine URLs for exactly ${releaseTargets.join(', ')}`);
    }
    for (const copy of copies) {
      if (!included(pkg, copy) || !fs.readFileSync(path.join(root, copy)).equals(bytes)) throw new Error(`${copy} must ship the same release manifest as ${versionFile}`);
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
