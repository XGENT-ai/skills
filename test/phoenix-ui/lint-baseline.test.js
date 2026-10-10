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

test('Windows Clippy paths retain the same baseline identity while changed warnings remain visible', async () => {
  const { additions, clippyDiagnostics } = await import('../../scripts/lib/phoenix-lint-baseline.mjs');
  const warning = (file, message = String.raw`a warning mentioning C:\user`) => JSON.stringify({
    reason: 'compiler-message', message: { level: 'warning', message, code: { code: 'clippy::question_mark' },
      spans: [{ file_name: file, is_primary: true }] },
  });
  const baseline = clippyDiagnostics(warning('crates/foundation/src/css/scan.rs'), '/checkout').counts;
  const workspace = String.raw`D:\a\skills\skills\tools\phoenix-ui`;
  for (const file of [String.raw`crates\foundation\src\css\scan.rs`,
    String.raw`\\?\D:\a\skills\skills\tools\phoenix-ui\crates\foundation\src\css\scan.rs`]) {
    const current = clippyDiagnostics(warning(file), workspace).counts;
    assert.deepEqual(additions(current, baseline), []);
    assert.equal(JSON.parse(Object.keys(current)[0])[2], String.raw`a warning mentioning C:\user`);
  }
  assert.equal(additions(clippyDiagnostics(warning(String.raw`crates\new.rs`), workspace).counts, baseline).length, 1);
  assert.equal(additions(clippyDiagnostics(warning(String.raw`crates\foundation\src\css\scan.rs`, 'different warning'), workspace).counts, baseline).length, 1);
});

test('Windows namespaced rustfmt paths and CRLF preserve hunk identity without hiding changed lines', async () => {
  const { additions, formatDiagnostics } = await import('../../scripts/lib/phoenix-lint-baseline.mjs');
  const baseline = formatDiagnostics('Diff in /checkout/crates/browser/src/cdp.rs:1:\n-let a=1;\n+let a = 1;\n', '/checkout');
  const workspace = String.raw`D:\a\skills\skills\tools\phoenix-ui`;
  const file = String.raw`\\?\D:\a\skills\skills\tools\phoenix-ui\crates\browser\src\cdp.rs`;
  const output = `Diff in ${file}:99:\r\n-let a=1;\r\n+let a = 1;\r\n`;
  assert.deepEqual(additions(formatDiagnostics(output, workspace), baseline), []);
  assert.equal(additions(formatDiagnostics(output.replace('a=1', 'a=2'), workspace), baseline).length, 1);
  const unc = formatDiagnostics(String.raw`Diff in \\?\UNC\runner\share\crates\browser\src\cdp.rs:99:` + '\n-let a=1;\n+let a = 1;\n', String.raw`\\runner\share`);
  assert.deepEqual(additions(unc, baseline), []);
});
