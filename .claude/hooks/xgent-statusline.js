// Claude Code Statusline — context-remaining only.
//
// Shows nothing but the remaining context budget as a 10-cell bar + percent.
// Pure Node, no third-party deps. Silent-fails on every error path — must
// NEVER throw or hang the user terminal.

// Claude Code auto-compacts at the window minus min(max output, 20K) and a 13K
// summary buffer (checked against 2.1.311); current models output over 20K.
const COMPACT_RESERVE = 20_000 + 13_000;

// CLAUDE_CODE_AUTO_COMPACT_WINDOW is a window size (400000, 400k, 1M) that
// Claude Code clamps to 100K–1M; it ignores values that are not a size.
function compactWindowFromEnv(raw) {
  if (!raw) return undefined;
  const value = raw.trim();
  const unit = { k: 1_000, m: 1_000_000 }[value.slice(-1).toLowerCase()];
  const tokens = unit ? parseFloat(value) * unit : Number(value);
  if (!(tokens >= 1)) return undefined;
  return Math.min(1_000_000, Math.max(100_000, Math.round(tokens)));
}

function runStatusline() {
  let input = '';
  // Exit silently on stalled stdin.
  const stdinTimeout = setTimeout(() => process.exit(0), 3000);
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', (chunk) => {
    input += chunk;
  });
  process.stdin.on('end', () => {
    clearTimeout(stdinTimeout);
    try {
      const data = input ? JSON.parse(input) : {};
      const usage = data.context_window?.current_usage;
      const windowSize = data.context_window?.context_window_size;
      if (!usage || !windowSize) return;

      // Context occupied by the last request, counted as Claude Code does.
      const usedTokens =
        usage.input_tokens + usage.cache_creation_input_tokens + usage.cache_read_input_tokens;

      // Count down to the auto-compact trigger so the bar reflects *usable* headroom.
      const acw = compactWindowFromEnv(process.env.CLAUDE_CODE_AUTO_COMPACT_WINDOW);
      const threshold = Math.min(windowSize, acw ?? Infinity) - COMPACT_RESERVE;

      // Usable tokens still free before auto-compact fires, shown in K.
      const usableRemaining = Math.max(0, threshold - usedTokens);
      const availK = `${Math.round(usableRemaining / 1000)}K available`;

      const left = Math.min(100, Math.round((usableRemaining / threshold) * 100));
      // Bar + percent reflect *used* (occupied) share of the usable window.
      const used = 100 - left;
      const filled = Math.floor(used / 10);
      const bar = '█'.repeat(filled) + '░'.repeat(10 - filled);

      // Greener with more headroom; blinking red when nearly out.
      let color;
      let prefix = '';
      if (left > 50) {
        color = '32';
      } else if (left > 35) {
        color = '33';
      } else if (left > 20) {
        color = '38;5;208';
      } else {
        color = '5;31';
        prefix = '💀 ';
      }

      process.stdout.write(`\x1b[${color}m${prefix}${bar} ${used}% ${availK}\x1b[0m`);
    } catch {
      // Silent fail — never break the statusline.
    }
  });
}

runStatusline();
