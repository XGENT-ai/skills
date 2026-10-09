/**
 * Build-pipeline emitters for the Phoenix UI design hook.
 *
 * Two emission targets exist:
 *
 * 1. Project-local install from an explicit Phoenix bundle:
 *      - Claude Code: `.claude/settings.json`   (${CLAUDE_PROJECT_DIR}-relative)
 *      - Codex:       `.codex/hooks.json`
 *      - Cursor:      `.cursor/hooks.json`
 *      - Grok Build:  `.grok/hooks/phoenix-ui.json`
 *
 * 2. Claude Code plugin package (the marketplace / `/plugin install` path):
 *      - `plugin/hooks/hooks.json`              (${CLAUDE_PLUGIN_ROOT}-relative)
 *        Also consumed by Grok Build via Claude Code plugin compatibility
 *        (`CLAUDE_PLUGIN_ROOT` is aliased to `GROK_PLUGIN_ROOT`).
 *
 * 3. OpenAI plugin package:
 *      - `hooks/hooks.json`                     (${PLUGIN_ROOT}-relative)
 *
 * The plugin variant resolves the hook script relative to the installed plugin
 * root rather than assuming a `.claude/skills/phoenix-ui/` layout, so it stays
 * correct wherever Claude Code unpacks the plugin.
 */

import fs from 'node:fs';

const TOOL_VERSION = fs.readFileSync(new URL('../../../../../skills/phoenix-ui/src/scripts/VERSION', import.meta.url), 'utf8').trim();
const SENTINEL = fs.readFileSync(new URL('../../../crates/hook/src/missing_launcher.cjs', import.meta.url), 'utf8')
  .split(/\r?\n/).map(line => line.trim()).join(' ').trim();
const shQuote = value => `'${value.replaceAll("'", "'\"'\"'")}'`;

export const PHOENIX_UI_HOOK_COMMAND_MARKER = 'skills/phoenix-ui/scripts/phoenix-ui';

const TIMEOUT_SECONDS = 5;
const STATUS_MESSAGE = 'Checking UI changes';
// The Stop deep pass scans every UI file touched in the session with the
// full rule set, so it gets a longer budget than the single-file per-edit
// pass. Wired only for Claude Code and Codex, which both dispatch a native
// `Stop` hook event; Cursor's stop hook is not consistently dispatched and
// GitHub Copilot's stop-style events do not feed context back to the model.
const STOP_TIMEOUT_SECONDS = 30;
const STOP_STATUS_MESSAGE = 'Design deep pass';

// The hook is a verb of the phoenix-ui launcher that ships in the skill's
// scripts dir: `<scripts>/phoenix-ui hook` (per-edit and Stop passes) and
// `<scripts>/phoenix-ui hook-before-edit` (Cursor's preToolUse). The launcher
// uses Node >=18 to verify the fixed cached engine. Normal hooks never download.
export const LAUNCHER_NAME = 'phoenix-ui';
export const LAUNCHER_NAME_WINDOWS = 'phoenix-ui.cmd';

// The embedded guard can remind once even when the entire skill is absent.
// Existing launchers retain their stdin and exit status.
export const guardedLauncher = (launcherPath, verb = 'hook', provider = 'source', quoted = `"${launcherPath}"`) =>
  `if [ -f ${quoted} ]; then ${quoted} ${verb}; else node -e ${shQuote(SENTINEL)} -- ${shQuote(TOOL_VERSION)} ${shQuote(provider)} || :; fi`;

// Node dispatches the .cmd shim through cmd.exe; PowerShell does not parse
// an unquoted cmd if/else expression before the guard gets control.
export const windowsLauncherCommand = (launcherCmdPath, verb = 'hook', provider = 'codex') =>
  `node -e "${SENTINEL}" -- "${TOOL_VERSION}" "${provider}" "${launcherCmdPath.replaceAll('/', '\\')}" ${verb}`;

function stopEntry(command, commandWindows) {
  return {
    hooks: [
      {
        type: 'command',
        command,
        ...(commandWindows ? { commandWindows } : {}),
        timeout: STOP_TIMEOUT_SECONDS,
        statusMessage: STOP_STATUS_MESSAGE,
      },
    ],
  };
}

const launcherIn = (scriptsDir) => `${scriptsDir}/${LAUNCHER_NAME}`;
const launcherCmdIn = (scriptsDir) => `${scriptsDir}/${LAUNCHER_NAME_WINDOWS}`;

const CLAUDE_PROJECT_SCRIPTS = '${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts';
const CLAUDE_PLUGIN_SCRIPTS = '${CLAUDE_PLUGIN_ROOT}/skills/phoenix-ui/scripts';
const CODEX_PLUGIN_SCRIPTS = '${PLUGIN_ROOT}/skills/phoenix-ui/scripts';
// Codex reads project hooks from `.codex/hooks.json`, but the skill payload the
// hook invokes lives under the install's own skills dir: a `.codex`-directory
// install keeps it at `.codex/skills/...`, while a `.agents` (Codex repo-skills)
// install keeps it at `.agents/skills/...`. Derive the path from the install dir
// so each generated manifest points at its own payload rather than a hardcoded
// `.agents`; otherwise the guarded hook silently no-ops on `.codex` installs.
const codexProjectScripts = (skillDir) => `${skillDir}/skills/phoenix-ui/scripts`;
const CURSOR_SCRIPTS = '.cursor/skills/phoenix-ui/scripts';
const GITHUB_PROJECT_SCRIPTS = '$(git rev-parse --show-toplevel)/.github/skills/phoenix-ui/scripts';
// Grok project hooks are relative to the git/workspace root. Claude tool names
// in the matcher (Edit|Write|MultiEdit) alias to Grok's search_replace family.
const GROK_PROJECT_SCRIPTS = '.grok/skills/phoenix-ui/scripts';

// `windows: true` adds the `commandWindows` sibling; only Codex-shaped
// consumers honor it, and an unknown key would fail Codex's strict parser if
// it were the other way round, so it stays opt-in per manifest.
function buildClaudeCompatibleHooks(matcher, scriptsDir, { windows = false, sessionIdentity = false, provider = 'claude' } = {}) {
  const command = guardedLauncher(launcherIn(scriptsDir), 'hook', provider);
  const commandWindows = windows ? windowsLauncherCommand(launcherCmdIn(scriptsDir), 'hook', provider) : undefined;
  return {
    ...(sessionIdentity ? { SessionStart: [{ hooks: [{ type: 'command', command,
      timeout: TIMEOUT_SECONDS, statusMessage: 'Preparing build session' }] }] } : {}),
    PostToolUse: [
      {
        matcher,
        hooks: [
          {
            type: 'command',
            command,
            ...(commandWindows ? { commandWindows } : {}),
            timeout: TIMEOUT_SECONDS,
            statusMessage: STATUS_MESSAGE,
          },
        ],
      },
    ],
    Stop: [stopEntry(command, commandWindows)],
  };
}

export function buildClaudeSettingsManifest() {
  return {
    description: 'Phoenix UI design detector: immediate-tier checks after Edit/Write on UI files, full-rule deep pass on Stop.',
    hooks: buildClaudeCompatibleHooks('Edit|Write', CLAUDE_PROJECT_SCRIPTS, { sessionIdentity: true }),
  };
}

// Plugin-packaged variant of the Claude hook. Claude Code reads the `hooks`
// object from a plugin's `hooks/hooks.json`, and the command resolves relative
// to ${CLAUDE_PLUGIN_ROOT} so it does not depend on the skill being copied into
// `.claude/skills/`. No top-level `description`: Codex also loads bundled plugin
// hooks from `hooks/hooks.json` and its strict parser rejects any field other
// than `hooks`, failing the whole manifest (issue #330).
export function buildClaudePluginHooksManifest() {
  return {
    hooks: buildClaudeCompatibleHooks('Edit|Write', CLAUDE_PLUGIN_SCRIPTS, { sessionIdentity: true }),
  };
}

// OpenAI plugin-packaged variant. Codex exposes ${PLUGIN_ROOT} for resources
// inside the installed plugin, so the public bundle can use the native path
// instead of relying on its Claude compatibility alias.
export function buildCodexPluginHooksManifest() {
  return {
    hooks: buildClaudeCompatibleHooks('Edit|Write|apply_patch', CODEX_PLUGIN_SCRIPTS, { windows: true, provider: 'codex' }),
  };
}

// `skillDir` is the install's own dot-directory (a provider's configDir), so the
// emitted command points at that install's payload. Defaults to `.codex` for the
// Codex provider, whose self-consistent bundle keeps the skill at `.codex/skills`.
export function buildCodexHooksManifest(skillDir = '.codex') {
  return {
    hooks: buildClaudeCompatibleHooks('Edit|Write|apply_patch', codexProjectScripts(skillDir), { windows: true, provider: 'codex' }),
  };
}

export function buildCursorHooksManifest(scriptsDir = CURSOR_SCRIPTS) {
  return {
    version: 1,
    hooks: {
      preToolUse: [
        {
          command: guardedLauncher(launcherIn(scriptsDir), 'hook-before-edit', 'cursor'),
          timeout: TIMEOUT_SECONDS,
        },
      ],
    },
  };
}

// GitHub Copilot reads project hooks from `.github/hooks/*.json`. Its schema
// differs from Claude/Codex/Cursor: the event key is lowercase `postToolUse`,
// each entry is flat (no nested `hooks` array), the command lives under `bash`
// (with an optional `powershell` sibling), the timeout key is `timeoutSec`, and
// `matcher` is a full-match regex (`^(?:PATTERN)$`) tested against the tool name.
// Copilot's file-editing tool names vary by surface (verified against CLI
// 1.0.63): `copilot -p` runs use `edit` ({path, old_str, new_str}) and `create`
// ({path, file_text}); interactive sessions and the cloud agent use
// `apply_patch` (a raw OpenAI-format patch string). The matcher covers all
// three. The same manifest is honored by both the CLI and the cloud/app agent.
// https://docs.github.com/en/copilot/reference/hooks-reference
export function buildGitHubHooksManifest() {
  return {
    version: 1,
    hooks: {
      postToolUse: [
        {
          type: 'command',
          matcher: 'edit|create|apply_patch',
          bash: guardedLauncher(launcherIn(GITHUB_PROJECT_SCRIPTS), 'hook', 'github'),
          timeoutSec: TIMEOUT_SECONDS,
        },
      ],
    },
  };
}

// Grok Build discovers project hooks from `.grok/hooks/*.json` and requires
// folder trust (`/hooks-trust` or `--trust`) before they run. Event schema is
// Claude-compatible (PostToolUse / Stop / PreToolUse); Claude tool names in
// matchers are aliased to Grok tools (Edit|Write|MultiEdit → search_replace).
// https://docs.x.ai/build/features/hooks
export function buildGrokHooksManifest() {
  return {
    hooks: buildClaudeCompatibleHooks('Edit|Write|MultiEdit', GROK_PROJECT_SCRIPTS, { provider: 'grok' }),
  };
}

// Gemini's hook timeouts are milliseconds. BeforeTool carries the session id
// into `build-phase` shell calls only; AfterAgent uses the engine's shared
// completion check. Gemini substitutes `$GEMINI_PROJECT_DIR` in the command
// text with an already shell-escaped path before `bash -c` runs it, so the
// token stays bare: inside double quotes the escaping would turn literal.
// There is no per-OS command field; a Windows install rewrites this to a
// PowerShell form (crates/skills hook_manifest.rs).
export function buildGeminiHooksManifest() {
  const launcher = '$GEMINI_PROJECT_DIR/.gemini/skills/phoenix-ui/scripts/phoenix-ui';
  const command = guardedLauncher(launcher, 'hook', 'gemini', launcher);
  return { hooks: {
    BeforeTool: [{ matcher: '^run_shell_command$', hooks: [{
      name: 'phoenix-ui-session', type: 'command', command, timeout: 5000,
    }] }],
    AfterAgent: [{ hooks: [{
      name: 'phoenix-ui-completion', type: 'command', command, timeout: 30000,
    }] }],
  } };
}

export function hooksJsonFor(provider, options = {}) {
  switch (provider) {
    case 'claude':
      return buildClaudeSettingsManifest();
    case 'codex':
      return buildCodexHooksManifest(options.configDir || '.codex');
    case 'cursor':
      return buildCursorHooksManifest();
    case 'github':
      return buildGitHubHooksManifest();
    case 'grok':
      return buildGrokHooksManifest();
    case 'gemini':
      return buildGeminiHooksManifest();
    default:
      return null;
  }
}
