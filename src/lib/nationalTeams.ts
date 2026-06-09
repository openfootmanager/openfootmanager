import type {
  FixtureData,
  GameStateData,
  PlayerData,
} from "../store/gameStore";

/** All national-team fixtures across every nation (stored on the home nation). */
export function getNationalTeamFixtures(
  gameState: Pick<GameStateData, "national_teams">,
): FixtureData[] {
  return (gameState.national_teams ?? []).flatMap((team) => team.fixtures ?? []);
}

/** Display name for a national team, falling back to its id when unknown. */
export function getNationalTeamName(
  gameState: Pick<GameStateData, "national_teams">,
  nationalTeamId: string,
): string {
  const team = (gameState.national_teams ?? []).find(
    (nation) => nation.id === nationalTeamId,
  );
  return team?.name ?? nationalTeamId;
}

export interface CalledUpPlayer {
  player: PlayerData;
  nationalTeamId: string;
  nationalTeamName: string;
}

/**
 * The user's club players who are in the squad of a national team that has
 * fixtures this season. Nations are matched against every fixture (home and
 * away), since fixtures are only stored on the home nation.
 */
export function getUserCalledUpPlayers(
  gameState: Pick<GameStateData, "national_teams" | "players" | "manager">,
): CalledUpPlayer[] {
  const userTeamId = gameState.manager.team_id;
  if (!userTeamId) {
    return [];
  }

  const nationalTeams = gameState.national_teams ?? [];
  const participatingNationIds = new Set<string>();
  for (const fixture of getNationalTeamFixtures(gameState)) {
    participatingNationIds.add(fixture.home_team_id);
    participatingNationIds.add(fixture.away_team_id);
  }

  const calledUp: CalledUpPlayer[] = [];
  for (const player of gameState.players) {
    if (player.team_id !== userTeamId) {
      continue;
    }
    const nation = nationalTeams.find(
      (team) =>
        participatingNationIds.has(team.id) &&
        team.squad_player_ids.includes(player.id),
    );
    if (nation) {
      calledUp.push({
        player,
        nationalTeamId: nation.id,
        nationalTeamName: nation.name,
      });
    }
  }
  return calledUp;
}
