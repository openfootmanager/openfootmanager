import { describe, expect, it } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import type { TrainingGroupData } from "../../services/trainingService";
import {
  buildPlayerGroupMap,
  reassignPlayerTrainingGroup,
  sortTrainingRoster,
} from "./trainingGroupsModel";

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2002-01-01",
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
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
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: "2027-06-30",
    wage: 12000,
    market_value: 350000,
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
  };
}

function createGroup(
  overrides: Partial<TrainingGroupData> = {},
): TrainingGroupData {
  return {
    id: "group-1",
    name: "Group 1",
    focus: "Physical",
    player_ids: [],
    ...overrides,
  };
}

describe("trainingGroupsModel", () => {
  it("builds a player-to-group lookup map", () => {
    const group = createGroup({ id: "group-a", player_ids: ["p1", "p2"] });

    const playerGroupMap = buildPlayerGroupMap([group]);

    expect(playerGroupMap.get("p1")?.id).toBe("group-a");
    expect(playerGroupMap.get("p2")?.id).toBe("group-a");
    expect(playerGroupMap.get("missing")).toBeUndefined();
  });

  it("reassigns a player between groups", () => {
    const groups = [
      createGroup({ id: "group-a", player_ids: ["p1"] }),
      createGroup({ id: "group-b", player_ids: [] }),
    ];

    const updatedGroups = reassignPlayerTrainingGroup(groups, "p1", "group-b");

    expect(updatedGroups[0].player_ids).toEqual([]);
    expect(updatedGroups[1].player_ids).toEqual(["p1"]);
  });

  it("sorts the roster by position order and then by name", () => {
    const roster = [
      createPlayer({ id: "fwd", match_name: "Zane", position: "Forward", natural_position: "Forward" }),
      createPlayer({ id: "def", match_name: "Adam", position: "Defender", natural_position: "Defender" }),
      createPlayer({ id: "mid", match_name: "Ben", position: "Midfielder", natural_position: "Midfielder" }),
      createPlayer({ id: "gk", match_name: "Chris", position: "Goalkeeper", natural_position: "Goalkeeper" }),
    ];

    expect(sortTrainingRoster(roster).map((player) => player.id)).toEqual([
      "gk",
      "def",
      "mid",
      "fwd",
    ]);
  });
});