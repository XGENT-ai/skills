'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');
const root = path.resolve(__dirname, '../..');
const fixtures = path.join(root, 'tools/phoenix-diagram/tests/provider');
const load = name => JSON.parse(fs.readFileSync(path.join(fixtures, name)));
const identity = value => {
  const bytes = Buffer.from(value);
  return { sha256: createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length };
};
// These byte identities come from the existing acceptance inputs. The
// tests stay independent of private acceptance directories and provider access.
const originals = {
  'intent-architecture': [500, '63541e17a3db147af370e30b33a78f6e7d93dbe73d759e9702c36ce3e960c424'],
  'intent-workflow': [623, 'f58a9937dbfb5b6c11a3abad2ea8efe5d3583a32773f1927ae5a5780e10ec55a'],
  'intent-sequence': [408, '9327801d406e1206f12df23c9b44f1e20ee8c2e7e6a7e899ea811a1db3aa5578'],
  'intent-dataflow': [394, 'af5f620d162276e08b30557306454066e4e88f544073bc28a23b322a1eb45779'],
  'intent-lifecycle': [367, 'c99c49f51a622498193b675a5245c16c632e7ad501d811c7168f0f4fda33dc6a'],
  'mermaid-flowchart': [307, '0c16624839be9994ec8928ce0ac87e87f093a4eaf7d06e3609bd1f24da8dda56'],
  'mermaid-graph': [314, '161938dddd8cee9c4c807fa863cbec824e9488e61ed96319486f3242f9e016c4'],
  'mermaid-sequence': [442, '57dfa74e7b21fad014e2c2521c2fc913a91ea9f76f3a20dac2c5d869608a5b71'],
  'mermaid-state': [438, '4b01cf4723dc065f1672aae9873526401bd1a73a9b804cd093751170ff75f260'],
  'missing-date': [223, 'ab661d6abccb5e4b77ed657596cd9aff4622ed3afd2471c97b704fbca1d55de8'],
  'missing-amount': [238, '02c7f738141954626d2704db41eefad43032225712d3d76effa1daba63312f74'],
  'missing-condition': [277, 'd8db8ab0f98e9f4e8212d04ae43884339e8412a33bbd535d63e6003f69f6e48f'],
  'repairable-diagnostic': [208, '4b66af223b6a22f679de02e47f1324cf5bf541254489342273eaccdb509bf84d'],
  'stalled-repair': [262, '28491227a40d3c10a3531c00abcf1edbbd6c0f633b0d010f74a7b0a0d01ae58e'],
  'browser-unavailable': [259, 'b16b71fd65b8d54fc539ad6243e6d1b625bb6164d48a99c56bf9d5ba9776c136'],
  'browser-failed': [279, 'bc950722e1b150a72295249169e8e028a5e87c31ace5afcd1d8379fbec14ab3d'],
  'visual-pending': [276, '313b19fb2a41c6dd86fac6323c9deb3c2a750263827fa193549a31b66e314876'],
  'legacy-lifecycle': [299, 'bde0a3519b63cc6454d2d4d4b394bd6447d98f047586d606d12a64e34c0e7e18'],
  'intent-erd': [706, 'b791b736bb2787350ca6ae735577f8b35f569e38954e9f5b20fb545f8aa844a8'],
  'intent-class': [484, 'ad7ce3fcec3c8df8da4defa70e39a5c3ab45ee2a56146d3e1a59b7922b286cda'],
  'intent-tree': [245, '5166e7e3ef1468287a89ce8df0d99e8404dc954e39430b4fd01d972a1d3cc8b0'],
  'intent-timeline': [335, '413324af353ef685aac2822a8895949129a27374f1e297af375ebe9b3f8600ce'],
  'intent-waterfall': [411, 'a35e703b58471202a0c5386de0587462089b1e558ee2d37fe2cb73b2363f35b6'],
  'mermaid-er': [522, '91f0ac9901de111b8d5741866dcbabfb8c302f77f37bb21bc83c8e992b0f1ec3'],
  'mermaid-class': [628, '92e3efab9ac7f517903f379808d6a3acc3dc411e3cf720dc1ff14315666026ba'],
  'repo-verified': [329, 'a234ddcd75981fd4005c7fec173162c1377f2e1c6fd42c3006dd6281026002d7'],
  'repo-dirty': [277, 'c39ef8d0d999a7d316efd9520e94ffaeb52b9155ca99712891ee891ea9676867'],
  'repo-timeline': [264, '743fa773ec7dc1865ef1a5e97c2b18702c411c4a35f40505c1a5288d919ed0c5'],
  'waterfall-illustrative': [254, 'd7d065edcee9abda1e7295d46f6191ce31c33d6ba9dcc4b18ce33908a7d33da4'],
  'reference-31-json': [163, 'a4a13f5df24e655139e511100f59422a9c6206c42b7151f66cc7cbe5487c6932'],
  'locale-zh-tw': [208, 'c60a1f6175e02c95bae56b563c57a1db7fa40519072d35cbd03a1990ed811457'],
};
const revised = {
  'repo-verified': '仓库`{{REPO}}`已检出在固定提交。请解释从已安装skill入口执行普通命令`doctor --json`时，如何找到、校验并启动原生引擎，画成一张供评审使用的图，主要步骤对应具体源码行。前提是cwd就是已有合法安装回执的项目根，缓存引擎已存在；只解释这条正常路径。帮助/版本、显式引擎安装、升级、目录回退和异常恢复另有专项，不要求展开。只看直接相关文件，答复简短；这里只作源码解释，不执行被解释仓库的命令。\n',
  'repo-dirty': '仓库`{{REPO}}`是本地工作副本。请解释`bin/phoenix-diagram.js`与`skills/phoenix-diagram/scripts/phoenix-launcher.cjs`分别接收普通命令`doctor --json`后如何交给已安装引擎，画成一张图，标出具体源码行。前提同上，只比较两入口的参数/配置、校验与交接正常路径；不展开帮助/版本、显式安装、升级或目录回退。另按实际来源核验说明未提交修改的影响，答复简短；不执行被解释仓库的命令。\n',
};

test('both providers retain all 31 formal roles and the two ERD counterexamples stay separate', () => {
  const manifest = load('p2-scenarios.json');
  const ids = Object.keys(originals);
  assert.equal(manifest.schemaVersion, 1);
  assert.equal(manifest.kind, 'phoenix-diagram-p2-provider-scenarios');
  assert.deepEqual(manifest.rows.map(row => row.id), ids);
  assert.deepEqual(Object.keys(manifest.providers), ['claude', 'codex']);
  for (const provider of Object.values(manifest.providers)) assert.deepEqual(provider, ids);
  assert.deepEqual(manifest.counterexamples, {
    providers: ['claude', 'codex'],
    fixture: 'tools/phoenix-diagram/tests/provider/erd-facts.json',
    promptField: 'missingFacts.prompt',
    id: 'erd-missing-facts',
    promptIdentity: { bytes: 675, sha256: '3e8fe03b6b256f906b9a6490c23fa1587838fbb0a836c9115cba74f41df91fd7' },
    captureOnly: true,
  });
  assert.equal(manifest.rows.length * 2 + manifest.counterexamples.providers.length, 64);
});

test('only the two repository topics change version; the other 29 preserve exact original bytes', () => {
  const manifest = load('p2-scenarios.json');
  for (const row of manifest.rows) {
    const [bytes, sha256] = originals[row.id];
    assert.deepEqual(row.original.identity, { bytes, sha256 }, row.id);
    assert.equal(row.original.version, 'original-v1');
    assert.ok(row.original.source.startsWith('local/phoenix-diagram/'));
    const originalText = Object.hasOwn(revised, row.id) ? row.original.prompt : row.prompt;
    assert.deepEqual(identity(originalText), row.original.identity, row.id);
    assert.deepEqual(identity(row.prompt), row.promptIdentity, row.id);
    assert.equal(row.version, Object.hasOwn(revised, row.id) ? 'main-v2' : 'original-v1');
    if (Object.hasOwn(revised, row.id)) {
      assert.equal(row.prompt, revised[row.id]);
      assert.notDeepEqual(row.promptIdentity, row.original.identity);
    } else assert.deepEqual(row.promptIdentity, row.original.identity);
  }
});

test('frozen materials and browser setup retain complete bytes and identities', () => {
  const manifest = load('p2-scenarios.json');
  const expected = {
    'repairable-diagnostic/input.json': [670, '7ba4e8f44d9ad7556709554fa0abebfa951a492215d58314220becff372234d0'],
    'stalled-repair/input.json': [895, 'ab898ee9f93470b6d692834357582139caefc4be20bc538ce013ee6c79463552'],
    'stalled-repair/constraints.txt': [870, '1959fb2aafb361c03ef2e85d3c43cd7069ac58ee199ef2ea02e08ad276f30883'],
    'legacy-lifecycle/input.json': [4274, '735c2c5819266e6ea2a71775b6eefce98b53030ce90cb721dc8e1d022ce5be71'],
    'reference-31-json/workflow.json': [4723, '630be98449925d2716ecb8439f404b2bff5fe65ba905b0ad0baea6d96890c526'],
  };
  const actual = [];
  for (const row of manifest.rows) for (const material of row.materials) {
    const key = row.id + '/' + material.name;
    actual.push(key);
    const [bytes, sha256] = expected[key];
    assert.deepEqual(material.identity, { bytes, sha256 }, key);
    const content = material.packageMember ? fs.readFileSync(path.join(root, material.packageMember)) : material.content;
    assert.deepEqual(identity(content), material.identity, key);
    assert.ok(material.packageMember || material.source.startsWith('local/phoenix-diagram/'));
  }
  assert.deepEqual(actual, Object.keys(expected));
  const unavailable = manifest.rows.find(row => row.id === 'browser-unavailable');
  assert.deepEqual(unavailable.sandboxProfile.identity, { bytes: 366, sha256: '39b37ccd203e3d869246314d9a52cc6584200cc238624ba6046dd0750dc3e0c4' });
  assert.deepEqual(identity(unavailable.sandboxProfile.content), unavailable.sandboxProfile.identity);
  assert.deepEqual(manifest.rows.find(row => row.id === 'browser-failed').env, { PHOENIX_DIAGRAM_BROWSER: '/usr/bin/false' });
});

test('repository preparation preserves the fixed source and the original dirty append', () => {
  const manifest = load('p2-scenarios.json');
  for (const row of manifest.rows.filter(value => value.repo)) {
    assert.equal(row.repo.revision, 'e29d4cafcada8809335e0a3dfaa939be95bb9643');
    assert.equal(row.repo.origin, 'https://github.com/XGENT-ai/skills.git');
    assert.equal(row.repo.checkout, 'detached');
    assert.equal(row.repo.readOnly, true);
    assert.equal(row.repo.dirtyPath, row.id === 'repo-dirty' ? 'skills/phoenix-diagram/scripts/phoenix-launcher.cjs' : null);
    assert.equal(row.repo.dirtyAppend, row.id === 'repo-dirty' ? '\n// Local experiment: not committed.\n' : null);
    assert.equal((row.prompt.match(/\{\{REPO\}\}/g) || []).length, 1);
  }
  assert.deepEqual(manifest.rows.filter(row => row.repo).map(row => row.id), ['repo-verified', 'repo-dirty', 'repo-timeline']);
});

test('author topics contain no review answers and are selected separately from review facts', () => {
  const manifest = load('p2-scenarios.json');
  const facts = load('repo-main-facts.json');
  assert.deepEqual(Object.keys(facts.author), Object.keys(revised));
  for (const [id, author] of Object.entries(facts.author)) {
    assert.deepEqual(Object.keys(author), ['version', 'prompt', 'promptIdentity']);
    assert.equal(author.version, 'main-v2');
    assert.equal(author.prompt, revised[id]);
    assert.deepEqual(author.promptIdentity, identity(author.prompt));
    assert.equal(manifest.rows.find(row => row.id === id).prompt, author.prompt);
    assert.doesNotMatch(author.prompt, /verifyReceipt|preflight|spawnSync|checkReceipt|allowInstall|评分|得分|修订原因|历史失败/);
  }
  for (const row of manifest.rows) assert.equal(Object.hasOwn(row, 'review'), false);
  const erd = load('erd-facts.json');
  assert.equal(manifest.rows.find(row => row.id === 'intent-erd').prompt, erd.complete.prompt);
  assert.deepEqual(identity(erd.missingFacts.prompt), manifest.counterexamples.promptIdentity);
});

test('repository reviews retain independent mandatory dimensions and source-backed normal-path facts', () => {
  const facts = load('repo-main-facts.json');
  assert.equal(facts.review.requireAllApplicable, true);
  assert.deepEqual(facts.review.dimensions, ['automated', 'semantic', 'provenance', 'visual', 'protocol', 'identity']);
  assert.equal(facts.review.nodeQuota, undefined);
  assert.equal(facts.review.referenceQuota, undefined);
  for (const id of Object.keys(revised)) {
    const scenario = facts.review.scenarios[id];
    assert.equal(scenario.version, 'main-v2');
    assert.deepEqual(scenario.promptIdentity, identity(revised[id]));
    assert.ok(scenario.requiredAssertions.length);
    for (const assertion of scenario.requiredAssertions) {
      assert.ok(assertion.fact.trim());
      assert.ok(assertion.sources.length);
      for (const source of assertion.sources) {
        assert.equal(source.revision, 'e29d4cafcada8809335e0a3dfaa939be95bb9643');
        assert.ok(Number.isInteger(source.startLine) && source.startLine > 0 && source.endLine >= source.startLine);
        assert.deepEqual(source.identity, identity(source.passage));
        assert.equal(source.passage.split('\n').length - 1, source.endLine - source.startLine + 1);
      }
    }
  }
  const statements = facts.review.scenarios['repo-dirty'].requiredAssertions.map(row => row.fact).join('\n');
  assert.match(statements, /bundleRoot/);
  assert.match(statements, /partial/);
  assert.match(statements, /doctor --json/);
  assert.match(facts.review.optionalBranches, /主动.*事实.*源码/);
  assert.match(facts.review.history, /不.*改判/);
});
