import { describe, expect, it } from "vitest";

import { shortLabels } from "./overviewModel";

describe("shortLabels", () => {
  /**
   * Given the eight attribute names of a Latin-script language
   * When they are shortened for the radar
   * Then each is its first three letters, upper-cased
   */
  it("takes the first three letters of each name", () => {
    expect(shortLabels(["Pace", "Shooting", "Aerial"])).toEqual(["PAC", "SHO", "AER"]);
  });

  /**
   * Given two names that begin alike (the Russian "Игра на выходах" and "Игра головой")
   * When they are shortened
   * Then each is lengthened until it differs from the other, so no two axes carry the same label
   */
  it("lengthens names that would collide", () => {
    const labels = shortLabels(["Игра на выходах", "Игра головой", "Реакция"]);

    expect(new Set(labels).size).toBe(3);
    expect(labels[2]).toBe("РЕА");
  });

  /**
   * Given Chinese names, which carry a word in two characters
   * When they are shortened
   * Then each keeps its first two characters
   */
  it("keeps two characters of a Chinese name", () => {
    expect(shortLabels(["速度", "射门", "制空权"])).toEqual(["速度", "射门", "制空"]);
  });
});
