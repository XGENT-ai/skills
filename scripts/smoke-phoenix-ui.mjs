#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import bootstrap from '../skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const json = value => JSON.stringify(value, null, 2) + '\n';

export function assertStaticDetection(output) {
  const findings = JSON.parse(output);
  assert(Array.isArray(findings) && findings.some(finding => finding.antipattern === 'overused-font'), 'Static HTML detector must report the seeded rule');
}

export function validateRuntimeArtifact(releaseDir, target) {
  const manifest = bootstrap.manifestAt(path.join(releaseDir, 'VERSION.json'));
  const engine = manifest.data.engines[target];
  assert(engine, `No verified artifact for ${target}`);
  const file = path.join(releaseDir, engine.asset);
  const stat = fs.lstatSync(file);
  assert(stat.isFile() && !stat.isSymbolicLink(), 'Runtime artifact must be a regular file');
  assert.equal(stat.size, engine.size, 'Runtime artifact size differs');
  assert.equal(bootstrap.sha256(fs.readFileSync(file)), engine.sha256, 'Runtime artifact SHA-256 differs');
  const record = JSON.parse(fs.readFileSync(path.join(releaseDir, 'build-record.json')));
  assert.equal(record.target, target, 'Compiler build record target differs');
  assert.equal(record.binary.sha256, engine.sha256, 'Compiler build record binary differs');
  assert.equal(record.binary.size, engine.size, 'Compiler build record binary size differs');
  assert.equal(record.bundleSha256, manifest.data.bundleSha256, 'Compiler build record bundle differs');
  return { manifest, file };
}

export function stageRuntime(releaseDir, packageRoot = root) {
  const dest = path.join(releaseDir, 'runtime');
  assert(!fs.existsSync(dest), 'Runtime staging already exists');
  const sourceSkill = path.join(packageRoot, 'skills/phoenix-ui');
  assert(fs.existsSync(path.join(sourceSkill, 'scripts/phoenix-bootstrap.cjs')), 'Build must generate the portable skill before smoke');
  fs.mkdirSync(path.join(dest, 'bin'), { recursive: true });
  fs.copyFileSync(path.join(packageRoot, 'bin/phoenix-ui.js'), path.join(dest, 'bin/phoenix-ui.js'));
  fs.cpSync(sourceSkill, path.join(dest, 'skills/phoenix-ui'), {
    recursive: true, dereference: false, filter: file => file !== path.join(sourceSkill, 'src'),
  });
  const vendor = path.join(dest, 'dist/phoenix-ui');
  fs.mkdirSync(vendor, { recursive: true });
  fs.cpSync(path.join(packageRoot, 'dist/phoenix-ui/bundle'), path.join(vendor, 'bundle'), { recursive: true });
  for (const name of ['VERSION.json', 'THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md', 'THIRD-PARTY-LICENSES']) {
    fs.cpSync(path.join(releaseDir, name), path.join(vendor, name), { recursive: true });
  }
  for (const name of ['LICENSE', 'NOTICE.md']) fs.copyFileSync(path.join(packageRoot, 'dist/phoenix-ui', name), path.join(vendor, name));
  fs.copyFileSync(path.join(releaseDir, 'VERSION.json'), path.join(dest, 'skills/phoenix-ui/scripts/ENGINE.json'));
  return dest;
}

export function runSmoke(releaseDir, target, { fallback = false, stage = false } = {}) {
  assert.equal(bootstrap.platformTarget(), target, 'Smoke must execute the artifact on the intended host');
  if (fallback) assert(process.platform === 'win32' && process.arch === 'arm64', 'Windows ARM fallback requires native ARM64 Node on windows-11-arm');
  const record = { schemaVersion: 1, startedAt: new Date().toISOString(), status: 'running', target,
    host: { platform: process.platform, architecture: process.arch, node: process.version },
    mode: fallback ? 'windows-arm64-x64-fallback' : 'native', checks: [],
    browserAcceptance: 'not-run', providerSessionAcceptance: 'not-run', paidAcceptance: 'not-run' };
  let temporary;
  try {
    const { manifest, file } = validateRuntimeArtifact(releaseDir, target);
    record.manifestSha256 = manifest.hash; record.binarySha256 = manifest.data.engines[target].sha256;
    const runtime = stage ? stageRuntime(releaseDir) : path.join(releaseDir, 'runtime');
    assert(fs.readFileSync(path.join(runtime, 'dist/phoenix-ui/VERSION.json')).equals(manifest.bytes), 'Downloaded runtime VERSION bytes differ');
    const bundle = fs.readFileSync(path.join(runtime, 'dist/phoenix-ui/bundle/manifest.json'));
    assert.equal(bootstrap.sha256(bundle), manifest.data.bundleSha256, 'Runtime bundle digest differs');
    if (process.platform !== 'win32') fs.chmodSync(file, 0o755); // Artifacts do not preserve Unix execute modes.
    temporary = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'phoenix ci smoke space '));
    const project = path.join(temporary, 'project'), home = path.join(temporary, 'home'), cache = path.join(temporary, 'engine cache');
    fs.mkdirSync(path.join(project, '.git'), { recursive: true }); fs.mkdirSync(home);
    fs.writeFileSync(path.join(project, 'PRODUCT.md'), '# Product\nPhoenix CI local fixture.\n');
    fs.writeFileSync(path.join(project, 'DESIGN.md'), '# Design\nUse Georgia for the fixture.\n');
    fs.writeFileSync(path.join(project, 'page.html'), '<!doctype html><html><head><style>body{font-family:Inter,sans-serif}</style></head><body><main><h1>CI fixture</h1></main></body></html>');
    const env = { ...process.env, HOME: home, USERPROFILE: home, PHOENIX_UI_HOME: cache,
      PHOENIX_UI_PROVIDER_ID: 'claude-code', PHOENIX_UI_SKILL_DIR: path.join(runtime, 'skills/phoenix-ui'),
      PHOENIX_UI_NO_STALENESS_CHECK: '1', CARGO_NET_OFFLINE: 'true',
      HTTPS_PROXY: 'http://127.0.0.1:9', HTTP_PROXY: 'http://127.0.0.1:9', ALL_PROXY: 'http://127.0.0.1:9',
      https_proxy: 'http://127.0.0.1:9', http_proxy: 'http://127.0.0.1:9', all_proxy: 'http://127.0.0.1:9', NO_PROXY: '', no_proxy: '' };
    delete env.PHOENIX_UI_BUNDLE_PATH;
    const call = (name, executable, args, expected, input) => {
      const result = spawnSync(executable, args, { cwd: project, env, input, encoding: 'utf8', timeout: 30000, maxBuffer: 4 * 1024 * 1024, windowsHide: true });
      record.checks.push({ name, argv: args, exitCode: result.status, error: result.error?.message });
      assert(!result.error && result.status === expected, `${name} failed: ${result.error?.message || result.stderr || result.stdout} (exit ${result.status})`);
      return result.stdout;
    };
    assert.equal(call('engine-probe', file, ['engine-probe'], 0).trim(), `phoenix-ui-engine ${manifest.data.toolVersion}`);
    assert.equal(call('version', file, ['--version'], 0).trim(), manifest.data.toolVersion);
    assert.match(call('root-help', file, ['--help'], 0), /Usage: phoenix-ui/);
    const skills = JSON.parse(call('local-command-catalog', file, ['help', '--json'], 0));
    assert.equal(skills.status, 'available'); assert.equal(Object.keys(skills.commands).length, 23);
    assertStaticDetection(call('static-html-detect', file, ['detect', '--no-config', '--json', 'page.html'], 2));
    assert.match(call('local-context', file, ['context', '--target', project], 0), /PRODUCT\.md[\s\S]*Phoenix CI local fixture/);
    call('hook-session-start', file, ['hook'], 0, JSON.stringify({ cwd: project, session_id: 'phoenix-ci', hook_event_name: 'SessionStart' }));
    const launcher = path.join(runtime, 'bin/phoenix-ui.js');
    const node = (name, args, expected) => call(name, process.execPath, [launcher, ...args], expected);
    // Browser and missing-engine checks need stderr as well as the exit code.
    const nodeResult = (name, args) => {
      const result = spawnSync(process.execPath, [launcher, ...args], { cwd: project, env, encoding: 'utf8', timeout: 120000, maxBuffer: 16 * 1024 * 1024, windowsHide: true });
      record.checks.push({ name, argv: args, exitCode: result.status, error: result.error?.message });
      assert(!result.error, `${name} failed: ${result.error?.message}`);
      return result;
    };
    assert.equal(JSON.parse(node('node-missing-engine', ['context', '--json'], 4)).status, 'unavailable');
    const missing = nodeResult('node-missing-engine-capture', ['capture', '--help']);
    assert(missing.status === 4 && /phoenix-ui engine install --project/.test(missing.stderr), `capture without an engine must exit 4 with the install command (exit ${missing.status})`);
    const missingMedia = nodeResult('node-missing-engine-media', ['media', '--help']);
    assert(missingMedia.status === 4 && /phoenix-ui engine install --project/.test(missingMedia.stderr), `media without an engine must exit 4 with the install command (exit ${missingMedia.status})`);
    const missingCompare = nodeResult('node-missing-engine-compare', ['compare', '--help']);
    assert(missingCompare.status === 4 && /phoenix-ui engine install --project/.test(missingCompare.stderr), `compare without an engine must exit 4 with the install command (exit ${missingCompare.status})`);
    assert.equal(JSON.parse(node('node-explicit-offline-install', ['engine', 'install', '--project', project, '--release-dir', releaseDir, '--json'], 0)).status, 'installed');
    assert.equal(node('node-engine-version', ['--version'], 0).trim(), manifest.data.toolVersion);
    assert.match(node('node-context', ['context', '--target', project], 0), /Phoenix CI local fixture/);
    assert.match(node('node-capture-help', ['capture', '--help'], 0), /phoenix-ui capture/);
    assert.match(node('node-media-help', ['media', '--help'], 0), /phoenix-ui media/);
    assert.match(node('node-prep-image-help', ['prep-image', '--help'], 0), /phoenix-ui prep-image/);
    assert.match(node('node-compare-help', ['compare', '--help'], 0), /phoenix-ui compare/);
    const media = JSON.parse(node('node-media-offline', ['media', 'search', 'map', '--timeout-ms', '1000', '--json'], 1));
    assert.equal(media.count, 0);
    assert.match(media.message, /网络不可用/);
    assert(!fs.existsSync(path.join(project, '.phoenix-ui/media/index.json')), 'Offline media search must not write an index');
    record.media = 'network-unavailable';
    const outside = nodeResult('node-media-outside-area', ['media', 'fetch', 'T1', '--dir', 'out']);
    assert(outside.status === 1 && /素材只取到 \.phoenix-ui\//.test(outside.stderr), `media must refuse a product directory (exit ${outside.status}): ${outside.stderr}`);
    assert(!fs.existsSync(path.join(project, 'out')), 'Rejected media destination must not be created');
    const imagePath = path.join(project, 'small.png');
    fs.writeFileSync(imagePath, Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC', 'base64'));
    const provenance = 'Phoenix CI smoke: generated one-pixel red fixture.';
    node('node-prep-image-provenance', ['embed-prompt', 'small.png', '--prompt', provenance], 0);
    const originalImage = fs.readFileSync(imagePath);
    const prepared = JSON.parse(node('node-prep-image-process', ['prep-image', 'small.png', '--width', '100', '--json'], 0));
    assert.equal(prepared.errors.length, 0);
    assert.equal(prepared.files.length, 1);
    assert.equal(prepared.files[0].outputs.length, 1);
    const output = prepared.files[0].outputs[0];
    assert.equal(output.format, 'jpeg');
    assert.equal(output.width, 1); assert.equal(output.height, 1);
    assert(prepared.files[0].notes.some(note => note.includes('未放大')), 'Small input must report that it was not enlarged');
    assert(fs.readFileSync(path.join(project, output.file)).subarray(0, 2).equals(Buffer.from([0xff, 0xd8])), 'prep-image must produce a JPEG');
    assert(fs.readFileSync(imagePath).equals(originalImage), 'prep-image must preserve its input bytes');
    assert.equal(node('node-prep-image-read-provenance', ['embed-prompt', output.file, '--read'], 0).trim(), provenance);
    record.prepImage = 'processed';
    for (const id of ['a', 'b']) {
      fs.mkdirSync(path.join(project, 'variants', id), { recursive: true });
      fs.writeFileSync(path.join(project, 'variants', id, 'index.html'), `<!doctype html><title>${id}</title><p>${id}</p>`);
    }
    const proposal = id => ({ id, name: `Proposal ${id}`, source: `variants/${id}/index.html` });
    fs.writeFileSync(path.join(project, 'single.json'), JSON.stringify({ proposals: [proposal('a')] }));
    fs.writeFileSync(path.join(project, 'round.json'), JSON.stringify({ lang: 'en', proposals: [proposal('a'), proposal('b')] }));
    // CI has no display; the page itself is exercised by the engine's browser tests.
    const compare = (name, args) => {
      const result = spawnSync(process.execPath, [launcher, 'compare', ...args], { cwd: project, env: { ...env, PHOENIX_UI_QUESTION_FORCE: '1' }, encoding: 'utf8', timeout: 60000, maxBuffer: 4 * 1024 * 1024, windowsHide: true });
      record.checks.push({ name, argv: ['compare', ...args], exitCode: result.status, error: result.error?.message });
      assert(!result.error, `${name} failed: ${result.error?.message}`);
      return result;
    };
    const single = compare('node-compare-single-proposal', ['--start', '--payload', 'single.json']);
    assert(single.status === 1 && !fs.existsSync(path.join(project, '.phoenix-ui/compare')), `compare must refuse a single proposal without serving (exit ${single.status}): ${single.stderr}`);
    const started = compare('node-compare-start', ['--start', '--key', 'smoke', '--payload', 'round.json', '--timeout', '60']);
    const compareUrl = /^COMPARE URL: (http:\/\/127\.0\.0\.1:\d+\/)$/m.exec(started.stdout)?.[1];
    assert(started.status === 0 && compareUrl && /^COMPARE KEY: smoke$/m.test(started.stdout), `compare --start must print the address and key (exit ${started.status}): ${started.stderr || started.stdout}`);
    try {
      // What the page does on confirm, plus one post from a foreign origin that must not count.
      const page = spawnSync(process.execPath, ['-e', `
        const http = require('node:http');
        const base = process.argv[1];
        const send = (method, target, headers, body) => new Promise((resolve, reject) => {
          const request = http.request(new URL(target, base), { method, headers }, response => {
            let text = ''; response.setEncoding('utf8');
            response.on('data', chunk => { text += chunk; });
            response.on('end', () => resolve({ status: response.statusCode, text }));
          });
          request.on('error', reject); request.end(body);
        });
        (async () => {
          const shell = await send('GET', '/', {});
          const json = { 'Content-Type': 'application/json' };
          const forged = await send('POST', '/answer?key=smoke', { ...json, Origin: 'https://example.invalid' }, JSON.stringify({ proposalId: 'a' }));
          const answer = await send('POST', '/answer?key=smoke', { ...json, Origin: new URL(base).origin }, JSON.stringify({ proposalId: 'b', liked: 'smoke' }));
          console.log(JSON.stringify({ shell: shell.status, data: shell.text.includes('id="compare-data"'), forged: forged.status, answer: answer.status }));
        })().catch(error => { console.error(error); process.exit(1); });
      `, compareUrl], { encoding: 'utf8', timeout: 30000, windowsHide: true });
      assert.deepEqual(JSON.parse(page.stdout || '{}'), { shell: 200, data: true, forged: 403, answer: 200 }, `compare page requests failed: ${page.stderr || page.stdout}`);
      const waited = compare('node-compare-wait', ['--wait', '--key', 'smoke', '--poll', '10']);
      const picked = JSON.parse(/^ANSWER: (.+)$/m.exec(waited.stdout)?.[1] || 'null');
      assert(waited.status === 0 && picked?.proposalId === 'b' && picked.liked === 'smoke' && picked.baseline === false, `compare --wait must return the choice (exit ${waited.status}): ${waited.stdout}`);
      record.compare = 'answered';
    } finally { compare('node-compare-stop', ['--stop', '--key', 'smoke']); }
    const shot = nodeResult('node-capture-shot', ['capture', 'page.html', '--viewport', '390x844', '--out', 'capture']);
    if (shot.status === 0) {
      const report = JSON.parse(fs.readFileSync(path.join(project, 'capture/capture-report.json'), 'utf8'));
      const entry = report.shots.find(item => item.file === '390x844.png');
      assert(entry?.valid && entry.pixels.width === 390 && entry.pixels.height === 844, 'capture must write a valid 390x844 shot');
      record.capture = 'screenshot';
    } else {
      assert(shot.status === 1 && /浏览器不可用/.test(shot.stderr) && !fs.existsSync(path.join(project, 'capture')), `capture must take a shot or report the browser unavailable (exit ${shot.status}): ${shot.stderr}`);
      record.capture = 'browser-unavailable';
    }
    assert.equal(JSON.parse(node('local-bundle-check', ['check', '--bundle-root', path.join(runtime, 'dist/phoenix-ui/bundle'), '--json'], 0)).status, 'not-installed');
    record.status = 'passed'; record.finishedAt = new Date().toISOString();
    return record;
  } catch (error) { record.status = 'failed'; record.error = error.message; throw error; }
  finally {
    fs.writeFileSync(path.join(releaseDir, fallback ? 'windows-arm-fallback-smoke.json' : `runtime-smoke-node${process.versions.node.split('.')[0]}.json`), json(record));
    // Windows can hold a just-exited engine image briefly (seen under ARM64 x64 emulation).
    if (temporary) fs.rmSync(temporary, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = process.argv.slice(2); let releaseDir, target, fallback = false, stage = false;
    for (let i = 0; i < args.length; i++) {
      if (args[i] === '--release-dir' && args[i + 1]) releaseDir = path.resolve(args[++i]);
      else if (args[i] === '--target' && args[i + 1]) target = args[++i];
      else if (args[i] === '--windows-arm-fallback') fallback = true;
      else if (args[i] === '--stage-runtime') stage = true;
      else throw new Error('Unknown smoke argument');
    }
    assert(releaseDir && target, 'Usage: node scripts/smoke-phoenix-ui.mjs --release-dir DIR --target TARGET [--stage-runtime | --windows-arm-fallback]');
    console.log(json(runSmoke(releaseDir, target, { fallback, stage })));
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
