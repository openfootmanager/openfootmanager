import type { CustomTactic } from "../../services/customTacticsService";
import type { TacticsLibraryEntry } from "./TacticsCommandBar";

/**
 * Custom tactics used to live in the browser's storage, one list per manager, start date AND
 * club, so a change of club lost them and clearing the webview data lost them for good. They are
 * in the save now. What is left here copies an old browser library into the save.
 */
const LEGACY_KEY_PREFIX = "ofm:tactics:custom";

/** The error key the backend refuses a tactic with (`be.error.customTactic.*`). */
const REFUSED_PREFIX = "be.error.customTactic.";

/**
 * The longest name the save keeps (`MAX_NAME_CHARS` in `ofm_core::custom_tactics`). The old screen
 * had no limit, so a name this long is plausible; it is shortened rather than lost.
 */
const MAX_NAME_CHARS = 60;

type StorageLike = Pick<Storage, "getItem" | "key" | "length">;

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

/** The library as the save holds it after an import, and whether nothing is left to retry. */
export interface LegacyImport {
  tactics: CustomTactic[];
  /** False when a save failed part way: the rest must be tried again, so it is not marked done. */
  complete: boolean;
}

/**
 * Copies an old browser library into the save.
 *
 * It copies and never moves, because the old key is not one career's: every career's manager has
 * the same id and a career starts on the same day of its start year, so careers begun in the same
 * year all read and write the one key. Removing it would take the library from the others.
 * Instead each old tactic the save does not already hold (by trimmed id) is saved, oldest first so
 * the order they were made in is kept; one the save holds is skipped, so a reload, a retry or a
 * second career cannot import it twice. A name longer than the save keeps is shortened. A tactic
 * the game still refuses (a shape it cannot lay out) is left out of the save and stays where it
 * is, in the browser's storage; a failed save stops the import for another try, and the result
 * says it is not complete.
 *
 * The import runs once per career (the save records it), so a tactic deleted afterwards is not
 * copied back; the caller decides whether it still has to run.
 */
export async function importLegacyTactics(
  managerId: string,
  startDate: string,
  saved: readonly CustomTactic[],
  save: (tactic: CustomTactic) => Promise<CustomTactic[]>,
  storage: StorageLike | null = getDefaultStorage(),
): Promise<LegacyImport> {
  let current = [...saved];
  if (!storage) {
    return { tactics: current, complete: true };
  }

  const known = new Set(current.map((tactic) => tactic.id.trim()));
  // A stored list is newest first; the save keeps the order made.
  const oldestFirst = legacyKeys(managerId, startDate, storage).flatMap((key) =>
    readStoredList(storage, key).reverse(),
  );

  for (const entry of oldestFirst) {
    const tactic = toCustomTactic({ ...entry, name: fitName(entry.name) });
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
      return { tactics: current, complete: false };
    }
  }

  return { tactics: current, complete: true };
}

function fitName(name: string): string {
  return Array.from(name.trim()).slice(0, MAX_NAME_CHARS).join("");
}
