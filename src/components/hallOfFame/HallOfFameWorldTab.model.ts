import type { GameStateData, TeamData, TeamSeasonRecord } from "../../store/gameStore";

export interface PastChampionEntry {
  season: number;
  team: TeamData;
  record: TeamSeasonRecord;
}

export function derivePastChampions(gameState: GameStateData): PastChampionEntry[] {
  return gameState.teams
    .flatMap((team) => {
      return team.history
        .filter((record) => record.league_position === 1)
        .map((record) => ({ season: record.season, team, record }));
    })
    .sort((left, right) => {
      return (
        right.season - left.season ||
        right.record.won - left.record.won ||
        left.team.name.localeCompare(right.team.name)
      );
    });
}
