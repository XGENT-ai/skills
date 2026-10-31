// Maintenance-only real-browser exploration acceptance for one artifact: lens,
// Route, Reach, tree folding, overview, presentation, deep links, keyboard and
// details, driven by trusted CDP input. The plan comes from the artifact's own
// native metadata, never from a fixture.
import { setTimeout as delay } from 'node:timers/promises';
import { sendReaderKey } from './export-reader.mjs';

export const REQUIRED_EXPLORATION = Object.freeze(['lens', 'route-path', 'route-none', 'reach', 'reach-empty', 'tree', 'overview', 'presentation', 'deep-link', 'keyboard', 'details']);
function requireThat(condition, message) { if (!condition) throw new Error('Exploration: ' + message); }

/** Pairs and nodes the exploration needs, derived from authored relations only.
 * Nodes inside an initially folded tree branch are not painted, so they are not chosen. */
export function explorationPlan(metadata, hidden = []) {
  const type = metadata.diagramType, all = (metadata.graph?.nodes || []).map(node => node.id), nodes = all.filter(id => !hidden.includes(id));
  const relations = (metadata.graph?.relations || []).filter(relation => relation.from !== undefined).map(relation => [relation.from, relation.to]);
  const out = new Map(all.map(id => [id, []])), into = new Map(all.map(id => [id, []]));
  for (const [from, to] of relations) { out.get(from)?.push(to); into.get(to)?.push(from); }
  const reachable = start => { const seen = new Set([start]), queue = [start]; while (queue.length) for (const next of out.get(queue.shift()) || []) if (!seen.has(next)) { seen.add(next); queue.push(next); } seen.delete(start); return seen; };
  let pathPair = null, noPathPair = null;
  for (const source of nodes) {
    const targets = new Set([...reachable(source)].filter(id => nodes.includes(id)));
    if (!pathPair && targets.size) pathPair = [source, [...targets].at(-1)];
    const blocked = nodes.find(target => target !== source && !targets.has(target));
    if (!noPathPair && out.get(source).length && blocked) noPathPair = [source, blocked];
  }
  return { type, nodes, relations: relations.length, pathPair, noPathPair, noOutgoing: nodes.find(id => !out.get(id).length) || null,
    reachNode: nodes.find(id => out.get(id).length) || null, noUpstream: nodes.find(id => !into.get(id).length) || null,
    noDownstream: nodes.find(id => !out.get(id).length) || null, hidden,
    tree: type === 'tree' ? nodes.find(id => out.get(id).length) || null : null };
}

// Page-side state probe; it reads Reader state, never authored geometry.
export function exploreSnapshot(options = false) {
  const api = PhoenixDiagram, svg = document.querySelector('.diagram-container > svg');
  const button = id => { const element = document.getElementById(id); return element ? { disabled: !!element.disabled, hidden: !!element.hidden, label: element.getAttribute('aria-label') || '' } : null; };
  const state = {
    hash: location.hash, present: document.documentElement.getAttribute('data-present'),
    focus: api.focus.active(), reach: api.focus.reachabilitySnapshot ? api.focus.reachabilitySnapshot() : null,
    route: { mode: api.routeProbe.active(), result: api.routeProbe.result(), startId: svg.querySelector('g[data-node-id][data-route-start]')?.getAttribute('data-node-id') ?? null, status: (document.getElementById('route-probe-status') || {}).textContent || '',
      noOutgoing: typeof phoenixDiagramI18nData === 'object' ? phoenixDiagramI18nData.messages['viewer.route.noOutgoing'] : null,
      distinct: typeof phoenixDiagramI18nData === 'object' ? phoenixDiagramI18nData.messages['viewer.route.distinct'] : null },
    lens: { open: api.semanticLens.isOpen(), active: api.semanticLens.active(), kinds: api.semanticLens.kinds(), matches: svg.querySelectorAll('[data-lens-match]').length },
    radar: { open: api.radar.isOpen(), count: api.radar.count() },
    tree: { active: api.treeBranches.active === true, collapsed: api.treeBranches.collapsedIds ? api.treeBranches.collapsedIds() : [], hidden: svg.querySelectorAll('[data-tree-hidden]').length },
    reachButtons: { upstream: button('btn-reach-upstream'), downstream: button('btn-reach-downstream') },
    details: { hidden: document.getElementById('focus-chip').hidden },
    active: { id: document.activeElement?.id || '', nodeId: document.activeElement?.getAttribute?.('data-node-id') ?? null },
    nodes: svg.querySelectorAll('g[data-node-id]').length,
  };
  if (options?.targetInput) state.route.targetInput = __phoenixDiagramExportProbe.readerInputs.filter(input => input.type === 'click' && input.trusted === true).at(-1) || null;
  return options === true ? { state, inputStart: __phoenixDiagramExportProbe.readerInputs.length } : state;
}

export async function runExplore(page, metadata, onProgress) {
  const report = page.report.explore = { status: 'failed', plan: null, observations: [] },
    observations = report.observations, snapshot = () => page.evaluate(exploreSnapshot);
  const key = (name, code) => sendReaderKey(page.send, name, code);
  const body = id => `.diagram-container svg g[data-node-id="${id}"] [data-graph-role="node-body"]`;
  async function observe(name, action) {
    onProgress?.({ stage: 'explore', name });
    const sample = { name, status: 'failed' };
    observations.push(sample);
    const clickStart = page.clicks.length, inputEpoch = page.documentEpoch;
    let inputStart, afterAttempted = false;
    const inputs = () => page.evaluate(start => __phoenixDiagramExportProbe.readerInputs.slice(start), inputStart);
    const errorRecord = error => ({ code: error.code || 'acceptance/failed', message: error.message || String(error),
      ...(error.code === 'exploration/route-target-input' ? { targetInput: error.targetInput } : {}) });
    try {
      const initial = await page.evaluate(exploreSnapshot, true);
      sample.before = initial.state; inputStart = initial.inputStart;
      sample.result = await action();
      afterAttempted = true; sample.after = await snapshot();
      sample.status = 'pass';
    } catch (error) {
      sample.error = errorRecord(error);
      sample.clicks = page.clicks.slice(clickStart);
      const diagnostic = async (stage, collect) => {
        try { await collect(); } catch (failure) { (sample.diagnostics ||= []).push({ stage, ...errorRecord(failure) }); }
      };
      const documentChanged = () => {
        const afterEpoch = page.documentEpoch;
        if (afterEpoch === inputEpoch) return false;
        (sample.diagnostics ||= []).push({ stage: 'inputs', code: 'exploration/document-changed',
          message: 'Exploration: navigation may have changed the Reader document; step inputs unavailable', beforeEpoch: inputEpoch, afterEpoch });
        return true;
      };
      if (!afterAttempted) await diagnostic('after', async () => { sample.after = await snapshot(); });
      if (inputStart !== undefined && !documentChanged()) await diagnostic('inputs', async () => {
        const collected = await inputs(); if (!documentChanged()) sample.inputs = collected;
      });
      throw error;
    }
  }
  const settle = () => delay(250);
  // Polls for a Reader state that a loaded machine may paint late.
  const until = async (check, ms = 3000) => { let state = await snapshot(); for (const end = Date.now() + ms; !check(state) && Date.now() < end; state = await snapshot()) await delay(100); return state; };
  await page.load('dark');
  const full = await snapshot();
  const plan = report.plan = explorationPlan(metadata, await page.evaluate(() => Array.from(document.querySelectorAll('.diagram-container > svg g[data-node-id][data-tree-hidden]'), node => node.getAttribute('data-node-id'))));
  await observe('lens', async () => {
    await page.click('#btn-semantic-lens'); await settle();
    const opened = await snapshot(), kind = opened.lens.kinds[0]?.id;
    requireThat(opened.lens.open && kind, 'semantic lens did not open with a kind');
    await page.click(`#semantic-lens-kinds button[data-kind="${kind}"]`); await settle();
    const selected = await snapshot();
    requireThat(JSON.stringify(selected.lens.active) === JSON.stringify([kind]) && selected.lens.matches > 0 && /lens=/.test(selected.hash), 'selecting a kind did not filter by it');
    await page.click('#semantic-lens-clear'); await settle();
    const cleared = await snapshot();
    requireThat(cleared.lens.active === null && !/lens=/.test(cleared.hash), 'clearing the lens did not restore the full diagram');
    await key('Escape', 'Escape'); await settle();
    return { kind, selected: selected.lens, cleared: cleared.lens };
  });
  if (plan.pathPair) {
    await observe('route-path', async () => {
      await page.click('#btn-route-probe'); await settle();
      await page.click(body(plan.pathPair[0])); await settle();
      await page.click(body(plan.pathPair[1])); await settle();
      const routed = await snapshot();
      requireThat(routed.route.mode === 'result' && routed.route.result?.source === plan.pathPair[0] && routed.route.result.target === plan.pathPair[1] && routed.route.result.hops >= 1 && routed.hash.includes('route='), 'a reachable route was not found');
      return { pair: plan.pathPair, result: routed.route.result, hash: routed.hash };
    });
  }
  await observe('route-none', async () => {
    const before = await snapshot();
    if (before.route.mode === 'result') { await page.click('#route-probe-clear'); await settle(); }
    if ((await snapshot()).route.mode === null) { await page.click('#btn-route-probe'); await settle(); }
    // A strongly connected diagram has no authored dead end; choosing the start
    // again is then its only no-route outcome. The docked route panel may cover
    // a node in source mode, so that check starts from one a reader can click.
    const source = plan.noPathPair ? plan.noPathPair[0] : plan.noOutgoing || await page.evaluate(ids => ids.find(id => {
      const node = document.querySelector('.diagram-container svg g[data-node-id="' + id + '"] [data-graph-role="node-body"]');
      if (!node) return false;
      node.scrollIntoView({ block: 'center', inline: 'center' });
      const rect = node.getBoundingClientRect(), hit = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
      return hit?.closest('[data-node-id]')?.getAttribute('data-node-id') === id;
    }) || ids[0], plan.nodes);
    await page.click(body(source)); await settle();
    const after = await snapshot();
    if (after.route.mode !== 'target' || after.route.startId !== source) {
      const error = new Error('Exploration: route-none did not select its planned source');
      error.code = 'exploration/route-source'; throw error;
    }
    if (!plan.noPathPair && !plan.noOutgoing) {
      await page.click(body(source)); await settle();
      const same = await snapshot();
      requireThat(same.route.result === null && same.route.mode === 'target' && same.route.status === same.route.distinct && same.route.distinct, 'choosing the start as destination was accepted');
      await key('Escape', 'Escape'); await settle();
      return { source, samePoint: true, status: same.route.status };
    }
    if (plan.noPathPair) {
      await page.click(body(plan.noPathPair[1])); await settle();
      const blocked = await page.evaluate(exploreSnapshot, { targetInput: true });
      if (blocked.route.targetInput?.target?.nodeId !== plan.noPathPair[1]) {
        const error = new Error('Exploration: route-none click did not hit its planned target');
        error.code = 'exploration/route-target-input'; error.targetInput = blocked.route.targetInput; throw error;
      }
      requireThat(blocked.route.result === null && blocked.route.mode === 'target', 'an unreachable target was accepted');
      requireThat(blocked.nodes === full.nodes, 'the route state changed authored nodes');
      await key('Escape', 'Escape'); await settle();
      return { pair: plan.noPathPair, status: blocked.route.status, targetInput: blocked.route.targetInput };
    }
    requireThat(after.route.result === null && after.route.status === after.route.noOutgoing && after.route.noOutgoing, 'a node without outgoing arrows did not report it');
    await key('Escape', 'Escape'); await settle();
    return { source, status: after.route.status };
  });
  await observe('reach', async () => {
    requireThat(plan.reachNode || !plan.relations, 'relation diagram without a reachable node');
    if (!plan.reachNode) {
      await page.click(body(plan.nodes[0])); await settle();
      const focused = await snapshot();
      requireThat(focused.reachButtons.upstream?.disabled && focused.reachButtons.downstream?.disabled, 'Reach is offered without authored relations');
      return { disabled: true };
    }
    await page.click(body(plan.reachNode)); await settle();
    await page.click('#btn-reach-downstream'); await settle();
    const reached = await until(state => state.focus === plan.reachNode && /reach=downstream/.test(state.hash));
    requireThat(reached.focus === plan.reachNode && /reach=downstream/.test(reached.hash), 'downstream reach did not activate');
    return { node: plan.reachNode, hash: reached.hash, reach: reached.reach };
  });
  await observe('reach-empty', async () => {
    await key('Escape', 'Escape'); await key('Escape', 'Escape'); await settle();
    // Every node of a strongly connected diagram has both reaches.
    if (plan.relations && !plan.noUpstream && !plan.noDownstream) return { applicable: false };
    const node = plan.noUpstream || plan.noDownstream || plan.nodes[0], direction = plan.noUpstream || !plan.noDownstream ? 'upstream' : 'downstream';
    await page.click(body(node)); await settle();
    const focused = await snapshot(), button = focused.reachButtons[direction];
    requireThat(button?.disabled === true && button.label.length > 0, `an empty ${direction} reach is not disabled with an explanation`);
    return { node, direction, button };
  });
  await observe('tree', async () => {
    if (!plan.tree) return { applicable: false };
    await key('Escape', 'Escape'); await settle();
    await page.click(`.diagram-container svg [data-tree-toggle="${plan.tree}"]`); await settle();
    const folded = await snapshot();
    requireThat(folded.tree.active && folded.tree.collapsed.includes(plan.tree) && folded.tree.hidden > 0, 'tree branch did not collapse');
    await page.click(`.diagram-container svg [data-tree-toggle="${plan.tree}"]`); await settle();
    const opened = await snapshot();
    requireThat(!opened.tree.collapsed.includes(plan.tree), 'tree branch did not expand');
    return { node: plan.tree, folded: folded.tree, opened: opened.tree };
  });
  await observe('overview', async () => {
    await key('Escape', 'Escape'); await settle();
    // The overview is offered once the canvas is pannable, i.e. zoomed in.
    for (let step = 0; step < 4 && !(await page.evaluate(() => getComputedStyle(document.getElementById('btn-overview-map')).display !== 'none')); step++) {
      await page.click('.diagram-nav [data-view="in"]'); await settle();
    }
    await page.click('#btn-overview-map'); await settle();
    const opened = await snapshot(), camera = await page.evaluate(() => PhoenixDiagram.view.state());
    requireThat(opened.radar.open && opened.radar.count === full.nodes, 'overview did not open with every node');
    const target = plan.nodes.at(-1);
    await page.click(`#overview-map-surface [data-radar-node-id="${target}"]`); await settle();
    const located = await page.evaluate(() => PhoenixDiagram.view.state());
    requireThat(located.x !== camera.x || located.y !== camera.y, 'choosing a node in the overview did not move the view');
    await page.click('#overview-map-close'); await settle();
    await page.click('.diagram-nav [data-view="reset"]'); await settle();
    return { count: opened.radar.count, target, before: camera, after: located };
  });
  await observe('presentation', async () => {
    await page.load('dark', false, { query: { present: '1' } }); await settle();
    const presenting = await snapshot();
    requireThat(presenting.present === 'true', 'presentation did not start from its link');
    await key('Escape', 'Escape'); await settle();
    const exited = await snapshot();
    requireThat(exited.present !== 'true', 'presentation did not exit');
    return { entered: presenting.present, exited: exited.present };
  });
  await observe('deep-link', async () => {
    const id = plan.reachNode || plan.nodes[0];
    await page.load('dark', false, { hash: `focus=${id}` }); await settle();
    const restored = await snapshot();
    requireThat(restored.focus === id && restored.details.hidden === false, 'a focus link did not restore the focused node');
    await page.click('#btn-focus-clear'); await settle();
    const cleared = await snapshot();
    requireThat(cleared.focus === null && cleared.hash === '' && cleared.details.hidden, 'clearing a deep link did not return to the full diagram');
    return { id };
  });
  await observe('keyboard', async () => {
    const reached = [];
    for (let index = 0; index < 80; index++) { await key('Tab', 'Tab'); const active = (await snapshot()).active; reached.push(active); if (active.id === 'btn-route-probe') break; }
    requireThat(reached.at(-1)?.id === 'btn-route-probe', 'the route control is not keyboard reachable');
    await key('Enter', 'Enter'); await settle();
    const started = await snapshot();
    requireThat(started.route.mode === 'source', 'Enter did not start a route');
    await key('Escape', 'Escape'); await settle();
    return { steps: reached.length };
  });
  await observe('details', async () => {
    await page.click(body(plan.nodes[0])); await settle();
    const opened = await snapshot();
    requireThat(opened.details.hidden === false && opened.focus === plan.nodes[0], 'details did not open');
    await key('Escape', 'Escape'); await settle();
    const closed = await snapshot();
    requireThat(closed.details.hidden && closed.focus === null, 'details did not close');
    return { node: plan.nodes[0] };
  });
  report.status = 'pass'; return report;
}

export function verifyExplore(value) {
  requireThat(value?.status === 'pass' && Array.isArray(value.observations), 'exploration did not pass');
  const names = value.observations.map(sample => sample.name);
  for (const name of REQUIRED_EXPLORATION.filter(name => name !== 'route-path' || value.plan.pathPair)) requireThat(names.includes(name), 'missing ' + name);
  requireThat(value.observations.every(sample => sample.status === 'pass'), 'an exploration step failed');
  if (value.plan.noPathPair) {
    const input = value.observations.find(sample => sample.name === 'route-none')?.result?.targetInput;
    requireThat(input?.type === 'click' && input.trusted === true && input.target?.nodeId === value.plan.noPathPair[1], 'unreachable target lacks its actual trusted click');
  }
  return true;
}
