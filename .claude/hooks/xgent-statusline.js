// Claude Code Statusline — context-remaining only.
//
// Shows nothing but the remaining context budget as a 10-cell bar + percent.
// Pure Node, no third-party deps. Silent-fails on every error path — must
// NEVER throw or hang the user terminal.

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
      const remaining = data.context_window?.remaining_percentage;
      if (remaining == null) return;

      // Reserve the auto-compact buffer so the bar reflects *usable* headroom
      // (the same accounting the GSD/gomad statuslines used).
      const totalCtx = data.context_window?.total_tokens || 1_000_000;
      const acw = parseInt(process.env.CLAUDE_CODE_AUTO_COMPACT_WINDOW || '0', 10);
      const bufferPct = acw > 0 ? Math.min(100, (acw / totalCtx) * 100) : 16.5;

      const left = Math.max(
        0,
        Math.min(100, Math.round(((remaining - bufferPct) / (100 - bufferPct)) * 100)),
      );
      // Bar + percent reflect *used* (occupied) share of the usable window.
      const used = 100 - left;
      const filled = Math.floor(used / 10);
      const bar = '█'.repeat(filled) + '░'.repeat(10 - filled);

      // Usable tokens still free (actual remaining minus the reserved
      // auto-compact buffer), shown in K.
      const usableRemaining = Math.max(0, Math.round((totalCtx * (remaining - bufferPct)) / 100));
      const availK = `${Math.round(usableRemaining / 1000)}K available`;

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
