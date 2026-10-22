// Maintenance oracle: run with `mise exec --no-deps -- node`, never system Node.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyFrozenSources } from '../../../../../scripts/check-phoenix-diagram-source.mjs';

const source = {
  node: process.versions.node,
  icu: process.versions.icu,
  unicode: process.versions.unicode,
  locale: new Intl.Segmenter().resolvedOptions().locale,
};
assert.deepEqual(source, { node: '25.2.1', icu: '77.1', unicode: '16.0', locale: 'en-US' });
const root = fileURLToPath(new URL('../../../../../', import.meta.url));
verifyFrozenSources({ localReference: path.join(root, 'local/phoenix-diagram/reference') });
const nativeSources = JSON.parse(await readFile(new URL('./sources.json', import.meta.url), 'utf8'));
for (const entry of nativeSources.files) {
  const digest = createHash('sha256').update(await readFile(new URL(entry.file, import.meta.url))).digest('hex');
  assert.equal(digest, entry.sha256, `${entry.file}: original ICU source differs`);
}
const properties = { han: /^\p{Script=Han}$/u, mark: /^\p{Mark}$/u, jsWhitespace: /^\s$/u };
const ranges = Object.fromEntries(Object.keys(properties).map(name => [name, []]));
const fingerprints = Object.fromEntries(Object.keys(properties).map(name => [name, { count: 0, digest: createHash('sha256') }]));
const yes = Buffer.from([1]), no = Buffer.from([0]);
for (let codepoint = 0; codepoint <= 0x10ffff; codepoint += 1) {
  if (codepoint >= 0xd800 && codepoint <= 0xdfff) continue;
  const character = String.fromCodePoint(codepoint);
  for (const [name, pattern] of Object.entries(properties)) {
    const member = pattern.test(character);
    fingerprints[name].digest.update(member ? yes : no);
    if (!member) continue;
    fingerprints[name].count += 1;
    const previous = ranges[name].at(-1);
    if (previous && previous[1] === codepoint - 1) previous[1] = codepoint;
    else ranges[name].push([codepoint, codepoint]);
  }
}
for (const fingerprint of Object.values(fingerprints)) {
  fingerprint.sha256 = fingerprint.digest.digest('hex');
  delete fingerprint.digest;
}
await writeFile(new URL('../../../assets/unicode16-properties.json', import.meta.url), `${JSON.stringify({ format: 1, ...source, ...ranges })}\n`);

const inputs = [
  ['empty', ''],
  ['ascii', 'Request accepted; processing completes or fails.'],
  ['han-note', '已持有：处理完成或失败'],
  ['han-punctuation', '状态：等待、处理中、已完成。'],
  ['han-mark-runs', '已\u0301持有\ufe00：处理\u20dd完成或失败'],
  ['han-dictionary', '中华人民共和国中国人民银行处理结果'],
  ['han-ambiguous', '研究生命起源南京市长江大桥北京大学生前来应聘'],
  ['han-identifiers', '已持有 lease_id +4；处理完成或失败。'],
  ['han-word-spacing', '订单已发货，预计明天下午送达。'],
  ['mixed-label', '用户 ID +4 / 已持有 lease_token：处理完成或失败'],
  ['stage-header', '03 / Process e\u0301tat 👩🏽‍💻 数据接入'],
  ['japanese', '東京都のデータ処理が完了しました。こんにちは世界'],
  ['katakana', 'カタカナ・ひらがな・漢字・ＡＢＣ１２３'],
  ['korean', '한글 한글 데이터 처리 완료'],
  ['thai-dictionary', 'ภาษาไทยทุกสองสัปดาห์ทดสอบการทำงาน'],
  ['lao-dictionary', 'ພາສາລາວທົດສອບການເຮັດວຽກ'],
  ['khmer-dictionary', 'ភាសាខ្មែរសាកល្បងការងារ'],
  ['myanmar-dictionary', 'မြန်မာဘာသာစကားစမ်းသပ်မှု'],
  ['url', 'https://api.example.test/v1/jobs?lease_id=ab_cd-4&retry=1#done'],
  ['path', '/var/log/worker_v2/retry-attempt-03.json'],
  ['email-and-domain', 'mail+retry@example.test docs.example.test'],
  ['numbers', '12,345.67 1.0.2 2026-10-16 +4 -9 ４２ ١٢٣٫٤٥'],
  ['apostrophes', "can't l’état d’Art rock'n'roll foo_bar A.B.C."],
  ['combining', 'e\u0301 a\u0308\u0301 \u0301\u0300 z\u0327\u0308'],
  ['spacing-enclosing-marks', 'का ᬵ a\u20dd a\u093e'],
  ['variation-selectors', '♥︎ ♥️ 漢\u{e0100} ☕\ufe0e ☕\ufe0f'],
  ['emoji-zwj', '👩🏽‍💻👨‍👩‍👧‍👦🏳️‍🌈🏴‍☠️👩‍❤️‍💋‍👨'],
  ['emoji-flags', '🇨🇳🇺🇸🇯🇵🇦🇧🇨 🇦'],
  ['emoji-tags', '🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}'],
  ['emoji-keycaps', '1️⃣2⃣#️⃣*️⃣'],
  ['emoji-unicode15-16', '\u{1fae8}\u{1fae9}\u{1fa89}\u{1fa8f}\u{1f6dc}'],
  ['unicode15-cjk', '\u{31350}\u{323af}\u{2ebf0}\u{2ee5d}'],
  ['unicode15-scripts', '\u{11f02}\u{11f04}\u{11f36}\u{11f42}\u{11f04} \u{1e4d0}\u{1e4ec}'],
  ['unicode16-scripts', '\u{11380}\u{113b8}\u{113d0}\u{11380} \u{16100}\u{1611e}\u{1612f} \u{16d40}\u{16d63}\u{1e5d0}\u{1e5ee}'],
  ['indic-conjuncts', 'क्षि क्‍षि क्‌षि ত্র ক্ষ മലയാളം'],
  ['arabic-hebrew', 'العربية لا مُرَحَّبًا שָׁלוֹם אב׳גד'],
  ['cr-lf-control', 'A\r\nB\rC\nD\u0000E\u000bF\u000cG'],
  ['js-whitespace', 'A\tB\u0020C\u00a0D\u1680E\u2000F\u200aG\u2028H\u2029I\u202fJ\u205fK\u3000L\ufeffM'],
  ['non-js-whitespace', 'A\u0085B\u180eC\u200bD\u2060E'],
  ['format-controls', 'A\u200cB\u200dC\u202aD\u202cE\u2066F\u2069G'],
];
const stems = ['已持有', '处理完成或失败', '数据接入', 'hello', 'e\u0301tat', 'lease_id', '+4', '12,345.67', '👩🏽‍💻', '🇨🇳', '\u{11380}\u{113b8}', '\u{31350}'];
const separators = ['', ' ', '\u00a0', '\ufeff', '\u200b', '\u0301', '\n', '：', '，', '/', '-'];
for (const [index, left] of stems.entries()) {
  const right = stems[(index + 1) % stems.length];
  for (const [separatorIndex, separator] of separators.entries()) {
    inputs.push([`mixed-${index}-${separatorIndex}`, `${left}${separator}${right}`]);
  }
}
const wordClasses = ['a', 'א', 'カ', '漢', 'あ', '가', '1', '_', ':', ',', '.', "'", '"', '\r', '\n', '\u0085', '\u0301', '\ufeff', '\u200d', ' ', '🇦', '👩', '\u{1e5ee}', '\u{31350}'];
for (const [leftIndex, left] of wordClasses.entries()) {
  for (const [rightIndex, right] of wordClasses.entries()) {
    for (const [separatorIndex, separator] of ['', '\u0301', '\ufeff'].entries()) {
      inputs.push([`word-class-${leftIndex}-${rightIndex}-${separatorIndex}`, `${left}${separator}${right}`]);
    }
  }
}
for (const [name, text] of [
  ['normalization-halfwidth', 'ｶﾀｶﾅﾊﾟﾋﾟﾌﾟﾍﾟﾎﾟｶﾞｷﾞｸﾞｹﾞｺﾞ'],
  ['normalization-compatibility', '神神練識兀嗀\u{1f200}\u{3300}'],
  ['normalization-marks', 'カ\u3099タ\u309aナ か\u3099は\u309a'],
  ['katakana-underscore', '_カタカナ_カナ_'],
  ['dictionary-format-boundaries', '中国\ufeff人民\u0301银行 研究\ufe0f生命起源'],
  ['regional-indicator-format', '🇦\u0301🇧\ufeff🇨🇩\u0301🇪'],
]) inputs.push([name, text]);
const dictionary = (await readFile(new URL('./icu77-cjdict.txt', import.meta.url), 'utf8'))
  .split(/\r?\n/).filter(line => line && !line.trimStart().startsWith('#'))
  .map(line => line.slice(0, line.lastIndexOf('\t'))).filter(Boolean);
const holdout = dictionary.filter((_, index) => index % 1250 === 0);
for (const [index, word] of holdout.entries()) {
  inputs.push([`dictionary-holdout-${index}`, `${word}${holdout[(index + 1) % holdout.length]}`]);
}
const examples = path.join(root, 'local/phoenix-diagram/reference/archify/examples');
for (const name of (await readdir(examples)).sort()) {
  if (!/\.(sequence|dataflow)\.json$/.test(name)) continue;
  const document = JSON.parse(await readFile(path.join(examples, name), 'utf8'));
  for (const collection of ['messages', 'stages', 'flows']) {
    for (const [index, item] of (document[collection] || []).entries()) {
      for (const field of ['note', 'label', 'classification']) {
        if (typeof item[field] === 'string') inputs.push([`${name}/${collection}/${index}/${field}`, item[field]]);
      }
    }
  }
}

const grapheme = new Intl.Segmenter('en-US', { granularity: 'grapheme' });
const word = new Intl.Segmenter('en-US', { granularity: 'word' });
const cases = inputs.map(([name, text]) => ({
  name,
  text,
  graphemes: [...grapheme.segment(text)].map(({ segment }) => segment),
  // Intl gives UTF-16 positions. Rust consumes UTF-8 byte offsets.
  wordBoundaries: [...word.segment(text)].map(({ index }) => Buffer.byteLength(text.slice(0, index))).concat(Buffer.byteLength(text)),
}));
await writeFile(new URL('./oracle.json', import.meta.url), `${JSON.stringify({ format: 1, source, nativeSources, properties: fingerprints, cases }, null, 2)}\n`);
console.log(`Wrote ${cases.length} text segmentation cases and ${Object.values(ranges).reduce((sum, entries) => sum + entries.length, 0)} Unicode property ranges.`);
