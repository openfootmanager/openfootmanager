import { render, screen, within } from "@testing-library/react";
import { createInstance } from "i18next";
import { describe, expect, it } from "vitest";

import en from "../../i18n/locales/en.json";
import pt from "../../i18n/locales/pt.json";
import type { AttributeKey } from "../../store/generated/AttributeKey";
import type { PositionRatings } from "../../store/generated/PositionRatings";
import PlayerProfilePositionRatings from "./PlayerProfilePositionRatings";

const translations = createInstance();
await translations.init({
  lng: "en",
  resources: { en: { translation: en }, pt: { translation: pt } },
  interpolation: { escapeValue: false },
});
const tIn = (language: string) => translations.getFixedT(language);

/** The club's own central midfielder: natural 77, can play defensive midfield 74, played at right back 61. */
const ownPlayer: PositionRatings = {
  source: { kind: "live" },
  rows: [
    {
      position: "CentralMidfielder",
      rating: 77,
      familiarity: "natural",
      deployed: false,
      key_attributes: [
        { attribute: "passing", value: 72 },
        { attribute: "vision", value: 68 },
        { attribute: "decisions", value: 70 },
      ],
    },
    {
      position: "DefensiveMidfielder",
      rating: 74,
      familiarity: "can_play",
      deployed: false,
      key_attributes: [
        { attribute: "tackling", value: 66 },
        { attribute: "positioning", value: 71 },
        { attribute: "passing", value: 72 },
      ],
    },
    {
      position: "RightBack",
      rating: 61,
      familiarity: "out_of_position",
      deployed: true,
      key_attributes: [
        { attribute: "tackling", value: 66 },
        { attribute: "defending", value: 58 },
        { attribute: "positioning", value: 71 },
      ],
    },
  ],
};

const renderRatings = (ratings: PositionRatings, language = "en") =>
  render(<PlayerProfilePositionRatings ratings={ratings} locale={language} t={tIn(language)} />);

const rowFor = (positionName: string) =>
  screen.getByText(positionName).closest("li") as HTMLElement;

describe("PlayerProfilePositionRatings", () => {
  /**
   * Given the club's own central midfielder, who can play defensive midfield and is played at
   *       right back
   * When his positions show
   * Then each position comes with his rating there and its familiarity word, the slot he is played
   *      in says so, and each lists the three attributes the position cannot do without
   */
  it("shows his rating and familiarity at each position he plays", () => {
    renderRatings(ownPlayer);

    const section = within(screen.getByRole("region", { name: "Positions" }));
    expect(section.getByText("Central Midfielder")).toBeInTheDocument();
    expect(within(rowFor("Central Midfielder")).getByText("77")).toBeInTheDocument();
    expect(within(rowFor("Central Midfielder")).getByText("Natural position")).toBeInTheDocument();
    expect(within(rowFor("Defensive Midfielder")).getByText("Can play there")).toBeInTheDocument();
    expect(
      within(rowFor("Right Back")).getByText("Out of position (where you play him)"),
    ).toBeInTheDocument();
    expect(
      within(rowFor("Central Midfielder")).getByText("Passing 72 · Vision 68 · Decisions 70"),
    ).toBeInTheDocument();
  });

  /**
   * Given the same three rows
   * When they show
   * Then only "out of position" carries the warning tone: nothing marks his natural position, and
   *      "can play there" is a quiet cue (maintainer, 21 Oct)
   */
  it("warns only where he is out of position", () => {
    renderRatings(ownPlayer);

    expect(screen.getByText("Out of position (where you play him)")).toHaveClass("text-warning");
    expect(screen.getByText("Natural position")).not.toHaveClass("text-warning");
    expect(screen.getByText("Can play there")).not.toHaveClass("text-warning");
  });

  /**
   * Given a scouted player whose report is from before the last season ended
   * When his positions show
   * Then they say they are the report's: its date, and out of date
   */
  it("marks a scouted player's ratings as the report's", () => {
    renderRatings({
      ...ownPlayer,
      source: { kind: "report", scouted_on: "2026-03-10", current: false },
    });

    const section = within(screen.getByRole("region", { name: "Positions" }));
    expect(section.getByText(/^Scouted on /)).toBeInTheDocument();
    expect(section.getByText("Out of date")).toBeInTheDocument();
  });

  /**
   * Given a player rated from his live attributes
   * When his positions show
   * Then no report date or out-of-date mark appears
   */
  it("gives no report date for live ratings", () => {
    renderRatings(ownPlayer);

    expect(screen.queryByText(/^Scouted on /)).not.toBeInTheDocument();
    expect(screen.queryByText("Out of date")).not.toBeInTheDocument();
  });

  /**
   * Given the three rows, in English and in Portuguese
   * When they show
   * Then no word "fit" appears in the rendered text: the game names positions, never a verdict
   *      (maintainer, 21 Oct)
   */
  it.each(["en", "pt"])("never says fit (%s)", (language) => {
    const { container } = renderRatings(ownPlayer, language);

    expect(container.textContent).not.toMatch(/\bfit\b/i);
    expect(container.textContent?.length).toBeGreaterThan(40);
  });

  /**
   * Given every attribute name the backend can send (AttributeKey)
   * When its label is looked up
   * Then English has one: a key attribute can never show as a raw key (the locale coverage test
   *      carries it to every other language)
   */
  it("has a label for every attribute the backend names", () => {
    // A Record over the union: a name missing here, or one the union lacks, fails to compile.
    const everyAttribute: Record<AttributeKey, true> = {
      pace: true,
      stamina: true,
      strength: true,
      agility: true,
      passing: true,
      shooting: true,
      tackling: true,
      dribbling: true,
      defending: true,
      positioning: true,
      vision: true,
      decisions: true,
      composure: true,
      aggression: true,
      teamwork: true,
      leadership: true,
      handling: true,
      reflexes: true,
      aerial: true,
    };
    const labels: Record<string, string> = en.common.attributes;

    for (const attribute of Object.keys(everyAttribute)) {
      expect(labels[attribute], attribute).toEqual(expect.any(String));
    }
  });
});
