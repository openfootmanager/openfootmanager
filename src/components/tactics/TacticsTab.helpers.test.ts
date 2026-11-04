import { describe, expect, it } from "vitest";
import type { PlayerData } from "../../store/gameStore";
import { createPositionTable } from "../../lib/positionTable";
import { cannedPositionTable } from "../../test-utils/positionTable";
import {
  buildTacticsRoster,
  countOutOfPositionPlayers,
  filterAndSortTacticsPlayers,
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
   * Given the same lists and a loaded table
   * When the XI list is sorted by position
   * Then it runs keeper, defence, attack too.
   */
  it("sorts a list by position along the table's lines", () => {
    const positionTable = cannedPositionTable();
    const sorted = filterAndSortTacticsPlayers(
      roster(),
      {
        playerSearch: "",
        positionFilter: "All",
        positionTable,
        section: "bench",
        xiActivePosition: new Map(),
      },
      {
        positionTable,
        section: "bench",
        sortDir: "asc",
        sortKey: "pos",
        xiActivePosition: new Map(),
      },
    );

    expect(sorted.map((p) => p.id)).toEqual(["gk", "def", "fwd"]);
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
});
