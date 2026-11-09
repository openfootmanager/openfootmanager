import { createWriteQueue } from "../../lib/writeQueue";
import {
  acknowledgedAsLoaded,
  saveChanges,
  type Acknowledged,
  type EditorCollections,
  type SaveOutcome,
} from "./savePlan";

export interface SaveSessionDeps {
  /** The collections the editor holds now, read when a save runs rather than when it is asked for. */
  current: () => EditorCollections;
  /** Write `collections` into the project folder `dir`; reports the collections written. */
  send: (dir: string, collections: Partial<EditorCollections>) => Promise<{ saved: string[] }>;
  /** The acknowledged state changed (null once no project is open). */
  onChange: (acknowledged: Acknowledged | null) => void;
}

/**
 * The editor's saves for the project that is open: one write at a time, in the order asked for,
 * each tied to the project it was asked for. Opening another project bumps the generation, so a
 * save still queued for the previous one sends nothing and one still in flight acknowledges nothing.
 */
export function createSaveSession(deps: SaveSessionDeps) {
  const enqueue = createWriteQueue();
  let generation = 0;
  let acknowledged: Acknowledged | null = null;

  function setAcknowledged(next: Acknowledged | null) {
    acknowledged = next;
    deps.onChange(next);
  }

  return {
    /** Another project is now open, exactly as `loaded` holds it. */
    openProject(loaded: EditorCollections) {
      generation += 1;
      setAcknowledged(acknowledgedAsLoaded(loaded));
    },

    /**
     * Save what changed in the project in `dir`. Which project that is, is fixed now, when the save
     * is asked for: by the time it reaches the front of the queue another may be open.
     */
    save(dir: string, overrides?: Partial<EditorCollections>): Promise<SaveOutcome> {
      const startedAt = generation;
      return enqueue(() =>
        saveChanges(startedAt, overrides, {
          generation: () => generation,
          current: deps.current,
          acknowledged: () => acknowledged as Acknowledged,
          send: (collections) => deps.send(dir, collections),
          onAcknowledged: setAcknowledged,
        }),
      );
    },
  };
}
