'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { spawnSync } = require('node:child_process');
const { createHash } = require('node:crypto');

const root = path.resolve(__dirname, '../..');
const skill = path.join(root, 'skills/phoenix-diagram');
const types = ['architecture', 'workflow', 'sequence', 'dataflow', 'lifecycle', 'erd', 'class', 'tree', 'timeline', 'waterfall'];
const references = ['authoring-defaults', 'delivery-contract', 'viewer-runtime', 'repository-authoring',
  'brand-marks', 'preview', 'architecture-compare', 'migration', 'compatibility', 'extending-types'];
const read = file => fs.readFileSync(path.join(skill, file), 'utf8');

test('the skill has a bounded ten-type description with its exclusions up front and normal automatic discovery', () => {
  const entry = read('SKILL.md');
  const frontmatter = /^---\n([\s\S]*?)\n---\n/.exec(entry)?.[1];
  assert.ok(frontmatter, 'valid frontmatter boundary');
  assert.match(frontmatter, /^name: phoenix-diagram$/m);
  const description = JSON.parse(/^description: (".*")$/m.exec(frontmatter)?.[1] || 'null');
  assert.equal(typeof description, 'string');
  assert.ok(Array.from(description).length <= 200);
  const first100 = Array.from(description).slice(0, 100).join('');
  for (const word of ['架构', '工作流', '时序', '数据流', '状态', 'ER', '类', '树', '时间线', '瀑布', 'Mermaid', '数值图表', '仪表盘', 'infographic']) assert.ok(first100.includes(word), word);
  assert.match(frontmatter, /^ {2}version: "0\.2\.0"$/m);
  assert.doesNotMatch(frontmatter, /disable-model-invocation|allow_implicit_invocation/);
  assert.equal(fs.existsSync(path.join(skill, 'agents/openai.yaml')), false);
});

test('entry and conditional references resolve within the installed skill package', () => {
  const documents = ['SKILL.md', ...references.map(name => `references/${name}.md`)];
  const entry = read(documents[0]);
  for (const name of references) assert.ok(entry.includes(`references/${name}.md`), name);
  for (const file of documents) {
    const content = read(file);
    for (const match of content.matchAll(/\[[^\]]+\]\(([^)]+)\)/g)) {
      const target = match[1].split('#')[0];
      if (!target || /^[a-z]+:/i.test(target)) continue;
      const resolved = path.resolve(skill, path.dirname(file), target);
      assert.ok(!path.relative(skill, resolved).startsWith('..'), `${file}: outside bundle ${target}`);
      assert.ok(fs.statSync(resolved).isFile(), `${file}: missing ${target}`);
    }
  }
  assert.ok(fs.statSync(path.join(skill, 'scripts/phoenix-diagram')).isFile());
});

test('all ten guide examples are new-contract complete semantic inputs with automatic routes', () => {
  const collections = { architecture: ['components', 'connections'], workflow: ['nodes', 'edges'],
    sequence: ['participants', 'messages'], dataflow: ['nodes', 'flows'], lifecycle: ['states', 'transitions'],
    erd: ['entities', 'relationships'], class: ['types', 'relationships'], tree: ['nodes'], timeline: ['events'], waterfall: ['spans'] };
  // Only the original five label every relationship; erd/class labels are optional and tree edges derive from parent.
  const labelled = ['architecture', 'workflow', 'sequence', 'dataflow', 'lifecycle'];
  for (const type of types) {
    const document = JSON.parse(read(`examples/minimal.${type}.json`));
    assert.equal(document.diagram_type, type);
    assert.equal(document.schema_version, type === 'workflow' ? 2 : type === 'lifecycle' ? 3 : 1);
    assert.equal(document.meta.input_contract, 'phoenix-v1');
    assert.equal(document.meta.quality_profile, 'showcase');
    assert.equal(document.meta.output, `minimal.${type}.html`);
    const [nodes, relationships = nodes.filter(node => node.parent).map(node => ({ from: node.parent, to: node.id }))] = collections[type].map(name => document[name]);
    assert.ok(nodes.length >= 2, type);
    const identities = new Set(nodes.map(node => node.id));
    assert.equal(identities.size, nodes.length);
    if (type !== 'timeline' && type !== 'waterfall') assert.ok(relationships.length >= 1, type);
    for (const relation of collections[type][1] ? relationships : []) {
      assert.ok(identities.has(relation.from) && identities.has(relation.to), type);
      if (labelled.includes(type)) assert.ok(relation.label.trim(), type);
      for (const pin of ['via', 'channelX', 'channelY', 'labelAt', 'fromSide', 'toSide']) assert.equal(pin in relation, false, pin);
    }
    for (const relation of relationships) assert.ok(identities.has(relation.from) && identities.has(relation.to), type);
    if (document.mainPath) for (let i = 1; i < document.mainPath.length; i += 1) {
      assert.ok(relationships.some(relation => relation.from === document.mainPath[i - 1] && relation.to === document.mainPath[i]), type);
    }
  }
});

test('all five compatibility samples are fixed original bytes inside the sole active skill', () => {
  const samples = {
    'web-app.architecture.json': '483350f5297df682aba4e4a0fa491307ce3d3abd725ae4df07fab177490752cf',
    'agent-tool-call.workflow.json': '4d3f6c23de2ca7d306e05d0041a9506833bb5a164a2bddc2d1984cf4da3ee90c',
    'async-job-roundtrip.sequence.json': '545f20920c89b69c2bb1f15508d5543fddbeb15cffba61e102206e04bd5a5cd9',
    'product-analytics.dataflow.json': '91411404fb4828a3b7ea358f3a7b9877781c9e9dc9c04b42b6eb4a757281b59f',
    'deployment-release.lifecycle.json': '735c2c5819266e6ea2a71775b6eefce98b53030ce90cb721dc8e1d022ce5be71',
  };
  assert.equal(fs.existsSync(path.join(root, 'skills/archify')), false);
  for (const [name, sha256] of Object.entries(samples)) {
    const bytes = fs.readFileSync(path.join(skill, 'examples', `legacy.${name}`));
    assert.equal(createHash('sha256').update(bytes).digest('hex'), sha256, name);
    assert.deepEqual(bytes, fs.readFileSync(path.join(root, 'tools/phoenix-diagram/upstream/legacy-2.17/examples', name)), name);
  }
});

test('documented commands use the flat native launcher without legacy renderer flags', () => {
  const documents = ['SKILL.md', ...references.map(name => `references/${name}.md`)].map(read).join('\n');
  assert.match(documents, /scripts\/phoenix-diagram/);
  assert.doesNotMatch(documents, /node\s+(?:bin\/archify|[^\n]*render-(?:architecture|workflow|sequence|dataflow|lifecycle))/);
  assert.doesNotMatch(documents, /--candidate-sha256|--require-provenance/);
  assert.match(documents, /finalize <type> candidate\.json diagram\.html --repo-root/);
  assert.match(documents, /evidence\.status/);
  const cli = fs.readFileSync(path.join(root, 'tools/phoenix-diagram/src/cli.rs'), 'utf8');
  for (const command of ['guide', 'validate', 'deliver', 'check', 'browser-check', 'visual-check', 'finalize', 'review-record', 'recover-output', 'doctor']) {
    if (new RegExp(`"\\$PD" ${command}\\b`).test(documents)) assert.ok(cli.includes(command), command);
  }
});

test('class acceptance keeps unspecified quantities separate from explicit Mermaid multiplicities', () => {
  const facts = JSON.parse(fs.readFileSync(path.join(root, 'tools/phoenix-diagram/tests/provider/class-facts.json'), 'utf8'));
  const prompt = Buffer.from(facts.natural.prompt);
  assert.equal(facts.schemaVersion, 1);
  assert.equal(prompt.length, 484);
  assert.equal(createHash('sha256').update(prompt).digest('hex'), 'ad7ce3fcec3c8df8da4defa70e39a5c3ab45ee2a56146d3e1a59b7922b286cda');
  assert.deepEqual(facts.natural.source.promptIdentity, { bytes: prompt.length, sha256: createHash('sha256').update(prompt).digest('hex') });
  assert.equal(facts.natural.review.relationship.minimumProvided, false);
  assert.deepEqual(facts.natural.review.relationship.numericCardinalityMaterials, []);
  assert.ok(Array.isArray(facts.natural.review.providedQuantities), 'Keep the supplied single Cart in the review ledger');
  assert.deepEqual(facts.natural.review.providedQuantities.map(({ from, to, count }) => ({ from, to, count })), [
    { from: 'Checkout', to: 'Cart', count: 1 },
  ]);
  for (const quantity of facts.natural.review.providedQuantities) {
    for (const passage of quantity.materials) assert.ok(facts.natural.prompt.includes(passage), passage);
  }
  for (const item of [facts.natural, facts.explicitMermaid]) {
    assert.deepEqual(item.review.expectedMissingFacts, []);
    for (const passage of item.review.relationship.materials) assert.ok(item.prompt.includes(passage), passage);
    assert.doesNotMatch(item.prompt, /expectedMissingFacts|minimumProvided|verdict|评分|判据/);
  }
  assert.equal(facts.explicitMermaid.review.relationship.fromMultiplicity, '0..1');
  assert.equal(facts.explicitMermaid.review.relationship.toMultiplicity, '0..*');
});

test('ERD acceptance material preserves the original bytes and separates prompts from review facts', () => {
  const facts = JSON.parse(fs.readFileSync(path.join(root, 'tools/phoenix-diagram/tests/provider/erd-facts.json'), 'utf8'));
  assert.equal(facts.schemaVersion, 1);
  const original = Buffer.from(facts.missingFacts.prompt);
  assert.equal(original.length, 675);
  assert.equal(createHash('sha256').update(original).digest('hex'), '3e8fe03b6b256f906b9a6490c23fa1587838fbb0a836c9115cba74f41df91fd7');
  assert.deepEqual(facts.missingFacts.source.promptIdentity, {
    bytes: original.length, sha256: createHash('sha256').update(original).digest('hex'),
  });
  assert.deepEqual(facts.missingFacts.review.expectedMissingFacts.map(gap => gap.subject), ['orders.customer_id', 'product -> order_item']);
  for (const fact of facts.missingFacts.review.suppliedFacts) assert.ok(facts.missingFacts.prompt.includes(fact.material));
  for (const material of [facts.complete, facts.explicitMermaid]) {
    assert.deepEqual(material.review.expectedMissingFacts, []);
    assert.equal(material.review.expectedRelationships.length, 3);
    for (const relation of material.review.expectedRelationships) {
      for (const passage of relation.materials) assert.ok(material.prompt.includes(passage), passage);
    }
  }
  assert.match(facts.complete.prompt, /customer_id bigint 非空/);
  assert.match(facts.complete.prompt, /一个商品可以没有明细/);
  for (const material of [facts.missingFacts, facts.complete, facts.explicitMermaid]) {
    assert.doesNotMatch(material.prompt, /expectedMissingFacts|expectedRelationships|verdict|评分|判据/);
  }
  assert.ok(facts.reviewInstructions.length > 0);
});

test('the packaged review draft recipe preserves receipt bindings and leaves observation pending', t => {
  const contract = read('references/delivery-contract.md');
  const recipe = /<<'NODE'\n([\s\S]*?)\nNODE/.exec(contract)?.[1];
  assert.ok(recipe, 'executable Node review draft recipe');
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram-skill-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const identity = { sha256: 'a'.repeat(64), bytes: 100 };
  const bound = name => ({ path: path.join(directory, name), identity });
  const screenshots = [1440, 2048].flatMap(width => ['light', 'dark'].map(theme => ({ ...bound(`${width}-${theme}.png`),
    viewport: { width, height: width === 1440 ? 900 : 1320 }, theme })));
  const browser = { screenshots };
  fs.writeFileSync(path.join(directory, 'browser.json'), JSON.stringify(browser));
  const summary = { schemaVersion: 1, tool: 'phoenix-diagram', toolVersion: '0.2.0', automatedStatus: 'pass',
    stages: ['validate', 'deliver', 'check', 'browser-check'].map(name => ({ name, status: 'pass' })),
    candidate: { ...bound('candidate.json'), originalBinding: { protocolFixture: true } },
    artifact: { ...bound('diagram.html'), originalBinding: { protocolFixture: true } },
    configSha256: 'b'.repeat(64), engine: { sourceInputSha256: 'c'.repeat(64), inputContractSchema: 1, receiptSchema: 1, viewerRevision: 'phoenix-v2', binary: identity },
    deliveryReceiptId: 'protocol-fixture-no-visual-claim', browserReceiptIdentity: identity, evidence: { browserReceipt: bound('browser.json') } };
  const summaryPath = path.join(directory, 'summary.json'), recordPath = path.join(directory, 'review.json');
  fs.writeFileSync(summaryPath, JSON.stringify(summary));
  const result = spawnSync(process.execPath, ['-', summaryPath, recordPath], { input: recipe, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const record = JSON.parse(fs.readFileSync(recordPath));
  assert.deepEqual(Object.keys(record).sort(), ['schemaVersion', 'tool', 'toolVersion', 'command', 'status', 'diagnostics',
    'artifact', 'candidate', 'configSha256', 'engine', 'deliveryReceiptId', 'browserReceiptIdentity', 'screenshotIdentities', 'reviewer', 'scope', 'verdict', 'findings'].sort());
  assert.deepEqual(record.artifact, bound('diagram.html'));
  assert.deepEqual(record.candidate, bound('candidate.json'));
  assert.deepEqual(record.engine, summary.engine);
  assert.equal(record.deliveryReceiptId, summary.deliveryReceiptId);
  assert.deepEqual(record.browserReceiptIdentity, identity);
  assert.equal(record.verdict, 'unavailable');
  assert.equal(record.reviewer, '');
  assert.ok(record.findings.length > 0);
  assert.deepEqual(record.scope, ['diagram', 'labels', 'cards', 'relationship-occlusion', 'hierarchy', 'controls']);
  assert.equal(record.screenshotIdentities.length, 4);
  for (const [index, observation] of record.screenshotIdentities.entries()) {
    assert.deepEqual(observation.file, { path: screenshots[index].path, identity });
    assert.equal(observation.observed, false);
    assert.equal(observation.observations, '');
  }
  const retry = spawnSync(process.execPath, ['-', summaryPath, recordPath], { input: recipe, encoding: 'utf8' });
  assert.notEqual(retry.status, 0, 'draft creation must preserve an existing review');
});
