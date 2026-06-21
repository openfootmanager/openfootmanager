import { describe, expect, it } from "vitest";
import { FORMATIONS } from "./types";
import { FORMATIONS as TACTICS_FORMATIONS } from "../tactics/TacticsTab.helpers";

describe("FORMATIONS", () => {
  it("includes 5-3-2 and 4-1-4-1 in the match types list", () => {
    expect(FORMATIONS).toContain("5-3-2");
    expect(FORMATIONS).toContain("4-1-4-1");
  });

  it("tactics and match FORMATIONS are identical", () => {
    expect(TACTICS_FORMATIONS).toEqual(FORMATIONS);
  });

  it("all formations are valid strings of the form N-N-N or N-N-N-N", () => {
    for (const f of FORMATIONS) {
      expect(f).toMatch(/^\d+-\d+-\d+(-\d+)?$/);
    }
  });
});
