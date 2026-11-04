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

/** The keys reported since the last call. */
export function takeMissingKeys(): string[] {
  const keys = [...missing];
  missing.clear();
  return keys;
}
