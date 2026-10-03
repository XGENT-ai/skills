// M4 组合测试用 Stop hook：stop_hook_active 为 false 时请求一次确认，并把决策记到 m4/combo.log。
const fs = require('node:fs');
const input = JSON.parse(fs.readFileSync(0, 'utf8'));
const block = input.stop_hook_active !== true;
fs.appendFileSync(__dirname + '/combo.log', JSON.stringify({ ts: new Date().toISOString(), sha: input.stop_hook_active, block }) + '\n');
if (block) process.stdout.write(JSON.stringify({ decision: 'block', reason: '组合测试：结束前确认 PLAN.md 已保存，一句话即可。' }));
