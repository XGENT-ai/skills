#!/usr/bin/env node
/**
 * Replay the corpus against a phoenix-ui binary and diff against the goldens.
 *   node tools/phoenix-ui/tests/oracle/run.mjs --bin <phoenix-ui> [id-prefix]
 * The binary defaults to $PHOENIX_UI_BIN. Exit 1 on any difference or
 * missing golden.
 */
import { allCases, runCase, readGolden, diffResults, caseRunsHere, parseArgs } from './lib.mjs';

const { bin, prefix } = parseArgs(process.argv.slice(2));
const cases = (await allCases()).filter(c => c.id.startsWith(prefix));
if (!cases.length) throw new Error(`no oracle case id starts with: ${prefix}`);
let pass = 0, fail = 0, missing = 0, skipped = 0;
for (const c of cases) {
  if (!caseRunsHere(c)) { skipped++; process.stdout.write(`-- ${c.id}: skipped (platforms: ${c.platforms.join(', ')})\n`); continue; }
  const golden = readGolden(c.id);
  if (!golden) { missing++; process.stdout.write(`?? ${c.id}: no golden (run record.mjs)\n`); continue; }
  let diffs;
  try { diffs = diffResults(golden, runCase(c, { bin })); } catch (error) { diffs = [error.message]; }
  if (!diffs.length) { pass++; continue; }
  fail++;
  process.stdout.write(`XX ${c.id}\n${diffs.map(d => '   ' + d.replace(/\n/g, '\n   ')).join('\n')}\n`);
}
process.stdout.write(`\n${pass} pass, ${fail} fail, ${missing} missing goldens${skipped ? `, ${skipped} skipped on ${process.platform}` : ''}\n`);
process.exitCode = fail || missing ? 1 : 0;
