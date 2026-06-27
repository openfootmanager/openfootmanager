import type {
  GameStateData,
  SeasonContextData,
  SeasonPhase,
  TransferWindowContextData,
  TransferWindowStatus,
} from "../store/gameStore";
import { TRANSFER_WINDOW_DAYS } from "./domainConstants";
import { getUserCompetition } from "./fixtures";

const DAY_IN_MS = 24 * 60 * 60 * 1000;

const DEFAULT_TRANSFER_WINDOW: TransferWindowContextData = {
  status: "Closed",
  opens_on: null,
  closes_on: null,
  days_until_opens: null,
  days_remaining: null,
};

const DEFAULT_SEASON_CONTEXT: SeasonContextData = {
  phase: "Preseason",
  season_start: null,
  season_end: null,
  days_until_season_start: null,
  transfer_window: DEFAULT_TRANSFER_WINDOW,
};

export function resolveSeasonContext(gameState: GameStateData): SeasonContextData {
  if (gameState.season_context) {
    return normaliseSeasonContext(gameState.season_context);
  }

  return deriveSeasonContext(gameState);
}

export function hasCompetitiveStandings(gameState: GameStateData): boolean {
  return resolveSeasonContext(gameState).phase !== "Preseason";
}

function normaliseSeasonContext(context: SeasonContextData): SeasonContextData {
  return {
    phase: context.phase,
    season_start: context.season_start ?? null,
    season_end: context.season_end ?? null,
    days_until_season_start: context.days_until_season_start ?? null,
    transfer_window: {
      status: context.transfer_window?.status ?? "Closed",
      opens_on: context.transfer_window?.opens_on ?? null,
      closes_on: context.transfer_window?.closes_on ?? null,
      days_until_opens: context.transfer_window?.days_until_opens ?? null,
      days_remaining: context.transfer_window?.days_remaining ?? null,
    },
  };
}

function deriveSeasonContext(gameState: GameStateData): SeasonContextData {
  const league = getUserCompetition(gameState);
  if (!league) {
    return DEFAULT_SEASON_CONTEXT;
  }

  const competitiveFixtures = league.fixtures.filter((fixture) =>
    !fixture.competition || fixture.competition === "League",
  );

  const fixtureDates = competitiveFixtures
    .map((fixture) => parseUtcDate(fixture.date))
    .filter((date): date is Date => date != null)
    .sort((leftDate, rightDate) => leftDate.getTime() - rightDate.getTime());

  const seasonStart = fixtureDates[0] ?? null;
  const seasonEnd =
    fixtureDates.length > 0 ? fixtureDates[fixtureDates.length - 1] : null;
  const currentDate = parseUtcDate(gameState.clock.current_date);
  const hasStarted =
    league.standings.some((entry) => entry.played > 0) ||
    competitiveFixtures.some((fixture) => fixture.status === "Completed");
  const isComplete =
    competitiveFixtures.length > 0 &&
    competitiveFixtures.every((fixture) => fixture.status === "Completed");

  let phase: SeasonPhase = "Preseason";
  if (isComplete) {
    phase = "PostSeason";
  } else if (hasStarted) {
    phase = "InSeason";
  }

  return {
    phase,
    season_start: formatUtcDate(seasonStart),
    season_end: formatUtcDate(seasonEnd),
    days_until_season_start:
      currentDate && seasonStart
        ? positiveDayDiff(currentDate, seasonStart)
        : null,
    transfer_window: deriveTransferWindowContext(currentDate, seasonStart),
  };
}

function deriveTransferWindowContext(
  currentDate: Date | null,
  seasonStart: Date | null,
): TransferWindowContextData {
  if (!currentDate || !seasonStart) {
    return DEFAULT_TRANSFER_WINDOW;
  }

  let windowSeasonStart = seasonStart;
  let opensOn = transferWindowOpensOn(windowSeasonStart);
  let closesOn = transferWindowClosesOn(windowSeasonStart);

  while (currentDate > closesOn) {
    windowSeasonStart = addYearsClamped(windowSeasonStart, 1);
    opensOn = transferWindowOpensOn(windowSeasonStart);
    closesOn = transferWindowClosesOn(windowSeasonStart);
  }

  let status: TransferWindowStatus = "Closed";
  let daysUntilOpens: number | null = null;
  let daysRemaining: number | null = null;

  if (currentDate < opensOn) {
    daysUntilOpens = dayDiff(currentDate, opensOn);
  } else if (currentDate <= closesOn) {
    daysRemaining = dayDiff(currentDate, closesOn);
    status = daysRemaining === 0 ? "DeadlineDay" : "Open";
  }

  return {
    status,
    opens_on: formatUtcDate(opensOn),
    closes_on: formatUtcDate(closesOn),
    days_until_opens: daysUntilOpens,
    days_remaining: daysRemaining,
  };
}

function parseUtcDate(input: string | null | undefined): Date | null {
  if (!input) {
    return null;
  }

  if (/^\d{4}-\d{2}-\d{2}$/.test(input)) {
    const parsed = new Date(`${input}T00:00:00Z`);
    return Number.isNaN(parsed.getTime()) ? null : parsed;
  }

  const parsed = new Date(input);
  if (Number.isNaN(parsed.getTime())) {
    return null;
  }

  return new Date(
    Date.UTC(parsed.getUTCFullYear(), parsed.getUTCMonth(), parsed.getUTCDate()),
  );
}

function formatUtcDate(date: Date | null): string | null {
  if (!date) {
    return null;
  }

  return date.toISOString().slice(0, 10);
}

function addDays(date: Date, days: number): Date {
  return new Date(date.getTime() + days * DAY_IN_MS);
}

function addYearsClamped(date: Date, years: number): Date {
  const targetYear = date.getUTCFullYear() + years;
  const candidate = new Date(
    Date.UTC(targetYear, date.getUTCMonth(), date.getUTCDate()),
  );

  if (candidate.getUTCMonth() === date.getUTCMonth()) {
    return candidate;
  }

  return new Date(Date.UTC(targetYear, date.getUTCMonth() + 1, 0));
}

function transferWindowOpensOn(seasonStart: Date): Date {
  return addDays(seasonStart, -TRANSFER_WINDOW_DAYS.preseasonOpenLead);
}

function transferWindowClosesOn(seasonStart: Date): Date {
  return addDays(seasonStart, TRANSFER_WINDOW_DAYS.postSeasonStartClose);
}

function dayDiff(startDate: Date, endDate: Date): number {
  return Math.round((endDate.getTime() - startDate.getTime()) / DAY_IN_MS);
}

function positiveDayDiff(startDate: Date, endDate: Date): number | null {
  const difference = dayDiff(startDate, endDate);
  return difference >= 0 ? difference : null;
}
