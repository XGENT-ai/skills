// Maintenance oracle. Evaluates the frozen timeline and waterfall formatting
// functions themselves on the pinned Node, so V8 and ICU decide every value.
import assert from 'node:assert/strict';
import { readFile, writeFile, realpath } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

assert.equal(process.version, 'v25.2.1', 'Use the pinned maintenance Node');
assert.equal(process.versions.icu, '77.1', 'The oracle is bound to ICU 77.1');
assert.equal(process.versions.tz, '2025b', 'The oracle is bound to tzdata 2025b');
const root = fileURLToPath(new URL('../../../../../', import.meta.url));
const reference = await realpath(path.join(root, 'local/phoenix-diagram/reference'));
verifyFrozenSources({ localReference: reference });
const renderer = async name => readFile(path.join(reference, 'archify/renderers', name), 'utf8');
const between = (source, start, end) => {
  const from = source.indexOf(start);
  const to = source.indexOf(end, from);
  assert(from >= 0 && to > from, `${start} not found in the frozen renderer`);
  return source.slice(from, to);
};

const timelineSource = await renderer('timeline/render-timeline.mjs');
const waterfallSource = await renderer('waterfall/render-waterfall.mjs');
const zoneFunctions = new Function('timezone', 'MINUTE',
  `${between(timelineSource, 'const partsFormat', '// ---- Events in time order')}\nreturn { wall, zoneOffset, offsetLabel };`);
const validCalendarDate = new Function(`${between(timelineSource, 'function validCalendarDate', 'const timezone =')}\nreturn validCalendarDate;`)();
const waterfallNumber = new Function(`${between(waterfallSource, 'function number(value)', 'const formatDuration')}\nreturn number;`)();
const accepted = timezone => {
  try { new Intl.DateTimeFormat('en-US', { timeZone: timezone }); return true; } catch { return false; }
};

const systemVNames = ['SystemV/AST4ADT', 'SystemV/CST6CDT', 'SystemV/EST5EDT',
  'SystemV/MST7MDT', 'SystemV/PST8PDT', 'SystemV/YST9YDT'];
const zoneinfo = await readFile(new URL('./icu77-zoneinfo64.txt', import.meta.url), 'utf8');
const vector = (source, name) => {
  const match = source.match(new RegExp(`${name}:intvector \\{([^}]+)\\}`));
  assert(match, `Missing ICU vector ${name}`);
  return match[1].split(',').map(value => Number(value.trim()));
};
const systemV = systemVNames.map(name => {
  const start = zoneinfo.indexOf(`/* ${name} */ :table {`);
  assert(start >= 0, `Missing ICU zone ${name}`);
  const source = zoneinfo.slice(start, zoneinfo.indexOf('} //Z#', start));
  const raw = Number(source.match(/finalRaw:int \{ (-?\d+) \}/)?.[1]);
  assert.equal(Number(source.match(/finalYear:int \{ (\d+) \}/)?.[1]), 1977);
  assert(source.includes('finalRule { "SystemV" }'));
  assert.deepEqual(vector(source, 'typeOffsets'), [raw, 0, raw, 3600]);
  const transitions = vector(source, 'trans');
  assert.equal(transitions.length, 150);
  assert(transitions.every((value, index) => Number.isSafeInteger(value) && (!index || value > transitions[index - 1])));
  assert.equal(source.match(/typeMap:bin \{ "([0-9]+)" \}/)?.[1], '0100'.repeat(75));
  return { name, raw, transitions, standardTransitions: transitions.map(value => value + raw) };
});
for (const zone of systemV) assert.deepEqual(zone.standardTransitions, systemV[0].standardTransitions, zone.name);
assert.deepEqual(vector(zoneinfo.slice(zoneinfo.indexOf('  SystemV:intvector {')), 'SystemV'),
  [3, -30, -1, 7200, 0, 9, -31, -1, 7200, 0, 3600]);

// Only the historical data is generated; the bounded Gregorian rule stays in Rust.
const table = systemV[0].standardTransitions;
const rustTable = `const TRANSITIONS: [i64; ${table.length}] = [\n${table.map(value => `    ${value},`).join('\n')}\n];`;
const rustFile = new URL('../../../src/format/time/system_v.rs', import.meta.url);
const rustSource = await readFile(rustFile, 'utf8');
assert.match(rustSource, /const TRANSITIONS: \[i64; \d+\] = \[[\s\S]*?\n\];/);
const generatedRust = rustSource.replace(/const TRANSITIONS: \[i64; \d+\] = \[[\s\S]*?\n\];/, rustTable);
if (process.argv.includes('--check')) assert.equal(rustSource, generatedRust, 'SystemV Rust transitions');
else await writeFile(rustFile, generatedRust);

function lastSunday(year, month) {
  const date = new Date(0);
  date.setUTCFullYear(year, month, 0);
  date.setUTCHours(2, 0, 0, 0);
  date.setUTCDate(date.getUTCDate() - date.getUTCDay());
  return date.getTime();
}

function systemVInstants(zone) {
  const instants = new Set();
  const boundary = ms => { for (const delta of [-1, 0, 1]) instants.add(ms + delta); };
  for (const seconds of zone.transitions) boundary(seconds * 1000);
  boundary(Date.UTC(1977, 0, 1) - zone.raw * 1000);
  for (const year of [...Array.from({ length: 400 }, (_, index) => 1977 + index), 2400, 2800, 9999]) {
    boundary(lastSunday(year, 4) - zone.raw * 1000);
    boundary(lastSunday(year, 10) - (zone.raw + 3600) * 1000);
    for (const month of [0, 6]) instants.add(Date.UTC(year, month, 15));
  }
  for (const at of ['0000-01-01T00:00Z', '0000-02-29T12:00Z', '0001-01-01T00:00Z',
    '1976-12-31T23:59:59.999Z', '1977-01-01T00:00Z', '9999-12-31T23:59:59.999Z']) instants.add(Date.parse(at));
  return instants;
}

// Deterministic magnitudes across the double range, plus the boundary list.
let seed = 0x2f6b9d1;
const random = () => ((seed = (Math.imul(seed, 1103515245) + 12345) >>> 0) / 2 ** 32);
const numbers = [1.005, 2.675, 0.125, 0.005, -0.5, 1234567.891, 0.001235, 0.0012345, 1.2345e-7, 1.235e-7,
  0, -0, 0.5, 1.5, 2.5, -1.5, -2.5, 0.045, 0.0045, 0.015, 0.025, 9.995, 99.995, 999.995, 0.0099999, 0.00999, 0.01, 0.009995,
  1e-6, 0.000001, 9.99e-7, 9.995e-7, 1.0000001e-6, 1e-7, 5e-324, 2.2250738585072014e-308, 1e21, 9.999999999999999e20, 123456789012345680000,
  1e300, 1.7976931348623157e308, 1.7976931348623157e306, 1.7976931348623158e306, 4503599627370495.5, 4503599627370497, 9007199254740993,
  0.1, 0.2, 0.30000000000000004, 1 / 3, 2 / 3, 100, 1000, 999999.995, 1000000, 12345.678, 0.0005, 0.00049999999999999999, 1e-5, 0.00012345,
  42, 7.5, 0.75, 320.5, 1500, 2350, 0.95, 0.05, 1e15, 1e16, 1.5e-10, 3.14159, 271.828, 0.333, 66.666666];
for (let index = 0; index < 400; index += 1) {
  const exponent = Math.floor(random() * 40) - 20;
  const value = Number((random() * 10 ** exponent).toPrecision(1 + Math.floor(random() * 17)));
  numbers.push(index % 7 === 0 ? -value : value);
}
// JSON number parsing need not round-trip in every reader; the bits are exact.
const bits = value => { const view = new DataView(new ArrayBuffer(8)); view.setFloat64(0, value); return view.getBigUint64(0).toString(16).padStart(16, '0'); };
const numberCases = numbers.map(value => ({
  bits: bits(value),
  string: String(value),
  precision: Object.fromEntries([1, 2, 3, 5, 10, 21].map(p => [p, value.toPrecision(p)])),
  fraction2: value.toLocaleString('en-US', { maximumFractionDigits: 2 }),
  significant3: value.toLocaleString('en-US', { maximumSignificantDigits: 3 }),
  waterfall: waterfallNumber(value),
}));

const timestamps = ['2026-10-01T10:05:00+08:00', '2026-10-01T10:05Z', '2026-10-01T10:05:07.5Z', '2026-10-01T10:05:07.123-03:30',
  '2026-10-01T24:00Z', '2026-10-01T24:00:00Z', '2026-10-01T24:00:00.000Z', '2026-10-01T24:00:01Z', '2026-10-01T24:00:00.001Z', '2026-10-01T23:60Z',
  '2026-10-01T23:59:60Z', '2026-10-01T10:00+24:00', '2026-10-01T10:00+23:59', '2026-10-01T10:00-23:59', '2026-10-01T10:00+05:60',
  '2026-02-30T10:00Z', '2026-02-29T10:00Z', '2024-02-29T10:00Z', '1900-02-29T10:00Z', '2000-02-29T10:00Z', '2026-04-31T10:00Z', '2026-13-01T10:00Z',
  '2026-00-10T10:00Z', '2026-01-00T10:00Z', '2026-01-32T10:00Z', '0000-01-01T00:00Z', '0000-01-01T00:00+05:30', '0000-02-29T12:00Z',
  '9999-12-31T23:59:59.999Z', '9999-12-31T23:59-14:00', '1970-01-01T00:00Z', '1969-12-31T23:59:59.999Z', '2026-03-08T02:30-05:00',
  '2026-10-01T10:05:00.1234Z', '2026-10-01T10:05:00.Z', '2026-10-01T1:05Z', '2026-10-01T10:05'];
// The engine parses only what the timeline schema admits; other strings are
// recorded to show where V8 is more permissive than the input contract.
const schema = JSON.parse(await readFile(path.join(root, 'skills/phoenix-diagram/schemas/timeline.schema.json'), 'utf8'));
const pattern = new RegExp(schema.properties.events.items.properties.at.pattern, 'u');
const timestampCases = timestamps.map(at => {
  const parsed = Date.parse(at);
  return { at, schema: pattern.test(at), ms: Number.isFinite(parsed) ? parsed : null, calendar: validCalendarDate(at) };
});

const zoneNames = ['UTC', 'utc', 'Utc', 'Etc/UTC', 'GMT', 'Asia/Shanghai', 'asia/shanghai', 'ASIA/SHANGHAI', 'America/New_York',
  'america/new_york', 'AMERICA/NEW_YORK', 'Europe/London', 'europe/london', 'Asia/Calcutta', 'asia/calcutta', 'Asia/Kolkata',
  'Australia/Lord_Howe', 'America/St_Johns', 'Asia/Kathmandu', 'Pacific/Chatham', 'Africa/Casablanca', 'America/Sao_Paulo',
  'Europe/Dublin', 'Pacific/Apia', 'Etc/GMT+5', 'Etc/GMT-14', 'EST', 'US/Pacific', 'IST', 'PST', 'CST', 'SystemV/EST5',
  'Canada/East-Saskatchewan', 'US/Pacific-New',
  '+05:30', '+0530', '+01', '-2359', '−05:00', '-00:00', '+00', '-00', '+23:59', '−0130', '+14:00', '-12', '+00:00'];
zoneNames.push(...systemVNames.flatMap(name => [name, name.toLowerCase(), name.toUpperCase()]));
const rejectedZones = ['+24:00', '+05:60', '+5:30', '+05:30:00', 'Z', 'UTC+05:30', ' +05:30', '+05:30 ', ' UTC', 'UTC ', '',
  'Mars/Phobos', 'SystemV/AST4EDT', 'SystemV/EST5EDT ', '+05:3', '+0530:', '05:30', '+05.5', '+0530.0', '--05:00', '+-05:00', '−−05:00'];
const scanned = new Set(['UTC', 'Asia/Shanghai', 'America/New_York', 'Europe/London', 'Asia/Calcutta', 'Australia/Lord_Howe',
  'America/St_Johns', 'Asia/Kathmandu', 'Pacific/Chatham', 'Africa/Casablanca', 'America/Sao_Paulo', 'Europe/Dublin', 'Pacific/Apia',
  'US/Pacific', 'IST', 'PST', 'CST', 'Canada/East-Saskatchewan', 'US/Pacific-New']);
const offsetOf = (zone, ms) => zone.zoneOffset(ms);
// Find every offset change by scanning and bisecting, then sample both sides.
function transitions(zone, from, to) {
  const found = [];
  const step = 20 * 86_400_000;
  for (let t = from; t < to; t += step) {
    if (offsetOf(zone, t) === offsetOf(zone, t + step)) continue;
    let [low, high] = [t, t + step];
    while (high - low > 1000) {
      const middle = low + Math.floor((high - low) / 2000) * 1000;
      if (offsetOf(zone, middle) === offsetOf(zone, low)) low = middle; else high = middle;
    }
    found.push(high);
  }
  return found;
}
const span = (from, to) => [Date.UTC(from, 0, 1), Date.UTC(to, 0, 1)];
const zoneCases = [];
for (const timezone of [...zoneNames, ...rejectedZones]) {
  const entry = { timezone, accepted: accepted(timezone) };
  if (entry.accepted) {
    const zone = zoneFunctions(timezone, 60_000);
    const instants = new Set([-62167219200000, -62135596800000, -62198755200000, -59011459201000, 0, -1, -500, 1772964000000,
      Date.parse('9999-12-31T23:59:59.999Z'), Date.parse('1900-01-01T00:00Z'), Date.parse('2050-07-01T12:00Z'), Date.parse('2099-01-15T12:00Z'),
      Date.parse('2099-07-15T12:00Z'), Date.parse('2026-10-01T10:05:07.5Z')]);
    const systemZone = systemV.find(zone => zone.name === timezone);
    if (systemZone) for (const ms of systemVInstants(systemZone)) instants.add(ms);
    const named = !/^[+\-−]/.test(timezone) && !timezone.toLowerCase().startsWith('systemv/');
    if (named && timezone === timezone.toUpperCase() ? false : named) {
      for (const t of transitions(zone, ...span(1880, 2041))) for (const delta of [-1000, -1, 0, 1000]) instants.add(t + delta);
    }
    entry.samples = [...instants].sort((a, b) => a - b).map(ms => {
      const w = zone.wall(ms);
      return [ms, `${w.y}-${w.mo}-${w.d} ${w.h}:${w.mi}:${w.s}`, zone.offsetLabel(ms)];
    });
  }
  zoneCases.push(entry);
}

const oracle = { format: 1, node: process.version, tz: process.versions.tz, icu: process.versions.icu,
  numbers: numberCases, timestamps: timestampCases, zones: zoneCases };
const file = new URL('./oracle.json', import.meta.url);
const bytes = `${JSON.stringify(oracle)}\n`;
if (process.argv.includes('--check')) assert.equal(await readFile(file, 'utf8'), bytes, 'oracle.json');
else await writeFile(file, bytes);
console.log(`${process.argv.includes('--check') ? 'Verified' : 'Wrote'} ${numberCases.length} numbers, ${timestampCases.length} timestamps, ${zoneCases.length} zones (${zoneCases.reduce((sum, zone) => sum + (zone.samples?.length ?? 0), 0)} instants).`);
