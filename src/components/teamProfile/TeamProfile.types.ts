import type { TFunction } from "i18next";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";

export interface TeamProfileProps {
  team: TeamData;
  gameState: GameStateData;
  isOwnTeam: boolean;
  onSelectPlayer?: (id: string) => void;
}

export type TeamProfileTranslate = TFunction;

/** The backend's answer for a club's league table: which table, where it stands, and its record. */
export interface ClubLeagueStanding {
  competition_id: string;
  competition_name: string;
  league_pos: number;
  standing: {
    played: number;
    won: number;
    drawn: number;
    lost: number;
    goals_for: number;
    goals_against: number;
    points: number;
  };
}

export type LeagueStanding = ClubLeagueStanding["standing"];

export interface TeamProfileViewModel {
  roster: PlayerData[];
  totalWages: number;
  totalValue: number;
  manager: GameStateData["manager"] | null;
  leaguePos: number;
  standings: LeagueStanding | null;
}

export interface TeamStatsOverview {
  matchesPlayed: number;
  goalsFor: number;
  goalsAgainst: number;
  goalDifference: number;
  possessionAverage: number | null;
  metrics: {
    shots: { total: number; perMatch: number | null };
    shotsOnTarget: { total: number; perMatch: number | null };
    passes: {
      completed: number;
      attempted: number;
      accuracy: number | null;
    };
    tacklesWon: { total: number; perMatch: number | null };
    interceptions: { total: number; perMatch: number | null };
    foulsCommitted: { total: number; perMatch: number | null };
  };
}

export interface TeamRecentMatchEntry {
  fixtureId: string;
  date: string;
  competition: string;
  matchday: number;
  opponentTeamId: string;
  opponentName: string;
  goalsFor: number;
  goalsAgainst: number;
  possessionPct: number;
  shots: number;
  shotsOnTarget: number;
}
