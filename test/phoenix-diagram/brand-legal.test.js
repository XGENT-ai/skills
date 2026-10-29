'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const root = path.resolve(__dirname, '../..');
const apiReady = import('../../scripts/check-phoenix-diagram-source.mjs');

function fixture(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'phoenix-brand-legal-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  for (const name of ['assets', 'upstream']) fs.mkdirSync(path.join(directory, name));
  const source = path.join(root, 'tools/phoenix-diagram');
  const manifest = JSON.parse(fs.readFileSync(path.join(source, 'assets/brand-license-sources.json')));
  for (const name of ['assets/brand-license-sources.json', 'assets/brand-marks.json',
    ...[...manifest.licenses, ...manifest.notices].map(row => 'assets/' + row.path),
    'upstream/SOURCE.json', 'upstream/archify-3.1.0-source.tar.gz']) {
    const destination = path.join(directory, name);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(path.join(source, name), destination);
  }
  return directory;
}

test('brand source verification binds the original catalog, complete terms and collection scope', async () => {
  const api = await apiReady;
  assert.equal(typeof api.verifyBrandLicenseSources, 'function');
  const checked = api.verifyBrandLicenseSources();
  assert.equal(checked.catalog.count, 107);
  assert.equal(checked.licenseCount, 8);
  assert.equal(checked.noticeCount, 2);
  const { manifest } = api.readBrandLicenseSources();
  const collection = manifest.licenses.find(row => row.scope === 'collection-only');
  assert.equal(collection.license, 'CC0-1.0');
  assert.deepEqual(collection.marks, []);
  assert.ok(manifest.licenses.some(row => row.license === 'CC-BY-NC-SA-4.0' && row.marks.includes('vue')));
});

test('brand source verification rejects altered catalog, full terms and source pins', async t => {
  const api = await apiReady, directory = fixture(t);
  const catalog = path.join(directory, 'assets/brand-marks.json'), original = fs.readFileSync(catalog);
  fs.appendFileSync(catalog, '\n');
  assert.throws(() => api.verifyBrandLicenseSources({ project: directory }), /catalog/i);
  fs.writeFileSync(catalog, original);
  const license = path.join(directory, 'assets/licenses/brand-CC-BY-NC-SA-4.0.txt');
  fs.writeFileSync(license, 'CC-BY-NC-SA-4.0\n');
  assert.throws(() => api.verifyBrandLicenseSources({ project: directory }), /license|resource/i);
  fs.copyFileSync(path.join(root, 'tools/phoenix-diagram/assets/licenses/brand-CC-BY-NC-SA-4.0.txt'), license);
  const sourcePath = path.join(directory, 'upstream/SOURCE.json'), source = JSON.parse(fs.readFileSync(sourcePath));
  source.nativeBrands.catalog.count = 106;
  fs.writeFileSync(sourcePath, JSON.stringify(source));
  assert.throws(() => api.verifyBrandLicenseSources({ project: directory }), /identity|pin/i);
});
