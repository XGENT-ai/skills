// Maintenance oracle: execute the frozen erd renderer; no Node is used at runtime.
// Run with `mise exec --no-deps -- node tools/phoenix-diagram/tests/fixtures/erd/generate.mjs [--check]`.
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
const source = 'local/phoenix-diagram/reference/archify/renderers/erd/render-erd.mjs';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const json = file => readFile(file, 'utf8').then(JSON.parse);

// ---- Cases ------------------------------------------------------------------
const attribute = (name, type, key, references) => ({ name, ...(type ? { type } : {}), ...(key ? { key } : {}), ...(references ? { references } : {}) });
const table = (id, row, col, attributes, extra = {}) => ({ id, label: id, row, col, attributes, ...extra });
const relation = (id, from, to, extra = {}) => ({ id, from, to, fromCardinality: 'many', toCardinality: 'one', ...extra });
const base = (title, extra = {}) => ({ schema_version: 1, diagram_type: 'erd', meta: { title, output: 'erd.html', ...extra } });

const cases = [];
const add = (name, document, quality = document.meta.quality_profile || 'standard') => cases.push({ name, quality, document });

for (const file of ['orders.erd.json', 'subscription-billing.erd.json']) {
  add(`reference-${file.replace('.erd.json', '')}`, await json(path.join(reference, 'examples', file)));
}
{
  const example = await json(path.join(root, 'skills/phoenix-diagram/examples/minimal.erd.json'));
  delete example.meta.input_contract;
  add('minimal-example', example);
}
add('default-grid-without-layout', {
  ...base('Default grid'),
  entities: [
    table('account', 0, 0, [attribute('id', 'uuid', 'pk'), attribute('handle', 'text', 'uk')]),
    table('session', 0, 1, [attribute('id', 'uuid', 'pk'), attribute('account_id', 'uuid', 'fk', 'account.id'), attribute('expires_at', 'timestamptz')]),
  ],
  relationships: [relation('session_account', 'session', 'account')],
});
add('composite-keys-and-narrow-tables', {
  ...base('Composite keys', { quality_profile: 'showcase' }),
  layout: { mode: 'grid', gapX: 80 },
  entities: [
    table('tenant', 0, 0, [attribute('id', 'bigint', 'pk'), attribute('slug', 'text', 'uk')]),
    table('member', 0, 1, [attribute('tenant_id', 'bigint', ['pk', 'fk'], 'tenant.id'), attribute('user_id', 'bigint', ['pk', 'fk', 'fk'], 'user.id'), attribute('role')], { width: 260 }),
    table('user', 0, 2, [attribute('id', 'bigint', 'pk'), attribute('email', 'citext', ['uk'])]),
    table('empty', 1, 1, []),
  ],
  relationships: [
    relation('member_tenant', 'member', 'tenant', { label: 'belongs to' }),
    relation('member_user', 'member', 'user', { toOptional: true }),
  ],
});
add('vertical-and-dashed-ends', {
  ...base('Vertical relationships', { quality_profile: 'showcase', locale: 'zh-CN' }),
  layout: { mode: 'grid', gapY: 60 },
  entities: [
    table('invoice', 0, 0, [attribute('id', 'bigint', 'pk'), attribute('total', 'numeric(12,2)')]),
    table('invoice_line', 1, 0, [attribute('id', 'bigint', 'pk'), attribute('invoice_id', 'bigint', 'fk', 'invoice.id')]),
    table('refund', 1, 1, [attribute('id', 'bigint', 'pk'), attribute('invoice_id', 'bigint', 'fk', 'invoice.id')]),
  ],
  relationships: [
    relation('line_invoice', 'invoice_line', 'invoice', { label: 'bills' }),
    relation('refund_invoice', 'refund', 'invoice', { identifying: false, fromOptional: true, toOptional: true, toCardinality: 'many' }),
  ],
});
add('fan-in-trunk', {
  ...base('Fan-in trunk', { quality_profile: 'showcase' }),
  layout: { mode: 'grid', gapX: 96, gapY: 30 },
  entities: [
    table('hub', 1, 0, [attribute('id', 'bigint', 'pk'), attribute('name', 'text'), attribute('created_at', 'timestamptz'), attribute('updated_at', 'timestamptz'), attribute('owner', 'text'), attribute('state', 'text')]),
    table('alpha', 0, 1, [attribute('id', 'bigint', 'pk'), attribute('hub_id', 'bigint', 'fk', 'hub.id')]),
    table('beta', 1, 1, [attribute('id', 'bigint', 'pk'), attribute('hub_id', 'bigint', 'fk', 'hub.id')]),
    table('gamma', 2, 1, [attribute('id', 'bigint', 'pk'), attribute('hub_id', 'bigint', 'fk', 'hub.id')]),
  ],
  relationships: [
    relation('alpha_hub', 'alpha', 'hub', { label: 'feeds' }),
    relation('beta_hub', 'beta', 'hub'),
    relation('gamma_hub', 'gamma', 'hub'),
  ],
});
add('parallel-lanes', {
  ...base('Parallel lanes', { quality_profile: 'standard' }),
  layout: { mode: 'grid', gapX: 120 },
  entities: [
    table('person', 0, 0, [attribute('id', 'bigint', 'pk'), attribute('name', 'text'), attribute('email', 'text'), attribute('phone', 'text')]),
    table('transfer', 0, 1, [attribute('id', 'bigint', 'pk'), attribute('sender_id', 'bigint', 'fk', 'person.id'), attribute('receiver_id', 'bigint', 'fk', 'person.id'), attribute('amount', 'numeric')]),
  ],
  relationships: [
    relation('transfer_sender', 'transfer', 'person', { label: 'sent by' }),
    relation('transfer_receiver', 'transfer', 'person', { label: 'received by', variant: 'emphasis' }),
  ],
});
add('absolute-positions-and-pins', {
  ...base('Absolute positions', { quality_profile: 'standard', legend: { mode: 'all', entries: { pk: { label: 'Identity' }, optional: { visible: false } } } }),
  entities: [
    { id: 'a', label: 'source table', pos: [40, 60], width: 220, attributes: [attribute('id', 'int', 'pk'), attribute('b_id', 'int', 'fk', 'b.id')] },
    { id: 'b', label: 'target table', pos: [420, 60], attributes: [attribute('id', 'int', 'pk'), attribute('label', 'text')] },
    { id: 'c', label: 'side table', pos: [420, 260], attributes: [attribute('id', 'int', 'pk'), attribute('a_id', 'int', 'fk', 'a.id')] },
  ],
  relationships: [
    relation('a_b', 'a', 'b', { label: 'points at', route: 'orthogonal-h', variant: 'security' }),
    relation('c_a', 'c', 'a', { via: [[340, 298], [340, 220], [150, 220]], fromSide: 'left', toSide: 'bottom', labelAt: [250, 214], width: 2 }),
  ],
});
add('authored-viewbox-hidden-legend', {
  ...base('Fixed canvas', { quality_profile: 'standard', viewBox: [760, 360], legend: { mode: 'hidden' } }),
  layout: { mode: 'grid', origin: [48, 56], entityW: 220, headerH: 34, rowH: 22 },
  entities: [
    table('product', 0, 0, [attribute('sku', 'text', 'pk'), attribute('title', 'text')]),
    table('stock', 0, 1, [attribute('sku', 'text', ['pk', 'fk'], 'product.sku'), attribute('on_hand', 'int')]),
  ],
  relationships: [relation('stock_product', 'stock', 'product', { fromCardinality: 'one', label: 'tracks' })],
});
add('translated-legend-and-long-names', {
  ...base('Translated', { quality_profile: 'showcase', locale: 'es', translations: { 'legend.title': 'Clave', 'legend.erd.fk': 'Clave externa propia' } }),
  layout: { mode: 'grid' },
  entities: [
    table('cuenta_principal_con_nombre_largo', 0, 0, [attribute('identificador_de_cuenta_principal', 'bigint', 'pk'), attribute('correo', 'text', 'uk')], { label: 'Cuenta principal con nombre largo', width: 320 }),
    table('movimiento', 0, 1, [attribute('id', 'bigint', 'pk'), attribute('cuenta_id', 'bigint', 'fk', 'cuenta_principal_con_nombre_largo.identificador_de_cuenta_principal')]),
  ],
  relationships: [relation('movimiento_cuenta', 'movimiento', 'cuenta_principal_con_nombre_largo', { label: 'pertenece a' })],
});
add('domain-block-band', {
  ...base('Domain block', { quality_profile: 'showcase' }),
  layout: { mode: 'grid', gapX: 72, gapY: 40 },
  entities: [
    table('a1', 0, 0, [attribute('id', 'int', 'pk')], { tag: 'core' }),
    table('a2', 0, 1, [attribute('id', 'int', 'pk'), attribute('a1_id', 'int', 'fk', 'a1.id')], { tag: 'core' }),
    table('a3', 1, 0, [attribute('id', 'int', 'pk')], { tag: 'core' }),
    table('a4', 1, 1, [attribute('id', 'int', 'pk'), attribute('a3_id', 'int', 'fk', 'a3.id')], { tag: 'core' }),
    table('b1', 0, 2, [attribute('id', 'int', 'pk'), attribute('a2_id', 'int', 'fk', 'a2.id')], { tag: 'edge' }),
  ],
  relationships: [
    relation('a2_a1', 'a2', 'a1'),
    relation('a4_a3', 'a4', 'a3'),
    relation('b1_a2', 'b1', 'a2', { label: 'extends' }),
  ],
});
// Rejected inputs: each is a geometry fact the input gate cannot know.
add('reject-header-text-capacity', {
  ...base('Header capacity'),
  layout: { mode: 'grid' },
  entities: [table('t', 0, 0, [attribute('id', 'int', 'pk')], { label: 'an extraordinarily long table title', width: 120 })],
});
add('reject-attribute-text-capacity', {
  ...base('Attribute capacity'),
  layout: { mode: 'grid' },
  entities: [table('t', 0, 0, [attribute('a_remarkably_long_column_name_here', 'varchar(255)', 'pk')], { width: 160 })],
});
add('reject-scattered-domain', {
  ...base('Scattered domain'),
  layout: { mode: 'grid' },
  entities: [
    table('x', 0, 0, [attribute('id', 'int', 'pk')], { tag: 'split' }),
    table('y', 1, 1, [attribute('id', 'int', 'pk')], { tag: 'split' }),
  ],
});
add('reject-ungridded-domain', {
  ...base('Off-grid domain'),
  entities: [{ id: 'x', label: 'x', pos: [40, 60], tag: 'loose', attributes: [attribute('id', 'int', 'pk')] }],
});
add('reject-marker-capacity', {
  ...base('Marker capacity'),
  layout: { mode: 'grid', gapY: 30 },
  entities: [
    table('hub', 1, 0, [attribute('id', 'int', 'pk')]),
    table('a', 0, 1, [attribute('id', 'int', 'pk')]),
    table('b', 1, 1, [attribute('id', 'int', 'pk')]),
    table('c', 2, 1, [attribute('id', 'int', 'pk')]),
  ],
  relationships: [relation('a_hub', 'a', 'hub'), relation('b_hub', 'b', 'hub', { variant: 'emphasis' }), relation('c_hub', 'c', 'hub', { variant: 'security' })],
});
add('reject-overlapping-tables', {
  ...base('Overlap'),
  entities: [
    { id: 'p', label: 'p', pos: [40, 60], attributes: [attribute('id', 'int', 'pk')] },
    { id: 'q', label: 'q', pos: [120, 80], attributes: [attribute('id', 'int', 'pk')] },
  ],
});
add('reject-short-relationship', {
  ...base('Short'),
  entities: [
    { id: 'p', label: 'p', pos: [40, 60], attributes: [attribute('id', 'int', 'pk')] },
    { id: 'q', label: 'q', pos: [290, 60], attributes: [attribute('id', 'int', 'pk')] },
  ],
  relationships: [relation('p_q', 'p', 'q')],
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
const renderer = path.join(reference, 'renderers/erd/render-erd.mjs');
const sourceSha256 = digest(await readFile(renderer));
const results = [];
const temporary = await mkdtemp(path.join(os.tmpdir(), 'phoenix-erd-oracle-'));
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
console.log(`${process.argv.includes('--check') ? 'Verified' : 'Wrote'} ${results.length} erd cases; ${results.filter(entry => entry.ok).length} real SVGs.`);
