'use strict';
// M1 探针：按计划逻辑判定，并把输入与决策记到 logs/。窗口与阈值可用环境变量临时调小。
const fs = require('node:fs');
const path = require('node:path');
const LOG_DIR = path.join(__dirname, '..', '..', 'logs');
const WINDOWS = { 'claude-opus-5-5': 1000000 };

function log(file, obj) {
  fs.appendFileSync(path.join(LOG_DIR, file), JSON.stringify({ ts: new Date().toISOString(), ...obj }) + '\n');
}

function inspect(transcriptPath) {
  let goal = null; let usage = null; let model = null; let compactAfterUsage = false;
  for (const line of fs.readFileSync(transcriptPath, 'utf8').split('\n')) {
    if (!line.trim()) continue;
    let d; try { d = JSON.parse(line); } catch { continue; }
    if (d.isSidechain === true) continue;
    if (d.type === 'attachment' && d.attachment?.type === 'goal_status') goal = d.attachment;
    if (d.type === 'system' && d.subtype === 'compact_boundary') compactAfterUsage = true;
    const u = d.type === 'assistant' ? d.message?.usage : undefined;
    if (u && d.message.model !== '<synthetic>' && !d.isApiErrorMessage) {
      const it = Array.isArray(u.iterations) && u.iterations.length ? u.iterations.at(-1) : u;
      usage = (it.input_tokens || 0) + (it.cache_creation_input_tokens || 0) + (it.cache_read_input_tokens || 0);
      model = d.message.model; compactAfterUsage = false;
    }
  }
  const active = !!goal && !goal.met && !goal.failed;
  return { goal: goal && { met: goal.met, failed: goal.failed, sentinel: goal.sentinel }, active, usage, model, compactAfterUsage };
}

let raw = '';
process.stdin.on('data', (c) => { raw += c; });
process.stdin.on('end', () => {
  let out = null; let decision = 'noop'; let info = {};
  let input = {};
  try {
    input = JSON.parse(raw);
    const slim = { ...input }; delete slim.tool_calls; delete slim.last_assistant_message;
    slim.tool_names = (input.tool_calls || []).map((t) => t.tool_name);
    slim.last_msg_head = (input.last_assistant_message || '').slice(0, 120);
    log('hook-input.jsonl', slim);
    if (input.agent_id) { decision = 'skip-subagent'; }
    else {
      info = inspect(input.transcript_path);
      let fileWindow = 0; try { fileWindow = Number(fs.readFileSync(path.join(LOG_DIR, '..', 'guard-window.txt'), 'utf8').trim()); } catch {}
      const window = Number(process.env.GUARD_WINDOW) || fileWindow || WINDOWS[info.model];
      const pct = Number(process.env.GUARD_PCT) || 65;
      info.window = window;
      if (!info.active) decision = 'noop-no-active-goal';
      else if (info.usage == null || info.compactAfterUsage) decision = 'noop-no-usage';
      else if (!window) { decision = 'unavailable'; if (input.hook_event_name === 'Stop') out = { systemMessage: `context goal guard 检测不可用：模型 ${info.model} 未验证` }; }
      else {
        const shown = Math.round(info.usage / window * 100); info.pct = shown;
        if (!(info.usage * 100 > window * pct)) decision = 'noop-below';
        else {
          const steps = `上下文估算使用率 ${shown}% 已超过 ${pct}%。请立即收尾：1) 不再开始新的实质任务；2) 按当前任务已有约定更新进度与记录（已完成、未完成、下一步），没有记录则在最终回复交接；3) 停止或记录仍在运行的后台任务；4) 不要声称 goal 条件已满足或不可能，写明因上下文收尾而暂停、工作未完成；5) 完成后结束本回合，并告诉用户：guard 会在回合结束时暂停 goal，请在同一项目目录新开对话或 /clear，提供计划路径并重新设置 /goal。`;
          if (input.hook_event_name === 'PostToolBatch') {
            decision = 'remind';
            out = { hookSpecificOutput: { hookEventName: 'PostToolBatch', additionalContext: steps } };
          } else if (input.hook_event_name === 'Stop' && !input.stop_hook_active) {
            decision = 'stop-block';
            out = { decision: 'block', reason: steps + ' 若已完成收尾，只需一句确认后结束。' };
          } else if (input.hook_event_name === 'Stop') {
            decision = 'stop-end-turn';
            const bg = (input.background_tasks || []).length;
            out = { continue: false, stopReason: `上下文估算使用率 ${shown}% 已超过 ${pct}%，guard 已结束本回合，goal 暂停。请在同一项目目录新开对话或 /clear，提供计划路径或交接摘要并重新设置 /goal。${bg ? `仍有 ${bg} 个后台任务在运行。` : ''}` };
          }
        }
      }
    }
  } catch (e) { decision = 'error'; info.error = String(e && e.stack || e); }
  log('decision.jsonl', { event: input.hook_event_name, stop_hook_active: input.stop_hook_active, agent_id: input.agent_id, decision, ...info, out });
  if (out) process.stdout.write(JSON.stringify(out));
});
