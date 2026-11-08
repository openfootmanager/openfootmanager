import { useEffect, useMemo, useState } from "react";

import type { GameStateData, PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import { useGameStore } from "../../store/gameStore";
import {
  applyTacticPreset,
  setFormation,
  setPlayStyle,
  setStartingXi,
  setTacticsPhase as setTacticsPhaseService,
  setTeamMatchRoles,
} from "../../services/squadService";
import { useFetchedSquad } from "../../hooks/useFetchedSquad";
import { useFamiliarity } from "../../hooks/useFamiliarity";
import { usePositionTable } from "../../hooks/usePositionGroups";
import { useAutoSetPieces } from "../../hooks/useAutoSetPieces";
import {
  tacticsLineupRevision,
  useTacticsLineupProjection,
} from "../../hooks/useTacticsLineupProjection";
import type { TacticsPhaseSettings } from "../../store/types";

import {
  applyLineupSwap,
  buildAssignBestFitSlot,
  buildActivePositionMap,
  buildDemoteFromStartingXi,
  buildPitchRows,
  buildPitchSlotRows,
  buildPromoteToStartingXi,
  type PitchSlotRow,
  type SquadSection,
  lineupReady,
} from "../squad/SquadTab.helpers";
import {
  buildTacticsPitchSlots,
  buildTacticsRoster,
  countOutOfPositionPlayers,
  findTacticsPresetBySetup,
  getSelectedAndComparePlayers,
} from "./TacticsTab.helpers";
import {
  buildUpdatedMatchRolesForAssignment,
  resolveEffectiveMatchRoles,
} from "./TacticsRoles.helpers";
import { isMatchDayEligible } from "../../lib/playerSquad";

interface UseTacticsLineupArgs {
  gameState: GameStateData | null;
  onGameUpdate: (g: GameStateData) => void;
}

function isPlayerEligibleForLineup(player: PlayerData | null | undefined): boolean {
  return Boolean(player && isMatchDayEligible(player));
}

export function useTacticsLineup({ gameState, onGameUpdate }: UseTacticsLineupArgs) {
  const { sessionState } = useGameStore();
  const positionTable = usePositionTable();
  const familiarity = useFamiliarity();
  const teamId = sessionState?.manager?.team_id ?? gameState?.manager?.team_id ?? null;
  const clockDate = sessionState?.clock.current_date ?? gameState?.clock.current_date ?? "";
  const [fetchedSquad, , squadAnswered] = useFetchedSquad(
    teamId,
    clockDate,
    sessionState ?? gameState,
  );
  const initialTeam = sessionState?.team ?? gameState?.teams?.find((t) => t.id === teamId) ?? null;
  const backendLineup = useTacticsLineupProjection(
    tacticsLineupRevision(
      gameState?.career_token ?? 0,
      initialTeam,
      fetchedSquad ?? gameState?.players ?? [],
      clockDate,
    ),
  );
  // The position table, the familiarity, the squad's answer and the backend's side for this
  // revision: a lineup can be graded and changed. A change worked out from an older eleven must not
  // be saved. The writers, the notice and the count all read this one flag.
  const canGradeLineup =
    lineupReady(positionTable, familiarity) && squadAnswered && backendLineup.loaded;
  const initialPreset = initialTeam
    ? findTacticsPresetBySetup(
        initialTeam.formation || "4-4-2",
        initialTeam.play_style || "Balanced",
      )
    : null;
  const [pendingStartingXiIds, setPendingStartingXiIds] = useState<string[] | null>(null);
  const [selectedPlayerId, setSelectedPlayerId] = useState<string | null>(null);
  const [selectedPlayerSection, setSelectedPlayerSection] = useState<SquadSection | null>(null);
  const [comparePlayerId, setComparePlayerId] = useState<string | null>(null);
  const [comparePlayerSection, setComparePlayerSection] = useState<SquadSection | null>(null);

  const team = sessionState?.team ?? gameState?.teams?.find((t) => t.id === teamId) ?? null;
  const players = fetchedSquad ?? gameState?.players ?? [];
  const roster = team ? buildTacticsRoster(players, team.id, positionTable) : [];

  const formation = team?.formation || "4-4-2";
  const activePlayStyle = team?.play_style || "Balanced";
  const savedStartingXiKey = (team?.starting_xi_ids || []).join(",");
  const playersById = useMemo(() => new Map(roster.map((player) => [player.id, player])), [roster]);
  const available = roster.filter(isMatchDayEligible);
  const pitchRows = useMemo(() => buildPitchRows(formation), [formation]);

  // The eleven is the one the match would field, as the backend picks it (`get_tactics_lineup`);
  // the screen keeps no selection of its own. A change the manager just made shows at once, until
  // the save's answer catches up, and the saved eleven stands in until the first answer arrives.
  const backendEleven = backendLineup.lineup?.eleven.map((slot) => slot.player_id) ?? null;
  const startingXiIds = useMemo(
    () =>
      (pendingStartingXiIds && pendingStartingXiIds.length > 0
        ? pendingStartingXiIds
        : (backendEleven ?? team?.starting_xi_ids ?? [])
      ).filter((id) => playersById.has(id)),
    [
      (pendingStartingXiIds || []).join(","),
      (backendEleven || []).join(","),
      (team?.starting_xi_ids || []).join(","),
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
  const pitchSlots = useMemo(() => buildTacticsPitchSlots(pitchSlotRows), [pitchSlotRows]);
  const xiIds = new Set(startingXiIds);
  const bench = roster.filter((player) => !xiIds.has(player.id));
  const xiActivePosition = useMemo(() => buildActivePositionMap(pitchSlotRows), [pitchSlotRows]);

  const { comparePlayer, selectedPlayer } = getSelectedAndComparePlayers(
    comparePlayerId,
    playersById,
    selectedPlayerId,
  );

  const canConfirmSwap = useMemo(() => {
    if (!selectedPlayerId || !selectedPlayerSection || !comparePlayerId || !comparePlayerSection) {
      return false;
    }

    if (
      (selectedPlayerSection === "bench" &&
        !isPlayerEligibleForLineup(selectedPlayerId ? playersById.get(selectedPlayerId) : null)) ||
      (comparePlayerSection === "bench" &&
        !isPlayerEligibleForLineup(comparePlayerId ? playersById.get(comparePlayerId) : null))
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

  // No count is claimed before the lineup can be graded: a "0" there would read as "all in place".
  const outOfPositionCount = canGradeLineup
    ? countOutOfPositionPlayers(startingXI, xiActivePosition, positionTable, familiarity)
    : null;
  const autoSetPieces = useAutoSetPieces(startingXiIds, sessionState ?? gameState);
  const effectiveMatchRoles = useMemo(
    () => resolveEffectiveMatchRoles(startingXI, team?.match_roles, autoSetPieces),
    [team?.match_roles, startingXI, autoSetPieces],
  );

  /** Whether the lineup was saved: a selection is kept when it was not (nothing changed). */
  async function persistStartingXI(playerIds: string[]): Promise<boolean> {
    // Nothing is saved from a lineup worked out before the position table and the familiarity have
    // loaded, nor from a squad the backend has not answered for this revision: its eligibility is
    // the last one's.
    if (!canGradeLineup) return false;
    setPendingStartingXiIds(playerIds);
    try {
      const updated = await setStartingXi(playerIds);
      onGameUpdate(updated);
      return true;
    } catch (error) {
      setPendingStartingXiIds(null);
      console.error("Failed to set starting XI:", error);
      return false;
    }
  }

  async function handleFormationChange(nextFormation: string): Promise<boolean> {
    try {
      const updated = await setFormation(nextFormation);
      onGameUpdate(updated);
      return true;
    } catch (error) {
      console.error("Failed to set formation:", error);
      return false;
    }
  }

  async function handlePlayStyleChange(playStyle: string): Promise<boolean> {
    try {
      const updated = await setPlayStyle(playStyle);
      onGameUpdate(updated);
      return true;
    } catch (error) {
      console.error("Failed to set play style:", error);
      return false;
    }
  }

  async function handleApplyTacticPreset(
    nextFormation: string,
    playStyle: string,
  ): Promise<boolean> {
    try {
      const updated = await applyTacticPreset(nextFormation, playStyle);
      onGameUpdate(updated);
      return true;
    } catch (error) {
      console.error("Failed to apply tactic preset:", error);
      return false;
    }
  }

  async function handleAssignBestFit(playerId: string): Promise<void> {
    const nextXiIds = buildAssignBestFitSlot(
      startingXiIds,
      playersById,
      formation,
      playerId,
      positionTable,
      familiarity,
    );

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    if (await persistStartingXI(nextXiIds)) clearLineupSelection();
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
      positionTable,
      familiarity,
    );

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    if (await persistStartingXI(nextXiIds)) clearLineupSelection();
  }

  async function handleDemoteStarter(playerId: string): Promise<void> {
    const nextXiIds = buildDemoteFromStartingXi(
      startingXiIds,
      available,
      formation,
      playerId,
      positionTable,
      familiarity,
    );

    if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
      return;
    }

    if (await persistStartingXI(nextXiIds)) clearLineupSelection();
  }

  function clearLineupSelection(): void {
    setSelectedPlayerId(null);
    setSelectedPlayerSection(null);
    setComparePlayerId(null);
    setComparePlayerSection(null);
  }

  async function handleLineupPlayerClick(playerId: string, section: SquadSection): Promise<void> {
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
    if (!selectedPlayerId || !selectedPlayerSection || !comparePlayerId || !comparePlayerSection) {
      return;
    }

    if (
      (selectedPlayerSection === "bench" &&
        !isPlayerEligibleForLineup(selectedPlayerId ? playersById.get(selectedPlayerId) : null)) ||
      (comparePlayerSection === "bench" &&
        !isPlayerEligibleForLineup(comparePlayerId ? playersById.get(comparePlayerId) : null))
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

    if (await persistStartingXI(nextXiIds)) clearLineupSelection();
  }

  async function persistMatchRoles(nextRoles: TeamMatchRolesData): Promise<void> {
    try {
      const updated = await setTeamMatchRoles(nextRoles);
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
      buildUpdatedMatchRolesForAssignment(startingXI, team?.match_roles, role, playerId),
    );
  }

  async function handleTacticsPhaseChange(patch: Partial<TacticsPhaseSettings>): Promise<void> {
    try {
      const updated = await setTacticsPhaseService(patch);
      onGameUpdate(updated);
    } catch (error) {
      console.error("Failed to set tactics phase:", error);
    }
  }

  return {
    /** Lineup changes are possible: the writers, the notice and the count read this one flag. */
    lineupReady: canGradeLineup,
    /** The one familiarity table the screen grades with (its count, its writers, its pitch). */
    familiarity,
    // The backend's side, for the table's per-slot words (role, familiarity) and its groups.
    lineup: backendLineup.lineup,
    team,
    formation,
    activePlayStyle,
    initialPreset,
    roster,
    startingXI,
    bench,
    xiActivePosition,
    pitchSlots,
    outOfPositionCount,
    effectiveMatchRoles,
    autoSetPieces,
    selectedPlayer,
    comparePlayer,
    selectedPlayerId,
    comparePlayerId,
    canConfirmSwap,
    handleFormationChange,
    handlePlayStyleChange,
    handleApplyTacticPreset,
    handleAssignBestFit,
    handlePromoteBenchPlayer,
    handleDemoteStarter,
    clearLineupSelection,
    handleLineupPlayerClick,
    handleConfirmSwap,
    handleAssignMatchRole,
    handleTacticsPhaseChange,
  };
}
