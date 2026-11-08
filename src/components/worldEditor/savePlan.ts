import type {
  CompetitionDef,
  ConfederationDef,
  CountryDef,
  NamesDefinition,
  PlayerDef,
  StaffDef,
  TeamDef,
  WorldMetaDef,
} from "../menu/PackageEditor/types";

/** The eight collections of an open project, as the editor holds them. */
export interface EditorCollections {
  meta: WorldMetaDef;
  confederations: ConfederationDef[];
  countries: CountryDef[];
  teams: TeamDef[];
  players: PlayerDef[];
  staff: StaffDef[];
  names: NamesDefinition;
  competitions: CompetitionDef[];
}

export type CollectionKey = keyof EditorCollections;

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
  acknowledged: EditorCollections,
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
export function hasUnsavedChanges(
  current: EditorCollections,
  acknowledged: EditorCollections,
): boolean {
  return KEYS.some((key) => current[key] !== acknowledged[key]);
}

/**
 * The acknowledged state after a save: only the collections the backend reported written, and only
 * as they were when submitted. A collection edited while the save ran keeps its newer value in the
 * editor, so it still differs from this and stays unsaved.
 */
export function acknowledge(
  acknowledged: EditorCollections,
  submitted: Partial<EditorCollections>,
  saved: readonly string[],
): EditorCollections {
  const next = { ...acknowledged };
  for (const key of KEYS) {
    if (saved.includes(key) && key in submitted) Object.assign(next, { [key]: submitted[key] });
  }
  return next;
}

export type SaveOutcome = "clean" | "saved" | "stale";

export interface SaveDeps {
  /** The project generation now: it changes whenever another project is opened. */
  generation: () => number;
  current: () => EditorCollections;
  acknowledged: () => EditorCollections;
  send: (collections: Partial<EditorCollections>) => Promise<{ saved: string[] }>;
  onAcknowledged: (next: EditorCollections) => void;
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
  const receipt = await deps.send(submitted);
  if (deps.generation() !== startedAt) return "stale";
  deps.onAcknowledged(acknowledge(deps.acknowledged(), submitted, receipt.saved));
  return "saved";
}
