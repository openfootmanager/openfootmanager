import type { TFunction } from "i18next";
import type { FixtureData, GameStateData, LeagueData } from "../store/gameStore";

export function getFixtureDisplayLabel(
    t: TFunction,
    fixture: FixtureData,
): string {
    if (fixture.competition === "PreseasonTournament") {
        return t("season.preseasonTournament");
    }

    if (fixture.competition === "Friendly") {
        return t("season.friendly");
    }

    return t("common.matchday", { n: fixture.matchday });
}

export function isCompetitiveFixture(fixture: FixtureData): boolean {
    return (
        !fixture.competition ||
        !["Friendly", "PreseasonTournament"].includes(fixture.competition)
    );
}

export function getCompetitiveFixtures(fixtures: FixtureData[]): FixtureData[] {
    return fixtures.filter(isCompetitiveFixture);
}

export function findNextFixture(
    fixtures: FixtureData[],
    teamId: string,
): FixtureData | undefined {
    return fixtures.reduce<FixtureData | undefined>((nextFixture, fixture) => {
        const involvesTeam = fixture.home_team_id === teamId || fixture.away_team_id === teamId;

        if (fixture.status !== "Scheduled" || !involvesTeam) {
            return nextFixture;
        }

        if (!nextFixture) {
            return fixture;
        }

        if (fixture.date !== nextFixture.date) {
            return fixture.date < nextFixture.date ? fixture : nextFixture;
        }

        if (fixture.matchday !== nextFixture.matchday) {
            return fixture.matchday < nextFixture.matchday ? fixture : nextFixture;
        }

        return fixture.id < nextFixture.id ? fixture : nextFixture;
    }, undefined);
}

export function expectedFixtureCount(teamCount: number): number | null {
    if (teamCount >= 2) {
        // Double round robin; odd-sized leagues reach the same total via byes.
        return teamCount * (teamCount - 1);
    }

    return null;
}

export function hasFullLeagueSchedule(league: LeagueData): boolean {
    const expectedCount = expectedFixtureCount(league.standings.length);

    if (expectedCount === null) {
        return false;
    }

    return getCompetitiveFixtures(league.fixtures).length === expectedCount;
}

export function isSeasonComplete(league: LeagueData | null | undefined): boolean {
    if (!league || !hasFullLeagueSchedule(league)) {
        return false;
    }

    return getCompetitiveFixtures(league.fixtures).every(
        (fixture) => fixture.status === "Completed",
    );
}

export function getPrimaryCompetition(
    gameState: Pick<GameStateData, "competitions" | "league">,
): LeagueData | null {
    if (gameState.competitions && gameState.competitions.length > 0) {
        return gameState.competitions[0];
    }

    return gameState.league ?? null;
}

export function getActiveCompetitions(
    gameState: Pick<GameStateData, "competitions" | "league" | "active_competition_ids">,
): LeagueData[] {
    const competitions =
        gameState.competitions && gameState.competitions.length > 0
            ? gameState.competitions
            : gameState.league
              ? [gameState.league]
              : [];
    const activeIds = gameState.active_competition_ids ?? [];
    if (activeIds.length === 0) {
        return competitions;
    }

    return competitions.filter((competition) => activeIds.includes(competition.id));
}

export function getAllFixturesAcrossCompetitions(
    gameState: Pick<GameStateData, "competitions" | "league" | "active_competition_ids">,
): FixtureData[] {
    return getActiveCompetitions(gameState).flatMap((competition) => competition.fixtures);
}

function competitionIncludesTeam(competition: LeagueData, teamId: string): boolean {
    return (
        (competition.participant_ids?.includes(teamId) ?? false) ||
        competition.standings.some((entry) => entry.team_id === teamId) ||
        competition.fixtures.some(
            (fixture) =>
                fixture.home_team_id === teamId || fixture.away_team_id === teamId,
        )
    );
}

// The manager's own domestic league — the source of truth for the league-table
// cards. In the multi-competition world this lives in `gameState.competitions`;
// the legacy `gameState.league` is kept only as a fallback for old saves.
export function getUserCompetition(
    gameState: Pick<GameStateData, "competitions" | "league" | "manager">,
): LeagueData | null {
    const teamId = gameState.manager?.team_id ?? null;
    const competitions = gameState.competitions ?? [];

    if (teamId && competitions.length > 0) {
        const domestic = competitions.find(
            (competition) =>
                competition.kind === "League" &&
                competition.scope === "Domestic" &&
                competitionIncludesTeam(competition, teamId),
        );
        if (domestic) {
            return domestic;
        }

        const fallback = competitions.find((competition) =>
            competitionIncludesTeam(competition, teamId),
        );
        if (fallback) {
            return fallback;
        }
    }

    return gameState.league ?? null;
}

// Every active competition the manager's team takes part in (league, cups,
// continental). Used to find the next fixture regardless of competition.
export function getUserCompetitions(
    gameState: Pick<
        GameStateData,
        "competitions" | "league" | "active_competition_ids" | "manager"
    >,
): LeagueData[] {
    const teamId = gameState.manager?.team_id ?? null;
    const active = getActiveCompetitions(gameState);

    if (!teamId) {
        return active;
    }

    const mine = active.filter((competition) =>
        competitionIncludesTeam(competition, teamId),
    );
    return mine.length > 0 ? mine : active;
}

export function getUserNextFixture(
    gameState: Pick<
        GameStateData,
        "competitions" | "league" | "active_competition_ids" | "manager"
    >,
): FixtureData | null {
    const teamId = gameState.manager?.team_id ?? null;
    if (!teamId) {
        return null;
    }

    const fixtures = getUserCompetitions(gameState).flatMap(
        (competition) => competition.fixtures,
    );
    return findNextFixture(fixtures, teamId) ?? null;
}
