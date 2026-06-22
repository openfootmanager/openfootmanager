import type {
  FixtureData,
  GameStateData,
  PlayerData,
} from "../store/gameStore";

/**
 * All national-team fixtures: window friendlies (stored on the home nation)
 * plus any national-team tournament (e.g. the World Cup), which lives as an
 * InternationalNation competition.
 */
export function getNationalTeamFixtures(
  gameState: Pick<GameStateData, "national_teams" | "competitions">,
): FixtureData[] {
  const windowFixtures = (gameState.national_teams ?? []).flatMap(
    (team) => team.fixtures ?? [],
  );
  const tournamentFixtures = (gameState.competitions ?? [])
    .filter((competition) => competition.kind === "InternationalNation")
    .flatMap((competition) => competition.fixtures);
  return [...windowFixtures, ...tournamentFixtures];
}

type TranslateFn = (key: string, options?: Record<string, unknown>) => string;

/** Display name for a national team, falling back to its id when unknown. */
export function getNationalTeamName(
  gameState: Pick<GameStateData, "national_teams">,
  nationalTeamId: string,
  t?: TranslateFn,
): string {
  const team = (gameState.national_teams ?? []).find(
    (nation) => nation.id === nationalTeamId,
  );
  if (!team) return nationalTeamId;
  if (t && team.name_key) {
    return t("nations.nationalTeamTemplate", { name: t(team.name_key) });
  }
  return team.name;
}

export interface CalledUpPlayer {
  player: PlayerData;
  nationalTeamId: string;
  nationalTeamName: string;
  nationalTeamNameKey?: string | null;
}

/**
 * The user's club players who are in the squad of a national team that has
 * fixtures this season. Nations are matched against every fixture (home and
 * away), since fixtures are only stored on the home nation.
 */
export function getUserCalledUpPlayers(
  gameState: Pick<GameStateData, "national_teams" | "players" | "manager" | "competitions">,
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
        nationalTeamNameKey: nation.name_key,
      });
    }
  }
  return calledUp;
}
