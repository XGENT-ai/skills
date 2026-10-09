import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export const workspace = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../tools/phoenix-ui');
export const versions = JSON.parse(fs.readFileSync(path.join(workspace, 'build-tools.lock.json')));

// Restrict IP traffic while preserving local Unix sockets used by mbx and Chrome.
export const offlineNetworkProfile = '(version 1) (allow default) (deny network-outbound (remote ip)) (deny network-inbound (local ip))';
export const testNetworkProfile = `${offlineNetworkProfile} (allow network-outbound (remote ip "localhost:*")) (allow network-inbound (local ip "localhost:*"))`;

export function miseEnvironment(overrides = {}) {
  const env = { ...process.env, ...overrides, MISE_AUTO_INSTALL: 'false', MISE_EXEC_AUTO_INSTALL: 'false' };
  // Keep the PATH diff so nested exec can distinguish selected tools from the
  // original system PATH; discard only transient shim/activation state.
  for (const key of Object.keys(env)) if (key.startsWith('__MISE') && key !== '__MISE_DIFF') delete env[key];
  // npm preserves activated PATH entries ahead of mise's wrapper. Let the Rust
  // backend reinsert rustup's proxy directory in the selected tool order.
  const cargoBins = [path.join(os.homedir(), '.cargo/bin'), process.env.CARGO_HOME && path.join(process.env.CARGO_HOME, 'bin')].filter(Boolean);
  env.PATH = env.PATH.split(path.delimiter).filter(p => !cargoBins.includes(p)).join(path.delimiter);
  return env;
}

export function mise(args, options = {}) {
  const env = miseEnvironment(options.env);
  const result = spawnSync('mise', ['exec', '--no-deps', '--', ...args], {
    cwd: workspace, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, ...options, env,
  });
  if (result.error || result.status !== 0) {
    throw new Error(result.error?.message || result.stderr || `${args.join(' ')} exited ${result.status}`);
  }
  return result.stdout;
}

function exact(command, args, expected) {
  const result = spawnSync(command, args, { encoding: 'utf8' });
  if (result.error || result.status !== 0 || !result.stdout.includes(expected)) {
    throw new Error(`Required ${path.basename(command)} ${expected}; prepare the pinned build tools before building.`);
  }
  return command;
}

export function preflight() {
  const selected = JSON.parse(mise(['mise', 'ls', '--current', '--json']));
  for (const [tool, version] of [['rust', versions.rust], ['mr-boxington', versions.mrBoxington],
    ['node', versions.node], ['bun', versions.bun]]) {
    if (!selected[tool]?.some(t => t.version === version && t.installed && t.active)) {
      throw new Error(`Missing installed mise tool ${tool}@${version}; explicitly run mise install in tools/phoenix-ui.`);
    }
  }
  for (const [command, expected] of [['rustc', versions.rust], ['cargo', versions.rust],
    ['mbx', versions.mrBoxington], ['node', versions.node], ['bun', versions.bun]]) {
    const output = mise([command, '--version']);
    if (!new RegExp(`(?:^|\\s|v)${expected.replaceAll('.', '\\.')}\\b`).test(output)) {
      throw new Error(`${command}: expected ${expected}, got ${output.trim()}. Run mise install in tools/phoenix-ui.`);
    }
  }
  const installed = mise(['rustup', 'target', 'list', '--installed']).trim().split(/\r?\n/);
  const host = /^host: (.+)$/m.exec(mise(['rustc', '-vV']))[1];
  const target = process.env.PHOENIX_UI_BUILD_TARGET || host;
  if (![target, versions.wasmTarget].every(t => installed.includes(t))) {
    throw new Error(`Missing native/WASM target; explicitly prepare rustup target add ${target} ${versions.wasmTarget}.`);
  }
  const components = mise(['rustup', 'component', 'list', '--installed']);
  if (!['rustfmt', 'clippy'].every(c => components.includes(`${c}-${host}`))) {
    throw new Error('Missing rustfmt/clippy; run mise install before building.');
  }
  const doctor = mise(['mbx', 'doctor']);
  if (!doctor.includes('managed targets enabled') || !doctor.includes('0 failures')) {
    throw new Error('mbx managed targets must be enabled and doctor must have no failures.');
  }
  const toolsEnv = JSON.parse(mise(['node', '-e', `
    const fs = require('node:fs'), path = require('node:path');
    const find = name => process.env.PATH.split(path.delimiter)
      .map(p => path.join(p, name + (process.platform === 'win32' ? '.exe' : ''))).find(p => fs.existsSync(p));
    const cargo = find('cargo');
    console.log(JSON.stringify({path: process.env.PATH, cargo, realCargo: fs.realpathSync(cargo)}));
  `]));
  // mbx 1.21.1's doctor does not recognize mise's universal command wrapper.
  if (!toolsEnv.cargo.includes(`${path.sep}command-wrappers${path.sep}`)
      || !/^mise(?:\.exe)?$/.test(path.basename(toolsEnv.realCargo))) {
    throw new Error('Cargo must resolve to the mise mr_boxington wrapper.');
  }
  const lock = fs.readFileSync(path.join(workspace, 'Cargo.lock'), 'utf8');
  if (!lock.includes(`name = "wasm-bindgen"\nversion = "${versions.wasmBindgen}"`)) {
    throw new Error('wasm-bindgen build tool differs from Cargo.lock.');
  }
  const wasmCache = path.join(os.homedir(), process.platform === 'darwin' ? 'Library/Caches' : '.cache', '.wasm-pack');
  const suffix = process.platform === 'win32' ? '.exe' : '';
  const bindgen = process.env.WASM_BINDGEN || path.join(wasmCache, `wasm-bindgen-cargo-install-${versions.wasmBindgen}`, `wasm-bindgen${suffix}`);
  const optCandidates = process.env.WASM_OPT ? [process.env.WASM_OPT] : [
    path.resolve(workspace, '../../local/phoenix-ui/build-tools', `binaryen-version_${versions.binaryen}`, 'bin', `wasm-opt${suffix}`),
    ...(fs.existsSync(wasmCache) ? fs.readdirSync(wasmCache).filter(n => n.startsWith('wasm-opt-')).map(n => path.join(wasmCache, n, 'bin', `wasm-opt${suffix}`)) : []),
  ];
  const opt = optCandidates.find(p => {
    const r = spawnSync(p, ['--version'], { encoding: 'utf8' });
    return r.status === 0 && new RegExp(`version ${versions.binaryen}\\b`).test(r.stdout);
  });
  if (!opt) throw new Error(`Missing wasm-opt ${versions.binaryen}; explicitly prepare it and set WASM_OPT.`);
  exact(bindgen, ['--version'], `wasm-bindgen ${versions.wasmBindgen}`);
  exact(opt, ['--version'], `version ${versions.binaryen}`);
  return { host, target, bindgen, opt, doctor, cargo: toolsEnv.cargo, path: toolsEnv.path,
  };
}

export function buildEnvironment(prepared) {
  if (process.env.RUSTFLAGS || process.env.CARGO_ENCODED_RUSTFLAGS) {
    throw new Error('Clear ambient Rust flags; Phoenix builds use the recorded build options.');
  }
  const env = {
    ...process.env,
    MISE_AUTO_INSTALL: 'false',
    MISE_EXEC_AUTO_INSTALL: 'false',
    PATH: [path.dirname(prepared.bindgen), path.dirname(prepared.opt), prepared.path].join(path.delimiter),
    CARGO_NET_OFFLINE: 'true',
    CARGO_ENCODED_RUSTFLAGS: [
      `--remap-path-prefix=${os.homedir()}=/build/home`,
      `--remap-path-prefix=${workspace}=/build/phoenix-ui`,
    ].join('\u001f'),
    PHOENIX_UI_CARGO_WRAPPER: prepared.cargo,
    PHOENIX_UI_WASM_BINDGEN: prepared.bindgen,
    PHOENIX_UI_WASM_OPT: prepared.opt,
  };
  return miseEnvironment(env);
}
