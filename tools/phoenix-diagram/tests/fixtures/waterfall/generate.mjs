// Maintenance oracle. Executes the frozen waterfall renderer: its layout report
// is the unrounded scene, its SVG the rendered artifact.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

assert.equal(process.version, 'v25.2.1', 'Use the pinned maintenance Node');
const root = fileURLToPath(new URL('../../../../../', import.meta.url));
const reference = await realpath(path.join(root, 'local/phoenix-diagram/reference'));
verifyFrozenSources({ localReference: reference });
const source = 'local/phoenix-diagram/reference/archify/renderers/waterfall/render-waterfall.mjs';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const example = async name => JSON.parse(await readFile(path.join(reference, 'archify/examples', name), 'utf8'));

const documents = [];
const add = (name, document) => documents.push({ name, document });
const base = (spans, meta = {}, extra = {}) => ({ schema_version: 1, diagram_type: 'waterfall',
  meta: { title: 'Waterfall oracle', output: 'waterfall.html', evidence: 'measured', quality_profile: 'showcase', ...meta }, ...extra, spans });
add('checkout-request', await example('checkout-request.waterfall.json'));
add('example-rebuild', await example('example-rebuild.waterfall.json'));
add('microseconds-and-rounding', base([
  { id: 'r', name: 'Request', start: 0, end: 1234.567 }, { id: 'a', name: 'Half cent', start: 1.005, end: 2.675, parent: 'r' },
  { id: 'b', name: 'Tiny', start: 3, duration: 0.001235, parent: 'r' }, { id: 'c', name: 'Tinier', start: 3, duration: 0.0012345, parent: 'r' },
  { id: 'd', name: 'Sub-micro', start: 4, duration: 1.2345e-7, parent: 'r' }, { id: 'e', name: 'Exact half', start: 5, duration: 0.125, parent: 'r' },
], { unit: 'us' }));
add('seconds-large-values', base([{ id: 'job', name: 'Nightly job', start: 0, end: 98765.4321, status: 'ok' },
  { id: 'load', name: 'Load partitions', start: 120.5, duration: 45000, parent: 'job', service: 'loader' },
  { id: 'agg', name: 'Aggregate', start: 45120.5, duration: 53000.9, parent: 'job', service: 'compute' }], { unit: 's', evidence: 'illustrative' }));
add('statuses-and-open-span', base([
  { id: 'req', name: 'GET /report', start: 0, end: 900, service: 'gateway' },
  { id: 'q1', name: 'Query A', start: 10, end: 400, parent: 'req', service: 'db', status: 'error' },
  { id: 'q2', name: 'Query B', start: 20, duration: 300, parent: 'req', service: 'db', status: 'cancelled' },
  { id: 'stream', name: 'Stream export', start: 450, parent: 'req', service: 'export', status: 'incomplete' },
  { id: 'cache', name: 'Cache fill', start: 500, end: 700, parent: 'req', status: 'incomplete' },
]));
add('six-services-share-palette', base(['a', 'b', 'c', 'd', 'e', 'f', 'g'].map((service, index) => ({
  id: `s${index}`, name: `Step ${service}`, start: index * 50, duration: 120, service: `svc-${service}` }))));
add('nested-order-and-inherited-service', base([
  { id: 'root', name: 'Pipeline', start: 0, end: 1000, service: 'ci' },
  { id: 'late', name: 'Late child', start: 600, end: 900, parent: 'root' },
  { id: 'early', name: 'Early child', start: 100, end: 500, parent: 'root', service: 'tests' },
  { id: 'tie2', name: 'Tie second', start: 100, end: 200, parent: 'early' },
  { id: 'tie1', name: 'Tie first', start: 100, end: 300, parent: 'early' },
  { id: 'deep', name: 'Deep step', start: 150, end: 180, parent: 'tie2' },
  { id: 'other', name: 'Second root', start: 50, end: 70 },
]));
add('single-zero-span', base([{ id: 'only', name: 'Instant marker', start: 42, duration: 0 }], { quality_profile: 'standard' }));
add('narrow-axis', { ...base([{ id: 'a', name: 'Alpha', start: 0, end: 10 }, { id: 'b', name: 'Beta', start: 2, end: 7, parent: 'a' }]), layout: { width: 360 } });
add('locale-zh-cn', { ...base([{ id: 'r', name: '结账请求', start: 0, end: 300, service: 'api' },
  { id: 'p', name: '支付', start: 50, parent: 'r', service: 'pay', status: 'incomplete' }]), meta: { title: '瀑布', output: 'waterfall.html', evidence: 'illustrative', quality_profile: 'showcase', locale: 'zh-CN', unit: 'ms' } });
add('locale-ko', { ...base([{ id: 'r', name: 'Request', start: 0, end: 80, status: 'error' }]), meta: { title: 'Waterfall', output: 'waterfall.html', evidence: 'measured', quality_profile: 'showcase', locale: 'ko' } });
add('invalid-end-before-start', base([{ id: 'a', name: 'A', start: 10, end: 5 }]));
add('invalid-end-duration-disagree', base([{ id: 'a', name: 'A', start: 0, end: 5, duration: 6 }]));
add('invalid-no-end', base([{ id: 'a', name: 'A', start: 0 }]));
add('invalid-derived-end-overflow', base([{ id: 'a', name: 'A', start: 1e308, duration: 1e308 }]));
add('invalid-missing-parent', base([{ id: 'a', name: 'A', start: 0, end: 1, parent: 'nowhere' }]));
add('invalid-cycle', base([{ id: 'a', name: 'A', start: 0, end: 1, parent: 'b' }, { id: 'b', name: 'B', start: 0, end: 1, parent: 'a' }]));
add('invalid-duplicate-id', base([{ id: 'a', name: 'A', start: 0, end: 1 }, { id: 'a', name: 'A again', start: 0, end: 2 }]));
add('invalid-scale', base([{ id: 'a', name: 'A', start: 0, end: 5e-324 }]));

const temporary = await mkdtemp(path.join(os.tmpdir(), 'phoenix-waterfall-oracle-'));
const cases = [];
try {
  for (const { name, document } of documents) {
    const input = path.join(temporary, `${name}.json`), output = path.join(temporary, `${name}.html`);
    await writeFile(input, JSON.stringify(document));
    const env = { ...process.env, ARCHIFY_QUALITY_PROFILE: document.meta.quality_profile, ARCHIFY_DIAGNOSTIC_FORMAT: 'json' };
    const run = args => spawnSync(process.execPath, [path.join(root, source), input, output, ...args], { cwd: root, encoding: 'utf8', env });
    const rendered = run([]);
    assert.equal(rendered.signal, null, `${name}: source terminated`);
    const entry = { name, document, ok: rendered.status === 0 };
    if (entry.ok) {
      const layout = run(['--layout-json']);
      assert.equal(layout.status, 0, `${name}: layout report failed`);
      entry.layout = JSON.parse(layout.stdout);
      entry.svg = (await readFile(output, 'utf8')).match(/<svg\b[^>]*viewBox="0 0 [^"]+"[\s\S]*?<\/svg>/)?.[0];
      assert(entry.svg, `${name}: no main SVG`);
    } else entry.diagnostics = JSON.parse(rendered.stdout.trim() || rendered.stderr.trim()).diagnostics.map(d => ({ code: d.code, path: d.subject?.path ?? null }));
    cases.push(entry);
  }
} finally { await rm(temporary, { recursive: true, force: true }); }
const oracle = { format: 1, source, sourceSha256: digest(await readFile(path.join(root, source))), cases };
const file = new URL('./oracle.json', import.meta.url);
const bytes = `${JSON.stringify(oracle, null, 1)}\n`;
if (process.argv.includes('--check')) assert.equal(await readFile(file, 'utf8'), bytes, 'oracle.json');
else await writeFile(file, bytes);
console.log(`${process.argv.includes('--check') ? 'Verified' : 'Wrote'} ${cases.length} waterfall cases; ${cases.filter(c => c.ok).length} rendered.`);
