import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";

import i18n, { i18nReady } from "../../i18n";
import type { CareerEntry } from "../../store/gameStore";
import PlayerProfileCareerHistoryCard from "./PlayerProfileCareerHistoryCard";

const entry = (overrides: Partial<CareerEntry>): CareerEntry => ({
  season: 2026,
  team_id: "team-1",
  team_name: "Alpha FC",
  appearances: 10,
  goals: 2,
  assists: 1,
  ...overrides,
});

/** The real translator for one locale: the card is handed `t`, and a stub would hide a missing key. */
function translatorFor(locale: string) {
  const fixed = i18n.getFixedT(locale);
  return (key: string, options?: Record<string, string | number>) => String(fixed(key, options));
}

describe("PlayerProfileCareerHistoryCard", () => {
  beforeAll(async () => {
    await i18nReady;
    await i18n.loadLanguages("es");
  });

  /**
   * Given a season at no club (team id empty), whose stored name is the old English "Free Agent"
   * When the card is read in Spanish
   * Then the row says "Agente Libre", the player's language, not the stored literal.
   */
  it("words a season at no club in the player's language", () => {
    render(
      <PlayerProfileCareerHistoryCard
        career={[entry({ team_id: "", team_name: "Free Agent" })]}
        t={translatorFor("es")}
      />,
    );

    expect(screen.getByText("Agente Libre")).toBeInTheDocument();
    expect(screen.queryByText("Free Agent")).toBeNull();
  });

  /**
   * Given a season at no club that an updated game stored without a name
   * When the card is read in Spanish
   * Then the row still says "Agente Libre", not an empty cell.
   */
  it("fills the name of a season stored with none", () => {
    render(
      <PlayerProfileCareerHistoryCard
        career={[entry({ team_id: "", team_name: "" })]}
        t={translatorFor("es")}
      />,
    );

    expect(screen.getByText("Agente Libre")).toBeInTheDocument();
  });

  /**
   * Given a season at a club the package authored by name only (no team id: the club is not in
   *      the package), as a package's career entry may
   * When the card is read in Spanish
   * Then the club's authored name shows: an empty id does not mean "no club".
   */
  it("keeps the authored name of a club the package does not define", () => {
    render(
      <PlayerProfileCareerHistoryCard
        career={[entry({ team_id: "", team_name: "Juventus" })]}
        t={translatorFor("es")}
      />,
    );

    expect(screen.getByText("Juventus")).toBeInTheDocument();
    expect(screen.queryByText("Agente Libre")).toBeNull();
  });

  /**
   * Given a season at a club
   * When the card is read
   * Then the club's own name shows.
   */
  it("shows a club's own name for a season at a club", () => {
    render(<PlayerProfileCareerHistoryCard career={[entry({})]} t={translatorFor("es")} />);

    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
  });
});
