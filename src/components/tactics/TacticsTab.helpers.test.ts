import { describe, expect, it } from "vitest";
import type { PlayerData } from "../../store/gameStore";
import { createPositionTable } from "../../lib/positionTable";
import type { FamiliarityTable } from "../../hooks/useFamiliarity";
import type { Familiarity } from "../../store/generated/Familiarity";
import { cannedPositionTable } from "../../test-utils/positionTable";
import type { TacticsLineup } from "../../store/generated/TacticsLineup";
import {
  buildTacticsRoster,
  countOutOfPositionPlayers,
  effectivePlayerRoles,
} from "./TacticsTab.helpers";

const makePlayer = (
  id: string,
  position: string,
  overrides: Partial<PlayerData> = {},
): PlayerData => ({
  id,
  match_name: id,
  full_name: `Player ${id}`,
  date_of_birth: "1998-01-01",
  nationality: "GB",
  position,
  natural_position: position,
  alternate_positions: [],
  training_focus: null,
  attributes: {
    pace: 60,
    stamina: 60,
    strength: 60,
    agility: 60,
    passing: 60,
    shooting: 60,
    tackling: 60,
    dribbling: 60,
    defending: 60,
    positioning: 60,
    vision: 60,
    decisions: 60,
    composure: 60,
    aggression: 60,
    teamwork: 60,
    leadership: 60,
    handling: 60,
    reflexes: 60,
    aerial: 60,
  },
  condition: 100,
  morale: 80,
  injury: null,
  team_id: "team1",
  retired: false,
  contract_end: "2027-06-30",
  wage: 1000,
  market_value: 100000,
  stats: {
    appearances: 0,
    goals: 0,
    assists: 0,
    clean_sheets: 0,
    yellow_cards: 0,
    red_cards: 0,
    avg_rating: 0,
    minutes_played: 0,
  },
  career: [],
  transfer_listed: false,
  loan_listed: false,
  transfer_offers: [],
  traits: [],
  ...overrides,
});

describe("the tactics roster and its lists, by the backend's position table", () => {
  const roster = () => [
    makePlayer("fwd", "Striker", { team_id: "t1", match_day_eligible: true }),
    makePlayer("gk", "Goalkeeper", { team_id: "t1", match_day_eligible: true }),
    makePlayer("def", "CenterBack", { team_id: "t1", match_day_eligible: true }),
  ];

  /**
   * Given a squad listed in no particular order
   * When the tactics roster is built with the loaded table
   * Then the lines run keeper, defence, attack: the order is the table's, not the input's.
   */
  it("orders the roster by the table's lines", () => {
    const ids = buildTacticsRoster(roster(), "t1", cannedPositionTable()).map((p) => p.id);

    expect(ids).toEqual(["gk", "def", "fwd"]);
  });

  /**
   * Given a defender put in the forward line, with the table loaded and not loaded
   * When the out-of-position players are counted
   * Then he is counted once loaded, and nobody is counted before the table arrives.
   */
  it("counts out-of-position starters only once the table has loaded", () => {
    const xi = [makePlayer("def", "CenterBack", { team_id: "t1" })];
    const slots = new Map([["def", "Striker"]]);

    expect(countOutOfPositionPlayers(xi, slots, cannedPositionTable())).toBe(1);
    expect(countOutOfPositionPlayers(xi, slots, createPositionTable([]))).toBe(0);
  });

  describe("when the screen reads the backend's familiarity", () => {
    // A table that says what the test says per player, whatever his own positions are: it proves
    // the helpers read the table and do not grade the player themselves.
    const byPlayer = (words: Record<string, Familiarity>): FamiliarityTable => ({
      loaded: true,
      familiarityOf: (playerId) => words[playerId] ?? null,
    });

    /**
     * Given a centre back in his own slot whom the backend calls out of position there
     * When the out-of-position players are counted with that table
     * Then he is counted: the backend's word wins over his positions.
     */
    it("counts what the backend calls out of position", () => {
      const xi = [makePlayer("cb", "CenterBack", { team_id: "t1" })];
      const slots = new Map([["cb", "CenterBack"]]);

      expect(
        countOutOfPositionPlayers(
          xi,
          slots,
          cannedPositionTable(),
          byPlayer({ cb: "out_of_position" }),
        ),
      ).toBe(1);
      expect(countOutOfPositionPlayers(xi, slots, cannedPositionTable())).toBe(0);
    });
  });
});

describe("effectivePlayerRoles", () => {
  const lineup: TacticsLineup = {
    formation: "4-4-2",
    eleven: [
      { position: "Goalkeeper", player_id: "gk", role: "SweeperKeeper", familiarity: "natural" },
      { position: "LeftBack", player_id: "st", role: "Standard", familiarity: "out_of_position" },
    ],
    substitutes: [],
    reserves: [],
  };

  /**
   * Given a striker whose stored role is Poacher, now standing at left-back where the backend's
   *       lineup says he plays Standard, and a bench player with a stored role
   * When the roles the screen shows are worked out
   * Then the starter's is the lineup's (so the table and the pitch agree on what the match plays),
   *      and the bench player keeps his stored one.
   */
  it("shows the role the match will play, not a stale stored one", () => {
    const roles = effectivePlayerRoles(lineup, { st: "Poacher", gk: "Standard", sub: "Stopper" });

    expect(roles).toEqual({ st: "Standard", gk: "SweeperKeeper", sub: "Stopper" });
  });

  /**
   * Given no lineup yet from the backend
   * When the roles are worked out
   * Then the stored roles stand as they are.
   */
  it("falls back to the stored roles until the lineup has answered", () => {
    expect(effectivePlayerRoles(null, { st: "Poacher" })).toEqual({ st: "Poacher" });
    expect(effectivePlayerRoles(null, undefined)).toBeUndefined();
  });
});
