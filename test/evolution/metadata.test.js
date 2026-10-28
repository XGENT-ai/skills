const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const test = require('node:test');

const root = path.resolve(__dirname, '../..');
const skill = path.join(root, 'skills/optimize-agents-md/SKILL.md');
const tool = path.join(root, 'skills/agi-mode/scripts/evolution');

test('AGENTS optimization remains discoverable within its first 100 description characters', () => {
  const content = fs.readFileSync(skill, 'utf8');
  const header = content.match(/^---\n([\s\S]*?)\n---\n/)[1];
  const description = JSON.parse(header.match(/^description: (.*)$/m)[1]);
  assert.ok(description.length <= 200);
  const prefix = description.slice(0, 100);
  for (const boundary of ['仓库 AGENTS.md', '普通编码', '初始化', '全局规则']) assert.ok(prefix.includes(boundary));
  assert.equal(/^disable-model-invocation:\s*true$/m.test(header), false);
  const agi = fs.readFileSync(path.join(root, 'skills/agi-mode/SKILL.md'), 'utf8');
  assert.ok(/^disable-model-invocation:\s*true$/m.test(agi));
  assert.ok(/^compatibility:.*use-prompt-dsl/m.test(header));
  assert.equal(/^compatibility:.*(?:agi-mode|Python|SSH)/m.test(header), false);
});

test('published routing surfaces expose the AGENTS skill and its DSL dependency', () => {
  const routes = [
    'README.md', 'skills/agi-mode/SKILL.md', 'skills/agi-mode/playbooks/eval.md',
    'skills/agi-mode/playbooks/sdlc.md', 'skills/agi-mode/playbooks/what-next.md',
    'skills/agi-mode/references/skill-evolution.md', 'skills/agi-mode/references/interface-design.md',
    'skills/agi-mode/evals/scenarios.md',
  ];
  for (const relative of routes) {
    const content = fs.readFileSync(path.join(root, relative), 'utf8');
    assert.ok(content.includes('optimize-agents-md'), relative);
    assert.ok(content.includes('use-prompt-dsl'), relative);
  }
});

test('AGENTS skill references resolve from its actual installation layout', () => {
  for (const relative of ['skills/optimize-agents-md/SKILL.md', 'skills/optimize-agents-md/assets/coding-principles.md']) {
    const file = path.join(root, relative);
    const content = fs.readFileSync(file, 'utf8');
    for (const [, target] of content.matchAll(/\]\(([^)#]+)(?:#[^)]*)?\)/g)) {
      if (/^https?:/.test(target)) continue;
      assert.ok(fs.existsSync(path.resolve(path.dirname(file), target)), `${relative}: ${target}`);
    }
  }
});

test('CLI discovery needs no Harbor import or model command', () => {
  const result = spawnSync('python3', [path.join(tool, 'evolution.py'), '--help'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  for (const command of ['validate', 'run', 'resume', 'apply']) assert.ok(result.stdout.includes(command));
});
