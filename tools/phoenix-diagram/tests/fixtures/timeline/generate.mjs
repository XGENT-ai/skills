// Maintenance oracle. Executes the frozen timeline renderer: its layout report
// is the unrounded scene, its SVG the rendered artifact.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

assert.equal(process.version, 'v25.2.1', 'Use the pinned maintenance Node');
assert.equal(process.versions.tz, '2025b', 'The oracle is bound to tzdata 2025b');
const root = fileURLToPath(new URL('../../../../../', import.meta.url));
const reference = await realpath(path.join(root, 'local/phoenix-diagram/reference'));
verifyFrozenSources({ localReference: reference });
const source = 'local/phoenix-diagram/reference/archify/renderers/timeline/render-timeline.mjs';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const example = async name => JSON.parse(await readFile(path.join(reference, 'archify/examples', name), 'utf8'));

const documents = [];
const add = (name, document, phoenix) => documents.push({ name, document, phoenix });
const base = (events, meta = {}, extra = {}) => ({ schema_version: 1, diagram_type: 'timeline',
  meta: { title: 'Timeline oracle', output: 'timeline.html', evidence: 'observed', quality_profile: 'showcase', ...meta }, ...extra, events });
const incident = await example('payment-incident.timeline.json');
add('payment-incident', incident);
add('archify-dev-activity', await example('archify-dev-activity.timeline.json'));
for (const timezone of ['+05:30', '+0530', '+01', '-2359', '−05:00', '-00:00', 'UTC', 'utc', 'asia/calcutta', 'Asia/Calcutta', 'IST',
  'SystemV/AST4ADT', 'SystemV/CST6CDT', 'SystemV/EST5EDT', 'SystemV/MST7MDT', 'SystemV/PST8PDT', 'SystemV/YST9YDT']) {
  const document = structuredClone(incident);
  document.meta.timezone = timezone;
  add(`incident-zone-${timezone.replace(/[^\w]/g, c => `u${c.codePointAt(0).toString(16)}`)}`, document);
}
const newYork = [['a', '2026-03-08T01:30:00-05:00'], ['b', '2026-03-08T01:59:00-05:00'], ['c', '2026-03-08T03:00:00-04:00'], ['d', '2026-03-08T04:15:00-04:00']];
add('dst-spring-new-york', base(newYork.map(([id, at]) => ({ id, at, title: `Event ${id}` })), { timezone: 'America/New_York' }));
add('dst-autumn-london', base([['a', '2026-10-25T00:30:00Z'], ['b', '2026-10-25T01:00:00Z'], ['c', '2026-10-25T01:30:00Z'], ['d', '2026-10-25T03:00:00Z']]
  .map(([id, at]) => ({ id, at, title: `Clock ${id}`, kind: id === 'b' ? 'alert' : 'default' })), { timezone: 'Europe/London' }));
add('lord-howe-half-hour-dst', base([['a', '2026-04-04T14:00:00Z'], ['b', '2026-04-04T15:10:00Z'], ['c', '2026-04-04T16:20:00Z']]
  .map(([id, at]) => ({ id, at, title: `Shift ${id}` })), { timezone: 'Australia/Lord_Howe' }));
add('breaks-auto', base([0, 1, 2, 3, 4, 300, 301, 302, 2000, 2001].map((minute, index) => ({
  id: `e${index}`, at: new Date(Date.UTC(2026, 6, 1, 8) + minute * 60_000).toISOString().replace('.000Z', 'Z'), title: `Step ${index}`,
  kind: ['default', 'change', 'alert', 'action', 'recovery'][index % 5] }))));
add('breaks-none', { ...base([0, 1, 2, 300, 301].map((minute, index) => ({ id: `e${index}`, at: new Date(Date.UTC(2026, 6, 1, 8) + minute * 60_000).toISOString().replace('.000Z', 'Z'), title: `Step ${index}` }))), layout: { breaks: 'none', width: 480 } });
add('many-breaks-budget', base(Array.from({ length: 12 }, (_, index) => ({ id: `e${index}`,
  at: new Date(Date.UTC(2026, 0, 1) + index * index * index * 3_600_000).toISOString().replace('.000Z', 'Z'), title: `Release ${index}` }))));
add('seconds-and-subminute-ticks', base([['a', '2026-10-01T10:00:00Z'], ['b', '2026-10-01T10:00:07.250Z'], ['c', '2026-10-01T10:00:31Z'], ['d', '2026-10-01T10:01:02Z']]
  .map(([id, at]) => ({ id, at, title: `Probe ${id}`, detail: 'Observed by the poller' }))));
add('multi-day', base([['a', '2026-09-28T19:13:16+08:00'], ['b', '2026-09-29T14:16:30+08:00'], ['c', '2026-09-30T09:00:00+08:00'], ['d', '2026-10-02T22:45:00+08:00']]
  .map(([id, at]) => ({ id, at, title: `Activity ${id}` })), { timezone: 'Asia/Shanghai', evidence: 'illustrative' }));
add('single-instant', base([{ id: 'only', at: '2026-10-01T10:00:00Z', title: 'The only event', kind: 'alert' }], { quality_profile: 'standard' }));
add('simultaneous-burst', { ...base(Array.from({ length: 9 }, (_, index) => ({ id: `b${index}`, at: `2026-10-01T10:0${index < 6 ? 0 : 1}:00Z`,
  title: index % 2 ? 'Simultaneous signal with a longer title that wraps' : `Signal ${index}`, lane: index % 3 ? 'ops' : 'app', kind: index % 2 ? 'action' : 'default' }))),
  lanes: [{ id: 'app', label: 'Application tier' }, { id: 'ops', label: 'Operations' }] });
add('bc-years', base([['a', '0000-01-01T00:00Z'], ['b', '0000-01-01T06:00Z'], ['c', '0000-01-01T12:00:00+05:30']].map(([id, at]) => ({ id, at, title: `Ancient ${id}` }))));
add('year-boundary-negative-offset', base([['a', '0001-01-01T03:00+05:00'], ['b', '0001-01-01T09:00Z']].map(([id, at]) => ({ id, at, title: `Edge ${id}` })), { timezone: '-12' }));
add('locale-zh-cn', { ...structuredClone(incident), meta: { ...incident.meta, locale: 'zh-CN', timezone: 'Asia/Shanghai' } });
add('locale-es-breaks', { ...base([0, 1, 2, 600, 601].map((minute, index) => ({ id: `e${index}`, at: new Date(Date.UTC(2026, 6, 1, 8) + minute * 60_000).toISOString().replace('.000Z', 'Z'), title: `Paso ${index}`, kind: index ? 'change' : 'default' }))), meta: { title: 'Cronología', output: 'timeline.html', evidence: 'illustrative', quality_profile: 'showcase', locale: 'es' } });
// Date-only records: the frozen renderer has no precision field, so it runs
// on the same instants; Phoenix labels them as dates.
const dayEvents = [['tag', '2026-09-28T12:00+08:00', 'Release tag created on main'], ['merge', '2026-09-29T15:42:10+08:00', 'Merge of the evidence pull request'],
  ['note', '2026-10-01T12:00:00+08:00', 'Changelog entry recorded for 3.1']];
add('day-precision', base(dayEvents.map(([id, at, title]) => ({ id, at, title })), { timezone: 'Asia/Shanghai' }),
  { precision: { tag: 'day', note: 'day', merge: 'time' } });
add('invalid-timezone-offset', base([{ id: 'a', at: '2026-10-01T10:00Z', title: 'A' }], { timezone: '+24:00' }));
add('invalid-timezone-prefixed', base([{ id: 'a', at: '2026-10-01T10:00Z', title: 'A' }], { timezone: 'UTC+05:30' }));
add('invalid-timezone-name', base([{ id: 'a', at: '2026-10-01T10:00Z', title: 'A' }], { timezone: 'Mars/Phobos' }));
add('invalid-calendar-date', base([{ id: 'a', at: '2026-02-30T10:00Z', title: 'A' }]));
add('invalid-hour-24-seconds', base([{ id: 'a', at: '2026-10-01T24:00:01Z', title: 'A' }]));
add('invalid-unknown-lane', { ...base([{ id: 'a', at: '2026-10-01T10:00Z', title: 'A', lane: 'nowhere' }]), lanes: [{ id: 'x', label: 'X' }] });
add('invalid-duplicate-lane', { ...base([{ id: 'a', at: '2026-10-01T10:00Z', title: 'A', lane: 'x' }]), lanes: [{ id: 'x', label: 'X' }, { id: 'x', label: 'X again' }] });
add('invalid-duplicate-event', base([{ id: 'a', at: '2026-10-01T10:00Z', title: 'A' }, { id: 'a', at: '2026-10-01T11:00Z', title: 'A again' }]));

const temporary = await mkdtemp(path.join(os.tmpdir(), 'phoenix-timeline-oracle-'));
const cases = [];
try {
  for (const { name, document, phoenix } of documents) {
    const input = path.join(temporary, `${name}.json`), output = path.join(temporary, `${name}.html`);
    await writeFile(input, JSON.stringify(document));
    const env = { ...process.env, ARCHIFY_QUALITY_PROFILE: document.meta.quality_profile, ARCHIFY_DIAGNOSTIC_FORMAT: 'json' };
    const run = args => spawnSync(process.execPath, [path.join(root, source), input, output, ...args], { cwd: root, encoding: 'utf8', env });
    const rendered = run([]);
    assert.equal(rendered.signal, null, `${name}: source terminated`);
    const entry = { name, document, ok: rendered.status === 0 };
    if (phoenix) entry.phoenix = phoenix;
    if (entry.ok) {
      const layout = run(['--layout-json']);
      assert.equal(layout.status, 0, `${name}: layout report failed`);
      entry.layout = JSON.parse(layout.stdout);
      entry.svg = (await readFile(output, 'utf8')).match(/<svg\b[^>]*viewBox="0 0 [^"]+"[\s\S]*?<\/svg>/)?.[0];
      assert(entry.svg, `${name}: no main SVG`);
    } else entry.diagnostics = JSON.parse(rendered.stdout.trim() || rendered.stderr.trim()).diagnostics.map(d => ({ code: d.code, path: d.subject?.path ?? null }));
    cases.push(entry);
  }
} finally { await rm(temporary, { recursive: true, force: true }); }
const oracle = { format: 1, source, sourceSha256: digest(await readFile(path.join(root, source))), cases };
const file = new URL('./oracle.json', import.meta.url);
const bytes = `${JSON.stringify(oracle, null, 1)}\n`;
if (process.argv.includes('--check')) assert.equal(await readFile(file, 'utf8'), bytes, 'oracle.json');
else await writeFile(file, bytes);
console.log(`${process.argv.includes('--check') ? 'Verified' : 'Wrote'} ${cases.length} timeline cases; ${cases.filter(c => c.ok).length} rendered.`);
