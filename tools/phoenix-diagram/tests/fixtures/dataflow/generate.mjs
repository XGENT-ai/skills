// Maintenance oracle: execute the frozen renderers; no Node is used at runtime.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

assert.equal(process.versions.node, '25.2.1', 'Run this oracle with mise exec --no-deps -- node');
assert.equal(process.versions.icu, '77.1');
assert.equal(process.versions.unicode, '16.0');
assert.equal(new Intl.Segmenter().resolvedOptions().locale, 'en-US');
const root = fileURLToPath(new URL('../../../../../', import.meta.url));
verifyFrozenSources({ localReference: path.join(root, 'local/phoenix-diagram/reference') });
process.env.ARCHIFY_DIAGNOSTIC_FORMAT = 'json';
delete process.env.ARCHIFY_QUALITY_PROFILE;
const reference = path.join(root, 'local/phoenix-diagram/reference/archify');
const legacy = path.join(root, 'tools/phoenix-diagram/upstream/legacy-2.17');
const { compileDataflow } = await import(pathToFileURL(path.join(reference, 'renderers/dataflow/dataflow-compiler.mjs')));
const { withIsolatedDiagnosticRecording, rendererFailure } = await import(pathToFileURL(path.join(reference, 'renderers/shared/diagnostics.mjs')));
const temp = fs.mkdtempSync(fileURLToPath(new URL('./.oracle-', import.meta.url)));
const documents = [];
const add = (name, document, qualityOption) => documents.push({ name, document: structuredClone(document), qualityOption });
const base = () => ({ schema_version: 1, diagram_type: 'dataflow', meta: { title: 'Dataflow oracle', output: 'dataflow.html', quality_profile: 'standard' },
  stages: [{ label: 'Source' }, { label: 'Sink' }], nodes: [
    { id: 'a', type: 'frontend', label: 'Input', stage: 0, row: 0 },
    { id: 'b', type: 'backend', label: 'Output', stage: 1, row: 0 },
  ], flows: [{ id: 'ab', from: 'a', to: 'b', label: 'payload' }] });
for (const quality of ['standard', 'showcase']) {
  const d = base(); d.meta.quality_profile = quality; add(`default-${quality}`, d);
  d.flows[0].classification = 'internal'; add(`classification-${quality}`, d);
}
{
  const d = base(); delete d.meta.quality_profile;
  add('omitted-quality-defaults-to-showcase', d);
  add('explicit-standard-without-meta-quality', d, 'standard');
  add('explicit-showcase-without-meta-quality', d, 'showcase');
}
let d = base(); d.nodes[1].row = 1; add('row-dogleg', d);
d.flows[0].labelSegment = 0; add('row-dogleg-label-segment-zero', d);
d = base(); d.nodes[1].stage = 0; d.nodes[1].row = 1; add('same-stage-vertical', d);
d.meta.quality_profile = 'showcase'; add('automatic-vertical-label', d);
d = base(); d.nodes.reverse(); d.flows[0] = { id: 'ba', from: 'b', to: 'a', label: 'reply' }; add('reverse-flow-preserves-direction', d);
for (const route of ['vertical-channel', 'bottom-channel', 'top-channel']) {
  d = base(); d.meta.viewBox = [940, 720];
  if (route === 'vertical-channel') { d.nodes[1].row = 1; d.flows[0].channelX = 210; }
  else { const side = route === 'bottom-channel' ? 'bottom' : 'top'; d.flows[0].fromSide = side; d.flows[0].toSide = side; d.flows[0].channelY = side === 'bottom' ? 220 : 104; }
  d.flows[0].route = route; add(`authored-${route}`, d);
}
d = base(); d.meta.viewBox = [940, 720]; d.nodes[0].width = 128.5; d.nodes[0].height = 63.5;
d.nodes[0].yOffset = 3.25; d.nodes[0].sublabel = 'A sublabel'; d.nodes[0].tag = 'raw';
d.flows[0] = { ...d.flows[0], fromSide: 'bottom', toSide: 'bottom', via: [[100, 220], [315, 220]], labelAt: [207.5, 205], labelDx: 0, labelDy: 0, labelSegment: 0, width: 2.25, classification: 'private', variant: 'security' };
add('all-explicit-geometry-and-classification', d);
for (const pin of ['labelDx', 'labelDy', 'labelSegment']) {
  d = base(); d.meta.quality_profile = 'showcase'; d.flows[0][pin] = 0; d.flows[0].classification = 'internal'; add(`zero-pin-${pin}`, d);
}
d = base(); d.nodes[0].label = 'Long node title'; d.nodes[0].sublabel = 'description'; d.nodes[0].tag = 'store'; add('decoration-rail-text-geometry', d);
d = base(); d.stages[0].label = 'A long stage title with whitespace and e\u0301 👩‍💻 中文'; add('stage-grapheme-and-word-wrapping', d);
d = base(); d.stages[0].label = '⌚\ufe0e'; d.nodes[0].sublabel = '⌚\ufe0e'; d.flows[0].label = '⌚\ufe0e'; add('contract-specific-text-units', d);
d = base(); d.meta.locale = 'zh-CN'; d.meta.legend = { mode: 'all', entries: { default: { label: '流动数据' } } }; add('localized-legend-overrides', d);
d = base(); d.meta.legend = { mode: 'hidden' }; add('hidden-legend', d);
for (const [name, mutate] of [
  ['node-above-area', value => { value.nodes[0].yOffset = -40; }],
  ['node-below-area', value => { value.nodes[0].height = 600; }],
  ['overlapping-nodes', value => { value.nodes[1].stage = 0; }],
  ['capacity-exhausted-label', value => { value.flows[0].labelAt = [100, 157]; }],
  ['non-orthogonal-via', value => { value.flows[0].via = [[200, 220]]; }],
  ['overlong-unshrinkable-context', value => { value.nodes[0].sublabel = 'long unshrinkable content '.repeat(8); }],
]) { d = base(); mutate(d); add(name, d); }
for (const source of ['product-analytics', 'event-stream']) {
  add(`legacy-example-${source}`, JSON.parse(fs.readFileSync(path.join(legacy, `examples/${source}.dataflow.json`))));
  add(`reference-example-${source}`, JSON.parse(fs.readFileSync(path.join(reference, `examples/${source}.dataflow.json`))));
}
for (const file of ['dataflow-label-visibility.json', 'dataflow-bidirectional-labels.json', ...[
  'mixed-port-recovery', 'joint-label-routing', 'vertical-label-neighbour', 'ml-features', 'rejected-grid-recovery', 'vertical-label-corridor',
].map(name => `dataflow-first-draft/${name}.dataflow.json`)]) {
  add(`reference-fixture-${file.replaceAll('/', '-')}`, JSON.parse(fs.readFileSync(path.join(reference, '../test/fixtures', file))));
}

const decode = text => text.replace(/&#(x[0-9a-f]+|\d+);/gi, (_, value) => String.fromCodePoint(value[0].toLowerCase() === 'x' ? parseInt(value.slice(1), 16) : +value))
  .replaceAll('&quot;', '"').replaceAll('&apos;', "'").replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&');
function parseSvg(svg) {
  const document = { tag: '#document', attrs: {}, children: [] }, stack = [document];
  for (const token of svg.match(/<!--[\s\S]*?-->|<[^>]+>|[^<]+/g) || []) {
    if (token.startsWith('<!--')) continue;
    if (token.startsWith('</')) { stack.pop(); continue; }
    if (token.startsWith('<')) {
      const tag = token.match(/^<([\w:-]+)/)?.[1]; if (!tag) continue;
      const attrs = Object.fromEntries([...token.matchAll(/([\w:-]+)="([^"]*)"/g)].map(([, key, value]) => [key, decode(value)]));
      const element = { tag, attrs, children: [] }; stack.at(-1).children.push(element);
      if (!token.endsWith('/>')) stack.push(element);
    } else stack.at(-1).children.push(decode(token));
  }
  return document.children.find(node => node.tag === 'svg');
}
const elements = node => node.children.filter(child => typeof child !== 'string');
const descendants = (node, predicate) => elements(node).flatMap(child => [...(predicate(child) ? [child] : []), ...descendants(child, predicate)]);
const contents = node => node.children.map(child => typeof child === 'string' ? child : contents(child)).join('');
const box = element => Object.fromEntries(['x', 'y', 'width', 'height'].map(key => [key, +element.attrs[key]]));
function textRows(element) {
  const x = +element.attrs.x, y = +element.attrs.y, fontSize = +element.attrs['font-size'];
  const spans = elements(element).filter(child => child.tag === 'tspan');
  if (!spans.length) return [{ text: contents(element), x, y, fontSize, anchor: element.attrs['text-anchor'] || 'start' }];
  let baseline = y;
  return spans.map(span => { baseline = span.attrs.y === undefined ? baseline + +(span.attrs.dy || 0) : +span.attrs.y;
    return { text: contents(span), x: +(span.attrs.x ?? x), y: baseline, fontSize: +(span.attrs['font-size'] ?? fontSize), anchor: span.attrs['text-anchor'] || element.attrs['text-anchor'] || 'start' }; });
}
function geometry(svg) {
  const tree = parseSvg(svg);
  const nodes = descendants(tree, node => node.tag === 'g' && node.attrs['data-node-id']).map(group => {
    const bounds = box(elements(group).find(node => node.tag === 'rect' && node.attrs.class === 'c-mask'));
    const sigil = descendants(group, node => node.attrs['data-semantic-sigil'])[0];
    const transform = sigil?.attrs.transform.match(/translate\(([^ ]+) ([^)]+)\) scale\(([^)]+)\)/);
    return { id: group.attrs['data-node-id'], bounds, texts: descendants(group, node => node.tag === 'text').flatMap(textRows),
      sigil: sigil ? { kind: sigil.attrs['data-semantic-sigil'], x: +transform[1], y: +transform[2], size: +transform[3] * 16 } : null };
  });
  const flows = descendants(tree, node => node.tag === 'path' && node.attrs['data-edge-from']).map(edge => {
    const key = edge.attrs['data-edge-key'];
    const group = descendants(tree, node => node.tag === 'g' && node.attrs['data-detail'] === 'context' && node.attrs['data-edge-key'] === key)[0];
    return { index: +key, points: edge.attrs['data-composition-points'].split(';').map(pair => pair.split(',').map(Number)),
      halo: edge.attrs['data-composition-crossover'] === 'halo', plate: group ? box(elements(group).find(node => node.tag === 'rect')) : null,
      texts: group ? descendants(group, node => node.tag === 'text').flatMap(textRows) : [] };
  });
  const frames = descendants(tree, node => node.attrs['data-composition-frame-kind'] === 'stage').map(frame => ({ index: +frame.attrs['data-composition-frame-id'], bounds: box(frame) }));
  // Stage texts are immediate SVG children; node/flow/legend texts live in groups.
  const stageTexts = elements(tree).filter(node => node.tag === 'text').flatMap(textRows);
  const legendRoot = descendants(tree, node => node.tag === 'g' && Object.hasOwn(node.attrs, 'data-legend'))[0];
  const legend = legendRoot ? descendants(legendRoot, node => node.tag === 'g' && node.attrs['data-legend-semantic-kind']).map(group => ({
    kind: group.attrs['data-legend-semantic-kind'], x: +group.attrs['data-legend-x'], baseline: +group.attrs['data-legend-baseline'], width: +group.attrs['data-legend-width'],
    texts: descendants(group, node => node.tag === 'text').flatMap(textRows),
  })) : [];
  const legendTitle = legendRoot ? elements(legendRoot).filter(node => node.tag === 'text').flatMap(textRows) : [];
  return { viewBox: tree.attrs.viewBox.split(/\s+/).slice(2).map(Number), readerFit: tree.attrs['data-reader-fit'] || '', nodes, flows, frames, stageTexts, legend, legendTitle };
}
const cases = [];
try {
  for (const { name, document, qualityOption } of documents) {
    for (const contract of ['legacy-2.17', 'phoenix-v1']) {
      const input = structuredClone(document); delete input.meta.input_contract;
      const effectiveQuality = qualityOption || input.meta.quality_profile || 'showcase';
      const sourceInput = structuredClone(input); sourceInput.meta.quality_profile = effectiveQuality;
      let svg, diagnostics = [], error;
      if (contract === 'legacy-2.17') {
        const inputPath = path.join(temp, 'input.json'), outputPath = path.join(temp, 'output.html');
        fs.writeFileSync(inputPath, JSON.stringify(sourceInput)); fs.rmSync(outputPath, { force: true });
        const env = { ...process.env, ARCHIFY_DIAGNOSTIC_FORMAT: 'json' }; delete env.ARCHIFY_QUALITY_PROFILE;
        const result = spawnSync(process.execPath, [path.join(legacy, 'renderers/dataflow/render-dataflow.mjs'), inputPath, outputPath], { cwd: temp, encoding: 'utf8', env });
        assert.equal(result.error, undefined);
        if (result.status === 0) {
          const html = fs.readFileSync(outputPath, 'utf8'); svg = html.match(/<svg\b[^>]*viewBox="0 0 [^]*?<\/svg>/)?.[0]; assert.ok(svg, name);
        } else { try { const failure = JSON.parse(result.stderr.trim()); diagnostics = failure.diagnostics; error = failure.error; } catch { throw new Error(`${name}: invalid renderer failure ${result.stderr}`); } }
        assert.deepEqual(JSON.parse(fs.readFileSync(inputPath)), sourceInput, `${name}: legacy changed its temporary effective-quality input`);
      } else {
        const before = JSON.stringify(sourceInput);
        withIsolatedDiagnosticRecording(() => { try { svg = compileDataflow({ dataflow: sourceInput, qualityProfile: effectiveQuality }); }
          catch (failure) { const report = rendererFailure(failure); diagnostics = report.diagnostics; error = report.error; } });
        assert.equal(JSON.stringify(sourceInput), before, `${name}: reference changed its temporary effective-quality input`);
      }
      assert.deepEqual(input, document, `${name}: quality projection must not change the authored fixture`);
      cases.push({ name, contract, document: input, effectiveQuality, ...(qualityOption ? { qualityOption } : {}), ok: Boolean(svg), ...(svg ? { expected: geometry(svg) } : { error, diagnostics }) });
    }
  }
} finally { fs.rmSync(temp, { recursive: true, force: true }); }
fs.writeFileSync(new URL('./oracle.json', import.meta.url), `${JSON.stringify({ format: 1, node: process.versions.node, icu: process.versions.icu, unicode: process.versions.unicode, cases }, null, 2)}\n`);
console.log(`Wrote ${cases.length} actual SVG dataflow oracles: ${cases.filter(value => value.ok).length} accepted.`);
