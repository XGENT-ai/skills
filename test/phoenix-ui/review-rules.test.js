'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');
const root = path.resolve(__dirname, '../..');

test('review rules match all four contract byte hashes and the supported rule version', () => {
  const rules = JSON.parse(fs.readFileSync(path.join(root, 'skills/review-ui/references/rules.v1.json'), 'utf8'));
  assert.equal(rules.schemaVersion, 1);
  assert.equal(rules.ruleVersion, '1.0.0');
  assert.deepEqual(Object.keys(rules.contractHashes).sort(), [
    'skills/apply-ui-review/references/record-contract.md',
    'skills/review-ui/references/judge.md',
    'skills/review-ui/references/report-contract.md',
    'skills/review-ui/references/target-and-capture.md',
  ]);
  for (const [file, expected] of Object.entries(rules.contractHashes)) {
    assert.equal(createHash('sha256').update(fs.readFileSync(path.join(root, file))).digest('hex'), expected, file);
  }
  assert.deepEqual(rules.parameters, {
    dimensionMaxima: ['3', '3', '3', '1'], defaultThreshold: '8', confirmationMargin: '0.5',
    plateauWindow: 3, minimumBestGrowth: '1.0', maxRefactorsPerScope: 1,
  });
  assert.deepEqual(rules.verdictOrder, ['需补证', '通过验收', '需用户裁定', '需要修改']);
});
