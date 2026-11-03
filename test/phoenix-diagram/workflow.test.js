'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const root = path.resolve(__dirname, '../..');
const prepare = import('../../scripts/prepare-phoenix-diagram.mjs');
const source = import('../../scripts/check-phoenix-diagram-source.mjs');
const assets = JSON.parse(fs.readFileSync(path.join(root, 'tools/phoenix-ui/build-tool-assets.json')));
const workflow = () => fs.readFileSync(path.join(root, '.github/workflows/phoenix.yml'), 'utf8');
const job = (text, name) => new RegExp(`^  ${name}:\\n([\\s\\S]*?)(?=^  [a-z][a-z0-9-]*:|$(?![\\s\\S]))`, 'm').exec(text)?.[1] || '';
function temporary(t) { const directory = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'diagram workflow ')); t.after(() => fs.rmSync(directory, { recursive: true, force: true })); return directory; }

test('workflow uses four native runners and only the already fixed UI action commits', () => {
  const text = workflow(), native = job(text, 'native');
  for (const name of ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64']) {
    const pin = assets.platforms[name];
    assert.match(native, new RegExp(`platform: ${name}\\s+runner: ${pin.runner}\\s+rust-target: ${pin.rustTarget}`));
  }
  for (const match of text.matchAll(/uses:\s*([^@\s]+)@([^\s]+)/g)) assert.equal(match[2], assets.actions[match[1]]?.commit, match[0]);
  assert.match(text, /contents: read/); assert.doesNotMatch(text, /contents: write|npm publish|gh release/);
});

test('native builds use the prepared clean source offline', () => {
  const native = job(workflow(), 'native');
  assert.match(native, /prepare-phoenix-diagram\.mjs --prepare/);
  assert.match(native, /build-phoenix-diagram\.mjs --development --output-dir/);
  assert.match(native, /mbx test --manifest-path tools\/phoenix-diagram\/Cargo\.toml --locked --offline/);
  assert.match(native, /PHOENIX_DIAGRAM_BUILD_TARGET: \$\{\{ matrix\.rust-target \}\}/);
  assert.match(native, /git status --porcelain/);
  assert.doesNotMatch(native, /prepare-phoenix-ui|build-phoenix-ui|test:phoenix|npm pack/);
});

test('browser pin helpers preserve fixed source identity and reject every changed Chromium field', async () => {
  const checker = await source, preparer = await prepare;
  assert.deepEqual(checker.browserPin(), assets.linuxArmBrowser);
  // Schema-only copy of actual pinned metadata; no executable or acceptance claim.
  const observation = { ...assets.linuxArmBrowser, status: 'available', executable: '/fixture-only/chromium', bytes: 10, verification: 'not-run' };
  assert.doesNotThrow(() => preparer.verifyPreparedBrowser(observation, 'linux-arm64'));
  for (const field of ['package', 'packageVersion', 'revision', 'expectedVersion', 'actualVersion', 'sha256', 'source']) {
    assert.throws(() => preparer.verifyPreparedBrowser({ ...observation, [field]: 'changed' }, 'linux-arm64'), /Chromium|pin|identity/);
  }
});

test('read-only discovery distinguishes a missing browser from a broken explicit path without launching anything', async t => {
  const checker = await source, directory = temporary(t), fallback = process.execPath;
  assert.equal(checker.discoverBrowser({ env: {}, candidates: [] }).status, 'unavailable');
  const env = { PHOENIX_DIAGRAM_BROWSER: path.join(directory, 'missing') };
  const bad = checker.discoverBrowser({ env, candidates: [fallback] });
  assert.equal(bad.status, 'failed'); assert.equal(bad.diagnostic.code, 'browser/configured-path');
  assert.equal(env.PHOENIX_DIAGRAM_BROWSER, path.join(directory, 'missing'));
  const found = checker.discoverBrowser({ env: {}, candidates: [fallback] });
  assert.equal(found.status, 'available'); assert.equal(found.path, fs.realpathSync(fallback)); assert.equal(found.verification, 'not-run');
});

test('ARM browser preparation is diagram-local, explicitly online and binds the fixed Playwright revision', async t => {
  const preparer = await prepare, directory = temporary(t), pin = assets.linuxArmBrowser;
  const commands = preparer.browserPreparationCommands('linux-arm64', { directory });
  const encoded = JSON.stringify(commands);
  assert.ok(encoded.includes(`playwright@${pin.packageVersion}`));
  assert.ok(encoded.includes('--ignore-scripts')); assert.ok(encoded.includes('--with-deps')); assert.ok(encoded.includes('--no-shell'));
  assert.ok(encoded.includes(directory)); assert.doesNotMatch(encoded, /bun|wasm|binaryen|phoenix-ui|fetch.*latest/i);
  assert.deepEqual(preparer.browserPreparationCommands('darwin-arm64', { directory }), []);
  assert.deepEqual(preparer.browserPreparationCommands('linux-x64', { directory }), []);
});

test('Intel preparation uses the fixed native mbx bootstrap without importing UI tool preparation', async () => {
  const preparer = await prepare, versions = (await source).loadVersions();
  const commands = preparer.preparationCommands(versions, true, { platform: 'darwin-x64', boxingtonAvailable: false });
  const bootstrap = commands.find(([, args]) => args.includes('--git'));
  assert.ok(bootstrap); assert.ok(bootstrap[1].includes(assets.nativeBootstrap['darwin-x64'].revision));
  assert.doesNotMatch(JSON.stringify(commands), /bun|wasm|binaryen/);
});
