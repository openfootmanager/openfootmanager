import type { JSX } from "react";
import { useEffect, useMemo, useState } from "react";
import type {
  GameStateData,
  PlayerData,
  PlayerSelectionOptions,
} from "../../store/gameStore";
import { useTranslation } from "react-i18next";
import {
  buildActivePositionMap,
  buildPitchRows,
  buildPitchSlotRows,
  type PitchSlotRow,
} from "../squad/SquadTab.helpers";
import {
  buildTacticsRoster,
  countOutOfPositionPlayers,
  filterAndSortTacticsPlayers,
  getSelectedAndComparePlayers,
  resolveStartingXiIds,
  type SortKey,
} from "./TacticsTab.helpers";
import TacticsRolesPanel from "./TacticsRolesPanel";
import {
  applyTacticsFormationChange,
  applyTacticsPlayStyleChange,
  persistTacticsStartingXI,
} from "./TacticsTab.controller";
import {
  TacticsLineupContent,
  TacticsTabModeTabs,
} from "./TacticsTab.sections";
import { useTacticsLineupInteractions } from "./useTacticsLineupInteractions";

interface TacticsTabProps {
  gameState: GameStateData;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onGameUpdate: (g: GameStateData) => void;
}

export default function TacticsTab({
  gameState,
  onSelectPlayer,
  onGameUpdate,
}: TacticsTabProps): JSX.Element {
  const { t } = useTranslation();
  const currentDate = gameState.clock.current_date;
  const myTeam = gameState.teams.find(
    (team) => team.id === gameState.manager.team_id,
  );
  const [playerSearch, setPlayerSearch] = useState("");
  const [positionFilter, setPositionFilter] = useState("All");
  const [sortKey, setSortKey] = useState<SortKey>("pos");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
  const [pendingStartingXiIds, setPendingStartingXiIds] = useState<
    string[] | null
  >(null);
  const [activeTab, setActiveTab] = useState<"lineup" | "roles">("lineup");

  if (!myTeam) {
    return (
      <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>
    );
  }

  const roster = buildTacticsRoster(gameState.players, myTeam.id);

  const formation = myTeam.formation || "4-4-2";
  const activePlayStyle = myTeam.play_style || "Balanced";
  const savedStartingXiKey = (myTeam.starting_xi_ids || []).join(",");
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
        savedStartingXiIds: myTeam.starting_xi_ids || [],
      }),
    [
      available.map((player) => player.id).join(","),
      formation,
      (myTeam.starting_xi_ids || []).join(","),
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

  const xiIds = new Set(startingXiIds);
  const bench = roster.filter((player) => !xiIds.has(player.id));
  const xiActivePosition = useMemo(
    () => buildActivePositionMap(pitchSlotRows),
    [pitchSlotRows],
  );

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
          currentDate,
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
          currentDate,
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

  async function persistStartingXI(playerIds: string[]): Promise<void> {
    await persistTacticsStartingXI(
      playerIds,
      onGameUpdate,
      setPendingStartingXiIds,
    );
  }

  async function handleFormationChange(nextFormation: string): Promise<void> {
    await applyTacticsFormationChange(nextFormation, onGameUpdate);
  }

  async function handlePlayStyleChange(playStyle: string): Promise<void> {
    await applyTacticsPlayStyleChange(playStyle, onGameUpdate);
  }

  const {
    canConfirmSwap,
    clearLineupSelection,
    comparePlayerId,
    dragPreviewRef,
    dragState,
    handleConfirmSwap,
    handleDragStart,
    handleLineupPlayerClick,
    handleSlotDragLeave,
    handleSlotDragOver,
    handleSlotDrop,
    hoveredSlot,
    resetDragState,
    selectedPlayerId,
  } = useTacticsLineupInteractions({
    currentXiIds: startingXiIds,
    persistStartingXI,
    xiIds,
  });

  const { comparePlayer, selectedPlayer } = getSelectedAndComparePlayers(
    comparePlayerId,
    playersById,
    selectedPlayerId,
  );

  function handleClearFilters(): void {
    setPlayerSearch("");
    setPositionFilter("All");
  }

  return (
    <div className="max-w-6xl mx-auto flex flex-col gap-4">
      <div
        ref={dragPreviewRef}
        aria-hidden="true"
        className="pointer-events-none fixed -left-20 top-0 h-8 w-8 rounded-full border border-white/15 bg-navy-900/90 shadow-lg"
      />
      <TacticsTabModeTabs
        activeTab={activeTab}
        onSelectTab={setActiveTab}
      />

      {activeTab === "lineup" ? (
        <TacticsLineupContent
          activePlayStyle={activePlayStyle}
          bench={bench}
          canConfirmSwap={canConfirmSwap}
          comparePlayer={comparePlayer}
          comparePlayerId={comparePlayerId}
          currentDate={currentDate}
          dragState={dragState}
          filteredBench={filteredBench}
          filteredStartingXI={filteredStartingXI}
          formation={formation}
          handleClearFilters={handleClearFilters}
          handleConfirmSwap={handleConfirmSwap}
          handleDragStart={handleDragStart}
          handleFormationChange={handleFormationChange}
          handleLineupPlayerClick={handleLineupPlayerClick}
          handlePlayStyleChange={handlePlayStyleChange}
          handleSlotDragLeave={handleSlotDragLeave}
          handleSlotDragOver={handleSlotDragOver}
          handleSlotDrop={handleSlotDrop}
          hoveredSlot={hoveredSlot}
          onClearSelection={clearLineupSelection}
          onPlayerSearchChange={setPlayerSearch}
          onPositionFilterChange={setPositionFilter}
          onSelectPlayer={onSelectPlayer}
          outOfPositionCount={outOfPositionCount}
          pitchSlotRows={pitchSlotRows}
          playerSearch={playerSearch}
          positionFilter={positionFilter}
          resetDragState={resetDragState}
          selectedPlayer={selectedPlayer}
          selectedPlayerId={selectedPlayerId}
          sortDir={sortDir}
          sortKey={sortKey}
          startingXiCount={startingXI.length}
          toggleSort={toggleSort}
          xiActivePosition={xiActivePosition}
        />
      ) : (
        <TacticsRolesPanel
          allSquad={roster}
          matchRoles={myTeam.match_roles}
          onGameUpdate={onGameUpdate}
          startingPlayers={startingXI}
        />
      )}
    </div>
  );
}
