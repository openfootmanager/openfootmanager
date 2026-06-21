import type { DragEvent, JSX } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  GameStateData,
  PlayerData,
  PlayerSelectionOptions,
  TeamMatchRolesData,
} from "../../store/gameStore";
import { useGameStore } from "../../store/gameStore";
import { useTranslation } from "react-i18next";
import { getSquad } from "../../services/squadService";
import {
  applyLineupDrop,
  applyLineupSwap,
  buildAssignBestFitSlot,
  buildAssignStartingXiSlot,
  buildActivePositionMap,
  buildDemoteFromStartingXi,
  buildPitchRows,
  buildPitchSlotRows,
  buildPromoteToStartingXi,
  type DragState,
  type PitchSlotRow,
  type SquadSection,
} from "../squad/SquadTab.helpers";
import TacticsFilters from "./TacticsFilters";
import {
  TACTICS_PRESETS,
  buildTacticsPitchSlots,
  buildTacticsRoster,
  buildFormationSlotOptions,
  countOutOfPositionPlayers,
  findTacticsPresetBySetup,
  filterAndSortTacticsPlayers,
  getSelectedAndComparePlayers,
  resolveStartingXiIds,
  type SortKey,
  type TacticsTableMode,
} from "./TacticsTab.helpers";
import TacticsPitch from "./TacticsPitch";
import TacticsPlayerFocusPanel from "./TacticsPlayerFocusPanel";
import TacticsPlayerTable from "./TacticsPlayerTable";
import TacticsRolesPanel from "./TacticsRolesPanel";
import {
  buildCustomTacticsStorageKey,
  loadCustomTactics,
  saveCustomTactics,
} from "./TacticsCustomTactics.helpers";
import {
  buildUpdatedMatchRolesForAssignment,
  resolveEffectiveMatchRoles,
} from "./TacticsRoles.helpers";
import TacticsCommandBar, {
  type TacticsLibraryEntry,
} from "./TacticsCommandBar";

interface TacticsTabProps {
  gameState: GameStateData | null;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onGameUpdate: (g: GameStateData) => void;
}

function isPlayerEligibleForLineup(
  player: PlayerData | null | undefined,
): boolean {
  return Boolean(player && !player.injury);
}

export default function TacticsTab({
  gameState,
  onSelectPlayer,
  onGameUpdate,
}: TacticsTabProps): JSX.Element {
  const { t } = useTranslation();
  const { sessionState } = useGameStore();
  const [fetchedSquad, setFetchedSquad] = useState<PlayerData[] | null>(null);
  const teamId = sessionState?.manager?.team_id ?? gameState?.manager?.team_id ?? null;
  const initialTeam = sessionState?.team ?? gameState?.teams?.find((t) => t.id === teamId) ?? null;
  const initialPreset = initialTeam
    ? findTacticsPresetBySetup(
        initialTeam.formation || "4-4-2",
        initialTeam.play_style || "Balanced",
      )
    : null;
  const [playerSearch, setPlayerSearch] = useState("");
  const [positionFilter, setPositionFilter] = useState("All");
  const [sortKey, setSortKey] = useState<SortKey>("pos");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
  const [dragState, setDragState] = useState<DragState | null>(null);
  const [hoveredSlot, setHoveredSlot] = useState<number | null>(null);
  const [pendingStartingXiIds, setPendingStartingXiIds] = useState<
    string[] | null
  >(null);
  const [presetAnchorId, setPresetAnchorId] = useState<string | null>(
    initialPreset?.id ?? null,
  );
  const [selectedPlayerId, setSelectedPlayerId] = useState<string | null>(null);
  const [selectedPlayerSection, setSelectedPlayerSection] =
    useState<SquadSection | null>(null);
  const [comparePlayerId, setComparePlayerId] = useState<string | null>(null);
  const [comparePlayerSection, setComparePlayerSection] =
    useState<SquadSection | null>(null);
  const [activeTab, setActiveTab] = useState<"lineup" | "roles">("lineup");
  const [tableMode, setTableMode] = useState<TacticsTableMode>("lineup");
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
  const dragStateRef = useRef<DragState | null>(null);
  const hoveredSlotRef = useRef<number | null>(null);
  const dragPreviewRef = useRef<HTMLDivElement | null>(null);
  const hydratedCustomTacticsScopeRef = useRef<string | null>(null);

  useEffect(() => {
    if (!teamId) return;
    void getSquad(teamId).then(setFetchedSquad).catch(() => {});
  }, [teamId]);

  const team = sessionState?.team ?? gameState?.teams?.find((t) => t.id === teamId) ?? null;
  const players = fetchedSquad ?? gameState?.players ?? [];
  const roster = team ? buildTacticsRoster(players, team.id) : [];
  const customTacticsStorageKey = gameState ? buildCustomTacticsStorageKey(gameState) : null;

  const formation = team?.formation || "4-4-2";
  const activePlayStyle = team?.play_style || "Balanced";
  const savedStartingXiKey = (team?.starting_xi_ids || []).join(",");
  const playersById = useMemo(
    () => new Map(roster.map((player) => [player.id, player])),
    [roster],
  );
  const available = roster.filter((player) => !player.injury);
  const pitchRows = useMemo(() => buildPitchRows(formation), [formation]);

  const startingXiIds = useMemo(
    () =>
      resolveStartingXiIds({
        availablePlayers: available,
        formation,
        pendingStartingXiIds,
        playersById,
        savedStartingXiIds: team?.starting_xi_ids || [],
      }),
    [
      available.map((player) => player.id).join(","),
      formation,
      (team?.starting_xi_ids || []).join(","),
      (pendingStartingXiIds || []).join(","),
      roster.map((player) => player.id).join(","),
    ],
  );

  const startingXI = useMemo(
    () =>
      startingXiIds
        .map((id) => playersById.get(id))
        .filter((player): player is PlayerData => player != null),
    [playersById, startingXiIds],
  );

  useEffect(() => {
    if (!pendingStartingXiIds) return;
    if (savedStartingXiKey === pendingStartingXiIds.join(",")) {
      setPendingStartingXiIds(null);
    }
  }, [pendingStartingXiIds, savedStartingXiKey]);

  const pitchSlotRows = useMemo<PitchSlotRow[]>(
    () => buildPitchSlotRows(pitchRows, startingXiIds, playersById),
    [pitchRows, playersById, startingXiIds],
  );
  const pitchSlots = useMemo(
    () => buildTacticsPitchSlots(pitchSlotRows),
    [pitchSlotRows],
  );
  const formationSlotOptions = useMemo(
    () => buildFormationSlotOptions(formation, t),
    [formation, t],
  );
  const xiSlotIndexByPlayerId = useMemo(
    () =>
      new Map(
        pitchSlots
          .filter((slot) => slot.player)
          .map((slot) => [slot.player!.id, slot.index] as const),
      ),
    [pitchSlots],
  );

  const xiIds = new Set(startingXiIds);
  const bench = roster.filter((player) => !xiIds.has(player.id));
  const xiActivePosition = useMemo(
    () => buildActivePositionMap(pitchSlotRows),
    [pitchSlotRows],
  );

  const { comparePlayer, selectedPlayer } = getSelectedAndComparePlayers(
    comparePlayerId,
    playersById,
    selectedPlayerId,
  );

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

  const canConfirmSwap = useMemo(() => {
    if (
      !selectedPlayerId ||
      !selectedPlayerSection ||
      !comparePlayerId ||
      !comparePlayerSection
    ) {
      return false;
    }

    if (
      (selectedPlayerSection === "bench" &&
        !isPlayerEligibleForLineup(
          selectedPlayerId ? playersById.get(selectedPlayerId) : null,
        )) ||
      (comparePlayerSection === "bench" &&
        !isPlayerEligibleForLineup(
          comparePlayerId ? playersById.get(comparePlayerId) : null,
        ))
    ) {
      return false;
    }

    const nextXiIds = applyLineupSwap(
      startingXiIds,
      { id: selectedPlayerId, from: selectedPlayerSection },
      comparePlayerId,
      comparePlayerSection,
    );

    return !!nextXiIds && nextXiIds.join(",") !== startingXiIds.join(",");
  }, [
    comparePlayerId,
    comparePlayerSection,
    playersById,
    selectedPlayerId,
    selectedPlayerSection,
    startingXiIds,
  ]);

  function toggleSort(key: SortKey): void {
    if (sortKey === key) {
      setSortDir((current) => (current === "asc" ? "desc" : "asc"));
      return;
    }

    setSortKey(key);
    setSortDir(key === "ovr" ? "desc" : "asc");
  }

  const filteredStartingXI = useMemo(
    () =>
      filterAndSortTacticsPlayers(
        startingXI,
        {
          playerSearch,
          positionFilter,
          section: "xi",
          xiActivePosition,
        },
        {
          section: "xi",
          sortDir,
          sortKey,
          xiActivePosition,
        },
      ),
    [
      startingXI,
      playerSearch,
      positionFilter,
      sortKey,
      sortDir,
      xiActivePosition,
    ],
  );
  const filteredBench = useMemo(
    () =>
      filterAndSortTacticsPlayers(
        bench,
        {
          playerSearch,
          positionFilter,
          section: "bench",
          xiActivePosition,
        },
        {
          section: "bench",
          sortDir,
          sortKey,
          xiActivePosition,
        },
      ),
    [bench, playerSearch, positionFilter, sortKey, sortDir, xiActivePosition],
  );

  const outOfPositionCount = countOutOfPositionPlayers(
    startingXI,
    xiActivePosition,
  );
  const effectiveMatchRoles = useMemo(
    () => resolveEffectiveMatchRoles(startingXI, team?.match_roles),
    [team?.match_roles, startingXI],
  );
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
      const didUpdateFormation = await handleFormationChange(nextTactic.formation);
      if (!didUpdateFormation) {
        return;
      }
    }

    if (activePlayStyle !== nextTactic.playStyle) {
      const didUpdatePlayStyle = await handlePlayStyleChange(nextTactic.playStyle);
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

  async function persistStartingXI(playerIds: string[]): Promise<void> {
    setPendingStartingXiIds(playerIds);
    try {
      const updated = await invoke<GameStateData>("set_starting_xi", {
        playerIds,
      });
      onGameUpdate(updated);
    } catch (error) {
      setPendingStartingXiIds(null);
      console.error("Failed to set starting XI:", error);
    }
  }

  async function handleFormationChange(nextFormation: string): Promise<boolean> {
    try {
      const updated = await invoke<GameStateData>("set_formation", {
        formation: nextFormation,
      });
      onGameUpdate(updated);
      return true;
    } catch (error) {
      console.error("Failed to set formation:", error);
      return false;
    }
  }

  async function handlePlayStyleChange(playStyle: string): Promise<boolean> {
    try {
      const updated = await invoke<GameStateData>("set_play_style", {
        playStyle,
      });
      onGameUpdate(updated);
      return true;
    } catch (error) {
      console.error("Failed to set play style:", error);
      return false;
    }
  }

  async function handleAssignSlot(
    playerId: string,
    targetSlotIndex: number,
  ): Promise<void> {
    const nextXiIds = buildAssignStartingXiSlot(
      startingXiIds,
      playerId,
      targetSlotIndex,
    );

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    await persistStartingXI(nextXiIds);
    clearLineupSelection();
  }

  async function handleAssignBestFit(playerId: string): Promise<void> {
    const nextXiIds = buildAssignBestFitSlot(
      startingXiIds,
      playersById,
      formation,
      playerId,
    );

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    await persistStartingXI(nextXiIds);
    clearLineupSelection();
  }

  async function handlePromoteBenchPlayer(playerId: string): Promise<void> {
    if (!isPlayerEligibleForLineup(playersById.get(playerId))) {
      return;
    }

    const nextXiIds = buildPromoteToStartingXi(
      startingXiIds,
      playersById,
      formation,
      playerId,
    );

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    await persistStartingXI(nextXiIds);
    clearLineupSelection();
  }

  async function handleDemoteStarter(playerId: string): Promise<void> {
    const nextXiIds = buildDemoteFromStartingXi(
      startingXiIds,
      available,
      formation,
      playerId,
    );

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    await persistStartingXI(nextXiIds);
    clearLineupSelection();
  }

  function clearLineupSelection(): void {
    setSelectedPlayerId(null);
    setSelectedPlayerSection(null);
    setComparePlayerId(null);
    setComparePlayerSection(null);
  }

  function setHoveredSlotValue(slotIndex: number | null): void {
    if (hoveredSlotRef.current === slotIndex) {
      return;
    }

    hoveredSlotRef.current = slotIndex;
    setHoveredSlot(slotIndex);
  }

  function resetDragState(): void {
    dragStateRef.current = null;
    setDragState(null);
    setHoveredSlotValue(null);
  }

  function applyLightweightDragPreview(event: DragEvent<HTMLElement>): void {
    if (!dragPreviewRef.current) {
      return;
    }

    if (typeof event.dataTransfer.setDragImage !== "function") {
      return;
    }

    event.dataTransfer.setDragImage(dragPreviewRef.current, 16, 16);
  }

  function handleDragStart(
    event: DragEvent<HTMLElement>,
    playerId: string,
    from: SquadSection,
    slotIndex: number | null = null,
  ): void {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", playerId);
    applyLightweightDragPreview(event);
    const nextDragState = { playerId, from, slotIndex };
    dragStateRef.current = nextDragState;
    setDragState(nextDragState);
  }

  function handleSlotDragOver(
    event: DragEvent<HTMLElement>,
    slotIndex: number,
  ): void {
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    setHoveredSlotValue(slotIndex);
  }

  function handleSlotDragLeave(slotIndex: number): void {
    if (hoveredSlotRef.current !== slotIndex) {
      return;
    }

    setHoveredSlotValue(null);
  }

  async function handleSlotDrop(
    event: DragEvent<HTMLElement>,
    slotIndex: number,
  ): Promise<void> {
    event.preventDefault();
    const draggedPlayerId = event.dataTransfer.getData("text/plain");
    const currentDragState = dragStateRef.current ?? dragState;
    const resolvedDragState =
      currentDragState ??
      (draggedPlayerId
        ? {
            playerId: draggedPlayerId,
            from: xiIds.has(draggedPlayerId) ? "xi" : "bench",
            slotIndex: xiIds.has(draggedPlayerId)
              ? startingXiIds.indexOf(draggedPlayerId)
              : null,
          }
        : null);

    if (!resolvedDragState) return;

    if (
      resolvedDragState.from === "bench" &&
      !isPlayerEligibleForLineup(playersById.get(resolvedDragState.playerId))
    ) {
      resetDragState();
      return;
    }

    const nextXiIds = applyLineupDrop(
      startingXiIds,
      resolvedDragState,
      slotIndex,
    );
    if (nextXiIds.join(",") === startingXiIds.join(",")) {
      resetDragState();
      return;
    }

    await persistStartingXI(nextXiIds);
    clearLineupSelection();
    resetDragState();
  }

  async function handleLineupPlayerClick(
    playerId: string,
    section: SquadSection,
  ): Promise<void> {
    if (!selectedPlayerId || !selectedPlayerSection) {
      setSelectedPlayerId(playerId);
      setSelectedPlayerSection(section);
      return;
    }

    if (selectedPlayerId === playerId && selectedPlayerSection === section) {
      if (comparePlayerId && comparePlayerSection) {
        setSelectedPlayerId(comparePlayerId);
        setSelectedPlayerSection(comparePlayerSection);
        setComparePlayerId(null);
        setComparePlayerSection(null);
        return;
      }

      clearLineupSelection();
      return;
    }

    if (comparePlayerId === playerId && comparePlayerSection === section) {
      setComparePlayerId(null);
      setComparePlayerSection(null);
      return;
    }

    setComparePlayerId(playerId);
    setComparePlayerSection(section);
  }

  async function handleConfirmSwap(): Promise<void> {
    if (
      !selectedPlayerId ||
      !selectedPlayerSection ||
      !comparePlayerId ||
      !comparePlayerSection
    ) {
      return;
    }

    if (
      (selectedPlayerSection === "bench" &&
        !isPlayerEligibleForLineup(
          selectedPlayerId ? playersById.get(selectedPlayerId) : null,
        )) ||
      (comparePlayerSection === "bench" &&
        !isPlayerEligibleForLineup(
          comparePlayerId ? playersById.get(comparePlayerId) : null,
        ))
    ) {
      return;
    }

    const nextXiIds = applyLineupSwap(
      startingXiIds,
      { id: selectedPlayerId, from: selectedPlayerSection },
      comparePlayerId,
      comparePlayerSection,
    );

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    await persistStartingXI(nextXiIds);
    clearLineupSelection();
  }

  function handleClearFilters(): void {
    setPlayerSearch("");
    setPositionFilter("All");
  }

  async function persistMatchRoles(
    nextRoles: TeamMatchRolesData,
  ): Promise<void> {
    try {
      const updated = await invoke<GameStateData>("set_team_match_roles", {
        matchRoles: nextRoles,
      });
      onGameUpdate(updated);
    } catch (error) {
      console.error("Failed to set team match roles:", error);
    }
  }

  async function handleAssignMatchRole(
    role: keyof TeamMatchRolesData,
    playerId: string,
  ): Promise<void> {
    await persistMatchRoles(
      buildUpdatedMatchRolesForAssignment(
        effectiveMatchRoles,
        startingXI,
        role,
        playerId,
      ),
    );
  }

  const lineupWorkspace = (
    <div className="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,0.9fr)_minmax(38rem,1.1fr)] xl:items-start 2xl:gap-8">
      <div className="flex flex-col gap-5">
        <TacticsFilters
          onClear={handleClearFilters}
          onPlayerSearchChange={setPlayerSearch}
          onPositionFilterChange={setPositionFilter}
          playerSearch={playerSearch}
          positionFilter={positionFilter}
        />

        <div className="flex items-center justify-between gap-3 rounded-xl border border-gray-200 bg-gray-50/80 px-3 py-2 dark:border-navy-600 dark:bg-navy-800/70">
          <div>
            <div className="text-[11px] font-heading font-bold uppercase tracking-[0.22em] text-gray-500 dark:text-gray-400">
              {t("tactics.tableView")}
            </div>
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
              {tableMode === "lineup"
                ? t("tactics.tableModeHints.lineup")
                : t("tactics.tableModeHints.roles")}
            </p>
          </div>
          <div className="flex gap-1 rounded-lg bg-white p-1 shadow-sm dark:bg-navy-700">
            {([
              ["lineup", t("tactics.tableModes.lineup")],
              ["roles", t("tactics.tableModes.roles")],
            ] as const).map(([mode, label]) => (
              <button
                key={mode}
                type="button"
                aria-pressed={tableMode === mode}
                onClick={() => setTableMode(mode)}
                className={`rounded-md px-3 py-1.5 text-[11px] font-heading font-bold uppercase tracking-[0.2em] transition-colors ${
                  tableMode === mode
                    ? "bg-primary-500 text-white"
                    : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                }`}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        <TacticsPlayerTable
          activePlayStyle={activePlayStyle}
          comparePlayerId={comparePlayerId}
          emptyMessage={t("squad.noLineupMatches")}
          formation={formation}
          highlightedPlayerId={selectedPlayerId}
          matchRoles={effectiveMatchRoles}
          onAssignBestFit={handleAssignBestFit}
          onAssignMatchRole={(role, playerId) => {
            void handleAssignMatchRole(role, playerId);
          }}
          onAssignSlot={(playerId, targetSlotIndex) => {
            void handleAssignSlot(playerId, targetSlotIndex);
          }}
          onClearTacticsSelection={clearLineupSelection}
          onDemoteStarter={(playerId) => {
            void handleDemoteStarter(playerId);
          }}
          onSelectPlayer={onSelectPlayer}
          onTacticalSelect={(playerId, section) => {
            void handleLineupPlayerClick(playerId, section);
          }}
          players={filteredStartingXI}
          section="xi"
          sortDir={sortDir}
          sortKey={sortKey}
          slotOptions={formationSlotOptions}
          tableMode={tableMode}
          title={t("preMatch.startingXI")}
          toggleSort={toggleSort}
          totalCount={startingXI.length}
          xiActivePosition={xiActivePosition}
          xiSlotIndexByPlayerId={xiSlotIndexByPlayerId}
        />

        <TacticsPlayerTable
          activePlayStyle={activePlayStyle}
          comparePlayerId={comparePlayerId}
          emptyMessage={t("squad.noBenchMatches")}
          formation={formation}
          highlightedPlayerId={selectedPlayerId}
          matchRoles={effectiveMatchRoles}
          onAssignMatchRole={(role, playerId) => {
            void handleAssignMatchRole(role, playerId);
          }}
          onSelectPlayer={onSelectPlayer}
          onPromoteBench={(playerId) => {
            void handlePromoteBenchPlayer(playerId);
          }}
          onTacticalSelect={(playerId, section) => {
            void handleLineupPlayerClick(playerId, section);
          }}
          players={filteredBench}
          section="bench"
          sortDir={sortDir}
          sortKey={sortKey}
          slotOptions={formationSlotOptions}
          tableMode={tableMode}
          title={t("preMatch.substitutes")}
          toggleSort={toggleSort}
          totalCount={bench.length}
          xiActivePosition={xiActivePosition}
          xiSlotIndexByPlayerId={xiSlotIndexByPlayerId}
        />
      </div>

      <div className="flex flex-col gap-5">
        <TacticsPitch
          benchPlayers={bench}
          dragState={dragState}
          formation={formation}
          comparePlayerId={comparePlayerId}
          hoveredSlot={hoveredSlot}
          matchRoles={effectiveMatchRoles}
          onAssignBestFit={(playerId) => {
            void handleAssignBestFit(playerId);
          }}
          onAssignMatchRole={(role, playerId) => {
            void handleAssignMatchRole(role, playerId);
          }}
          onClearSelection={clearLineupSelection}
          onDemoteStarter={(playerId) => {
            void handleDemoteStarter(playerId);
          }}
          onDragEnd={resetDragState}
          onDragStart={handleDragStart}
          onLineupPlayerClick={(playerId, section) => {
            void handleLineupPlayerClick(playerId, section);
          }}
          onOpenPlayerProfile={(playerId) => {
            onSelectPlayer(playerId);
          }}
          onPromoteBench={(playerId) => {
            void handlePromoteBenchPlayer(playerId);
          }}
          onSlotDragLeave={handleSlotDragLeave}
          onSlotDragOver={handleSlotDragOver}
          onSlotDrop={(event, slotIndex) => {
            void handleSlotDrop(event, slotIndex);
          }}
          outOfPositionCount={outOfPositionCount}
          pitchSlots={pitchSlots}
          selectedPlayer={selectedPlayer}
          selectedPlayerId={selectedPlayerId}
        />
        <TacticsPlayerFocusPanel
          canConfirmSwap={canConfirmSwap}
          onConfirmSwap={() => {
            void handleConfirmSwap();
          }}
          selectedPlayer={selectedPlayer}
          comparePlayer={comparePlayer}
        />
      </div>
    </div>
  );

  if (!team) {
    return (
      <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>
    );
  }

  return (
    <div className="mx-auto flex w-full max-w-[118rem] flex-col gap-5">
      <div
        ref={dragPreviewRef}
        aria-hidden="true"
        className="pointer-events-none fixed -left-20 top-0 h-8 w-8 rounded-full border border-white/15 bg-navy-900/90 shadow-lg"
      />
      <TacticsCommandBar
        activeTactic={activeTactic}
        activePlayStyle={activePlayStyle}
        formation={formation}
        isDirty={isCommandBarDirty}
        onCreateNew={handleCreateCustomTactic}
        onDuplicate={handleDuplicateTactic}
        onFormationChange={(nextFormation) => {
          void handleFormationChange(nextFormation);
        }}
        onPlayStyleChange={(playStyle) => {
          void handlePlayStyleChange(playStyle);
        }}
        onSave={handleSaveTactic}
        onSelectTactic={(id) => {
          const nextTactic = tacticLibrary.find((entry) => entry.id === id);
          if (!nextTactic) {
            return;
          }

          void applyTacticSelection(nextTactic);
        }}
        tacticLibrary={tacticLibrary}
      />
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex gap-1 self-start rounded-lg bg-gray-100 p-1 dark:bg-navy-800">
          <button
            type="button"
            onClick={() => setActiveTab("lineup")}
            aria-pressed={activeTab === "lineup"}
            className={`rounded-md px-4 py-2 text-xs font-heading font-bold uppercase tracking-wider transition-colors ${
              activeTab === "lineup"
                ? "bg-white text-gray-900 shadow-sm dark:bg-navy-700 dark:text-white"
                : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
            }`}
          >
            {t("tactics.lineupTab")}
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("roles")}
            aria-pressed={activeTab === "roles"}
            className={`rounded-md px-4 py-2 text-xs font-heading font-bold uppercase tracking-wider transition-colors ${
              activeTab === "roles"
                ? "bg-white text-gray-900 shadow-sm dark:bg-navy-700 dark:text-white"
                : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
            }`}
          >
            {t("tactics.rolesTab")}
          </button>
        </div>
      </div>

      {activeTab === "lineup" ? (
        lineupWorkspace
      ) : (
        <TacticsRolesPanel
          allSquad={roster}
          matchRoles={team.match_roles}
          onGameUpdate={onGameUpdate}
          startingPlayers={startingXI}
        />
      )}
    </div>
  );
}
