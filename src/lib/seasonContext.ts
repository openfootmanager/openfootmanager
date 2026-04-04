import type {
  GameStateData,
  SeasonContextData,
  SeasonPhase,
  TransferWindowContextData,
  TransferWindowStatus,
} from "../store/gameStore";
import {
  addGameDays,
  diffGameDays,
  formatGameDate,
  parseGameDate,
  positiveGameDayDiff,
} from "./gameDate";
import { TRANSFER_WINDOW_DAYS } from "./domainConstants";

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
  const league = gameState.league;
  if (!league) {
    return DEFAULT_SEASON_CONTEXT;
  }

  const competitiveFixtures = league.fixtures.filter((fixture) =>
    !fixture.competition || fixture.competition === "League",
  );

  const fixtureDates = competitiveFixtures
    .map((fixture) => parseGameDate(fixture.date))
    .filter((date): date is Date => date != null)
    .sort((leftDate, rightDate) => leftDate.getTime() - rightDate.getTime());

  const seasonStart = fixtureDates[0] ?? null;
  const seasonEnd =
    fixtureDates.length > 0 ? fixtureDates[fixtureDates.length - 1] : null;
  const currentDate = parseGameDate(gameState.clock.current_date);
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
    season_start: formatGameDate(seasonStart),
    season_end: formatGameDate(seasonEnd),
    days_until_season_start:
      currentDate && seasonStart
        ? positiveGameDayDiff(currentDate, seasonStart)
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

  const opensOn = addGameDays(
    seasonStart,
    -TRANSFER_WINDOW_DAYS.preseasonOpenLead,
  );
  const closesOn = addGameDays(
    seasonStart,
    TRANSFER_WINDOW_DAYS.postSeasonStartClose,
  );

  let status: TransferWindowStatus = "Closed";
  let daysUntilOpens: number | null = null;
  let daysRemaining: number | null = null;

  if (currentDate < opensOn) {
    daysUntilOpens = diffGameDays(currentDate, opensOn);
  } else if (currentDate <= closesOn) {
    daysRemaining = diffGameDays(currentDate, closesOn);
    status = daysRemaining === 0 ? "DeadlineDay" : "Open";
  }

  return {
    status,
    opens_on: formatGameDate(opensOn),
    closes_on: formatGameDate(closesOn),
    days_until_opens: daysUntilOpens,
    days_remaining: daysRemaining,
  };
}
