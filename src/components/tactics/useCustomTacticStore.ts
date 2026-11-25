import { useEffect, useRef, useState } from "react";

import { useGameStore } from "../../store/gameStore";
import {
  type CustomTactic,
  legacyTacticsImportPending,
  listCustomTactics,
  markLegacyTacticsImported,
  saveCurrentTactic,
  saveCustomTactic,
} from "../../services/customTacticsService";
import {
  importLegacyTactics,
  layPendingChanges,
  toCustomTactic,
  toLibraryEntries,
} from "./TacticsCustomTactics.helpers";
import type { TacticsLibraryEntry } from "./TacticPicker";

interface UseCustomTacticStoreArgs {
  /** The career's identity: the manager, and the date its game began. Null with no game open. */
  managerId: string | null;
  careerStartDate: string | null;
  /** Handed back with every write so the backend can refuse one meant for another career. */
  careerToken: number;
  /** A write the backend refused: the screen tells the manager it did not save. */
  onWriteError: (error: unknown) => void;
  /** A custom-tactic save or delete the backend accepted: the screen drops an earlier alert. */
  onWriteSaved: () => void;
}

/**
 * The career's saved tactics as the screen holds them, and everything that talks to the save about
 * them: the first load, the one-off import of an old browser library, and the writes (`changeSave`),
 * which all run one at a time, in the order they were asked. It takes only the career's identity, not
 * the game: what it reads of the game is the manager, the career token and the start date.
 */
export function useCustomTacticStore({
  managerId,
  careerStartDate,
  careerToken,
  onWriteError,
  onWriteSaved,
}: UseCustomTacticStoreArgs) {
  // The library is the career's, kept in the save: it follows a change of club and a reload. It
  // starts empty and is filled from the save (and from an old browser library, once) below.
  const [customTactics, setCustomTactics] = useState<TacticsLibraryEntry[]>([]);
  // The library as of the latest render, for a step that reads it after an await.
  const latestCustomTactics = useRef(customTactics);
  latestCustomTactics.current = customTactics;

  // Everything that talks to the save about the library runs one step at a time, in the order it
  // was asked: the first load, the import of an old browser library, and every save. Replies from
  // overlapping calls are not promised to arrive in order, and a load that began before a save must
  // not put its older list over the save's newer one. Each step's reply is the whole list, so the
  // last step to finish is the library as the save has it.
  const savesInOrder = useRef<Promise<void>>(Promise.resolve());
  // Whether the first load (and the one-off import) of this career's library has settled. A save
  // asked before then would queue behind it, and record whatever the manager changed meanwhile.
  const [libraryLoaded, setLibraryLoaded] = useState(false);
  // Which asking of the load is the latest. Effects can run twice with the same epoch (StrictMode,
  // a token change), and each asking queues its own load behind the last: only the latest one
  // settling means the queue holds no load, so an earlier one settling must not turn Save on.
  const loadGeneration = useRef(0);
  const confirmedLibrary = useRef<{ epoch: number; entries: TacticsLibraryEntry[] } | null>(null);
  function confirmLibrary(tactics: CustomTactic[], epoch: number): TacticsLibraryEntry[] {
    const entries = toLibraryEntries(tactics);
    confirmedLibrary.current = { epoch, entries };
    return entries;
  }
  // What the manager has made, changed or deleted whose own save has not answered yet. A save's reply
  // is the whole list as the save had it when it answered, so one that predates a later change does
  // not show it: published as it is, it would drop a tactic just made (and the active tactic with
  // it), bring back one just deleted, or put back the fields of an earlier save of the same tactic.
  // What is pending is laid over every reply. Each id has one owner: the latest operation on it, so
  // an earlier operation settling leaves the later one's change in place, and an operation removes
  // its change only when it still owns it, before its own reply is published (a refused change is
  // then absent from the list the save hands back).
  const pendingChanges = useRef(
    new Map<string, { operation: number; entry: TacticsLibraryEntry | null }>(),
  );
  const operations = useRef(0);
  function withPending(entries: TacticsLibraryEntry[]): TacticsLibraryEntry[] {
    return layPendingChanges(
      entries,
      new Map([...pendingChanges.current].map(([id, change]) => [id, change.entry])),
    );
  }
  // The career whose library is on screen. A load for another career empties the picker at once,
  // so its answer does not have to replace the old career's tactics; the same career reloading under
  // a new career token keeps what it shows.
  const shownCareer = useRef<string | null>(null);
  // The commands act on whichever game is active, and a read carries no career with it. A step still
  // waiting in the queue when the career changes (another game loaded or the game left) would
  // write one career's tactic into the next, so each step remembers the store's career epoch it
  // was asked in and, once that is no longer the current one, is skipped if it has not started and
  // changes nothing on the screen if it has. What it does not do is stop a call already on its way:
  // the writes carry the career token, which the backend checks, but a read (the list asked after a
  // refused write) is not career-scoped; its answer is dropped, not prevented. Leaving this screen
  // while the same career stays open changes nothing: the step is still that career's.
  const careerEpoch = useGameStore((state) => state.careerEpoch);

  useEffect(() => {
    setLibraryLoaded(false);
    const career = `${managerId}|${careerStartDate}|${careerEpoch}`;
    if (shownCareer.current !== null && shownCareer.current !== career) {
      setCustomTactics([]);
      pendingChanges.current.clear();
      confirmedLibrary.current = null;
    }
    shownCareer.current = career;
    if (!managerId || !careerStartDate) return;

    const askedIn = useGameStore.getState().careerEpoch;
    const stale = () => useGameStore.getState().careerEpoch !== askedIn;
    const generation = ++loadGeneration.current;
    const isLatestLoad = () => !stale() && generation === loadGeneration.current;
    savesInOrder.current = savesInOrder.current
      .then(async () => {
        if (stale()) return;
        let tactics = await listCustomTactics();
        // Shown at once: a step after this one that fails must not hide the list that was read.
        if (!stale()) setCustomTactics(withPending(confirmLibrary(tactics, askedIn)));
        // Once per career: the save records that the old browser library was copied in, so a
        // tactic the player has since deleted is not copied back from the browser's key.
        if (await legacyTacticsImportPending()) {
          const imported = await importLegacyTactics(
            managerId,
            careerStartDate,
            tactics,
            async (tactic) => {
              if (stale()) throw new Error("the career changed");
              const saved = await saveCustomTactic(tactic, careerToken);
              if (!stale()) confirmLibrary(saved, askedIn);
              return saved;
            },
          );
          tactics = imported.tactics;
          if (imported.complete && !stale()) await markLegacyTacticsImported(careerToken);
        }
        if (!stale()) setCustomTactics(withPending(confirmLibrary(tactics, askedIn)));
        if (isLatestLoad()) setLibraryLoaded(true);
      })
      // A load that failed still ends the wait: Save is the way to put a tactic in the save. What it
      // did read or import before it failed stays on screen.
      .catch(() => {
        const confirmed = confirmedLibrary.current;
        if (!stale() && confirmed?.epoch === askedIn)
          setCustomTactics(withPending(confirmed.entries));
        if (isLatestLoad()) setLibraryLoaded(true);
      });
  }, [managerId, careerStartDate, careerEpoch, careerToken]);

  // The screen answers at once; the save's own list then replaces it, or, if the save refused the
  // change (a shape the game cannot lay out, an id that is already gone), puts the library back as
  // the save has it.
  // Resolves whether the save accepted the change, once it has answered.
  function changeSave(
    change: () => Promise<CustomTactic[]>,
    // Where a refusal goes: the screen's alert, or a dialog that asked and shows the reason itself.
    reportRefusal: (error: unknown) => void = onWriteError,
    // What this change makes, changes or (with no entry) deletes, shown until the save has answered.
    pending?: { id: string; entry?: TacticsLibraryEntry },
  ): Promise<boolean> {
    const operation = ++operations.current;
    if (pending) {
      pendingChanges.current.set(pending.id, { operation, entry: pending.entry ?? null });
    }
    const release = () => {
      if (pending && pendingChanges.current.get(pending.id)?.operation === operation) {
        pendingChanges.current.delete(pending.id);
      }
    };
    const askedIn = useGameStore.getState().careerEpoch;
    const stale = () => useGameStore.getState().careerEpoch !== askedIn;
    let accepted = true;
    const answered = savesInOrder.current
      .then(() =>
        stale()
          ? null
          : change().catch((error: unknown) => {
              // The save refused the change: tell the screen, and put the library back as the
              // save has it.
              accepted = false;
              if (!stale()) reportRefusal(error);
              return listCustomTactics().catch(() => {
                if (!stale()) {
                  release();
                  const confirmed = confirmedLibrary.current;
                  setCustomTactics(
                    withPending(confirmed?.epoch === askedIn ? confirmed.entries : []),
                  );
                }
                return null;
              });
            }),
      )
      .then((saved) => {
        release();
        if (saved && !stale()) setCustomTactics(withPending(confirmLibrary(saved, askedIn)));
        const held = saved !== null && accepted && !stale();
        if (held) onWriteSaved();
        return accepted && !stale();
      })
      .catch(() => {
        release();
        return false;
      });
    savesInOrder.current = answered.then(() => undefined);
    return answered;
  }

  // A save is the team's tactic as it stands, whole (#1189): the backend reads it from the team,
  // and the entry only gives it its id, name and description.
  function persistTactic(tactic: TacticsLibraryEntry): Promise<boolean> {
    return changeSave(() => saveCurrentTactic(toCustomTactic(tactic), careerToken), onWriteError, {
      id: tactic.id,
      entry: tactic,
    });
  }

  return {
    customTactics,
    setCustomTactics,
    latestCustomTactics,
    libraryLoaded,
    changeSave,
    persistTactic,
    careerToken,
  };
}
