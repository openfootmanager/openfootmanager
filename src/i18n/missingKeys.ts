/**
 * Keys the app asked i18next for and no locale had. A missing key renders as its raw name
 * ("be.msg.foo.subject") to the player, so it is a bug wherever it happens.
 *
 * In a dev build each one is logged loudly. Under vitest `src/test-setup.ts` fails the test that
 * rendered it. A production build keeps no record: it shows the key and moves on.
 */
const missing = new Set<string>();

export function reportMissingKey(language: string, key: string): void {
  missing.add(key);
  if (import.meta.env.DEV) {
    console.error(`[i18n] missing translation key "${key}" (language: ${language})`);
  }
}

// The locale's top-level namespaces, supplied by `i18n/index.ts` once English has loaded. Asked of
// i18next rather than imported from en.json, so the locale file is not pulled into the main chunk.
let topLevelNamespaces: () => ReadonlySet<string> = () => new Set();

export function provideTopLevelNamespaces(source: () => ReadonlySet<string>): void {
  topLevelNamespaces = source;
}

/**
 * Whether a value is shaped like a translation key: dotted, no spaces, and under one of the
 * locale's top-level namespaces ("be.msg.x", not "€1.8M" or "Mr. Smith"). Prose and figures that
 * happen to contain a dot are not.
 */
export function looksLikeTranslationKey(value: string): boolean {
  return (
    /^[A-Za-z][A-Za-z0-9_-]*(\.[A-Za-z0-9_-]+)+$/.test(value) &&
    topLevelNamespaces().has(value.slice(0, value.indexOf(".")))
  );
}

/**
 * A resolver asks `i18n.exists(value)` before translating a string that may be a key or plain text.
 * When the answer is no, a value that is shaped like a key is a missing key the player would read
 * raw, so it is reported here as `exists` would otherwise hide it. Dev and tests only.
 */
export function reportIfKeyShaped(language: string, value: string): void {
  if (import.meta.env.DEV && looksLikeTranslationKey(value)) reportMissingKey(language, value);
}

/** The keys reported since the last call. */
export function takeMissingKeys(): string[] {
  const keys = [...missing];
  missing.clear();
  return keys;
}
