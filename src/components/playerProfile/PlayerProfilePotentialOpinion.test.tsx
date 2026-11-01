import { render, screen, within } from "@testing-library/react";
import { createInstance } from "i18next";
import { describe, expect, it } from "vitest";

import en from "../../i18n/locales/en.json";
import { ratingTierClass } from "../../lib/ratingTier";
import type { Certainty } from "../../store/generated/Certainty";
import type { PotentialOpinion } from "../../store/generated/PotentialOpinion";
import PlayerProfilePotentialOpinion from "./PlayerProfilePotentialOpinion";

const translations = createInstance();
await translations.init({
  lng: "en",
  resources: { en: { translation: en } },
  interpolation: { escapeValue: false },
});
const t = translations.t.bind(translations);

const coaches: PotentialOpinion = {
  low: 76,
  high: 84,
  band: "A",
  certainty: "fairly_sure",
  source: { kind: "coaches" },
};

const scouted = (scoutName: string | null, current: boolean): PotentialOpinion => ({
  ...coaches,
  source: { kind: "scout", scout_name: scoutName, scouted_on: "2026-09-01", current },
});

const renderOpinion = (opinion: PotentialOpinion) =>
  render(<PlayerProfilePotentialOpinion opinion={opinion} locale="en" t={t} />);

const opinionRegion = (name: string) => screen.getByRole("region", { name });

describe("PlayerProfilePotentialOpinion", () => {
  /**
   * Given the coaches' opinion of a player: 76-84, band A, fairly sure
   * When the profile shows it
   * Then it reads as the coaches' opinion, with the band's words, the range and how sure they are,
   *      and nothing about a scout
   */
  it("shows the coaches' opinion with its band, range and certainty", () => {
    renderOpinion(coaches);

    const region = within(opinionRegion("Coaches' opinion"));
    expect(region.getByText("Could become a first-team regular")).toBeInTheDocument();
    expect(region.getByText("Potential 76–84")).toBeInTheDocument();
    expect(region.getByText("Fairly sure")).toBeInTheDocument();
    expect(region.queryByText(/Scouted/)).not.toBeInTheDocument();
  });

  /**
   * Given an opinion in band A
   * When the profile shows it
   * Then the band's words are painted in the backend tier's colour, as every rating is
   */
  it("paints the band's words in the backend tier's colour", () => {
    renderOpinion(coaches);

    expect(screen.getByText("Could become a first-team regular")).toHaveClass(
      ...ratingTierClass("A", "text").split(" "),
    );
  });

  /**
   * Given a current report by Ann Scout, written on 1 Sep 2026
   * When the profile shows her opinion
   * Then it reads as the scout's opinion, naming her and the date, and is not called out of date
   */
  it("names the scout and the date of a current report", () => {
    renderOpinion(scouted("Ann Scout", true));

    const region = within(opinionRegion("Scout's opinion"));
    expect(region.getByText("Scouted by Ann Scout on September 1, 2026")).toBeInTheDocument();
    expect(region.queryByText("Out of date")).not.toBeInTheDocument();
  });

  /**
   * Given a report written before the last season ended
   * When the profile shows its opinion
   * Then it is still shown, marked out of date: an old report, not no opinion
   */
  it("marks an opinion from an out-of-date report", () => {
    renderOpinion(scouted("Ann Scout", false));

    const region = within(opinionRegion("Scout's opinion"));
    expect(region.getByText("Potential 76–84")).toBeInTheDocument();
    expect(region.getByText("Out of date")).toBeInTheDocument();
  });

  /**
   * Given a report from before reports named their scout
   * When the profile shows its opinion
   * Then it gives the date alone
   */
  it("gives the date alone when the report names no scout", () => {
    renderOpinion(scouted(null, true));

    expect(screen.getByText("Scouted on September 1, 2026")).toBeInTheDocument();
  });

  /**
   * Given opinions of each certainty
   * When the profile shows each
   * Then each reads with its own word, one per judging band
   */
  it("words each certainty", () => {
    const words: Record<Certainty, string> = {
      confident: "Confident",
      fairly_sure: "Fairly sure",
      unsure: "Unsure",
      guess: "A guess",
    };
    for (const [certainty, word] of Object.entries(words) as [Certainty, string][]) {
      const { unmount } = renderOpinion({ ...coaches, certainty });
      expect(screen.getByText(word)).toBeInTheDocument();
      unmount();
    }
  });

  /**
   * Given opinions in each band
   * When the profile shows each
   * Then each reads with its own words
   */
  it("words each band", () => {
    const words = {
      S: "Could become one of the best",
      A: "Could become a first-team regular",
      B: "Could be a useful squad player",
      Rest: "Unlikely to go much further",
    } as const;
    for (const [band, word] of Object.entries(words) as [PotentialOpinion["band"], string][]) {
      const { unmount } = renderOpinion({ ...coaches, band });
      expect(screen.getByText(word)).toBeInTheDocument();
      unmount();
    }
  });
});
