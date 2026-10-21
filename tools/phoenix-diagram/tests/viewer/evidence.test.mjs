// Source evidence in the Reader: the assembled template's metadata reader and
// the actual passport renderer from focus.js, run over the fixture DOM.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import { Element } from './fixtures/runtime.mjs';

const read = relative => fs.readFileSync(new URL(relative, import.meta.url), 'utf8');
const template = read('../../assets/template.html');
const focus = read('../../viewer/focus.js');
const catalogs = JSON.parse(read('../../assets/catalogs.json'));
const sha = 'a'.repeat(40);

function extract(source, start, end) {
  const from = source.indexOf(start);
  assert.ok(from >= 0, `missing ${start}`);
  const to = source.indexOf(end, from);
  assert.ok(to > from, `missing end of ${start}`);
  return source.slice(from, to + end.length);
}
const payloadSource = extract(template, 'PhoenixDiagram.sourceEvidence = (function () {', '\n    })();');
const rendererSource = extract(focus, 'function renderSourceEvidence(id) {', '\n        evidence.hidden = false;\n      }');

function text(locale) {
  const messages = catalogs[locale];
  const viewerText = (key, values) => String(Object.hasOwn(messages, key) ? messages[key] : key)
    .replace(/\{([a-zA-Z0-9_]+)\}/g, (match, name) => values && Object.hasOwn(values, name) ? String(values[name]) : match);
  const viewerCount = (key, count, values) => viewerText(key + (Number(count) === 1 ? '.one' : '.other'), { ...values, count });
  return { viewerText, viewerCount };
}

function evidence(linkMode = 'web') {
  const web = linkMode === 'web';
  return {
    schemaVersion: 1, status: 'partial',
    repository: { label: 'example/repo', url: 'https://github.com/example/repo', revision: sha, shortRevision: sha.slice(0, 7), linkMode, ...(web ? { treeHref: `https://github.com/example/repo/tree/${sha}` } : {}) },
    observation: { headMatchesRevision: true, worktree: 'compared' },
    references: [
      { pointer: '/components/0/sources/0', nodeId: 'api', path: 'src/api.rs', line: 3, endLine: 5, label: 'handler', state: 'verified', ...(web ? { href: `https://github.com/example/repo/blob/${sha}/src/api.rs#L3-L5` } : {}) },
      { pointer: '/components/1/sources/0', nodeId: 'db', path: 'src/db.rs', state: 'unverified', reason: 'worktree-differs' },
    ],
    unsourced: ['queue'],
  };
}

function reader(sourceEvidence, locale = 'en') {
  const html = new Element('html');
  const svg = html.appendChild(new Element('svg'));
  for (const id of ['api', 'db', 'queue', 'other']) svg.appendChild(new Element('g', { 'data-node-id': id, 'aria-label': id.toUpperCase() }));
  const metadata = html.appendChild(new Element('script', { id: 'phoenix-diagram-metadata', type: 'application/json' },
    JSON.stringify({ meta: {}, sourceEvidence })));
  const document = {
    getElementById: id => html.querySelector('#' + id),
    querySelector: selector => selector === '.diagram-container svg' ? svg : html.querySelector(selector),
    createElement: tag => new Element(tag),
  };
  const context = vm.createContext({ document, PhoenixDiagram: {}, ...text(locale) });
  vm.runInContext(payloadSource, context);
  return { svg, metadata, context, node: id => svg.querySelector(`[data-node-id="${id}"]`) };
}

function passport(environment) {
  const evidenceElement = new Element('div', { id: 'focus-evidence' });
  const status = evidenceElement.appendChild(new Element('span', { class: 'semantic-passport-evidence-status' }));
  const repositoryLink = evidenceElement.appendChild(new Element('a', { id: 'focus-repository' }));
  const evidenceLinks = evidenceElement.appendChild(new Element('div', { id: 'focus-evidence-links' }));
  Object.assign(environment.context, { evidence: evidenceElement, evidenceStatus: status, repositoryLink, evidenceLinks });
  vm.runInContext(rendererSource, environment.context);
  return { evidence: evidenceElement, status, repositoryLink, evidenceLinks, render: id => environment.context.renderSourceEvidence(id) };
}

test('metadata evidence states and screen-reader labels follow each node', () => {
  const environment = reader(evidence());
  const api = environment.context.PhoenixDiagram.sourceEvidence;
  assert.equal(api.available(), true);
  assert.deepEqual(['api', 'db', 'queue', 'other'].map(api.state), ['verified', 'unverified', 'unsourced', null]);
  assert.equal(api.node('api')[0].path, 'src/api.rs');
  assert.equal(api.node('constructor').length, 0, 'prototype keys are not nodes');
  assert.equal(environment.node('api').getAttribute('aria-label'), 'API, 1 verified source reference');
  assert.equal(environment.node('db').getAttribute('aria-label'), 'DB, Pending verification');
  assert.equal(environment.node('queue').getAttribute('aria-label'), 'QUEUE, No source provided');
  assert.equal(environment.node('other').getAttribute('aria-label'), 'OTHER');
  assert.equal(environment.node('queue').getAttribute('data-source-evidence-original-label'), 'QUEUE', 'export cleanup can restore the authored label');
  assert.equal(reader(evidence(), 'zh-CN').node('db').getAttribute('aria-label'), 'DB, 待核实');
});

test('absent, none and malformed evidence leave the Reader inert', () => {
  for (const value of [undefined, { schemaVersion: 1, status: 'none', references: [], unsourced: [] }, { status: 'verified', references: 'x' }]) {
    const environment = reader(value);
    const api = environment.context.PhoenixDiagram.sourceEvidence;
    assert.equal(api.available(), false);
    assert.equal(api.state('api'), null);
    assert.equal(environment.node('api').getAttribute('aria-label'), 'API');
    assert.equal(environment.node('api').hasAttribute('data-source-evidence-count'), false);
  }
});

test('the passport shows verified links, pending reasons and unsourced elements distinctly', () => {
  const environment = reader(evidence());
  const view = passport(environment);
  view.render('api');
  assert.equal(view.evidence.hidden, false);
  assert.equal(view.evidence.getAttribute('data-evidence-state'), 'verified');
  assert.equal(view.status.textContent, 'Verified source');
  assert.equal(view.repositoryLink.textContent, `example/repo @ ${sha.slice(0, 7)}`);
  assert.equal(view.repositoryLink.href, `https://github.com/example/repo/tree/${sha}`);
  const [verified] = view.evidenceLinks.children;
  assert.equal(verified.localName, 'a');
  assert.equal(verified.href, `https://github.com/example/repo/blob/${sha}/src/api.rs#L3-L5`);
  assert.equal(verified.rel, 'noopener noreferrer');
  assert.match(verified.textContent, /handlerL3–5 ↗src\/api\.rs/);
  assert.equal(view.evidence.style.getPropertyValue('--backend-stroke'), '');
  view.render('db');
  assert.equal(view.evidence.getAttribute('data-evidence-state'), 'unverified');
  assert.equal(view.status.textContent, 'Pending verification');
  assert.equal(view.evidence.style.getPropertyValue('--backend-stroke'), 'var(--cloud-stroke)');
  const [pending] = view.evidenceLinks.children;
  assert.equal(pending.localName, 'div', 'an unverified source never links out');
  assert.equal(pending.href, undefined);
  assert.match(pending.textContent, /src\/db\.rs · Pending verification: The working copy differs from the pinned revision\./);
  assert.equal(view.evidence.getAttribute('title'), null, 'the verification scope only describes verified sources');
  view.render('queue');
  assert.equal(view.status.textContent, 'No source provided');
  assert.equal(view.evidenceLinks.children.length, 0);
  assert.equal(view.evidence.style.getPropertyValue('--backend-stroke'), 'var(--external-stroke)');
  view.render('other');
  assert.equal(view.evidence.hidden, true);
});

test('local-only repositories show path and lines without links', () => {
  const environment = reader(evidence('local-only'), 'zh-TW');
  const view = passport(environment);
  view.render('api');
  assert.equal(view.repositoryLink.textContent, `example/repo @ ${sha.slice(0, 7)} · 僅本機`);
  assert.equal(view.repositoryLink.href, undefined);
  const [row] = view.evidenceLinks.children;
  assert.equal(row.localName, 'div');
  assert.match(row.textContent, /handlerL3–5src\/api\.rs/);
});
