/** Windows spellings of machine-dependent paths; POSIX text has none of these forms. */

const jsonEscaped = (text) => JSON.stringify(text).slice(1, -1);

/** Substitute placeholders inside string values; a Windows path is not valid raw JSON text. */
export function stdinJson(value, sub) {
  return JSON.stringify(value, (key, inner) => typeof inner === 'string' ? sub(inner) : inner);
}

/** Raw, JSON-escaped and forward-slash spellings of one machine path. */
export function pathForms(value) {
  return [...new Set([value, jsonEscaped(value), value.replaceAll('\\', '/')])];
}

/**
 * Windows prints the hook administration command through
 * `quote_command_arg(self, true)`: double quotes and doubled backslashes.
 * JSON output escapes that command and the executable path once more.
 */
export function windowsBinaryMasks(text, bin) {
  if (!bin.includes('\\')) return text;
  const quoted = `"${bin.replaceAll('\\', '\\\\').replaceAll('"', '\\"')}"`;
  let out = text;
  for (const form of [`${quoted} hooks`, jsonEscaped(`${quoted} hooks`)]) out = out.split(form).join('<HOOK_ADMIN_CMD>');
  for (const form of [`'${jsonEscaped(bin)}'`, jsonEscaped(bin)]) out = out.split(form).join('<IMPECCABLE>');
  return out;
}

/** The historical relative-climb mask, for raw and JSON-escaped backslash climbs. */
export function windowsClimbMask(text) {
  return text
    .replace(/(?:\.\.\\\\){2,}(?=\.phoenix-ui\\\\)/g, '<UP_TO_ROOT>\\\\')
    .replace(/(?:\.\.\\){2,}(?=\.phoenix-ui\\)/g, '<UP_TO_ROOT>\\');
}
