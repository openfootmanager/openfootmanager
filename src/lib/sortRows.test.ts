import { describe, expect, it } from "vitest";

import { nextSort, sortRows } from "./sortRows";

interface Club {
  name: string;
  average: number | null;
}

const clubs: Club[] = [
  { name: "Beta", average: 70 },
  { name: "Withheld A", average: null },
  { name: "alpha", average: 64 },
  { name: "Gamma", average: 70 },
  { name: "Withheld B", average: null },
];

const names = (rows: readonly Club[]) => rows.map((club) => club.name);

describe("sortRows", () => {
  /**
   * Given clubs whose average is known for some and withheld (null) for others
   * When they are sorted by average, low to high and then high to low
   * Then the known averages run in that order, and the withheld ones come after them both times,
   *      so a club the manager doesn't know never heads the column.
   */
  it("puts rows with no value last in both directions", () => {
    expect(names(sortRows(clubs, (club) => club.average, "asc", "en"))).toEqual([
      "alpha",
      "Beta",
      "Gamma",
      "Withheld A",
      "Withheld B",
    ]);
    expect(names(sortRows(clubs, (club) => club.average, "desc", "en"))).toEqual([
      "Beta",
      "Gamma",
      "alpha",
      "Withheld A",
      "Withheld B",
    ]);
  });

  /**
   * Given two clubs with the same average, Beta listed before Gamma
   * When they are sorted by average in either direction
   * Then Beta stays ahead of Gamma: a tie keeps the order the list arrived in (the backend's).
   */
  it("keeps tied rows in the order they arrived, in either direction", () => {
    const tied = (direction: "asc" | "desc") =>
      names(sortRows(clubs, (club) => club.average, direction, "en")).filter((name) =>
        ["Beta", "Gamma"].includes(name),
      );
    expect(tied("asc")).toEqual(["Beta", "Gamma"]);
    expect(tied("desc")).toEqual(["Beta", "Gamma"]);
  });

  /**
   * Given names in mixed case
   * When they are sorted by name
   * Then they run in the reader's alphabetical order, not by character code ("alpha" first).
   */
  it("sorts text alphabetically, ignoring case", () => {
    expect(names(sortRows(clubs, (club) => club.name, "asc", "en"))).toEqual([
      "alpha",
      "Beta",
      "Gamma",
      "Withheld A",
      "Withheld B",
    ]);
  });

  /**
   * Given "Zenit" and "Örebro"
   * When they are sorted by name in Swedish, and then in German
   * Then Swedish puts Ö after Z, as its alphabet does, and German puts it with O: the order is the
   *      game language's, not the machine's.
   */
  it("sorts text in the alphabet of the language it is given", () => {
    const pair = [{ name: "Zenit" }, { name: "Örebro" }];
    const byName = (locale: string) =>
      sortRows(pair, (club) => club.name, "asc", locale).map((club) => club.name);
    expect(byName("sv")).toEqual(["Zenit", "Örebro"]);
    expect(byName("de")).toEqual(["Örebro", "Zenit"]);
  });

  /**
   * Given "Málaga" listed before "Malaga"
   * When they are sorted by name, up and then down
   * Then they take the language's order for an accent (the plain letter first going up), not the
   *      order they arrived in, as they would if an accent counted for nothing.
   */
  it("tells names apart by their accents, in the language's order", () => {
    const pair = [{ name: "Málaga" }, { name: "Malaga" }];
    const byName = (direction: "asc" | "desc") =>
      sortRows(pair, (club) => club.name, direction, "es").map((club) => club.name);
    expect(byName("asc")).toEqual(["Malaga", "Málaga"]);
    expect(byName("desc")).toEqual(["Málaga", "Malaga"]);
  });

  /**
   * Given a list
   * When it is sorted
   * Then the list itself is left as it was: a sort returns a new one.
   */
  it("leaves the list it was given untouched", () => {
    const before = names(clubs);
    sortRows(clubs, (club) => club.average, "desc", "en");
    expect(names(clubs)).toEqual(before);
  });
});

describe("nextSort", () => {
  /**
   * Given a table sorted by average, high to low
   * When the average header is pressed again
   * Then the order turns round, low to high.
   */
  it("turns the order round when the sorted column is picked again", () => {
    expect(nextSort({ key: "average", direction: "desc" }, "average", "desc")).toEqual({
      key: "average",
      direction: "asc",
    });
    expect(nextSort({ key: "average", direction: "asc" }, "average", "desc")).toEqual({
      key: "average",
      direction: "desc",
    });
  });

  /**
   * Given a table sorted by average, low to high
   * When the name header is pressed, whose first direction is A to Z, and then the reputation
   *      header, whose first direction is high to low
   * Then each starts in its own first direction, not the one the table had.
   */
  it("starts a newly picked column in that column's first direction", () => {
    expect(nextSort({ key: "average", direction: "asc" }, "name", "asc")).toEqual({
      key: "name",
      direction: "asc",
    });
    expect(nextSort({ key: "average", direction: "asc" }, "reputation", "desc")).toEqual({
      key: "reputation",
      direction: "desc",
    });
  });
});
