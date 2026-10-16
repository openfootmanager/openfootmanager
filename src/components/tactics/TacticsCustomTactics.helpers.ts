import type { CustomTactic } from "../../services/customTacticsService";
import type { TacticsLibraryEntry } from "./TacticsCommandBar";

/**
 * Custom tactics used to live in the browser's storage, one list per manager, start date AND
 * club, so a change of club lost them and clearing the webview data lost them for good. They are
 * in the save now. What is left here reads an old browser library once, into the save.
 */
const LEGACY_KEY_PREFIX = "ofm:tactics:custom";

/** The error key the backend refuses a tactic with (`be.error.customTactic.*`). */
const REFUSED_PREFIX = "be.error.customTactic.";

type StorageLike = Pick<Storage, "getItem" | "removeItem" | "key" | "length">;

function getDefaultStorage(): StorageLike | null {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** The save lists tactics in the order they were made; the library shows the newest first. */
export function toLibraryEntries(tactics: readonly CustomTactic[]): TacticsLibraryEntry[] {
  return [...tactics].reverse().map((tactic) => ({
    description: tactic.description,
    formation: tactic.formation,
    id: tactic.id,
    name: tactic.name,
    playStyle: tactic.playStyle,
    sourcePresetName: tactic.sourcePresetName ?? null,
    type: "custom",
  }));
}

/** An id is kept trimmed, as the save keeps it, so padding can never make a second tactic. */
export function toCustomTactic(entry: TacticsLibraryEntry): CustomTactic {
  return {
    description: entry.description,
    formation: entry.formation,
    id: entry.id.trim(),
    name: entry.name,
    playStyle: entry.playStyle,
    sourcePresetName: entry.sourcePresetName ?? null,
  };
}

function isStoredTactic(value: unknown): value is TacticsLibraryEntry {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Partial<TacticsLibraryEntry>;

  return (
    candidate.type === "custom" &&
    typeof candidate.id === "string" &&
    typeof candidate.name === "string" &&
    typeof candidate.description === "string" &&
    typeof candidate.formation === "string" &&
    typeof candidate.playStyle === "string"
  );
}

/** Every key an old library of this career could be under, whichever club it was saved at. */
function legacyKeys(managerId: string, startDate: string, storage: StorageLike): string[] {
  const careerPrefix = `${LEGACY_KEY_PREFIX}:${managerId}:${startDate}:`;
  const keys: string[] = [];

  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (key?.startsWith(careerPrefix)) {
      keys.push(key);
    }
  }

  return keys;
}

function readStoredList(storage: StorageLike, key: string): TacticsLibraryEntry[] {
  try {
    const parsed: unknown = JSON.parse(storage.getItem(key) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter(isStoredTactic) : [];
  } catch {
    return [];
  }
}

/**
 * Moves an old browser library into the save, once.
 *
 * Each old tactic the save does not already hold (by trimmed id) is saved, oldest first, so the
 * order they were made in is kept. The old keys are cleared only when nothing was lost: a tactic
 * the game refuses (a shape it cannot lay out) will never be accepted, so it does not keep the
 * key alive; any other failure leaves the keys for the next try. A tactic already saved is
 * skipped, so a retry, a reload or a second window cannot import it twice.
 *
 * Returns the library as the save now holds it.
 */
export async function importLegacyTactics(
  managerId: string,
  startDate: string,
  saved: readonly CustomTactic[],
  save: (tactic: CustomTactic) => Promise<CustomTactic[]>,
  storage: StorageLike | null = getDefaultStorage(),
): Promise<CustomTactic[]> {
  if (!storage) {
    return [...saved];
  }

  const keys = legacyKeys(managerId, startDate, storage);
  if (keys.length === 0) {
    return [...saved];
  }

  let current = [...saved];
  const known = new Set(current.map((tactic) => tactic.id.trim()));
  // A stored list is newest first; the save keeps the order made.
  const oldestFirst = keys.flatMap((key) => readStoredList(storage, key).reverse());

  for (const entry of oldestFirst) {
    const tactic = toCustomTactic(entry);
    if (known.has(tactic.id)) {
      continue;
    }

    try {
      current = await save(tactic);
      known.add(tactic.id);
    } catch (error) {
      if (typeof error === "string" && error.startsWith(REFUSED_PREFIX)) {
        continue;
      }
      return current;
    }
  }

  for (const key of keys) {
    try {
      storage.removeItem(key);
    } catch {
      // Storage access denied: the keys stay, and the saved ids keep a retry from importing twice.
    }
  }

  return current;
}
