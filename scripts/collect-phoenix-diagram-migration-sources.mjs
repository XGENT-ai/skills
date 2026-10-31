#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { gzipSync, gunzipSync } from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const workspace = path.join(root, 'tools/phoenix-diagram');
const sourceCommit = '4a65a1525f9ed03c775a0e18935901e1edbb773f';
const legacyArchive = 'upstream/archify-2.17-full-source.tar.gz';
const referenceArchive = 'upstream/archify-3.1.0-source.tar.gz';
const referenceSha256 = '6831ba7f7d9d5c0fd473e317bd9b513f2581b9d4d63bd157d1de4680af3b32a4';
const manifestPath = path.join(workspace, 'migration-sources/archify-v1.json');
const fixtureMember = 'tools/phoenix-diagram/tests/fixtures/migration/legacy-sources.v1.json.gz';
const whenToUse = 'when_to_use: \'Requests like "draw the request flow", "turn this Mermaid into a diagram", or "export it as PNG or video".\'';
const pins = {
  'legacy-2.17': { count: 219, sha256: 'd00a98aff8e461c4dd446054ea1cede3bec5b3239863fb44db3c0173b61cb7f1' },
  'legacy-2.17-when-to-use': { count: 219, sha256: '2943184c17a5d8f9f5b383e7460eb3f8057f8d0b3e1ffde11fbfe7caf04e6d55' },
  'reference-3.1.0': { count: 157, sha256: 'da4787260b911254547cf0651c46b2e48917bd7105a3b20739ce68b8fed5d66f' },
};
const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const canonical = value => Buffer.from(JSON.stringify(value) + '\n');
const compare = (left, right) => left < right ? -1 : left > right ? 1 : 0;

function safePath(name) {
  if (!name || /[\\:\0-\x1f\x7f]/.test(name) || name.split('/').some(part => !part || part === '.' || part === '..')) {
    throw new Error(`Unsafe migration source path: ${name}`);
  }
  return name;
}

function inventory(members) {
  return [...members].sort(([left], [right]) => compare(left, right)).map(([name, entry]) => ({
    path: safePath(name), bytes: entry.bytes.length, mode: entry.mode, sha256: sha256(entry.bytes),
  }));
}

function verifyInventory(id, members) {
  const rows = inventory(members), pin = pins[id];
  if (rows.length !== pin.count || sha256(canonical(rows)) !== pin.sha256) throw new Error(`Fixed migration source differs: ${id}`);
  return rows;
}

function parseTar(bytes) {
  if (bytes.length < 10 || bytes.readUInt32LE(4) !== 0) throw new Error('Frozen migration gzip mtime differs');
  const tar = gunzipSync(bytes, { maxOutputLength: 128 * 1024 * 1024 }), members = new Map();
  const field = (header, offset, length) => header.subarray(offset, offset + length).toString().replace(/\0.*$/s, '');
  const octal = text => {
    if (!/^[0-7]+$/.test(text.trim())) throw new Error('Invalid migration tar number');
    return Number.parseInt(text.trim(), 8);
  };
  let ended = false;
  for (let offset = 0; offset + 512 <= tar.length;) {
    const header = tar.subarray(offset, offset + 512);
    if (header.every(byte => byte === 0)) {
      if (offset + 1024 > tar.length || tar.subarray(offset).some(byte => byte !== 0)) throw new Error('Invalid migration tar footer');
      ended = true; break;
    }
    const prefix = field(header, 345, 155), name = safePath((prefix ? prefix + '/' : '') + field(header, 0, 100));
    const sum = header.reduce((total, byte, index) => total + (index >= 148 && index < 156 ? 32 : byte), 0);
    const mode = octal(field(header, 100, 8)), size = octal(field(header, 124, 12));
    if (sum !== octal(field(header, 148, 8)) || field(header, 156, 1) !== '0' || field(header, 157, 100)
        || field(header, 257, 6) !== 'ustar' || field(header, 263, 2) !== '00' || ![0o644, 0o755].includes(mode)
        || octal(field(header, 108, 8)) !== 0 || octal(field(header, 116, 8)) !== 0 || octal(field(header, 136, 12)) !== 0
        || field(header, 265, 32) || field(header, 297, 32) || members.has(name)) throw new Error(`Invalid migration tar metadata: ${name}`);
    const next = offset + 512 + Math.ceil(size / 512) * 512;
    if (!Number.isSafeInteger(size) || next > tar.length) throw new Error(`Truncated migration tar member: ${name}`);
    members.set(name, { mode, bytes: tar.subarray(offset + 512, offset + 512 + size) }); offset = next;
  }
  if (!ended || !members.size) throw new Error('Incomplete migration archive');
  for (const name of members.keys()) {
    for (let parent = path.posix.dirname(name); parent !== '.'; parent = path.posix.dirname(parent)) {
      if (members.has(parent)) throw new Error(`Migration archive file/directory conflict: ${name}`);
    }
  }
  return members;
}

function packTar(members) {
  const chunks = [];
  for (const [name, entry] of [...members].sort(([left], [right]) => compare(left, right))) {
    safePath(name);
    let filename = name, prefix = '';
    if (Buffer.byteLength(filename) > 100) {
      const slash = name.lastIndexOf('/'); prefix = name.slice(0, slash); filename = name.slice(slash + 1);
    }
    if (Buffer.byteLength(filename) > 100 || Buffer.byteLength(prefix) > 155) throw new Error(`Migration tar path too long: ${name}`);
    const header = Buffer.alloc(512);
    const text = (value, offset, length) => { if (Buffer.byteLength(value) > length) throw new Error('Migration tar field overflow'); header.write(value, offset, length); };
    const number = (value, offset, length) => text(value.toString(8).padStart(length - 1, '0') + '\0', offset, length);
    text(filename, 0, 100); number(entry.mode, 100, 8); number(0, 108, 8); number(0, 116, 8);
    number(entry.bytes.length, 124, 12); number(0, 136, 12); header.fill(32, 148, 156); header[156] = 48;
    text('ustar\0', 257, 6); text('00', 263, 2); text(prefix, 345, 155);
    text(header.reduce((sum, byte) => sum + byte, 0).toString(8).padStart(6, '0') + '\0 ', 148, 8);
    chunks.push(header, entry.bytes, Buffer.alloc((512 - entry.bytes.length % 512) % 512));
  }
  chunks.push(Buffer.alloc(1024));
  const compressed = gzipSync(Buffer.concat(chunks), { level: 9 });
  compressed[9] = 19; // Preserve the frozen archive's macOS OS byte on every producer host.
  return compressed;
}

function subTree(members, prefix) {
  return new Map([...members].filter(([name]) => name.startsWith(prefix + '/')).map(([name, value]) => [name.slice(prefix.length + 1), value]));
}

function gitMembers() {
  const run = (args, input) => {
    const result = spawnSync('git', args, { cwd: root, input, maxBuffer: 128 * 1024 * 1024 });
    if (result.error || result.status !== 0) throw result.error || new Error(`Fixed Git source unavailable: ${result.stderr}`);
    return result.stdout;
  };
  const entries = run(['ls-tree', '-r', '-z', sourceCommit, '--', 'skills/archify']).toString().split('\0').filter(Boolean).map(line => {
    const match = /^(100644|100755) blob ([a-f0-9]{40})\tskills\/archify\/(.+)$/.exec(line);
    if (!match) throw new Error('Fixed Git source contains a nonregular member');
    return { mode: Number.parseInt(match[1], 8) & 0o777, oid: match[2], name: safePath(match[3]) };
  });
  const batch = run(['cat-file', '--batch'], entries.map(entry => entry.oid).join('\n') + '\n'), members = new Map(); let offset = 0;
  for (const entry of entries) {
    const end = batch.indexOf(10, offset), header = batch.subarray(offset, end).toString();
    const match = /^([a-f0-9]{40}) blob (\d+)$/.exec(header);
    if (end < 0 || !match || match[1] !== entry.oid) throw new Error('Fixed Git blob identity differs');
    const size = Number(match[2]); offset = end + 1;
    if (batch[offset + size] !== 10) throw new Error('Truncated fixed Git blob');
    members.set(entry.name, { mode: entry.mode, bytes: batch.subarray(offset, offset + size) }); offset += size + 1;
  }
  if (offset !== batch.length) throw new Error('Unexpected fixed Git blob output');
  verifyInventory('legacy-2.17', members);
  return members;
}

function knownVariant(original) {
  const variant = new Map(original), skill = original.get('SKILL.md'), text = skill.bytes.toString();
  if (text.includes('\nwhen_to_use:') || !text.includes('\nlicense: MIT\n')) throw new Error('Fixed legacy frontmatter differs');
  variant.set('SKILL.md', { mode: skill.mode, bytes: Buffer.from(text.replace('\nlicense: MIT\n', `\n${whenToUse}\nlicense: MIT\n`)) });
  verifyInventory('legacy-2.17-when-to-use', variant);
  return variant;
}

function verifyCurrentVariant(expected) {
  const active = path.join(root, 'skills/archify');
  try { fs.lstatSync(active); }
  catch (error) { if (error.code === 'ENOENT') return { status: 'absent' }; throw error; }
  const members = new Map(), directories = new Set(['']);
  let normalizedPermissionFiles = 0;
  for (const name of expected.keys()) for (let parent = path.posix.dirname(name); parent !== '.'; parent = path.posix.dirname(parent)) directories.add(parent);
  const visit = (directory, prefix = '') => {
    const stat = fs.lstatSync(directory);
    if (!stat.isDirectory() || stat.isSymbolicLink() || !directories.has(prefix)) throw new Error(`Unexpected current archify directory: ${prefix}`);
    for (const name of fs.readdirSync(directory).sort()) {
      const relative = safePath(prefix ? `${prefix}/${name}` : name), file = path.join(directory, name), before = fs.lstatSync(file);
      if (before.isDirectory() && !before.isSymbolicLink()) visit(file, relative);
      else {
        if (!before.isFile() || before.isSymbolicLink()) throw new Error(`Current archify contains a nonregular file: ${relative}`);
        const bytes = fs.readFileSync(file), after = fs.lstatSync(file);
        for (const key of ['dev', 'ino', 'size', 'mode', 'mtimeMs', 'ctimeMs']) if (before[key] !== after[key]) throw new Error(`Current archify changed during read: ${relative}`);
        const actualMode = before.mode & 0o777, mode = actualMode & 0o111 ? 0o755 : 0o644;
        if (mode !== actualMode) normalizedPermissionFiles += 1;
        members.set(relative, { mode, bytes });
      }
    }
  };
  visit(active);
  if (!canonical(inventory(members)).equals(canonical(inventory(expected)))) throw new Error('Current archify is not the fixed when_to_use-only variant; refusing to broaden migration sources');
  return { status: 'verified-when-to-use-only', permissionComparison: 'Git executable bit; frozen source uses strict 0644/0755', normalizedPermissionFiles };
}

export function collectMigrationSources({ write = false, project = workspace } = {}) {
  project = path.resolve(project);
  if (write && project !== workspace) throw new Error('Write frozen migration sources only in the collector workspace');
  const projectManifest = path.join(project, 'migration-sources/archify-v1.json');
  const referenceBytes = fs.readFileSync(path.join(project, referenceArchive));
  if (sha256(referenceBytes) !== referenceSha256) throw new Error('Fixed 3.1.0 archive identity differs');
  const reference = subTree(parseTar(referenceBytes), 'archify'); verifyInventory('reference-3.1.0', reference);
  let archiveBytes, original, variant;
  if (write) {
    original = gitMembers(); variant = knownVariant(original);
    const packed = new Map();
    for (const [prefix, members] of [['legacy-2.17', original], ['legacy-2.17-when-to-use', variant]]) {
      for (const [name, entry] of members) packed.set(`${prefix}/${name}`, entry);
    }
    archiveBytes = packTar(packed);
  } else {
    archiveBytes = fs.readFileSync(path.join(project, legacyArchive));
    const packed = parseTar(archiveBytes); original = subTree(packed, 'legacy-2.17'); variant = subTree(packed, 'legacy-2.17-when-to-use');
    if (packed.size !== original.size + variant.size || !archiveBytes.equals(packTar(packed))) throw new Error('Legacy migration archive is not the canonical complete source archive');
    if (!canonical(inventory(knownVariant(original))).equals(canonical(inventory(variant)))) throw new Error('Archived when_to_use variant differs');
  }
  const current = project === workspace ? verifyCurrentVariant(variant) : { status: 'not-checked', reason: 'Only the specified frozen source workspace is checked' };
  const sources = [['legacy-2.17', original], ['legacy-2.17-when-to-use', variant], ['reference-3.1.0', reference]].map(([id, members]) => {
    const files = verifyInventory(id, members), isReference = id === 'reference-3.1.0';
    return { id, version: isReference ? '3.1.0' : '2.17', origin: isReference
      ? { kind: 'fixed-local-reference-snapshot', upstreamCommitVerified: false, sourcePath: 'archify' }
      : { kind: id === 'legacy-2.17' ? 'fixed-git-tree' : 'fixed-git-tree-with-frontmatter-patch', repository: 'https://github.com/XGENT-ai/skills.git', commit: sourceCommit, sourcePath: 'skills/archify',
          ...(id === 'legacy-2.17-when-to-use' ? { modification: { path: 'SKILL.md', insertedFrontmatterLine: whenToUse } } : {}) },
    archive: { path: isReference ? referenceArchive : legacyArchive, bytes: (isReference ? referenceBytes : archiveBytes).length,
      sha256: sha256(isReference ? referenceBytes : archiveBytes), root: isReference ? 'archify' : id },
    fileCount: files.length, contentSha256: sha256(canonical(files)), files };
  });
  const manifest = { schemaVersion: 1, kind: 'archify-migration-sources',
    policy: { purpose: 'Exact complete source recognition and reinstallation reference only', establishesOwnership: false,
      targetInstallationBackup: false, legacyRemovalRequiresExplicitConfirmation: true },
    inventoryEncoding: 'SHA256 of UTF-8 JSON.stringify(files) plus LF; files sorted by path; mode is numeric POSIX permission bits',
    sources };
  const bytes = Buffer.from(JSON.stringify(manifest, null, 2) + '\n');
  if (write) {
    fs.mkdirSync(path.dirname(projectManifest), { recursive: true });
    fs.writeFileSync(path.join(project, legacyArchive), archiveBytes); fs.writeFileSync(projectManifest, bytes);
  } else if (!fs.readFileSync(projectManifest).equals(bytes)) throw new Error('Frozen migration manifest differs from complete source bytes');
  return { status: 'pass', mode: write ? 'write' : 'check', current, sourceCount: sources.length,
    sources: sources.map(({ id, fileCount, contentSha256 }) => ({ id, fileCount, contentSha256 })),
    archiveSha256: sha256(archiveBytes), manifestSha256: sha256(bytes) };
}

// Assembly-only, read-only full archive/manifest verification. Import lazily;
// source-less consumers need only the exact inert fixture and its reader.
export function verifyMigrationSourceFiles({ project = workspace } = {}) {
  return collectMigrationSources({ project });
}

// Inert test input only: not shipped in npm, loaded by the engine, or executed.
export function migrationFixtureBytes() {
  collectMigrationSources();
  const manifestBytes = fs.readFileSync(manifestPath), manifest = JSON.parse(manifestBytes);
  const archives = new Map([legacyArchive, referenceArchive].map(name => [name, parseTar(fs.readFileSync(path.join(workspace, name)))]));
  const sources = manifest.sources.map(source => {
    const members = subTree(archives.get(source.archive.path), source.archive.root);
    const rows = verifyInventory(source.id, members);
    if (!canonical(rows).equals(canonical(source.files))) throw new Error(`Fixture manifest differs: ${source.id}`);
    return { id: source.id, files: rows.map(file => ({ ...file, base64: members.get(file.path).bytes.toString('base64') })) };
  });
  const bytes = canonical({ schemaVersion: 1, kind: 'phoenix-diagram-inert-migration-fixture',
    manifest: { path: 'tools/phoenix-diagram/migration-sources/archify-v1.json', bytes: manifestBytes.length, sha256: sha256(manifestBytes), base64: manifestBytes.toString('base64') }, sources });
  const compressed = gzipSync(bytes, { level: 9 });
  compressed[9] = 255; // Canonical OS-neutral gzip header, with fixed zero mtime.
  return compressed;
}

export function collectMigrationFixture({ write = false } = {}) {
  const bytes = migrationFixtureBytes(), file = path.join(root, fixtureMember);
  if (write) { fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, bytes); }
  else if (!fs.readFileSync(file).equals(bytes)) throw new Error('Frozen migration fixture differs from complete source archives');
  return { status: 'pass', mode: write ? 'write-fixture' : 'check-fixture', fixture: { path: fixtureMember, bytes: bytes.length, sha256: sha256(bytes) } };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (args.length > 1 || args.length && !['--check', '--write', '--check-fixture', '--write-fixture'].includes(args[0])) throw new Error('Usage: node scripts/collect-phoenix-diagram-migration-sources.mjs [--check|--write|--check-fixture|--write-fixture]');
  console.log(JSON.stringify(args[0]?.endsWith('-fixture') ? collectMigrationFixture({ write: args[0] === '--write-fixture' }) : collectMigrationSources({ write: args[0] === '--write' })));
}
