'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const http = require('node:http');
const { gzipSync, gunzipSync } = require('node:zlib');
const { spawnSync } = require('node:child_process');

const root = path.resolve(__dirname, '../..');
const prepare = import('../../scripts/prepare-phoenix-ui.mjs');
const smoke = import('../../scripts/smoke-phoenix-ui.mjs');
const buildTools = import('../../scripts/phoenix-build-tools.mjs');

function sandbox(t) {
  const directory = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'phoenix build tools space '));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  return directory;
}

function tar(entries, footer = true) {
  const chunks = [];
  for (const { name, bytes = '', type = '0', link = '' } of entries) {
    const data = Buffer.from(bytes), header = Buffer.alloc(512);
    header.write(name, 0, 100); header.write('0000755\0', 100, 8);
    header.write('0000000\0', 108, 8); header.write('0000000\0', 116, 8);
    header.write(data.length.toString(8).padStart(11, '0') + '\0', 124, 12);
    header.write('00000000000\0', 136, 12); header.fill(32, 148, 156);
    header.write(type, 156, 1); header.write(link, 157, 100); header.write('ustar\0', 257, 6);
    const checksum = header.reduce((sum, byte) => sum + byte, 0);
    header.write(checksum.toString(8).padStart(6, '0') + '\0 ', 148, 8);
    chunks.push(header, data, Buffer.alloc((512 - data.length % 512) % 512));
  }
  if (footer) chunks.push(Buffer.alloc(1024));
  return gzipSync(Buffer.concat(chunks));
}

async function archive(entries = []) {
  const { sha256 } = await prepare;
  const bytes = tar([{ name: 'fixture-tool', type: '5' }, { name: 'fixture-tool/bin/tool', bytes: 'fixture executable bytes\n' }, ...entries]);
  return { bytes, asset: { name: 'fixture.tar.gz', url: 'https://example.invalid/fixture.tar.gz', size: bytes.length, sha256: sha256(bytes), archiveRoot: 'fixture-tool', executable: 'bin/tool' } };
}

test('five native tool pins match the unchanged tool lock and verified official action commits', async () => {
  const { assets, validatePins, nativePlatform, preparationCommands } = await prepare;
  validatePins();
  assert.equal(Object.keys(assets.platforms).length, 5);
  assert.equal(nativePlatform('win32', 'x64'), 'windows-x64');
  assert.throws(() => nativePlatform('win32', 'arm64'), /separately verified x64 fallback/);
  for (const platform of Object.keys(assets.platforms)) {
    const commands = preparationCommands(platform);
    assert.deepEqual(commands.find(([, args]) => args.includes('fetch')), ['mise', ['exec', '--no-deps', '--', 'mbx', 'fetch', '--locked']]);
    assert(commands.some(([, args]) => args.includes('install') && args.includes('--frozen-lockfile')));
    assert(commands.some(([, args]) => args.includes(assets.platforms[platform].rustTarget) && args.includes('wasm32-unknown-unknown')));
    assert(!JSON.stringify(commands).includes('wasm-pack'));
  }
  for (const [repository, entry] of Object.entries(assets.actions)) {
    assert.match(entry.commit, /^[a-f0-9]{40}$/);
    assert.equal(entry.source, `https://github.com/${repository}/commit/${entry.commit}`);
    assert.equal(entry.definition, `https://raw.githubusercontent.com/${repository}/${entry.commit}/action.yml`);
  }
});

test('Linux ARM Chromium is verified against its frozen executable identity', async () => {
  const { verifyPreparedBrowser } = await prepare;
  // Actual frozen Playwright install from the Linux ARM CI preparation record.
  const browser = { package: 'playwright', packageVersion: '1.59.1', revision: '1217',
    expectedVersion: '147.0.7727.15', actualVersion: 'Chromium 147.0.7727.0',
    sha256: '6b25a228f91c7a08c5404568816edc9365dd09cfd35b8620dca12e77a82fbb4b' };
  assert.doesNotThrow(() => verifyPreparedBrowser(browser));
  for (const [field, value] of Object.entries({ package: 'other', packageVersion: '1.59.2', revision: '1218',
    expectedVersion: '148.0.7727.15', actualVersion: 'Chromium 147.0.7727.15', sha256: '0'.repeat(64) })) {
    assert.throws(() => verifyPreparedBrowser({ ...browser, [field]: value }), /Prepared Chromium/);
  }
});

test('Intel macOS bootstraps the exact mbx source before attempting its unavailable release asset', async () => {
  const { assets, preparationCommands } = await prepare;
  const commands = preparationCommands('darwin-x64');
  const bootstrap = commands.findIndex(([, args]) => args.includes('--git'));
  assert(bootstrap >= 0, 'mbx 1.21.1 has no Intel macOS release asset');
  assert(commands[0][1].includes('rust@1.99.0'));
  assert(!commands[0][1].includes('mr-boxington@1.21.1'));
  const args = commands[bootstrap][1];
  assert.deepEqual(args.slice(0, 9), ['exec', '--no-deps', '--', 'rustup', 'run', '1.99.0', 'cargo', 'install', '--locked']);
  assert.equal(args[args.indexOf('--git') + 1], 'https://github.com/jdx/mr-boxington');
  assert.equal(args[args.indexOf('--rev') + 1], 'a0a44c61ca6aaa8da41d59deeebdfc46fc9d3313');
  assert.equal(args.at(-1), 'mbx');
  assert.equal(assets.nativeBootstrap['darwin-x64'].version, '1.21.1');
  const link = commands.findIndex(([, args]) => args[0] === 'link');
  assert(link > bootstrap);
  assert.equal(commands[link][1][1], 'mr-boxington@1.21.1');
  assert(!commands[link][1].includes('--force'));
  assert(commands.findIndex(([, args]) => args.length === 1 && args[0] === 'install') > link);
  for (const platform of ['darwin-arm64', 'linux-x64', 'linux-arm64', 'windows-x64']) {
    assert(!preparationCommands(platform).some(([, args]) => args.includes('--git')));
  }
  assert(!preparationCommands('darwin-x64', true).some(([, args]) => args.includes('--git')));
});

test('mise environment retains its nested PATH record while removing Cargo proxies and transient shim state', async () => {
  const { miseEnvironment } = await buildTools;
  const env = miseEnvironment({
    __MISE_DIFF: 'opaque mise environment record', __MISE_SHIM: '1',
    PATH: [path.join(os.homedir(), '.cargo/bin'), 'explicit tool directory', 'original system directory'].join(path.delimiter),
  });
  assert.equal(env.__MISE_DIFF, 'opaque mise environment record');
  assert.equal(env.__MISE_SHIM, undefined);
  assert.deepEqual(env.PATH.split(path.delimiter), ['explicit tool directory', 'original system directory']);
  assert.equal(env.MISE_AUTO_INSTALL, 'false'); assert.equal(env.MISE_EXEC_AUTO_INSTALL, 'false');
});

test('Cargo wrapper accepts the exact Windows native shim copy and refuses unrelated executables', async t => {
  const { validateCargoWrapper } = await buildTools;
  const directory = sandbox(t), bin = path.join(directory, 'mise-bin');
  const wrappers = path.join(directory, 'command-wrappers/bin');
  fs.mkdirSync(bin); fs.mkdirSync(wrappers, { recursive: true });
  const mise = path.join(bin, 'mise.exe'), shim = path.join(bin, 'mise-shim.exe');
  const cargo = path.join(wrappers, 'cargo.exe');
  fs.writeFileSync(mise, 'mise executable'); fs.writeFileSync(shim, 'original native dispatcher');
  fs.copyFileSync(shim, cargo);
  const tools = { cargo, realCargo: fs.realpathSync(cargo), mise };
  validateCargoWrapper(tools, 'win32');
  fs.writeFileSync(cargo, 'unrelated cargo executable');
  assert.throws(() => validateCargoWrapper(tools, 'win32'), /native shim/);
  fs.copyFileSync(shim, cargo);
  assert.throws(() => validateCargoWrapper({ ...tools, cargo: mise }, 'win32'), /wrapper/);
  fs.unlinkSync(shim);
  assert.throws(() => validateCargoWrapper(tools, 'win32'), /native shim/);
});

if (process.platform === 'darwin') {
  test('macOS build and test profiles preserve Unix IPC and enforce their IP boundaries', async t => {
    const { offlineNetworkProfile, testNetworkProfile } = await buildTools;
    const directory = sandbox(t);
    const probe = (profile, source) => {
      const result = spawnSync('sandbox-exec', ['-p', profile, process.execPath, '-e', source],
        { encoding: 'utf8', timeout: 5000 });
      assert.equal(result.status, 0, result.stderr || result.stdout);
      return result.stdout.trim();
    };
    for (const profile of [offlineNetworkProfile, testNetworkProfile]) {
      assert.equal(probe(profile, `const net=require('node:net'),fs=require('node:fs');const p=${JSON.stringify(path.join(directory, 'socket'))};const s=net.createServer();s.on('error',e=>{console.error(e.code);process.exitCode=1;});s.listen(p,()=>s.close(()=>{fs.rmSync(p,{force:true});console.log('allowed');}));`), 'allowed');
      assert.equal(probe(profile, "const s=require('node:net').connect({host:'192.0.2.1',port:443});s.on('error',e=>console.log(e.code));s.on('connect',()=>{s.destroy();process.exitCode=1;});s.setTimeout(1000,()=>{s.destroy();process.exitCode=1;});"), 'EPERM');
    }
    const listen = "const s=require('node:net').createServer();s.on('error',e=>console.log(e.code));s.listen(0,'127.0.0.1',()=>s.close(()=>console.log('allowed')));";
    assert.equal(probe(offlineNetworkProfile, listen), 'EPERM');
    assert.equal(probe(testNetworkProfile, listen), 'allowed');
  });
}

if (process.env.PHOENIX_BUILD_TOOL_NESTED_NODE === '1') {
  test('explicit installed-tool check keeps pinned Node and the mise Cargo wrapper in nested and build environments', async () => {
    const { workspace, versions, validateCargoWrapper } = await buildTools;
    const source = `
      import path from 'node:path';
      import { mise, buildEnvironment } from '../../scripts/phoenix-build-tools.mjs';
      const inspect = "const fs=require('node:fs'),path=require('node:path'); const find=name=>process.env.PATH.split(path.delimiter).map(p=>path.join(p,name+(process.platform==='win32'?'.exe':''))).find(p=>fs.existsSync(p));const cargo=find('cargo');console.log(JSON.stringify({version:process.versions.node,cargo,realCargo:cargo&&fs.realpathSync(cargo),mise:find('mise')}));";
      const nested = JSON.parse(mise(['node','-e',inspect]));
      const env = buildEnvironment({ bindgen:path.resolve('fixture-tools/wasm-bindgen'), opt:path.resolve('fixture-tools/wasm-opt'), path:process.env.PATH, cargo:nested.cargo });
      const build = JSON.parse(mise(['node','-e',inspect], {env}));
      console.log(JSON.stringify({outer:process.versions.node,nested,build}));
    `;
    const result = spawnSync('mise', ['exec', '--no-deps', '--', 'node', '--input-type=module', '-e', source], {
      cwd: workspace, env: { ...process.env, MISE_AUTO_INSTALL: 'false', MISE_EXEC_AUTO_INSTALL: 'false' }, encoding: 'utf8', timeout: 15000,
    });
    assert.equal(result.status, 0, result.stderr);
    const actual = JSON.parse(result.stdout);
    assert.equal(actual.outer, versions.node, 'outer command must use the pinned Node');
    for (const selected of [actual.nested, actual.build]) {
      assert.equal(selected.version, versions.node);
      validateCargoWrapper(selected);
    }
  });
}

test('download accepts exact streamed bytes and rejects incomplete, oversized, substituted and non-HTTPS responses', async () => {
  const { downloadAsset } = await prepare;
  const { bytes, asset } = await archive();
  const request = value => async (url, options) => {
    assert.equal(url, asset.url); assert(options.signal instanceof AbortSignal);
    return new Response(new ReadableStream({ start(controller) {
      controller.enqueue(value.subarray(0, 10)); controller.enqueue(value.subarray(10)); controller.close();
    } }));
  };
  assert.deepEqual(await downloadAsset(asset, request(bytes)), bytes);
  for (const bad of [bytes.subarray(0, bytes.length - 1), Buffer.concat([bytes, Buffer.from('x')]), Buffer.alloc(bytes.length)]) {
    await assert.rejects(downloadAsset(asset, request(bad)), /size|SHA-256/);
  }
  await assert.rejects(downloadAsset({ ...asset, url: 'http://example.invalid/fixture' }, request(bytes)), /HTTPS/);
  await assert.rejects(downloadAsset(asset, async () => new Response('missing', { status: 404 })), /404/);
});

if (process.env.PHOENIX_BUILD_TOOL_PROXY === '1') {
  test('explicit pinned-Node proxy fixture sends CONNECT and refuses a rejected tunnel without a direct fallback', async t => {
    const { downloadAsset } = await prepare;
    const f = await archive();
    const requests = [];
    const server = http.createServer();
    server.on('connect', (request, socket) => {
      requests.push(request.url);
      socket.end('HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n');
    });
    await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
    t.after(() => new Promise(resolve => server.close(resolve)));
    const keys = ['HTTP_PROXY', 'http_proxy', 'HTTPS_PROXY', 'https_proxy', 'NO_PROXY', 'no_proxy'];
    const old = Object.fromEntries(keys.map(key => [key, process.env[key]]));
    try {
      for (const key of keys) delete process.env[key];
      process.env.HTTPS_PROXY = `http://127.0.0.1:${server.address().port}`;
      await assert.rejects(downloadAsset(f.asset));
      assert.deepEqual(requests, ['example.invalid:443']);
    } finally {
      for (const key of keys) { if (old[key] === undefined) delete process.env[key]; else process.env[key] = old[key]; }
    }
  });
}

test('safe extraction materializes real directories/files, preserves bytes and rejects modified or extra installed files', async t => {
  const { extractArchive } = await prepare;
  const f = await archive([{ name: 'fixture-tool/README.md', bytes: 'fixed source document\n' }]);
  const destination = path.join(sandbox(t), 'nested tool directory/fixture');
  const executable = extractArchive(f.bytes, f.asset, destination);
  assert.equal(fs.readFileSync(executable, 'utf8'), 'fixture executable bytes\n');
  assert.equal(fs.readFileSync(path.join(destination, 'README.md'), 'utf8'), 'fixed source document\n');
  assert.equal(extractArchive(f.bytes, f.asset, destination), executable, 'identical existing trees are reusable');
  fs.writeFileSync(executable, 'modified');
  assert.throws(() => extractArchive(f.bytes, f.asset, destination), /differs/);
  fs.writeFileSync(executable, 'fixture executable bytes\n');
  fs.writeFileSync(path.join(destination, 'unrelated-user-file'), 'retain me');
  assert.throws(() => extractArchive(f.bytes, f.asset, destination), /differs/);
  assert.equal(fs.readFileSync(path.join(destination, 'unrelated-user-file'), 'utf8'), 'retain me');
});

test('traversal, Windows aliases, duplicate names, parent-file collisions and link/device/extended-header members write nothing', async t => {
  const { extractArchive } = await prepare;
  const directory = sandbox(t), marker = path.join(directory, 'user-file');
  fs.writeFileSync(marker, 'retain original');
  const badMembers = [
    { name: '../user-file' }, { name: '/absolute/file' }, { name: 'C:/file' },
    { name: 'fixture-tool/../user-file' }, { name: 'fixture-tool\\escape' },
    { name: 'fixture-tool/./file' }, { name: 'fixture-tool/NUL.txt' },
    { name: 'fixture-tool/file.' }, { name: 'wrong-root/file' },
    { name: 'fixture-tool/bin/TOOL' }, { name: 'fixture-tool/bin', bytes: 'file used as directory' },
    ...['1', '2', '3', '4', '6', 'x', 'g', 'L'].map(type => ({ name: `fixture-tool/member-${type}`, type, link: type === '2' ? '../../user-file' : '' })),
  ];
  for (const member of badMembers) {
    const f = await archive([member]), destination = path.join(directory, 'destination');
    assert.throws(() => extractArchive(f.bytes, f.asset, destination));
    assert.equal(fs.existsSync(destination), false, JSON.stringify(member));
    assert.equal(fs.readFileSync(marker, 'utf8'), 'retain original');
  }
  assert.deepEqual(fs.readdirSync(directory), ['user-file']);
});

test('archive digest, tar checksum and footer are all verified before creating a destination', async t => {
  const { extractArchive, sha256 } = await prepare;
  const f = await archive(), directory = sandbox(t);
  const raw = gunzipSync(f.bytes); raw[2] ^= 1;
  const brokenHeader = gzipSync(raw), noFooter = tar([{ name: 'fixture-tool/bin/tool', bytes: 'bytes' }], false);
  const cases = [
    [Buffer.concat([f.bytes, Buffer.from('x')]), f.asset],
    [brokenHeader, { ...f.asset, size: brokenHeader.length, sha256: sha256(brokenHeader) }],
    [noFooter, { ...f.asset, size: noFooter.length, sha256: sha256(noFooter) }],
  ];
  for (const [bytes, asset] of cases) {
    assert.throws(() => extractArchive(bytes, asset, path.join(directory, 'out')));
    assert.deepEqual(fs.readdirSync(directory), []);
  }
});

test('existing symlinks in tool parents, destinations and installed files are refused', async t => {
  const { extractArchive } = await prepare;
  const f = await archive(), directory = sandbox(t), other = path.join(directory, 'other');
  fs.mkdirSync(other);
  fs.symlinkSync(other, path.join(directory, 'linked'), process.platform === 'win32' ? 'junction' : 'dir');
  assert.throws(() => extractArchive(f.bytes, f.asset, path.join(directory, 'linked/out')), /Unsafe/);
  assert.throws(() => extractArchive(f.bytes, f.asset, path.join(directory, 'linked')), /Unsafe/);
  const destination = path.join(directory, 'installed');
  extractArchive(f.bytes, f.asset, destination);
  fs.unlinkSync(path.join(destination, 'bin/tool'));
  fs.symlinkSync(path.join(directory, 'missing'), path.join(destination, 'bin/tool'));
  assert.throws(() => extractArchive(f.bytes, f.asset, destination), /differs/);
  assert.deepEqual(fs.readdirSync(other), []);
});

test('default/help/invalid preparation invocations make no writes and need no mise, Cargo or download', t => {
  const directory = sandbox(t);
  for (const args of [[], ['--help'], ['--unexpected']]) {
    const result = spawnSync(process.execPath, [path.join(root, 'scripts/prepare-phoenix-ui.mjs'), ...args], {
      cwd: directory, env: { ...process.env, PATH: '', HTTP_PROXY: 'http://127.0.0.1:9', HTTPS_PROXY: 'http://127.0.0.1:9' }, encoding: 'utf8', timeout: 5000,
    });
    assert.equal(result.status, args[0] === '--unexpected' ? 1 : 0, result.stderr);
    assert.match(result.stdout || result.stderr, /Usage:/);
    assert.deepEqual(fs.readdirSync(directory), []);
  }
});

async function runtimeFixture(t) {
  const { assets, sha256 } = await prepare;
  const target = `${process.platform === 'win32' ? 'windows' : process.platform}-${process.arch}`;
  assert(assets.platforms[target]);
  const directory = sandbox(t), release = path.join(directory, 'release'), pkg = path.join(directory, 'package');
  fs.mkdirSync(release);
  const bytes = Buffer.from('A protocol fixture only; never a Rust runtime artifact.\n');
  const asset = `phoenix-ui-${target}${target.startsWith('windows') ? '.exe' : ''}`;
  const bundle = Buffer.from('{"schema":1,"providers":{}}\n');
  const engine = { asset, sha256: sha256(bytes), size: bytes.length };
  const manifest = { schemaVersion: 1, distribution: 'development', toolVersion: '0.1.0', npmPackageVersion: '0.7.0-rc.0',
    sourceCommit: 'a'.repeat(40), bundleSchema: 1, bundleSha256: sha256(bundle), reviewSchema: 1, engines: { [target]: engine } };
  const version = Buffer.from(JSON.stringify(manifest, null, 2) + '\n');
  fs.writeFileSync(path.join(release, asset), bytes);
  fs.writeFileSync(path.join(release, 'VERSION.json'), version);
  fs.writeFileSync(path.join(release, 'build-record.json'), JSON.stringify({ target, binary: engine, bundleSha256: manifest.bundleSha256 }));
  for (const [relative, value] of Object.entries({
    'bin/phoenix-ui.js': '// fixture public entry\n',
    'skills/phoenix-ui/scripts/phoenix-bootstrap.cjs': '// fixture generated bootstrap\n',
    'skills/phoenix-ui/scripts/ENGINE.json': version,
    'skills/phoenix-ui/src/editable.md': 'must not distribute the editable source\n',
    'vendor/phoenix-ui/bundle/manifest.json': bundle,
    'vendor/phoenix-ui/LICENSE': 'fixture license\n', 'vendor/phoenix-ui/NOTICE.md': 'fixture notice\n',
  })) {
    const file = path.join(pkg, relative); fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, value);
  }
  fs.writeFileSync(path.join(release, 'THIRD-PARTY.json'), '{}\n');
  fs.writeFileSync(path.join(release, 'THIRD-PARTY-NOTICES.md'), 'fixture third-party notice\n');
  fs.mkdirSync(path.join(release, 'THIRD-PARTY-LICENSES'));
  fs.writeFileSync(path.join(release, 'THIRD-PARTY-LICENSES/terms.txt'), 'full fixture terms\n');
  return { directory, release, pkg, target, asset, version };
}

test('runtime stage carries the exact manifest, generated portable files, bundle and full license sidecars', async t => {
  const { stageRuntime, validateRuntimeArtifact } = await smoke, f = await runtimeFixture(t);
  validateRuntimeArtifact(f.release, f.target);
  const output = stageRuntime(f.release, f.pkg);
  assert.equal(fs.existsSync(path.join(output, 'skills/phoenix-ui/src')), false);
  assert(fs.readFileSync(path.join(output, 'vendor/phoenix-ui/VERSION.json')).equals(f.version));
  assert(fs.readFileSync(path.join(output, 'skills/phoenix-ui/scripts/ENGINE.json')).equals(f.version));
  assert.equal(fs.readFileSync(path.join(output, 'vendor/phoenix-ui/THIRD-PARTY-LICENSES/terms.txt'), 'utf8'), 'full fixture terms\n');
  fs.appendFileSync(path.join(f.release, f.asset), 'tampered');
  assert.throws(() => validateRuntimeArtifact(f.release, f.target), /size differs/);
});

test('runtime failures preserve evidence and never label a corrupt fixture as a passed engine smoke', async t => {
  const { stageRuntime, runSmoke } = await smoke, f = await runtimeFixture(t);
  stageRuntime(f.release, f.pkg);
  fs.appendFileSync(path.join(f.release, f.asset), 'tampered');
  assert.throws(() => runSmoke(f.release, f.target), /size differs/);
  const record = JSON.parse(fs.readFileSync(path.join(f.release, `runtime-smoke-node${process.versions.node.split('.')[0]}.json`)));
  assert.equal(record.status, 'failed'); assert.deepEqual(record.checks, []);
  assert.equal(record.browserAcceptance, 'not-run'); assert.equal(record.providerSessionAcceptance, 'not-run');
});

test('detector smoke validates the actual CLI Finding field and rejects unrelated or empty output', async () => {
  const { assertStaticDetection } = await smoke;
  assertStaticDetection(JSON.stringify([{ antipattern: 'overused-font', severity: 'warning' }]));
  for (const value of [[], [{ antipattern: 'different-rule' }], [{ type: 'overused-font' }], {}]) {
    assert.throws(() => assertStaticDetection(JSON.stringify(value)), /seeded rule/);
  }
});

test('CI has exactly five native runners, explicit prepare, locked offline gates, separate consumer/fallback and read-only pinned actions', async () => {
  const { assets } = await prepare, workflow = fs.readFileSync(path.join(root, '.github/workflows/phoenix-ui.yml'), 'utf8');
  const native = workflow.slice(workflow.indexOf('  native:'), workflow.indexOf('  windows-arm-fallback:'));
  assert.equal((native.match(/- platform:/g) || []).length, 5);
  for (const [platform, entry] of Object.entries(assets.platforms)) {
    assert(native.includes(`platform: ${platform}\n            runner: ${entry.runner}\n            rust-target: ${entry.rustTarget}`));
  }
  for (const [, repository, commit] of workflow.matchAll(/uses: ([\w/-]+)@([^\s#]+)/g)) assert.equal(commit, assets.actions[repository]?.commit);
  assert.match(workflow, /permissions:\n  contents: read/);
  const preparation = workflow.slice(workflow.indexOf('      - name: Explicit online preparation'), workflow.indexOf('      - name: Audit prepared dependencies offline'));
  assert.match(preparation, /GITHUB_TOKEN: \$\{\{ github\.token \}\}/);
  assert.match(workflow, /workflow_dispatch:/); assert.match(workflow, /pull_request:/);
  assert(workflow.indexOf('prepare-phoenix-ui.mjs --prepare') < workflow.indexOf('build-phoenix-ui.mjs --development'));
  const light = workflow.indexOf('npm --prefix ../.. test');
  assert(workflow.indexOf('prepare-phoenix-ui.mjs --prepare') < light);
  assert(light < workflow.indexOf('build-phoenix-ui.mjs --development'),
    'Committed artifact evidence must be checked before CI rewrites generated metadata.');
  assert.match(workflow, /check-phoenix-deps\.mjs --write/);
  assert.match(workflow, /run test:phoenix/); assert.match(workflow, /--prefix \.\.\/\.\. test/);
  assert.equal((native.match(/CARGO_NET_OFFLINE: 'true'/g) || []).length, 5);
  assert.match(workflow, /runs-on: windows-11-arm[\s\S]*name: phoenix-ui-windows-x64[\s\S]*--windows-arm-fallback/);
  assert.match(workflow, /node-version: '18\.20\.8'/);
  assert(!/secrets\.|publish|CARGO_TARGET_DIR|wasm-pack/.test(workflow));
  assert(!/continue-on-error/.test(workflow));
});

if (process.env.PHOENIX_BUILD_TOOL_ARCHIVE) {
  test('explicit real fixed tool archive verifies its raw digest and materializes every actual file', async t => {
    const { assets, archiveEntries, extractArchive, sha256 } = await prepare;
    const file = path.resolve(process.env.PHOENIX_BUILD_TOOL_ARCHIVE);
    const asset = Object.values(assets.tools).flatMap(tool => Object.values(tool.assets)).find(entry => entry.name === path.basename(file));
    assert(asset, 'Real archive must match a fixed asset');
    const bytes = fs.readFileSync(file), entries = archiveEntries(bytes, asset), destination = path.join(sandbox(t), 'verified actual tree');
    const executable = extractArchive(bytes, asset, destination);
    assert(fs.statSync(executable).isFile());
    for (const entry of entries.filter(member => !member.directory)) {
      const installed = path.join(destination, entry.name.slice(asset.archiveRoot.length + 1));
      assert.equal(sha256(fs.readFileSync(installed)), sha256(entry.bytes));
    }
  });
}
