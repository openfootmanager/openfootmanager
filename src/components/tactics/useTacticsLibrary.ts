import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { type GameStateData, useGameStore } from "../../store/gameStore";
import {
  applyCustomTactic,
  type CustomTactic,
  deleteCustomTactic,
  legacyTacticsImportPending,
  listCustomTactics,
  markLegacyTacticsImported,
  saveCurrentTactic,
  saveCustomTactic,
} from "../../services/customTacticsService";
import { getTacticsBlueprintStatus } from "../../services/squadService";
import {
  TACTICS_PRESETS,
  findTacticsPresetBySetup,
  type TacticsPresetDefinition,
} from "./TacticsTab.helpers";
import {
  importLegacyTactics,
  toCustomTactic,
  toLibraryEntries,
} from "./TacticsCustomTactics.helpers";
import type { TacticsLibraryEntry } from "./TacticPicker";

interface UseTacticsLibraryArgs {
  gameState: GameStateData | null;
  formation: string;
  activePlayStyle: string;
  initialPreset: TacticsPresetDefinition | null;
  onApplyPreset: (formation: string, playStyle: string) => Promise<boolean>;
  onGameUpdate: (game: GameStateData) => void;
  /** A write the backend refused: the screen tells the manager it did not save. */
  onWriteError: (error: unknown) => void;
  /** A custom-tactic save or delete the backend accepted: the screen drops an earlier alert. */
  onWriteSaved: () => void;
}

export function useTacticsLibrary({
  gameState,
  formation,
  activePlayStyle,
  initialPreset,
  onApplyPreset,
  onGameUpdate,
  onWriteError,
  onWriteSaved,
}: UseTacticsLibraryArgs) {
  const { t } = useTranslation();
  // The library is the career's, kept in the save: it follows a change of club and a reload. It
  // starts empty and is filled from the save (and from an old browser library, once) below.
  const [customTactics, setCustomTactics] = useState<TacticsLibraryEntry[]>([]);
  const [activeTacticId, setActiveTacticId] = useState<string | null>(
    initialPreset ? `preset:${initialPreset.id}` : null,
  );
  const [draftTacticName, setDraftTacticName] = useState(
    initialPreset?.id
      ? t(`tactics.presetNames.${initialPreset.id}`, initialPreset.id)
      : t("tactics.customTactic"),
  );
  const [presetAnchorId, setPresetAnchorId] = useState<string | null>(initialPreset?.id ?? null);
  const managerId = gameState?.manager.id ?? null;
  // Handed back with every write so the backend can refuse one meant for another career.
  const careerToken = gameState?.career_token ?? 0;
  const careerStartDate = gameState?.clock.start_date ?? null;

  // Whether the phase dials still equal the style's blueprint is the backend's rule (`blueprint_for`);
  // it is asked again whenever the dials or the style change.
  const [blueprintModified, setBlueprintModified] = useState(false);
  const userTeam = gameState?.teams.find((team) => team.id === gameState.manager.team_id);
  const blueprintKey = `${activePlayStyle}|${JSON.stringify(userTeam?.tactics_phase ?? null)}`;

  const hasClub = Boolean(userTeam);

  useEffect(() => {
    // No game or no club has no blueprint to ask about.
    if (!hasClub) {
      setBlueprintModified(false);
      return;
    }

    let cancelled = false;
    getTacticsBlueprintStatus()
      .then(({ modified }) => {
        if (!cancelled) setBlueprintModified(modified);
      })
      .catch(() => {
        if (!cancelled) setBlueprintModified(false);
      });
    return () => {
      cancelled = true;
    };
  }, [blueprintKey, hasClub]);

  // Everything that talks to the save about the library runs one step at a time, in the order it
  // was asked: the first load, the import of an old browser library, and every save. Replies from
  // overlapping calls are not promised to arrive in order, and a load that began before a save must
  // not put its older list over the save's newer one. Each step's reply is the whole list, so the
  // last step to finish is the library as the save has it.
  const savesInOrder = useRef<Promise<void>>(Promise.resolve());
  const confirmedLibrary = useRef<{ epoch: number; entries: TacticsLibraryEntry[] } | null>(null);
  function confirmLibrary(tactics: CustomTactic[], epoch: number): TacticsLibraryEntry[] {
    const entries = toLibraryEntries(tactics);
    confirmedLibrary.current = { epoch, entries };
    return entries;
  }
  // The commands act on whichever game is active, and carry no career with them. A step still
  // waiting in the queue when the career changes (another game loaded or the game left) would
  // write one career's tactic into the next, so each step remembers the store's career epoch it
  // was asked in and does nothing once that is no longer the current one. Leaving this screen
  // while the same career stays open changes nothing: the step is still that career's.
  const careerEpoch = useGameStore((state) => state.careerEpoch);

  useEffect(() => {
    if (!managerId || !careerStartDate) return;

    const askedIn = useGameStore.getState().careerEpoch;
    const stale = () => useGameStore.getState().careerEpoch !== askedIn;
    savesInOrder.current = savesInOrder.current
      .then(async () => {
        if (stale()) return;
        let tactics = await listCustomTactics();
        if (!stale()) confirmLibrary(tactics, askedIn);
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
        if (!stale()) setCustomTactics(confirmLibrary(tactics, askedIn));
      })
      .catch(() => {});
  }, [managerId, careerStartDate, careerEpoch, careerToken]);

  // The screen answers at once; the save's own list then replaces it, or, if the save refused the
  // change (a shape the game cannot lay out, an id that is already gone), puts the library back as
  // the save has it.
  // Resolves whether the save accepted the change, once it has answered.
  function changeSave(change: () => Promise<CustomTactic[]>): Promise<boolean> {
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
              if (!stale()) onWriteError(error);
              return listCustomTactics().catch(() => {
                if (!stale()) {
                  const confirmed = confirmedLibrary.current;
                  setCustomTactics(confirmed?.epoch === askedIn ? confirmed.entries : []);
                }
                return null;
              });
            }),
      )
      .then((saved) => {
        if (saved && !stale()) setCustomTactics(confirmLibrary(saved, askedIn));
        const held = saved !== null && accepted && !stale();
        if (held) onWriteSaved();
        return accepted && !stale();
      })
      .catch(() => false);
    savesInOrder.current = answered.then(() => undefined);
    return answered;
  }

  // A save is the team's tactic as it stands, whole (#1189): the backend reads it from the team,
  // and the entry only gives it its id, name and description.
  function persistTactic(tactic: TacticsLibraryEntry): Promise<boolean> {
    return changeSave(() => saveCurrentTactic(toCustomTactic(tactic), careerToken));
  }

  const matchedPreset = findTacticsPresetBySetup(formation, activePlayStyle);
  const anchoredPreset = presetAnchorId
    ? (TACTICS_PRESETS.find((preset) => preset.id === presetAnchorId) ?? null)
    : null;
  const isPresetDirty = Boolean(
    anchoredPreset &&
      (formation !== anchoredPreset.formation || activePlayStyle !== anchoredPreset.playStyle),
  );
  const translatedPresetLibrary = useMemo<TacticsLibraryEntry[]>(
    () =>
      TACTICS_PRESETS.map((preset) => ({
        description: t(preset.descriptionKey),
        formation: preset.formation,
        id: `preset:${preset.id}`,
        name: t(`tactics.presetNames.${preset.id}`, preset.id),
        playStyle: preset.playStyle,
        sourcePresetName: null,
        type: "preset",
      })),
    [t],
  );
  const tacticLibrary = useMemo(
    () => [...customTactics, ...translatedPresetLibrary],
    [customTactics, translatedPresetLibrary],
  );
  const currentSetupFallbackTactic = useMemo<TacticsLibraryEntry>(
    () => ({
      description: t("tactics.customTacticDescription"),
      formation,
      id: "current:setup",
      name: t("tactics.customTactic"),
      playStyle: activePlayStyle,
      sourcePresetName: null,
      type: "custom",
    }),
    [activePlayStyle, formation, t],
  );
  const activeTactic =
    tacticLibrary.find((entry) => entry.id === activeTacticId) ??
    translatedPresetLibrary.find((entry) => entry.id === `preset:${matchedPreset?.id}`) ??
    currentSetupFallbackTactic;
  const isActiveCustomTactic = activeTactic?.type === "custom";
  const isActiveTacticDirty = Boolean(
    activeTactic &&
      (formation !== activeTactic.formation ||
        activePlayStyle !== activeTactic.playStyle ||
        (isActiveCustomTactic &&
          draftTacticName.trim().length > 0 &&
          draftTacticName.trim() !== activeTactic.name)),
  );
  // A saved tactic now holds its dials, eleven, roles and takers too (#1189), but this badge still
  // compares a custom tactic by formation, style and name only; the tactic bar slice makes it read
  // the whole tactic. A preset is also its blueprint, so tuned dials make it differ from the preset.
  const isCommandBarDirty = isActiveCustomTactic
    ? isActiveTacticDirty
    : isActiveTacticDirty || isPresetDirty || blueprintModified;

  useEffect(() => {
    if (!matchedPreset) {
      return;
    }

    if (matchedPreset.id !== presetAnchorId) {
      setPresetAnchorId(matchedPreset.id);
    }

    const nextActivePresetId = `preset:${matchedPreset.id}`;
    setActiveTacticId((current) =>
      tacticLibrary.find((entry) => entry.id === current)?.type === "custom" ||
      current === nextActivePresetId
        ? current
        : nextActivePresetId,
    );
  }, [matchedPreset, presetAnchorId, tacticLibrary]);

  useEffect(() => {
    if (!activeTactic) {
      return;
    }

    const nextName =
      activeTactic.type === "custom"
        ? activeTactic.name
        : t(`tactics.presetNames.${activeTactic.id.replace("preset:", "")}`);
    setDraftTacticName(nextName);
  }, [activeTactic?.id, activeTactic?.name, activeTactic?.type, t]);

  function createCustomTacticEntry(
    overrides: Partial<TacticsLibraryEntry> = {},
  ): TacticsLibraryEntry {
    const customCount = customTactics.length + 1;
    const sourcePresetName = matchedPreset
      ? t(`tactics.presetNames.${matchedPreset.id}`, matchedPreset.id)
      : null;

    return {
      description: overrides.description ?? t("tactics.customTacticDescription"),
      formation: overrides.formation ?? formation,
      id: overrides.id ?? `custom:${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name: overrides.name ?? t("tactics.customTacticNumber", { count: customCount }),
      playStyle: overrides.playStyle ?? activePlayStyle,
      sourcePresetName:
        overrides.sourcePresetName === undefined ? sourcePresetName : overrides.sourcePresetName,
      type: "custom",
    };
  }

  // A saved tactic is put back whole by the backend: its formation and style, and for one that
  // holds them its instructions, eleven, roles and takers (#1189).
  async function applySavedTactic(nextTactic: TacticsLibraryEntry): Promise<boolean> {
    // An answer that lands after the career changed (the game left, another loaded) is the old
    // career's game: it must not replace the one now open, as `changeSave`'s steps don't.
    const askedIn = useGameStore.getState().careerEpoch;
    try {
      const applied = await applyCustomTactic(nextTactic.id.trim(), careerToken);
      if (useGameStore.getState().careerEpoch !== askedIn) return false;
      onGameUpdate(applied);
      return true;
    } catch (error) {
      console.error("Failed to apply the saved tactic:", error);
      onWriteError(error);
      return false;
    }
  }

  async function applyTacticSelection(nextTactic: TacticsLibraryEntry): Promise<void> {
    const isPreset = nextTactic.type === "preset";

    // A preset is a whole setup: the backend sets its formation, play style and
    // phase blueprint together (#365). A saved tactic is applied whole too.
    const didApply = isPreset
      ? await onApplyPreset(nextTactic.formation, nextTactic.playStyle)
      : await applySavedTactic(nextTactic);
    if (!didApply) {
      return;
    }

    setActiveTacticId(nextTactic.id);
    setDraftTacticName(nextTactic.name);

    if (isPreset) {
      setPresetAnchorId(nextTactic.id.replace("preset:", ""));
    }
  }

  function handleCreateCustomTactic(): void {
    const nextTactic = createCustomTacticEntry();
    setCustomTactics((current) => [nextTactic, ...current]);
    persistTactic(nextTactic);
    setActiveTacticId(nextTactic.id);
    setDraftTacticName(nextTactic.name);
  }

  function handleDuplicateTactic(): void {
    const nextTactic = createCustomTacticEntry({
      description: activeTactic?.description,
      formation,
      name: t("tactics.copyOfTactic", {
        name: draftTacticName.trim() || activeTactic?.name || t("tactics.customTactic"),
      }),
      playStyle: activePlayStyle,
      sourcePresetName: activeTactic?.sourcePresetName ?? activeTactic?.name ?? null,
    });

    setCustomTactics((current) => [nextTactic, ...current]);
    persistTactic(nextTactic);
    setActiveTacticId(nextTactic.id);
    setDraftTacticName(nextTactic.name);
  }

  // Only a tactic the save holds can be deleted: not a preset, and not the unsaved setup that
  // stands in when nothing else matches.
  const canDeleteTactic =
    isActiveCustomTactic && customTactics.some((entry) => entry.id === activeTactic.id);

  // The team keeps the formation and play style it has. With the tactic gone from the library the
  // setup shown falls back to the preset it matches, or to the current setup, applying nothing.
  function handleDeleteTactic(): void {
    if (!canDeleteTactic) return;

    const { id } = activeTactic;
    setCustomTactics((current) => current.filter((entry) => entry.id !== id));
    changeSave(() => deleteCustomTactic(id.trim(), careerToken));
  }

  function handleSaveTactic(): Promise<boolean> {
    const nextName = draftTacticName.trim() || t("tactics.customTactic");

    if (
      isActiveCustomTactic &&
      activeTactic &&
      customTactics.some((e) => e.id === activeTactic.id)
    ) {
      const updated: TacticsLibraryEntry = {
        ...activeTactic,
        description: activeTactic.description,
        formation,
        name: nextName,
        playStyle: activePlayStyle,
      };
      setCustomTactics((current) =>
        current.map((entry) => (entry.id === activeTactic.id ? updated : entry)),
      );
      return persistTactic(updated);
    }

    const nextTactic = createCustomTacticEntry({
      description: activeTactic?.description,
      formation,
      name: nextName,
      playStyle: activePlayStyle,
      sourcePresetName: activeTactic?.name ?? null,
    });

    setCustomTactics((current) => [nextTactic, ...current]);
    const saved = persistTactic(nextTactic);
    setActiveTacticId(nextTactic.id);
    setDraftTacticName(nextTactic.name);
    return saved;
  }

  // What Reset goes back to: a preset applies as a whole (formation, style and phase blueprint, the
  // backend's own preset); a saved custom tactic goes back to the formation and style it was
  // saved with; an unsaved setup falls back to the preset it was started from, if any.
  const anchoredPresetEntry =
    translatedPresetLibrary.find((entry) => entry.id === `preset:${presetAnchorId}`) ?? null;
  const resetTarget = isActiveCustomTactic
    ? (customTactics.find((entry) => entry.id === activeTactic.id) ?? anchoredPresetEntry)
    : activeTactic;
  // Reset is offered exactly when the badge says something differs: a preset against its whole
  // definition (formation, style and the backend's verdict on the dials), a saved custom tactic
  // against what it was saved with.
  const canResetTactic =
    resetTarget !== null &&
    (resetTarget.type === "preset"
      ? isActiveTacticDirty || isPresetDirty || blueprintModified
      : isActiveTacticDirty);

  async function handleResetTactic(): Promise<void> {
    if (resetTarget) {
      await applyTacticSelection(resetTarget);
    }
  }

  return {
    activeTactic,
    tacticLibrary,
    isCommandBarDirty,
    canResetTactic,
    handleResetTactic,
    applyTacticSelection,
    handleCreateCustomTactic,
    handleDuplicateTactic,
    handleSaveTactic,
    canDeleteTactic,
    handleDeleteTactic,
  };
}
