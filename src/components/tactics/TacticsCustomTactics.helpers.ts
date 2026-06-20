import type { GameStateData } from "../../store/gameStore";
import type { TacticsLibraryEntry } from "./TacticsCommandBar";

const TACTICS_STORAGE_KEY_PREFIX = "ofm:tactics:custom";

type StorageLike = Pick<Storage, "getItem" | "setItem">;

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

export function buildCustomTacticsStorageKey(
  gameState: GameStateData,
): string {
  return [
    TACTICS_STORAGE_KEY_PREFIX,
    gameState.manager.id,
    gameState.clock.start_date,
    gameState.manager.team_id ?? "no-team",
  ].join(":");
}

function isCustomTacticEntry(value: unknown): value is TacticsLibraryEntry {
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

export function loadCustomTactics(
  gameState: GameStateData,
  storage: StorageLike | null = getDefaultStorage(),
): TacticsLibraryEntry[] {
  if (!storage) {
    return [];
  }

  try {
    const storedValue = storage.getItem(buildCustomTacticsStorageKey(gameState));

    if (!storedValue) {
      return [];
    }

    const parsedValue: unknown = JSON.parse(storedValue);

    if (!Array.isArray(parsedValue)) {
      return [];
    }

    return parsedValue.filter(isCustomTacticEntry);
  } catch {
    return [];
  }
}

export function saveCustomTactics(
  gameState: GameStateData,
  customTactics: readonly TacticsLibraryEntry[],
  storage: StorageLike | null = getDefaultStorage(),
): void {
  if (!storage) {
    return;
  }

  const persistedTactics = customTactics.filter(
    (entry): entry is TacticsLibraryEntry => entry.type === "custom",
  );

  try {
    storage.setItem(
      buildCustomTacticsStorageKey(gameState),
      JSON.stringify(persistedTactics),
    );
  } catch {
    // Storage quota exceeded or access denied — skip persist
  }
}
