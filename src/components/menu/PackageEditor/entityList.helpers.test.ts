import { describe, expect, it } from "vitest";

import { buildTeamNameMap, capRows } from "./entityList.helpers";
import { emptyTeam } from "./helpers";
import type { TeamDef } from "./types";

function team(id: string, name: string): TeamDef {
  return { ...emptyTeam(), id, name };
}

describe("buildTeamNameMap", () => {
  it("resolves a club id to its display name", () => {
    const names = buildTeamNameMap([team("nsfc", "Northshire FC"), team("hav", "Havenport")]);

    expect(names.get("nsfc")).toBe("Northshire FC");
    expect(names.get("hav")).toBe("Havenport");
  });

  it("returns an empty map when there are no teams", () => {
    expect(buildTeamNameMap(undefined).size).toBe(0);
    expect(buildTeamNameMap([]).size).toBe(0);
  });

  it("skips teams with no id, which a half-typed record has", () => {
    const names = buildTeamNameMap([team("", "Unnamed"), team("hav", "Havenport")]);

    expect(names.has("")).toBe(false);
    expect(names.size).toBe(1);
  });
});

describe("capRows", () => {
  const rows = Array.from({ length: 500 }, (_, i) => i);
  const nothingSelected = () => false;

  it("shows only the revealed rows and counts the rest", () => {
    const { visible, hiddenCount } = capRows(rows, 50, 50, nothingSelected);

    expect(visible).toHaveLength(50);
    expect(visible[49]).toBe(49);
    expect(hiddenCount).toBe(450);
  });

  it("hides nothing once every row has been revealed", () => {
    const { visible, hiddenCount } = capRows(rows, 500, 50, nothingSelected);

    expect(visible).toHaveLength(500);
    expect(hiddenCount).toBe(0);
  });

  it("stretches to reach a selection just past the edge", () => {
    // Duplicating the last visible row selects the copy one place below it.
    // Without this the form would open on a record the list cannot show.
    const { visible } = capRows(rows, 50, 50, (row) => row === 50);

    expect(visible).toHaveLength(51);
    expect(visible[50]).toBe(50);
  });

  it("does not stretch to a selection far beyond the edge", () => {
    // Clearing a search can leave the selected record thousands of rows down.
    // Following it there would render the whole list and undo the cap.
    const { visible } = capRows(rows, 50, 50, (row) => row === 400);

    expect(visible).toHaveLength(50);
  });

  it("leaves a list shorter than the cap alone", () => {
    const { visible, hiddenCount } = capRows([1, 2, 3], 50, 50, nothingSelected);

    expect(visible).toEqual([1, 2, 3]);
    expect(hiddenCount).toBe(0);
  });
});
