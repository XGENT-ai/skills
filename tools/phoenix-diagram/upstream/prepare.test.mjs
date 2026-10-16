import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { preparationCommands } from '../../../scripts/prepare-phoenix-diagram.mjs';

const versions = { rust: '1.99.0', node: '25.2.1', mrBoxington: '1.21.1' };

test('explicit preparation selects only the pinned native diagram tools', () => {
  const commands = preparationCommands(versions, true);
  assert.deepEqual(commands[0], ['mise', ['install', 'rust@1.99.0', 'node@25.2.1', 'mr-boxington@1.21.1']]);
  assert.doesNotMatch(JSON.stringify(commands), /bun|wasm|binaryen|phoenix-ui|npm (?:install|publish)/i);
});

test('preparation preserves an existing lock and fetches exactly its dependency set', () => {
  const commands = preparationCommands(versions, true);
  assert.equal(commands.some(([, args]) => args.includes('generate-lockfile')), false);
  assert.deepEqual(commands.at(-1), ['mise', ['exec', '--no-deps', '--', 'mbx', 'fetch', '--locked']]);
});

test('only a missing lock enables initial lock generation', () => {
  const commands = preparationCommands(versions, false);
  assert.deepEqual(commands.at(-2), ['mise', ['exec', '--no-deps', '--', 'mbx', 'generate-lockfile']]);
});

test('an explicit lock update enables dependency changes without weakening ordinary preparation', () => {
  const commands = preparationCommands(versions, true, { updateLock: true });
  assert.deepEqual(commands.at(-2), ['mise', ['exec', '--no-deps', '--', 'mbx', 'generate-lockfile']]);
  assert.equal(preparationCommands(versions, true).some(([, args]) => args.includes('generate-lockfile')), false);
});

test('preparation requires its one explicit command before running maintenance', () => {
  const script = fileURLToPath(new URL('../../../scripts/prepare-phoenix-diagram.mjs', import.meta.url));
  for (const args of [[], ['--check'], ['--prepare', '--other']]) {
    const result = spawnSync(process.execPath, [script, ...args], { encoding: 'utf8' });
    assert.equal(result.status, 2);
    assert.match(result.stderr, /^Usage: node scripts\/prepare-phoenix-diagram\.mjs --prepare/);
    assert.equal(result.stdout, '');
  }
});
