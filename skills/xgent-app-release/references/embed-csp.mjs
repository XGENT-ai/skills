// Standalone CSP declaration syntax snapshot; server capability and approval remain separate.
export const EMBED_CSP_SOURCE_FIELDS = Object.freeze([
  "connectSrc",
  "scriptSrc",
  "styleSrc",
  "fontSrc",
  "imgSrc",
  "mediaSrc"
]);
export const EMBED_CSP_PLATFORM_FIELDS = Object.freeze([
  "scriptSrc",
  "styleSrc",
  "fontSrc",
  "connectSrc"
]);
const unsafe = /[\s\u0000-\u001f\u007f-\u009f\\'"`{};*]/u;
const CONNECT_SCHEME_SOURCES = ["blob:", "data:"];
function fail(field, reason) {
  throw new Error(`${field}: ${reason}`);
}
function record(value, allowed, field) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail(field, "expected an object");
  }
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null)
    fail(field, "expected a plain object");
  for (const key of Reflect.ownKeys(value)) {
    if (typeof key !== "string" || !allowed.includes(key))
      fail(field, "unknown field");
    if (!("value" in Object.getOwnPropertyDescriptor(value, key)))
      fail(field, "accessors are not allowed");
  }
  return value;
}
function origin(value, field, options, allowWebSocket, allowPath = false) {
  if (typeof value !== "string")
    fail(field, "expected a string");
  const source = value.replace(/^ +| +$/g, "");
  if (!source || unsafe.test(source))
    fail(field, "empty source or unsafe characters");
  const parts = /^([a-z][a-z0-9+.-]*):\/\/([^/?#]+)(\/[^?#]*)?$/i.exec(source);
  if (!parts)
    fail(field, "expected an explicit origin without query or fragment");
  if (parts[2].includes("@"))
    fail(field, "credentials are not allowed");
  if (!allowPath && parts[3] !== undefined && parts[3] !== "/")
    fail(field, "paths are not allowed");
  let url;
  try {
    url = new URL(source);
  } catch {
    fail(field, "invalid URL");
  }
  const secure = url.protocol === "https:" || allowWebSocket && url.protocol === "wss:";
  const devScheme = url.protocol === "http:" || allowWebSocket && url.protocol === "ws:";
  const exactLoopback = /^(?:localhost|127\.0\.0\.1|\[::1\])(?::[0-9]+)?$/i.test(parts[2]);
  if (!secure && !(options.allowDevLoopback === true && devScheme && exactLoopback)) {
    fail(field, "scheme is not allowed; insecure origins require explicit development loopback");
  }
  if (url.username || url.password || url.search || url.hash || url.origin === "null" || unsafe.test(url.origin)) {
    fail(field, "invalid origin");
  }
  return url.origin;
}
export function normalizeEmbedCsp(value, options = {}) {
  if (value === undefined || value === null)
    return value;
  const input = record(value, [...EMBED_CSP_SOURCE_FIELDS, "platformSources"], "embedCsp");
  const result = {};
  for (const field of EMBED_CSP_SOURCE_FIELDS) {
    if (!Object.prototype.hasOwnProperty.call(input, field))
      continue;
    const sources = input[field];
    if (!Array.isArray(sources))
      fail(`embedCsp.${field}`, "expected an array");
    const normalized = [];
    for (const source of sources) {
      if (field === "connectSrc" && typeof source === "string" && CONNECT_SCHEME_SOURCES.includes(source.replace(/^ +| +$/g, "")))
        normalized.push(source.replace(/^ +| +$/g, ""));
      else
        normalized.push(origin(source, `embedCsp.${field}`, options, field === "connectSrc"));
    }
    if (normalized.length)
      result[field] = [...new Set(normalized)].sort();
  }
  if (Object.prototype.hasOwnProperty.call(input, "platformSources")) {
    const aliases = record(input.platformSources, EMBED_CSP_PLATFORM_FIELDS, "embedCsp.platformSources");
    const platformSources = {};
    for (const field of EMBED_CSP_PLATFORM_FIELDS) {
      if (!Object.prototype.hasOwnProperty.call(aliases, field))
        continue;
      const values = aliases[field];
      if (!Array.isArray(values))
        fail(`embedCsp.platformSources.${field}`, "expected an array");
      for (const alias of values) {
        if (alias !== "jsCdn")
          fail(`embedCsp.platformSources.${field}`, "expected jsCdn alias");
      }
      if (values.length)
        platformSources[field] = ["jsCdn"];
    }
    if (Object.keys(platformSources).length)
      result.platformSources = platformSources;
  }
  return result;
}
export function resolveEmbedCsp(csp, jsCdnBase, options = {}) {
  const normalized = normalizeEmbedCsp(csp, options);
  const result = {};
  if (!normalized)
    return result;
  for (const field of EMBED_CSP_SOURCE_FIELDS) {
    if (normalized[field])
      result[field] = [...normalized[field]];
  }
  if (normalized.platformSources) {
    const cdnOrigin = origin(jsCdnBase, "jsCdnBase", options, false, true);
    for (const field of EMBED_CSP_PLATFORM_FIELDS) {
      if (normalized.platformSources[field]?.includes("jsCdn")) {
        result[field] = [...new Set([...result[field] ?? [], cdnOrigin])].sort();
      }
    }
  }
  return result;
}
