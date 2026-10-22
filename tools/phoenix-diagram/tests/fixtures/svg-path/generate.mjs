// Development-only oracle. Production SVG inspection uses the Rust parser.
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

const root = fileURLToPath(new URL('../../../../../', import.meta.url));
verifyFrozenSources({ localReference: path.join(root, 'local/phoenix-diagram/reference') });
const source = 'local/phoenix-diagram/reference/archify/renderers/shared/svg-path-analysis.mjs';
const { analyzeSvgPath } = await import(pathToFileURL(path.join(root, source)).href);
const inputs = [
  ['empty', ''],
  ['ascii-whitespace', ' \t\r\n'],
  ['move-only', 'M 1 2'],
  ['absolute-line', 'M 0 0 L 20 30'],
  ['implicit-moveto-lines', 'M 1 2 3 4 5 6'],
  ['relative-moveto-lines', 'm 1 2 3 4 -2 -1'],
  ['relative-lines', 'M 10 20 l 4 -2 -1 3'],
  ['horizontal-vertical', 'M 1 2 H 4 8 V 10 -1 h -3 2 v 9 -4'],
  ['relative-moveto-after-line', 'M 1 2 L 4 5 m 2 3 4 5'],
  ['signed-adjacent-numbers', 'M+1-2L-3+4'],
  ['decimal-adjacent-numbers', 'M.5.25L1.2.3'],
  ['decimal-exponent', 'M+1.e2-.25E+2L2.5e-1+3E1'],
  ['commas-and-whitespace', 'M 1 , 2, 3,4 L5, 6'],
  ['zero-length-line', 'M 1 1 L 1 1'],
  ['negative-zero', 'M -0 -0 L +0 0'],
  ['underflow', 'M 1e-999 -1e-999 L 0 0'],
  ['extreme-finite-span', 'M -1e308 0 L 1e308 0'],
  ['closepath', 'M 1 2 L 5 6 Z'],
  ['relative-closepath', 'm 1 2 l 4 4 z'],
  ['close-move-only', 'M 1 2Z'],
  ['close-twice', 'M 1 2L3 4ZZ'],
  ['line-after-close', 'M 1 2 L 3 4 Z L 7 8'],
  ['relative-line-after-close', 'M 1 2 L 3 4 Z l 7 8'],
  ['quadratic-after-close', 'M 1 2 L 3 4 Z Q 7 8 9 10'],
  ['relative-move-after-close', 'M 1 2 L 3 4 Z m 7 8 l 1 2'],
  ['disconnected-subpaths', 'M 0 0 L 10 0 M 100 0 L 110 0'],
  ['consecutive-moves', 'M 0 0 M 10 10 m 5 5'],
  ['quadratic', 'M 0 0 Q 10 20 20 0'],
  ['relative-quadratic', 'M 4 5 q 10 20 20 0'],
  ['repeated-quadratic', 'M 0 0 Q 10 20 20 0 30 -20 40 0'],
  ['repeated-relative-quadratic', 'm 4 5 q 10 20 20 0 10 -20 20 0'],
  ['collinear-quadratic', 'M 0 0 Q 10 10 20 20'],
  ['collinear-quadratic-reversal', 'M 0 0 Q 30 30 20 20'],
  ['zero-endpoint-quadratic', 'M 0 0 Q 10 10 0 0'],
  ['near-collinear-quadratic', 'M 0 0 Q 1 .0000000001 2 0'],
  ['non-collinear-quadratic', 'M 0 0 Q 1 .000000001 2 0'],
  ['curved-only-subpath', 'M 0 0 Q 10 20 20 0 M 50 0 L 60 0'],
  ['curve-between-straight-parts', 'M 0 0 L 10 0 Q 20 0 20 10 L 20 20'],
  ['invalid-first-number', '1 2'],
  ['invalid-first-line', 'L 1 2'],
  ['invalid-first-close', 'Z'],
  ['invalid-first-unsupported', 'C 1 2 3 4 5 6'],
  ['unknown-command', 'M 0 0 R 1 2'],
  ['invalid-character', 'M 0 0 @'],
  ['unicode-character', 'M 0 0 🦀'],
  ['non-ascii-whitespace', 'M 0\u00a00'],
  ['form-feed', 'M 0\f0'],
  ['bare-sign', 'M 0 +'],
  ['bare-decimal-point', 'M 0 .'],
  ['missing-exponent', 'M 0 1e'],
  ['missing-exponent-after-sign', 'M 0 1e+'],
  ['double-exponent', 'M 0 1e2e3'],
  ['nan', 'M 0 NaN'],
  ['infinity', 'M 0 Infinity'],
  ['overflow-number', 'M 0 1e309'],
  ['negative-overflow-number', 'M 0 -1e309'],
  ['overflow-relative-move', 'M 1e308 0 m 1e308 0'],
  ['overflow-relative-horizontal', 'M 1e308 0 h 1e308'],
  ['overflow-relative-vertical', 'M 0 -1e308 v -1e308'],
  ['overflow-relative-control', 'M 1e308 0 q 1e308 0 -1 1'],
  ['overflow-relative-end', 'M 1e308 0 q -1 1 1e308 0'],
  ['extreme-finite-sampled-coordinate', 'M 1.7976931348623157e308 0 Q 1.7976931348623157e308 0 1.7976931348623157e308 0'],
  ['numbers-after-close', 'M 0 0 Z 1 2'],
  ['trailing-move', 'M 0 0 M'],
  ['trailing-line', 'M 0 0 L'],
  ['interrupted-parameters', 'M 0 0 Q 1 L 2 3'],
  ['leading-comma', ',M 0 0'],
  ['comma-after-command', 'M,0 0'],
  ['repeated-comma', 'M 0,,0'],
  ['trailing-comma', 'M 0 0,'],
  ['comma-before-command', 'M 0 0,L 1 2'],
  ['comma-after-close', 'M 0 0 Z,'],
  ['unsupported-before-lexical-error', 'M 0 0 C 1 2 @'],
];
for (const command of 'CcAaSsTt') inputs.push([`unsupported-${command}`, `M 0 0 ${command} 1 2 3 4 5 6 7`]);
for (const [command, arity] of [['M', 2], ['L', 2], ['H', 1], ['V', 1], ['Q', 4]]) {
  for (let count = 0; count < arity; count += 1) {
    const prefix = command === 'M' ? '' : 'M 0 0 ';
    inputs.push([`incomplete-${command}-${count}`, `${prefix}${command} ${Array(count).fill('1').join(' ')}`]);
  }
}

// This is the source checker's continuousRoutePoints projection, not metadata.
function routeSubpaths(subpaths) {
  return subpaths.flatMap(({ straightSegments }) => {
    const parts = [];
    for (const { start, end } of straightSegments) {
      const previous = parts.at(-1);
      if (previous && previous.at(-1)[0] === start[0] && previous.at(-1)[1] === start[1]) previous.push(end);
      else parts.push([start, end]);
    }
    return parts.length ? parts : [[]];
  });
}

const cases = inputs.map(([name, d]) => {
  const result = analyzeSvgPath(d);
  return { name, d, expected: result, ...(result.ok ? { routeSubpaths: routeSubpaths(result.subpaths) } : {}) };
});
const sourceSha256 = createHash('sha256').update(await readFile(path.join(root, source))).digest('hex');
await writeFile(new URL('./oracle.json', import.meta.url), `${JSON.stringify({ format: 1, source, sourceSha256, cases }, null, 2)}\n`);
console.log(`Wrote ${cases.length} SVG path oracle cases; ${cases.filter(testCase => testCase.expected.ok).length} accepted.`);
