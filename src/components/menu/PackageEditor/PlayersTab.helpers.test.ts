import { describe, expect, it } from "vitest";

import {
  filterPlayerRows,
  matchesPositionFilter,
  positionFilterGroups,
} from "./PlayersTab.helpers";
import { emptyPlayer } from "./helpers";
import en from "../../../i18n/locales/en.json";
import type { PlayerDef, Position } from "./types";

function player(overrides: Partial<PlayerDef> = {}): PlayerDef {
  return { ...emptyPlayer(), ...overrides };
}

const NO_TEAMS = new Map<string, string>();

describe("filterPlayerRows", () => {
  it("carries the original array index, which is what edit and delete address", () => {
    const players = [
      player({ id: "a", name: "Ana" }),
      player({ id: "b", name: "Bo" }),
      player({ id: "c", name: "Cyd" }),
      player({ id: "d", name: "Bo Reilly" }),
    ];

    const { filtered } = filterPlayerRows({ players, query: "bo", teamNames: NO_TEAMS });

    expect(filtered.map(({ i }) => i)).toEqual([1, 3]);
  });

  it("scopes to youth or to seniors, still with unfiltered indices", () => {
    const players = [
      player({ id: "a", name: "Ana" }),
      player({ id: "b", name: "Bo", youth: true }),
      player({ id: "c", name: "Cyd" }),
    ];

    const youth = filterPlayerRows({ players, youthOnly: true, query: "", teamNames: NO_TEAMS });
    const seniors = filterPlayerRows({ players, youthOnly: false, query: "", teamNames: NO_TEAMS });

    expect(youth.filtered.map(({ i }) => i)).toEqual([1]);
    expect(seniors.filtered.map(({ i }) => i)).toEqual([0, 2]);
  });

  it("reports the scoped total separately from the matches", () => {
    const players = [
      player({ id: "a", name: "Ana" }),
      player({ id: "b", name: "Bo" }),
    ];

    const { scoped, filtered } = filterPlayerRows({
      players,
      query: "ana",
      teamNames: NO_TEAMS,
    });

    expect(scoped).toHaveLength(2);
    expect(filtered).toHaveLength(1);
  });

  it("matches a club by the name shown in the row, not only by its id", () => {
    // The subtitle reads "Northshire FC" while the record stores "nsfc", so
    // searching for what is on screen used to find nothing.
    const players = [
      player({ id: "a", name: "Ana", club: "nsfc" }),
      player({ id: "b", name: "Bo", club: "hav" }),
    ];
    const teamNames = new Map([["nsfc", "Northshire FC"], ["hav", "Havenport"]]);

    const { filtered } = filterPlayerRows({ players, query: "northshire", teamNames });

    expect(filtered.map(({ player: p }) => p.id)).toEqual(["a"]);
  });

  it("still matches the raw id, position and nationality", () => {
    const players = [
      player({ id: "keeper-one", name: "Ana", position: "Goalkeeper" as Position, nationality: "BRA" }),
      player({ id: "striker-one", name: "Bo", position: "Striker" as Position, nationality: "ARG" }),
    ];

    const byId = filterPlayerRows({ players, query: "keeper-o", teamNames: NO_TEAMS });
    const byPosition = filterPlayerRows({ players, query: "goalkeeper", teamNames: NO_TEAMS });
    const byNationality = filterPlayerRows({ players, query: "arg", teamNames: NO_TEAMS });

    expect(byId.filtered.map(({ player: p }) => p.id)).toEqual(["keeper-one"]);
    expect(byPosition.filtered.map(({ player: p }) => p.id)).toEqual(["keeper-one"]);
    expect(byNationality.filtered.map(({ player: p }) => p.id)).toEqual(["striker-one"]);
  });

  it("falls back to the first and last name when there is no display name", () => {
    const players = [player({ id: "a", firstName: "Ana", lastName: "Reyes" })];

    expect(filterPlayerRows({ players, query: "reyes", teamNames: NO_TEAMS }).filtered).toHaveLength(1);
  });
});

describe("matchesPositionFilter", () => {
  it("lets everything through when no position is chosen", () => {
    expect(matchesPositionFilter("Striker", "All")).toBe(true);
    expect(matchesPositionFilter("Goalkeeper", "All")).toBe(true);
  });

  it("matches one exact position", () => {
    expect(matchesPositionFilter("CenterBack", "CenterBack")).toBe(true);
    expect(matchesPositionFilter("LeftBack", "CenterBack")).toBe(false);
  });

  it("matches every position in a group", () => {
    expect(matchesPositionFilter("CenterBack", "group:Defender")).toBe(true);
    expect(matchesPositionFilter("LeftWingBack", "group:Defender")).toBe(true);
    expect(matchesPositionFilter("Striker", "group:Defender")).toBe(false);
  });

  it("keeps a group apart from the bare enum value of the same name", () => {
    // Some packages set the broad "Defender" literally. Choosing that should
    // find those players, not every defender in the game.
    expect(matchesPositionFilter("Defender", "Defender")).toBe(true);
    expect(matchesPositionFilter("CenterBack", "Defender")).toBe(false);
    expect(matchesPositionFilter("Defender", "group:Defender")).toBe(true);
  });
});

describe("positionFilterGroups", () => {
  it("names every option and section with a key that exists", () => {
    // These keys are built by interpolation, so frontendKeyCoverage — which
    // parses literal t("…") calls — cannot see them. Without this, deleting or
    // renaming one ships a raw key string as a dropdown label in every locale
    // with the whole suite green.
    const groups = positionFilterGroups();
    const keys = groups.flatMap((group) => [
      group.labelKey,
      ...group.options.map((option) => option.labelKey),
    ]);

    for (const key of keys) {
      const value = key.split(".").reduce<unknown>(
        (node, part) => (node as Record<string, unknown> | undefined)?.[part],
        en as unknown,
      );
      expect(typeof value, `missing en.json key: ${key}`).toBe("string");
    }
  });

  it("offers the four groups and all seventeen positions, each once", () => {
    const groups = positionFilterGroups();
    const values = groups.flatMap((group) => group.options.map((option) => option.value));

    expect(groups).toHaveLength(2);
    expect(values.filter((v) => v.startsWith("group:"))).toHaveLength(4);
    expect(values).toHaveLength(21);
    expect(new Set(values).size).toBe(21);
  });
});

describe("filterPlayerRows with a position filter", () => {
  const squad = [
    player({ id: "gk", name: "Ana", position: "Goalkeeper" as Position }),
    player({ id: "cb", name: "Bo", position: "CenterBack" as Position }),
    player({ id: "lb", name: "Bo Reilly", position: "LeftBack" as Position }),
    player({ id: "st", name: "Cyd", position: "Striker" as Position }),
  ];

  it("narrows to a group, keeping unfiltered indices", () => {
    const { filtered } = filterPlayerRows({
      players: squad,
      positionFilter: "group:Defender",
      query: "",
      teamNames: NO_TEAMS,
    });

    expect(filtered.map(({ i }) => i)).toEqual([1, 2]);
  });

  it("narrows to one exact position", () => {
    const { filtered } = filterPlayerRows({
      players: squad,
      positionFilter: "Goalkeeper",
      query: "",
      teamNames: NO_TEAMS,
    });

    expect(filtered.map(({ player: p }) => p.id)).toEqual(["gk"]);
  });

  it("applies the position and the search together", () => {
    const { filtered } = filterPlayerRows({
      players: squad,
      positionFilter: "group:Defender",
      query: "bo r",
      teamNames: NO_TEAMS,
    });

    expect(filtered.map(({ player: p }) => p.id)).toEqual(["lb"]);
  });

  it("counts the scoped total before the position filter narrows it", () => {
    const { scoped, filtered } = filterPlayerRows({
      players: squad,
      positionFilter: "Goalkeeper",
      query: "",
      teamNames: NO_TEAMS,
    });

    expect(scoped).toHaveLength(4);
    expect(filtered).toHaveLength(1);
  });
});
