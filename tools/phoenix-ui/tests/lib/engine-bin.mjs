/**
 * Locate the phoenix-ui engine binary for tests that drive verbs end to end.
 *
 * Order: $PHOENIX_UI_BIN, then target/debug and target/release builds of this
 * workspace. Returns null when none exists so a suite can skip cleanly.
 */
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SKILL_DIR = path.resolve(REPO_ROOT, '../../skills/phoenix-ui/src');

export function findEngineBinary() {
  const fromEnv = process.env.PHOENIX_UI_BIN;
  if (fromEnv && fs.existsSync(fromEnv)) return path.resolve(fromEnv);
  const exe = os.platform() === 'win32' ? 'phoenix-ui.exe' : 'phoenix-ui';
  const candidates = ['debug', 'release'].map((profile) => path.join(REPO_ROOT, 'target', profile, exe));
  return candidates.find((p) => fs.existsSync(p)) || null;
}

export const ENGINE_MISSING_MESSAGE =
  'engine binary not found: run `cargo build -p phoenix-ui`, or set PHOENIX_UI_BIN';

/** Environment the launcher would export for the binary when run from this repo's skill dir. */
export function engineEnv(bin, extra = {}) {
  return {
    ...process.env,
    PHOENIX_UI_SKILL_DIR: SKILL_DIR,
    PHOENIX_UI_SELF: bin,
    ...extra,
  };
}
