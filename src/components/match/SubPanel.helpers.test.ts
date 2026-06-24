import { describe, expect, it } from "vitest";
import type { EnginePlayerData, MatchSnapshot } from "./types";
import {
  getMatchScenario,
  buildRecommendationReasons,
  buildRecommendedSubstitutions,
} from "./SubPanel.helpers";

function makePlayer(overrides: Partial<EnginePlayerData> = {}): EnginePlayerData {
  return {
    id: "p1",
    name: "Player One",
    position: "Midfielder",
    role: "Midfielder",
    ovr: 70,
    condition: 80,
    pace: 70, stamina: 70, strength: 70, agility: 70,
    passing: 70, shooting: 70, tackling: 70, dribbling: 70,
    defending: 70, positioning: 70, vision: 70, decisions: 70,
    composure: 70, aggression: 70, teamwork: 70, leadership: 70,
    handling: 20, reflexes: 20, aerial: 70,
    traits: [],
    ...overrides,
  };
}

const EMPTY_SET_PIECES = {
  free_kick_taker: null, corner_taker: null, penalty_taker: null, captain: null,
};

function makeSnapshot(overrides: Partial<MatchSnapshot> = {}): MatchSnapshot {
  const starter = makePlayer({ id: "starter-1" });
  const bench = makePlayer({ id: "bench-1" });
  return {
    phase: "InProgress",
    current_minute: 45,
    home_score: 0,
    away_score: 0,
    possession: "Home",
    ball_zone: "Midfield",
    home_team: { id: "home", name: "Home FC", formation: "4-4-2", play_style: "Balanced", players: [starter] },
    away_team: { id: "away", name: "Away FC", formation: "4-4-2", play_style: "Balanced", players: [] },
    home_bench: [bench],
    away_bench: [],
    home_possession_pct: 50,
    away_possession_pct: 50,
    events: [],
    home_subs_made: 0,
    away_subs_made: 0,
    max_subs: 5,
    home_set_pieces: EMPTY_SET_PIECES,
    away_set_pieces: EMPTY_SET_PIECES,
    substitutions: [],
    allows_extra_time: false,
    home_yellows: {},
    away_yellows: {},
    sent_off: [],
    ...overrides,
  };
}

describe("getMatchScenario", () => {
  it("returns steady when score is level and game is early", () => {
    const snap = makeSnapshot({ current_minute: 44, home_score: 1, away_score: 1 });
    expect(getMatchScenario(snap, "Home").id).toBe("steady");
  });

  it("returns protect-lead when winning at or after minute 70", () => {
    const snap = makeSnapshot({ current_minute: 70, home_score: 2, away_score: 1 });
    expect(getMatchScenario(snap, "Home").id).toBe("protect-lead");
    expect(getMatchScenario(snap, "Away").id).toBe("chase-goal");
  });

  it("does not return protect-lead when winning before minute 70", () => {
    const snap = makeSnapshot({ current_minute: 69, home_score: 2, away_score: 1 });
    expect(getMatchScenario(snap, "Home").id).toBe("steady");
  });

  it("returns chase-goal when losing at or after minute 55", () => {
    const snap = makeSnapshot({ current_minute: 55, home_score: 0, away_score: 1 });
    expect(getMatchScenario(snap, "Home").id).toBe("chase-goal");
  });

  it("returns find-winner when drawing at or after minute 70", () => {
    const snap = makeSnapshot({ current_minute: 75, home_score: 0, away_score: 0 });
    expect(getMatchScenario(snap, "Home").id).toBe("find-winner");
  });

  it("includes the recommended play style", () => {
    const protectLead = makeSnapshot({ current_minute: 80, home_score: 1, away_score: 0 });
    expect(getMatchScenario(protectLead, "Home").recommendedPlayStyle).toBe("Defensive");
  });
});

describe("buildRecommendationReasons", () => {
  const baseOff = makePlayer({ condition: 70, ovr: 70 });
  const baseBench = makePlayer({ condition: 80, ovr: 70 });

  it("includes low-fitness when off player condition is 58 or below", () => {
    const reasons = buildRecommendationReasons({
      benchPlayer: baseBench,
      offPlayer: makePlayer({ condition: 58 }),
      scenario: "steady",
      yellowCount: 0,
    });
    expect(reasons).toContain("low-fitness");
  });

  it("does not include low-fitness when condition is 59", () => {
    const reasons = buildRecommendationReasons({
      benchPlayer: baseBench,
      offPlayer: makePlayer({ condition: 59 }),
      scenario: "steady",
      yellowCount: 0,
    });
    expect(reasons).not.toContain("low-fitness");
  });

  it("includes yellow-risk when off player has a yellow card", () => {
    const reasons = buildRecommendationReasons({
      benchPlayer: baseBench,
      offPlayer: baseOff,
      scenario: "steady",
      yellowCount: 1,
    });
    expect(reasons).toContain("yellow-risk");
  });

  it("includes fresh-legs when bench condition is 15+ higher than off player", () => {
    const reasons = buildRecommendationReasons({
      benchPlayer: makePlayer({ condition: 85 }),
      offPlayer: makePlayer({ condition: 70 }),
      scenario: "steady",
      yellowCount: 0,
    });
    expect(reasons).toContain("fresh-legs");
  });

  it("includes upgrade when bench ovr is 3+ higher than off player", () => {
    const reasons = buildRecommendationReasons({
      benchPlayer: makePlayer({ ovr: 73 }),
      offPlayer: makePlayer({ ovr: 70 }),
      scenario: "steady",
      yellowCount: 0,
    });
    expect(reasons).toContain("upgrade");
  });

  it("includes role-match when positions are identical", () => {
    const reasons = buildRecommendationReasons({
      benchPlayer: makePlayer({ position: "Forward" }),
      offPlayer: makePlayer({ position: "Forward" }),
      scenario: "steady",
      yellowCount: 0,
    });
    expect(reasons).toContain("role-match");
  });

  it("includes attacking-boost when bench is a higher-priority position in chase-goal", () => {
    const reasons = buildRecommendationReasons({
      benchPlayer: makePlayer({ position: "Forward" }),
      offPlayer: makePlayer({ position: "Defender" }),
      scenario: "chase-goal",
      yellowCount: 0,
    });
    expect(reasons).toContain("attacking-boost");
  });

  it("includes defensive-cover when bench is a higher-priority position in protect-lead", () => {
    const reasons = buildRecommendationReasons({
      benchPlayer: makePlayer({ position: "Defender" }),
      offPlayer: makePlayer({ position: "Forward" }),
      scenario: "protect-lead",
      yellowCount: 0,
    });
    expect(reasons).toContain("defensive-cover");
  });

  it("caps reasons at 3", () => {
    const reasons = buildRecommendationReasons({
      benchPlayer: makePlayer({ position: "Forward", ovr: 80, condition: 90 }),
      offPlayer: makePlayer({ position: "Defender", ovr: 70, condition: 58 }),
      scenario: "chase-goal",
      yellowCount: 1,
    });
    expect(reasons.length).toBeLessThanOrEqual(3);
  });
});

describe("buildRecommendedSubstitutions", () => {
  it("returns empty array when bench is empty", () => {
    const snap = makeSnapshot({ home_bench: [] });
    expect(buildRecommendedSubstitutions(snap, "Home")).toEqual([]);
  });

  it("returns empty array when no pairing has a reason", () => {
    // Different positions, similar fitness and ovr, no yellows in a steady mid-game scenario
    // → no reason (no role-match, no fresh-legs, no upgrade, no scenario boost)
    const starter = makePlayer({ id: "s1", condition: 80, ovr: 70, position: "Midfielder" });
    const bench = makePlayer({ id: "b1", condition: 82, ovr: 70, position: "Forward" });
    const snap = makeSnapshot({
      current_minute: 45,
      home_score: 0,
      away_score: 0,
      home_team: { id: "h", name: "H", formation: "4-4-2", play_style: "Balanced", players: [starter] },
      home_bench: [bench],
    });
    expect(buildRecommendedSubstitutions(snap, "Home")).toEqual([]);
  });

  it("recommends a substitution for a tired starter with a fresher bench player", () => {
    const tired = makePlayer({ id: "tired", condition: 45, ovr: 70, position: "Midfielder" });
    const fresh = makePlayer({ id: "fresh", condition: 90, ovr: 70, position: "Midfielder" });
    const snap = makeSnapshot({
      home_team: { id: "h", name: "H", formation: "4-4-2", play_style: "Balanced", players: [tired] },
      home_bench: [fresh],
    });
    const subs = buildRecommendedSubstitutions(snap, "Home");
    expect(subs.length).toBeGreaterThan(0);
    expect(subs[0].offId).toBe("tired");
    expect(subs[0].onId).toBe("fresh");
    expect(subs[0].reasons).toContain("low-fitness");
  });

  it("excludes players already subbed off", () => {
    const tired = makePlayer({ id: "tired", condition: 45, ovr: 70 });
    const fresh = makePlayer({ id: "fresh", condition: 90, ovr: 70 });
    const snap = makeSnapshot({
      home_team: { id: "h", name: "H", formation: "4-4-2", play_style: "Balanced", players: [tired] },
      home_bench: [fresh],
      substitutions: [{ minute: 60, side: "Home", player_off_id: "tired", player_on_id: "fresh" }],
    });
    expect(buildRecommendedSubstitutions(snap, "Home")).toEqual([]);
  });
});
