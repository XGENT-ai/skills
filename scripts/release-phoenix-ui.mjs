#!/usr/bin/env node
// Release @xgent-ai/skills from a clean checkout whose HEAD passed the Phoenix CI:
// take the UI and Diagram engines built by one CI run, upload all eight to R2, pack the npm tarball once,
// install-check it from the public URLs, then `npm publish` that same tarball and verify the registry.
// Records stay under local/phoenix-ui/releases/v<version>-<sha>/<ci-RUN|reuse-VERSION>/; rerunning resumes from them.
//
//   mise exec --no-deps -- npm run release:phoenix -- [--run-id N] [--node18 PATH] [--otp CODE] [--tag TAG] [--yes] [--prepare-only]
//   mise exec --no-deps -- npm run release:phoenix -- --reuse-engines VERSION [--node18 PATH] [--otp CODE] [--tag TAG] [--yes] [--prepare-only]
//
// --prepare-only stops after the install check; running the same command again publishes.
// --reuse-engines skips CI and R2: the UI and Diagram engines of that published npm version are kept
// when no engine source changed since it, and this checkout's committed skill files are packed around them.
//
// R2 credentials: the AGENT_RELEASE_R2_* variables, read from the environment or from .env.cf.
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import readline from 'node:readline/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { assertReleaseBranch, assertPublishablePackage } from './require-release-package.mjs';
import { sourceInputs as diagramSourceInputs, bundleFiles as diagramBundleFiles } from './build-phoenix-diagram.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const repository = 'XGENT-ai/skills';
const tools = ['phoenix-ui', 'phoenix-diagram'];
const targets = ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64'];
const sidecarFiles = ['THIRD-PARTY.json', 'THIRD-PARTY-NOTICES.md', 'THIRD-PARTY-LICENSES'];
const sha = (bytes, algorithm = 'sha256', encoding = 'hex') => crypto.createHash(algorithm).update(bytes).digest(encoding);
const json = value => JSON.stringify(value, null, 2) + '\n';
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8'));

// `export KEY=VALUE` / `KEY=VALUE` lines; only the R2 release variables are taken.
export function parseEnvFile(text) {
  const values = {};
  for (const line of text.split(/\r?\n/)) {
    const match = /^\s*(?:export\s+)?(AGENT_RELEASE_R2_[A-Z0-9_]+)=(.*?)\s*$/.exec(line);
    if (!match) continue;
    let value = match[2];
    if (/^(['"]).*\1$/.test(value)) value = value.slice(1, -1);
    values[match[1]] = value;
  }
  return values;
}

// Variables already exported win over the file.
export function releaseEnv(env, fileText) {
  const merged = { ...env };
  for (const [key, value] of Object.entries(fileText ? parseEnvFile(fileText) : {})) if (!merged[key]) merged[key] = value;
  return merged;
}

// Newest successful Phoenix run for exactly this commit.
export function selectRun(runs, head) {
  const matching = runs.filter(run => run.headSha === head);
  const success = matching.filter(run => run.status === 'completed' && run.conclusion === 'success')
    .sort((a, b) => b.databaseId - a.databaseId)[0];
  return { success, pending: matching.find(run => run.status !== 'completed'), failed: matching.find(run => run.status === 'completed' && run.conclusion !== 'success') };
}

function parseArgs(argv) {
  const options = {};
  for (let i = 0; i < argv.length; i++) {
    const flag = argv[i];
    if (flag === '--yes') options.yes = true;
    else if (flag === '--prepare-only') options.prepareOnly = true;
    else if (['--run-id', '--node18', '--otp', '--tag', '--reuse-engines'].includes(flag) && argv[i + 1] && !options[flag]) options[flag] = argv[++i];
    else throw new Error(`Unknown or duplicate argument ${flag}`);
  }
  assert(!options['--reuse-engines'] || !options['--run-id'], '--reuse-engines takes both engines from that npm version; drop --run-id');
  return options;
}

function command(program, args, { cwd = root, env = process.env, input, allowFailure = false, inherit = false } = {}) {
  const result = spawnSync(program, args, { cwd, env, input, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024, stdio: inherit ? 'inherit' : 'pipe' });
  if (!allowFailure && (result.error || result.status !== 0)) throw new Error(`${program} ${args.join(' ')} failed: ${result.error?.message || result.stderr || `exit ${result.status}`}`);
  return result;
}
const npmCli = path.resolve(path.dirname(process.execPath), '../lib/node_modules/npm/bin/npm-cli.js');
const npm = (args, extra) => command(process.execPath, [npmCli, ...args], extra);
const gh = (args, extra) => command('gh', args, extra).stdout;
function bytesOf(program, args) {
  const result = spawnSync(program, args, { cwd: root, maxBuffer: 1024 * 1024 * 1024 });
  if (result.error || result.status !== 0) throw new Error(`${program} ${args.join(' ')} failed: ${result.error?.message || result.stderr}`);
  return result.stdout;
}
const step = message => console.log(`\n== ${message}`);

async function confirm(question, expected, options) {
  if (options.yes) return;
  assert(process.stdin.isTTY, `${question}\nNon-interactive run: pass --yes to confirm.`);
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  const answer = (await rl.question(`${question}\nType ${expected} to continue: `)).trim();
  rl.close();
  assert.equal(answer, expected, 'Not confirmed; nothing was changed by this step.');
}

function newestNode18() {
  const base = path.join(os.homedir(), '.nvm/versions/node');
  if (!fs.existsSync(base)) return undefined;
  const key = name => name.slice(1).split('.').map(Number);
  const found = fs.readdirSync(base).filter(name => /^v18\.\d+\.\d+$/.test(name))
    .sort((a, b) => key(a)[1] - key(b)[1] || key(a)[2] - key(b)[2]).pop();
  return found && path.join(base, found, 'bin/node');
}

function registryVersion(name, version) {
  const result = npm(['view', `${name}@${version}`, 'dist', '--json'], { allowFailure: true });
  if (result.status !== 0) { assert(/E404|404/.test(result.stderr), `npm view failed: ${result.stderr}`); return null; }
  return result.stdout.trim() ? JSON.parse(result.stdout) : null;
}

function tarballDigests(file) {
  const bytes = fs.readFileSync(file);
  return { sha256: sha(bytes), shasum: sha(bytes, 'sha1'), integrity: `sha512-${sha(bytes, 'sha512', 'base64')}` };
}

const fileSha = (repoRoot, name) => { const file = path.join(repoRoot, name); return fs.existsSync(file) ? sha(fs.readFileSync(file)) : null; };
function filesUnder(repoRoot, directory) {
  return fs.readdirSync(path.join(repoRoot, directory), { withFileTypes: true }).flatMap(entry => {
    const name = `${directory}/${entry.name}`;
    return entry.isDirectory() ? filesUnder(repoRoot, name) : entry.name === '.DS_Store' ? [] : [name];
  });
}

// Paths compiled into the UI engine: its Rust workspace (minus the skill renderer) plus every
// non-test include_str!/include_bytes! target outside it, such as skills/phoenix-ui/src/scripts.
export function uiEngineSources(repoRoot = root) {
  const embedded = new Set();
  for (const file of filesUnder(repoRoot, 'tools/phoenix-ui/crates')) {
    if (!file.endsWith('.rs') || file.split('/').includes('tests')) continue;
    for (const match of fs.readFileSync(path.join(repoRoot, file), 'utf8').matchAll(/include_(?:str|bytes)!\(\s*"([^"]+)"/g)) {
      const target = path.posix.normalize(path.posix.join(path.posix.dirname(file), match[1]));
      if (!target.startsWith('tools/phoenix-ui/')) embedded.add(target);
    }
  }
  return ['tools/phoenix-ui', ':(exclude)tools/phoenix-ui/scripts/render.mjs', ':(exclude)tools/phoenix-ui/scripts/lib', 'rust-toolchain.toml', ...[...embedded].sort()];
}

// Both engines must be exactly what the previous release shipped. Diagram embeds its source-input
// digest, so compare that; the UI engine has no such digest, so compare its sources in git.
export function assertEnginesReusable(previous, repoRoot = root) {
  const commit = previous.ui.sourceCommit;
  assert(spawnSync('git', ['cat-file', '-e', `${commit}^{commit}`], { cwd: repoRoot }).status === 0, `Commit ${commit} of the reused UI engine is not in this clone; git fetch first`);
  const changed = command('git', ['diff', '--name-only', commit, 'HEAD', '--', ...uiEngineSources(repoRoot)], { cwd: repoRoot }).stdout.trim();
  assert(!changed, `UI engine sources changed since ${previous.version}; build new engines through CI instead:\n${changed}`);
  assert.equal(diagramSourceInputs(repoRoot).sha256, previous.diagram.sourceInputSha256, `Diagram engine sources changed since ${previous.version}; build new engines through CI instead`);
}

// The packed skills come straight from the checkout, so their generated files must match their sources.
export function assertGeneratedCurrent(repoRoot = root) {
  const generated = readJson(path.join(repoRoot, 'skills/phoenix-ui/GENERATED.json')), stale = [];
  for (const name of filesUnder(repoRoot, 'skills/phoenix-ui/src')) if (!(name in generated.inputs)) stale.push(name);
  for (const [name, hash] of Object.entries(generated.inputs)) if (fileSha(repoRoot, name) !== hash) stale.push(name);
  for (const [section, prefix] of [['skill', 'skills/phoenix-ui/'], ['bundle', 'dist/phoenix-ui/bundle/'], ['notices', 'dist/phoenix-ui/']]) {
    for (const [name, hash] of Object.entries(generated.outputs[section])) {
      if (prefix + name !== 'skills/phoenix-ui/scripts/ENGINE.json' && fileSha(repoRoot, prefix + name) !== hash) stale.push(prefix + name);
    }
  }
  if (generated.bundleSha256 !== fileSha(repoRoot, 'dist/phoenix-ui/bundle/manifest.json')) stale.push('dist/phoenix-ui/bundle/manifest.json');
  if (diagramBundleFiles(repoRoot).sha256 !== fileSha(repoRoot, 'dist/phoenix-diagram/bundle/manifest.json')) stale.push('dist/phoenix-diagram/bundle/manifest.json');
  assert(!stale.length, `Generated skill files are stale; regenerate and commit them first:\n${[...new Set(stale)].join('\n')}`);
}

// Engine fields stay as built; package identity and bundle digest follow this checkout.
export function releaseManifest(tool, base, engines, { version, head, tree }) {
  return { ...base, distribution: 'release', npmPackageVersion: version, sourceCommit: head, ...(tool === 'phoenix-ui' && { sourceTree: tree }),
    bundleSha256: fileSha(root, `dist/${tool}/bundle/manifest.json`), engines };
}

// GENERATED.json records the digest of the derived UI ENGINE.json, which the release manifest replaces.
function engineDigests(generated, hash) {
  generated.outputs.skill['scripts/ENGINE.json'] = hash;
  for (const name of Object.keys(generated.outputs.providers)) if (name.endsWith('/skills/phoenix-ui/scripts/ENGINE.json')) generated.outputs.providers[name] = hash;
  return generated;
}

// Stage HEAD, write both release manifests and their derived copies, take the engines' third-party
// notices from `sidecars[tool]`, check the staged package is publishable and pack it once.
function packRelease(packDir, manifests, sidecars, provenance) {
  const stage = path.join(packDir, 'package'), tarballs = path.join(packDir, 'tarballs');
  fs.rmSync(packDir, { recursive: true, force: true });
  fs.mkdirSync(stage, { recursive: true }); fs.mkdirSync(tarballs);
  command('tar', ['-xf', '-', '-C', stage], { input: bytesOf('git', ['archive', '--format=tar', 'HEAD']) });
  const ui = json(manifests['phoenix-ui']), diagram = json(manifests['phoenix-diagram']);
  for (const name of ['dist/phoenix-ui/VERSION.json', 'skills/phoenix-ui/scripts/ENGINE.json']) fs.writeFileSync(path.join(stage, name), ui);
  const generatedFile = path.join(stage, 'skills/phoenix-ui/GENERATED.json');
  fs.writeFileSync(generatedFile, json(engineDigests(readJson(generatedFile), sha(ui))));
  for (const name of ['dist/phoenix-diagram/VERSION.json', 'dist/phoenix-diagram/bundle/ENGINE.json', 'skills/phoenix-diagram/ENGINE.json']) fs.writeFileSync(path.join(stage, name), diagram);
  for (const tool of tools) for (const name of sidecarFiles) {
    fs.rmSync(path.join(stage, 'dist', tool, name), { recursive: true, force: true });
    fs.cpSync(path.join(sidecars[tool], name), path.join(stage, 'dist', tool, name), { recursive: true });
  }
  assertPublishablePackage(stage);
  const [packed] = JSON.parse(npm(['pack', '--json', '--pack-destination', tarballs], { cwd: stage }).stdout);
  fs.writeFileSync(path.join(packDir, 'VERSION.json'), ui);
  fs.writeFileSync(path.join(packDir, 'pack-record.json'), json({ provenance, tarball: packed.filename }));
}

// <prefix>/<tool>/v<version>/ci-<run>/<target>/<asset>: redoing a failed attempt with a new CI run
// never meets the immutable objects of the earlier one.
export function engineKey(prefix, tool, version, runId, target, asset) {
  return [prefix, tool, `v${version}`, `ci-${runId}`, target, asset].filter(Boolean).join('/');
}

function r2Config(env) {
  const need = name => { assert(env[name], `Missing ${name}; export it or add it to .env.cf`); return env[name]; };
  const network = { ...env };
  // curl and the aws CLI read https_proxy for HTTPS URLs; let a lone HTTP_PROXY cover them too.
  if (!(env.https_proxy || env.HTTPS_PROXY || env.all_proxy || env.ALL_PROXY) && (env.http_proxy || env.HTTP_PROXY)) network.https_proxy = env.http_proxy || env.HTTP_PROXY;
  const awsEnv = { ...network, AWS_ACCESS_KEY_ID: need('AGENT_RELEASE_R2_ACCESS_KEY'), AWS_SECRET_ACCESS_KEY: need('AGENT_RELEASE_R2_SECRET_KEY'),
    AWS_DEFAULT_REGION: 'auto', AWS_REQUEST_CHECKSUM_CALCULATION: 'when_required', AWS_PAGER: '' };
  delete awsEnv.AWS_SESSION_TOKEN;
  return { bucket: need('AGENT_RELEASE_R2_BUCKET'), endpoint: need('AGENT_RELEASE_R2_ENDPOINT'), publicBase: need('AGENT_RELEASE_R2_PUBLIC_BASE').replace(/\/+$/, ''),
    prefix: (env.AGENT_RELEASE_R2_PREFIX || '').replace(/^\/+|\/+$/g, ''), awsEnv, curlEnv: network };
}

const sameBytes = (file, engine) => { const bytes = fs.readFileSync(file); return bytes.length === engine.size && sha(bytes) === engine.sha256; };

// Objects are never replaced (--if-none-match); one left by an earlier try must hold the same bytes.
export function uploadEngine(file, engine, key, config, scratch) {
  const aws = args => spawnSync('aws', ['s3api', ...args], { env: config.awsEnv, encoding: 'utf8' });
  const object = ['--bucket', config.bucket, '--key', key, '--endpoint-url', config.endpoint];
  const put = aws(['put-object', ...object, '--body', file, '--content-type', 'application/octet-stream', '--if-none-match', '*']);
  if (put.status === 0) return 'uploaded';
  assert(/PreconditionFailed|ConditionalRequestConflict/.test(put.stderr), `Upload of ${key} failed: ${put.error?.message || put.stderr.trim()}`);
  const existing = path.join(scratch, 'existing-object');
  const get = aws(['get-object', ...object, existing]);
  assert.equal(get.status, 0, `Reading existing ${key} failed: ${get.error?.message || get.stderr.trim()}`);
  assert(sameBytes(existing, engine), `${key} already exists with different bytes; release from a new CI run`);
  return 'already uploaded';
}

function readBack(url, engine, config, scratch) {
  const file = path.join(scratch, 'public-readback');
  command('curl', ['-fsSL', '--retry', '3', '--retry-delay', '2', '--max-time', '600', '--output', file, url], { env: config.curlEnv });
  assert(sameBytes(file, engine), `Public ${url} differs from the CI engine`);
}

// Download every per-platform engine artifact of the run, check it against GitHub's digest and unpack it.
function collectCi(runId, head, dir) {
  const api = `repos/${repository}/actions`;
  const run = JSON.parse(gh(['api', `${api}/runs/${runId}`]));
  assert(run.head_sha === head && run.path === '.github/workflows/phoenix.yml' && run.status === 'completed' && run.conclusion === 'success',
    `CI run ${runId} is not a successful Phoenix run of this commit`);
  const { artifacts } = JSON.parse(gh(['api', `${api}/runs/${runId}/artifacts?per_page=100`]));
  for (const tool of tools) for (const target of targets) {
    const name = `${tool}-${target}`, artifact = artifacts.find(item => item.name === name);
    assert(artifact && !artifact.expired, `CI run ${runId} has no unexpired artifact ${name}`);
    const zip = path.join(dir, `${name}.zip`), digest = artifact.digest.replace(/^sha256:/, '');
    if (!fs.existsSync(zip) || sha(fs.readFileSync(zip)) !== digest) fs.writeFileSync(zip, bytesOf('gh', ['api', `${api}/artifacts/${artifact.id}/zip`]));
    assert.equal(sha(fs.readFileSync(zip)), digest, `Downloaded ${name} differs from its GitHub digest`);
    fs.rmSync(path.join(dir, name), { recursive: true, force: true });
    command('unzip', ['-q', zip, '-d', path.join(dir, name)]);
  }
}

async function packFromCi(runId, options, pkg, head, tree, dir) {
  step(`CI run ${runId}`);
  const ci = path.join(dir, 'artifacts');
  fs.mkdirSync(ci, { recursive: true });
  collectCi(runId, head, ci);
  assertGeneratedCurrent();
  const built = Object.fromEntries(tools.map(tool => [tool, Object.fromEntries(targets.map(target => [target, readJson(path.join(ci, `${tool}-${target}`, 'VERSION.json'))]))]));
  for (const tool of tools) {
    assert.equal(built[tool][targets[0]].bundleSha256, fileSha(root, `dist/${tool}/bundle/manifest.json`), `CI built ${tool} against another bundle than this commit's; regenerate and commit it`);
  }
  console.log('eight engine artifacts match their GitHub digests');

  step('R2 engines');
  await confirm(`Upload the eight engines of CI run ${runId} to R2 for v${pkg.version}? Uploaded objects cannot be replaced.`, pkg.version, options);
  const envFile = path.join(root, '.env.cf');
  const config = r2Config(releaseEnv(process.env, fs.existsSync(envFile) ? fs.readFileSync(envFile, 'utf8') : ''));
  const manifests = {}, sidecars = {};
  for (const tool of tools) {
    const engines = {};
    for (const target of targets) {
      const engine = built[tool][target].engines[target], key = engineKey(config.prefix, tool, pkg.version, runId, target, engine.asset);
      const url = `${config.publicBase}/${key}`, status = uploadEngine(path.join(ci, `${tool}-${target}`, engine.asset), engine, key, config, dir);
      readBack(url, engine, config, dir);
      console.log(`${url} (${status}, read back)`);
      engines[target] = { ...engine, url };
    }
    manifests[tool] = releaseManifest(tool, built[tool][targets[0]], engines, { version: pkg.version, head, tree });
    sidecars[tool] = path.join(ci, `${tool}-${targets[0]}`);
  }

  step('Pack');
  packRelease(path.join(dir, 'pack'), manifests, sidecars, { runId });
}

async function packReusingEngines(version, pkg, head, tree, dir) {
  step(`Reuse engines of ${pkg.name}@${version}`);
  const archive = path.join(dir, `${version}.tgz`), extracted = path.join(dir, 'package');
  if (!fs.existsSync(extracted)) {
    const registry = registryVersion(pkg.name, version);
    assert(registry, `${pkg.name}@${version} is not on npm`);
    const download = await fetch(registry.tarball, { signal: AbortSignal.timeout(300000) });
    assert(download.ok, `Download ${registry.tarball} failed: HTTP ${download.status}`);
    const bytes = Buffer.from(await download.arrayBuffer());
    assert.equal(`sha512-${sha(bytes, 'sha512', 'base64')}`, registry.integrity, `${version} tarball differs from its registry integrity`);
    fs.writeFileSync(archive, bytes);
    command('tar', ['-xzf', archive, '-C', dir]);
  }
  const previous = { version, ui: readJson(path.join(extracted, 'dist/phoenix-ui/VERSION.json')), diagram: readJson(path.join(extracted, 'dist/phoenix-diagram/VERSION.json')) };
  assert(previous.ui.distribution === 'release' && previous.diagram.distribution === 'release', `${version} does not carry release engines`);
  assertEnginesReusable(previous);
  assertGeneratedCurrent();
  for (const tool of ['ui', 'diagram']) for (const engine of Object.values(previous[tool].engines)) console.log(engine.url);

  step('Pack');
  const identity = { version: pkg.version, head, tree };
  const manifests = { 'phoenix-ui': releaseManifest('phoenix-ui', previous.ui, previous.ui.engines, identity),
    'phoenix-diagram': releaseManifest('phoenix-diagram', previous.diagram, previous.diagram.engines, identity) };
  // Third-party notices describe the reused engines' dependencies.
  const sidecars = Object.fromEntries(tools.map(tool => [tool, path.join(extracted, 'dist', tool)]));
  packRelease(path.join(dir, 'pack'), manifests, sidecars, { reusedEngines: { version, uiSourceCommit: previous.ui.sourceCommit, diagramSourceCommit: previous.diagram.sourceCommit } });
}

function selectRunId(options, head) {
  if (options['--run-id']) return options['--run-id'];
  const runs = JSON.parse(gh(['run', 'list', '--repo', repository, '--workflow', 'phoenix.yml', '--limit', '50', '--json', 'databaseId,headSha,status,conclusion']));
  const selected = selectRun(runs, head);
  assert(!(!selected.success && selected.pending), `CI run ${selected.pending?.databaseId} for this commit is still running; rerun when it succeeds`);
  assert(selected.success, selected.failed ? `CI run ${selected.failed.databaseId} for this commit failed` : 'No Phoenix CI run for this commit; push it first');
  return String(selected.success.databaseId);
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const lockedTools = readJson(path.join(root, 'tools/phoenix-ui/build-tools.lock.json'));
  assert.equal(process.version, `v${lockedTools.node}`, `Run with the pinned maintenance Node ${lockedTools.node}: mise exec --no-deps -- npm run release:phoenix`);
  step('Checkout');
  assertReleaseBranch(command('git', ['branch', '--show-current']).stdout.trim());
  assert.equal(command('git', ['status', '--porcelain']).stdout, '', 'Commit or stash changes first; the release is built from HEAD only');
  const head = command('git', ['rev-parse', 'HEAD']).stdout.trim(), tree = command('git', ['rev-parse', 'HEAD^{tree}']).stdout.trim();
  const pkg = readJson(path.join(root, 'package.json'));
  assert(!pkg.version.includes('-') || options['--tag'], `${pkg.version} is a prerelease; pass --tag (for example --tag next)`);
  const reuse = options['--reuse-engines'], runId = reuse ? undefined : selectRunId(options, head);
  const dir = path.join(root, 'local/phoenix-ui/releases', `v${pkg.version}-${head.slice(0, 12)}`, reuse ? `reuse-${reuse}` : `ci-${runId}`);
  fs.mkdirSync(dir, { recursive: true });
  console.log(`${pkg.name}@${pkg.version} from ${head}${reuse ? `, engines of ${reuse}` : `, CI run ${runId}`}\nRelease records: ${path.relative(root, dir)}`);

  const packRecordFile = path.join(dir, 'pack/pack-record.json');
  const published = registryVersion(pkg.name, pkg.version);
  if (published) {
    const local = fs.existsSync(packRecordFile) && tarballDigests(path.join(dir, 'pack/tarballs', readJson(packRecordFile).tarball));
    assert(local && local.shasum === published.shasum, `${pkg.name}@${pkg.version} is already on npm and is not this release; bump the version`);
  }
  // A pack record is written only after every engine URL was read back, so a rerun starts from it.
  if (!fs.existsSync(packRecordFile)) await (reuse ? packReusingEngines(reuse, pkg, head, tree, dir) : packFromCi(runId, options, pkg, head, tree, dir));
  const { provenance, tarball: packed } = readJson(packRecordFile);
  const tarball = path.join(dir, 'pack/tarballs', packed), manifestFile = path.join(dir, 'pack/VERSION.json');
  const manifestSha256 = sha(fs.readFileSync(manifestFile)), digests = tarballDigests(tarball);
  console.log(`${tarball}\nsha256 ${digests.sha256}`);

  step('Install check from the public URLs');
  const passedFile = path.join(dir, 'check-passed.json');
  if (!(fs.existsSync(passedFile) && readJson(passedFile).tarballSha256 === digests.sha256)) {
    const node18 = options['--node18'] || process.env.PHOENIX_RELEASE_NODE18 || newestNode18();
    assert(node18 && fs.existsSync(node18), 'Node 18 not found; pass --node18 PATH');
    const out = path.join(dir, `check-${Date.now()}`);
    const result = command(process.execPath, [path.join(root, 'scripts/check-phoenix-ui-release.mjs'), '--tarball', tarball, '--tarball-sha256', digests.sha256,
      '--manifest', manifestFile, '--manifest-sha256', manifestSha256, '--node18', node18, '--node25', process.execPath, '--output-dir', out,
      '--legacy-dir', path.join(root, 'local/phoenix-ui')], { allowFailure: true });
    const resultsFile = path.join(out, 'results.json'), results = fs.existsSync(resultsFile) ? readJson(resultsFile) : {};
    const failures = [results.error, ...(results.groups || []).filter(g => g.error).map(g => `${g.node.version} ${g.kind}: ${g.error.trim()}`)].filter(Boolean);
    assert(result.status === 0 && results.status === 'release-check-passed', `Install check failed (${resultsFile}):\n${failures.join('\n') || (result.stderr || '').slice(-2000)}`);
    fs.writeFileSync(passedFile, json({ tarballSha256: digests.sha256, results: path.relative(dir, out), groups: results.groups.map(g => `${g.node.version} ${g.kind}`) }));
  }
  console.log(readJson(passedFile).groups.join('\n'));
  if (options.prepareOnly && !registryVersion(pkg.name, pkg.version)) {
    console.log(`\nPrepared ${pkg.name}@${pkg.version}. Publish with: mise exec --no-deps -- npm run release:phoenix -- ${reuse ? `--reuse-engines ${reuse}` : `--run-id ${runId}`}`);
    return;
  }

  step('npm publish');
  if (!registryVersion(pkg.name, pkg.version)) {
    await confirm(`Publish ${pkg.name}@${pkg.version}${options['--tag'] ? ` with tag ${options['--tag']}` : ' as latest'} from ${tarball}?`, pkg.version, options);
    const args = ['publish', tarball, ...(options['--tag'] ? ['--tag', options['--tag']] : []), ...(options['--otp'] ? ['--otp', options['--otp']] : [])];
    command(process.execPath, [npmCli, ...args], { inherit: true });
  }
  let registry;
  // npm can take several minutes to expose a new version (0.7.1 took about six).
  for (let attempt = 0; attempt < 60 && !registry; attempt++) {
    registry = registryVersion(pkg.name, pkg.version);
    if (!registry) await new Promise(resolve => setTimeout(resolve, 10000));
  }
  assert(registry, 'Published version is not visible on the registry yet; rerun to verify');
  assert.equal(registry.shasum, digests.shasum, 'Registry shasum differs from the released tarball');
  assert.equal(registry.integrity, digests.integrity, 'Registry integrity differs from the released tarball');
  const download = await fetch(registry.tarball, { signal: AbortSignal.timeout(300000) });
  assert(download.ok, `Download ${registry.tarball} failed: HTTP ${download.status}`);
  assert.equal(sha(Buffer.from(await download.arrayBuffer())), digests.sha256, 'Registry tarball bytes differ');
  fs.writeFileSync(path.join(dir, 'published.json'), json({ name: pkg.name, version: pkg.version, ...provenance, sourceCommit: head, sourceTree: tree, manifestSha256, ...digests, registryTarball: registry.tarball, verifiedAt: new Date().toISOString() }));
  console.log(`\n${pkg.name}@${pkg.version} published and verified (sha256 ${digests.sha256})`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(error => { console.error(`\nRelease stopped: ${error.message}`); process.exitCode = 1; });
}
