import { useTranslation } from "react-i18next";

import type { GameStateData } from "../../store/gameStore";
import { deleteCustomTactic, renameCustomTactic } from "../../services/customTacticsService";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import type { TacticsPresetDefinition } from "./TacticsTab.helpers";
import type { TacticsLibraryEntry } from "./TacticPicker";
import { useApplyTactic } from "./useApplyTactic";
import { useBlueprintModified } from "./useBlueprintModified";
import { useCustomTacticStore } from "./useCustomTacticStore";
import { useTacticSelection } from "./useTacticSelection";

interface UseTacticsLibraryArgs {
  gameState: GameStateData | null;
  formation: string;
  activePlayStyle: string;
  initialPreset: TacticsPresetDefinition | null;
  onApplyPreset: (formation: string, playStyle: string) => Promise<boolean>;
  /** A preset's formation and play style only: the manager's own instructions stay as they are. */
  onKeepInstructions: (formation: string, playStyle: string) => Promise<boolean>;
  onGameUpdate: (game: GameStateData) => void;
  /** A write the backend refused: the screen tells the manager it did not save. */
  onWriteError: (error: unknown) => void;
  /** A custom-tactic save or delete the backend accepted: the screen drops an earlier alert. */
  onWriteSaved: () => void;
}

/**
 * The Tactics screen's tactic library: the presets and the career's saved tactics, which one is active
 * and whether it has been changed, and what the manager can do with it (apply, make, duplicate, save,
 * rename, delete, reset). The work is split by responsibility: the saved tactics and their writes
 * (`useCustomTacticStore`), the dials' verdict (`useBlueprintModified`), the selection and its dirty
 * state (`useTacticSelection`), and applying a tactic (`useApplyTactic`). The hooks are called in the
 * order their effects must run in: the verdict, the first load, then the selection's.
 */
export function useTacticsLibrary({
  gameState,
  formation,
  activePlayStyle,
  initialPreset,
  onApplyPreset,
  onKeepInstructions,
  onGameUpdate,
  onWriteError,
  onWriteSaved,
}: UseTacticsLibraryArgs) {
  const { t } = useTranslation();
  const userTeam = gameState?.teams.find((team) => team.id === gameState.manager.team_id);
  const blueprintModified = useBlueprintModified(activePlayStyle, userTeam);
  const {
    customTactics,
    setCustomTactics,
    latestCustomTactics,
    libraryLoaded,
    changeSave,
    persistTactic,
    careerToken,
  } = useCustomTacticStore({
    managerId: gameState?.manager.id ?? null,
    careerStartDate: gameState?.clock.start_date ?? null,
    careerToken: gameState?.career_token ?? 0,
    onWriteError,
    onWriteSaved,
  });
  const {
    translatedPresetLibrary,
    tacticLibrary,
    activeTactic,
    isActiveCustomTactic,
    isActiveTacticDirty,
    isPresetDirty,
    isCommandBarDirty,
    draftTacticName,
    setActiveTacticId,
    setDraftTacticName,
    presetAnchorId,
    setPresetAnchorId,
    createCustomTacticEntry,
  } = useTacticSelection({
    customTactics,
    formation,
    activePlayStyle,
    initialPreset,
    userTeam,
    blueprintModified,
  });
  const applyTacticSelection = useApplyTactic({
    onApplyPreset,
    onKeepInstructions,
    onGameUpdate,
    onWriteError,
    careerToken,
    latestCustomTactics,
    setActiveTacticId,
    setDraftTacticName,
    setPresetAnchorId,
  });

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
  // The same tactics can be renamed: a preset's name is the game's, and the unsaved setup has none.
  const canRenameTactic = canDeleteTactic;

  // The team keeps the formation and play style it has. With the tactic gone from the library the
  // setup shown falls back to the preset it matches, or to the current setup, applying nothing.
  function handleDeleteTactic(): void {
    if (!canDeleteTactic) return;

    const { id } = activeTactic;
    setCustomTactics((current) => current.filter((entry) => entry.id !== id));
    changeSave(() => deleteCustomTactic(id.trim(), careerToken), undefined, { id });
  }

  // Renames the saved tactic `id` (the one the dialog was opened for, which may no longer be the
  // active one) through the same queue as every other library write. Unlike a delete the screen waits
  // for the save's answer: it resolves null once the save accepted the name (or the career is no
  // longer the open one), or the reason it refused (an empty name, one too long, a tactic that is
  // gone), in the player's language, for the dialog that asked to show.
  async function handleRenameTactic(id: string, name: string): Promise<string | null> {
    let reason: string | null = null;
    await changeSave(
      () => renameCustomTactic(id.trim(), name, careerToken),
      (error) => {
        reason = resolveTranslatedErrorMessage(error);
      },
    );
    return reason;
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
    libraryLoaded,
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
    canRenameTactic,
    handleRenameTactic,
  };
}
