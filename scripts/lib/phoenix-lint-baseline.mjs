import crypto from 'node:crypto';
import path from 'node:path';
import { stripVTControlCharacters } from 'node:util';

function count(map, key) { map[key] = (map[key] || 0) + 1; }

function diagnosticPath(file, workspace) {
  const windows = /^[a-z]:[\\/]|^\\\\|^\/\//i.test(workspace);
  const paths = windows ? path.win32 : path;
  const canonical = value => windows ? value.replace(/^\\\\\?\\UNC\\/i, '\\\\').replace(/^\\\\\?\\/, '') : value;
  const source = canonical(file);
  return (paths.isAbsolute(source) ? paths.relative(canonical(workspace), source) : source).replaceAll('\\', '/');
}

export function clippyDiagnostics(output, workspace) {
  const counts = {}, errors = [];
  for (const line of output.split('\n')) {
    if (!line.trim()) continue;
    let row;
    try { row = JSON.parse(line); } catch { continue; }
    if (row.reason !== 'compiler-message') continue;
    const message = row.message;
    if (message.level === 'error') errors.push(message.message);
    if (!['error', 'warning'].includes(message.level)) continue;
    const files = message.spans.filter(s => s.is_primary).map(s => diagnosticPath(s.file_name, workspace));
    count(counts, JSON.stringify([files, message.code?.code || null, message.message]));
  }
  return { counts, errors };
}

export function formatDiagnostics(output, workspace) {
  const counts = {};
  for (const chunk of stripVTControlCharacters(output).split(/^Diff in /m).slice(1)) {
    const lines = chunk.split(/\r?\n/);
    const file = /^(.*):\d+:$/.exec(lines.shift())?.[1];
    if (!file) throw new Error('Unrecognized rustfmt diagnostic');
    const changed = lines.filter(l => /^[+-]/.test(l)).join('\n');
    if (!changed) continue;
    const digest = crypto.createHash('sha256').update(changed).digest('hex');
    count(counts, JSON.stringify([diagnosticPath(file, workspace), digest]));
  }
  return counts;
}

export function additions(actual, baseline) {
  return Object.entries(actual).filter(([key, count]) => count > (baseline[key] || 0))
    .map(([key, count]) => ({ diagnostic: JSON.parse(key), added: count - (baseline[key] || 0) }));
}
