// Maintenance oracle. Executes the frozen tree renderer: its layout report is
// the unrounded scene, its SVG the rendered artifact.
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
const source = 'local/phoenix-diagram/reference/archify/renderers/tree/render-tree.mjs';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const example = async name => JSON.parse(await readFile(path.join(reference, 'archify/examples', name), 'utf8'));

const documents = [];
const add = (name, document, adapted) => documents.push({ name, document, adapted });
const base = (nodes, meta = {}) => ({ schema_version: 1, diagram_type: 'tree', meta: { title: 'Tree oracle', output: 'tree.html', quality_profile: 'showcase', ...meta }, nodes });
const payment = await example('payment-platform.tree.json');
add('payment-platform', payment);
// Too wide for the 930px desktop projection with the default node cap;
// Phoenix narrows only that cap, so the oracle runs the renderer with it.
add('archify-repository', await example('archify-repository.tree.json'), { layout: { nodeMaxW: 140 }, reason: 'composition/desktop-readability' });
add('payment-platform-right', { ...structuredClone(payment), layout: { direction: 'right' } });
add('mixed-groups', base([
  { id: 'root', label: 'Platform', sublabel: 'All capabilities' },
  { id: 'a', label: 'Accounts', parent: 'root' }, { id: 'a1', label: 'Open', parent: 'a' }, { id: 'a2', label: 'Close', parent: 'a' },
  { id: 'l1', label: 'Audit log', parent: 'root' }, { id: 'l2', label: 'Billing export', parent: 'root' }, { id: 'l3', label: 'Search', parent: 'root' },
  { id: 'b', label: 'Ledger', parent: 'root', collapsed: true }, { id: 'b1', label: 'Post entry', parent: 'b' },
  { id: 'l4', label: 'Alerts', parent: 'root' }, { id: 'l5', label: 'Quotas', parent: 'root' },
]));
add('only-stack', base([
  { id: 'repo', label: 'repository' },
  { id: 'src', label: 'src/', parent: 'repo', collapsed: true },
  ...['main.rs', 'lib.rs', 'cli.rs', 'render.rs'].map((label, index) => ({ id: `f${index}`, label, parent: 'src' })),
  { id: 'docs', label: 'docs/', parent: 'repo', sublabel: 'Guides and references' },
  { id: 'readme', label: 'README.md', parent: 'docs' },
]));
add('cjk-and-long-words', base([
  { id: 'r', label: '支付平台能力分解与运营管理中心', sublabel: '按能力拆分，每项能力只属于一个上级' },
  { id: 'x', label: '订单 lifecycle 管理服务', parent: 'r', sublabel: 'Order state machine and its compensation steps' },
  { id: 'y', label: 'Supercalifragilisticexpialidocious-capability-identifier-label', parent: 'r' },
  { id: 'z', label: 'A label that is long enough to wrap onto several balanced lines', parent: 'r', sublabel: '短' },
  { id: 'z1', label: '退款', parent: 'z' }, { id: 'z2', label: '對帳與結算報表', parent: 'z' },
], { locale: 'zh-CN' }));
add('right-wrap', { ...base([
  { id: 'r', label: 'Company', sublabel: 'Org chart' },
  { id: 'e', label: 'Engineering and platform reliability', parent: 'r' }, { id: 's', label: 'Sales', parent: 'r', sublabel: 'Regional teams across three continents' },
  { id: 'e1', label: 'Runtime', parent: 'e' }, { id: 'e2', label: 'Tooling', parent: 'e' }, { id: 'e3', label: 'Observability', parent: 'e' },
  { id: 's1', label: 'EMEA', parent: 's' },
]), layout: { direction: 'right' } });
add('layout-options', { ...base([
  { id: 'r', label: 'Root' }, { id: 'a', label: 'Alpha', parent: 'r' }, { id: 'b', label: 'Beta', parent: 'r' }, { id: 'c', label: 'Gamma', parent: 'r' },
  { id: 'a1', label: 'Alpha one', parent: 'a' }, { id: 'c1', label: 'Gamma one', parent: 'c' }, { id: 'c2', label: 'Gamma two', parent: 'c' },
]), layout: { gapX: 40, gapY: 70, nodeW: 120, nodeMaxW: 180 } });
add('authored-viewbox', base([{ id: 'r', label: 'Root' }, { id: 'a', label: 'Alpha', parent: 'r' }, { id: 'b', label: 'Beta', parent: 'r' }], { viewBox: [900, 600], locale: 'es' }));
add('single-node', base([{ id: 'only', label: 'Only node', sublabel: 'No children' }], { quality_profile: 'standard' }));
add('deep-chain', base(['a', 'b', 'c', 'd', 'e'].map((id, index, ids) => ({ id, label: `Level ${index}`, ...(index ? { parent: ids[index - 1] } : {}) }))));
// The frozen renderer starts an only-child list's spine at the rounded toggle
// x but runs it at the unrounded x, a 0.5px diagonal; Phoenix keeps the spine
// on the toggle x so the link stays orthogonal.
add('fractional-only-stack', base([{ id: 'r', label: 'R'.repeat(20) }, { id: 'x', label: 'X', parent: 'r' },
  ...['a', 'b', 'c'].map(id => ({ id, label: id, parent: 'x' }))]), { reason: 'composition/orthogonal-arrows' });
add('invalid-viewbox-too-small', base([{ id: 'r', label: 'Root' }, ...['a', 'b', 'c', 'd'].map(id => ({ id, label: `Branch ${id}`, parent: 'r' }))], { viewBox: [320, 240] }));
add('invalid-missing-parent', base([{ id: 'r', label: 'Root' }, { id: 'a', label: 'A', parent: 'nowhere' }]));
add('invalid-cycle', base([{ id: 'r', label: 'Root' }, { id: 'a', label: 'A', parent: 'b' }, { id: 'b', label: 'B', parent: 'a' }]));
add('invalid-two-roots', base([{ id: 'r', label: 'Root' }, { id: 's', label: 'Second root' }]));
add('invalid-duplicate-id', base([{ id: 'r', label: 'Root' }, { id: 'a', label: 'A', parent: 'r' }, { id: 'a', label: 'A again', parent: 'r' }]));

const temporary = await mkdtemp(path.join(os.tmpdir(), 'phoenix-tree-oracle-'));
const cases = [];
try {
  for (const { name, document, adapted } of documents) {
    const input = path.join(temporary, `${name}.json`), output = path.join(temporary, `${name}.html`);
    const renderedDocument = adapted ? { ...document, layout: { ...document.layout, ...adapted.layout } } : document;
    await writeFile(input, JSON.stringify(renderedDocument));
    const env = { ...process.env, ARCHIFY_QUALITY_PROFILE: document.meta.quality_profile, ARCHIFY_DIAGNOSTIC_FORMAT: 'json' };
    const run = args => spawnSync(process.execPath, [path.join(root, source), input, output, ...args], { cwd: root, encoding: 'utf8', env });
    const rendered = run([]);
    assert.equal(rendered.signal, null, `${name}: source terminated`);
    const entry = { name, document, ok: rendered.status === 0 };
    if (adapted) {
      await writeFile(input, JSON.stringify(document));
      entry.adapted = { ...adapted, unadaptedViewBox: JSON.parse(run(['--layout-json']).stdout).viewBox };
      await writeFile(input, JSON.stringify(renderedDocument));
    }
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
console.log(`${process.argv.includes('--check') ? 'Verified' : 'Wrote'} ${cases.length} tree cases; ${cases.filter(c => c.ok).length} rendered.`);
