import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createTeam } from "../../test-utils/factories";
import type { ClubSquadRatings } from "../../services/teamsService";
import ChosenClubCard from "./ChosenClubCard";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params?.amount !== undefined ? `${key}:${params.amount}` : key,
    i18n: { language: "en" },
  }),
}));

vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  return cannedPositionsModule();
});

const TEAM = createTeam({
  id: "a",
  name: "Northbridge Athletic",
  city: "Northbridge",
  stadium_name: "North Road",
  stadium_capacity: 18400,
  founded_year: 1899,
});

const FACTS: ClubSquadRatings = {
  team_id: "a",
  average_rating: 74,
  players: [],
  stadium_name: "North Road",
  stadium_capacity: 18400,
  transfer_budget: 8_500_000,
  weekly_wage_budget: 210_000,
  balance: 12_100_000,
  colors: { primary: "#991b1b", secondary: "#ffffff" },
  founded_year: 1899,
  competitions: [
    {
      id: "d1",
      name: "First Division",
      name_key: null,
      season: 2026,
      kind: "League",
      scope: "Domestic",
    },
    {
      id: "cup",
      name: "Swedish Cup",
      name_key: null,
      season: 2026,
      kind: "Cup",
      scope: "Domestic",
    },
  ],
  key_players: [
    { player_id: "p1", name: "E. Sandvall", position: "Goalkeeper", ovr: 80 },
    { player_id: "p2", name: "J. Berg", position: "CenterBack", ovr: 77 },
  ],
  reputation_key: "teamSelect.repStrong",
  opening_expectation: { target_position: 3, label_key: "teamSelect.expectations.continental" },
};
const AGES: Record<string, number> = { p1: 29 };

describe("ChosenClubCard", () => {
  /**
   * Given no club is chosen
   * When the card is drawn
   * Then it says so and what picking a club shows, and draws none of a club's facts.
   */
  it("says no club is chosen yet", () => {
    render(<ChosenClubCard team={null} average={null} facts={undefined} ageOf={() => null} />);

    expect(screen.getByText("teamSelect.card.emptyTitle")).toBeInTheDocument();
    expect(screen.getByText("teamSelect.card.emptyHint")).toBeInTheDocument();
    expect(screen.queryByText("teamSelect.card.transferBudget")).toBeNull();
  });

  /**
   * Given a club chosen and the backend's facts for it
   * When the card is drawn
   * Then it shows the reputation word and the board's opening expectation the backend names, its
   *      budgets (the wage budget already weekly, its unit stated once), balance and stadium, its
   *      key players and the competitions it contests, all the backend's.
   */
  it("shows the backend's facts for the chosen club", () => {
    render(
      <ChosenClubCard team={TEAM} average={74} facts={FACTS} ageOf={(id) => AGES[id] ?? null} />,
    );

    expect(screen.getByRole("heading", { name: "Northbridge Athletic" })).toBeInTheDocument();
    // The squad rating, painted in the tier the backend gives it.
    expect(screen.getAllByTitle(/common\.overallRating/).length).toBeGreaterThan(0);
    expect(screen.getByText("teamSelect.card.founded")).toBeInTheDocument();
    expect(screen.getByText("North Road · 18,400")).toBeInTheDocument();
    // The word and the expectation are the keys the backend names, translated here and judged nowhere.
    expect(screen.getByText("teamSelect.card.boardExpects").nextSibling).toHaveTextContent(
      "teamSelect.expectations.continental",
    );
    expect(screen.getByText("teamSelect.reputation").nextSibling).toHaveTextContent(
      "teamSelect.repStrong",
    );
    const wage = screen.getByText("teamSelect.card.wageBudget").nextSibling;
    expect(wage?.textContent).toMatch(/\/wk$/);
    expect(wage?.textContent).not.toMatch(/week/i);
    expect(screen.getByText("teamSelect.card.balance").nextSibling).toHaveTextContent("12.1M");
    expect(screen.getByText("E. Sandvall")).toBeInTheDocument();
    expect(screen.getByText("J. Berg")).toBeInTheDocument();
    expect(screen.getByRole("list", { name: "teamSelect.competitions" })).toHaveTextContent(
      "First DivisionSwedish Cup",
    );
  });

  /**
   * Given a key player the game state does not hold (so his age is unknown)
   * When the card is drawn
   * Then his row has no age, and the others keep theirs.
   */
  it("leaves out the age of a key player the game state does not hold", () => {
    render(
      <ChosenClubCard team={TEAM} average={74} facts={FACTS} ageOf={(id) => AGES[id] ?? null} />,
    );

    const rows = screen
      .getAllByRole("listitem")
      .filter(
        (item) => item.textContent?.includes("Sandvall") || item.textContent?.includes("Berg"),
      );
    expect(rows[0]).toHaveTextContent("29");
    expect(rows[1]).not.toHaveTextContent("common.age");
  });

  /**
   * Given a club chosen before the backend has answered
   * When the card is drawn
   * Then every backend fact reads a dash, and no number is invented from the game state.
   */
  it("reads a dash for every fact until the backend answers", () => {
    render(<ChosenClubCard team={TEAM} average={null} facts={undefined} ageOf={() => null} />);

    expect(screen.queryByTitle(/common\.overallRating/)).toBeNull();
    expect(screen.getByText("teamSelect.card.boardExpects").nextSibling).toHaveTextContent("—");
    expect(screen.getByText("teamSelect.reputation").nextSibling).toHaveTextContent("—");
    expect(screen.getByText("teamSelect.card.transferBudget").nextSibling).toHaveTextContent("—");
    expect(screen.getByText("teamSelect.card.stadium").nextSibling).toHaveTextContent("—");
  });
});
