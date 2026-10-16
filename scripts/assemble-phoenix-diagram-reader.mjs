#!/usr/bin/env node
// Derived from the frozen Archify 3.1.0 scripts/generate-viewer.mjs (MIT).
// Phoenix changes: independent identity, eight native slots, inline catalogs,
// archive-derived SVG assets, pinned maintenance Node and read-only checking.
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { createHash } from 'node:crypto';
import { gunzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { verifyFrozenSources, loadVersions, workspace as defaultWorkspace } from './check-phoenix-diagram-source.mjs';

export const fragments = [
  ['/* PHOENIX:VIEWER_CSS */', 'viewer.css', 4],
  ['/* PHOENIX:EXPORT */', 'export.js'],
  ['/* PHOENIX:READER_LAYOUT */', 'reader-layout.js'],
  ['/* PHOENIX:CHROME_LAYOUT */', 'viewer-chrome-layout.js'],
  ['/* PHOENIX:CAMERA */', 'viewer-camera.js'],
  ['/* PHOENIX:RADAR */', 'semantic-radar.js'],
  ['/* PHOENIX:MOTION_GOVERNOR */', 'motion-governor.js'],
  ['/* PHOENIX:NODE_FINDER */', 'node-finder.js'],
  ['/* PHOENIX:NODE_OUTLINE */', 'node-outline.js'],
  ['/* PHOENIX:TREE_BRANCHES */', 'tree-branches.js'],
  ['/* PHOENIX:FOCUS */', 'focus.js'],
  ['/* PHOENIX:INTENT_TRACE */', 'intent-trace.js'],
  ['/* PHOENIX:SEMANTIC_LENS */', 'semantic-lens.js'],
  ['/* PHOENIX:ROUTE_PROBE */', 'route-probe.js'],
  ['/* PHOENIX:EXPORT_CLEANUP */', 'export-cleanup.js'],
];
const childMarker = '/* PHOENIX:EXPORT_CLEANUP */';
const catalogMarker = '/* PHOENIX:CATALOG_DATA */';
const slots = ['LOCALE', 'PRESET', 'TITLE', 'HEADING', 'SUBTITLE', 'SVG', 'CARDS', 'METADATA'];
const digest = bytes => createHash('sha256').update(bytes).digest('hex');

// No fallback to the old runtime/storage namespace. Attribution is added after
// this mechanical adaptation so original author and license names stay intact.
export function namespaceSource(source) {
  return source.replace(/\bArchify\b/g, 'PhoenixDiagram')
    .replace(/\barchify(?=[A-Z])/g, 'phoenixDiagram')
    .replaceAll('archify-', 'phoenix-diagram-')
    .replaceAll('ARCHIFY:', 'PHOENIX:');
}

function regularText(file) {
  const stat = fs.lstatSync(file);
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error(`Reader source must be a regular file: ${file}`);
  return fs.readFileSync(file, 'utf8');
}

function verifyReaderLayout(source, workspace, members) {
  const audit = regularText(path.join(workspace, 'viewer/reader-layout-adaptations.json'));
  if (digest(audit) !== '8d1d09f132b8490494d85665ba2c8566dcfdf1d59f2af968f71a31b83f54a2d3') {
    throw new Error('Reader Layout adaptation differs from the pinned legacy-fit hunk');
  }
  const adaptation = JSON.parse(audit), frozen = members.get('viewer/reader-layout.js');
  const normalized = namespaceSource(frozen), original = Buffer.from(normalized);
  if (adaptation.sourcePath !== 'viewer/reader-layout.js' || adaptation.sourceSha256 !== digest(frozen)
      || adaptation.normalizedSourceSha256 !== digest(normalized) || adaptation.hunks.length !== 1) {
    throw new Error('Reader Layout adaptation does not match its frozen source');
  }
  const [hunk] = adaptation.hunks, before = Buffer.from(hunk.sourceText), at = hunk.sourceByteOffset;
  if (hunk.scope !== 'native legacy portrait height fitting' || hunk.sourceText !== '      var WIDE_RATIO = 1.55;\n'
      || !original.subarray(at, at + before.length).equals(before)
      || normalized.slice(0, at).split('\n').length !== hunk.sourceLine) {
    throw new Error('Reader Layout adaptation is outside its legacy-fit classification');
  }
  const adapted = Buffer.concat([original.subarray(0, at), Buffer.from(hunk.replacementText), original.subarray(at + before.length)]).toString();
  if (digest(adapted) !== adaptation.adaptedBodySha256 || source.replace(/^\/\*[\s\S]*?\*\/\n/, '') !== adapted) {
    throw new Error('Reader Layout source differs outside its recorded frozen adaptation');
  }
}

function verifyReaderCss(source, workspace, members) {
  const audit = regularText(path.join(workspace, 'viewer/css-adaptations.json'));
  if (digest(audit) !== '4aaa684ba1fee713cf0e827967e42ce9a3a656771c7f7ae080151a0614800dd7') {
    throw new Error('Reader CSS adaptation differs from the pinned spacing hunks');
  }
  const adaptation = JSON.parse(audit), frozen = members.get('viewer/viewer.css');
  const normalized = namespaceSource(frozen), original = Buffer.from(normalized);
  if (adaptation.sourcePath !== 'viewer/viewer.css' || adaptation.sourceSha256 !== digest(frozen)
      || adaptation.normalizedSourceSha256 !== digest(normalized) || adaptation.hunks.length !== 2) {
    throw new Error('Reader CSS adaptation does not match its frozen source');
  }
  const scopes = new Set(['ordinary narrow navigation clearance', 'native legacy portrait compact chrome']);
  const parts = []; let consumed = 0;
  for (const hunk of adaptation.hunks) {
    const at = hunk.sourceByteOffset, before = Buffer.from(hunk.sourceText);
    if (!scopes.delete(hunk.scope) || at < consumed || !original.subarray(at, at + before.length).equals(before)
        || original.subarray(0, at).toString().split('\n').length !== hunk.sourceLine) {
      throw new Error('Reader CSS adaptation is outside its recorded spacing scopes');
    }
    parts.push(original.subarray(consumed, at), Buffer.from(hunk.replacementText)); consumed = at + before.length;
  }
  parts.push(original.subarray(consumed));
  const adapted = Buffer.concat(parts).toString();
  if (digest(adapted) !== adaptation.adaptedBodySha256 || source.replace(/^\/\*[\s\S]*?\*\/\n/, '') !== adapted) {
    throw new Error('Reader CSS source differs outside its recorded frozen adaptations');
  }
}

function archiveMembers() {
  verifyFrozenSources({ localReference: null });
  const tar = gunzipSync(fs.readFileSync(path.join(defaultWorkspace, 'upstream/archify-3.1.0-source.tar.gz')));
  const result = new Map();
  // The source checker has already validated every header, name, byte and hash.
  for (let at = 0; at + 512 <= tar.length && tar[at] !== 0;) {
    const header = tar.subarray(at, at + 512);
    const field = (start, size) => header.subarray(start, start + size).toString().replace(/\0.*$/s, '');
    const prefix = field(345, 155), name = (prefix ? prefix + '/' : '') + field(0, 100);
    const size = Number.parseInt(field(124, 12), 8);
    result.set(name, tar.subarray(at + 512, at + 512 + size).toString());
    at += 512 + Math.ceil(size / 512) * 512;
  }
  return result;
}

function scriptJson(value) {
  return JSON.stringify(value).replaceAll('<', '\\u003c').replaceAll('>', '\\u003e').replaceAll('&', '\\u0026')
    .replaceAll('\u2028', '\\u2028').replaceAll('\u2029', '\\u2029');
}

function escape(value) {
  return String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&#39;');
}

function catalogsAt(workspace) {
  const directory = path.join(workspace, 'viewer/locales');
  const manifest = JSON.parse(regularText(path.join(directory, 'manifest.json')));
  const result = {};
  for (const { locale, file } of manifest.catalogs) {
    if (!/^[a-zA-Z0-9-]+\.json$/.test(file) || Object.hasOwn(result, locale)) throw new Error('Invalid Reader catalog manifest');
    const catalog = JSON.parse(regularText(path.join(directory, file)));
    result[locale] = Object.fromEntries(Object.entries(catalog).map(([key, value]) => [key,
      typeof value === 'string' ? value.replace(/\bArchify\b/g, 'Phoenix Diagram') : value]));
  }
  if (!result.en || !result['zh-CN']) throw new Error('Reader requires bundled English and Simplified Chinese catalogs');
  return result;
}

function localizeShell(source, english) {
  const token = /\{\{i18n:([a-zA-Z0-9_.]+)\}\}/g;
  const valueFor = key => {
    if (typeof english[key] !== 'string') throw new Error(`Missing Reader English message ${key}`);
    return escape(english[key]);
  };
  return source.split(/(<[^>]*>)/g).map(part => {
    if (part.startsWith('<')) {
      const replaced = part.replace(/([a-zA-Z0-9:-]+)="\{\{i18n:([a-zA-Z0-9_.]+)\}\}"/g, (_, attribute, key) => {
        if (!/^(?:aria-label|title|placeholder|data-preset-badge-[a-z-]+)$/.test(attribute)) throw new Error(`Unsafe Reader translated attribute ${attribute}`);
        return `${attribute}="${valueFor(key)}" data-phoenix-i18n-${attribute}="${key}"`;
      });
      if (replaced.match(token)) throw new Error('Reader i18n token must occupy one whole safe attribute');
      return replaced;
    }
    return part.replace(token, (_, key) => `<span data-phoenix-i18n="${key}">${valueFor(key)}</span>`);
  }).join('');
}

function replaceOnce(source, marker, replacement) {
  const parts = source.split(marker);
  if (parts.length !== 2) throw new Error(`Reader must contain exactly one ${marker} marker`);
  return parts[0] + replacement + parts[1];
}

function definitionsFrom(members) {
  const source = members.get('archify/renderers/shared/utils.mjs');
  const definition = source.match(/export function renderDefinitions\(extra = ''\) \{[\s\S]*?\n\}/)?.[0];
  if (!definition) throw new Error('Frozen SVG definitions function is missing');
  return vm.runInNewContext('(' + definition.replace('export ', '') + ')()', {}, { timeout: 1000 });
}

function nodeCardStyleFrom(members) {
  const source = members.get('archify/renderers/shared/node-card-style.mjs');
  if (!source || /\bimport\s/.test(source)) throw new Error('Unexpected frozen node-card style dependencies');
  return vm.runInNewContext(source.replace(/^export /gm, '') + '\nnodeCardStyle()', {}, { timeout: 1000 });
}

export function assembleReader({ workspace = defaultWorkspace, check = false } = {}) {
  const members = archiveMembers();
  let generated = regularText(path.join(workspace, 'viewer/template.source.html'));
  const catalogs = catalogsAt(workspace);
  generated = replaceOnce(generated, catalogMarker, scriptJson(catalogs));
  const known = new Set(fragments.map(([marker]) => marker));
  for (const marker of generated.match(/\/\* PHOENIX:[A-Z_]+ \*\//g) || []) {
    if (!known.has(marker)) throw new Error(`Unknown Reader fragment marker ${marker}`);
  }
  if (generated.includes(childMarker)) throw new Error('Export cleanup fragment belongs only inside Export');
  generated = localizeShell(generated, catalogs.en);
  for (const [marker, file, indent = 0] of fragments) {
    const source = regularText(path.join(workspace, 'viewer', file));
    if (file === 'reader-layout.js') verifyReaderLayout(source, workspace, members);
    if (file === 'viewer.css') verifyReaderCss(source, workspace, members);
    const nested = source.match(/\/\* PHOENIX:[A-Z_]+ \*\//g) || [];
    const allowed = file === 'export.js' ? [childMarker] : [];
    if (!source.trim() || JSON.stringify(nested) !== JSON.stringify(allowed)) throw new Error(`Reader fragment ${file} has invalid nested markers`);
    const parts = generated.split(marker);
    if (parts.length !== 2) throw new Error(`Reader must contain exactly one ${file} marker`);
    const body = indent ? source.split('\n').map(line => line ? ' '.repeat(indent) + line : line).join('\n') : source;
    const tail = indent && parts[1].startsWith('\n') ? parts[1].slice(1) : parts[1];
    generated = parts[0] + body + tail;
  }
  const actualSlots = [...generated.matchAll(/\{\{PHOENIX:([^}]+)\}\}/g)].map(match => match[1]);
  if (JSON.stringify([...actualSlots].sort()) !== JSON.stringify([...slots].sort())) throw new Error('Reader requires exactly eight unique native template slots');
  if (/\/\* PHOENIX:[A-Z_]+ \*\//.test(generated) || /\{\{i18n:/.test(generated)) throw new Error('Unresolved Reader fragment or locale marker');
  for (const script of generated.matchAll(/<script([^>]*)>([\s\S]*?)<\/script>/g)) {
    if (!/application\/json/.test(script[1])) new vm.Script(script[2]);
  }
  const attribution = '<!-- Derived from Archify 3.1.0 frozen snapshot; Phoenix identity/native-scene adaptation.\n'
    + members.get('LICENSE') + '-->\n';
  const outputs = [
    ['template.html', generated],
    ['svg-definitions.svg', attribution + definitionsFrom(members) + '\n'],
    ['node-card-style.svg', attribution + nodeCardStyleFrom(members) + '\n'],
  ];
  if (check) {
    for (const [name, bytes] of outputs) {
      const file = path.join(workspace, 'assets', name);
      if (!fs.existsSync(file) || regularText(file) !== bytes) throw new Error(`Reader asset ${name} is stale; run mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs`);
    }
  } else {
    fs.mkdirSync(path.join(workspace, 'assets'), { recursive: true });
    for (const [name, bytes] of outputs) {
      const file = path.join(workspace, 'assets', name), temporary = `${file}.${process.pid}.tmp`;
      try { fs.writeFileSync(temporary, bytes, { flag: 'wx', mode: 0o644 }); fs.renameSync(temporary, file); }
      finally { fs.rmSync(temporary, { force: true }); }
    }
  }
  return { sourceFileCount: 16, catalogCount: Object.keys(catalogs).length,
    assets: outputs.map(([name, bytes]) => ({ path: `assets/${name}`, bytes: Buffer.byteLength(bytes), sha256: digest(bytes) })) };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = process.argv.slice(2);
    if (args.length > 1 || (args.length === 1 && args[0] !== '--check')) throw new Error('Usage: node scripts/assemble-phoenix-diagram-reader.mjs [--check]');
    const expected = loadVersions().node;
    if (process.versions.node !== expected) throw new Error(`Reader assembly requires pinned Node ${expected}; use mise exec --no-deps -- node`);
    console.log(JSON.stringify(assembleReader({ check: args[0] === '--check' })));
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
