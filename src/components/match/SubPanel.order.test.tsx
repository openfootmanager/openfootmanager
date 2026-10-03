import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import PreMatchLineup from "./PreMatchLineup";
import { SubPanel } from "./SubPanel";
import { sortByPositionGroup } from "./SubPanel.helpers";
import type { EnginePlayerData, MatchSnapshot } from "./types";

/**
 * #371 — the bench is listed in the same goalkeeper → defence → midfield →
 * attack order as the players on the pitch, so the replacement for a given
 * position is found without hunting. Uses the real i18n hook.
 */

const player = (id: string, position: string, name = id): EnginePlayerData =>
  ({
    id,
    name,
    position,
    ovr: 70,
    condition: 80,
    pace: 70,
    stamina: 70,
    strength: 70,
    agility: 70,
    passing: 70,
    shooting: 70,
    tackling: 70,
    dribbling: 70,
    defending: 70,
    positioning: 70,
    vision: 70,
    decisions: 70,
    composure: 70,
    aggression: 60,
    teamwork: 70,
    leadership: 60,
    handling: 20,
    reflexes: 20,
    aerial: 60,
    traits: [],
    role: "Standard",
  }) as EnginePlayerData;

function snapshotWithBench(bench: EnginePlayerData[]): MatchSnapshot {
  const teamOf = (id: string, players: EnginePlayerData[]) => ({
    id,
    name: id,
    formation: "4-4-2",
    play_style: "Balanced",
    players,
  });
  return {
    phase: "first_half",
    current_minute: 32,
    home_score: 0,
    away_score: 0,
    possession: "Home",
    ball_zone: "MiddleThird",
    home_team: teamOf("home", [player("s1", "Forward"), player("s2", "Goalkeeper")]),
    away_team: teamOf("away", [player("o1", "Midfielder")]),
    home_bench: bench,
    away_bench: [],
    home_possession_pct: 50,
    away_possession_pct: 50,
    events: [],
    home_subs_made: 0,
    away_subs_made: 0,
    max_subs: 5,
    home_set_pieces: {
      free_kick_taker: null,
      corner_taker: null,
      penalty_taker: null,
      captain: null,
    },
    away_set_pieces: {
      free_kick_taker: null,
      corner_taker: null,
      penalty_taker: null,
      captain: null,
    },
    substitutions: [],
    allows_extra_time: false,
    home_yellows: {},
    away_yellows: {},
    sent_off: [],
  } as unknown as MatchSnapshot;
}

describe("sortByPositionGroup", () => {
  /**
   * Given players listed in arbitrary order
   * When sorted by position group
   * Then keepers come first, then defence, midfield and attack, ties by name,
   *      and the input array is not mutated
   */
  it("orders keepers, defenders, midfielders, forwards and does not mutate", () => {
    const input = [
      player("f", "Forward", "Zed"),
      player("m", "Midfielder", "Mia"),
      player("d2", "Defender", "Bea"),
      player("g", "Goalkeeper", "Gus"),
      player("d1", "Defender", "Abe"),
    ];
    const snapshot = input.map((p) => p.id);

    const sorted = sortByPositionGroup(input);

    expect(sorted.map((p) => p.id)).toEqual(["g", "d1", "d2", "m", "f"]);
    expect(input.map((p) => p.id)).toEqual(snapshot);
  });
});

describe("SubPanel bench order", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  /**
   * Given a bench supplied as forward, goalkeeper, midfielder, defender
   * When the substitutions panel is open
   * Then the bench rows read goalkeeper, defender, midfielder, forward
   *      and so do the players on the pitch
   */
  it("lists the bench in the same position order as the players on the pitch", () => {
    const bench = [
      player("b-fwd", "Forward"),
      player("b-gk", "Goalkeeper"),
      player("b-mid", "Midfielder"),
      player("b-def", "Defender"),
    ];

    render(
      <SubPanel
        snapshot={snapshotWithBench(bench)}
        side="Home"
        onSubstitute={vi.fn()}
        onFormationChange={vi.fn()}
        onPlayStyleChange={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const benchIds = screen
      .getAllByTestId(/^sub-panel-bench-/)
      .map((row) => row.getAttribute("data-testid"));
    expect(benchIds).toEqual([
      "sub-panel-bench-b-gk",
      "sub-panel-bench-b-def",
      "sub-panel-bench-b-mid",
      "sub-panel-bench-b-fwd",
    ]);

    const pitchIds = screen
      .getAllByTestId(/^sub-panel-off-/)
      .map((row) => row.getAttribute("data-testid"));
    expect(pitchIds).toEqual(["sub-panel-off-s2", "sub-panel-off-s1"]);
  });
});

describe("PreMatchLineup bench order", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  /**
   * Given a pre-match bench supplied as forward, goalkeeper, defender
   * When the lineup is shown
   * Then the substitutes read goalkeeper, defender, forward
   */
  it("lists the pre-match substitutes by position group", () => {
    const team = snapshotWithBench([]).home_team;

    render(
      <PreMatchLineup
        userTeam={team}
        userBench={[
          player("b-fwd", "Forward"),
          player("b-gk", "Goalkeeper"),
          player("b-def", "Defender"),
        ]}
        oppTeam={snapshotWithBench([]).away_team}
        userColor="#00ff00"
        homeTeamColor="#ff0000"
        awayTeamColor="#0000ff"
        userSide="Home"
        formationNeeds={{ Goalkeeper: 1, Defender: 4, Midfielder: 4, Forward: 2 }}
        selectedStarterId={null}
        isAutoSelecting={false}
        onSelectStarter={vi.fn()}
        onSwap={vi.fn()}
        onAutoSelect={vi.fn()}
      />,
    );

    expect(
      screen.getAllByTestId(/^pre-match-bench-/).map((row) => row.getAttribute("data-testid")),
    ).toEqual(["pre-match-bench-b-gk", "pre-match-bench-b-def", "pre-match-bench-b-fwd"]);
  });
});
