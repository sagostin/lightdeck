/**
 * Deep copy of plain show data. Works on Svelte's live state as well as plain objects
 * (structuredClone throws on reactive proxies, so the shared libraries use this instead).
 */
export function clone<T>(x: T): T {
  return x === undefined ? x : JSON.parse(JSON.stringify(x));
}
