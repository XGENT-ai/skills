// Maintenance-only oracle. Production and Rust tests never execute Node renderers.
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

const root = fileURLToPath(new URL('../../../../../', import.meta.url));
verifyFrozenSources({ localReference: path.join(root, 'local/phoenix-diagram/reference') });
const sources = [
  ['legacy-2.17', 'tools/phoenix-diagram/upstream/legacy-2.17/renderers/shared/geometry.mjs'],
  ['phoenix-v1', 'local/phoenix-diagram/reference/archify/renderers/shared/geometry.mjs'],
];
const cases = [];
const relation = (from, to, points, extra = {}) => ({ relation: { from, to, ...extra }, points });
const label = (relationIndex, rect, extra = {}) => ({ relationIndex, relation: { id: `label-${relationIndex}`, from: 'label-a', to: 'label-b', label: 'label', ...extra }, rect });
const lineA = relation('a', 'b', [[0, 0], [40, 0]], { id: 'ab' });
const lineB = relation('c', 'd', [[10, 0], [50, 0]], { id: 'cd' });
const pair = (pointsA, pointsB, extraA = {}, extraB = {}) => [relation('a', 'b', pointsA, extraA), relation('c', 'd', pointsB, extraB)];
const add = (operation, name, input, phoenixOnly = false) => cases.push({ operation, name, input, phoenixOnly });

add('corridors', 'unrelated-horizontal-longest', { routedRelations: [lineA, lineB] });
add('corridors', 'unrelated-vertical-reversed', { routedRelations: pair([[8, 0], [8, 45]], [[8, 50], [8, 10]]) });
add('corridors', 'separate-parallel-lines', { routedRelations: pair([[0, 0], [40, 0]], [[0, 1], [40, 1]]) });
add('corridors', 'diagonal-is-not-axis-corridor', { routedRelations: pair([[0, 0], [40, 40]], [[10, 10], [50, 50]]) });
add('corridors', 'below-overlap-floor', { routedRelations: pair([[0, 0], [7.9998, 0]], [[0, 0], [40, 0]]) });
add('corridors', 'numeric-overlap-floor', { routedRelations: pair([[0, 0], [7.99995, 0]], [[0, 0], [40, 0]]) });
add('corridors', 'forward-waypoints-are-normalized', { routedRelations: pair([[0, 0], [8, 0], [20, 0], [20, 40]], [[4, 0], [15, 0], [15, -40]]) });
add('corridors', 'first-equal-longest-wins', { routedRelations: pair([[0, 0], [20, 0], [20, 40], [0, 40]], [[5, 0], [15, 0], [15, 40], [5, 40]]) });
for (const length of [24, 24.0002]) {
  const routes = [relation('a', 'b', [[0, 0], [length, 0], [length, 40]]), relation('a', 'c', [[0, 0], [length, 0], [length, -40]])];
  add('corridors', `shared-source-default-${length}`, { routedRelations: routes });
  add('corridors', `shared-source-full-${length}`, { routedRelations: routes, includeSharedEndpoints: true });
  add('corridors', `shared-source-short-trunk-${length}`, { routedRelations: routes, includeSharedEndpoints: true, allowShortWorkflowTrunks: true });
}
for (const [name, field] of [['width', { width: 3 }], ['variant', { variant: 'emphasis' }], ['role', { role: 'return' }]]) {
  add('corridors', `short-trunk-different-${name}`, { routedRelations: [relation('a', 'b', [[0, 0], [20, 0], [20, 40]]), relation('a', 'c', [[0, 0], [20, 0], [20, -40]], field)], includeSharedEndpoints: true, allowShortWorkflowTrunks: true });
}
add('corridors', 'bracket-piece-has-no-source-endpoint', { routedRelations: [{ ...relation('a', 'b', [[0, 0], [20, 0], [20, 40]]), sourceEndpoint: false }, relation('a', 'c', [[0, 0], [20, 0], [20, -40]])], includeSharedEndpoints: true, allowShortWorkflowTrunks: true });
add('corridors', 'shared-target-short-trunk', { routedRelations: [relation('a', 'c', [[0, 0], [20, 0], [20, 20]]), relation('b', 'c', [[40, 0], [20, 0], [20, 20]])], includeSharedEndpoints: true, allowShortWorkflowTrunks: true });
add('corridors', 'shared-target-opposite-direction', { routedRelations: [relation('a', 'c', [[0, 0], [40, 0]]), relation('b', 'c', [[50, 0], [10, 0]])], includeSharedEndpointCounterflow: true });
add('corridors', 'counterflow-opt-in-keeps-forward-exemption', { routedRelations: [relation('a', 'b', [[0, 0], [40, 0]]), relation('a', 'c', [[10, 0], [50, 0]])], includeSharedEndpointCounterflow: true });
add('corridors', 'full-check-takes-precedence-over-counterflow', { routedRelations: [relation('a', 'b', [[0, 0], [40, 0]]), relation('a', 'c', [[10, 0], [50, 0]])], includeSharedEndpoints: true, includeSharedEndpointCounterflow: true });
add('corridors', 'junction-horizontal-bus', { routedRelations: [relation('a', 'b', [[0, 0], [40, 0]], { junction: 'bus' }), relation('c', 'd', [[10, 0], [50, 0]], { junction: 'bus' })] });
add('corridors', 'junction-does-not-exempt-interior-vertical', { routedRelations: [relation('a', 'b', [[0, 0], [0, 40]], { junction: 'bus' }), relation('c', 'd', [[0, 10], [0, 50]], { junction: 'bus' })] });
add('corridors', 'junction-merged-target-drop', { routedRelations: [relation('a', 'c', [[0, 0], [0, 40]], { junction: 'bus' }), relation('b', 'c', [[0, 10], [0, 40]], { junction: 'bus' })], includeSharedEndpoints: true });
add('corridors', 'junction-same-source-port-tick', { routedRelations: [relation('a', 'b', [[0, 0], [0, 40]], { junction: 'bus' }), relation('a', 'c', [[0, 0], [0, 50]], { junction: 'bus' })], includeSharedEndpoints: true });

const rect = { kind: 'boundary', id: 'frame', x: 0, y: 0, width: 100, height: 80 };
add('border', 'parallel-top', { routedRelations: [lineA], frames: [rect] });
add('border', 'perpendicular-crossing', { routedRelations: [relation('a', 'b', [[50, -20], [50, 40]])], frames: [rect] });
add('border', 'short-border-is-still-invalid', { routedRelations: [relation('a', 'b', [[40, 0], [40.5, 0]])], frames: [rect] });
add('border', 'epsilon-contact-is-ignored', { routedRelations: [relation('a', 'b', [[40, 0], [40.00005, 0]])], frames: [rect] });
add('border', 'rounded-corner-is-trimmed', { routedRelations: [relation('a', 'b', [[0, 0], [12, 0]])], frames: [{ ...rect, radius: 16 }] });
add('border', 'rounded-top-straight-part', { routedRelations: [lineA], frames: [{ ...rect, radius: 16 }] });
add('border', 'overlapping-and-disjoint-segments-merge', { routedRelations: [{ ...lineA, segments: [{ start: [4, 0], end: [24, 0] }, { start: [14, 0], end: [34, 0] }, { start: [60, 0], end: [90, 0] }] }], frames: [rect] });
add('border', 'line-frame-vertical', { routedRelations: [relation('a', 'b', [[5, 0], [5, 60]])], frames: [{ kind: 'lifeline', shape: 'line', id: 'line', start: [5, 10], end: [5, 50] }] });
add('border', 'line-frame-fallback-coordinates', { routedRelations: [lineA], frames: [{ shape: 'line', x1: 10, y1: 0, x2: 30, y2: 0 }] });
add('border', 'line-frame-diagonal-is-not-axis-overlap', { routedRelations: [relation('a', 'b', [[0, 0], [40, 40]])], frames: [{ shape: 'line', start: [0, 0], end: [40, 40] }] });

const rhythmRoutes = [relation('a', 'b', [[0, 0], [6, 0], [6, 10], [40, 10], [40, 16]]), relation('c', 'd', [[0, 40], [8, 40], [8, 56], [24, 56]]), relation('e', 'f', [[0, 80], [5, 80], [24, 80]])];
add('rhythm', 'micro-interior-and-terminal-stubs', { routedRelations: rhythmRoutes });
add('rhythm', 'epsilon-floors-are-not-errors', { routedRelations: [relation('a', 'b', [[0, 0], [7.99995, 0], [7.99995, 15.99995], [32, 15.99995]])] });
add('rhythm', 'reversal-retains-segment', { routedRelations: [relation('a', 'b', [[0, 0], [40, 0], [32, 0], [32, 24]])] });
add('rhythm', 'single-short-route-is-source-stub', { routedRelations: [relation('a', 'b', [[0, 0], [5, 0]])] });
add('budget', 'all-counts-and-stretch', { routedRelations: rhythmRoutes });
add('budget', 'closed-loop-has-no-direct-stretch', { routedRelations: [relation('a', 'a', [[0, 0], [20, 0], [20, 20], [0, 20], [0, 0]])] });
add('budget', 'diagonal-budget-is-manhattan', { routedRelations: [relation('a', 'b', [[0, 0], [3, 4], [3, 10]])], bendsPerRelationship: 0, stretch: 1, segmentPx: 8, microSegmentPx: 7 });

const incoming = (x, extra = {}) => relation(`a${x}`, 'target', [[x, 0], [x, 40]], extra);
add('arrows', 'colliding-neighbor-tips', { routedRelations: [incoming(0), incoming(8)] }, true);
add('arrows', 'exact-marker-width-is-clear', { routedRelations: [incoming(0), incoming(10.5)] }, true);
add('arrows', 'different-approach-direction', { routedRelations: [incoming(0), relation('b', 'target', [[8, 80], [8, 40]])] }, true);
add('arrows', 'different-tip-depth', { routedRelations: [incoming(0), relation('b', 'target', [[8, 0], [8, 50]])] }, true);
add('arrows', 'width-and-emphasis-change-budget', { routedRelations: [incoming(0, { width: 3 }), incoming(12, { variant: 'emphasis' })] }, true);
add('arrows', 'junction-merges-one-arrowhead', { routedRelations: [incoming(0, { junction: 'bus' }), incoming(8, { junction: 'bus' })] }, true);
add('arrows', 'short-identical-target-trunk', { routedRelations: [relation('a', 'c', [[0, 0], [20, 0], [20, 20]]), relation('b', 'c', [[40, 0], [20, 0], [20, 20]])], allowShortWorkflowTrunks: true }, true);
add('arrows', 'long-identical-target-trunk-collides', { routedRelations: [relation('a', 'c', [[0, 0], [20, 0], [20, 40]]), relation('b', 'c', [[40, 0], [20, 0], [20, 40]])], allowShortWorkflowTrunks: true }, true);

const labelRect = { x: 10, y: 2, width: 10, height: 8 };
add('label-route', 'unrelated-route-near-label', { labels: [label(0, labelRect)], routedRelations: [{ ...lineA, relationIndex: 1 }], threshold: 4 });
add('label-route', 'own-route-exempt-by-index', { labels: [label(0, labelRect)], routedRelations: [{ ...lineA, relationIndex: 0 }], threshold: 4 });
add('label-route', 'own-route-exempt-by-identity', { labels: [label(7, labelRect, { id: 'ab', from: 'a', to: 'b' })], routedRelations: [lineA], threshold: 4 });
add('label-route', 'same-id-different-endpoints-is-unrelated', { labels: [label(7, labelRect, { id: 'ab' })], routedRelations: [lineA], threshold: 4 });
add('label-route', 'own-route-exempt-by-key', { labels: [label(7, labelRect, { key: 'route-key' })], routedRelations: [relation('a', 'b', [[0, 0], [40, 0]], { key: 'route-key' })], threshold: 4 });
add('label-route', 'different-keys-do-not-fall-back-to-identical-id', { labels: [label(7, labelRect, { id: 'ab', from: 'a', to: 'b', key: 'one' })], routedRelations: [relation('a', 'b', [[0, 0], [40, 0]], { id: 'ab', key: 'two' })], threshold: 4 });
add('label-route', 'intersection-length-and-nearest-source-segment', { labels: [label(7, { x: 10, y: -2, width: 10, height: 8 })], routedRelations: [relation('a', 'b', [[0, 0], [15, 0], [15, 20], [40, 20]])], threshold: 4 });
add('label-route', 'deduplicate-relations-and-labels', { labels: [label(7, labelRect), label(8, labelRect, { id: 'label-7' })], routedRelations: [lineA, lineA], threshold: 4 });
add('label-route', 'threshold-numeric-tolerance', { labels: [label(7, { ...labelRect, y: 3.99995 })], routedRelations: [lineA], threshold: 4 });
add('label-route', 'zero-threshold-has-no-issues', { labels: [label(7, { ...labelRect, y: -2 })], routedRelations: [lineA], threshold: 0 });
add('label-route', 'negative-label-dimensions-skipped', { labels: [label(7, { ...labelRect, width: -10 })], routedRelations: [lineA], threshold: 4 });
add('label-route', 'large-route-index-fallback-still-finds-nearest', { labels: [label(-1, labelRect)], routedRelations: [relation('a', 'b', [[-100000, 0], [100000, 0]])], threshold: 4 });

add('canvas', 'multiple-overflow-sides', { labels: [label(7, { x: -1.26, y: -2.34, width: 104, height: 84 })], viewBox: [100, 80] }, true);
add('canvas', 'nonzero-viewbox-origin', { labels: [label(7, { x: 9, y: 18, width: 104, height: 84 })], viewBox: [10, 20, 100, 80] }, true);
add('canvas', 'subpixel-tolerance-is-clear', { labels: [label(7, { x: -0.5, y: 0, width: 100.5, height: 80 })], viewBox: [100, 80] }, true);
add('canvas', 'negative-size-is-unknown', { labels: [label(7, { x: -10, y: 0, width: -20, height: 10 })], viewBox: [100, 80] }, true);

const crossPair = pair([[0, 0], [40, 40]], [[0, 40], [40, 0]]);
add('crossings', 'proper-interior-x', { routedRelations: crossPair });
add('crossings', 'shared-endpoint-default', { routedRelations: [crossPair[0], { ...crossPair[1], relation: { from: 'a', to: 'c' } }] });
add('crossings', 'shared-endpoint-opt-in', { routedRelations: [crossPair[0], { ...crossPair[1], relation: { from: 'a', to: 'c' } }], includeSharedEndpoints: true });
add('crossings', 'authored-forward-waypoint-hidden-without-merge', { routedRelations: pair([[0, 0], [20, 0], [40, 0]], [[20, -20], [20, 20]]) });
add('crossings', 'forward-waypoint-merge-retains-source-index', { routedRelations: pair([[0, 0], [10, 0], [20, 0], [40, 0]], [[25, -20], [25, 20]]), mergeForwardCollinearWaypoints: true });
add('crossings', 'waypoint-hit-prefers-first-authored-segment', { routedRelations: pair([[0, 0], [20, 0], [40, 0]], [[20, -20], [20, 20]]), mergeForwardCollinearWaypoints: true });
add('crossings', 'endpoint-touch-is-not-x', { routedRelations: pair([[0, 0], [20, 0]], [[20, 0], [20, 20]]) });
add('crossings', 'resolved-crossing', { routedRelations: crossPair, crossingResolved: true });
add('bend-contacts', 'bend-meets-interior', { left: [[0, 0], [20, 0], [20, 20]], right: [[10, -20], [30, 20]] }, true);
add('bend-contacts', 'other-endpoint-contact-is-exempt', { left: [[0, 0], [20, 0], [20, 20]], right: [[20, 0], [40, 0]] }, true);
add('bend-contacts', 'forward-waypoint-is-not-a-bend', { left: [[0, 0], [20, 0], [40, 0]], right: [[20, -20], [20, 20]] }, true);

const obstacles = [{ id: 'a', x: -10, y: -10, width: 10, height: 20 }, { id: 'b', x: 40, y: -10, width: 10, height: 20 }, { id: 'opaque', x: 10, y: -5, width: 10, height: 10 }];
add('obstacles', 'unrelated-opaque-node-hit', { routedRelations: [lineA], obstacles, clearance: 2 });
add('obstacles', 'endpoint-boxes-are-exempt', { routedRelations: [lineA], obstacles: obstacles.slice(0, 2), clearance: 2 });
add('obstacles', 'clearance-includes-border', { routedRelations: [relation('a', 'b', [[0, 7], [40, 7]])], obstacles, clearance: 2 });
add('obstacles', 'unknown-endpoint-is-skipped', { routedRelations: [relation('missing', 'b', [[0, 0], [40, 0]])], obstacles, clearance: 2 });
add('obstacles', 'first-authored-hit-segment', { routedRelations: [relation('a', 'b', [[0, 0], [15, 0], [15, 20], [40, 20]])], obstacles, clearance: 2 });
for (const [name, points, fromSide, toSide] of [
  ['horizontal-facing', [[0, 0], [40, 0]], 'right', 'left'],
  ['horizontal-backwards', [[0, 0], [40, 0]], 'left', 'right'],
  ['tangent-source', [[0, 0], [0, 20], [40, 20]], 'right', 'left'],
  ['vertical-facing', [[0, 0], [0, 40]], 'bottom', 'top'],
  ['normalized-stubs', [[0, 0], [0, 0], [0, 20], [0, 40]], 'bottom', 'top'],
  ['unknown-sides-add-no-contract', [[0, 0], [40, 40]], 'auto', ''],
]) add('endpoint-sides', name, { points, fromSide, toSide });

const results = [];
const pairPredicate = value => () => value === true;
for (const [contract, source] of sources) {
  const module = await import(pathToFileURL(path.join(root, source)).href);
  const sourceSha256 = createHash('sha256').update(await readFile(path.join(root, source))).digest('hex');
  for (const testCase of cases) {
    if (testCase.phoenixOnly && contract !== 'phoenix-v1') continue;
    const input = structuredClone(testCase.input);
    input.routedRelations?.forEach((entry, index) => { entry.relationIndex ??= index; });
    const before = JSON.stringify(input);
    const options = { ...input, includeSharedEndpoints: pairPredicate(input.includeSharedEndpoints), includeSharedEndpointCounterflow: pairPredicate(input.includeSharedEndpointCounterflow) };
    let expected;
    switch (testCase.operation) {
      case 'corridors': expected = module.collectAmbiguousCorridors(options).map(hit => ({ left: hit.left.relationIndex, right: hit.right.relationIndex, leftSegment: hit.leftSegment, rightSegment: hit.rightSegment, overlapLength: hit.overlapLength, start: hit.overlapStart, end: hit.overlapEnd })); break;
      case 'border': expected = module.collectBorderRuns(options).map(hit => ({ relation: hit.relationIndex, frame: hit.frameIndex, side: hit.side, segment: hit.segmentIndex, overlapLength: hit.overlapLength, start: hit.overlapStart, end: hit.overlapEnd })); break;
      case 'rhythm': expected = module.collectRouteRhythmIssues(options).map(hit => ({ code: hit.code, relation: hit.relationIndex, segment: hit.segmentIndex, position: hit.position, length: hit.length, start: hit.start, end: hit.end })); break;
      case 'budget': expected = module.routeBudgetMetrics(options); break;
      case 'arrows': expected = module.collectArrowheadCollisions(options).map(hit => ({ left: hit.left.relationIndex, right: hit.right.relationIndex, leftTip: hit.left.tip, rightTip: hit.right.tip, distance: hit.distance, minimum: hit.minimum })); break;
      case 'label-route': expected = module.collectLabelRouteClearance(options).map(hit => ({ labelRelation: hit.labelRelationIndex, otherRelation: hit.otherRelationIndex, segment: hit.segmentIndex, clearance: hit.clearance, intersectionLength: hit.intersectionLength, start: hit.start, end: hit.end })); break;
      case 'canvas': expected = module.collectLabelCanvasOverflow(options).map(hit => ({ relation: hit.relationIndex, viewBox: hit.viewBox, origin: hit.viewBoxOrigin, sides: hit.sides, overflowPx: hit.overflowPx })); break;
      case 'bend-contacts': expected = module.polylineBendContacts(input.left, input.right); break;
      case 'endpoint-sides': expected = module.routeHonorsEndpointSides(input.points, input.fromSide, input.toSide); break;
      case 'crossings': {
        const relations = input.routedRelations.map(r => r.relation);
        const endpointIds = new Set(relations.flatMap(r => [r.from, r.to]));
        const messages = module.cleanCrossingProblems({ ...options, relations, endpointIds, pathFor: r => input.routedRelations[relations.indexOf(r)], diagramType: 'workflow', relationCollection: 'relations', profile: 'showcase', profileIsAuthoritative: true, crossingResolved: () => input.crossingResolved === true });
        expected = messages.map(message => {
          const match = message.match(/relations\[(\d+)\].* crosses .*relations\[(\d+)\].* at \[([^\]]+)\] \(segments (\d+) and (\d+)\)/);
          assert(match, message);
          return { left: +match[1], right: +match[2], point: match[3].split(', ').map(Number), leftSegment: +match[4], rightSegment: +match[5] };
        });
        break;
      }
      case 'obstacles': {
        const relations = input.routedRelations.map(r => r.relation);
        const messages = module.cleanFlowProblems({ ...options, relations, pathFor: r => input.routedRelations[relations.indexOf(r)], diagramType: 'workflow', relationCollection: 'relations', obstacleKind: 'node' });
        expected = messages.map(message => {
          const match = message.match(/relations\[(\d+)\].* crosses node "([^"]+)" .* on segment (\d+) /);
          assert(match, message);
          return { relation: +match[1], obstacle: match[2], segment: +match[3] };
        });
        break;
      }
      default: throw new Error(testCase.operation);
    }
    assert.equal(JSON.stringify(input), before, `${contract}/${testCase.name}: source mutated input`);
    results.push({ contract, source, sourceSha256, operation: testCase.operation, name: testCase.name, input: testCase.input, expected });
  }
}
// border collectors preserve the caller-supplied index instead of inventing one.
// Always supply it for the oracle just as the Rust adapter does.
assert(results.some(testCase => testCase.operation === 'corridors' && testCase.expected.length));
await writeFile(new URL('./oracle.json', import.meta.url), `${JSON.stringify({ format: 1, cases: results }, null, 2)}\n`);
console.log(`Wrote ${results.length} frozen composition predicate oracle cases.`);
