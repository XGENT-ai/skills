'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { createHash } = require('node:crypto');
const root = path.resolve(__dirname, '../..');
const prepare = import('../../scripts/prepare-phoenix-diagram.mjs');
const source = import('../../scripts/check-phoenix-diagram-source.mjs');
const assets = JSON.parse(fs.readFileSync(path.join(root, 'tools/phoenix-ui/build-tool-assets.json')));
const workflow = () => fs.readFileSync(path.join(root, '.github/workflows/phoenix-diagram.yml'), 'utf8');
const job = (text, name) => new RegExp(`^  ${name}:\\n([\\s\\S]*?)(?=^  [a-z][a-z0-9-]*:|$(?![\\s\\S]))`, 'm').exec(text)?.[1] || '';
function temporary(t) { const directory = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'diagram workflow ')); t.after(() => fs.rmSync(directory, { recursive: true, force: true })); return directory; }

test('workflow uses four native runners and only the already fixed UI action commits', () => {
  const text = workflow(), native = job(text, 'native'), consume = job(text, 'consume');
  for (const name of ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64']) {
    const pin = assets.platforms[name];
    assert.match(native, new RegExp(`platform: ${name}\\s+runner: ${pin.runner}\\s+rust-target: ${pin.rustTarget}`));
    assert.match(consume, new RegExp(`platform: ${name}\\s+runner: ${pin.runner}`));
  }
  for (const match of text.matchAll(/uses:\s*([^@\s]+)@([^\s]+)/g)) assert.equal(match[2], assets.actions[match[1]]?.commit, match[0]);
  assert.match(text, /contents: read/); assert.doesNotMatch(text, /contents: write|npm publish|gh release|workflow_dispatch:[^\n]*\n[ \t]+inputs:/);
});

test('native builds preserve clean source and native permissions before the single candidate assembly', () => {
  const text = workflow(), native = job(text, 'native'), assemble = job(text, 'assemble');
  assert.match(native, /prepare-phoenix-diagram\.mjs --prepare/);
  assert.match(native, /build-phoenix-diagram\.mjs --development --output-dir/);
  assert.match(native, /mbx test --manifest-path tools\/phoenix-diagram\/Cargo\.toml --locked --offline/);
  assert.match(native, /PHOENIX_DIAGRAM_BUILD_TARGET: \$\{\{ matrix\.rust-target \}\}/);
  assert.match(native, /git status --porcelain/); assert.match(native, /tar .*-[^\n]*c[^\n]*p/);
  assert.match(assemble, /needs: native/); assert.match(assemble, /tar .*-[^\n]*x[^\n]*p/);
  assert.match(assemble, /verifyFrozenSources\(\)/);
  assert.equal((assemble.match(/pack-phoenix-diagram-candidate\.mjs --artifacts-dir/g) || []).length, 1);
  assert.match(assemble, /--output-dir local\/phoenix-diagram\/candidate/);
  assert.doesNotMatch(text, /prepare-phoenix-ui|build-phoenix-ui|test:phoenix|npm pack/);
  assert.doesNotMatch(assemble, /writeFileSync\([^\n]*(?:ENGINE|VERSION|source-inputs)/);
});

test('every Linux runner prepares the narrow unshare profile and fails early on the actual offline namespace command', () => {
  const text=workflow(),native=job(text,'native'),consume=job(text,'consume'),floor=job(text,'node18-consumer');
  const step=/      - &linux_netns\n([\s\S]*?)(?=\n      - )/.exec(native)?.[1];
  assert.ok(step,'One reusable Linux namespace preparation step is required');
  assert.equal((text.match(/&linux_netns\b/g)||[]).length,1);
  assert.equal((text.match(/- \*linux_netns\b/g)||[]).length,2);
  assert.match(step,/if: runner\.os == 'Linux'/);
  assert.match(step,/set -euo pipefail/);
  assert.match(step,/profile phoenix-diagram-unshare "\/usr\/bin\/unshare" flags=\(unconfined\)\s*\{\s*userns,\s*\}/);
  const load=step.indexOf('sudo apparmor_parser --replace "$profile"'),smoke=step.indexOf('/usr/bin/unshare --user --map-current-user --net -- /usr/bin/true');
  assert.ok(load>=0&&smoke>load,'Load the application profile before running the unchanged namespace command');
  assert.match(step,/\/usr\/bin\/unshare --user --map-current-user --net -- \/bin\/sh/);
  for(const diagnostic of ['id -u','/proc/self/ns/user','/proc/self/ns/net','/proc/self/uid_map'])assert.ok(step.includes(diagnostic),diagnostic);
  assert.ok(native.indexOf('&linux_netns')<native.indexOf('name: Explicit online native and browser preparation'));
  for(const consumer of [consume,floor])assert.ok(consumer.indexOf('- *linux_netns')>0&&consumer.indexOf('- *linux_netns')<consumer.indexOf('uses: actions/download-artifact'),'Each new Linux consumer must load the profile before downloading or installing');
  assert.doesNotMatch(step,/continue-on-error|\|\|\s*true|sudo\s+\/usr\/bin\/unshare/);
  assert.doesNotMatch(text,/sysctl|apparmor_restrict_unprivileged_userns\s*=\s*0|--no-sandbox/);
  const script=/        run: \|\n([\s\S]*)/.exec(step)?.[1].replace(/^ {10}/gm,'');
  assert.ok(script,'Extract the actual namespace preparation shell fragment');
  const syntax=spawnSync('/bin/bash',['-n'],{input:script,encoding:'utf8'});
  assert.equal(syntax.status,0,syntax.stderr);
});

test('assembly identity step uses the actual packer size/hash contract and freezes the candidate envelope bytes', t => {
  const directory = temporary(t), text = job(workflow(), 'assemble');
  const step = /- name: Freeze the candidate envelope and actual identities([\s\S]*?)(?=\n      - name: Upload)/.exec(text)?.[1];
  const script = /node --input-type=module <<'NODE'\n([\s\S]*?)\n          NODE/.exec(step || '')?.[1];
  assert.ok(script, 'Executable workflow identity step');
  const hash = bytes => createHash('sha256').update(bytes).digest('hex');
  const bytes = fs.readFileSync(path.join(root, 'LICENSE')), commit = 'a'.repeat(40), candidate = path.join(directory, 'local/phoenix-diagram/candidate');
  fs.mkdirSync(candidate, { recursive: true });
  // Identity-step byte fixture only; never consumed as a native/package proof.
  fs.writeFileSync(path.join(candidate, 'fixture.tgz'), bytes);
  fs.writeFileSync(path.join(candidate, 'candidate.json'), JSON.stringify({ status: 'assembled', source: { commit }, tarball: { path: 'fixture.tgz', size: bytes.length, sha256: hash(bytes) } }));
  fs.writeFileSync(path.join(directory, 'phoenix-diagram-candidate.tar'), bytes);
  const harness = fs.readFileSync(path.join(root, 'tools/phoenix-ui/LICENSE'));
  fs.writeFileSync(path.join(directory, 'phoenix-diagram-harness.tar'), harness);
  const output = path.join(directory, 'github-output');
  const run = () => spawnSync(process.execPath, ['--input-type=module'], { input: script, cwd: directory, encoding: 'utf8', env: { ...process.env, GITHUB_SHA: commit, RUNNER_TEMP: directory, GITHUB_OUTPUT: output } });
  const result = run(); assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.readFileSync(output, 'utf8'), `candidate-sha256=${hash(bytes)}\ntgz-sha256=${hash(bytes)}\nsource-commit=${commit}\nharness-sha256=${hash(harness)}\n`);
  fs.appendFileSync(path.join(candidate, 'fixture.tgz'), '\n'); assert.notEqual(run().status, 0);
});

test('all consumers use one digest-bound candidate, with a separate Node 18 floor and no Rust preparation', () => {
  const text = workflow(), assemble = job(text, 'assemble');
  for (const name of ['consume', 'node18-consumer']) {
    const consumer = job(text, name);
    assert.match(consumer, /needs: assemble/);
    assert.match(consumer, /name: phoenix-diagram-candidate/);
    assert.match(consumer, /PHOENIX_DIAGRAM_CANDIDATE_SHA256: \$\{\{ needs\.assemble\.outputs\.candidate-sha256 \}\}/);
    assert.match(consumer, /PHOENIX_DIAGRAM_TGZ_SHA256: \$\{\{ needs\.assemble\.outputs\.tgz-sha256 \}\}/);
    assert.match(consumer, /name: phoenix-diagram-harness/);
    assert.match(consumer, /PHOENIX_DIAGRAM_HARNESS_SHA256: \$\{\{ needs\.assemble\.outputs\.harness-sha256 \}\}/);
    assert.match(consumer, /node local\/phoenix-diagram\/harness\/scripts\/prepare-phoenix-diagram\.mjs --prepare-browser/);
    assert.match(consumer, /node local\/phoenix-diagram\/harness\/scripts\/check-phoenix-diagram-candidate\.mjs --candidate-dir local\/phoenix-diagram\/download\/candidate --target/);
    assert.doesNotMatch(consumer, /actions\/checkout|sandbox-exec|deny file-read|--no-sandbox/);
    assert.doesNotMatch(consumer, /mise-action|mbx|rustup|cargo |--prepare\s*\n|npm install|npm exec/);
  }
  assert.match(job(text, 'node18-consumer'), /node-version: '18\.20\.8'/);
  for (const output of ['candidate-sha256', 'tgz-sha256', 'source-commit', 'artifact-id', 'artifact-digest', 'harness-sha256', 'harness-artifact-id', 'harness-artifact-digest']) assert.ok(assemble.includes(output), output);
});

test('workflow phase evidence records actual outcomes without inventing provider or image review acceptance', () => {
  const text = workflow();
  assert.match(text, /steps\.prepare\.outcome/); assert.match(text, /steps\.build\.outcome/); assert.match(text, /steps\.consume\.outcome/);
  assert.match(text, /GITHUB_RUN_ID/); assert.match(text, /GITHUB_SHA/);
  assert.match(text, /providerSessionAcceptance: 'not-run'/);
  assert.match(text, /visualReview: 'required-pending'/);
  assert.doesNotMatch(text, /(?:providerSessionAcceptance|visualReview): ['"](?:pass|passed|complete)['"]/);
  assert.match(text, /apparmor_parser --replace/); assert.doesNotMatch(text, /--no-sandbox/);
});

test('visual review resumes the original consumer state in the same running job after publishing review requests', () => {
  const consume = job(workflow(), 'consume'), node18 = job(workflow(), 'node18-consumer');
  const material = consume.indexOf('id: review_material');
  const request = consume.indexOf('name: Upload the bound screenshots for actual image review');
  const resume = consume.indexOf('name: Resume the same delivery after actual image review');
  assert.ok(material > consume.indexOf('id: consume') && request > material && resume > request);
  assert.match(consume, /name: phoenix-diagram-review-request-\$\{\{ matrix\.platform \}\}/);
  const upload = /id: review_request\n([\s\S]*?)(?=\n      #)/.exec(consume)?.[1];
  assert.match(upload, /if: steps\.consume\.outcome == 'success' && steps\.review_material\.outcome == 'success'/);
  assert.match(upload, /path: \$\{\{ steps\.review_material\.outputs\.review_paths \}\}/);
  assert.match(consume, /if: steps\.consume\.outcome == 'success' && steps\.review_request\.outcome == 'success'/);
  assert.match(consume, /steps\.consume\.outputs\.state_path/);
  assert.match(consume, /--resume "\$PHOENIX_DIAGRAM_RESUME_STATE" --review-ref "phoenix-diagram-review\/\$GITHUB_RUN_ID"/);
  assert.match(consume, /timeout-minutes: 90/); assert.match(consume, /timeout-minutes: 120/);
  const finalUpload = /name: phoenix-diagram-consume-\$\{\{ matrix\.platform \}\}-evidence([\s\S]*)/.exec(consume)?.[1];
  assert.match(finalUpload, /steps\.consume\.outputs\.evidence_dir/);
  assert.match(finalUpload, /include-hidden-files: true/);
  assert.doesNotMatch(node18, /--resume|--review-ref/);
});

function reviewMaterialScript() {
  const step = /- name: Select the original files for actual image review\n([\s\S]*?)(?=\n      - name:)/.exec(job(workflow(), 'consume'))?.[1];
  assert.ok(step, 'Select bound review files before uploading the request');
  assert.match(step, /id: review_material/); assert.match(step, /if: steps\.consume\.outcome == 'success'/);
  for (const name of ['state_path', 'evidence_dir']) assert.match(step, new RegExp(`steps\\.consume\\.outputs\\.${name}`));
  const script = /node --input-type=module <<'NODE'\n([\s\S]*?)\n          NODE/.exec(step)?.[1];
  assert.ok(script, 'Execute the actual inline selector');
  const shell = /run: \|\n([\s\S]*)/.exec(step)?.[1].replace(/^ {10}/gm, '');
  const syntax = spawnSync('/bin/bash', ['-n'], { input: shell, encoding: 'utf8' });
  assert.equal(syntax.status, 0, syntax.stderr); return script;
}

async function reviewMaterialFixture(t) {
  const directory = temporary(t), runRoot = path.join(directory, 'original evidence'), harness = path.join(directory, 'local/phoenix-diagram/harness/scripts');
  fs.mkdirSync(runRoot, { mode: 0o700 }); fs.mkdirSync(harness, { recursive: true });
  fs.copyFileSync(path.join(root, 'scripts/test-phoenix-diagram-core.mjs'), path.join(harness, 'test-phoenix-diagram-core.mjs'));
  const { readSnapshot, bound } = await import('../../scripts/test-phoenix-diagram-core.mjs');
  const write = (file, value) => { fs.writeFileSync(file, typeof value === 'string' || Buffer.isBuffer(value) ? value : JSON.stringify(value), { mode: 0o600 }); return bound(readSnapshot(file)); };
  // File-selection mechanism fixture only, never native or visual acceptance.
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6vWQAAAAASUVORK5CYII=', 'base64');
  const cases = ['architecture', 'workflow', 'sequence', 'dataflow', 'lifecycle'].flatMap(type => ['new', 'legacy'].map(contract => {
    const id = `${contract}-${type}`, folder = path.join(runRoot, id); fs.mkdirSync(folder);
    return { id, state: { screenshots: [1440, 2048].flatMap(width => ['light', 'dark'].map(theme => ({ ...write(path.join(folder, `${width}-${theme}.png`), png), viewport: { width, height: width === 1440 ? 900 : 1320 }, theme }))) } };
  }));
  const stat = fs.statSync(runRoot), sourceCommit = 'a'.repeat(40), tgzSha = 'b'.repeat(64);
  const state = { schemaVersion: 1, kind: 'phoenix-diagram-candidate-consumer', repository: 'fixture/repository', runId: '12345', runAttempt: '1', job: 'consume', target: 'darwin-arm64', nonce: 'c'.repeat(32), runRoot, directoryBinding: { device: String(stat.dev), inode: String(stat.ino), uid: stat.uid }, source: { commit: sourceCommit }, candidate: { tarball: { identity: { sha256: tgzSha, bytes: 123 } } }, phase: 'pending-review', automatedStatus: 'pass', status: 'automated-pass', cases };
  const stateFile = write(path.join(runRoot, 'consumer-state.json'), state);
  const request = { schemaVersion: 1, kind: 'phoenix-diagram-review-request', state: stateFile, envelope: { schemaVersion: 1, kind: 'phoenix-diagram-candidate-review', repository: state.repository, runId: state.runId, target: state.target, nonce: state.nonce, stateSha256: stateFile.identity.sha256, candidateSha256: tgzSha, sourceCommit, records: Object.fromEntries(cases.map(entry => [entry.id, { screenshotIdentities: entry.state.screenshots.map(file => ({ file: { path: file.path, identity: file.identity }, viewport: file.viewport, theme: file.theme })) }])) } };
  const requestFile = write(path.join(runRoot, 'review-request.json'), request), output = path.join(directory, 'github-output');
  fs.mkdirSync(path.join(runRoot, 'npm-cache')); write(path.join(runRoot, 'npm-cache/omit.tgz'), 'cache');
  fs.mkdirSync(path.join(runRoot, 'exports')); write(path.join(runRoot, 'exports/omit.png'), png);
  const files = [stateFile, requestFile, ...cases.flatMap(entry => entry.state.screenshots.map(file => bound(readSnapshot(file.path))))];
  const env = { ...process.env, PHOENIX_REVIEW_STATE: stateFile.path, PHOENIX_REVIEW_ROOT: runRoot, PHOENIX_REVIEW_TARGET: state.target, GITHUB_REPOSITORY: state.repository, GITHUB_RUN_ID: state.runId, GITHUB_RUN_ATTEMPT: state.runAttempt, GITHUB_JOB: state.job, GITHUB_SHA: sourceCommit, PHOENIX_DIAGRAM_SOURCE_COMMIT: sourceCommit, PHOENIX_DIAGRAM_TGZ_SHA256: tgzSha, GITHUB_OUTPUT: output };
  const run = () => spawnSync(process.execPath, ['--input-type=module'], { input: reviewMaterialScript(), cwd: directory, encoding: 'utf8', env });
  return { run, files, request, requestFile, state, output, env, write, readSnapshot, bound };
}

test('review upload selects exactly the 42 original bound files without caches, exports or rewrites', async t => {
  const fixture = await reviewMaterialFixture(t), result = fixture.run(); assert.equal(result.status, 0, result.stderr);
  const output = fs.readFileSync(fixture.output, 'utf8'), match = /^review_paths<<([^\n]+)\n([\s\S]*)\n\1\n$/.exec(output); assert.ok(match, output);
  const selected = match[2].split('\n'); assert.equal(selected.length, 42); assert.equal(new Set(selected).size, 42);
  assert.deepEqual(selected, fixture.files.map(file => file.path));
  assert.equal(path.dirname(selected[0]), fixture.state.runRoot); assert.equal(path.dirname(selected[1]), fixture.state.runRoot);
  assert.ok(selected.every(file => !path.relative(fixture.state.runRoot, file).startsWith('..')));
  assert.doesNotMatch(selected.join('\n'), /npm-cache|\/exports\/|[*?\[\]]/);
  for (const file of fixture.files) assert.deepEqual(fixture.bound(fixture.readSnapshot(file.path)), file, 'Original file bytes and physical bindings must remain unchanged');
});

test('review file selection rejects changed screenshot bytes and a missing screenshot before producing outputs', async t => {
  for (const failure of ['changed', 'missing']) {
    const fixture = await reviewMaterialFixture(t), file = fixture.files[2].path;
    if (failure === 'changed') fs.appendFileSync(file, 'changed'); else fs.unlinkSync(file);
    assert.notEqual(fixture.run().status, 0, failure); assert.ok(!fs.existsSync(fixture.output), failure);
  }
});

test('review file selection rejects wrong state SHA, nonce, current workflow and screenshot scope', async t => {
  for (const failure of ['state-sha', 'nonce', 'run', 'source', 'tgz', 'duplicate-screenshot', 'missing-case']) {
    const fixture = await reviewMaterialFixture(t);
    if (failure === 'state-sha') fixture.request.state.identity.sha256 = 'd'.repeat(64);
    if (failure === 'nonce') fixture.request.envelope.nonce = 'd'.repeat(32);
    if (failure === 'run') fixture.env.GITHUB_RUN_ID = '999';
    if (failure === 'source') fixture.env.PHOENIX_DIAGRAM_SOURCE_COMMIT = 'd'.repeat(40);
    if (failure === 'tgz') fixture.env.PHOENIX_DIAGRAM_TGZ_SHA256 = 'd'.repeat(64);
    if (failure === 'duplicate-screenshot') fixture.request.envelope.records[fixture.state.cases[0].id].screenshotIdentities[1] = fixture.request.envelope.records[fixture.state.cases[0].id].screenshotIdentities[0];
    if (failure === 'missing-case') delete fixture.request.envelope.records[fixture.state.cases[0].id];
    fs.writeFileSync(fixture.requestFile.path, JSON.stringify(fixture.request));
    assert.notEqual(fixture.run().status, 0, failure); assert.ok(!fs.existsSync(fixture.output), failure);
  }
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
  const preparer = await prepare, versions = { rust: '1.99.0', node: '25.2.1', mrBoxington: '1.21.1' };
  const commands = preparer.preparationCommands(versions, true, { platform: 'darwin-x64', boxingtonAvailable: false });
  const bootstrap = commands.find(([, args]) => args.includes('--git'));
  assert.ok(bootstrap); assert.ok(bootstrap[1].includes(assets.nativeBootstrap['darwin-x64'].revision));
  assert.doesNotMatch(JSON.stringify(commands), /bun|wasm|binaryen/);
});

test('isolated harness imports and browser-only preparation require neither source checkout nor native tooling', t => {
  const directory = temporary(t), assemble = job(workflow(), 'assemble');
  const step = /- name: Archive the isolated maintenance harness([\s\S]*?)(?=\n      - name:)/.exec(assemble)?.[1];
  assert.ok(step, 'Explicit harness allowlist archive');
  const files = [...step.matchAll(/^\s+((?:scripts|skills|tools)\/[a-zA-Z0-9/_.-]+|LICENSE)(?: \\)?$/gm)].map(match => match[1]);
  assert.equal(files.length, 21); assert.equal(new Set(files).size, files.length);
  assert.ok(files.includes('tools/phoenix-diagram/tests/support/capture-stdio.mjs'));
  assert.doesNotMatch(files.join('\n'), /(?:^|\/)(?:src|target|node_modules|upstream|renderers)(?:\/|$)|\.rs$|Cargo\.(?:lock|toml)|\.tar/mi);
  const archive = path.join(temporary(t), 'harness.tar');
  const packed = spawnSync('tar', ['-cpf', archive, ...files], { cwd: root, env: { ...process.env, COPYFILE_DISABLE: '1' }, encoding: 'utf8' });
  assert.equal(packed.status, 0, packed.stderr);
  const extracted = spawnSync('tar', ['-C', directory, '-xpf', archive], { encoding: 'utf8' });
  assert.equal(extracted.status, 0, extracted.stderr);
  assert.ok(!fs.existsSync(path.join(directory, '.git'))); assert.ok(!fs.existsSync(path.join(directory, 'package.json')));
  const help = spawnSync(process.execPath, [path.join(directory, 'scripts/check-phoenix-diagram-candidate.mjs'), '--help'], { cwd: directory, env: { ...process.env, PATH: '' }, encoding: 'utf8' });
  assert.equal(help.status, 0, help.stderr); assert.match(help.stdout, /--candidate-dir/);
  const captureHelp = spawnSync(process.execPath, [path.join(directory, 'tools/phoenix-diagram/tests/support/capture-stdio.mjs'), '--help'], { cwd: directory, env: { ...process.env, PATH: '' }, encoding: 'utf8' });
  assert.equal(captureHelp.status, 0, captureHelp.stderr); assert.match(captureHelp.stdout, /--program.*--stdout.*--stderr/);
  const script = path.join(directory, 'scripts/prepare-phoenix-diagram.mjs');
  for (const args of [['--prepare-browser', '--update-lock'], ['--prepare-browser', '--other'], ['--browser']]) {
    const result = spawnSync(process.execPath, [script, ...args], { env: { ...process.env, PATH: '' }, encoding: 'utf8' });
    assert.equal(result.status, 2); assert.match(result.stderr, /^Usage:/); assert.equal(result.stdout, '');
  }
  const pin = spawnSync(process.execPath, ['--input-type=module', '-e', 'const m=await import(process.argv[1]); process.stdout.write(JSON.stringify(m.browserPin()));', require('node:url').pathToFileURL(path.join(directory, 'scripts/check-phoenix-diagram-source.mjs')).href], { cwd: directory, env: { ...process.env, PATH: '' }, encoding: 'utf8' });
  assert.equal(pin.status, 0, pin.stderr); assert.deepEqual(JSON.parse(pin.stdout), assets.linuxArmBrowser);
  const outside = temporary(t), failed = spawnSync(process.execPath, [script, '--prepare-browser'], { cwd: directory, env: { ...process.env, PATH: '', GITHUB_ENV: '', GITHUB_OUTPUT: '', RUNNER_TEMP: outside, PHOENIX_DIAGRAM_BROWSER: path.join(outside, 'missing-browser') }, encoding: 'utf8' });
  assert.equal(failed.status, 1); assert.match(failed.stderr, /browser\/configured-path/);
  assert.ok(!fs.existsSync(path.join(directory, 'local')), 'Browser preparation must leave the transferred harness unchanged');
  const scratch = fs.readdirSync(outside); assert.equal(scratch.length, 1);
  const evidence = path.join(outside, scratch[0], 'evidence'), receipts = fs.readdirSync(evidence).filter(name => name.endsWith('.json'));
  assert.equal(receipts.length, 1); const receipt = JSON.parse(fs.readFileSync(path.join(evidence, receipts[0])));
  assert.equal(receipt.status, 'failed'); assert.equal(receipt.browserAcceptance, 'not-run'); assert.deepEqual(receipt.commands, []);
});
