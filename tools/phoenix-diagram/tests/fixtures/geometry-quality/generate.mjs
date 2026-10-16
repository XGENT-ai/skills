// Maintenance-only oracle: the runtime never executes these Node modules.
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

const root = fileURLToPath(new URL('../../../../../', import.meta.url));
verifyFrozenSources({ localReference: path.join(root, 'local/phoenix-diagram/reference') });
process.env.ARCHIFY_DIAGNOSTIC_FORMAT = 'json';
const source = 'local/phoenix-diagram/reference/archify/renderers/shared';
const load = name => import(pathToFileURL(path.join(root, source, name)).href);
const { placeAutomaticLabels, reservedLabelRect } = await load('automatic-labels.mjs');
const { shortestOrthogonalGridRoute, cleanRouteDetourProblems } = await load('route-quality.mjs');
const { withIsolatedDiagnosticRecording, throwDiagnosticProblems } = await load('diagnostics.mjs');
const rect = (x, y, width, height) => ({ x, y, width, height });
const label = (relationIndex, lx, ly, width = 60, height = 20, relation = {}) => ({
  ...rect(lx - width / 2, ly - 10, width, height), relationIndex, lx, ly, relation,
});
const route = (relationIndex, points) => ({ relationIndex, points });
const labelCases = [];
const addLabels = (name, input) => {
  const complete = { labels: [], routes: [], components: [], titles: [], viewBox: [500, 300], ...input };
  const before = JSON.stringify(complete);
  const expected = placeAutomaticLabels(complete);
  assert.equal(JSON.stringify(complete), before, `${name}: labels mutated input`);
  labelCases.push({ name, input: complete, expected });
};
addLabels('valid-label-retains-original-anchor', {
  labels: [label(0, 100, 50)], routes: [route(0, [[20, 100], [280, 100]])], placementBottom: 20, maxOwnRouteDistance: 1,
});
for (const [field, value] of [['labelAt', [110, 100]], ['labelDx', 0], ['labelDy', 0], ['labelSegment', 0]]) {
  addLabels(`pin-${field}-retains-collision`, {
    labels: [label(0, 110, 100, 60, 20, { [field]: value })], routes: [route(0, [[20, 100], [280, 100]])],
    components: [rect(90, 80, 60, 60)],
  });
}
addLabels('horizontal-fixed-fractions-before-sliding', {
  labels: [label(0, 130, 100)], routes: [route(0, [[20, 100], [280, 100]])], components: [rect(100, 80, 80, 50)],
});
addLabels('vertical-candidates-keep-source-order', {
  labels: [label(0, 180, 130)], routes: [route(0, [[180, 20], [180, 260]])], components: [rect(160, 110, 50, 50)],
});
addLabels('later-label-vacates-earlier-label-only-slot', {
  labels: [label(0, 230, 90, 56), label(1, 110, 90, 56)],
  routes: [route(0, [[80, 100], [140, 100]]), route(1, [[230, 30], [370, 30]])],
  components: [rect(200, 70, 60, 40)], viewBox: [400, 140], placementBottom: 108, gridSweep: true, fallbackRing: false,
});
addLabels('sliding-finds-unsampled-obstacle-edge-slot', {
  labels: [label(0, 50, 90, 80)], routes: [route(0, [[20, 100], [480, 100]])],
  components: [rect(0, 60, 153, 40), rect(237, 60, 263, 40)], placementBottom: 100, fallbackRing: false,
});
addLabels('vertical-sliding-finds-unsampled-obstacle-edge-slot', {
  labels: [label(0, 94, 20, 20, 80)], routes: [route(0, [[100, 20], [100, 480]])],
  components: [rect(60, 0, 80, 153), rect(60, 237, 80, 263)], viewBox: [200, 500], fallbackRing: false,
});
addLabels('grid-sweep-reranks-a-valid-label', {
  labels: [label(0, 80, 40)], routes: [route(0, [[20, 150], [480, 150]])], gridSweep: true,
});
addLabels('grid-padding-protects-caller-plate-measurement', {
  labels: [label(0, 80, 40)], routes: [route(0, [[20, 150], [480, 150]])], titles: [rect(0, 100, 500, 28)], gridSweep: true,
});
for (const vertical of [false, true]) {
  for (const low of [false, true]) {
    const own = vertical ? [[160, 20], [160, 260]] : [[20, 150], [480, 150]];
    const other = own.map(([x, y]) => vertical ? [x + (low ? -24 : 24), y] : [x, y + (low ? -24 : 24)]).reverse();
    addLabels(`grid-reciprocal-${vertical ? 'vertical' : 'horizontal'}-${low ? 'low' : 'high'}-neighbour`, {
      labels: [label(0, 80, 40)], routes: [route(0, own), route(1, other)], gridSweep: true,
    });
  }
}
for (const keepFallbackNearRoute of [false, true]) {
  addLabels(`fallback-ring-${keepFallbackNearRoute ? 'rejects-distant-island' : 'fixed-offset-order'}`, {
    labels: [label(0, 110, 100)], routes: [route(0, [[100, 100], [120, 100]])],
    components: [rect(0, 40, 230, 160)], viewBox: [240, 240], keepFallbackNearRoute,
  });
}
addLabels('maximum-own-route-distance-bounds-fallback', {
  labels: [label(0, 110, 100)], routes: [route(0, [[100, 100], [120, 100]])],
  components: [rect(0, 40, 230, 160)], viewBox: [240, 240], maxOwnRouteDistance: 30,
});
addLabels('no-feasible-position-keeps-original-failure', {
  labels: [label(0, 100, 100)], routes: [route(0, [[20, 100], [180, 100]])], components: [rect(0, 0, 200, 200)], viewBox: [200, 200],
});
for (const delta of [0, 0.0002]) {
  addLabels(`route-mask-clearance-${delta ? 'inside' : 'exactly'}-four-pixels`, {
    labels: [label(0, 150, 86 + delta)], routes: [route(0, [[20, 150], [280, 150]]), route(1, [[20, 100], [280, 100]])],
  });
}
addLabels('diagonal-route-is-only-a-mask-not-a-placement-segment', {
  labels: [label(0, 150, 150)], routes: [route(0, [[20, 20], [280, 280]])], components: [rect(100, 100, 100, 100)], fallbackRing: false,
});
addLabels('spatial-grid-overflow-still-masks-route', {
  labels: [label(0, 150, 100)], routes: [route(0, [[20, 100], [280, 100]]), route(1, [[-1e18, 100], [1e18, 100]])],
});

const reservationCases = [];
for (const [name, input] of [
  ['reserves-beside-own-route', { label: label(2, 150, 90), points: [[20, 100], [280, 100]], routes: [], labels: [], components: [], viewBox: [500, 300], placementBottom: 300 }],
  ['does-not-reserve-an-unrepairable-component-collision', { label: label(2, 100, 100), points: [[90, 100], [110, 100]], routes: [], labels: [], components: [rect(0, 0, 200, 200)], viewBox: [200, 200], placementBottom: 200 }],
  ['reservation-only-rejects-component-overlap', { label: label(2, -40, -40), points: [[-50, -30], [-30, -30]], routes: [], labels: [], components: [], viewBox: [20, 20], placementBottom: 20 }],
  ['reservation-preserves-pinned-anchor', { label: label(2, 150, 90, 60, 20, { labelDx: 0 }), points: [[20, 100], [280, 100]], routes: [route(3, [[0, 90], [300, 90]])], labels: [], components: [], viewBox: [500, 300], placementBottom: 300 }],
  ['reservation-honours-existing-route-and-label-inventory', { label: label(2, 150, 90), points: [[20, 100], [280, 100]], routes: [route(3, [[0, 90], [300, 90]])], labels: [label(4, 150, 120)], components: [], viewBox: [500, 300], placementBottom: 300 }],
]) reservationCases.push({ name, input, expected: reservedLabelRect(input) });

const gridCases = [];
const addGrid = (name, overrides, acceptMode = 'all') => {
  const input = { start: [20, 60], end: [280, 60], points: [[20, 60], [280, 60]], obstacles: [], fromSide: 'right', toSide: 'left', clearance: 2, maximumObstacleCount: 80, ...overrides };
  let calls = 0;
  const metrics = {};
  const expected = shortestOrthogonalGridRoute({ ...input, metrics, accept: () => { calls++; return acceptMode === 'all' || (acceptMode === 'reject-first' && calls > 1); } });
  gridCases.push({ name, input, acceptMode, acceptCalls: calls, expected, metrics });
};
addGrid('straight-route', {});
addGrid('symmetric-obstacle-retains-minheap-tie-break', { obstacles: [rect(100, 40, 100, 40)] });
addGrid('all-obstacles-remain-visible-outside-initial-route-bounds', { obstacles: [rect(100, 40, 100, 40), rect(220, -30, 20, 70)] });
addGrid('avoid-crossing-and-touching-occupied-interior', { avoidedSegments: [{ start: [150, 0], end: [150, 120] }] });
addGrid('allow-proper-avoided-crossing', { avoidedSegments: [{ start: [150, 0], end: [150, 120] }], allowAvoidedCrossings: true });
addGrid('avoid-collinear-corridor-at-overlap-threshold', { avoidedSegments: [{ start: [100, 60], end: [108, 60] }] });
addGrid('disjoint-segment-with-zero-overlap-threshold-rejects-edges', { avoidedSegments: [{ start: [600, 600], end: [610, 600] }], minimumAvoidedOverlapPx: 0 });
addGrid('frame-border-may-be-crossed-but-never-borrowed', { borderSegments: [{ start: [50, 60], end: [250, 60] }] });
addGrid('border-recovery-adds-full-segment-corridor', { borderSegments: [{ start: [24, -40], end: [24, 150] }], routeSeparationPx: 2, minimumSegmentPx: 8 }, 'reject-first');
addGrid('caller-rejection-retries-denser-grid-with-same-budget', { obstacles: [rect(100, 40, 100, 40)] }, 'reject-first');
addGrid('retry-cannot-reset-original-grid-node-budget', { maximumGridNodes: 4 }, 'reject-first');
addGrid('rejected-both-candidates-remains-rejected', {}, 'none');
addGrid('node-budget-is-not-a-success', { maximumGridNodes: 0 });
addGrid('obstacle-budget-is-not-a-success', { maximumObstacleCount: 0, obstacles: [rect(100, 40, 100, 40)] });
addGrid('blocked-endpoint-does-not-start-search', { obstacles: [rect(20, 50, 20, 20)] });
addGrid('unsupported-side-does-not-pretend-to-route', { fromSide: 'diagonal' });
addGrid('bend-penalty-keeps-direction-in-search-state', { points: [[20, 60], [95, 60], [104, 80], [109, 91], [280, 60]], obstacles: [rect(100, 40, 100, 40)], bendPenaltyPx: 64 });
addGrid('strict-overlap-below-corridor-threshold', { avoidedSegments: [{ start: [100, 60], end: [107.999, 60] }] });

const endpointRects = [rect(0, 40, 20, 40), rect(280, 40, 20, 40)];
const detourPoints = [[20, 60], [40, 60], [40, 360], [260, 360], [260, 60], [280, 60]];
const relation = (id, from, to, points = detourPoints) => ({ id, from, to, via: points.slice(1, -1), points });
const detourCases = [];
const addDetour = (name, overrides) => {
  const input = { relations: [relation('ab', 'a', 'b')], obstacles: endpointRects, contentRects: endpointRects, endpointIds: ['a', 'b', 'c', 'd'], diagramType: 'architecture', relationCollection: 'connections', profile: 'showcase', ...overrides };
  const { endpointIds, ...rest } = input;
  const expected = withIsolatedDiagnosticRecording(() => {
    const messages = cleanRouteDetourProblems({ ...rest, endpointIds: new Set(endpointIds), pathFor: value => ({ points: value.points }), fromSideFor: value => value.fromSide, toSideFor: value => value.toSide });
    let diagnostics = [];
    if (messages.length) {
      try { throwDiagnosticProblems('detour oracle', messages); } catch (error) { diagnostics = error.archifyDiagnostics; }
    }
    return { messages, diagnostics };
  });
  detourCases.push({ name, input, expected });
};
addDetour('authored-empty-excursion-is-rejected', {});
addDetour('standard-profile-does-not-run-showcase-detour-check', { profile: 'standard' });
addDetour('automatic-route-is-not-an-authored-detour', { relations: [{ ...relation('ab', 'a', 'b'), via: [] }] });
addDetour('unknown-endpoint-is-not-evaluated-as-a-detour', { endpointIds: ['a'] });
addDetour('diagonal-via-is-outside-orthogonal-detour-scope', { relations: [relation('ab', 'a', 'b', [[20, 60], [120, 360], [280, 60]])] });
addDetour('short-authored-route-does-not-exceed-detour-threshold', { relations: [relation('ab', 'a', 'b', [[20, 60], [40, 60], [40, 100], [260, 100], [260, 60], [280, 60]])] });
addDetour('opaque-obstacle-explains-long-route', { relations: [relation('ab', 'a', 'b', [[20, 60], [40, 60], [40, 560], [260, 560], [260, 60], [280, 60]])], obstacles: [...endpointRects, rect(100, -500, 100, 1000)] });
for (const length of [31.999, 32]) {
  addDetour(`related-shared-outer-corridor-${length}`, { relations: [relation('ab', 'a', 'b'), { ...relation('ac', 'a', 'c', [[100, 360], [100 + length, 360]]), via: [] }] });
}
addDetour('unrelated-shared-outer-corridor-does-not-excuse-detour', { relations: [relation('ab', 'a', 'b'), { ...relation('cd', 'c', 'd', [[100, 360], [200, 360]]), via: [] }] });
addDetour('empty-control-inside-wide-content-bounds-is-still-distant', { contentRects: [rect(0, -500, 20, 1), rect(280, 500, 20, 1)] });
addDetour('detour-obstacle-budget-leaves-evaluation-incomplete', { thresholds: { maximumObstacleCount: 1 } });
addDetour('empty-excursion-threshold-can-exclude-nearby-route', { thresholds: { minimumEmptyExcursionPx: 301 } });
addDetour('exact-empty-excursion-threshold-is-inclusive', { thresholds: { minimumEmptyExcursionPx: 300 } });
for (const minimumDetourRatio of [2.5, 2.500001]) {
  addDetour(`detour-ratio-threshold-${minimumDetourRatio}`, {
    relations: [relation('ab', 'a', 'b', [[20, 60], [40, 60], [40, 255], [260, 255], [260, 60], [280, 60]])], thresholds: { minimumDetourRatio },
  });
}
for (const minimumExcessLengthPx of [200, 200.0001]) {
  addDetour(`detour-excess-length-threshold-${minimumExcessLengthPx}`, {
    relations: [relation('ab', 'a', 'b', [[20, 60], [40, 60], [40, 160], [260, 160], [260, 60], [280, 60]])], thresholds: { minimumDetourRatio: 1.5, minimumExcessLengthPx },
  });
}
addDetour('explicit-unsupported-side-cannot-prove-detour-check', { relations: [{ ...relation('ab', 'a', 'b'), fromSide: 'diagonal' }] });

const output = { format: 1, source, labelCases, reservationCases, gridCases, detourCases };
await writeFile(new URL('./oracle.json', import.meta.url), `${JSON.stringify(output, null, 2)}\n`);
console.log(`Wrote ${labelCases.length} label, ${reservationCases.length} reservation, ${gridCases.length} grid and ${detourCases.length} detour oracle cases.`);
