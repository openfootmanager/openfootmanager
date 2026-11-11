import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { takeMissingKeys } from "./i18n/missingKeys";

// Polyfill localStorage for jsdom environment in Node 22+.
// Node 24+ removed the built-in localStorage implementation and now requires
// the --localstorage-file flag. Vitest's jsdom environment doesn't pass that
// flag, so we provide a minimal in-memory shim here.
if (typeof localStorage === "undefined" || localStorage === null) {
  const store: Record<string, string> = {};
  Object.defineProperty(globalThis, "localStorage", {
    value: {
      getItem(key: string) {
        return key in store ? store[key] : null;
      },
      setItem(key: string, value: string) {
        store[key] = value;
      },
      removeItem(key: string) {
        delete store[key];
      },
      clear() {
        for (const key of Object.keys(store)) {
          delete store[key];
        }
      },
      get length() {
        return Object.keys(store).length;
      },
      key(index: number) {
        const keys = Object.keys(store);
        return keys[index] ?? null;
      },
    },
    writable: false,
    configurable: true,
  });
}

// A key no locale has renders to the player as its raw name. The real i18n instance reports each
// one it was asked for; the test that rendered it fails here, naming the key. A test that is
// meant to ask for an absent key drains the report itself (`takeMissingKeys()`).
afterEach(() => {
  const missing = takeMissingKeys();
  if (missing.length > 0) {
    throw new Error(`Rendered translation keys that no locale has: ${missing.join(", ")}`);
  }
});
