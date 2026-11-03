'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { pathToFileURL } = require('node:url');
const root = path.resolve(__dirname, '../..');
const targets = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64'];
const tools = ['phoenix-ui', 'phoenix-diagram'];
function write(dir, name, value) {
  const file = path.join(dir, name); fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, value);
}
// A package that carries both tools' release manifests; `change` edits one manifest or package.json first.
function fixture(t, change = () => {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix publish guard '));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const pkg = { name: '@xgent-ai/skills', version: '1.2.3', bin: { 'phoenix-ui': 'bin/phoenix-ui.js', 'phoenix-diagram': 'bin/phoenix-diagram.js' },
    files: ['bin', 'skills', 'dist/phoenix-ui/bundle', 'dist/phoenix-ui/VERSION.json', 'dist/phoenix-diagram/bundle', 'dist/phoenix-diagram/VERSION.json'] };
  const manifests = Object.fromEntries(tools.map(tool => [tool, { distribution: 'release', npmPackageVersion: pkg.version,
    engines: Object.fromEntries(targets.map(target => [target, { asset: `${tool}-${target}`, size: 1, sha256: 'a'.repeat(64), url: `https://download.example.test/${tool}-${target}` }])) }]));
  change({ pkg, manifests });
  write(dir, 'package.json', JSON.stringify(pkg));
  for (const tool of tools) {
    write(dir, `bin/${tool}.js`, ''); write(dir, `skills/${tool}/scripts/phoenix-bootstrap.cjs`, ''); write(dir, `dist/${tool}/bundle/manifest.json`, '{}');
    if (manifests[tool]) write(dir, `dist/${tool}/VERSION.json`, JSON.stringify(manifests[tool]));
  }
  return dir;
}
const guard = dir => spawnSync(process.execPath, [path.join(root, 'scripts/require-release-package.mjs'), dir], { encoding: 'utf8' });

test('the publish guard refuses development packages when invoked through a workspace symlink', { skip: process.platform === 'win32' }, t => {
  const alias = path.join(fixture(t), 'workspace');
  fs.symlinkSync(root, alias, 'dir');
  const result = spawnSync(process.execPath, [path.join(alias, 'scripts/require-release-package.mjs'), root], { encoding: 'utf8' });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Refusing to publish:[\s\S]*npm run release:phoenix/);
});

test('the publish guard accepts both release manifests with four public HTTPS engine URLs', t => {
  const result = guard(fixture(t));
  assert.equal(result.status, 0, result.stderr);
});

test('the publish guard refuses either tool without a complete release manifest or what its bin loads', t => {
  const changes = {
    missing: (m, tool) => delete m[tool],
    development: (m, tool) => { m[tool].distribution = 'development'; },
    'missing platform': (m, tool) => delete m[tool].engines['darwin-x64'],
    'other package version': (m, tool) => { m[tool].npmPackageVersion = '9.9.9'; },
    'loopback URL': (m, tool) => { m[tool].engines['linux-x64'].url = 'http://127.0.0.1:48765/engine'; },
    'missing URL': (m, tool) => delete m[tool].engines['linux-arm64'].url,
    'bundle left out of files': (m, tool, pkg) => { pkg.files = pkg.files.filter(entry => entry !== `dist/${tool}/bundle`); },
  };
  for (const tool of tools) for (const [name, change] of Object.entries(changes)) {
    const result = guard(fixture(t, ({ pkg, manifests }) => change(manifests, tool, pkg)));
    assert.equal(result.status, 1, `${tool} ${name}`);
    assert.match(result.stderr, new RegExp(tool), `${tool} ${name}`);
  }
});

test('releases require main and reject release branches and detached HEAD', async () => {
  const { assertReleaseBranch } = await import(pathToFileURL(path.join(root, 'scripts/require-release-package.mjs')));
  assert.doesNotThrow(() => assertReleaseBranch('main'));
  for (const branch of ['release/0.7.6', 'feature/diagram', 'HEAD', '']) {
    assert.throws(() => assertReleaseBranch(branch), /main/);
  }
});
