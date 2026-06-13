import { afterAll, beforeAll, describe, expect, it } from "vitest";
import i18n, { i18nReady } from "../../i18n";
import { getCommentary } from "./commentary";
import type { MatchEvent, MatchSnapshot, EnginePlayerData } from "./types";

const makePlayer = (id: string, name: string): EnginePlayerData =>
  ({ id, name, position: "FW" } as unknown as EnginePlayerData);

const snapshot = (events: MatchEvent[] = []): MatchSnapshot =>
  ({
    home_team: { id: "h", name: "Home FC", players: [makePlayer("p1", "Haaland")] },
    away_team: { id: "a", name: "Away FC", players: [makePlayer("p2", "Mbappe"), makePlayer("p3", "Marquinhos")] },
    home_bench: [],
    away_bench: [],
    events,
  } as unknown as MatchSnapshot);

const goal = (minute: number, player_id: string): MatchEvent => ({
  minute,
  event_type: "Goal",
  side: "Home",
  zone: "AwayBox",
  player_id,
  secondary_player_id: null,
  detail: { Goal: { context: "Extends" } },
});

let previousLanguage: string;

beforeAll(async () => {
  await i18nReady;
  previousLanguage = i18n.language;
  await i18n.changeLanguage("en");
});

afterAll(async () => {
  await i18n.changeLanguage(previousLanguage);
});

describe("getCommentary", () => {
  it("returns null for non-key events", () => {
    const evt: MatchEvent = {
      minute: 5,
      event_type: "PassCompleted",
      side: "Home",
      zone: "Midfield",
      player_id: "p1",
      secondary_player_id: null,
    };
    expect(getCommentary(evt, snapshot(), i18n.t.bind(i18n))).toBeNull();
  });

  it("produces a non-empty headline and line for a goal", () => {
    const evt = goal(10, "p1");
    const result = getCommentary(evt, snapshot([evt]), i18n.t.bind(i18n));
    expect(result).not.toBeNull();
    expect(result!.headline.length).toBeGreaterThan(0);
    expect(result!.line.length).toBeGreaterThan(0);
    expect(result!.line).toContain("Haaland");
  });

  it("is deterministic — same event yields the same line", () => {
    const evt = goal(10, "p1");
    const a = getCommentary(evt, snapshot([evt]), i18n.t.bind(i18n));
    const b = getCommentary(evt, snapshot([evt]), i18n.t.bind(i18n));
    expect(a).toEqual(b);
  });

  it("uses the brace variant for a player's second goal", () => {
    const g1 = goal(10, "p1");
    const g2 = goal(40, "p1");
    const result = getCommentary(g2, snapshot([g1, g2]), i18n.t.bind(i18n));
    expect(result!.line.toLowerCase()).toMatch(/brace|two/);
  });

  it("never leaks unresolved interpolation tokens", () => {
    const evt: MatchEvent = {
      minute: 22,
      event_type: "Foul",
      side: "Away",
      zone: "Midfield",
      player_id: "p3",
      secondary_player_id: "p2",
      detail: { Foul: { severity: "Hard" } },
    };
    const result = getCommentary(evt, snapshot([evt]), i18n.t.bind(i18n));
    expect(result!.line).not.toMatch(/\{\{.*?\}\}/);
  });

  it("falls back to the base key when detail is absent (penalty goal)", () => {
    const evt: MatchEvent = {
      minute: 50,
      event_type: "PenaltyGoal",
      side: "Home",
      zone: "AwayBox",
      player_id: "p1",
      secondary_player_id: null,
    };
    const result = getCommentary(evt, snapshot([evt]), i18n.t.bind(i18n));
    expect(result).not.toBeNull();
    expect(result!.line.length).toBeGreaterThan(0);
  });

  it("uses the hat-trick variant and headline for a player's third goal", () => {
    const g1 = goal(10, "p1");
    const g2 = goal(40, "p1");
    const g3 = goal(70, "p1");
    const result = getCommentary(g3, snapshot([g1, g2, g3]), i18n.t.bind(i18n));
    expect(result!.headline).toBe("HAT-TRICK!");
    expect(result!.line.toLowerCase()).toMatch(/hat-trick|three/);
  });

  it("falls back from a missing variant key to the base key", () => {
    // ShotBlocked has a "bigChance" variant in en.json but NO "speculative"
    // variant, so a Speculative-danger blocked shot must fall back to the base
    // ShotBlocked commentary rather than returning null.
    const evt: MatchEvent = {
      minute: 33,
      event_type: "ShotBlocked",
      side: "Home",
      zone: "AwayBox",
      player_id: "p1",
      secondary_player_id: null,
      detail: { Shot: { danger: "Speculative" } },
    };
    const result = getCommentary(evt, snapshot([evt]), i18n.t.bind(i18n));
    expect(result).not.toBeNull();
    expect(result!.line.length).toBeGreaterThan(0);
    expect(result!.line).not.toMatch(/\{\{.*?\}\}/);
  });
});
