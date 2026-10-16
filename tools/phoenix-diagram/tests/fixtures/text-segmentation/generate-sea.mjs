// Independent Southeast Asian word-engine oracle, bounded by ICU source samples.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';

const source = { node: process.versions.node, icu: process.versions.icu, unicode: process.versions.unicode,
  locale: new Intl.Segmenter().resolvedOptions().locale };
assert.deepEqual(source, { node: '25.2.1', icu: '77.1', unicode: '16.0', locale: 'en-US' });
const nativeSources = JSON.parse(await readFile(new URL('./sources.json', import.meta.url), 'utf8'));
const inputs = [
  ['thai-lookahead', 'แพนด้าแดง สัตว์เลี้ยงลูกด้วยนมชนิดหนึ่ง มีชื่อวิทยาศาสตร์ว่า'],
  ['thai-mixed', 'aภาษาไทยภาษาไทยb'],
  ['thai-suffix', 'ภาษาไทยฯ ภาษาไทยๆ ภาษาไทยๆๆ ภาษาไทยฯๆ'],
  ['thai-unknown', 'กขฃคฅฆงจฉชซฌญฎฏฐฑฒณ'],
  ['thai-marks', 'ภาษา\u0301ไทย ภาษา\ufeffไทย ภาษ\u0e31ไทย'],
  ['thai-short', 'กขคง กขคงจ กขฯ กขๆ กๆๆ'],
  ['lao', 'ພາສາລາວທົດສອບການເຮັດວຽກ'],
  ['lao-mixed', 'aພາສາລາວb'],
  ['lao-unknown', 'ກຂຄຆງຈຉຊຌຍຎຏຐຑຒຓ'],
  ['lao-marks', 'ພາສາ\u0301ລາວ ພາສາ\ufeffລາວ'],
  ['burmese', 'မြန်မာစာမြန်မာစာမြန်မာစာ'],
  ['burmese-mixed', 'aမြန်မာစာမြန်မာစာb'],
  ['burmese-unknown', 'ကခဂဃငစဆဇဈဉညဋဌဍဎဏ'],
  ['burmese-marks', 'မြန်မာ\u0301စာ မြန်မာ\ufeffစာ'],
  ['burmese-unicode16', '\u{116d0}\u{116d1}မြန်မာ\u{116e3}စာ'],
  ['khmer', 'ភាសាខ្មែរសាកល្បងការងារ'],
  ['khmer-mixed', 'aភាសាខ្មែរភាសាខ្មែរb'],
  ['khmer-unknown', 'កខគឃងចឆជឈញដឋឌឍណ'],
  ['khmer-coeng', 'ក្សក្ស ក្ខខ្មែរ ខ្មែរក្រោម'],
  ['khmer-marks', 'ភាសា\u0301ខ្មែរ ភាសា\ufeffខ្មែរ'],
  ['mixed-dictionaries', 'ภาษาไทยພາສາລາວမြန်မာစာភាសាខ្មែរ'],
];
for (const name of ['thaidict', 'laodict', 'burmesedict', 'khmerdict']) {
  const filename = `icu77-${name}.txt`;
  const entry = nativeSources.files.find(entry => entry.file === filename);
  assert.ok(entry, `${filename}: missing frozen source identity`);
  const bytes = await readFile(new URL(filename, import.meta.url));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), entry.sha256, `${filename}: source differs`);
  const words = bytes.toString('utf8').split(/\r?\n/)
    .filter(line => line.trim() && !line.trimStart().startsWith('#'));
  assert.ok(words.length > 1000);
  const holdout = Array.from({ length: 100 }, (_, index) => words[Math.floor(index * words.length / 100)]);
  for (const [index, word] of holdout.entries()) {
    const next = holdout[(index + 1) % holdout.length];
    inputs.push([`${name}-holdout-${index}`, `${word}${next}`]);
    inputs.push([`${name}-mixed-${index}`, `a${word}${next}b`]);
    inputs.push([`${name}-mark-${index}`, `${word}\u0301${next}`]);
  }
}
const segmenter = new Intl.Segmenter('en-US', { granularity: 'word' });
const cases = inputs.map(([name, text]) => ({ name, text,
  wordBoundaries: [...segmenter.segment(text)].map(({ index }) => Buffer.byteLength(text.slice(0, index))).concat(Buffer.byteLength(text)),
}));
await writeFile(new URL('./sea-oracle.json', import.meta.url), `${JSON.stringify({ format: 1, source, cases }, null, 2)}\n`);
console.log(`Wrote ${cases.length} independent Southeast Asian word oracle cases.`);
