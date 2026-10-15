#!/usr/bin/env node
// Release @xgent-ai/skills from a clean checkout whose HEAD passed the Phoenix UI CI:
// download the tested CI artifacts, upload the four engines to R2, pack the npm tarball once,
// install-check it from the public URLs, then `npm publish` that same tarball and verify the registry.
// Every step keeps its output under local/phoenix-ui/releases/; rerunning resumes and re-verifies.
//
//   mise exec --no-deps -- npm run release:phoenix -- [--run-id N] [--node18 PATH] [--otp CODE] [--tag TAG] [--yes] [--prepare-only]
//
// --prepare-only stops after the install check; running the same command again publishes.
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

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const repository = 'XGENT-ai/skills';
const targets = ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64'];
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

// Newest successful Phoenix UI run for exactly this commit.
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
    else if (['--run-id', '--node18', '--otp', '--tag'].includes(flag) && argv[i + 1] && !options[flag]) options[flag] = argv[++i];
    else throw new Error(`Unknown or duplicate argument ${flag}`);
  }
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

function collectCi(runId, head, tree, dir) {
  const api = `repos/${repository}/actions`;
  fs.mkdirSync(path.join(dir, 'archives'), { recursive: true });
  const record = (name, endpoint) => { const raw = gh(['api', endpoint]); fs.writeFileSync(path.join(dir, `${name}.json`), raw); return JSON.parse(raw); };
  const run = record('run', `${api}/runs/${runId}`);
  assert.equal(String(run.id), String(runId)); assert.equal(run.head_sha, head, 'CI run is for a different commit');
  assert(run.status === 'completed' && run.conclusion === 'success', `CI run ${runId} did not succeed`);
  const jobs = record('jobs', `${api}/runs/${runId}/jobs?per_page=100`);
  assert(jobs.total_count === 5 && jobs.jobs.every(job => job.conclusion === 'success'), 'Expected five successful CI jobs');
  const artifacts = record('artifacts', `${api}/runs/${runId}/artifacts?per_page=100`);
  const expected = new Set(['phoenix-ui-node18-consumer-evidence', ...targets.flatMap(target => [`phoenix-ui-${target}`, `phoenix-ui-${target}-evidence`])]);
  assert.equal(artifacts.total_count, 9); assert.deepEqual(new Set(artifacts.artifacts.map(item => item.name)), expected);
  for (const artifact of artifacts.artifacts) {
    assert(!artifact.expired && artifact.workflow_run.head_sha === head, `Artifact ${artifact.name} expired or from another commit`);
    const file = path.join(dir, 'archives', `${artifact.name}.zip`), digest = artifact.digest.replace(/^sha256:/, '');
    if (!fs.existsSync(file) || sha(fs.readFileSync(file)) !== digest) fs.writeFileSync(file, bytesOf('gh', ['api', `${api}/artifacts/${artifact.id}/zip`]));
    assert.equal(sha(fs.readFileSync(file)), digest, `Downloaded ${artifact.name} differs from the GitHub digest`);
  }
  assert.equal(command('git', ['rev-parse', `${head}^{tree}`]).stdout.trim(), tree);
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

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const tools = readJson(path.join(root, 'tools/phoenix-ui/build-tools.lock.json'));
  assert.equal(process.version, `v${tools.node}`, `Run with the pinned maintenance Node ${tools.node}: mise exec --no-deps -- npm run release:phoenix`);
  step('Checkout');
  assert.equal(command('git', ['status', '--porcelain']).stdout, '', 'Commit or stash changes first; the release is built from HEAD only');
  const head = command('git', ['rev-parse', 'HEAD']).stdout.trim(), tree = command('git', ['rev-parse', 'HEAD^{tree}']).stdout.trim();
  const pkg = readJson(path.join(root, 'package.json'));
  const prerelease = pkg.version.includes('-');
  assert(!prerelease || options['--tag'], `${pkg.version} is a prerelease; pass --tag (for example --tag next)`);
  const dir = path.join(root, 'local/phoenix-ui/releases', `v${pkg.version}-${head.slice(0, 12)}`);
  fs.mkdirSync(dir, { recursive: true });
  console.log(`${pkg.name}@${pkg.version} from ${head}\nRelease records: ${path.relative(root, dir)}`);

  const published = registryVersion(pkg.name, pkg.version);
  const packRecordFile = path.join(dir, 'pack/pack-record.json');
  if (published) {
    const packed = fs.existsSync(packRecordFile) && readJson(packRecordFile).tarball;
    const local = packed && tarballDigests(path.join(dir, 'pack/tarballs', packed.file));
    assert(local && local.shasum === published.shasum, `${pkg.name}@${pkg.version} is already on npm and is not this release; bump the version`);
  }

  step('CI run');
  let runId = options['--run-id'];
  if (!runId) {
    const runs = JSON.parse(gh(['run', 'list', '--repo', repository, '--workflow', 'phoenix-ui.yml', '--limit', '50', '--json', 'databaseId,headSha,status,conclusion']));
    const selected = selectRun(runs, head);
    assert(!(!selected.success && selected.pending), `CI run ${selected.pending?.databaseId} for this commit is still running; rerun when it succeeds`);
    assert(selected.success, selected.failed ? `CI run ${selected.failed.databaseId} for this commit failed` : 'No Phoenix UI CI run for this commit; push it first');
    runId = String(selected.success.databaseId);
  }
  console.log(`run ${runId}`);
  const ci = path.join(dir, 'ci');
  collectCi(runId, head, tree, ci);
  const build = path.join(ci, 'canonical-build-record.json');
  if (!fs.existsSync(build)) {
    const zip = path.join(ci, 'archives', `phoenix-ui-${targets[0]}.zip`);
    fs.writeFileSync(build, bytesOf('unzip', ['-p', zip, 'build-record.json']));
  }
  console.log('nine CI artifacts match their GitHub digests');

  step('R2 engines');
  const release = path.join(dir, 'release');
  const frozen = fs.existsSync(path.join(release, 'publication-record.json')) && readJson(path.join(release, 'publication-record.json')).status === 'frozen';
  if (!frozen) {
    await confirm(`Upload the four engines of ${head.slice(0, 12)} to R2 as v${pkg.version}? Uploaded objects cannot be replaced.`, pkg.version, options);
    const envFile = path.join(root, '.env.cf');
    const env = releaseEnv(process.env, fs.existsSync(envFile) ? fs.readFileSync(envFile, 'utf8') : '');
    const { publishRelease } = await import('./publish-phoenix-ui-r2.mjs');
    const result = await publishRelease({ runId, sourceCommit: head, sourceTree: tree, runRecord: path.join(ci, 'run.json'), jobsRecord: path.join(ci, 'jobs.json'),
      artifactsRecord: path.join(ci, 'artifacts.json'), archivesDir: path.join(ci, 'archives'), outputDir: release, resume: fs.existsSync(release) }, env);
    assert.equal(result.status, 'frozen');
  }
  const manifestFile = path.join(release, 'VERSION.json'), manifestSha256 = sha(fs.readFileSync(manifestFile));
  for (const [target, engine] of Object.entries(readJson(manifestFile).engines)) console.log(`${target} ${engine.url}`);

  step('Pack');
  const packArgs = [path.join(root, 'scripts/pack-phoenix-ui-release.mjs'), '--release-dir', release, '--handoff', path.join(release, 'release-handoff.json'),
    '--build-record', build, '--manifest-sha256', manifestSha256, '--run-id', runId, '--source-commit', head, '--source-tree', tree, '--output-dir', path.join(dir, 'pack'), '--execute'];
  command(process.execPath, packArgs);
  const packed = readJson(packRecordFile).tarball, tarball = path.join(dir, 'pack/tarballs', packed.file), digests = tarballDigests(tarball);
  assert.equal(digests.sha256, packed.sha256);
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
    console.log(`\nPrepared ${pkg.name}@${pkg.version}. Publish with: mise exec --no-deps -- npm run release:phoenix`);
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
  fs.writeFileSync(path.join(dir, 'published.json'), json({ name: pkg.name, version: pkg.version, runId, sourceCommit: head, sourceTree: tree, manifestSha256, ...digests, registryTarball: registry.tarball, verifiedAt: new Date().toISOString() }));
  console.log(`\n${pkg.name}@${pkg.version} published and verified (sha256 ${digests.sha256})`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(error => { console.error(`\nRelease stopped: ${error.message}`); process.exitCode = 1; });
}
