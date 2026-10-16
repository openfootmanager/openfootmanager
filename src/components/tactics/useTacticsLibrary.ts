import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type { GameStateData } from "../../store/gameStore";
import { listCustomTactics, saveCustomTactic } from "../../services/customTacticsService";
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
import type { TacticsLibraryEntry } from "./TacticsCommandBar";

interface UseTacticsLibraryArgs {
  gameState: GameStateData | null;
  formation: string;
  activePlayStyle: string;
  initialPreset: TacticsPresetDefinition | null;
  onFormationChange: (nextFormation: string) => Promise<boolean>;
  onPlayStyleChange: (playStyle: string) => Promise<boolean>;
  onApplyPreset: (formation: string, playStyle: string) => Promise<boolean>;
}

export function useTacticsLibrary({
  gameState,
  formation,
  activePlayStyle,
  initialPreset,
  onFormationChange,
  onPlayStyleChange,
  onApplyPreset,
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

  useEffect(() => {
    if (!managerId || !careerStartDate) return;

    let cancelled = false;
    listCustomTactics()
      .then((saved) => importLegacyTactics(managerId, careerStartDate, saved, saveCustomTactic))
      .then((tactics) => {
        if (!cancelled) setCustomTactics(toLibraryEntries(tactics));
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [managerId, careerStartDate]);

  // The screen answers at once; the save's own list then replaces it, or, if the save refused the
  // tactic (a shape the game cannot lay out), puts the library back as the save has it.
  function persistTactic(tactic: TacticsLibraryEntry): void {
    saveCustomTactic(toCustomTactic(tactic))
      .then((saved) => setCustomTactics(toLibraryEntries(saved)))
      .catch(() =>
        listCustomTactics()
          .then((saved) => setCustomTactics(toLibraryEntries(saved)))
          .catch(() => {}),
      );
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
  // A custom tactic saves formation, style and name only, so its dials are not part of what could
  // be "unsaved"; a preset is also its blueprint, so tuned dials make it differ from the preset.
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

  async function applyFormationAndStyle(nextTactic: TacticsLibraryEntry): Promise<boolean> {
    if (formation !== nextTactic.formation && !(await onFormationChange(nextTactic.formation))) {
      return false;
    }

    return activePlayStyle === nextTactic.playStyle || onPlayStyleChange(nextTactic.playStyle);
  }

  async function applyTacticSelection(nextTactic: TacticsLibraryEntry): Promise<void> {
    const isPreset = nextTactic.type === "preset";

    // A preset is a whole setup: the backend sets its formation, play style and
    // phase blueprint together (#365). Custom tactics only carry a formation and
    // style, so they keep the separate calls and leave the manager's dials alone.
    const didApply = isPreset
      ? await onApplyPreset(nextTactic.formation, nextTactic.playStyle)
      : await applyFormationAndStyle(nextTactic);
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

  function handleSaveTactic(): void {
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
      persistTactic(updated);
      return;
    }

    const nextTactic = createCustomTacticEntry({
      description: activeTactic?.description,
      formation,
      name: nextName,
      playStyle: activePlayStyle,
      sourcePresetName: activeTactic?.name ?? null,
    });

    setCustomTactics((current) => [nextTactic, ...current]);
    persistTactic(nextTactic);
    setActiveTacticId(nextTactic.id);
    setDraftTacticName(nextTactic.name);
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
  };
}
