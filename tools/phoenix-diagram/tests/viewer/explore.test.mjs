// Reader exploration state machine over the assembled template: Route,
// Reach, Semantic Lens, tree branches, deep links and presentation, driven
// through the actual Reader modules on native-SVG-shaped DOM fixtures. The
// DOM is deterministic and has no layout engine; real browser walkthroughs
// remain a separate gate.
import test from 'node:test';
import assert from 'node:assert/strict';
import { bootReader, bundledCatalogs, plain } from './fixtures/reader.mjs';
import * as fixtures from './fixtures/diagrams.mjs';

const en = bundledCatalogs.en;
const text = (key, values = {}) => {
  assert.ok(Object.hasOwn(en, key), `catalog key ${key}`);
  return en[key].replace(/\{([a-zA-Z0-9_]+)\}/g, (match, name) => Object.hasOwn(values, name) ? String(values[name]) : match);
};
const count = (key, value, values = {}) => text(key + (value === 1 ? '.one' : '.other'), { ...values, count: value });

async function open(svg, type, options = {}) {
  const env = bootReader({ svg, metadata: fixtures.metadata(type), ...options });
  await env.settle(60);
  assert.deepEqual(env.errors, [], 'the Reader boots without logged errors');
  return env;
}

const EXPLORATION_STATE = ['data-focus-active', 'data-reach-active', 'data-route-active', 'data-route-picking', 'data-route-journey', 'data-lens-active',
  'data-relationship-pin-active', 'data-relationship-preview-active', 'data-legend-preview-active'];
const EXPLORATION_MARKS = '[data-focus-match], [data-focus-selected], [data-reach-match], [data-reach-origin], [data-route-match], [data-route-start], [data-route-end], [data-route-candidate], '
  + '[data-lens-match], [data-lens-peer], [data-relationship-preview], [data-route-probe-overlay], [data-semantic-lens-overlay], [data-route-journey-overlay], [data-node-id][aria-pressed="true"]';

function assertFullDiagram(env, label) {
  assert.deepEqual(EXPLORATION_STATE.filter(name => env.svg.hasAttribute(name)), [], label);
  assert.equal(env.svg.querySelectorAll(EXPLORATION_MARKS).length, 0, label);
  assert.equal(env.api.focus.active(), null, label);
  assert.equal(env.api.focus.reachability(), null, label);
  assert.equal(env.api.routeProbe.active(), null, label);
  assert.equal(env.api.semanticLens.active(), null, label);
  assert.equal(env.$('#focus-chip').hidden, true, label);
  assert.equal(env.$('#route-probe').hidden, true, label);
}

function menuItems(env) {
  env.api.exportMenu.open();
  const items = Object.fromEntries(['route-share-card', 'reach-share-card'].map(action => {
    const item = env.$(`#export-menu button[data-action="${action}"]`);
    return [action, { hidden: item.hidden, disabled: item.disabled }];
  }));
  env.api.exportMenu.close();
  return items;
}

const routePanel = env => ({ state: env.$('#route-probe').getAttribute('data-state'), title: env.$('#route-probe-title').textContent, status: env.$('#route-probe-status').textContent });

test('Route from a node without outgoing arrows, to the same node and to an unreachable node keeps picking and invents no route', async () => {
  const env = await open(fixtures.graph(), 'architecture');
  const route = env.api.routeProbe;
  env.click('#btn-route-probe');
  assert.equal(route.active(), 'source');
  assert.equal(env.svg.getAttribute('data-route-picking'), 'source');
  assert.deepEqual(plain(route.finderContext().allowedIds), ['a', 'b', 'c'], 'only nodes with an outgoing arrow are offered as starts');

  env.click(env.node('d').querySelector('[data-graph-role="node-body"]'));
  assert.equal(route.active(), 'target');
  assert.equal(env.node('d').hasAttribute('data-route-start'), true);
  assert.equal(routePanel(env).status, text('viewer.route.noOutgoing'));
  assert.equal(env.svg.querySelectorAll('[data-route-candidate]').length, 0);
  assert.deepEqual(plain(route.finderContext().allowedIds), []);

  env.click(env.node('d'));
  assert.deepEqual(routePanel(env), { state: 'error', title: text('viewer.route.differentDestination'), status: text('viewer.route.distinct') });
  assert.equal(route.active(), 'target');
  assert.equal(route.result(), null);

  route.clear();
  assert.equal(route.begin({ source: 'b' }), true);
  assert.equal(routePanel(env).status, count('viewer.route.destination.count', 2));
  assert.deepEqual(env.$$('[data-route-candidate]').map(node => node.getAttribute('data-node-id')), ['c', 'd']);
  for (const [target, label] of [['a', 'Alpha'], ['e', 'Echo'], ['f', 'Foxtrot']]) {
    assert.equal(route.choose(target), false, `${target} is not reachable along the arrows from b`);
    assert.deepEqual(routePanel(env), { state: 'error', title: text('viewer.route.unreachable', { label }), status: text('viewer.route.unreachable.detail', { target: label, source: 'Bravo' }) });
    assert.equal(route.active(), 'target');
    assert.equal(route.result(), null);
    assert.equal(route.exportSnapshot(), null);
    assert.equal(env.svg.hasAttribute('data-route-active'), false);
    assert.equal(env.svg.querySelectorAll('[data-route-probe-overlay]').length, 0);
  }
  assert.equal(route.choose('missing'), false);
  assert.doesNotMatch(env.hash, /route=/);
  assert.deepEqual(menuItems(env)['route-share-card'], { hidden: true, disabled: true });
});

test('a traced Route is addressable and clearing it returns to the full diagram', async () => {
  const env = await open(fixtures.graph(), 'architecture');
  const route = env.api.routeProbe;
  env.click('#btn-route-probe');
  env.click(env.node('a'));
  env.click(env.node('d'));
  assert.deepEqual(plain(route.result()), { source: 'a', target: 'd', nodes: ['a', 'b', 'c', 'd'], hops: 3, journey: -1, playing: false });
  assert.equal(env.hash, '#route=a~d');
  assert.equal(env.svg.getAttribute('data-route-active'), 'a~d');
  assert.equal(routePanel(env).title, text('viewer.route.result.title', { source: 'Alpha', target: 'Delta' }));
  assert.equal(env.svg.querySelector('[data-route-probe-overlay]').children.length, 3);
  assert.deepEqual(env.$$('path[data-route-match]').map(edge => edge.getAttribute('data-edge-key')), ['0', '1', '2']);
  const snapshot = plain(route.exportSnapshot());
  assert.deepEqual(snapshot.nodeIds, ['a', 'b', 'c', 'd']);
  assert.deepEqual(snapshot.edges.map(edge => [edge.key, edge.id]), [['0', 'a-calls-b'], ['1', ''], ['2', 'c-writes-d']]);
  assert.deepEqual(snapshot.source, { id: 'a', label: 'Alpha' });
  assert.deepEqual(menuItems(env), { 'route-share-card': { hidden: false, disabled: false }, 'reach-share-card': { hidden: true, disabled: true } });

  env.click('#route-probe-clear');
  await env.settle(40);
  assertFullDiagram(env, 'after clearing the route');
  assert.equal(env.hash, '');
  assert.equal(route.exportSnapshot(), null);
  assert.deepEqual(menuItems(env)['route-share-card'], { hidden: true, disabled: true });
  assert.equal(env.api.view.state().mode, 'overview');
});

test('Reach reports empty directions as disabled controls with their no-upstream/no-downstream text', async () => {
  const env = await open(fixtures.graph(), 'architecture');
  const focus = env.api.focus;
  const reachControls = () => ['upstream', 'downstream'].map(direction => {
    const button = env.$(`#btn-reach-${direction}`);
    return { disabled: button.disabled, label: button.getAttribute('aria-label'), count: env.$(`#focus-reach-${direction}-count`).textContent };
  });

  env.click(env.node('a'));
  assert.equal(focus.active(), 'a');
  assert.equal(env.hash, '#focus=a');
  assert.deepEqual(reachControls(), [
    { disabled: true, label: text('viewer.passport.reach.noUpstream'), count: '0' },
    { disabled: false, label: count('viewer.passport.reach.downstream', 4), count: '4' },
  ]);
  assert.equal(focus.reach('upstream'), false);
  assert.equal(focus.reachabilitySnapshot(), null);

  env.click('#btn-reach-downstream');
  assert.equal(env.hash, '#focus=a&reach=downstream');
  assert.equal(env.svg.getAttribute('data-reach-active'), 'downstream');
  assert.deepEqual(Object.fromEntries(env.$$('[data-node-id][data-reach-match]').map(node => [node.getAttribute('data-node-id'), node.getAttribute('data-reach-depth')])),
    { a: '0', b: '1', c: '2', d: '3', e: '1' });
  const snapshot = plain(focus.reachabilitySnapshot());
  assert.deepEqual([snapshot.direction, snapshot.origin, snapshot.nodeIds, snapshot.maxDepth], ['downstream', { id: 'a', label: 'Alpha' }, ['a', 'b', 'e', 'c', 'd'], 3]);
  assert.deepEqual(menuItems(env)['reach-share-card'], { hidden: false, disabled: false });
  env.click('#btn-reach-downstream');
  assert.equal(env.hash, '#focus=a', 'the same direction toggles reach off and keeps the focus');
  assert.equal(focus.reachability(), null);

  focus.set('d');
  assert.deepEqual(reachControls(), [
    { disabled: false, label: count('viewer.passport.reach.upstream', 3), count: '3' },
    { disabled: true, label: text('viewer.passport.reach.noDownstream'), count: '0' },
  ]);
  assert.equal(focus.reach('downstream'), false);

  focus.set('f');
  assert.deepEqual(reachControls(), [
    { disabled: true, label: text('viewer.passport.reach.noUpstream'), count: '0' },
    { disabled: true, label: text('viewer.passport.reach.noDownstream'), count: '0' },
  ]);
  assert.equal(focus.reach('upstream'), false);
  assert.equal(focus.reach('downstream'), false);
  assert.equal(env.$('#relationship-lens-list').textContent, text('viewer.passport.relationship.none'));
  assert.equal(env.svg.hasAttribute('data-reach-active'), false);
  assert.deepEqual(menuItems(env)['reach-share-card'], { hidden: true, disabled: true });
});

for (const [type, build] of [['timeline', fixtures.timeline], ['waterfall', fixtures.waterfall]]) {
  test(`${type} has no authored relationship: Route reports no outgoing arrows, Reach is disabled and nothing is invented`, async () => {
    const env = await open(build(), type);
    const ids = env.svg.querySelectorAll('[data-node-id]').map(node => node.getAttribute('data-node-id'));
    assert.equal(ids.length, 3);
    assert.equal(env.svg.querySelectorAll('[data-edge-from], [data-edge-to]').length, 0);
    assert.equal(env.svg.querySelectorAll('[data-relationship-hit-overlay]').length, 0);
    assert.deepEqual(plain(env.api.guide.facts()), { nodes: 3, relationships: 0 });

    const route = env.api.routeProbe;
    assert.equal(route.begin(), true);
    assert.deepEqual(plain(route.finderContext().allowedIds), []);
    env.click(env.node(ids[0]));
    assert.equal(route.active(), 'target');
    assert.equal(routePanel(env).status, text('viewer.route.noOutgoing'));
    assert.equal(env.svg.querySelectorAll('[data-route-candidate]').length, 0);
    env.click(env.node(ids[1]));
    assert.equal(routePanel(env).state, 'error');
    assert.equal(route.result(), null);
    assert.equal(route.exportSnapshot(), null);
    route.clear();

    for (const id of ids) {
      env.api.focus.set(id);
      for (const [direction, key] of [['upstream', 'viewer.passport.reach.noUpstream'], ['downstream', 'viewer.passport.reach.noDownstream']]) {
        const button = env.$(`#btn-reach-${direction}`);
        assert.equal(button.disabled, true, `${id} ${direction}`);
        assert.equal(button.getAttribute('aria-label'), text(key));
        assert.equal(env.api.focus.reach(direction), false);
      }
      assert.equal(env.$('#relationship-lens-list').textContent, text('viewer.passport.relationship.none'));
      assert.equal(env.$('#focus-summary').textContent, text('viewer.passport.relationship.summary', { out: 0, in: 0, loops: '' }));
      assert.equal(env.api.focus.reachabilitySnapshot(), null);
    }
    assert.deepEqual(menuItems(env), { 'route-share-card': { hidden: true, disabled: true }, 'reach-share-card': { hidden: true, disabled: true } });
    env.api.focus.clear();
    assertFullDiagram(env, `${type} after exploration`);
    assert.equal(env.svg.querySelectorAll('[data-edge-from], [data-reach-match], [data-route-probe-overlay]').length, 0, 'no relationship was created by the Reader');
  });
}

test('Semantic Lens compares at most two kinds and rejects a third kind', async () => {
  const env = await open(fixtures.graph(), 'architecture');
  const lens = env.api.semanticLens;
  env.click('#btn-semantic-lens');
  assert.equal(lens.isOpen(), true);
  const kindButton = kind => env.$(`#semantic-lens-kinds [data-kind="${kind}"]`);
  assert.deepEqual(env.$$('#semantic-lens-kinds [data-kind]').map(button => button.getAttribute('data-kind')).sort(), ['backend', 'cloud', 'database', 'external', 'frontend']);
  env.click(kindButton('backend'));
  env.click(kindButton('database'));
  assert.deepEqual(plain(lens.active()), ['backend', 'database']);
  assert.equal(env.hash, '#lens=backend~database');
  assert.deepEqual(['cloud', 'external', 'frontend'].map(kind => kindButton(kind).disabled), [true, true, true]);
  env.click(kindButton('cloud'));
  assert.equal(lens.select('cloud'), false);
  assert.equal(lens.select('bogus'), false);
  assert.deepEqual(plain(lens.active()), ['backend', 'database'], 'a third kind never joins the comparison');
  assert.equal(env.svg.getAttribute('data-lens-active'), 'backend database');
  assert.equal(env.$('#semantic-lens-status').textContent, count('viewer.lens.compare', 1, { first: 'Backend', second: 'Database', forward: 1, reverse: 0 }));
  assert.deepEqual(env.$$('path[data-lens-match]').map(edge => edge.getAttribute('data-edge-key')), ['2']);

  env.click(kindButton('backend'));
  assert.deepEqual(plain(lens.active()), ['database']);
  env.click('#semantic-lens-clear');
  assert.equal(lens.active(), null);
  assert.equal(env.hash, '');
  assert.equal(env.svg.hasAttribute('data-lens-active'), false);
});

for (const [hash, expected] of [['#lens=bogus', null], ['#lens=backend~database~cloud', ['backend', 'database']], ['#lens=bogus~backend~backend', ['backend']], ['#lens=', null]]) {
  test(`a lens deep link ${hash} applies only valid, distinct kinds up to two`, async () => {
    const env = await open(fixtures.graph(), 'architecture', { hash });
    assert.deepEqual(plain(env.api.semanticLens.active()), expected);
    if (expected) assert.equal(env.svg.getAttribute('data-lens-active'), expected.join(' '));
    else assertFullDiagram(env, hash);
  });
}

test('route and reach snapshots fail closed on node, edge and relation identity conflicts', async t => {
  const duplicateNode = { id: 'c', label: 'Charlie copy', kind: 'backend', x: 440, y: 400 };
  const cases = {
    'duplicate node id': { nodes: [...fixtures.GRAPH_NODES, duplicateNode] },
    'two drawable fragments share an edge key': { extraBody: fixtures.edge({ from: 'b', to: 'c', key: '1', d: 'M 360 80 L 440 80' }) },
    'fragments of one key disagree on the relation id': { extraBody: '<g data-edge-from="b" data-edge-to="c" data-edge-key="1" data-edge-id="other"><rect x="380" y="50" width="20" height="10"/></g>' },
    'relationship without a key': { edges: fixtures.GRAPH_EDGES.map(edge => edge.key === '1' ? { ...edge, key: undefined } : edge) },
  };
  for (const [name, options] of Object.entries(cases)) {
    await t.test(name, async () => {
      const env = await open(fixtures.graph(options), 'architecture');
      assert.equal(env.api.routeProbe.begin({ source: 'a' }), true);
      assert.equal(env.api.routeProbe.choose('d'), true, 'the live route remains a reading aid');
      assert.equal(env.api.routeProbe.exportSnapshot(), null);
      assert.deepEqual(menuItems(env)['route-share-card'], { hidden: true, disabled: true });
      env.api.routeProbe.clear();
      env.api.focus.set('a');
      assert.equal(env.api.focus.reach('downstream'), true);
      assert.equal(env.api.focus.reachabilitySnapshot(), null);
      assert.deepEqual(menuItems(env)['reach-share-card'], { hidden: true, disabled: true });
    });
  }
  await t.test('duplicate relation id cannot be deep linked', async () => {
    const edges = fixtures.GRAPH_EDGES.map(edge => edge.key === '3' ? { ...edge, id: 'a-calls-b' } : edge);
    const env = await open(fixtures.graph({ edges }), 'architecture', { hash: '#relation=a-calls-b' });
    assert.equal(env.api.focus.inspectRelationshipById('a-calls-b'), false);
    assert.equal(env.api.focus.relationship(), null);
    assertFullDiagram(env, 'conflicting relation id');
  });
});

test('tree branches collapse in place, follow the keyboard, reveal selected descendants and expand all', async () => {
  const env = await open(fixtures.tree(), 'tree');
  const tree = env.api.treeBranches;
  const hiddenNodes = () => env.svg.querySelectorAll('[data-node-id][data-tree-hidden]').map(node => node.getAttribute('data-node-id'));
  const hiddenEdges = () => env.svg.querySelectorAll('path[data-edge-to][data-tree-hidden]').map(edge => edge.getAttribute('data-edge-to'));
  const authored = '[data-node-id], [data-edge-from], [data-tree-toggle]';
  const toggle = id => env.$(`[data-tree-toggle="${id}"]`);
  const geometry = () => env.$$('[data-graph-role="node-body"]').map(body => ['x', 'y', 'width', 'height'].map(name => body.getAttribute(name)).join(' '));
  const before = geometry();

  assert.equal(tree.active, true);
  assert.deepEqual(plain(tree.collapsedIds()), ['ops'], 'an authored collapsed branch starts collapsed');
  assert.deepEqual(hiddenNodes(), ['refunds']);
  assert.deepEqual(hiddenEdges(), ['refunds']);
  assert.equal(toggle('ops').getAttribute('aria-expanded'), 'false');
  assert.equal(toggle('ops').getAttribute('aria-label'), 'Expand Operations (1 hidden)');
  assert.equal(env.svg.hasAttribute('data-tree-any-collapsed'), true);

  env.click(toggle('payments'));
  assert.deepEqual(plain(tree.collapsedIds()).sort(), ['ops', 'payments']);
  assert.deepEqual(hiddenNodes(), ['card', 'wallet', 'refunds']);
  assert.deepEqual(hiddenEdges(), ['card', 'wallet', 'refunds']);
  assert.equal(toggle('payments').getAttribute('aria-label'), 'Expand Payments (2 hidden)');
  assert.equal(env.node('payments').hasAttribute('data-tree-collapsed'), true);
  assert.equal(env.document.activeElement, toggle('payments'));
  assert.deepEqual(geometry(), before, 'collapsing never moves what stays visible');

  env.key('Enter', toggle('payments'));
  assert.deepEqual(plain(tree.collapsedIds()), ['ops']);
  env.key('ArrowLeft', env.node('payments'));
  assert.deepEqual(plain(tree.collapsedIds()).sort(), ['ops', 'payments']);
  env.key('ArrowRight', env.node('payments'));
  assert.deepEqual(plain(tree.collapsedIds()), ['ops']);

  assert.equal(env.api.focus.set('refunds'), true);
  assert.deepEqual(plain(tree.collapsedIds()), [], 'selecting a hidden node reveals its ancestors');
  assert.deepEqual(hiddenNodes(), []);
  env.api.focus.clear();

  tree.collapse('ops');
  env.node('refunds').setAttribute('aria-pressed', 'true');
  await env.settle(0);
  assert.deepEqual(plain(tree.collapsedIds()), [], 'a direct pressed-state change also reveals the node');
  env.node('refunds').setAttribute('aria-pressed', 'false');

  tree.collapse('ops'); tree.collapse('payments');
  env.click('[data-tree-expand-all]');
  assert.deepEqual(plain(tree.collapsedIds()), []);
  assert.equal(env.svg.hasAttribute('data-tree-any-collapsed'), false);
  assert.equal(env.svg.querySelectorAll(authored).filter(element => element.hasAttribute('data-tree-hidden') || element.hasAttribute('data-tree-collapsed')).length, 0);
  assert.deepEqual(env.svg.querySelectorAll('[data-tree-toggle]').map(element => element.getAttribute('aria-expanded')), ['true', 'true', 'true']);
  assert.equal(env.document.activeElement, toggle('root'));
  assert.deepEqual(geometry(), before);
});

// The relationship explorer clones each relationship path, including its
// tree ancestry, after Tree Branches has started.
test('relationship hit targets follow their branch through collapse and expand', async () => {
  const env = await open(fixtures.tree(), 'tree');
  const hitTarget = to => env.svg.querySelector(`[data-relationship-hit-key][data-relationship-to="${to}"]`);
  const hidden = target => target.hasAttribute('data-tree-hidden') || target.children.filter(rail => rail.localName === 'path').every(rail => rail.hasAttribute('data-tree-hidden'));
  const visible = target => !target.hasAttribute('data-tree-hidden') && target.children.every(rail => !rail.hasAttribute('data-tree-hidden'));
  assert.equal(hidden(hitTarget('refunds')), true, 'the relationship into a collapsed branch is not reachable');
  env.click('[data-tree-toggle="ops"]');
  assert.equal(visible(hitTarget('refunds')), true, 'expanding the branch restores its relationship');
  env.click('[data-tree-toggle="payments"]');
  assert.deepEqual(['card', 'wallet'].map(id => hidden(hitTarget(id))), [true, true]);
  env.click('[data-tree-expand-all]');
  assert.equal(env.svg.querySelectorAll('[data-tree-hidden], [data-tree-collapsed]').length, 0);
});

test('a tree deep link reveals collapsed ancestors and removing it returns to the full diagram', async () => {
  const env = await open(fixtures.tree(), 'tree', { hash: '#focus=refunds' });
  assert.equal(env.api.focus.active(), 'refunds');
  assert.deepEqual(plain(env.api.treeBranches.collapsedIds()), []);
  assert.equal(env.node('refunds').hasAttribute('data-tree-hidden'), false);
  assert.equal(env.$('#focus-chip').hidden, false);
  assert.equal(env.api.view.state().mode, 'semantic', 'the camera follows the linked focus');
  await env.navigate('');
  assertFullDiagram(env, 'tree hash removed');
  assert.equal(env.api.view.state().mode, 'overview');
});

test('deep links restore focus, reach, relation, route, lens and camera; clearing each returns to the full diagram', async t => {
  const cases = [
    ['#focus=b', env => { assert.equal(env.api.focus.active(), 'b'); assert.equal(env.$('#focus-chip').hidden, false); }],
    ['#focus=a&reach=downstream', env => { assert.equal(env.api.focus.reachability().direction, 'downstream'); assert.ok(env.api.focus.reachabilitySnapshot()); }],
    ['#relation=c-writes-d', env => { assert.deepEqual(plain(env.api.focus.relationship()), { id: 'c-writes-d', key: '2', from: 'c', to: 'd', label: 'writes' }); assert.equal(env.svg.getAttribute('data-relationship-pin-active'), '2'); }],
    ['#route=a~d', env => { assert.deepEqual(plain(env.api.routeProbe.result().nodes), ['a', 'b', 'c', 'd']); assert.ok(env.api.routeProbe.exportSnapshot()); }],
    ['#lens=backend', env => { assert.deepEqual(plain(env.api.semanticLens.active()), ['backend']); }],
  ];
  for (const [hash, check] of cases) {
    await t.test(hash, async () => {
      const env = await open(fixtures.graph(), 'architecture', { hash });
      check(env);
      assert.equal(env.api.view.state().mode, hash.startsWith('#lens') ? 'overview' : 'semantic', 'the camera frames a linked selection or route');
      await env.navigate('');
      assertFullDiagram(env, `${hash} removed`);
      assert.equal(env.api.view.state().mode, 'overview');
    });
  }
  await t.test('removing reach from the hash keeps the focus', async () => {
    const env = await open(fixtures.graph(), 'architecture', { hash: '#focus=a&reach=downstream' });
    await env.navigate('#focus=a');
    assert.equal(env.api.focus.active(), 'a');
    assert.equal(env.api.focus.reachability(), null);
    assert.equal(env.svg.hasAttribute('data-reach-active'), false);
  });
  await t.test('clearing from the controls also clears the hash', async () => {
    const env = await open(fixtures.graph(), 'architecture', { hash: '#focus=a&reach=downstream' });
    env.click('#btn-focus-clear');
    await env.settle(40);
    assertFullDiagram(env, 'focus cleared');
    assert.equal(env.hash, '');
  });
  await t.test('relation= addresses only relationships with an explicit id', async () => {
    const env = await open(fixtures.graph(), 'architecture', { hash: '#relation=1' });
    assert.equal(env.api.focus.relationship(), null);
    assertFullDiagram(env, 'key-only relation link');
  });
  await t.test('invalid routes are not restored', async () => {
    const unreachable = await open(fixtures.graph(), 'architecture', { hash: '#route=a~f' });
    assert.equal(unreachable.api.routeProbe.result(), null);
    assert.equal(unreachable.api.routeProbe.exportSnapshot(), null);
    for (const hash of ['#route=a~missing', '#route=a', '#route=a~b~c']) {
      const env = await open(fixtures.graph(), 'architecture', { hash });
      assertFullDiagram(env, hash);
    }
  });
  await t.test('a hash change switches from one state to another without leaking the first', async () => {
    const env = await open(fixtures.graph(), 'architecture', { hash: '#focus=b' });
    await env.navigate('#route=a~d');
    assert.equal(env.api.focus.active(), null);
    assert.deepEqual(plain(env.api.routeProbe.result().nodes), ['a', 'b', 'c', 'd']);
    assert.equal(env.svg.querySelectorAll('[data-focus-match], [data-focus-selected]').length, 0);
  });
});

test('query deep links set theme, embed and presentation before the Reader starts', async () => {
  const light = await open(fixtures.graph(), 'architecture', { search: '?theme=light' });
  assert.equal(light.html.getAttribute('data-theme'), 'light');
  const embedded = await open(fixtures.graph(), 'architecture', { search: '?embed=1', hash: '#route=a~d' });
  assert.equal(embedded.html.getAttribute('data-embed'), 'true');
  assert.equal(embedded.api.routeProbe.active(), null, 'embed ignores interactive route links');
  assert.equal(embedded.api.routeProbe.begin(), false);
  const embeddedRelation = await open(fixtures.graph(), 'architecture', { search: '?embed=1', hash: '#relation=c-writes-d' });
  assertFullDiagram(embeddedRelation, 'embed relation link');
  const embeddedFocus = await open(fixtures.graph(), 'architecture', { search: '?embed=1', hash: '#focus=a&reach=downstream' });
  assert.equal(embeddedFocus.api.focus.active(), 'a');
  assert.equal(embeddedFocus.api.focus.reachability().direction, 'downstream');
  const presenting = await open(fixtures.graph(), 'architecture', { search: '?present=1&theme=dark' });
  assert.equal(presenting.api.presentation.active(), true);
  assert.equal(presenting.html.getAttribute('data-theme'), 'dark');
  assert.equal(presenting.$('#btn-present').getAttribute('aria-pressed'), 'true');
});

test('presentation enters and exits from its control, its shortcut and Escape after temporary state', async () => {
  const env = await open(fixtures.graph(), 'architecture', { search: '?theme=light' });
  const presentation = env.api.presentation;
  const authored = env.svg.outerHTML;
  const control = () => ({ pressed: env.$('#btn-present').getAttribute('aria-pressed'), label: env.$('#present-label').textContent, aria: env.$('#btn-present').getAttribute('aria-label') });

  env.click('#btn-present');
  assert.equal(presentation.active(), true);
  assert.equal(env.html.getAttribute('data-present'), 'true');
  assert.equal(new URL(env.href).search, '?theme=light&present=1');
  assert.deepEqual(control(), { pressed: 'true', label: text('viewer.present.exit.label'), aria: text('viewer.present.exit') });
  assert.equal(env.svg.outerHTML, authored, 'presentation changes layout state only');
  env.click('#btn-present');
  assert.equal(presentation.active(), false);
  assert.equal(env.html.hasAttribute('data-present'), false);
  assert.equal(new URL(env.href).search, '?theme=light');
  assert.deepEqual(control(), { pressed: 'false', label: text('viewer.present.present'), aria: text('viewer.present.enter') });

  env.key('f', env.document.body);
  assert.equal(presentation.active(), true);
  env.api.view.zoomIn();
  env.api.focus.set('b');
  await env.settle(40);
  env.key('Escape', env.document.body);
  assert.equal(env.api.focus.active(), null, 'Escape clears the focus before leaving the stage');
  assert.equal(presentation.active(), true);
  for (let presses = 0; presentation.active() && presses < 3; presses++) env.key('Escape', env.document.body);
  assert.equal(presentation.active(), false);
  await env.settle(40);
  assertFullDiagram(env, 'after leaving the stage');
  assert.equal(env.api.view.state().mode, 'overview');
  assert.equal(env.api.view.state().scale, 1);
  assert.equal(new URL(env.href).searchParams.has('present'), false);
});
