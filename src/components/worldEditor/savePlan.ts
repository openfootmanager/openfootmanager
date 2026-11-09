import type { PackageCollections } from "../../store/packageEditorTypes";

/** The eight collections of an open project, as the editor holds them. */
export type EditorCollections = PackageCollections;

export type CollectionKey = keyof EditorCollections;

/**
 * Stands for "what is on disk for this collection is not known". A failed save can have renamed
 * some of its files before it failed, so the collection must be sent again whatever the editor
 * holds now, even if the author undoes back to a value that was once acknowledged. It equals no
 * collection, so every comparison against it reads as changed.
 */
export const UNKNOWN_ON_DISK: unique symbol = Symbol("unknown on disk");

/** What the backend last acknowledged writing for each collection, or that it is not known. */
export type Acknowledged = { [K in CollectionKey]: EditorCollections[K] | typeof UNKNOWN_ON_DISK };

const KEYS: CollectionKey[] = [
  "meta",
  "confederations",
  "countries",
  "teams",
  "players",
  "staff",
  "names",
  "competitions",
];

/**
 * What the next save must send: each collection whose value is not the one the backend last
 * acknowledged. A collection is compared by reference, which is exact because the editor never
 * mutates one in place: a changed collection is always a new array or object, and undoing back to
 * the acknowledged one restores the same reference.
 */
export function planSave(
  current: EditorCollections,
  acknowledged: Acknowledged,
  overrides?: Partial<EditorCollections>,
): Partial<EditorCollections> {
  const submitted: Partial<EditorCollections> = {};
  for (const key of KEYS) {
    const value = overrides?.[key] ?? current[key];
    if (value !== acknowledged[key]) Object.assign(submitted, { [key]: value });
  }
  return submitted;
}

/** Whether any collection differs from what the backend last acknowledged. */
export function hasUnsavedChanges(current: EditorCollections, acknowledged: Acknowledged): boolean {
  return KEYS.some((key) => current[key] !== acknowledged[key]);
}

/**
 * The acknowledged state after a save: only the collections the backend reported written, and only
 * as they were when submitted. A collection edited while the save ran keeps its newer value in the
 * editor, so it still differs from this and stays unsaved.
 */
export function acknowledge(
  acknowledged: Acknowledged,
  submitted: Partial<EditorCollections>,
  saved: readonly string[],
): Acknowledged {
  const next = { ...acknowledged };
  for (const key of KEYS) {
    if (saved.includes(key) && key in submitted) Object.assign(next, { [key]: submitted[key] });
  }
  return next;
}

/**
 * After a send that failed: every collection it carried is unknown on disk, because the backend
 * renames its files one after another and a failure partway leaves some of them new.
 */
export function invalidate(
  acknowledged: Acknowledged,
  submitted: Partial<EditorCollections>,
): Acknowledged {
  const next = { ...acknowledged };
  for (const key of KEYS) {
    if (key in submitted) Object.assign(next, { [key]: UNKNOWN_ON_DISK });
  }
  return next;
}

/** Collections of a loaded project, all acknowledged: it is exactly what is on disk. */
export function acknowledgedAsLoaded(loaded: EditorCollections): Acknowledged {
  return { ...loaded };
}

export type SaveOutcome = "clean" | "saved" | "stale";

export interface SaveDeps {
  /** The project generation now: it changes whenever another project is opened. */
  generation: () => number;
  current: () => EditorCollections;
  acknowledged: () => Acknowledged;
  send: (collections: Partial<EditorCollections>) => Promise<{ saved: string[] }>;
  onAcknowledged: (next: Acknowledged) => void;
}

/**
 * Save what changed, for the project that was open at `startedAt`. A save queued for a project that
 * has since been replaced sends nothing: its collections are read from the editor as it is now, and
 * would be written into the old project's folder. Nothing is acknowledged for a project that was
 * replaced while the save ran, either, or the new project would start with the old one's state.
 */
export async function saveChanges(
  startedAt: number,
  overrides: Partial<EditorCollections> | undefined,
  deps: SaveDeps,
): Promise<SaveOutcome> {
  if (deps.generation() !== startedAt) return "stale";
  const submitted = planSave(deps.current(), deps.acknowledged(), overrides);
  if (Object.keys(submitted).length === 0) return "clean";
  let receipt: { saved: string[] };
  try {
    receipt = await deps.send(submitted);
  } catch (error) {
    // The failure may have come after some files were renamed: nothing in this request is known
    // to be as acknowledged any more. A project opened meanwhile has its own state, left alone.
    if (deps.generation() === startedAt) {
      deps.onAcknowledged(invalidate(deps.acknowledged(), submitted));
    }
    throw error;
  }
  if (deps.generation() !== startedAt) return "stale";
  deps.onAcknowledged(acknowledge(deps.acknowledged(), submitted, receipt.saved));
  return "saved";
}
