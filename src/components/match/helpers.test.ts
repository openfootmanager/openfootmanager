import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import {
  getPlayerName,
  phaseLabel,
  getEventDisplay,
  getEventTypeLabel,
  resolveMatchFixture,
} from "./helpers";
import i18n, { i18nReady } from "../../i18n";
import { getTeamTalkOptions } from "./types";
import type { MatchSnapshot, EnginePlayerData, EngineTeamData } from "./types";
import type { GameStateData } from "../../store/gameStore";

// ---------------------------------------------------------------------------
// Minimal fixtures
// ---------------------------------------------------------------------------

const makePlayer = (overrides: Partial<EnginePlayerData> = {}): EnginePlayerData => {
  const { ovr = 70, ...rest } = overrides;

  return {
    id: "p1",
    name: "Test Player",
    position: "Midfielder",
    ovr,
    condition: 100,
    pace: 70, stamina: 70, strength: 70, agility: 70,
    passing: 70, shooting: 70, tackling: 70, dribbling: 70,
    defending: 70, positioning: 70, vision: 70, decisions: 70,
    composure: 50, aggression: 50, teamwork: 50,
    leadership: 50, handling: 30, reflexes: 30, aerial: 50,
    traits: [],
    ...rest,
  };
};

const makeTeam = (overrides: Partial<EngineTeamData> = {}): EngineTeamData => ({
  id: "team1",
  name: "Test FC",
  formation: "4-4-2",
  play_style: "Balanced",
  players: [],
  ...overrides,
});

const makeSnapshot = (overrides: Partial<MatchSnapshot> = {}): MatchSnapshot => ({
  phase: "FirstHalf",
  current_minute: 25,
  home_score: 0,
  away_score: 0,
  possession: "Home",
  ball_zone: "Midfield",
  home_team: makeTeam({ id: "home1", players: [makePlayer({ id: "h1", name: "Home Player" })] }),
  away_team: makeTeam({ id: "away1", players: [makePlayer({ id: "a1", name: "Away Player" })] }),
  home_bench: [makePlayer({ id: "hb1", name: "Home Bench" })],
  away_bench: [makePlayer({ id: "ab1", name: "Away Bench" })],
  home_possession_pct: 55,
  away_possession_pct: 45,
  events: [],
  home_subs_made: 0,
  away_subs_made: 0,
  max_subs: 3,
  home_set_pieces: { free_kick_taker: null, corner_taker: null, penalty_taker: null, captain: null },
  away_set_pieces: { free_kick_taker: null, corner_taker: null, penalty_taker: null, captain: null },
  substitutions: [],
  allows_extra_time: false,
  home_yellows: {},
  away_yellows: {},
  sent_off: [],
  ...overrides,
});

beforeAll(async () => {
  await i18nReady;
});

let defaultLanguage = "en";

beforeAll(() => {
  defaultLanguage = i18n.resolvedLanguage || i18n.language || "en";
});

afterEach(async () => {
  await i18n.changeLanguage(defaultLanguage);
});

// ---------------------------------------------------------------------------
// getPlayerName
// ---------------------------------------------------------------------------

describe("getPlayerName", () => {
  const snapshot = makeSnapshot();

  it("finds player in home team", () => {
    expect(getPlayerName(snapshot, "h1")).toBe("Home Player");
  });

  it("finds player in away team", () => {
    expect(getPlayerName(snapshot, "a1")).toBe("Away Player");
  });

  it("finds player on home bench", () => {
    expect(getPlayerName(snapshot, "hb1")).toBe("Home Bench");
  });

  it("finds player on away bench", () => {
    expect(getPlayerName(snapshot, "ab1")).toBe("Away Bench");
  });

  it("returns empty string for null id", () => {
    expect(getPlayerName(snapshot, null)).toBe("");
  });

  it("returns the id when player not found", () => {
    expect(getPlayerName(snapshot, "unknown_id")).toBe("unknown_id");
  });
});

describe("resolveMatchFixture", () => {
  const gameState = {
    league: {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [
        {
          id: "fixture-1",
          matchday: 3,
          date: "2026-08-01",
          home_team_id: "home1",
          away_team_id: "away1",
          competition: "League",
          status: "Scheduled",
          result: null,
        },
      ],
      standings: [],
    },
  } as unknown as GameStateData;

  it("resolves the fixture by index when available", () => {
    expect(resolveMatchFixture(gameState, makeSnapshot(), 0)?.id).toBe("fixture-1");
  });

  it("falls back to matching the snapshot teams", () => {
    expect(resolveMatchFixture(gameState, makeSnapshot())?.id).toBe("fixture-1");
  });

  it("returns null when no league fixtures are available", () => {
    expect(resolveMatchFixture({ league: null } as unknown as GameStateData, makeSnapshot())).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// phaseLabel
// ---------------------------------------------------------------------------

describe("phaseLabel", () => {
  it("maps all known phases in english", async () => {
    await i18n.changeLanguage("en");

    expect(phaseLabel("PreKickOff")).toBe("Pre-Match");
    expect(phaseLabel("FirstHalf")).toBe("1st Half");
    expect(phaseLabel("HalfTime")).toBe("Half Time");
    expect(phaseLabel("SecondHalf")).toBe("2nd Half");
    expect(phaseLabel("FullTime")).toBe("Full Time");
    expect(phaseLabel("ExtraTimeFirstHalf")).toBe("ET 1st Half");
    expect(phaseLabel("ExtraTimeHalfTime")).toBe("ET Half Time");
    expect(phaseLabel("ExtraTimeSecondHalf")).toBe("ET 2nd Half");
    expect(phaseLabel("ExtraTimeEnd")).toBe("ET End");
    expect(phaseLabel("PenaltyShootout")).toBe("Penalties");
    expect(phaseLabel("Finished")).toBe("Final");
  });

  it("returns translated phase labels for pt-BR", async () => {
    await i18n.changeLanguage("pt-BR");

    const t = i18n.t.bind(i18n);

    expect(phaseLabel("FirstHalf", t)).toBe("1º Tempo");
    expect(phaseLabel("SecondHalf", t)).toBe("2º Tempo");
    expect(phaseLabel("PenaltyShootout", t)).toBe("Pênaltis");
  });

  it("humanizes unknown phases when no fixed label exists", () => {
    expect(phaseLabel("SomeOtherPhase")).toBe("Some Other Phase");
  });

  it("passes a humanized default value when translating unknown phases", () => {
    const translate = vi.fn((_: string, options?: { defaultValue?: string }) => {
      return options?.defaultValue ?? "";
    });

    expect(phaseLabel("SomeOtherPhase", translate)).toBe("Some Other Phase");
    expect(translate).toHaveBeenCalledWith("match.phases.SomeOtherPhase", {
      defaultValue: "Some Other Phase",
    });
  });
});

// ---------------------------------------------------------------------------
// getEventDisplay
// ---------------------------------------------------------------------------

describe("getEventDisplay", () => {
  it("returns known display for Goal event", () => {
    const display = getEventDisplay({ minute: 10, event_type: "Goal", side: "Home", zone: "Box", player_id: "p1", secondary_player_id: null });
    expect(display.color).toBe("text-accent-700 dark:text-accent-400");
    expect(display.important).toBe(true);
  });

  it("returns known display for YellowCard event", () => {
    const display = getEventDisplay({ minute: 25, event_type: "YellowCard", side: "Away", zone: "Midfield", player_id: "p2", secondary_player_id: null });
    expect(display.color).toBe("text-yellow-400");
    expect(display.important).toBe(true);
  });

  it("returns known display for ShotSaved (non-important)", () => {
    const display = getEventDisplay({ minute: 30, event_type: "ShotSaved", side: "Home", zone: "Box", player_id: "p1", secondary_player_id: null });
    expect(display.color).toBe("text-green-700 dark:text-green-400");
    expect(display.important).toBe(false);
  });

  it("returns default display for unknown event type", () => {
    const display = getEventDisplay({ minute: 1, event_type: "UnknownEvent", side: "Home", zone: "Midfield", player_id: null, secondary_player_id: null });
    expect(display.color).toBe("text-gray-700 dark:text-gray-400");
    expect(display.important).toBe(false);
  });
});

describe("getEventTypeLabel", () => {
  it("returns translated event labels for pt-BR", async () => {
    await i18n.changeLanguage("pt-BR");

    const t = i18n.t.bind(i18n);

    expect(getEventTypeLabel("Goal", t)).toBe("Gol");
    expect(getEventTypeLabel("SecondHalfStart", t)).toBe("Início do 2º Tempo");
    expect(getEventTypeLabel("PenaltyAwarded", t)).toBe("Pênalti Marcado");
  });

  it("falls back to a humanized label for unknown events", async () => {
    await i18n.changeLanguage("en");

    expect(getEventTypeLabel("UnknownEvent")).toBe("Unknown Event");
  });
});

describe("getTeamTalkOptions", () => {
  it("returns the expected english team talk labels and descriptions", async () => {
    await i18n.changeLanguage("en");

    const options = getTeamTalkOptions(i18n.t.bind(i18n));

    expect(options.map((option) => option.id)).toEqual([
      "calm",
      "motivational",
      "assertive",
      "aggressive",
      "praise",
      "disappointed",
    ]);
    expect(options[0]).toEqual({
      id: "calm",
      icon: "calm",
      label: "Stay Calm",
      description: "Keep composure and focus on the game plan.",
    });
    expect(options[5]).toEqual({
      id: "disappointed",
      icon: "disappointed",
      label: "Show Disappointment",
      description: "Express disappointment in their effort.",
    });
  });

  it("returns translated team talk options for pt-BR", async () => {
    await i18n.changeLanguage("pt-BR");

    const options = getTeamTalkOptions(i18n.t.bind(i18n));

    expect(options[0]).toEqual({
      id: "calm",
      icon: "calm",
      label: "Mantenham a calma",
      description: "Peça calma ao time e mantenha o foco no plano de jogo.",
    });
    expect(options[3]).toEqual({
      id: "aggressive",
      icon: "aggressive",
      label: "Incendiar o time",
      description: "Faça uma preleção agressiva e cheia de energia.",
    });
  });

  it("returns translated team talk options for italian", async () => {
    await i18n.changeLanguage("it");

    const options = getTeamTalkOptions(i18n.t.bind(i18n));

    expect(options[1]).toEqual({
      id: "motivational",
      icon: "motivational",
      label: "Motiva",
      description: "Spingi i giocatori a dare il massimo.",
    });
    expect(options[4]).toEqual({
      id: "praise",
      icon: "praise",
      label: "Loda",
      description: "Dì alla squadra che finora è stata eccellente.",
    });
  });
});
