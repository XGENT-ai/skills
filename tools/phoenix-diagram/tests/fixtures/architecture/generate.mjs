// Maintenance oracle. Execute the frozen renderers; never use layout report rounding.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

assert.equal(process.version, 'v25.2.1', 'Use the pinned maintenance Node');
const root = fileURLToPath(new URL('../../../../../', import.meta.url));
verifyFrozenSources({ localReference: path.join(root, 'local/phoenix-diagram/reference') });
const sources = [['legacy-2.17', 'tools/phoenix-diagram/upstream/legacy-2.17'], ['phoenix-v1', 'local/phoenix-diagram/reference/archify']];
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const decode = text => text.replace(/&(?:amp|lt|gt|quot|#39);/g, entity => ({ '&amp;': '&', '&lt;': '<', '&gt;': '>', '&quot;': '"', '&#39;': "'" }[entity]));
const attrs = tag => Object.fromEntries([...tag.matchAll(/([\w:-]+)="([^"]*)"/g)].map(m => [m[1], decode(m[2])]));
function tree(svg) {
  const dom = { tag: 'root', attrs: {}, children: [] }, stack = [dom];
  for (const match of svg.matchAll(/<!--[\s\S]*?-->|<[^>]+>|[^<]+/g)) {
    const token = match[0];
    if (token.startsWith('<!')) continue;
    if (token.startsWith('</')) { stack.pop(); continue; }
    if (token.startsWith('<')) {
      const node = { tag: token.match(/^<([\w:-]+)/)[1], attrs: attrs(token), children: [] };
      stack.at(-1).children.push(node);
      if (!token.endsWith('/>')) stack.push(node);
    } else stack.at(-1).children.push({ tag: '#text', text: decode(token), children: [] });
  }
  return dom;
}
const all = (node, predicate) => node.children.flatMap(child => [...(predicate(child) ? [child] : []), ...all(child, predicate)]);
const textContent = node => node.tag === '#text' ? node.text : node.children.map(textContent).join('');
const texts = node => all(node, n => n.tag === 'text').map(n => ({ content: textContent(n), x: +n.attrs.x, y: +n.attrs.y, font: +n.attrs['font-size'], anchor: n.attrs['text-anchor'] || 'start' }));
const box = a => Object.fromEntries(['x', 'y', 'width', 'height'].map(k => [k, +a[k]]));
function measure(svg) {
  const dom = tree(svg);
  return {
    viewBox: all(dom, n => n.tag === 'svg')[0].attrs.viewBox.split(' ').map(Number),
    nodes: all(dom, n => n.tag === 'g' && n.attrs['data-node-id']).map(n => ({ id: n.attrs['data-node-id'], bounds: box(all(n, n => n.tag === 'rect')[0].attrs), texts: texts(n) })),
    edges: all(dom, n => n.tag === 'path' && n.attrs['data-edge-key'] !== undefined).map(n => ({ key: +n.attrs['data-edge-key'], d: n.attrs.d, points: n.attrs['data-composition-points'] })),
    legends: all(dom, n => n.tag === 'g' && n.attrs['data-legend-semantic-kind']).map(n => ({ kind: n.attrs['data-legend-semantic-kind'], x: +n.attrs['data-legend-x'], baseline: +n.attrs['data-legend-baseline'], width: +n.attrs['data-legend-width'], swatches: all(n, n => n.tag === 'rect').map(n => ({ ...box(n.attrs), rx: +n.attrs.rx, class: n.attrs.class })), texts: texts(n) })),
    texts: texts(dom),
  };
}
const existing = JSON.parse(await readFile(new URL('./oracle.json', import.meta.url), 'utf8'));
const documents = existing.cases.map(c => ({ name: c.name, document: c.input }));
const base = () => ({ schema_version: 1, diagram_type: 'architecture', meta: { title: 'Legend source', output: 'architecture.html', quality_profile: 'standard', legend: { mode: 'all' } }, components: [{ id: 'a', type: 'frontend', label: 'A', pos: [60, 100] }, { id: 'b', type: 'backend', label: 'B', pos: [320, 100] }], connections: [] });
for (const label of ['Control plane component', '跨区域组件控制边界']) {
  const document = base();
  document.meta.legend.entries = Object.fromEntries(['frontend', 'backend', 'database', 'cloud', 'security', 'external', 'messagebus'].map(kind => [kind, { label }]));
  documents.push({ name: `legend-${label.startsWith('Control') ? 'latin' : 'han'}`, document });
}
const cases = [], expectedFiles = new Map();
const temporary = await mkdtemp(path.join(os.tmpdir(), 'phoenix-architecture-source-'));
try {
  for (const [contract, sourceRoot] of sources) {
    const contractDocuments = [...documents];
    if (contract === 'phoenix-v1') for (const [name, locale, label] of [['no-locale', null, 'Ignored'], ['numeric-placeholder', 'en', 'Legend {1}'], ['literal-brace', 'en', 'Legend{']]) {
      const document = base();
      if (locale) document.meta.locale = locale;
      document.meta.translations = { 'legend.title': label };
      contractDocuments.push({ name: `translation-${name}`, document });
    }
    const source = `${sourceRoot}/renderers/architecture/render-architecture.mjs`;
    const sourceSha256 = digest(await readFile(path.join(root, source)));
    for (const { name, document } of contractDocuments) {
      const key = `${contract}-${name.replace(/[^\w.-]/g, '-')}`, input = path.join(temporary, `${key}.json`), output = path.join(temporary, `${key}.html`);
      const quality = document.meta.quality_profile || 'standard';
      await writeFile(input, JSON.stringify(document));
      const run = spawnSync(process.execPath, [path.join(root, source), input, output], { cwd: root, encoding: 'utf8', env: { ...process.env, ARCHIFY_QUALITY_PROFILE: quality, ARCHIFY_DIAGNOSTIC_FORMAT: 'json' } });
      assert.equal(run.signal, null, `${key}: source terminated`);
      const entry = { contract, name, source, sourceSha256, document, quality, ok: run.status === 0 };
      if (entry.ok) {
        const html = await readFile(output, 'utf8');
        const svg = html.match(/<svg\b[^>]*viewBox="0 0 [^"]+"[\s\S]*?<\/svg>/)?.[0];
        assert(svg, `${key}: no main SVG`);
        entry.svgFile = `${key}.svg`; entry.svgSha256 = digest(svg); entry.geometry = measure(svg);
        expectedFiles.set(entry.svgFile, svg);
      } else entry.diagnostics = JSON.parse(run.stdout.trim() || run.stderr.trim()).diagnostics;
      cases.push(entry);
    }
  }
} finally { await rm(temporary, { recursive: true, force: true }); }
expectedFiles.set('presentation-oracle.json', `${JSON.stringify({ format: 1, cases }, null, 2)}\n`);
for (const [name, bytes] of expectedFiles) {
  const file = new URL(name, import.meta.url);
  if (process.argv.includes('--check')) assert.equal(await readFile(file, 'utf8'), bytes, name);
  else await writeFile(file, bytes);
}
console.log(`${process.argv.includes('--check') ? 'Verified' : 'Wrote'} ${cases.length} architecture source cases; ${cases.filter(c => c.ok).length} real SVGs.`);
