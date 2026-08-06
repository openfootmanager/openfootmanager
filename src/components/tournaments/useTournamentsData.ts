import { useEffect, useState } from "react";

import type { GameStateData, LeagueData } from "../../store/gameStore";
import type { SeasonContextData, WorldCupChampionData } from "../../store/types";
import {
  fetchCompetitionsView,
  type CompetitionsView,
  type PlayerNameEntry,
} from "../../services/competitionsService";
import { resolveSeasonContext } from "../../lib/seasonContext";

interface UseTournamentsDataResult {
  teamNames: Record<string, string>;
  nationalTeamNames: Record<string, string>;
  nationalTeamNameKeys: Record<string, string>;
  playerNames: Record<string, PlayerNameEntry>;
  userTeamId: string | null;
  seasonContext: SeasonContextData;
  isPreseason: boolean;
  activeCompetitions: LeagueData[];
  setSelectedCompetitionId: (id: string | null) => void;
  league: LeagueData | null;
  currentSeason: number;
  isWorldCup: boolean;
  worldCupChampion: WorldCupChampionData | null;
}

/**
 * Everything the tournaments screen reads: the competitions slice, the name
 * lookups it resolves, and which competition is on show.
 *
 * The slice is the source of truth, but it arrives asynchronously, so every
 * field falls back to the equivalent on `gameState` while it loads. That keeps
 * the screen populated on first paint instead of flashing an empty table.
 */
export function useTournamentsData(
  gameState: GameStateData,
): UseTournamentsDataResult {
  const [competitionsView, setCompetitionsView] =
    useState<CompetitionsView | null>(null);
  const [selectedCompetitionId, setSelectedCompetitionId] = useState<
    string | null
  >(null);

  const currentDate = gameState.clock?.current_date;

  useEffect(() => {
    let cancelled = false;
    fetchCompetitionsView()
      .then((view) => {
        if (!cancelled) setCompetitionsView(view);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [currentDate]);

  // Fall back to building a teamNames map from gameState.teams while slice loads.
  const fallbackTeamNames: Record<string, string> = Object.fromEntries(
    (gameState.teams ?? []).map((t) => [t.id, t.name]),
  );
  const teamNames = competitionsView?.team_names ?? fallbackTeamNames;
  const fallbackNationalTeamNames: Record<string, string> = Object.fromEntries(
    (gameState.national_teams ?? []).map((nt) => [nt.id, nt.name]),
  );
  const nationalTeamNames =
    competitionsView?.national_team_names ?? fallbackNationalTeamNames;
  const nationalTeamNameKeys = competitionsView?.national_team_name_keys ?? {};

  // The player-name fallback stays in the component: it walks every player in
  // the world, and the component only needs it once a competition is on show.
  const playerNames = competitionsView?.player_names ?? {};

  const userTeamId =
    competitionsView?.manager_team_id ?? gameState.manager.team_id;
  const seasonContext = resolveSeasonContext(gameState);
  const isPreseason = seasonContext.phase === "Preseason";

  // Derive active competitions from slice; fall back to gameState while loading.
  const gsLeague = gameState.league ? [gameState.league] : [];
  const allCompetitions =
    competitionsView?.competitions ?? gameState.competitions ?? gsLeague;
  const activeIds =
    competitionsView?.active_competition_ids ??
    gameState.active_competition_ids ??
    [];
  const activeCompetitions =
    activeIds.length === 0
      ? allCompetitions
      : allCompetitions.filter((c) => activeIds.includes(c.id));

  const userCompetitions = activeCompetitions.filter((competition) =>
    competition.participant_ids?.includes(userTeamId ?? ""),
  );
  const league =
    activeCompetitions.find(
      (competition) => competition.id === selectedCompetitionId,
    ) ??
    userCompetitions[0] ??
    activeCompetitions[0] ??
    gameState.league ??
    null;
  const currentSeason = league?.season ?? 0;
  const isWorldCup = league?.kind === "InternationalNation";
  const worldCupChampions =
    competitionsView?.world_cup_champions ??
    gameState.world_history?.world_cup_champions ??
    [];
  const worldCupChampion = isWorldCup
    ? (worldCupChampions.find((c) => c.year === currentSeason) ?? null)
    : null;

  const activeCompetitionIds = activeCompetitions.map((c) => c.id).join(",");
  const userCompetitionIds = userCompetitions.map((c) => c.id).join(",");

  useEffect(() => {
    if (activeCompetitions.length === 0) {
      if (selectedCompetitionId !== null) {
        setSelectedCompetitionId(null);
      }
      return;
    }

    const hasSelection = activeCompetitions.some(
      (competition) => competition.id === selectedCompetitionId,
    );
    if (hasSelection) {
      return;
    }

    setSelectedCompetitionId(userCompetitions[0]?.id ?? activeCompetitions[0].id);
  // activeCompetitionIds / userCompetitionIds are stable string keys derived from
  // the arrays; using the arrays directly would cause the effect to fire on every
  // render because getActiveCompetitions() and .filter() always return new refs.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeCompetitionIds, selectedCompetitionId, userCompetitionIds]);

  return {
    teamNames,
    nationalTeamNames,
    nationalTeamNameKeys,
    playerNames,
    userTeamId,
    seasonContext,
    isPreseason,
    activeCompetitions,
    setSelectedCompetitionId,
    league,
    currentSeason,
    isWorldCup,
    worldCupChampion,
  };
}
