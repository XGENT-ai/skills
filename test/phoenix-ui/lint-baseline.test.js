const assert = require('node:assert/strict');
const { test } = require('node:test');

test('lint comparison ignores line shifts and rejects new warning locations and compiler errors', async () => {
  const { additions, clippyDiagnostics } = await import('../../scripts/lib/phoenix-lint-baseline.mjs');
  function diagnostic(file, line, level = 'warning') {
    return JSON.stringify({ reason: 'compiler-message', message: { level, message: 'unused variable', code: { code: 'unused_variables' },
      spans: [{ file_name: file, line_start: line, is_primary: true }] } });
  }
  const baseline = clippyDiagnostics(diagnostic('crates/a.rs', 1), '/checkout');
  const moved = clippyDiagnostics(diagnostic('crates/a.rs', 20), '/checkout');
  assert.deepEqual(additions(moved.counts, baseline.counts), []);
  assert.equal(additions(clippyDiagnostics(diagnostic('crates/b.rs', 1), '/checkout').counts, baseline.counts).length, 1);
  assert.equal(clippyDiagnostics(diagnostic('crates/a.rs', 1, 'error'), '/checkout').errors.length, 1);
});

test('format comparison follows changed lines across checkout paths and detects added formatting drift', async () => {
  const { additions, formatDiagnostics } = await import('../../scripts/lib/phoenix-lint-baseline.mjs');
  const baseline = formatDiagnostics('Diff in /one/crates/a.rs:1:\n context\n-let a=1;\n+let a = 1;\n', '/one');
  const moved = formatDiagnostics('Diff in /two/crates/a.rs:99:\n other context\n-let a=1;\n+let a = 1;\n', '/two');
  assert.deepEqual(additions(moved, baseline), []);
  const colored = formatDiagnostics('Diff in /two/crates/a.rs:99:\n\x1b[31m-let a=1;\n\x1b(B\x1b[m\x1b[32m+let a = 1;\n\x1b(B\x1b[m', '/two');
  assert.deepEqual(additions(colored, baseline), []);
  const added = formatDiagnostics('Diff in /two/crates/a.rs:99:\n-let b=2;\n+let b = 2;\n', '/two');
  assert.equal(additions(added, baseline).length, 1);
});
