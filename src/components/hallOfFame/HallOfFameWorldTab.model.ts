import type { GameStateData, PlayerData, TeamData, TeamSeasonRecord } from "../../store/gameStore";

export interface HallOfFameLegend {
  player: PlayerData;
  appearances: number;
  goals: number;
  assists: number;
  titles: number;
  lastClubName: string | null;
  finalSeason: number | null;
}

export interface PastChampionEntry {
  season: number;
  team: TeamData;
  record: TeamSeasonRecord;
}

function championSeasonsByTeam(gameState: GameStateData): Map<string, Set<number>> {
  const champions = new Map<string, Set<number>>();

  for (const team of gameState.teams) {
    const seasons = new Set<number>();

    for (const record of team.history) {
      if (record.league_position === 1) {
        seasons.add(record.season);
      }
    }

    if (seasons.size > 0) {
      champions.set(team.id, seasons);
    }
  }

  return champions;
}

export function deriveHallOfFameLegends(
  gameState: GameStateData,
): HallOfFameLegend[] {
  const championSeasons = championSeasonsByTeam(gameState);

  return gameState.players
    .filter((player) => player.retired && player.career.length > 0)
    .map((player) => {
      const appearances = player.career.reduce(
        (total, entry) => total + entry.appearances,
        0,
      );
      const goals = player.career.reduce((total, entry) => total + entry.goals, 0);
      const assists = player.career.reduce(
        (total, entry) => total + entry.assists,
        0,
      );
      const lastEntry = [...player.career].sort(
        (left, right) => right.season - left.season,
      )[0] ?? null;
      const titles = player.career.reduce((count, entry) => {
        return count + (championSeasons.get(entry.team_id)?.has(entry.season) ? 1 : 0);
      }, 0);

      return {
        player,
        appearances,
        goals,
        assists,
        titles,
        lastClubName: lastEntry?.team_name ?? null,
        finalSeason: lastEntry?.season ?? null,
      };
    })
    .sort((left, right) => {
      return right.titles - left.titles
        || right.appearances - left.appearances
        || right.goals - left.goals
        || left.player.full_name.localeCompare(right.player.full_name);
    });
}

export function derivePastChampions(gameState: GameStateData): PastChampionEntry[] {
  return gameState.teams
    .flatMap((team) => {
      return team.history
        .filter((record) => record.league_position === 1)
        .map((record) => ({ season: record.season, team, record }));
    })
    .sort((left, right) => {
      return right.season - left.season
        || right.record.won - left.record.won
        || left.team.name.localeCompare(right.team.name);
    });
}
