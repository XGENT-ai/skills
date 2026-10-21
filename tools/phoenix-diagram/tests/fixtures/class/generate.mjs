// Maintenance oracle: execute the frozen class renderer; no Node is used at runtime.
// Run with `mise exec --no-deps -- node tools/phoenix-diagram/tests/fixtures/class/generate.mjs [--check]`.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

assert.equal(process.version, 'v25.2.1', 'Use the pinned maintenance Node');
const root = fileURLToPath(new URL('../../../../../', import.meta.url));
// A worktree may link the verified unpack instead of holding a copy.
const localReference = realpathSync(path.join(root, 'local/phoenix-diagram/reference'));
verifyFrozenSources({ localReference });
const reference = path.join(localReference, 'archify');
const source = 'local/phoenix-diagram/reference/archify/renderers/class/render-class.mjs';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const json = file => readFile(file, 'utf8').then(JSON.parse);

// ---- Cases ------------------------------------------------------------------
const type = (id, kind, row, col, members = {}, extra = {}) => ({ id, label: extra.label || id, kind, row, col, ...members, ...extra });
const field = (name, type, visibility, extra = {}) => ({ name, ...(type ? { type } : {}), ...(visibility ? { visibility } : {}), ...extra });
const method = (name, parameters, returns, visibility, extra = {}) => ({ name, ...(parameters !== undefined ? { parameters } : {}), ...(returns ? { returns } : {}), ...(visibility ? { visibility } : {}), ...extra });
const relation = (id, from, to, kind, extra = {}) => ({ id, from, to, kind, ...extra });
const base = (title, extra = {}) => ({ schema_version: 1, diagram_type: 'class', meta: { title, output: 'class.html', ...extra } });

const cases = [];
const add = (name, document, quality = document.meta.quality_profile || 'standard') => cases.push({ name, quality, document });

for (const file of ['payments.class.json', 'payment-processors.class.json']) {
  add(`reference-${file.replace('.class.json', '')}`, await json(path.join(reference, 'examples', file)));
}
{
  const example = await json(path.join(root, 'skills/phoenix-diagram/examples/minimal.class.json'));
  delete example.meta.input_contract;
  add('minimal-example', example);
}
add('every-kind-and-notation', {
  ...base('Every notation', { quality_profile: 'showcase' }),
  layout: { mode: 'grid' },
  types: [
    type('service', 'class', 0, 0, { attributes: [field('repository', 'Repository', 'private'), field('instances', 'int', 'public', { static: true })], methods: [method('handle', 'request: Request', 'Response', 'public'), method('audit', '', undefined, 'protected', { abstract: true })] }, { sublabel: 'Entry point' }),
    type('repository', 'interface', 0, 1, { methods: [method('find', 'id: Id', 'Entity?', 'public')] }),
    type('base', 'abstract', 1, 0, { attributes: [field('id', 'Id', 'protected')], methods: [method('validate', '', 'bool', 'package', { abstract: true })] }, { label: 'BaseEntity' }),
    type('status', 'enum', 1, 1, { attributes: [field('OPEN'), field('CLOSED')] }),
    type('money', 'record', 2, 0, { attributes: [field('amount', 'Decimal'), field('currency', 'Currency')] }),
    type('cache', 'class', 0, 2, { attributes: [field('entries', 'Map<Id, Entity>', 'private')] }),
  ],
  relationships: [
    relation('uses', 'service', 'repository', 'dependency', { label: 'queries' }),
    relation('state', 'base', 'status', 'association', { label: 'state' }),
    relation('owns', 'base', 'money', 'composition'),
    relation('holds', 'repository', 'cache', 'aggregation', { label: 'caches' }),
    relation('extends', 'service', 'base', 'inheritance'),
  ],
});
add('member-wrapping', {
  ...base('Wrapped members', { quality_profile: 'showcase' }),
  layout: { mode: 'grid', typeMaxW: 220, gapX: 140 },
  types: [
    type('processor', 'class', 0, 0, { attributes: [field('retryPolicyWithExponentialBackoff', 'RetryPolicy<PaymentAttempt, ProviderError>', 'private')], methods: [method('authorize', 'payment: Payment, idempotencyKey: IdempotencyKey, timeout: Duration', 'Result<Authorization, ProviderError>', 'public'), method('refund', 'authorization: Authorization', 'Refund', 'public', { static: true })] }),
    type('narrow', 'class', 0, 1, { methods: [method('verylongmethodnamewithoutanybreakopportunity', '', undefined, 'public')] }, { width: 160 }),
  ],
  relationships: [relation('calls', 'processor', 'narrow', 'dependency')],
});
add('half-column-supertype', {
  ...base('Half column', { quality_profile: 'standard' }),
  layout: { mode: 'grid', gapX: 100, gapY: 90 },
  types: [
    type('shape', 'abstract', 0, 0.5, { methods: [method('area', '', 'number', 'public', { abstract: true }), method('describeWithAVeryLongMethodName', 'precision: int', 'string', 'public')] }, { label: 'Shape' }),
    type('circle', 'class', 1, 0, { attributes: [field('radius', 'number', 'private')] }),
    type('square', 'class', 1, 1, { attributes: [field('side', 'number', 'private')] }),
  ],
  relationships: [relation('circle_shape', 'circle', 'shape', 'inheritance'), relation('square_shape', 'square', 'shape', 'inheritance')],
});
add('bus-conflict-falls-back', {
  ...base('Bus fallback', { quality_profile: 'standard' }),
  layout: { mode: 'grid', gapX: 100, gapY: 120 },
  types: [
    type('contract', 'interface', 0, 1, { methods: [method('run', '', 'void', 'public')] }),
    type('left', 'class', 1, 0, { methods: [method('run', '', 'void', 'public')] }),
    type('right', 'class', 1, 2, { methods: [method('run', '', 'void', 'public')] }),
    type('middle', 'class', 1, 1, { methods: [method('run', '', 'void', 'public')] }),
  ],
  relationships: [
    relation('left_contract', 'left', 'contract', 'realization'),
    relation('right_contract', 'right', 'contract', 'realization'),
    relation('middle_contract', 'middle', 'contract', 'dependency', { label: 'uses' }),
  ],
});
add('absolute-positions-and-pins', {
  ...base('Absolute', { quality_profile: 'standard', locale: 'zh-CN', legend: { mode: 'all', entries: { dependency: { label: 'Uses at runtime' }, aggregation: { visible: false } } } }),
  types: [
    { id: 'a', label: 'Alpha', kind: 'class', pos: [40, 60], methods: [method('go', '', 'void', 'public')] },
    { id: 'b', label: 'Beta', kind: 'interface', pos: [420, 60], methods: [method('go', '', 'void', 'public')] },
    { id: 'c', label: 'Gamma', kind: 'record', pos: [420, 300], attributes: [field('value', 'int')] },
  ],
  relationships: [
    relation('a_b', 'a', 'b', 'realization', { route: 'orthogonal-h', label: 'implements' }),
    relation('c_a', 'c', 'a', 'association', { via: [[300, 337], [300, 230], [130, 230]], fromSide: 'left', toSide: 'bottom', labelAt: [215, 222] }),
    relation('a_c', 'a', 'c', 'dependency', { via: [] }),
  ],
});
add('authored-viewbox-hidden-legend', {
  ...base('Fixed canvas', { quality_profile: 'standard', viewBox: [720, 420], legend: { mode: 'hidden' }, translations: { 'node.context.class.class': 'Klasse' } }),
  layout: { mode: 'grid', origin: [48, 56], typeW: 200 },
  types: [
    type('order', 'class', 0, 0, { attributes: [field('id', 'OrderId', 'private')] }),
    type('line', 'record', 0, 1, { attributes: [field('sku', 'Sku'), field('qty', 'int')] }),
  ],
  relationships: [relation('order_lines', 'order', 'line', 'composition', { label: 'lines' })],
});
add('reject-title-text-capacity', {
  ...base('Title capacity'),
  layout: { mode: 'grid' },
  types: [type('t', 'class', 0, 0, {}, { label: 'AnExtraordinarilyLongTypeNameThatCannotFit', width: 140 })],
});
add('reject-marker-capacity', {
  ...base('Marker capacity'),
  layout: { mode: 'grid', gapY: 40 },
  types: [
    type('hub', 'class', 1, 0),
    type('a', 'class', 0, 1),
    type('b', 'class', 1, 1),
    type('c', 'class', 2, 1),
  ],
  relationships: [relation('a_hub', 'a', 'hub', 'association'), relation('b_hub', 'b', 'hub', 'dependency'), relation('c_hub', 'c', 'hub', 'association')],
});
add('reject-overlapping-types', {
  ...base('Overlap'),
  types: [
    { id: 'p', label: 'P', kind: 'class', pos: [40, 60] },
    { id: 'q', label: 'Q', kind: 'class', pos: [120, 70] },
  ],
});
add('reject-short-relationship', {
  ...base('Short'),
  types: [
    { id: 'p', label: 'P', kind: 'class', pos: [40, 60] },
    { id: 'q', label: 'Q', kind: 'class', pos: [240, 60] },
  ],
  relationships: [relation('p_q', 'p', 'q', 'association')],
});

// ---- Measurement ------------------------------------------------------------
const decode = text => text.replace(/&#(x[0-9a-f]+|\d+);/gi, (_, value) => String.fromCodePoint(value[0].toLowerCase() === 'x' ? parseInt(value.slice(1), 16) : +value))
  .replaceAll('&quot;', '"').replaceAll('&apos;', "'").replaceAll('&#39;', "'").replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&');
// Every element outside <defs>/<style> in document order:
// [tag, attributes, parent, own text, full text of a <text> element].
function elements(svg) {
  const result = [], stack = [-1];
  let skip = 0;
  for (const token of svg.match(/<!--[\s\S]*?-->|<[^>]+>|[^<]+/g) || []) {
    if (token.startsWith('<!--')) continue;
    if (token.startsWith('</')) {
      if (skip) skip -= 1; else stack.pop();
      continue;
    }
    if (token.startsWith('<')) {
      const tag = token.match(/^<([\w:-]+)/)[1];
      const closed = token.endsWith('/>');
      if (skip || tag === 'defs' || tag === 'style') { if (!closed) skip += 1; continue; }
      const attrs = Object.fromEntries([...token.matchAll(/([\w:-]+)="([^"]*)"/g)].map(([, key, value]) => [key, decode(value)]));
      result.push([tag, attrs, stack.at(-1), '', '']);
      if (!closed) stack.push(result.length - 1);
    } else if (!skip && stack.at(-1) >= 0) {
      if (token.trim()) result[stack.at(-1)][3] += decode(token);
      for (const open of stack) if (open >= 0 && result[open][0] === 'text') result[open][4] += decode(token);
    }
  }
  return result;
}

const run = (args, quality) => spawnSync(process.execPath, args, {
  cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
  env: { ...process.env, ARCHIFY_QUALITY_PROFILE: quality, ARCHIFY_DIAGNOSTIC_FORMAT: 'json' },
});
const renderer = path.join(reference, 'renderers/class/render-class.mjs');
const sourceSha256 = digest(await readFile(renderer));
const results = [];
const temporary = await mkdtemp(path.join(os.tmpdir(), 'phoenix-class-oracle-'));
try {
  for (const { name, quality, document } of cases) {
    const input = path.join(temporary, `${name}.json`), output = path.join(temporary, `${name}.html`);
    await writeFile(input, JSON.stringify(document));
    const rendered = run([renderer, input, output], quality);
    assert.equal(rendered.signal, null, `${name}: source terminated`);
    const entry = { name, quality, document, ok: rendered.status === 0 };
    if (entry.ok) {
      const layout = run([renderer, input, output, '--layout-json'], quality);
      assert.equal(layout.status, 0, `${name}: layout report failed`);
      entry.layout = JSON.parse(layout.stdout);
      const html = await readFile(output, 'utf8');
      const svg = html.match(/<svg\b[^>]*viewBox="0 0 [^"]+"[\s\S]*?<\/svg>/)?.[0];
      assert(svg, `${name}: no main SVG`);
      entry.svgSha256 = digest(svg);
      entry.elements = elements(svg);
    } else {
      const failure = JSON.parse(rendered.stderr.trim() || rendered.stdout.trim());
      entry.diagnostics = failure.diagnostics.map(({ code, subject }) => ({ code, subject }));
    }
    if (name.startsWith('reject-')) assert(!entry.ok, `${name}: the source must reject this case`);
    else assert(entry.ok, `${name}: the source must accept this case: ${rendered.stderr}`);
    results.push(entry);
  }
} finally { await rm(temporary, { recursive: true, force: true }); }

const bytes = `{"format":1,"source":${JSON.stringify(source)},"sourceSha256":"${sourceSha256}","cases":[\n${results.map(entry => JSON.stringify(entry)).join(',\n')}\n]}\n`;
const file = new URL('oracle.json', import.meta.url);
if (process.argv.includes('--check')) assert.equal(await readFile(file, 'utf8'), bytes, 'oracle.json');
else await writeFile(file, bytes);
console.log(`${process.argv.includes('--check') ? 'Verified' : 'Wrote'} ${results.length} class cases; ${results.filter(entry => entry.ok).length} real SVGs.`);
