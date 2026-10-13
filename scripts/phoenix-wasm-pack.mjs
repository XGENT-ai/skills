#!/usr/bin/env node
// Offline adapter for the unchanged upstream bundler during baseline capture.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const { PHOENIX_UI_WASM_PACK: wasmPack, PHOENIX_UI_CARGO_WRAPPER: cargo } = process.env;
if (!wasmPack || !cargo) throw new Error('Run the pinned Phoenix build preflight first.');
const args = process.argv.slice(2);
const separator = args.indexOf('--');
const packArgs = separator < 0 ? args : args.slice(0, separator);
const cargoArgs = separator < 0 ? [] : args.slice(separator + 1);
const invocation = [...packArgs, '--mode', 'no-install', '--', ...cargoArgs, '--locked', '--offline'];
if (process.env.PHOENIX_UI_BUILD_TRACE) {
  fs.appendFileSync(process.env.PHOENIX_UI_BUILD_TRACE, JSON.stringify({
    command: 'wasm-pack', args: invocation, cargo, target: process.env.CARGO_TARGET_DIR || null,
  }) + '\n');
}
const result = spawnSync(wasmPack, invocation, {
  stdio: 'inherit', env: { ...process.env, PATH: [path.dirname(cargo), process.env.PATH].join(path.delimiter) },
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
