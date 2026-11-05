import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { verifySources } from '../../../../scripts/check-phoenix-diagram-source.mjs';

const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const nativeManifest = 'tests/fixtures/text-segmentation/sources.json';
const nativeSource = 'tests/fixtures/text-segmentation/icu77-dictbe.cpp';
const nativeLicense = 'assets/icu77-LICENSE.txt';
const timeManifest = 'tests/fixtures/format/system-v-sources.json';
const timeSource = 'tests/fixtures/format/icu77-zoneinfo64.txt';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');

// A disposable project holding exactly the files the source gate reads.
function fixture(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram-source-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  for (const name of ['tests/fixtures/text-segmentation', nativeLicense, timeManifest, timeSource, 'Cargo.lock',
    'assets/license-sources.json', 'assets/licenses', 'assets/brand-marks.json', 'assets/brand-license-sources.json']) {
    fs.mkdirSync(path.dirname(path.join(directory, name)), { recursive: true });
    fs.cpSync(path.join(project, name), path.join(directory, name), { recursive: true });
  }
  return directory;
}

function snapshot(directory) {
  const rows = [];
  const visit = relative => {
    const file = path.join(directory, relative), stat = fs.lstatSync(file);
    if (stat.isDirectory()) {
      for (const name of fs.readdirSync(file).sort()) visit(path.join(relative, name));
    } else rows.push([relative, stat.mode, stat.size, stat.mtimeMs, digest(fs.readFileSync(file))]);
  };
  visit('');
  return rows;
}

test('the pinned native text, time, Rust license and brand sources verify', () => {
  const result = verifySources();
  assert.equal(result.nativeText.fileCount, 12);
  assert.equal(result.nativeText.manifestSha256, '28c4de15cce787d92c638a9d54bdbc12c329e30caba55e3935f1b33515f14aff');
  assert.equal(result.nativeText.licenseSpdx, 'Unicode-3.0');
  assert.equal(result.nativeText.licenseSha256, '451167c55c0fa447cc2d5632714f5e3c567fe4f1e1badefab2c1333852198aca');
  assert.equal(result.nativeTime.fileCount, 1);
  assert.equal(result.nativeTime.zoneCount, 6);
  assert.equal(result.nativeTime.tzVersion, '2025a');
  assert.equal(result.nativeTime.manifestSha256, '5cca4def77aea4452b0877c10caee53d08ab0414e7a42706edffb9a6b6da0067');
  assert.equal(result.nativeTime.sourceSha256, 'c0c34daadb59d2552252405b9be08894f83d2cd8145d0f6eb35e1557721dd55f');
  assert.equal(result.nativeTime.licenseSha256, result.nativeText.licenseSha256);
  assert.equal(result.nativeBrands.catalog.count, 107);
});

test('a native source manifest change fails without refreshing its pin', t => {
  for (const [member, message] of [[nativeManifest, /Native text manifest differs/], [timeManifest, /Native time manifest differs/]]) {
    const directory = fixture(t);
    fs.appendFileSync(path.join(directory, member), '\n');
    const before = snapshot(directory);
    assert.throws(() => verifySources({ project: directory }), message);
    assert.deepEqual(snapshot(directory), before, 'verification must not refresh the source identity or manifest');
  }
});

test('missing native source, license or manifest bytes fail without recovery writes', t => {
  for (const [member, message] of [[nativeSource, /Missing native text source/], [nativeLicense, /Missing native text license/],
    [nativeManifest, /Missing native text manifest/], [timeSource, /Missing native time source/], [timeManifest, /Missing native time manifest/]]) {
    const directory = fixture(t), missing = path.join(directory, member);
    fs.unlinkSync(missing);
    const before = snapshot(directory);
    assert.throws(() => verifySources({ project: directory }), message);
    assert.equal(fs.existsSync(missing), false);
    assert.deepEqual(snapshot(directory), before, 'verification must not restore missing bytes');
  }
});

test('native source or copied license changes fail without automatic repair', t => {
  for (const [member, message] of [[nativeSource, /Native text source SHA-256 differs/], [nativeLicense, /Native text license differs/],
    [timeSource, /Native time source size\/SHA-256 differs/]]) {
    const directory = fixture(t);
    fs.appendFileSync(path.join(directory, member), '\n');
    const before = snapshot(directory);
    assert.throws(() => verifySources({ project: directory }), message);
    assert.deepEqual(snapshot(directory), before, 'verification must preserve the mismatched bytes for diagnosis');
  }
});
