#!/usr/bin/env node
/**
 * Record goldens from a phoenix-ui binary.
 *   node tools/phoenix-ui/tests/oracle/record.mjs --bin <phoenix-ui> <id-prefix>
 * The binary defaults to $PHOENIX_UI_BIN. Record only the cases you added or
 * whose behavior changed on purpose, then review each golden diff by hand: the
 * binary under test is also the recorder, so a bug would be frozen too.
 */
import { allCases, runCase, writeGolden, parseArgs } from './lib.mjs';

const { bin, prefix } = parseArgs(process.argv.slice(2));
const cases = (await allCases()).filter(c => c.id.startsWith(prefix));
if (!cases.length) throw new Error(`no oracle case id starts with: ${prefix}`);
for (const c of cases) {
  const res = runCase(c, { bin });
  writeGolden(c.id, res);
  const head = res.steps ? `${res.steps.length} steps, exits ${res.steps.map(s => s.exit).join('/')}` : `exit ${res.exit}, ${res.stdout.length}b out`;
  process.stdout.write(`recorded ${c.id} (${head}, ${Object.keys(res.files).length} files)\n`);
}
process.stdout.write(`\n${cases.length} goldens written\n`);
