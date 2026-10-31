import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { verifyFrozenSources } from '../../../scripts/check-phoenix-diagram-source.mjs';

const upstream = path.dirname(fileURLToPath(import.meta.url));
const nativeManifest = 'tests/fixtures/text-segmentation/sources.json';
const nativeSource = 'tests/fixtures/text-segmentation/icu77-dictbe.cpp';
const nativeLicense = 'assets/icu77-LICENSE.txt';
const timeManifest = 'tests/fixtures/format/system-v-sources.json';
const timeSource = 'tests/fixtures/format/icu77-zoneinfo64.txt';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');

function fixture(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram-source-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const temporaryUpstream = path.join(directory, 'upstream');
  fs.mkdirSync(temporaryUpstream);
  for (const name of ['SOURCE.json', 'archify-3.1.0-source.tar.gz', 'archify-3.1.0-manifest.json',
    'legacy-2.17-manifest.json', 'legacy-2.17', 'LICENSE-archify-MIT.txt', 'JetBrainsMono-OFL.txt']) {
    fs.cpSync(path.join(upstream, name), path.join(temporaryUpstream, name), { recursive: true });
  }
  for (const name of ['tests/fixtures/text-segmentation', nativeLicense, timeManifest, timeSource,
    'Cargo.lock', 'assets/license-sources.json', 'assets/licenses']) {
    fs.mkdirSync(path.dirname(path.join(directory, name)), { recursive: true });
    fs.cpSync(path.join(upstream, '..', name), path.join(directory, name), { recursive: true });
  }
  return temporaryUpstream;
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

test('the frozen archive and legacy tree verify without the original checkout', () => {
  const result = verifyFrozenSources({ upstream, localReference: null });
  assert.equal(result.reference.contentSha256, '4f892b0c68fef5d21788264ba6a43fe9e1d377d3c2d5b558f6aa346784238395');
  assert.equal(result.reference.fileCount, 498);
  assert.equal(result.legacy.fileCount, 70);
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
});

test('a missing archive stops verification instead of using the local checkout', t => {
  const directory = fixture(t);
  fs.unlinkSync(path.join(directory, 'archify-3.1.0-source.tar.gz'));
  assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }), /Missing frozen reference archive/);
});

test('changed compressed bytes stop verification before reading archive members', t => {
  const directory = fixture(t), archive = path.join(directory, 'archify-3.1.0-source.tar.gz');
  const bytes = fs.readFileSync(archive);
  bytes[bytes.length - 8] ^= 1;
  fs.writeFileSync(archive, bytes);
  assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }), /Frozen reference archive size\/SHA-256 differs/);
});

test('a changed legacy renderer cannot pass with the untouched source manifest', t => {
  const directory = fixture(t);
  fs.appendFileSync(path.join(directory, 'legacy-2.17/renderers/sequence/render-sequence.mjs'), '\n');
  assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }), /Frozen legacy tree differs/);
});

test('a changed canonical manifest cannot select a different baseline', t => {
  const directory = fixture(t), manifest = path.join(directory, 'archify-3.1.0-manifest.json');
  fs.appendFileSync(manifest, '\n');
  assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }), /Reference manifest is not the pinned canonical JSON/);
});

test('an available local snapshot must match the frozen bytes', t => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-diagram-local-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  fs.writeFileSync(path.join(directory, 'LICENSE'), 'changed');
  assert.throws(() => verifyFrozenSources({ upstream, localReference: directory }), /Local reference differs from frozen source/);
});

test('native source manifest bytes cannot refresh their own pinned identity', t => {
  for (const updateIdentity of [false, true]) {
    const directory = fixture(t), project = path.dirname(directory);
    const manifest = path.join(project, nativeManifest);
    fs.appendFileSync(manifest, '\n');
    if (updateIdentity) {
      const identityPath = path.join(directory, 'SOURCE.json');
      const identity = JSON.parse(fs.readFileSync(identityPath, 'utf8'));
      identity.nativeText = { ...identity.nativeText, manifestSha256: digest(fs.readFileSync(manifest)) };
      fs.writeFileSync(identityPath, JSON.stringify(identity));
    }
    const before = snapshot(project);
    assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }),
      updateIdentity ? /Frozen native text identity differs/ : /Native text manifest differs/);
    assert.deepEqual(snapshot(project), before, 'verification must not refresh the source identity or manifest');
  }
});

test('missing native source and license bytes fail without recovery writes', t => {
  for (const [member, message] of [[nativeSource, /Missing native text source/], [nativeLicense, /Missing native text license/],
    [nativeManifest, /Missing native text manifest/]]) {
    const directory = fixture(t), project = path.dirname(directory);
    const missing = path.join(project, member);
    fs.unlinkSync(missing);
    const before = snapshot(project);
    assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }), message);
    assert.equal(fs.existsSync(missing), false);
    assert.deepEqual(snapshot(project), before, 'verification must not restore missing upstream bytes');
  }
});

test('native source or copied license changes fail without automatic repair', t => {
  for (const [member, message] of [[nativeSource, /Native text source SHA-256 differs/], [nativeLicense, /Native text license differs/]]) {
    const directory = fixture(t), project = path.dirname(directory);
    fs.appendFileSync(path.join(project, member), '\n');
    const before = snapshot(project);
    assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }), message);
    assert.deepEqual(snapshot(project), before, 'verification must preserve the mismatched bytes for diagnosis');
  }
});

test('native time manifest bytes cannot refresh their own pinned identity', t => {
  for (const updateIdentity of [false, true]) {
    const directory = fixture(t), project = path.dirname(directory);
    const manifest = path.join(project, timeManifest);
    fs.appendFileSync(manifest, '\n');
    if (updateIdentity) {
      const identityPath = path.join(directory, 'SOURCE.json');
      const identity = JSON.parse(fs.readFileSync(identityPath, 'utf8'));
      identity.nativeTime = { ...identity.nativeTime, manifestSha256: digest(fs.readFileSync(manifest)) };
      fs.writeFileSync(identityPath, JSON.stringify(identity));
    }
    const before = snapshot(project);
    assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }),
      updateIdentity ? /Frozen native time identity differs/ : /Native time manifest differs/);
    assert.deepEqual(snapshot(project), before, 'verification must not refresh native time identities or bytes');
  }
});

test('missing native time source or manifest fails without recovery writes', t => {
  for (const [member, message] of [[timeSource, /Missing native time source/], [timeManifest, /Missing native time manifest/]]) {
    const directory = fixture(t), project = path.dirname(directory), missing = path.join(project, member);
    fs.unlinkSync(missing);
    const before = snapshot(project);
    assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }), message);
    assert.equal(fs.existsSync(missing), false);
    assert.deepEqual(snapshot(project), before, 'verification must not restore missing native time bytes');
  }
});

test('native time source changes fail without automatic repair', t => {
  const directory = fixture(t), project = path.dirname(directory);
  fs.appendFileSync(path.join(project, timeSource), '\n');
  const before = snapshot(project);
  assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }), /Native time source size\/SHA-256 differs/);
  assert.deepEqual(snapshot(project), before, 'verification must preserve the mismatched native time source');
});

test('native time source symlinks cannot substitute the fixed ordinary file', t => {
  const directory = fixture(t), project = path.dirname(directory), source = path.join(project, timeSource);
  const replacement = path.join(project, 'zoneinfo-copy.txt');
  fs.renameSync(source, replacement);
  fs.symlinkSync(replacement, source);
  const before = snapshot(project);
  assert.throws(() => verifyFrozenSources({ upstream: directory, localReference: null }), /Expected regular native time source/);
  assert.deepEqual(snapshot(project), before);
});
