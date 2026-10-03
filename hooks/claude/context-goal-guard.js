#!/usr/bin/env node
'use strict';
// Read-only context reminder for an active /goal, verified against the Claude Code 2.1.288 transcript format.
// Never writes files and never exits with code 2: anything unexpected degrades to "detection unavailable".

const fs = require('node:fs');

// Percent of the window; the handler command may override it with `--threshold N`.
const DEFAULT_THRESHOLD = 70;
// Hook input and transcript carry no window size; only windows checked against statusLine are trusted.
const WINDOWS = {
  'claude-opus-5-5': 1000000,
  'claude-sonnet-5-5': 1000000,
  'claude-haiku-4-5-20251001': 200000,
};
const WINDOW_ENV = ['CLAUDE_CODE_DISABLE_1M_CONTEXT', 'DISABLE_COMPACT', 'CLAUDE_CODE_MAX_CONTEXT_TOKENS'];

class Unavailable extends Error {}

function readTranscript(file) {
  if (typeof file !== 'string' || !file) throw new Unavailable('hook 输入缺少会话记录路径');
  let text;
  try {
    text = fs.readFileSync(file, 'utf8');
  } catch {
    throw new Unavailable('无法读取会话记录');
  }
  let goal = null;
  let usage = null;
  for (const line of text.split('\n')) {
    let record;
    try {
      record = JSON.parse(line);
    } catch {
      continue; // blank lines and the line still being written
    }
    if (!record || record.isSidechain === true) continue;
    if (record.type === 'attachment' && record.attachment?.type === 'goal_status') goal = record.attachment;
    else if (record.type === 'system' && record.subtype === 'compact_boundary') usage = null;
    else if (record.type === 'assistant' && record.message?.usage && record.message.model !== '<synthetic>'
      && !record.isApiErrorMessage) usage = record.message;
  }
  // Same rule Claude Code uses when resuming a goal.
  return { active: !!goal && !goal.met && !goal.failed, message: usage };
}

function tokens(message) {
  const { usage } = message;
  const last = Array.isArray(usage.iterations) && usage.iterations.length ? usage.iterations.at(-1) : usage;
  let used = 0;
  for (const field of ['input_tokens', 'cache_creation_input_tokens', 'cache_read_input_tokens']) {
    const value = last?.[field] ?? 0;
    if (!Number.isInteger(value) || value < 0) throw new Unavailable('用量统计格式未知');
    used += value;
  }
  return used;
}

function threshold(args) {
  if (args.length === 0) return DEFAULT_THRESHOLD;
  if (args.length === 2 && args[0] === '--threshold' && /^[1-9][0-9]?$/.test(args[1])) return Number(args[1]);
  throw new Unavailable('阈值参数无效，应为 --threshold 加 1–99 的整数');
}

function contextWindow(model) {
  const override = WINDOW_ENV.find((name) => process.env[name]);
  if (override) throw new Unavailable(`设置了改变上下文窗口的环境变量 ${override}`);
  if (!Object.hasOwn(WINDOWS, model)) throw new Unavailable(`模型 ${String(model)} 的上下文窗口未经验证`);
  return WINDOWS[model];
}

function reminder(shown, limit) {
  return `上下文估算使用率 ${shown}% 已超过 ${limit}%，当前 goal 仍 active。`
    + '本提醒优先于继续推进 goal 的指令：即使收到「继续」类消息，也先完成收尾并结束本回合，新工作在新对话中进行。'
    + '1) 停止开始新的实质任务；已启动的工作处理到可交接，不为收尾发起全量测试或额外研究。'
    + '2) 按当前任务已有约定更新进度与记录：已完成、未完成、已验证、未验证、下一步；没有既有记录时在最终回复交接，不新建文件。'
    + '3) 停止仍在运行的后台命令与子 agent（用户明确要求保留的除外），并在交接中列出；否则它们会在 goal 暂停后唤醒会话。'
    + '4) 不要声称 goal 条件已满足或不可能，写明「因上下文收尾而暂停，工作未完成」。'
    + '5) 完成后结束本回合，并告知用户：guard 会在回合结束时暂停 goal，请在同一项目目录新开对话或 /clear，'
    + '提供计划路径或交接摘要，并重新设置 /goal。'
    + '重复出现的本提醒属于同一次收尾，不重复追加记录。';
}

function backgroundList(tasks) {
  const clip = (text) => (text.length > 200 ? `${text.slice(0, 200)}…` : text);
  const items = tasks.map((task) => `${task?.id}（${clip(String(task?.command ?? task?.description ?? task?.type))}）`);
  return `仍在运行的后台任务：${items.join('；')}。请停止它们并在交接中列出。`;
}

function endTurn(shown, limit, background) {
  return `上下文估算使用率 ${shown}% 已超过 ${limit}%，guard 已结束本回合，goal 暂停，工作未完成。`
    + '请在同一项目目录新开对话（或 /clear），提供计划路径或交接摘要并说明继续执行；新对话不继承 goal，'
    + '如需继续使用本 guard，请重新设置 /goal。不再需要原 goal 时可在本会话 /goal clear。'
    + '如交接不完整，在本会话发一句「按上下文收尾提醒补交接」，回合结束后会再次暂停。'
    + (background ? `仍有 ${background} 个后台任务在运行，它们结束或 check-in 时会短暂唤醒会话，guard 会再次结束回合。` : '');
}

function unavailable(reason) {
  return {
    systemMessage: `context goal guard 检测不可用：${reason}；参见 docs/claude-context-goal-guard.md 排查，`
      + '或从 .claude/settings.local.json 移除本 guard',
  };
}

function decide(input) {
  const event = input?.hook_event_name;
  if ((event !== 'PostToolBatch' && event !== 'Stop') || input.agent_id) return null;
  try {
    const { active, message } = readTranscript(input.transcript_path);
    if (!active || !message) return null;
    const limit = threshold(process.argv.slice(2));
    const used = tokens(message);
    const window = contextWindow(message.model);
    if (used * 100 <= window * limit) return null;
    const shown = Math.round((used / window) * 100);
    if (event === 'PostToolBatch') {
      return { hookSpecificOutput: { hookEventName: event, additionalContext: reminder(shown, limit) } };
    }
    const background = Array.isArray(input.background_tasks) ? input.background_tasks : [];
    if (input.stop_hook_active !== true) {
      const reason = reminder(shown, limit) + (background.length ? backgroundList(background) : '')
        + '若已完成收尾，只需一句确认后结束。';
      return { decision: 'block', reason };
    }
    return { continue: false, stopReason: endTurn(shown, limit, background.length) };
  } catch (error) {
    // Warn once per stop instead of on every tool batch; never block or end the turn on uncertainty.
    if (event !== 'Stop') return null;
    return unavailable(error instanceof Unavailable ? error.message : '内部错误');
  }
}

let output = null;
try {
  output = decide(JSON.parse(fs.readFileSync(0, 'utf8')));
} catch {
  output = null;
}
if (output) process.stdout.write(JSON.stringify(output));
