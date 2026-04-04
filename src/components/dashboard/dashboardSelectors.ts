import type {
  FixtureData,
  GameStateData,
} from "../../store/gameStore";

export function getTodayMatchFixture(gameState: GameStateData): FixtureData | null {
  const fixtures = gameState.league?.fixtures;

  if (!fixtures) {
    return null;
  }

  const today = gameState.clock.current_date.split("T")[0];

  return (
    fixtures.find((fixture) => {
      return (
        fixture.date === today &&
        fixture.status === "Scheduled" &&
        (fixture.home_team_id === gameState.manager.team_id ||
          fixture.away_team_id === gameState.manager.team_id)
      );
    }) ?? null
  );
}

export function getUnreadMessagesCount(gameState: GameStateData): number {
  return gameState.messages.filter((message) => !message.read).length;
}

export function getManagerTeamName(gameState: GameStateData): string | null {
  return (
    gameState.teams.find((team) => team.id === gameState.manager.team_id)?.name ??
    null
  );
}

export function getPlayerBadgeVariant(
  position: string,
): "accent" | "danger" | "primary" | "success" {
  switch (position) {
    case "Goalkeeper":
      return "accent";
    case "Defender":
      return "primary";
    case "Midfielder":
      return "success";
    default:
      return "danger";
  }
}