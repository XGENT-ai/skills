/**
 * Oracle harness: runs every case of the corpus against the phoenix-ui
 * binary at $PHOENIX_UI_BIN (`<bin> <verb> ...args`) and captures the
 * observable behavior (stdout, stderr, exit code, files written).
 *
 * A case is { id, verb, args, cwd?, stdin?, env?, files?, workspace?, offline? }:
 *   - workspace: name of a dir under tests/oracle/workspaces to copy into a
 *     temp dir and use as cwd (so writes never touch the repo)
 *   - cwd: subpath inside the staged workspace (default '.')
 *   - files: globs (relative to staged workspace) to snapshot after the run
 *   - args may contain <WS> and <REPO> placeholders
 *   - setup(ws): stages extra state after the workspace copy
 *   - isolateHome: false keeps the real $HOME (default: an empty one in <WS>)
 *   - normalize, platforms: see applyCaseNormalizers() and caseRunsHere()
 *   - offline: on macOS, run the verb without network access, so a static
 *     fixture cannot discover a dev server another project left listening
 *   - steps: multi-step cases share one staged workspace; a step may carry
 *     its own setup(ws) (run right before that step) and may set
 *     `daemon: true` to spawn its verb detached (see runDaemonStep) so later
 *     steps run against a live process; the daemon is killed after the last
 *     step and its captured output lands in the golden as `daemon`.
 *
 * Normalization replaces the staged workspace path with <WS>, the repo root
 * with <REPO>, $HOME with <HOME>, and masks ISO timestamps, so goldens are
 * stable across machines and runs.
 */
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { armLiveServerReaper, trackServerChild } from '../lib/live-servers.mjs';
import { offlineNetworkProfile } from '../../../../scripts/phoenix-build-tools.mjs';

// Daemon steps spawn the engine's `live-server` detached, so it is orphaned to
// pid 1 the moment this process dies and stopDaemon() is the only thing that
// ever ends it. Arm the reaper before any case runs: it stamps this process's
// environment, and buildInvocation() below inherits process.env, so the marker
// reaches the daemon and a SIGKILLed oracle run leaves no server behind.
// The marker vars are read by nothing in the engine, so goldens are unchanged.
armLiveServerReaper();

export const ORACLE_DIR = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(ORACLE_DIR, '..', '..');
export const GOLDEN_DIR = path.join(ORACLE_DIR, 'golden');
export const CASES_DIR = path.join(ORACLE_DIR, 'cases');
export const WORKSPACES_DIR = path.join(ORACLE_DIR, 'workspaces');
export const SKILL_SOURCE = path.resolve(REPO_ROOT, '..', '..', 'skills', 'phoenix-ui', 'src');

/**
 * `[--bin <path>] [prefix]`; the binary defaults to $PHOENIX_UI_BIN. Exports
 * the resolved path as PHOENIX_UI_BIN, which normalize() masks as <PHOENIX_UI>.
 */
export function parseArgs(argv) {
  let bin = process.env.PHOENIX_UI_BIN, prefix = '';
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--bin') bin = argv[++i];
    else if (argv[i].startsWith('--bin=')) bin = argv[i].slice('--bin='.length);
    else if (argv[i].startsWith('-')) throw new Error(`unknown argument: ${argv[i]}`);
    else prefix = argv[i];
  }
  if (!bin) throw new Error('pass --bin <phoenix-ui> or set PHOENIX_UI_BIN');
  process.env.PHOENIX_UI_BIN = fs.realpathSync(bin);
  return { bin: process.env.PHOENIX_UI_BIN, prefix };
}

/** verb -> argv (verb name is the subcommand) */
export function binArgv(bin, verb) {
  if (verb === 'cli-help') return [bin, '--help'];
  if (verb === 'cli-version') return [bin, '--version'];
  return [bin, verb];
}

export async function allCases() {
  const out = [];
  for (const f of fs.readdirSync(CASES_DIR).sort()) {
    if (!f.endsWith('.mjs')) continue;
    const mod = await import(pathToFileURL(path.join(CASES_DIR, f)).href);
    const list = typeof mod.default === 'function' ? await mod.default() : mod.default;
    for (const item of Array.isArray(list) ? list : [list]) out.push({ ...item, sourceFile: f });
  }
  const seen = new Set();
  for (const c of out) {
    if (seen.has(c.id)) throw new Error(`duplicate oracle case id: ${c.id}`);
    seen.add(c.id);
  }
  return out;
}

export function stageWorkspace(name) {
  // realpath, so every path the verbs see and every <WS> the harness passes
  // (env, args, lock files) is the same string. macOS's tmpdir is a symlink
  // (/var -> /private/var); without this, goldens recorded there carried
  // symlink artifacts (`../../../../../../..<WS>/...` relative paths, lock
  // files that never matched their own file) that Linux does not reproduce.
  const tmp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-ui-oracle-')));
  if (name) {
    const src = path.join(WORKSPACES_DIR, name);
    if (!fs.existsSync(src)) throw new Error(`oracle workspace not found: ${name}`);
    fs.cpSync(src, tmp, { recursive: true });
  }
  return tmp;
}

// Replace `needle` only where it ends a path segment: at the end of the text
// or followed by anything but a name character (a separator, quote, dot,
// whitespace, JSON punctuation).
// A short home directory (`/root` in a container) is otherwise a substring of
// ordinary words, and `.phoenix-ui/live/roots.json` came out as
// `.phoenix-ui/live<HOME>s.json`.
function maskPath(text, needle, tag) {
  let out = '';
  let i = 0;
  while (i < text.length) {
    const j = text.indexOf(needle, i);
    if (j === -1) break;
    out += text.slice(i, j);
    const after = text[j + needle.length];
    const boundary = after === undefined || !/[A-Za-z0-9_-]/.test(after);
    out += boundary ? tag : needle;
    i = j + needle.length;
  }
  return out + text.slice(i);
}

export function normalize(text, { ws, home = os.homedir() }) {
  if (typeof text !== 'string') return text;
  let out = text;
  // Audit entries record the rendered message length, which includes the binary's path.
  out = out.replace(/"chars":\s*\d+/g, '"chars": <N>');
  const bin = process.env.PHOENIX_UI_BIN;
  if (bin) {
    // The hook footer embeds the admin command "'<bin>' hooks". This must run
    // before the generic binary-path mask below.
    for (const form of [`'${bin}' hooks`, `"${bin}" hooks`, `${bin} hooks`]) out = out.split(form).join('<HOOK_ADMIN_CMD>');
    // The binary's own path: it may sit under $HOME or the repo.
    for (const form of [`'${bin}'`, `"${bin}"`, bin]) out = out.split(form).join('<PHOENIX_UI>');
  }
  let wsReal = null;
  try { wsReal = ws ? fs.realpathSync(ws) : null; } catch { /* staged dir already gone */ }
  for (const [needle, tag] of [
    [wsReal, '<WS>'], [ws, '<WS>'], [SKILL_SOURCE, '<PHOENIX_SKILL>'], [REPO_ROOT, '<REPO>'], [home, '<HOME>'],
  ]) {
    if (needle) out = maskPath(out, needle, tag);
  }
  // context.mjs probes `which cwebp/sips/magick/ffmpeg`; the set found is a
  // property of the recording machine, not of the implementation.
  out = out.replace(/IMAGE_TOOLS: available image converters on this machine: [^.]*\. Use the first suitable one; never probe again this session\./g, 'IMAGE_TOOLS: <IMAGE_TOOLS_PROBE>');
  out = out.replace(/IMAGE_TOOLS: no image converter found \(cwebp, sips, magick, ffmpeg\)\. Ship PNG output unconverted rather than probing per image\./g, 'IMAGE_TOOLS: <IMAGE_TOOLS_PROBE>');
  // context-signals probes localhost dev-server ports (4321, 3000, 5173, ...);
  // whatever is listening on the recording machine is not part of the contract.
  out = out.replace(/"devServer": \{\s*"running": (?:true|false),\s*"ports": \[[^\]]*\]\s*\}/g, '"devServer": <DEV_SERVER_PROBE>');
  // critique-storage stamps snapshots with the wall clock in dash form
  // (2026-05-12T18-30-00Z), both in the file name (<stamp>__<slug>.md) and in
  // the `timestamp:` frontmatter it writes.
  out = out.replace(/\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}Z/g, '<STAMP>');
  // A target given as an absolute path outside the workspace makes the verb
  // print the surface path relative to the root, which climbs as many levels
  // as the staged tmpdir is deep (7 on macOS, 2 on Linux). The climb is a
  // property of the machine, not of the verb.
  out = out.replace(/(?:\.\.\/){2,}(?=\.phoenix-ui\/)/g, '<UP_TO_ROOT>/');
  // Hook audit entries carry wall-clock durations.
  out = out.replace(/"durationMs":\s*\d+(?:\.\d+)?/g, '"durationMs": <MS>');
  // ISO timestamps and epoch millis are run-dependent.
  out = out.replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z/g, '<ISO>');
  out = out.replace(/"(updatedAt|createdAt|checkedAt|lastCheck|lastChecked|timestamp|ts|mtimeMs|mtime|startedAt|endedAt)":\s*\d{10,}/g, '"$1": <EPOCH>');
  // The staleness notice cache (~/.phoenix-ui/staleness-check.json) keys epoch
  // stamps by finding id: { projects: { "<root>": { "<finding-id>": ms } } }.
  out = out.replace(/"([a-z][a-z0-9-]*)":\s*1[6-9]\d{11}(?=[,}\s])/g, '"$1": <EPOCH>');
  // Live mode: server.json, the inject journal, and source locks record the
  // writing process's pid; the helper server mints a UUID token. Both vary
  // per run. Ports and lease stamps are per-case (see `normalize` on a case).
  out = out.replace(/"pid":(\s*)\d+/g, '"pid":$1<PID>');
  out = out.replace(/\(pid \d+\)/g, '(pid <PID>)');
  out = out.replace(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi, '<UUID>');
  return out;
}

/**
 * Case-scoped replacements: `c.normalize` is a list of [regexSource, flags,
 * replacement] applied after the global pass to stdout, stderr, files, and
 * daemon output of that case only. Keeps run-dependent values that only one
 * flow produces (a dynamically chosen server port, lease deadlines) from
 * widening the global normalizer and masking real diffs elsewhere.
 */
function applyCaseNormalizers(text, rules) {
  if (typeof text !== 'string' || !rules?.length) return text;
  let out = text;
  for (const [src, flags, repl] of rules) out = out.replace(new RegExp(src, flags), repl);
  return out;
}

function globToRegex(glob) {
  let re = "";
  for (let i = 0; i < glob.length; i++) {
    const ch = glob[i];
    if (ch === "*") {
      if (glob[i + 1] === "*") {
        i++;
        if (glob[i + 1] === "/") { i++; re += "(?:.*/)?"; } else re += ".*";
      } else re += "[^/]*";
    } else if (ch === "?") re += "[^/]";
    else re += ch.replace(/[.+^${}()|[\]\\]/g, "\\$&");
  }
  return new RegExp("^" + re + "$");
}

export function snapshotFiles(ws, globs) {
  const out = {};
  if (!globs || !globs.length) return out;
  const regs = globs.map(globToRegex);
  const walk = (dir) => {
    for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, ent.name);
      const rel = path.relative(ws, full).split(path.sep).join('/');
      if (ent.isDirectory()) {
        if (ent.name === '.git') continue;
        if (ent.name === 'node_modules') {
          // Only the live-mode preview tree is ours; never walk installed or
          // symlinked packages.
          const preview = path.join(full, '.phoenix-ui-live');
          if (fs.existsSync(preview)) walk(preview);
          continue;
        }
        walk(full);
      } else if (regs.some(r => r.test(rel))) {
        const buf = fs.readFileSync(full);
        out[rel] = isProbablyText(buf) ? buf.toString('utf8') : `<binary ${buf.length} bytes>`;
      }
    }
  };
  walk(ws);
  return Object.fromEntries(Object.entries(out).sort(([a], [b]) => a.localeCompare(b)));
}

function isProbablyText(buf) {
  const n = Math.min(buf.length, 512);
  for (let i = 0; i < n; i++) if (buf[i] === 0) return false;
  return true;
}

/**
 * A case may declare `platforms: ['darwin', 'win32']` when its behavior is a
 * property of the host (case-insensitive file systems, for example) rather
 * than of the implementation. Such a case runs only on those platforms; the
 * runner reports it as skipped elsewhere instead of failing.
 */
export function caseRunsHere(c, platform = process.platform) {
  return !Array.isArray(c.platforms) || c.platforms.includes(platform);
}

/** Run one case. Returns { stdout, stderr, exit, signal, files } normalized. */
export function runCase(c, { bin = process.env.PHOENIX_UI_BIN } = {}) {
  const ws = stageWorkspace(c.workspace);
  try {
    const isolatedHome = path.join(ws, '.oracle-home');
    if (c.isolateHome !== false) fs.mkdirSync(isolatedHome, { recursive: true });
    if (typeof c.setup === 'function') c.setup(ws);
    const steps = c.steps || [c];
    const results = [];
    const daemons = [];
    try {
      for (const step of steps) {
        const merged = { ...c, ...step, verb: step.verb || c.verb };
        // A step-level setup stages state between verbs (e.g. the agent's
        // variant files between wrap and accept).
        if (c.steps && typeof step.setup === 'function') step.setup(ws);
        if (step.daemon) {
          daemons.push(runDaemonStep(merged, { bin, ws, isolatedHome }));
          results.push({ stdout: '', stderr: '', status: null, signal: null, daemon: true });
          continue;
        }
        results.push(runStep(merged, { bin, ws, isolatedHome }));
      }
    } finally {
      for (const d of daemons) stopDaemon(d);
    }
    const files = snapshotFiles(ws, c.files);
    const ctx = { ws };
    const N = (text) => applyCaseNormalizers(normalize(text, ctx), c.normalize);
    const norm = (r) => ({
      stdout: N(r.stdout ?? ''),
      stderr: N(r.stderr ?? ''),
      exit: r.status,
      signal: r.signal || null,
      ...(r.daemon ? { daemon: true } : {}),
    });
    const filesNorm = Object.fromEntries(Object.entries(files).map(([k, v]) => [k, N(v)]));
    const daemonOut = daemons.length
      ? { daemon: daemons.map((d) => ({ stdout: N(d.stdout()), stderr: N(d.stderr()) })) }
      : {};
    if (c.steps) return { steps: results.map(norm), files: filesNorm, ...daemonOut };
    return { ...norm(results[0]), files: filesNorm, ...daemonOut };
  } finally {
    fs.rmSync(ws, { recursive: true, force: true });
  }
}

function buildInvocation(c, { bin, ws, isolatedHome }) {
  const cwd = path.join(ws, c.cwd || '.');
  if (!bin) throw new Error('PHOENIX_UI_BIN not set');
  const argv = binArgv(bin, c.verb);
  const sub = (v) => String(v).replaceAll('<WS>', ws).replaceAll('<REPO>', REPO_ROOT);
  argv.push(...(c.args || []).map(sub));
  const env = {
    ...process.env,
    NO_COLOR: '1',
    FORCE_COLOR: '0',
    DO_NOT_TRACK: '1',
    ...(c.isolateHome === false ? {} : { HOME: isolatedHome, USERPROFILE: isolatedHome }),
    // What the installed launcher exports for the binary; a host API key must not leak in.
    PHOENIX_UI_SKILL_DIR: SKILL_SOURCE,
    PHOENIX_UI_SELF: bin,
    OPENAI_API_KEY: null,
    ...Object.fromEntries(Object.entries(c.env || {}).map(([k, v]) => [k, v == null ? v : sub(v)])),
  };
  for (const [k, v] of Object.entries(env)) if (v == null) delete env[k];
  const stdin = typeof c.stdin === 'string' ? sub(c.stdin) : c.stdin != null ? sub(JSON.stringify(c.stdin)) : '';
  return { argv, cwd, env, stdin };
}

/**
 * Spawn a step's verb detached and wait until `readyFile` (relative to the
 * staged workspace) exists. stdout/stderr go to files under
 * <ws>/.oracle-daemon/ and are read back at teardown so the golden records
 * what the daemon printed over its whole life.
 */
function runDaemonStep(c, opts) {
  const { argv, cwd, env } = buildInvocation(c, opts);
  const outDir = path.join(opts.ws, '.oracle-daemon');
  fs.mkdirSync(outDir, { recursive: true });
  const n = fs.readdirSync(outDir).length;
  const outPath = path.join(outDir, `${n}.stdout`);
  const errPath = path.join(outDir, `${n}.stderr`);
  const outFd = fs.openSync(outPath, 'w');
  const errFd = fs.openSync(errPath, 'w');
  const child = trackServerChild(spawn(argv[0], argv.slice(1), {
    cwd, env, stdio: ['ignore', outFd, errFd], detached: true, windowsHide: true,
  }));
  fs.closeSync(outFd);
  fs.closeSync(errFd);
  const readyFile = path.join(opts.ws, c.readyFile);
  const deadline = Date.now() + (c.readyTimeoutMs || 10_000);
  while (!fs.existsSync(readyFile)) {
    if (child.exitCode !== null || Date.now() > deadline) break;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 25);
  }
  if (!fs.existsSync(readyFile)) {
    throw new Error(`daemon ${c.verb} for case ${c.id} did not create ${c.readyFile}\n${safeRead(errPath)}`);
  }
  return {
    child,
    stdout: () => safeRead(outPath),
    stderr: () => safeRead(errPath),
  };
}

function stopDaemon(d) {
  const { child } = d;
  if (child.exitCode !== null || child.signalCode) return;
  try { child.kill('SIGTERM'); } catch { /* already gone */ }
  const deadline = Date.now() + 3000;
  while (child.exitCode === null && !child.signalCode && Date.now() < deadline) {
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 25);
    // A detached child never reports exit to a synchronous loop; probe the pid.
    try { process.kill(child.pid, 0); } catch { break; }
  }
  try { process.kill(child.pid, 0); child.kill('SIGKILL'); } catch { /* exited */ }
  // Give the OS a beat to release the port and flush the output files.
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
}

function safeRead(p) {
  try { return fs.readFileSync(p, 'utf8'); } catch { return ''; }
}

function runStep(c, opts) {
  const { argv: base, cwd, env, stdin } = buildInvocation(c, opts);
  const argv = c.offline && process.platform === 'darwin'
    ? ['/usr/bin/sandbox-exec', '-p', offlineNetworkProfile, ...base]
    : base;
  return spawnSync(argv[0], argv.slice(1), {
    cwd, env, input: stdin, encoding: 'utf8', timeout: c.timeoutMs || 60_000,
    windowsHide: true, maxBuffer: 64 * 1024 * 1024,
  });
}

export function goldenPath(id) {
  return path.join(GOLDEN_DIR, `${id}.json`);
}

export function writeGolden(id, result) {
  fs.mkdirSync(GOLDEN_DIR, { recursive: true });
  fs.writeFileSync(goldenPath(id), JSON.stringify(result, null, 2) + '\n');
}

export function readGolden(id) {
  const p = goldenPath(id);
  if (!fs.existsSync(p)) return null;
  return JSON.parse(fs.readFileSync(p, 'utf8'));
}

/** Return a list of human-readable differences, empty if equal. */
export function diffResults(golden, actual) {
  const diffs = [];
  if (golden.steps || actual.steps) {
    const g = golden.steps || [], a = actual.steps || [];
    if (g.length !== a.length) diffs.push(`steps: expected ${g.length}, got ${a.length}`);
    for (let i = 0; i < Math.min(g.length, a.length); i++) {
      for (const d of diffResults({ ...g[i], files: {} }, { ...a[i], files: {} })) diffs.push(`step ${i + 1} ${d}`);
    }
    for (const d of diffResults({ files: golden.files, exit: 0, signal: null, stdout: '', stderr: '' }, { files: actual.files, exit: 0, signal: null, stdout: '', stderr: '' })) diffs.push(d);
    const gd = golden.daemon || [], ad = actual.daemon || [];
    if (gd.length !== ad.length) diffs.push(`daemons: expected ${gd.length}, got ${ad.length}`);
    for (let i = 0; i < Math.min(gd.length, ad.length); i++) {
      for (const k of ['stdout', 'stderr']) {
        if (gd[i][k] !== ad[i][k]) diffs.push(`daemon ${i + 1} ${k} differs:\n${firstDiff(gd[i][k], ad[i][k])}`);
      }
    }
    return diffs;
  }
  for (const k of ['exit', 'signal']) {
    if (golden[k] !== actual[k]) diffs.push(`${k}: expected ${golden[k]}, got ${actual[k]}`);
  }
  for (const k of ['stdout', 'stderr']) {
    if (golden[k] !== actual[k]) diffs.push(`${k} differs:\n${firstDiff(golden[k], actual[k])}`);
  }
  const keys = new Set([...Object.keys(golden.files || {}), ...Object.keys(actual.files || {})]);
  for (const k of [...keys].sort()) {
    const g = golden.files?.[k], a = actual.files?.[k];
    if (g === undefined) diffs.push(`file ${k}: unexpected (written by actual only)`);
    else if (a === undefined) diffs.push(`file ${k}: missing (golden has it)`);
    else if (g !== a) diffs.push(`file ${k} differs:\n${firstDiff(g, a)}`);
  }
  return diffs;
}

function firstDiff(a, b) {
  const al = String(a).split('\n'), bl = String(b).split('\n');
  const n = Math.max(al.length, bl.length);
  for (let i = 0; i < n; i++) {
    if (al[i] !== bl[i]) {
      return `  line ${i + 1}\n  - ${JSON.stringify(al[i] ?? '<EOF>')}\n  + ${JSON.stringify(bl[i] ?? '<EOF>')}`;
    }
  }
  return '  (lengths differ)';
}
