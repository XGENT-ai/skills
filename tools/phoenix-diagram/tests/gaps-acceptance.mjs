#!/usr/bin/env node
// Bounded native/browser regression acceptance. Visual review remains a separate
// actual-image operation on each delivery created by this run.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyFrozenSources } from '../../../scripts/check-phoenix-diagram-source.mjs';
import { bound, inspectFinalize, readBound, readSnapshot } from '../../../scripts/test-phoenix-diagram-core.mjs';
import { invokeNative } from '../../../scripts/test-phoenix-diagram-runtime.mjs';
import { runExportAcceptance, runPassportAcceptance } from './export-acceptance.mjs';
import { discoverBrowser } from './support/export-browser.mjs';
import { runBrowserFailureAcceptance } from './support/gaps-browser.mjs';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const zones = ['SystemV/AST4ADT', 'SystemV/CST6CDT', 'SystemV/EST5EDT',
  'SystemV/MST7MDT', 'SystemV/PST8PDT', 'SystemV/YST9YDT'];

function options(argv) {
  const values = {};
  for (let at = 0; at < argv.length; at += 2) {
    const key = argv[at], value = argv[at + 1];
    assert(['--engine', '--output-dir'].includes(key) && !Object.hasOwn(values, key), `Unknown or duplicate option: ${key}`);
    assert(value && !value.startsWith('--'), `${key} requires a path`);
    values[key] = path.resolve(value);
  }
  assert.equal(Object.keys(values).length, 2, 'Provide --engine and a new --output-dir');
  return { engine: values['--engine'], outputDir: values['--output-dir'] };
}

function writeNew(file, value) {
  fs.writeFileSync(file, typeof value === 'string' || Buffer.isBuffer(value)
    ? value : `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  return bound(readSnapshot(file));
}

function timelineFacts(html) {
  const attribute = (tag, name) => tag.match(new RegExp(`\\b${name}="([^"]*)"`))?.[1];
  const nodes = [...html.matchAll(/<g\b[^>]*\bdata-node-id="[^"]+"[^>]*>/g)].map(([tag]) => ({
    id: attribute(tag, 'data-node-id'), context: attribute(tag, 'data-node-context'),
    at: attribute(tag, 'data-timeline-at'), ms: Number(attribute(tag, 'data-timeline-ms')),
    x: Number(attribute(tag, 'data-timeline-x')),
  }));
  const axis = html.match(/<text\b[^>]*\bdata-timeline-axis-caption=""[^>]*>([\s\S]*?)<\/text>/)?.[1];
  const labels = [...html.matchAll(/<text\b[^>]*\bclass="[^"]*\btl-time\b[^"]*"[^>]*>([\s\S]*?)<\/text>/g)]
    .map(match => match[1].replace(/<[^>]+>/g, ''));
  assert(nodes.length === 5 && axis && labels.length === 5, 'Incident time facts are missing');
  return { axis, labels, nodes };
}

export async function runTimelineAcceptance({ engine, browser, directory, original, oracle }, timezone, entry) {
  assert(zones.includes(timezone), 'Use one of the six fixed SystemV zones');
  const invoke = async (caseDirectory, name, args) => {
    const response = await invokeNative(engine, args, { options: { browserPath: browser } });
    const record = writeNew(path.join(caseDirectory, `${name}.json`), {
      program: engine.path, argv: args, exitCode: response.exitCode, signal: response.signal,
      stdout: writeNew(path.join(caseDirectory, `${name}.stdout.json`), response.stdout),
      stderr: writeNew(path.join(caseDirectory, `${name}.stderr.txt`), response.stderr),
    });
    assert.equal(response.exitCode, 0, `${name}: ${response.stderr || response.stdout}`);
    return { receipt: response.receipt, record };
  };
  const id = timezone.slice('SystemV/'.length).toLowerCase();
  const caseDirectory = path.join(directory, id);
  fs.mkdirSync(caseDirectory, { mode: 0o700 });
  Object.assign(entry, { id, timezone, status: 'failed', visualReview: 'required-pending' });
  const document = JSON.parse(original.content);
  document.meta.timezone = timezone;
  const candidate = writeNew(path.join(caseDirectory, 'candidate.timeline.json'), document);
  const artifact = path.join(caseDirectory, 'diagram.html');
  const finalized = await invoke(caseDirectory, 'finalize',
    ['finalize', 'timeline', candidate.path, artifact, '--out-dir', path.join(caseDirectory, 'browser'), '--json']);
  assert.deepEqual(finalized.receipt.engine.binary, engine.identity, 'Finalize used a different engine binary');
  const state = inspectFinalize(finalized.receipt, { candidate, engine: finalized.receipt.engine, type: 'timeline', contract: 'phoenix-v1' });
  entry.finalize = finalized.record;
  entry.candidate = candidate;
  entry.artifact = state.artifact;
  entry.deliveryReceipt = state.deliveryReceipt;
  entry.browserReceipt = state.browserReceipt;
  entry.summary = state.summary;
  entry.screenshots = state.screenshots;
  assert.equal(entry.screenshots.length, 4);
  for (const screenshot of entry.screenshots) readBound(screenshot, 'new timeline screenshot');
  const strict = await invoke(caseDirectory, 'strict-check', ['check', artifact, '--strict-provenance', '--json']);
  assert.equal(strict.receipt.status, 'pass');
  assert.equal(strict.receipt.provenance, 'current');
  entry.strict = strict.record;
  const expected = oracle.cases.find(value => value.document.meta.timezone === timezone);
  assert(expected?.ok, `Missing frozen timeline oracle: ${timezone}`);
  assert.deepEqual(expected.document, document, 'Reference differs beyond the display timezone');
  const expectedFacts = timelineFacts(expected.svg);
  const actualFacts = timelineFacts(readBound(state.artifact, 'new timeline').content.toString('utf8'));
  assert.equal(actualFacts.axis, expectedFacts.axis);
  assert.deepEqual(actualFacts.labels, expectedFacts.labels);
  assert.equal(actualFacts.nodes.length, expectedFacts.nodes.length);
  for (const [index, node] of actualFacts.nodes.entries()) {
    const reference = expectedFacts.nodes[index];
    assert.deepEqual({ ...node, x: undefined }, { ...reference, x: undefined });
    assert(Math.abs(node.x - reference.x) < 1e-5, `Event proportional position changed: ${node.id}`);
  }
  entry.timeFacts = actualFacts;
  const passports = await runPassportAcceptance({ artifactPath: artifact, evidenceDir: caseDirectory,
    browserPath: browser, evidence: false, nodes: document.events.map(event => ({ id: event.id })) });
  entry.details = bound(readSnapshot(passports.receiptPath));
  assert.equal(passports.status, 'pass');
  for (const observed of passports.passports) {
    assert.equal(observed.context, expectedFacts.nodes.find(node => node.id === observed.id).context);
  }
  const exports = await runExportAcceptance({ artifactPath: artifact, evidenceDir: caseDirectory, browserPath: browser, offline: true });
  entry.exports = bound(readSnapshot(exports.receiptPath));
  assert.equal(exports.status, 'pass', JSON.stringify(exports.errors));
  readBound(candidate, 'new timezone input');
  readBound(state.artifact, 'finalized timeline after browser/export checks');
  entry.status = 'pass';
  entry.record = writeNew(path.join(caseDirectory, 'result.json'), entry);
  process.stderr.write(`${timezone}: native, time facts, details and exports passed; visual review pending\n`);
  return entry;
}

export async function main(argv = process.argv.slice(2)) {
  let directory, report;
  try {
    const supplied = options(argv);
    const parent = fs.lstatSync(path.dirname(supplied.outputDir));
    assert(parent.isDirectory() && !parent.isSymbolicLink(), 'Run parent must be an ordinary directory');
    fs.mkdirSync(supplied.outputDir, { mode: 0o700 });
    directory = fs.realpathSync(supplied.outputDir);
    report = { schemaVersion: 1, tool: 'phoenix-diagram', kind: 'p2-gaps-acceptance',
      status: 'not-run', visualReview: 'required-pending', startedAt: new Date().toISOString(),
      platform: `${process.platform}-${process.arch}`, timelines: [], browserFailures: { status: 'not-run' } };
    assert.equal(report.platform, 'darwin-arm64', 'Real acceptance requires darwin-arm64');
    fs.accessSync('/usr/bin/sandbox-exec', fs.constants.X_OK);
    const browser = discoverBrowser();
    const engine = bound(readSnapshot(supplied.engine));
    fs.accessSync(engine.path, fs.constants.X_OK);
    const nativeMagic = fs.readFileSync(engine.path).subarray(0, 8);
    assert.equal(nativeMagic.readUInt32LE(0), 0xfeedfacf, 'Use the actual Mach-O native engine');
    assert.equal(nativeMagic.readUInt32LE(4), 0x0100000c, 'Use the arm64 native engine');
    report.engine = engine;
    report.browser = browser;
    verifyFrozenSources({ localReference: null });
    const original = readSnapshot(path.join(root, 'tools/phoenix-diagram/tests/fixtures/input/phoenix-v1/payment-incident.timeline.json'));
    const manifest = JSON.parse(fs.readFileSync(path.join(root, 'tools/phoenix-diagram/upstream/archify-3.1.0-manifest.json')));
    const originalMember = manifest.find(row => row[0] === 'archify/examples/payment-incident.timeline.json');
    assert(originalMember, 'Frozen incident source is missing');
    assert.deepEqual([original.identity.bytes, original.identity.sha256], originalMember.slice(1), 'Incident fixture differs from frozen source');
    report.original = bound(original);
    const oracle = JSON.parse(fs.readFileSync(new URL('./fixtures/timeline/oracle.json', import.meta.url)));
    for (const timezone of zones) {
      const entry = {};
      report.timelines.push(entry);
      await runTimelineAcceptance({ engine, browser, directory, original, oracle }, timezone, entry);
    }
    report.browserFailures = await runBrowserFailureAcceptance({ engine: engine.path, outputDir: directory,
      fixture: path.join(root, 'skills/phoenix-diagram/examples/minimal.tree.json') });
    readBound(report.original, 'frozen original incident');
    readBound(engine, 'supplied native engine');
    report.status = 'pass';
  } catch (error) {
    if (!report) { process.stderr.write(`${error.message}\n`); return 2; }
    report.status = report.timelines.length || report.browserFailures.rows?.length ? 'failed' : 'not-run';
    report.diagnostic = { code: error.code || 'gaps/acceptance', message: error.message,
      ...(error.evidence ? { evidence: error.evidence } : {}) };
  }
  report.finishedAt = new Date().toISOString();
  const index = writeNew(path.join(directory, 'index.json'), report);
  process.stdout.write(`${JSON.stringify({ status: report.status, visualReview: report.visualReview, index })}\n`);
  return report.status === 'pass' ? 0 : 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await main();
}
