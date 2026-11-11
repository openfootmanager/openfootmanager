import { describe, expect, it } from "vitest";

import { buildTeamNameMap } from "./entityList.helpers";
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
