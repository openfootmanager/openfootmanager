import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import type { GameStateData } from "../../store/gameStore";
import {
  TACTICS_PRESETS,
  findTacticsPresetBySetup,
  type TacticsPresetDefinition,
} from "./TacticsTab.helpers";
import {
  buildCustomTacticsStorageKey,
  loadCustomTactics,
  saveCustomTactics,
} from "./TacticsCustomTactics.helpers";
import type { TacticsLibraryEntry } from "./TacticsCommandBar";

interface UseTacticsLibraryArgs {
  gameState: GameStateData | null;
  formation: string;
  activePlayStyle: string;
  initialPreset: TacticsPresetDefinition | null;
  onFormationChange: (nextFormation: string) => Promise<boolean>;
  onPlayStyleChange: (playStyle: string) => Promise<boolean>;
}

export function useTacticsLibrary({
  gameState,
  formation,
  activePlayStyle,
  initialPreset,
  onFormationChange,
  onPlayStyleChange,
}: UseTacticsLibraryArgs) {
  const { t } = useTranslation();
  const [customTactics, setCustomTactics] = useState<TacticsLibraryEntry[]>(() =>
    gameState ? loadCustomTactics(gameState) : [],
  );
  const [activeTacticId, setActiveTacticId] = useState<string | null>(
    initialPreset ? `preset:${initialPreset.id}` : null,
  );
  const [draftTacticName, setDraftTacticName] = useState(
    initialPreset?.id
      ? t(`tactics.presetNames.${initialPreset.id}`, initialPreset.id)
      : t("tactics.customTactic"),
  );
  const [presetAnchorId, setPresetAnchorId] = useState<string | null>(
    initialPreset?.id ?? null,
  );
  const hydratedCustomTacticsScopeRef = useRef<string | null>(null);

  const customTacticsStorageKey = gameState
    ? buildCustomTacticsStorageKey(gameState)
    : null;

  useEffect(() => {
    if (!gameState || !customTacticsStorageKey) return;
    hydratedCustomTacticsScopeRef.current = null;
    setCustomTactics(loadCustomTactics(gameState));
  }, [customTacticsStorageKey, gameState]);

  useEffect(() => {
    if (!gameState || !customTacticsStorageKey) return;
    if (hydratedCustomTacticsScopeRef.current !== customTacticsStorageKey) {
      hydratedCustomTacticsScopeRef.current = customTacticsStorageKey;
      return;
    }

    saveCustomTactics(gameState, customTactics);
  }, [customTactics, customTacticsStorageKey, gameState]);

  const matchedPreset = findTacticsPresetBySetup(formation, activePlayStyle);
  const anchoredPreset = presetAnchorId
    ? TACTICS_PRESETS.find((preset) => preset.id === presetAnchorId) ?? null
    : null;
  const isPresetDirty = Boolean(
    anchoredPreset &&
      (formation !== anchoredPreset.formation ||
        activePlayStyle !== anchoredPreset.playStyle),
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
  const isCommandBarDirty = isActiveCustomTactic
    ? isActiveTacticDirty
    : isActiveTacticDirty || isPresetDirty;

  useEffect(() => {
    if (!matchedPreset) {
      return;
    }

    if (matchedPreset.id !== presetAnchorId) {
      setPresetAnchorId(matchedPreset.id);
    }

    const nextActivePresetId = `preset:${matchedPreset.id}`;
    setActiveTacticId((current) =>
      current?.startsWith("custom:") || current === nextActivePresetId
        ? current
        : nextActivePresetId,
    );
  }, [matchedPreset, presetAnchorId]);

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
    const sourcePresetName =
      matchedPreset
        ? t(`tactics.presetNames.${matchedPreset.id}`, matchedPreset.id)
        : null;

    return {
      description:
        overrides.description ??
        t("tactics.customTacticDescription"),
      formation: overrides.formation ?? formation,
      id:
        overrides.id ??
        `custom:${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name:
        overrides.name ??
        t("tactics.customTacticNumber", { count: customCount }),
      playStyle: overrides.playStyle ?? activePlayStyle,
      sourcePresetName:
        overrides.sourcePresetName === undefined
          ? sourcePresetName
          : overrides.sourcePresetName,
      type: "custom",
    };
  }

  async function applyTacticSelection(nextTactic: TacticsLibraryEntry): Promise<void> {
    if (formation !== nextTactic.formation) {
      const didUpdateFormation = await onFormationChange(nextTactic.formation);
      if (!didUpdateFormation) {
        return;
      }
    }

    if (activePlayStyle !== nextTactic.playStyle) {
      const didUpdatePlayStyle = await onPlayStyleChange(nextTactic.playStyle);
      if (!didUpdatePlayStyle) {
        return;
      }
    }

    setActiveTacticId(nextTactic.id);
    setDraftTacticName(nextTactic.name);

    if (nextTactic.id.startsWith("preset:")) {
      setPresetAnchorId(nextTactic.id.replace("preset:", ""));
    }
  }

  function handleCreateCustomTactic(): void {
    const nextTactic = createCustomTacticEntry();
    setCustomTactics((current) => [nextTactic, ...current]);
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
    setActiveTacticId(nextTactic.id);
    setDraftTacticName(nextTactic.name);
  }

  function handleSaveTactic(): void {
    const nextName = draftTacticName.trim() || t("tactics.customTactic");

    if (isActiveCustomTactic && activeTactic && customTactics.some((e) => e.id === activeTactic.id)) {
      setCustomTactics((current) =>
        current.map((entry) =>
          entry.id === activeTactic.id
            ? {
                ...entry,
                description: activeTactic.description,
                formation,
                name: nextName,
                playStyle: activePlayStyle,
              }
            : entry,
        ),
      );
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
    setActiveTacticId(nextTactic.id);
    setDraftTacticName(nextTactic.name);
  }

  return {
    activeTactic,
    tacticLibrary,
    isCommandBarDirty,
    applyTacticSelection,
    handleCreateCustomTactic,
    handleDuplicateTactic,
    handleSaveTactic,
  };
}
