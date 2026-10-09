#!/usr/bin/env node
/** Strict replay of every original oracle case; original golden files are read only. */
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';
import { verifyCorpus, loadPhoenixCases, readGolden, phoenixExpected, diffResults,
  caseRunsHere, metadata, sha256, ROOT } from '../tools/phoenix-ui/tests/oracle/adapters/phoenix.mjs';

export async function checkOracle({ bin, prefix = '', reportActual = false, progress = () => {} }) {
  if (!bin) throw new Error('explicit --bin or PHOENIX_UI_BIN is required; no download or old-runtime fallback');
  bin = fs.realpathSync(path.resolve(bin));
  const binarySha256 = sha256(fs.readFileSync(bin));
  const corpus = verifyCorpus();
  const adapterSources = ['scripts/check-phoenix-oracle.mjs',
    'tools/phoenix-ui/tests/oracle/adapters/phoenix.mjs',
    'tools/phoenix-ui/tests/oracle/adapters/network.mjs',
    'tools/phoenix-ui/tests/oracle/adapters/phoenix.json',
    'scripts/phoenix-build-tools.mjs'].map((file) => ({ path:file, sha256:sha256(fs.readFileSync(path.join(ROOT,file))) }));
  const probe = spawnSync(bin, ['engine-probe'], { encoding: 'utf8', timeout: 10000 });
  if (probe.error || probe.status !== 0 || probe.signal || probe.stdout !== 'phoenix-ui-engine 0.1.0\n' || probe.stderr) throw new Error('binary is not the expected Phoenix 0.1.0 engine');
  const { cases: all, runCase } = await loadPhoenixCases(bin);
  const cases = all.filter((c) => c.id.startsWith(prefix));
  if (!cases.length) throw new Error(`no original oracle cases match prefix: ${prefix}`);
  const report = {
    schemaVersion: 1, recordedAt: new Date().toISOString(), platform: process.platform, arch: process.arch,
    runtime: { path: path.relative(ROOT, bin), sha256: binarySha256, probe: probe.stdout.trim() },
    corpus, adapterSha256: sha256(JSON.stringify(metadata)), adapterSources,
    adapterSourceSha256: sha256(JSON.stringify(adapterSources)), prefix, fullCorpus: !prefix,
    totalCases: all.length, selectedCases: cases.length,
    counts: { pass: 0, adaptedPass: 0, fail: 0, error: 0, missing: 0, skipped: 0 }, cases: [],
  };
  for (const c of cases) {
    const item = { id: c.id, sourceFile: c.sourceFile };
    if (!caseRunsHere(c)) {
      item.status = 'skipped'; item.platforms = c.platforms; report.counts.skipped++;
    } else {
      const golden = readGolden(c.id);
      if (!golden) { item.status = 'missing'; report.counts.missing++; }
      else {
        try {
          const expected = phoenixExpected(c.id, golden), actual = runCase(c, { impl: 'bin', bin });
          const outputs = actual.steps || [actual];
          if (outputs.some((r) => !r.daemon && (r.exit === null || r.signal))) throw new Error(`process failed: ${JSON.stringify(outputs.map(({ exit, signal }) => ({ exit, signal })))}`);
          const differences = diffResults(expected, actual);
          if (differences.length) {
            item.status = 'fail'; item.differences = differences; report.counts.fail++;
            if (reportActual) { item.actual = actual; item.expected = expected; }
          } else {
            const adapted = JSON.stringify(golden) !== JSON.stringify(expected);
            item.status = adapted ? 'adapted-pass' : 'pass';
            report.counts.pass++; if (adapted) report.counts.adaptedPass++;
          }
        } catch (error) { item.status = 'error'; item.error = error.message; report.counts.error++; }
      }
    }
    report.cases.push(item); progress(item, report);
  }
  report.ok = report.counts.fail === 0 && report.counts.error === 0 && report.counts.missing === 0;
  if (sha256(fs.readFileSync(bin)) !== binarySha256) throw new Error('runtime binary changed during oracle replay');
  for (const input of adapterSources) if (sha256(fs.readFileSync(path.join(ROOT,input.path))) !== input.sha256) throw new Error(`oracle adapter changed during replay: ${input.path}`);
  report.acceptanceComplete = report.ok && report.fullCorpus && report.counts.skipped === 0;
  return report;
}

async function main() {
  const args = process.argv.slice(2);
  let bin = process.env.PHOENIX_UI_BIN, prefix = '', json = false, reportActual = false;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--bin') bin = args[++i];
    else if (arg === '--prefix') prefix = args[++i];
    else if (arg === '--json') json = true;
    else if (arg === '--include-actual') reportActual = true;
    else if (arg === '--help') {
      process.stdout.write('Usage: node scripts/check-phoenix-oracle.mjs --bin <current-engine> [--prefix <case-prefix>] [--json] [--include-actual]\nNo accepted-case suppression. Missing, execution errors and differences fail; platform skips are reported and prevent complete acceptance.\n');
      return;
    } else throw new Error(`unknown argument: ${arg}`);
  }
  const report = await checkOracle({ bin, prefix, reportActual,
    progress: json ? undefined : (c) => { if (['fail', 'error', 'missing', 'skipped'].includes(c.status)) process.stdout.write(`${c.status}: ${c.id}\n${(c.differences || [c.error || '']).join('\n')}\n`); } });
  process.stdout.write(json ? JSON.stringify(report, null, 2) + '\n' : `${report.counts.pass} pass (${report.counts.adaptedPass} exact adaptations), ${report.counts.fail} fail, ${report.counts.error} errors, ${report.counts.missing} missing, ${report.counts.skipped} platform skips; ${report.selectedCases}/${report.totalCases} cases\n`);
  process.exitCode = report.ok ? 0 : 1;
}

if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  main().catch((error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
}
