// Native SVG fixtures shaped like the Rust renderer's Reader contract:
// `g[data-node-id]` with one node body and `data-node-kind`, relationship
// paths with `data-edge-from/to/key` and an optional `data-edge-id`, and the
// legend bridge with `data-legend-kind`. The frozen erd, class, timeline and
// waterfall legends are not interactive and the tree draws none. Tree fixtures
// follow the frozen tree renderer's `data-tree-*` attributes. Geometry is illustrative.
const escape = value => String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
const attributes = values => Object.entries(values).filter(([, value]) => value !== undefined && value !== null && value !== false)
  .map(([name, value]) => ` ${name}="${escape(value === true ? '' : value)}"`).join('');

function node({ id, label = id.toUpperCase(), kind = 'backend', x = 0, y = 0, width = 120, height = 48, step = 0, extra = {}, body = '' }) {
  return `<g${attributes({ id: 'node-' + id, 'data-node-id': id, 'data-node-label': label, 'data-node-kind': kind, tabindex: 0, role: 'button', 'aria-label': label, 'aria-pressed': 'false', ...extra })}>`
    + `<title>${escape(label)}</title><rect x="${x}" y="${y}" width="${width}" height="${height}" rx="8" class="c-mask"/>`
    + `<rect data-graph-role="node-body" x="${x}" y="${y}" width="${width}" height="${height}" rx="8" class="c-${kind}" stroke-width="1.5" data-animate="node" style="--step:${step}"/>`
    + `<text x="${x + width / 2}" y="${y + height / 2 + 4}" class="t-primary" text-anchor="middle">${escape(label)}</text>${body}</g>`;
}

export function edge({ from, to, key, id, label = '', d, extra = {} }) {
  return `<path${attributes({ 'data-edge-from': from, 'data-edge-to': to, 'data-edge-label': label, 'data-edge-key': key, 'data-edge-id': id, d, class: 'a-default', 'stroke-width': '1.5', 'data-animate': 'edge', style: '--step:0', 'marker-end': 'url(#arrowhead)', ...extra })}/>`;
}

function legend(entries, interactive = true) {
  return '<g data-legend="" data-legend-bridge="">' + entries.map(({ kind, label, x = 0 }) =>
    `<g data-graph-role="legend" data-legend-semantic-kind="${kind}"${interactive ? ` data-legend-kind="${kind}" data-legend-label="${escape(label)}"` : ''}><rect x="${x}" y="560" width="9" height="9" class="c-${kind}"/><text x="${x + 14}" y="568" class="t-muted">${escape(label)}</text></g>`).join('') + '</g>';
}

function svg({ type, width = 960, height = 600, attrs = {}, body }) {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="phoenix-diagram-title phoenix-diagram-description" lang="en" data-diagram-type="${type}" data-layout-contract="fixed-v1" data-quality-profile="standard" data-reader-fit="" data-animation="trace" data-preset="classic" data-input-contract="phoenix-v1"${attributes(attrs)}>`
    + `<title id="phoenix-diagram-title">${escape(type)} fixture</title><desc id="phoenix-diagram-description">Reader fixture</desc>`
    + `<defs><marker id="arrowhead" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto"><path d="M 0 0 L 10 5 L 0 10 z"/></marker></defs>`
    + `<rect width="100%" height="100%" fill="url(#grid)"/>${body}</svg>`;
}

export const metadata = (type, meta = {}, extra = {}) => ({ tool: 'phoenix-diagram', meta: { title: type + ' fixture', ...meta }, diagramType: type, schemaVersion: 1, inputContract: 'phoenix-v1', ...extra });

// A directed graph: a → b → c → d, a → e, an isolated f. Five node kinds.
export const GRAPH_NODES = [
  { id: 'a', label: 'Alpha', kind: 'frontend', x: 40, y: 40 },
  { id: 'b', label: 'Bravo', kind: 'backend', x: 240, y: 40 },
  { id: 'c', label: 'Charlie', kind: 'backend', x: 440, y: 40 },
  { id: 'd', label: 'Delta', kind: 'database', x: 640, y: 40 },
  { id: 'e', label: 'Echo', kind: 'cloud', x: 240, y: 200 },
  { id: 'f', label: 'Foxtrot', kind: 'external', x: 640, y: 200 },
];
export const GRAPH_EDGES = [
  { from: 'a', to: 'b', key: '0', id: 'a-calls-b', label: 'calls', d: 'M 160 64 L 240 64' },
  { from: 'b', to: 'c', key: '1', label: 'reads', d: 'M 360 64 L 440 64' },
  { from: 'c', to: 'd', key: '2', id: 'c-writes-d', label: 'writes', d: 'M 560 64 L 640 64' },
  { from: 'a', to: 'e', key: '3', label: 'deploys', d: 'M 100 88 L 100 224 L 240 224' },
];
const GRAPH_LEGEND = [
  { kind: 'frontend', label: 'Frontend', x: 40 }, { kind: 'backend', label: 'Backend', x: 160 }, { kind: 'database', label: 'Database', x: 280 },
  { kind: 'cloud', label: 'Cloud', x: 400 }, { kind: 'external', label: 'External', x: 520 },
];

export function graph({ type = 'architecture', nodes = GRAPH_NODES, edges = GRAPH_EDGES, extraBody = '' } = {}) {
  return svg({ type, body: edges.map(edge).join('') + nodes.map((value, index) => node({ ...value, step: index })).join('') + extraBody + legend(GRAPH_LEGEND) });
}

// erd and class keep their relationships as drawable paths with explicit ids.
export function entityGraph(type) {
  const kinds = type === 'erd' ? ['database', 'database', 'database'] : ['backend', 'frontend', 'backend'];
  const nodes = [
    { id: 'customer', label: 'Customer', kind: kinds[0], x: 40, y: 40 },
    { id: 'order', label: 'Order', kind: kinds[1], x: 320, y: 40 },
    { id: 'line', label: 'Line', kind: kinds[2], x: 600, y: 40 },
  ];
  const edges = [
    { from: 'customer', to: 'order', key: '0', id: 'places', label: 'places', d: 'M 160 64 L 320 64' },
    { from: 'order', to: 'line', key: '1', id: 'contains', label: 'contains', d: 'M 440 64 L 600 64' },
  ];
  return svg({ type, body: edges.map(value => edge({ ...value, extra: { 'data-graph-role': 'edge', 'marker-start': 'url(#arrowhead)' } })).join('')
    + nodes.map((value, index) => node({ ...value, step: index })).join('') + legend(type === 'erd' ? [{ kind: 'pk', label: 'Primary key' }] : [{ kind: 'inheritance', label: 'Inheritance' }], false) });
}

// Tree fixture after the frozen tree renderer: ancestors are listed nearest
// first, edges carry the child's ancestors, toggles follow their nodes.
const TREE = [
  { id: 'root', label: 'Platform', parent: null },
  { id: 'payments', label: 'Payments', parent: 'root' },
  { id: 'card', label: 'Card', parent: 'payments' },
  { id: 'wallet', label: 'Wallet', parent: 'payments' },
  { id: 'ops', label: 'Operations', parent: 'root', collapsed: true },
  { id: 'refunds', label: 'Refunds', parent: 'ops' },
];

export function tree() {
  const entries = TREE;
  const byId = new Map(entries.map(entry => [entry.id, entry]));
  const ancestors = id => { const chain = []; for (let current = byId.get(id); current?.parent; current = byId.get(current.parent)) chain.push(current.parent); return chain; };
  const children = id => entries.filter(entry => entry.parent === id);
  const count = id => children(id).reduce((sum, child) => sum + 1 + count(child.id), 0);
  const tag = id => ancestors(id).length ? ancestors(id).join(' ') : undefined;
  const placed = entries.map((entry, index) => ({ ...entry, x: 40 + ancestors(entry.id).length * 200, y: 40 + index * 70 }));
  const edges = placed.filter(entry => entry.parent).map((entry, index) => {
    const parent = placed.find(candidate => candidate.id === entry.parent);
    return edge({ from: entry.parent, to: entry.id, key: String(index), d: `M ${parent.x + 60} ${parent.y + 48} L ${parent.x + 60} ${entry.y + 24} L ${entry.x} ${entry.y + 24}`, extra: { 'data-tree-ancestors': tag(entry.id), class: 'a-default tree-edge', 'marker-end': undefined } });
  });
  const nodes = placed.map((entry, index) => {
    const leaf = !children(entry.id).length;
    const kind = !entry.parent ? 'frontend' : leaf ? 'external' : 'backend';
    return node({ ...entry, kind, step: ancestors(entry.id).length, extra: { 'data-tree-depth': ancestors(entry.id).length, 'data-tree-ancestors': tag(entry.id) },
      body: leaf ? '' : `<rect data-tree-stack="" x="${entry.x + 5}" y="${entry.y + 5}" width="120" height="48" rx="10"/>` });
  });
  const toggles = placed.filter(entry => children(entry.id).length).map(entry => {
    const hidden = count(entry.id);
    return `<g${attributes({ 'data-tree-toggle': entry.id, 'data-tree-count': hidden, 'data-tree-ancestors': tag(entry.id), 'data-tree-initially-collapsed': entry.collapsed || undefined,
      'data-tree-label-collapse': `Collapse ${entry.label} (${hidden} below)`, 'data-tree-label-expand': `Expand ${entry.label} (${hidden} hidden)`, role: 'button', tabindex: 0, 'aria-expanded': 'true', 'aria-label': `Collapse ${entry.label} (${hidden} below)` })}>`
      + `<circle cx="${entry.x + 60}" cy="${entry.y + 56}" r="7" class="tree-toggle-disc"/></g>`;
  });
  return svg({ type: 'tree', attrs: { 'data-tree-ui': '', 'data-tree-direction': 'down' }, body: edges.join('') + nodes.join('') + toggles.join('')
    + '<g data-tree-expand-all="" role="button" tabindex="0" aria-label="Expand all"><rect x="40" y="18" width="90" height="24" rx="12"/><text x="52" y="34">Expand all</text></g>' });
}

// Timeline and waterfall have no authored relationships: no edge paths.
export function timeline() {
  const events = [
    { id: 'kickoff', label: 'Kickoff', kind: 'frontend', x: 80 },
    { id: 'beta', label: 'Beta', kind: 'backend', x: 360 },
    { id: 'launch', label: 'Launch', kind: 'database', x: 640 },
  ];
  return svg({ type: 'timeline', attrs: { 'data-motion-choreography': 'sweep', 'data-timeline-ui': '' }, body:
    '<g data-timeline-lane="main"><rect x="32" y="120" width="880" height="160" rx="10" class="tl-lane"/><line x1="48" y1="200" x2="900" y2="200" class="tl-lane-line"/></g>'
    + events.map((event, index) => node({ ...event, x: event.x - 60, y: 140, step: index, extra: { 'data-timeline-at': '2026-0' + (index + 1) + '-01', 'data-timeline-ms': String(1767225600000 + index * 2678400000), 'data-timeline-x': event.x } })).join('')
    + legend([{ kind: 'change', label: 'Change' }, { kind: 'release', label: 'Release', x: 120 }], false) });
}

export function waterfall() {
  const spans = [
    { id: 'request', label: 'GET /orders', kind: 'frontend', start: 0, end: 120, depth: 0 },
    { id: 'auth', label: 'auth', kind: 'backend', start: 4, end: 20, depth: 1 },
    { id: 'query', label: 'db query', kind: 'database', start: 22, end: 110, depth: 1 },
  ];
  return svg({ type: 'waterfall', attrs: { 'data-motion-choreography': 'replay', 'data-waterfall-ui': '', 'data-waterfall-unit': 'ms', 'data-reader-fit': 'width-first' }, body:
    spans.map((span, index) => node({ ...span, x: 200 + span.start * 5, y: 60 + index * 40, width: (span.end - span.start) * 5, height: 24, step: index,
      extra: { 'data-waterfall-start': span.start, 'data-waterfall-end': span.end, 'data-waterfall-duration': span.end - span.start, 'data-waterfall-depth': span.depth, 'data-waterfall-status': 'ok' } })).join('')
    + legend([{ kind: 'service:edge', label: 'edge' }, { kind: 'service:orders', label: 'orders', x: 120 }], false) });
}
