import { describe, expect, it } from "vitest";

import { createPlayer } from "../../../test-utils/factories";
import { attributeColumns } from "./attributesTabModel";

const names = (key: string) => key;

describe("attributeColumns", () => {
  /**
   * Given an outfield player
   * When his attributes are laid out in columns
   * Then they are Technical, Mental and Physical, in that order, each attribute once, with each
   *      column's average
   */
  it("has Technical, Mental and Physical for an outfield player", () => {
    const columns = attributeColumns(createPlayer().attributes, false, names);

    expect(columns.map((column) => column.label)).toEqual([
      "common.attrGroups.technical",
      "common.attrGroups.mental",
      "common.attrGroups.physical",
    ]);
    const keys = columns.flatMap((column) => column.attrs.map((attr) => attr.key));
    expect(new Set(keys).size).toBe(keys.length);
    expect(keys).not.toContain("handling");
    for (const column of columns) {
      const mean = column.attrs.reduce((sum, attr) => sum + attr.value, 0) / column.attrs.length;
      expect(column.average).toBe(Math.round(mean));
    }
  });

  /**
   * Given a goalkeeper
   * When his attributes are laid out
   * Then Goalkeeping replaces Technical: Goalkeeping, Mental and Physical, with handling and
   *      reflexes in the first and no passing, shooting or tackling
   */
  it("has Goalkeeping in place of Technical for a goalkeeper", () => {
    const columns = attributeColumns(createPlayer().attributes, true, names);

    expect(columns.map((column) => column.label)).toEqual([
      "common.attrGroups.goalkeeper",
      "common.attrGroups.mental",
      "common.attrGroups.physical",
    ]);
    expect(columns[0].attrs.map((attr) => attr.key)).toEqual(["handling", "reflexes"]);
    const keys = columns.flatMap((column) => column.attrs.map((attr) => attr.key));
    expect(keys).not.toContain("shooting");
    expect(keys).not.toContain("passing");
  });

  /**
   * Given an outfield player and a goalkeeper
   * When their columns are laid out
   * Then aerial is in Physical, and every attribute the engine has is in exactly one column for the
   *      outfield player and in one column of the goalkeeper's unless it is Technical-only (which a
   *      keeper's profile leaves out)
   */
  it("puts aerial in Physical and every attribute in exactly one column", () => {
    const player = createPlayer();
    const outfield = attributeColumns(player.attributes, false, names);
    const keeper = attributeColumns(player.attributes, true, names);

    const physical = outfield.find((column) => column.label === "common.attrGroups.physical");
    expect(physical?.attrs.map((attr) => attr.key)).toContain("aerial");
    const outfieldKeys = outfield.flatMap((column) => column.attrs.map((attr) => attr.key)).sort();
    expect(outfieldKeys).toEqual(
      Object.keys(player.attributes)
        .filter((key) => key !== "handling" && key !== "reflexes")
        .sort(),
    );
    const keeperKeys = keeper.flatMap((column) => column.attrs.map((attr) => attr.key));
    expect(new Set(keeperKeys).size).toBe(keeperKeys.length);
  });
});
