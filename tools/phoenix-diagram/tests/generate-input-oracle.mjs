// Maintenance only: rebuild the checked-in input corpus without installing Node packages.
// Run from any directory; --reference-root may point to the verified 3.1.0 archive extraction.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { verifyFrozenSources } from '../../../scripts/check-phoenix-diagram-source.mjs';

const projectRoot = fileURLToPath(new URL('../../../', import.meta.url));
const fixtureRoot = path.join(projectRoot, 'tools/phoenix-diagram/tests/fixtures/input');
const referenceArg = process.argv.indexOf('--reference-root');
const referenceRoot = referenceArg < 0
  ? path.join(projectRoot, 'local/phoenix-diagram/reference/archify')
  : path.resolve(process.argv[referenceArg + 1]);
const legacyKinds = ['architecture', 'workflow', 'sequence', 'dataflow', 'lifecycle'];
const phoenixKinds = [...legacyKinds, 'erd', 'class', 'tree', 'timeline', 'waterfall'];
const sources = [
  ['legacy-2.17', path.join(projectRoot, 'tools/phoenix-diagram/upstream/legacy-2.17'), legacyKinds],
  ['phoenix-v1', referenceRoot, phoenixKinds],
];
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const encode = key => key.replaceAll('~', '~0').replaceAll('/', '~1');
const decode = key => key.replaceAll('~1', '/').replaceAll('~0', '~');
const clone = value => structuredClone(value);

// The archive and legacy tree are checked before loading either executable oracle.
// An alternate extraction is only a location change, never a new source baseline.
verifyFrozenSources({ localReference: null });
const checkedSources = [];
for (const [contract, sourceRoot, kinds] of sources) {
  const manifestName = contract === 'legacy-2.17' ? 'legacy-2.17-manifest.json' : 'archify-3.1.0-manifest.json';
  const manifestBytes = await readFile(path.join(projectRoot, 'tools/phoenix-diagram/upstream', manifestName));
  const manifest = new Map(JSON.parse(manifestBytes).map(([name, size, hash]) => [name, { size, hash }]));
  const examples = (await readdir(path.join(sourceRoot, 'examples'))).sort().filter(name => kinds.some(kind => name.endsWith(`.${kind}.json`)));
  const oracleFiles = ['renderers/shared/generated-validators.mjs', ...(contract === 'phoenix-v1' ? ['renderers/shared/portable-path.mjs'] : [])];
  const files = new Map();
  for (const relative of [...oracleFiles, ...['common', ...kinds].map(kind => `schemas/${kind}.schema.json`), ...examples.map(name => `examples/${name}`)]) {
    const member = contract === 'legacy-2.17' ? relative : `archify/${relative}`;
    const expected = manifest.get(member);
    assert.ok(expected, `${relative} is not in the pinned ${manifestName}`);
    const bytes = await readFile(path.join(sourceRoot, relative));
    assert.equal(bytes.length, expected.size, `${sourceRoot}/${relative}: pinned size differs`);
    assert.equal(sha256(bytes), expected.hash, `${sourceRoot}/${relative}: pinned SHA-256 differs`);
    files.set(relative, bytes);
  }
  checkedSources.push({ contract, sourceRoot, kinds, manifestName, manifestSha256: sha256(manifestBytes), examples, oracleFiles, files });
}

function patch(document, operation) {
  if (!operation.path) return operation.op === 'remove' ? null : clone(operation.value);
  const segments = operation.path.slice(1).split('/').map(decode);
  const key = segments.pop();
  const parent = segments.reduce((node, segment) => node[segment], document);
  if (operation.op === 'remove') {
    if (Array.isArray(parent)) parent.splice(Number(key), 1);
    else delete parent[key];
  } else parent[key] = clone(operation.value);
  return document;
}

function resolve(schema, owner, schemas) {
  if (!schema.$ref) return { schema, owner };
  const [filename, fragment = ''] = schema.$ref.split('#');
  if (filename) owner = path.basename(filename);
  let resolved = schemas[owner];
  for (const segment of fragment.split('/').slice(1)) resolved = resolved[decode(segment)];
  return resolve(resolved, owner, schemas);
}

function sample(unresolved, owner, schemas, pointer = '') {
  const { schema, owner: resolvedOwner } = resolve(unresolved, owner, schemas);
  if (schema.const !== undefined) return schema.const;
  if (schema.enum) return schema.enum[0];
  if (schema.oneOf || schema.anyOf) return sample({ ...schema, oneOf: undefined, anyOf: undefined, ...(schema.oneOf?.at(-1) || schema.anyOf[0]) }, resolvedOwner, schemas, pointer);
  if (schema.type === 'object') {
    const result = {};
    for (const [key, child] of Object.entries(schema.properties || {})) {
      result[key] = sample(child, resolvedOwner, schemas, `${pointer}/${encode(key)}`);
    }
    if (schema.additionalProperties && typeof schema.additionalProperties === 'object') {
      result['corpus.label'] = sample(schema.additionalProperties, resolvedOwner, schemas, `${pointer}/corpus.label`);
    }
    return result;
  }
  if (schema.type === 'array') {
    if (schema.prefixItems) return schema.prefixItems.map((child, i) => sample(child, resolvedOwner, schemas, `${pointer}/${i}`));
    return Array.from({ length: Math.max(1, schema.minItems || 0) }, (_, i) => sample(schema.items, resolvedOwner, schemas, `${pointer}/${i}`));
  }
  if (schema.type === 'number' || schema.type === 'integer') {
    return schema.minimum ?? (schema.exclusiveMinimum !== undefined ? schema.exclusiveMinimum + 1 : 0);
  }
  if (schema.type === 'boolean') return true;
  if (schema.type === 'string') {
    if (pointer.endsWith('/output')) return 'corpus.html';
    if (schema.pattern === '^[a-fA-F0-9]{40}$') return 'a'.repeat(40);
    if (schema.pattern === '^[a-f0-9]{64}$') return 'a'.repeat(64);
    if (schema.pattern?.startsWith('^https?://')) return 'https://example.test/corpus.svg';
    if (schema.pattern?.startsWith('^\\d{4}-\\d{2}-\\d{2}T')) return '2026-01-01T12:00Z';
    if (schema.pattern === '^[^.]+\\.[^.]+$') return 'a.a';
    if (pointer.endsWith('/locale')) return 'en';
    return 'a'.repeat(Math.max(1, schema.minLength || 0));
  }
  throw new Error(`No fixture sample for ${resolvedOwner}${pointer}: ${JSON.stringify(schema)}`);
}

const oracle = { format: 1, sources: [], examples: [], specimens: [], cases: [] };
const addCase = (specimen, name, operations, validate) => {
  let document = clone(specimen.document);
  for (const operation of operations) document = patch(document, operation);
  const valid = validate(document);
  oracle.cases.push({
    specimen: specimen.id,
    name,
    operations,
    valid,
    errors: valid ? [] : validate.errors.map(({ instancePath, keyword, params }) => ({ path: instancePath, keyword, params })),
  });
};

function walk(unresolved, owner, schemas, pointer, value, visit) {
  const { schema, owner: resolvedOwner } = resolve(unresolved, owner, schemas);
  visit(schema, resolvedOwner, pointer, value);
  const variant = schema.oneOf?.find(child => child.type === (Array.isArray(value) ? 'array' : typeof value));
  if (variant) return walk(variant, resolvedOwner, schemas, pointer, value, visit);
  if (schema.type === 'object') {
    for (const [key, child] of Object.entries(schema.properties || {})) {
      walk(child, resolvedOwner, schemas, `${pointer}/${encode(key)}`, value[key], visit);
    }
    if (typeof schema.additionalProperties === 'object') {
      walk(schema.additionalProperties, resolvedOwner, schemas, `${pointer}/corpus.label`, value['corpus.label'], visit);
    }
  }
  if (schema.type === 'array') {
    if (schema.prefixItems) schema.prefixItems.forEach((child, i) => walk(child, resolvedOwner, schemas, `${pointer}/${i}`, value[i], visit));
    else walk(schema.items, resolvedOwner, schemas, `${pointer}/0`, value[0], visit);
  }
}

await mkdir(fixtureRoot, { recursive: true });
for (const { contract, sourceRoot, kinds, manifestName, manifestSha256, examples, oracleFiles, files } of checkedSources) {
  const validatorFile = path.join(sourceRoot, 'renderers/shared/generated-validators.mjs');
  const validators = await import(pathToFileURL(validatorFile).href);
  const schemas = {};
  const source = {
    contract, manifest: manifestName, manifestSha256,
    validator: path.relative(projectRoot, validatorFile), validatorSha256: sha256(files.get('renderers/shared/generated-validators.mjs')),
    dependencies: Object.fromEntries(oracleFiles.slice(1).map(relative => [relative, sha256(files.get(relative))])),
    schemas: {},
  };
  for (const kind of ['common', ...kinds]) {
    const bytes = files.get(`schemas/${kind}.schema.json`);
    schemas[`${kind}.schema.json`] = JSON.parse(bytes);
    source.schemas[`${kind}.schema.json`] = sha256(bytes);
  }
  oracle.sources.push(source);
  for (const name of examples) {
    const kind = kinds.find(kind => name.endsWith(`.${kind}.json`));
    if (!kind) continue;
    const bytes = files.get(`examples/${name}`);
    assert.equal(validators[kind](JSON.parse(bytes)), true, `${contract}/${name}`);
    const relative = `${contract}/${name}`;
    await mkdir(path.join(fixtureRoot, contract), { recursive: true });
    await writeFile(path.join(fixtureRoot, relative), bytes);
    oracle.examples.push({ contract, kind, file: relative, source: path.relative(projectRoot, path.join(sourceRoot, 'examples', name)), sha256: sha256(bytes) });
  }
  for (const kind of kinds) {
    const owner = `${kind}.schema.json`;
    const specimen = { id: `${contract}/${kind}`, contract, kind, document: sample(schemas[owner], owner, schemas) };
    assert.equal(validators[kind](specimen.document), true, `${specimen.id}: ${JSON.stringify(validators[kind].errors)}`);
    oracle.specimens.push(specimen);
    addCase(specimen, 'all-fields', [], validators[kind]);
    const fields = [];
    walk(schemas[owner], owner, schemas, '', specimen.document, (schema, resolvedOwner, pointer, value) => {
      const set = (name, replacement) => addCase(specimen, `${pointer || '/'}:${name}`, [{ op: 'set', path: pointer, value: replacement }], validators[kind]);
      if (pointer) {
        fields.push(pointer);
        addCase(specimen, `${pointer}:omitted`, [{ op: 'remove', path: pointer }], validators[kind]);
      }
      set('null', null);
      if (schema.const !== undefined) set('wrong-const', typeof schema.const === 'number' ? 999 : 'unsupported');
      if (schema.enum) {
        for (const member of schema.enum) set(`enum-${String(member)}`, member);
        set('unknown-enum', '__unsupported__');
        set('wrong-type', typeof schema.enum[0] === 'number' ? '1' : 42);
      }
      if (schema.type === 'object') {
        set('wrong-type', []);
        set('empty-object', {});
        set('unknown-field', { ...value, __unsupported: true });
        if (schema.propertyNames) set('invalid-property-name', { ...value, 'Invalid Key': 'a' });
        if (schema.maxProperties) {
          for (const offset of [0, 1]) set(`maxProperties-${offset}`, Object.fromEntries(Array.from({ length: schema.maxProperties + offset }, (_, i) => [`key${i}`, 'a'])));
        }
      }
      if (schema.type === 'array') {
        set('wrong-type', {});
        set('empty-array', []);
        if (schema.minItems) {
          set('minItems-at', Array.from({ length: schema.minItems }, (_, i) => clone(value[i % value.length])));
          set('minItems-under', value.slice(0, schema.minItems - 1));
        }
        if (schema.maxItems) {
          set('maxItems-at', Array.from({ length: schema.maxItems }, (_, i) => clone(value[i % value.length])));
          set('maxItems-over', Array.from({ length: schema.maxItems + 1 }, (_, i) => clone(value[i % value.length])));
        }
      }
      if (schema.type === 'number' || schema.type === 'integer') {
        set('wrong-type', '0');
        set('fraction', 0.25);
        for (const keyword of ['minimum', 'maximum', 'exclusiveMinimum', 'exclusiveMaximum']) {
          if (schema[keyword] === undefined) continue;
          for (const offset of [-1, 0, 1]) set(`${keyword}-${offset}`, schema[keyword] + offset);
        }
        set('negative', -1);
      }
      if (schema.type === 'string') {
        set('wrong-type', 42);
        set('empty-string', '');
        set('unicode-content', '汉字🌳 e\u0301\u2028\u2029 </script><svg onload=alert(1)>');
        if (schema.pattern) set('invalid-pattern', '1 invalid\n');
        for (const keyword of ['minLength', 'maxLength']) {
          if (schema[keyword] === undefined) continue;
          for (const offset of [-1, 0, 1]) if (schema[keyword] + offset >= 0) set(`${keyword}-${offset}`, 'a'.repeat(schema[keyword] + offset));
        }
      }
      if (schema.type === 'boolean') set('wrong-type', 'true');
      if (schema.oneOf) {
        for (const [i, alternative] of schema.oneOf.entries()) {
          const replacement = sample(alternative, resolvedOwner, schemas, pointer);
          set(`oneOf-${i}`, replacement);
          if (alternative.type === 'string') {
            for (const [j, replacement] of ['', 'catalog', 'https://example.test/a.svg', 'a\nb', 'a'.repeat(80), 'a'.repeat(81), 'a'.repeat(2048), 'a'.repeat(2049)].entries()) set(`oneOf-${i}-string-${j}`, replacement);
          }
          walk(alternative, resolvedOwner, schemas, pointer, replacement, (child, _owner, childPointer) => {
            if (childPointer === pointer) return;
            const operations = [{ op: 'set', path: pointer, value: replacement }, { op: 'remove', path: childPointer }];
            addCase(specimen, `${childPointer}:oneOf-${i}-omitted`, operations, validators[kind]);
          });
        }
      }
    });
    specimen.fields = [...new Set(fields)];
    // Phoenix's only schema extension beyond the input_contract marker (plan D7):
    // the frozen validator plus an optional precision enum on timeline events.
    if (contract === 'phoenix-v1' && kind === 'timeline') {
      const extended = document => {
        const precision = document?.events?.[0]?.precision;
        const original = clone(document);
        if (original?.events?.[0]) delete original.events[0].precision;
        extended.errors = null;
        const valid = validators.timeline(original) && (precision === undefined || ['day', 'time'].includes(precision));
        if (!valid) extended.errors = validators.timeline.errors || [{ instancePath: '/events/0/precision', keyword: 'enum', params: { allowedValues: ['day', 'time'] } }];
        return valid;
      };
      for (const [name, value] of [['day', 'day'], ['time', 'time'], ['unknown-enum', 'hour'], ['wrong-type', 1], ['null', null]]) {
        addCase(specimen, `d7:/events/0/precision:${name}`, [{ op: 'set', path: '/events/0/precision', value }], extended);
      }
    }
    for (const reference of ['https://example.invalid/external.schema.json', 'file:///etc/passwd', 'common.schema.json#/$defs/id']) {
      addCase(specimen, `input-ref:${reference}`, [{ op: 'set', path: '/$ref', value: reference }], validators[kind]);
    }
    if (contract === 'phoenix-v1') {
      for (const output of ['diagram.html', 'a/b.HTML', '汉字/🌳.html', 'a'.repeat(250) + '.html', 'a'.repeat(251) + '.html', '../diagram.html', './diagram.html', '/diagram.html', 'a//diagram.html', 'a\\diagram.html', 'file:///tmp/diagram.html', 'https://example.test/x.html', 'diagram.svg', 'CON.html', 'con/file.html', 'LPT¹.html', 'folder./x.html', 'folder /x.html', 'short~1.html', 'a\u0000.html', 'a\u007f.html', 'a\u0085.html']) {
        addCase(specimen, `portable-output:${JSON.stringify(output)}`, [{ op: 'set', path: '/meta/output', value: output }], validators[kind]);
      }
    }
  }
}
assert.equal(oracle.examples.filter(example => example.contract === 'legacy-2.17').length, 14);
assert.equal(oracle.examples.filter(example => example.contract === 'phoenix-v1').length, 25);
await writeFile(path.join(fixtureRoot, 'schema-oracle.json'), `${JSON.stringify({ ...oracle, cases: undefined }).slice(0, -1)},\n"cases":[\n${oracle.cases.map(testCase => JSON.stringify(testCase)).join(',\n')}\n]}\n`);
console.log(`Wrote ${oracle.examples.length} frozen examples, ${oracle.specimens.length} all-field specimens, ${oracle.cases.length} oracle cases.`);
