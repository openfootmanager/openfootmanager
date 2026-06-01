import type { TFunction } from "i18next";
import type { FixtureData, LeagueData } from "../store/gameStore";

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
    return !fixture.competition || fixture.competition === "League";
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
    if (teamCount >= 2 && teamCount % 2 === 0) {
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
