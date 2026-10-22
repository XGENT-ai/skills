import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { verifyFrozenSources } from '../../../scripts/check-phoenix-diagram-source.mjs';

const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const assets = path.join(project, 'assets');
const manifestPath = path.join(assets, 'license-sources.json');
const manifestSha256 = 'b8a8e29bf8b295f85426c4ffe2e1b25620b2a105865cc0e946db645ef4d8b48e';
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const pins = [
  ['jsonschema-regex', '0.58.6', 'a968fffd387b2ab005bf00c1851f5d26e90df05b6f7ad609ed0d620033cca5e7', '55ac1664384793c5d3b20745d3f9c169dfe2c06f'],
  ['jsonschema-value', '0.58.6', '39b7670345e5ce87371339d13705dbb9276b7e3e934bbe1efa2c6734a1f65a96', '55ac1664384793c5d3b20745d3f9c169dfe2c06f'],
  ['r-efi', '5.3.0', '69cdb34c158ceb288df11e18b4bd39de994f6657d83847bdffdbd7f346754b0f', '97b55bed1c2c91dcbf787674849f05337ff80b33'],
  ['selectors', '0.38.0', '8adfa1c298912827b8a28b223b3b874357397ae706e6190acd9bf28cee99114d', '572ecba2d1600e7c3d490586692a209faf703baa'],
  ['uuid-simd', '0.8.0', '23b082222b4f6619906941c17eb2297fff4c2fb96cb60164170522942a200bd8', 'd74c030d9dc4f3cae02146d1f497ff62726ef09a'],
  ['vsimd', '0.8.0', '5c3082ca00d5a5ef149bb8b555a72ae84c9c59f7250f013ac822ac2e49b19c64', 'd74c030d9dc4f3cae02146d1f497ff62726ef09a'],
];

test('six missing crate license sources retain exact lock and upstream revision pins', () => {
  const bytes = fs.readFileSync(manifestPath);
  assert.equal(bytes.length, 13949);
  assert.equal(sha256(bytes), manifestSha256);
  const manifest = JSON.parse(bytes);
  assert.equal(manifest.schemaVersion, 1);
  assert.deepEqual(manifest.rust.map(({ name, version, checksum, revision }) => [name, version, checksum, revision]), pins);
  const lock = fs.readFileSync(path.join(project, 'Cargo.lock'), 'utf8');
  for (const [name, version, checksum] of pins) {
    const block = lock.split('[[package]]').find(block => block.includes(`name = "${name}"\n`) && block.includes(`version = "${version}"\n`));
    assert.ok(block, `${name}@${version}`);
    assert.ok(block.includes(`checksum = "${checksum}"`));
  }
});

test('every prepared license is a regular complete pinned file with an immutable source', () => {
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const files = new Map();
  for (const entry of manifest.rust) {
    assert.ok(entry.files.length > 0, entry.name);
    assert.ok(entry.license && entry.repository && entry.selectedLicense, entry.name);
    for (const source of entry.files) {
      assert.match(source.path, /^licenses\/[a-zA-Z0-9_.-]+\.txt$/);
      const file = path.join(assets, source.path);
      const stat = fs.lstatSync(file);
      assert.ok(stat.isFile() && !stat.isSymbolicLink(), source.path);
      const bytes = fs.readFileSync(file);
      assert.equal(bytes.length, source.size, source.path);
      assert.equal(sha256(bytes), source.sha256, source.path);
      assert.ok(bytes.length > 1000, `${source.path} cannot be a short declaration`);
      assert.match(source.url, /^https:\/\/raw\.githubusercontent\.com\/[^/]+\/[^/]+\/[a-f0-9]{40}\//);
      assert.ok(source.sourcePath && source.url.endsWith(`/${source.sourcePath}`));
      files.set(source.path, bytes);
    }
  }
  assert.equal(files.size, 4, 'same repository and commit licenses are reused without duplicate copies');
  const rEfi = manifest.rust.find(entry => entry.name === 'r-efi');
  assert.equal(rEfi.license, 'MIT OR Apache-2.0 OR LGPL-2.1-or-later');
  assert.equal(rEfi.selectedLicense, 'MIT');
  assert.equal(rEfi.files[0].sourcePath, 'AUTHORS');
  const selectors = manifest.rust.find(entry => entry.name === 'selectors');
  assert.equal(selectors.files[0].repository, 'https://github.com/mozilla/bedrock');
  assert.notEqual(selectors.files[0].revision, selectors.revision);
  const notice = selectors.applicability.sourceNotice;
  assert.equal(notice.repository, selectors.repository);
  assert.equal(notice.revision, selectors.revision);
  assert.equal(notice.sourcePath, 'selectors/lib.rs');
  assert.equal(Buffer.byteLength(notice.header), notice.headerSize);
  assert.equal(sha256(Buffer.from(notice.header)), notice.headerSha256);
  assert.ok(notice.header.includes(notice.licenseReference));
});

function sourceFixture(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-license-source-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const temporaryUpstream = path.join(directory, 'upstream');
  fs.mkdirSync(temporaryUpstream);
  for (const name of ['SOURCE.json', 'archify-3.1.0-source.tar.gz', 'archify-3.1.0-manifest.json',
    'legacy-2.17-manifest.json', 'legacy-2.17', 'LICENSE-archify-MIT.txt', 'JetBrainsMono-OFL.txt']) {
    fs.cpSync(path.join(project, 'upstream', name), path.join(temporaryUpstream, name), { recursive: true });
  }
  for (const name of ['Cargo.lock', 'tests/fixtures/text-segmentation', 'assets/icu77-LICENSE.txt',
    'assets/license-sources.json', 'assets/licenses']) {
    fs.mkdirSync(path.dirname(path.join(directory, name)), { recursive: true });
    fs.cpSync(path.join(project, name), path.join(directory, name), { recursive: true });
  }
  return directory;
}

function snapshot(directory) {
  const rows = [];
  const visit = relative => {
    const file = path.join(directory, relative), stat = fs.lstatSync(file);
    if (stat.isDirectory() && !stat.isSymbolicLink()) {
      rows.push([relative, 'directory', stat.mode, stat.mtimeMs]);
      for (const name of fs.readdirSync(file).sort()) visit(path.join(relative, name));
    } else if (stat.isSymbolicLink()) rows.push([relative, 'symlink', fs.readlinkSync(file)]);
    else rows.push([relative, 'file', stat.mode, stat.size, stat.mtimeMs, sha256(fs.readFileSync(file))]);
  };
  visit('');
  return rows;
}

test('the production frozen-source gate includes the exact prepared license set', () => {
  const result = verifyFrozenSources({ upstream: path.join(project, 'upstream'), localReference: null });
  assert.equal(result.rustLicenses?.manifestSha256, manifestSha256);
  assert.equal(result.rustLicenses?.crateCount, 6);
  assert.equal(result.rustLicenses?.fileCount, 4);
});

test('production source verification refuses missing, changed or redirected license inputs without repair', t => {
  const cases = ['missing-manifest', 'missing-identity', 'changed-manifest', 'missing-text', 'changed-text',
    'self-rehashed', 'self-refreshed-source', 'changed-lock', 'symlink-file', 'symlink-directory'];
  for (const kind of cases) {
    const directory = sourceFixture(t);
    const manifest = path.join(directory, 'assets/license-sources.json');
    const license = path.join(directory, 'assets/licenses/jsonschema-0.58.6-MIT.txt');
    switch (kind) {
      case 'missing-manifest': fs.unlinkSync(manifest); break;
      case 'missing-identity': {
        const identityPath = path.join(directory, 'upstream/SOURCE.json');
        const identity = JSON.parse(fs.readFileSync(identityPath, 'utf8'));
        delete identity.rustLicenses;
        fs.writeFileSync(identityPath, JSON.stringify(identity, null, 2) + '\n');
        break;
      }
      case 'changed-manifest': fs.appendFileSync(manifest, '\n'); break;
      case 'missing-text': fs.unlinkSync(license); break;
      case 'changed-text': fs.appendFileSync(license, '\n'); break;
      case 'self-rehashed': {
        fs.appendFileSync(license, '\n');
        const value = JSON.parse(fs.readFileSync(manifest, 'utf8'));
        for (const entry of value.rust) {
          for (const file of entry.files.filter(file => file.path === 'licenses/jsonschema-0.58.6-MIT.txt')) {
            file.sha256 = sha256(fs.readFileSync(license)); file.size = fs.statSync(license).size;
          }
        }
        fs.writeFileSync(manifest, JSON.stringify(value, null, 2) + '\n');
        break;
      }
      case 'self-refreshed-source': {
        fs.appendFileSync(manifest, '\n');
        const identityPath = path.join(directory, 'upstream/SOURCE.json');
        const identity = JSON.parse(fs.readFileSync(identityPath, 'utf8'));
        identity.rustLicenses.manifestSize = fs.statSync(manifest).size;
        identity.rustLicenses.manifestSha256 = sha256(fs.readFileSync(manifest));
        fs.writeFileSync(identityPath, JSON.stringify(identity, null, 2) + '\n');
        break;
      }
      case 'changed-lock': {
        const lock = path.join(directory, 'Cargo.lock');
        fs.writeFileSync(lock, fs.readFileSync(lock, 'utf8').replace(pins[0][2], '0'.repeat(64)));
        break;
      }
      case 'symlink-file': {
        const target = path.join(directory, 'original-license');
        fs.renameSync(license, target); fs.symlinkSync(target, license, 'file');
        break;
      }
      case 'symlink-directory': {
        const target = path.join(directory, 'original-licenses');
        const folder = path.join(directory, 'assets/licenses');
        fs.renameSync(folder, target); fs.symlinkSync(target, folder, process.platform === 'win32' ? 'junction' : 'dir');
        break;
      }
    }
    const before = snapshot(directory);
    assert.throws(() => verifyFrozenSources({ upstream: path.join(directory, 'upstream'), localReference: null }),
      /Rust license|rust license|license directory/, kind);
    assert.deepEqual(snapshot(directory), before, `${kind}: checking cannot fetch, repair, or refresh pins`);
  }
});
