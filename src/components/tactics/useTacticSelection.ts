import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type { TeamData } from "../../store/gameStore";
import {
  TACTICS_PRESETS,
  findTacticsPresetBySetup,
  type TacticsPresetDefinition,
} from "./TacticsTab.helpers";
import type { TacticsLibraryEntry } from "./TacticPicker";
import { useActiveSavedTacticDiffers } from "./useSavedTacticDiffers";

interface UseTacticSelectionArgs {
  customTactics: TacticsLibraryEntry[];
  formation: string;
  activePlayStyle: string;
  initialPreset: TacticsPresetDefinition | null;
  userTeam: TeamData | undefined;
  /** Whether the dials differ from the style's blueprint (the backend's rule). */
  blueprintModified: boolean;
}

/**
 * Which tactic the screen shows as the active one (a preset, a saved tactic, or the unsaved setup that
 * stands in), the name being drafted, the preset the setup started from, the library of presets and
 * saved tactics the picker lists, and whether the active one has been changed.
 */
export function useTacticSelection({
  customTactics,
  formation,
  activePlayStyle,
  initialPreset,
  userTeam,
  blueprintModified,
}: UseTacticSelectionArgs) {
  const { t } = useTranslation();
  const [activeTacticId, setActiveTacticId] = useState<string | null>(
    initialPreset ? `preset:${initialPreset.id}` : null,
  );
  const [draftTacticName, setDraftTacticName] = useState(
    initialPreset?.id
      ? t(`tactics.presetNames.${initialPreset.id}`, initialPreset.id)
      : t("tactics.customTactic"),
  );
  const [presetAnchorId, setPresetAnchorId] = useState<string | null>(initialPreset?.id ?? null);
  // A saved tactic whose delete the save has not answered yet. It is gone from the picker at once, but
  // it stays the active one until the save has accepted (the command bar keeps showing it, and the
  // selection does not fall back to a preset for it): a refused delete puts the tactic back.
  const [pendingDelete, setPendingDelete] = useState<TacticsLibraryEntry | null>(null);

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
    (pendingDelete && pendingDelete.id === activeTacticId ? pendingDelete : undefined) ??
    translatedPresetLibrary.find((entry) => entry.id === `preset:${matchedPreset?.id}`) ??
    currentSetupFallbackTactic;
  const isActiveCustomTactic = activeTactic?.type === "custom";

  // A saved tactic holds the whole tactic (#1189): whether the team still equals it is the backend's
  // rule, asked by `useActiveSavedTacticDiffers`. False for a preset or an unsaved setup.
  const savedTacticDiffers = useActiveSavedTacticDiffers(activeTactic, customTactics, userTeam);

  const isActiveTacticDirty = Boolean(
    activeTactic &&
      (formation !== activeTactic.formation ||
        activePlayStyle !== activeTactic.playStyle ||
        (isActiveCustomTactic &&
          draftTacticName.trim().length > 0 &&
          draftTacticName.trim() !== activeTactic.name) ||
        savedTacticDiffers),
  );
  // A preset is also its blueprint, so tuned dials make it differ from the preset.
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
      current === nextActivePresetId ||
      (pendingDelete !== null && current === pendingDelete.id)
        ? current
        : nextActivePresetId,
    );
  }, [matchedPreset, presetAnchorId, tacticLibrary, pendingDelete]);

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

  return {
    createCustomTacticEntry,
    translatedPresetLibrary,
    tacticLibrary,
    activeTactic,
    isActiveCustomTactic,
    isActiveTacticDirty,
    isPresetDirty,
    isCommandBarDirty,
    draftTacticName,
    setActiveTacticId,
    setPendingDelete,
    setDraftTacticName,
    presetAnchorId,
    setPresetAnchorId,
  };
}
