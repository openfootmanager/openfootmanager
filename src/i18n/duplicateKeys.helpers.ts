export interface Duplicate {
  /** The key's path from the root, e.g. `match.playerRoles`. */
  path: string;
  key: string;
}

type Frame =
  | { kind: "object"; keys: Set<string>; expectKey: boolean; name: string | null }
  | { kind: "array"; name: string | null };

/**
 * The keys that appear more than once in the same object of a JSON text. `JSON.parse` keeps the
 * last of two equal keys and says nothing, so a repeated key drops a translation unseen.
 *
 * It reads the text once, tracking each open object's keys. A string is a key only where an object
 * expects one, so a value that looks like a key, an escaped quote, or the same key in two different
 * objects is not a hit. Keys are decoded before they are compared, so `"a"` and `"a"` are the
 * same key. A string that never closes (a truncated or badly merged file) throws, rather than
 * running past the end of the text.
 */
export function findDuplicateKeys(text: string): Duplicate[] {
  const duplicates: Duplicate[] = [];
  const stack: Frame[] = [];
  // The key the next `{` or `[` is the value of, which names that container in a path.
  let pendingName: string | null = null;
  const pathOf = (): string[] =>
    stack.flatMap((frame) => (frame.name !== null ? [frame.name] : []));

  let index = 0;
  while (index < text.length) {
    const char = text[index];
    if (char === "{") {
      stack.push({ kind: "object", keys: new Set(), expectKey: true, name: pendingName });
      pendingName = null;
    } else if (char === "[") {
      stack.push({ kind: "array", name: pendingName });
      pendingName = null;
    } else if (char === "}" || char === "]") {
      stack.pop();
    } else if (char === ",") {
      const top = stack[stack.length - 1];
      if (top?.kind === "object") top.expectKey = true;
    } else if (char === '"') {
      let end = index + 1;
      while (end < text.length && text[end] !== '"') end += text[end] === "\\" ? 2 : 1;
      if (end >= text.length) {
        throw new Error(`unterminated string starting at offset ${index}`);
      }
      const top = stack[stack.length - 1];
      if (top?.kind === "object" && top.expectKey) {
        const key = JSON.parse(text.slice(index, end + 1)) as string;
        if (top.keys.has(key)) {
          duplicates.push({ path: [...pathOf(), key].join("."), key });
        }
        top.keys.add(key);
        top.expectKey = false;
        // If an object or array is this key's value, the key names it.
        pendingName = /^\s*:\s*[{[]/.test(text.slice(end + 1, end + 40)) ? key : null;
      }
      index = end;
    }
    index += 1;
  }
  return duplicates;
}
