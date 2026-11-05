#!/usr/bin/env node
// Derived from Archify 3.1.0 scripts/generate-viewer.mjs (MIT); Phoenix Diagram modifications:
// eight native slots, inline catalogs, pinned maintenance Node and read-only checking.
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { loadVersions, workspace as defaultWorkspace } from './check-phoenix-diagram-source.mjs';

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

function regularText(file) {
  const stat = fs.lstatSync(file);
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error(`Reader source must be a regular file: ${file}`);
  return fs.readFileSync(file, 'utf8');
}

function scriptJson(value) {
  return JSON.stringify(value).replaceAll('<', '\\u003c').replaceAll('>', '\\u003e').replaceAll('&', '\\u0026')
    .replaceAll('\u2028', '\\u2028').replaceAll('\u2029', '\\u2029');
}

function escape(value) {
  return String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&#39;');
}

const placeholders = message => [...new Set([...message.matchAll(/\{([a-zA-Z0-9_]+)\}/g)].map(match => match[1]))].sort();

// The locale files are the only catalog source; every locale must match the English
// key set and placeholders.
export function catalogsAt(workspace) {
  const directory = path.join(workspace, 'viewer/locales');
  const manifest = JSON.parse(regularText(path.join(directory, 'manifest.json')));
  const result = {};
  for (const { locale, file } of manifest.catalogs) {
    if (!/^[a-zA-Z0-9-]+\.json$/.test(file) || Object.hasOwn(result, locale)) throw new Error('Invalid Reader catalog manifest');
    result[locale] = JSON.parse(regularText(path.join(directory, file)));
  }
  if (!result.en || !result['zh-CN']) throw new Error('Reader requires bundled English and Simplified Chinese catalogs');
  const keys = Object.keys(result.en).sort();
  for (const [locale, catalog] of Object.entries(result)) {
    if (JSON.stringify(Object.keys(catalog).sort()) !== JSON.stringify(keys)) throw new Error(`Reader catalog ${locale} key set differs from en`);
    for (const key of keys) {
      if (typeof catalog[key] !== 'string' || JSON.stringify(placeholders(catalog[key])) !== JSON.stringify(placeholders(result.en[key]))) {
        throw new Error(`Reader catalog ${locale} ${key} placeholders differ from en`);
      }
      if (/archify/i.test(catalog[key])) throw new Error(`Reader catalog ${locale} ${key} keeps the Archify identity`);
    }
  }
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

export function assembleReader({ workspace = defaultWorkspace, check = false } = {}) {
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
  const outputs = [
    ['template.html', generated],
    ['catalogs.json', JSON.stringify(catalogs, null, 2) + '\n'],
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
