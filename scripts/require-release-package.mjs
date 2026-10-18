#!/usr/bin/env node
// Checkout prepublish guard. Publishing an existing tgz does not run this hook;
// the UI-only release workflow must also stop before any release side effect.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const diagram = require('../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8'));

function included(pkg, file) {
  return Object.values(pkg.bin || {}).includes(file) || (pkg.files || []).some(entry =>
    typeof entry === 'string' && (entry === file || file.startsWith(`${entry.replace(/\/$/, '')}/`)));
}
export function includedPhoenixTools(root, pkg = readJson(path.join(root, 'package.json'))) {
  return ['phoenix-ui', 'phoenix-diagram'].filter(tool => pkg.bin?.[tool]
    || included(pkg, `dist/${tool}/VERSION.json`)
    || (included(pkg, `skills/${tool}/SKILL.md`) && fs.existsSync(path.join(root, `skills/${tool}/SKILL.md`))));
}
export function assertUiOnlyRelease(root) {
  if (includedPhoenixTools(root).includes('phoenix-diagram')) throw new Error(
    'Phoenix Diagram is included in this package. The UI-only release command cannot publish it: a joint release contract is required. Candidate assembly does not authorize public publication.');
}
function ordinary(file) {
  const stat = fs.lstatSync(file);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.nlink !== 1) throw new Error(`Untrusted release resource: ${file}`);
  const bytes = fs.readFileSync(file);
  if (!bytes.length) throw new Error(`Empty release resource: ${file}`);
  return bytes;
}
function publicHttps(value) {
  const url = new URL(value);
  return url.protocol === 'https:' && !url.username && !url.password
    && !['localhost', '[::1]'].includes(url.hostname) && !/^127\./.test(url.hostname)
    && !url.hostname.endsWith('.localhost');
}
function checkDiagramLicenses(root, pkg) {
  for (const name of ['LICENSE', 'NOTICE.md', 'THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md']) {
    const file = `dist/phoenix-diagram/${name}`;
    if (!included(pkg, file)) throw new Error(`Package files omit ${file}`);
    ordinary(path.join(root, file));
  }
  const catalog = readJson(path.join(root, 'dist/phoenix-diagram/THIRD-PARTY.json'));
  let count = 0;
  function visit(value) {
    if (!value || typeof value !== 'object') return;
    if (typeof value.path === 'string' && value.path.startsWith('THIRD-PARTY-LICENSES/')) {
      if (value.path.includes('\\') || value.path.split('/').some(part => !part || part === '.' || part === '..') || !/^[a-f0-9]{64}$/.test(value.sha256 || '')) throw new Error('Unsafe license resource binding');
      const file = `dist/phoenix-diagram/${value.path}`;
      if (!included(pkg, file) || sha(ordinary(path.join(root, file))) !== value.sha256) throw new Error(`Release license binding differs: ${file}`);
      count++;
    }
    for (const child of Object.values(value)) visit(child);
  }
  visit(catalog);
  if (!count) throw new Error('Diagram release has no full license resources');
}
export function assertPublishablePackage(root) {
  const pkg = readJson(path.join(root, 'package.json'));
  const tools = includedPhoenixTools(root, pkg);
  for (const tool of tools) {
    const file = path.join(root, `dist/${tool}/VERSION.json`);
    if (tool === 'phoenix-ui') {
      // Preserve the UI-only gate; its release workflow validates the full
      // platform/build/hosting contract separately.
      const manifest = readJson(file), engines = Object.values(manifest.engines || {});
      if (manifest.distribution !== 'release' || !engines.length || !engines.every(engine => /^https:\/\//.test(engine.url || ''))) throw new Error('dist/phoenix-ui/VERSION.json is a development manifest without public engine URLs');
      continue;
    }
    if (pkg.bin?.[tool] !== 'bin/phoenix-diagram.js' || !included(pkg, 'dist/phoenix-diagram/bundle/manifest.json')) throw new Error('Package bin/files omit the diagram consumer interface');
    const manifest = diagram.manifestAt(file, { packageJsonPath: path.join(root, 'package.json') });
    if (manifest.data.distribution !== 'release' || Object.keys(manifest.data.engines).length !== 4
      || !diagram.TARGETS.every(target => manifest.data.engines[target] && publicHttps(manifest.data.engines[target].url))) throw new Error('phoenix-diagram requires a complete release manifest with four public HTTPS URLs');
    const bundle = diagram.verifyBundle(path.join(root, 'dist/phoenix-diagram/bundle'), manifest);
    for (const item of bundle.data.files) if (!included(pkg, `dist/phoenix-diagram/bundle/${item.path}`)) throw new Error(`Package files omit diagram bundle entry ${item.path}`);
    if (!included(pkg, 'dist/phoenix-diagram/bundle/ENGINE.json')) throw new Error('Package files omit diagram ENGINE.json');
    checkDiagramLicenses(root, pkg);
  }
  return { tools };
}
if (process.argv[1] && fs.realpathSync(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    assertPublishablePackage(path.resolve(process.argv[2] || path.join(path.dirname(fileURLToPath(import.meta.url)), '..')));
  } catch (error) {
    console.error(`Refusing to publish: ${error.message}.\nUse npm run release:phoenix only for a UI-only package. Diagram candidates require a separate, explicitly authorized joint release workflow.`);
    process.exitCode = 1;
  }
}
