// Real native failure receipts. Protocol fixtures remain separate from this
// macOS discovery/launch matrix; no user browser or installation is changed.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { bound, readBound, readSnapshot } from '../../../../scripts/test-phoenix-diagram-core.mjs';
import { invokeNative } from '../../../../scripts/test-phoenix-diagram-runtime.mjs';
import { discoverBrowser } from './export-browser.mjs';

function writeNew(file, contents) {
  fs.writeFileSync(file, typeof contents === 'string' || Buffer.isBuffer(contents)
    ? contents : `${JSON.stringify(contents, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  return bound(readSnapshot(file));
}

function unavailableProfile() {
  const roots = ['/Applications', path.join(os.homedir(), 'Applications')];
  const bundles = [
    'Google Chrome.app/Contents/MacOS/Google Chrome',
    'Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary',
    'Chromium.app/Contents/MacOS/Chromium',
  ];
  const hidden = bundles.flatMap(bundle => roots.map(root => path.join(root, bundle)));
  return { hidden, text: `(version 1)\n(allow default)\n(deny file-read* ${hidden.map(file => `(literal ${JSON.stringify(file)})`).join(' ')})\n` };
}

export async function runBrowserFailureAcceptance({ engine, outputDir, fixture }) {
  const parent = fs.lstatSync(outputDir);
  assert(parent.isDirectory() && !parent.isSymbolicLink(), 'Browser matrix needs an ordinary run directory');
  const directory = path.join(fs.realpathSync(outputDir), 'browser-failures');
  fs.mkdirSync(directory, { mode: 0o700 });
  const report = { schemaVersion: 1, tool: 'phoenix-diagram', kind: 'real-browser-failure-receipts', status: 'not-run', rows: [] };
  let failure;
  try {
    assert.equal(process.platform, 'darwin', 'The browser-absence profile requires macOS');
    assert.equal(process.arch, 'arm64', 'The bounded browser matrix requires darwin-arm64');
    fs.accessSync('/usr/bin/sandbox-exec', fs.constants.X_OK);
    fs.accessSync('/usr/bin/false', fs.constants.X_OK);
    const browserEnvironment = { ...process.env };
    delete browserEnvironment.PHOENIX_DIAGRAM_BROWSER;
    report.browser = discoverBrowser({ env: browserEnvironment });
    const native = bound(readSnapshot(engine));
    const source = readSnapshot(fixture);
    assert.equal(JSON.parse(source.content).diagram_type, 'tree');
    report.engine = native;
    report.fixture = bound(source);
    const profile = unavailableProfile();
    report.unavailableProfile = writeNew(path.join(directory, 'browser-unavailable.sb'), profile.text);
    report.hiddenDiscoveryPaths = profile.hidden;
    const invoke = async (rowDirectory, name, args, { prefix = [], environment = {} } = {}) => {
      const invocation = {
        program: prefix[0] || native.path,
        argv: prefix.length ? [...prefix.slice(1), native.path, ...args] : [...args],
        environment, clearedEnvironment: ['PHOENIX_DIAGRAM_BROWSER'],
      };
      try {
        const result = await invokeNative(native, args, { prefix, env: { PHOENIX_DIAGRAM_BROWSER: undefined, ...environment } });
        return {
          ...invocation, exitCode: result.exitCode, signal: result.signal,
          stdout: writeNew(path.join(rowDirectory, `${name}.stdout.json`), result.stdout),
          stderr: writeNew(path.join(rowDirectory, `${name}.stderr.txt`), result.stderr),
          receipt: writeNew(path.join(rowDirectory, `${name}.receipt.json`), result.receipt),
          value: result.receipt,
        };
      } catch (error) {
        if (error.processResult) {
          const result = error.processResult;
          writeNew(path.join(rowDirectory, `${name}.failed-process.json`), {
            ...invocation, exitCode: result.exitCode, signal: result.signal,
            stdout: writeNew(path.join(rowDirectory, `${name}.stdout.txt`), result.stdout),
            stderr: writeNew(path.join(rowDirectory, `${name}.stderr.txt`), result.stderr),
          });
        }
        throw error;
      }
    };
    for (const command of ['finalize', 'browser-check', 'visual-check']) {
      for (const directoryState of ['fresh', 'precreated']) {
        for (const environment of ['unavailable', 'launch-failed']) {
          const id = `${command}-${directoryState}-${environment}`;
          const rowDirectory = path.join(directory, id);
          fs.mkdirSync(rowDirectory, { mode: 0o700 });
          const row = { id, command, directoryState, environment, status: 'not-run' };
          report.rows.push(row);
          const candidate = writeNew(path.join(rowDirectory, 'candidate.tree.json'), source.content);
          const html = path.join(rowDirectory, 'diagram.html');
          const evidence = path.join(rowDirectory, 'evidence');
          let sentinel;
          if (directoryState === 'precreated') {
            fs.mkdirSync(evidence, { mode: 0o700 });
            sentinel = writeNew(path.join(evidence, 'unrelated.txt'), 'Preserve this preexisting file.\n');
          }
          if (command !== 'finalize') {
            const delivered = await invoke(rowDirectory, 'deliver', ['deliver', 'tree', candidate.path, html, '--json']);
            row.deliver = { ...delivered }; delete row.deliver.value;
            assert.equal(delivered.exitCode, 0, `Delivery precondition failed: ${id}`);
            assert.equal(delivered.value.status, 'pass');
            row.artifactBefore = bound(readSnapshot(html));
          }
          row.status = 'failed';
          const prefix = environment === 'unavailable' ? ['/usr/bin/sandbox-exec', '-f', report.unavailableProfile.path] : [];
          const childEnvironment = environment === 'launch-failed' ? { PHOENIX_DIAGRAM_BROWSER: '/usr/bin/false' } : {};
          const args = command === 'finalize'
            ? [command, 'tree', candidate.path, html, '--out-dir', evidence, '--json']
            : [command, html, '--out-dir', evidence, '--json'];
          const result = await invoke(rowDirectory, command, args, { prefix, environment: childEnvironment });
          row.invocation = { ...result }; delete row.invocation.value;
          const receipt = result.value;
          const expectedStatus = environment === 'unavailable' ? 'unavailable' : 'failed';
          const expectedCode = environment === 'unavailable' ? 'browser/unavailable' : 'browser/launch';
          assert.equal(result.exitCode, environment === 'unavailable' ? 5 : 4, id);
          assert.equal(receipt.status, expectedStatus, id);
          assert(receipt.diagnostics.some(entry => entry.code === expectedCode), `Original diagnostic missing: ${id}`);
          const browser = command === 'finalize' ? receipt.stages[3].receipt : receipt;
          assert.equal(browser.status, expectedStatus, id);
          assert.deepEqual(browser.screenshots, [], id);
          assert.deepEqual(browser.coverage, [], id);
          if (command === 'finalize') {
            assert(receipt.stages.slice(0, 3).every(stage => stage.status === 'pass'), id);
            assert.equal(receipt.stages[3].status, expectedStatus, id);
            assert.equal(receipt.producedByThisRun, true, id);
            assert.equal(receipt.deliveryStatus, 'incomplete', id);
            assert.equal(receipt.visualReview.status, 'required-pending', id);
            const summary = readBound(receipt.evidence.summaryReceipt, 'failure summary');
            row.summary = bound(summary);
            const expectedSummary = structuredClone(receipt);
            delete expectedSummary.evidence.summaryReceipt;
            assert.deepEqual(JSON.parse(summary.content), expectedSummary, id);
          }
          row.candidate = bound(readBound(candidate, 'failure candidate'));
          row.artifact = bound(readSnapshot(html));
          if (row.artifactBefore) readBound(row.artifactBefore, 'prior delivered HTML');
          assert.deepEqual(browser.artifact, row.artifact.identity, id);
          assert.deepEqual(browser.candidate.identity, candidate.identity, id);
          const sealed = readBound(receipt.evidence.browserReceipt, 'failure browser receipt');
          row.browserReceipt = bound(sealed);
          assert.equal(path.dirname(sealed.path), fs.realpathSync(evidence), id);
          const expectedBrowser = { ...browser }; delete expectedBrowser.evidence;
          assert.deepEqual(JSON.parse(sealed.content), expectedBrowser, id);
          assert(fs.readdirSync(evidence).every(name => !name.startsWith('phoenix-diagram-browser-evidence-') && !name.endsWith('.png')), `Unexpected screenshots or screenshot stages: ${id}`);
          if (sentinel) row.preserved = bound(readBound(sentinel, 'preexisting evidence file'));
          const strict = await invoke(rowDirectory, 'strict-check', ['check', html, '--strict-provenance', '--json']);
          row.strict = { ...strict }; delete row.strict.value;
          assert.equal(strict.exitCode, 0, id);
          assert.equal(strict.value.provenance, 'current', id);
          row.status = 'pass';
          row.record = writeNew(path.join(rowDirectory, 'result.json'), { ...row });
        }
      }
    }
    assert.equal(report.rows.length, 12);
    readBound(report.fixture, 'original minimal tree fixture');
    report.status = 'pass';
  } catch (error) {
    report.status = report.rows.length ? 'failed' : 'not-run';
    report.diagnostic = { code: error.code || 'gaps/browser-failures', message: error.message };
    failure = error;
  }
  report.record = writeNew(path.join(directory, 'result.json'), { ...report });
  if (failure) { failure.evidence = report.record; throw failure; }
  return report;
}
