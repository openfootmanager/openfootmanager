import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createTeam } from "../../test-utils/factories";
import ChosenClubCard from "./ChosenClubCard";

vi.mock("react-i18next", () => ({
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

describe("ChosenClubCard", () => {
  /**
   * Given no club is chosen
   * When the card is drawn
   * Then it says so and what picking a club shows, and draws none of a club's facts.
   */
  it("says no club is chosen yet", () => {
    render(<ChosenClubCard team={null} average={null} keyPlayers={[]} competitionNames={[]} />);

    expect(screen.getByText("teamSelect.card.emptyTitle")).toBeInTheDocument();
    expect(screen.getByText("teamSelect.card.emptyHint")).toBeInTheDocument();
    expect(screen.queryByText("teamSelect.card.transferBudget")).toBeNull();
  });

  /**
   * Given a club chosen, with its key players and competitions
   * When the card is drawn
   * Then it names the club and its squad rating, gives its budgets, stadium, key players and
   *      competitions, and reads the board's expectation and the reputation as a dash: those are
   *      the backend's words, and it has not given them.
   */
  it("shows the chosen club's facts, and a dash for the words the backend has not given", () => {
    render(
      <ChosenClubCard
        team={TEAM}
        average={74}
        keyPlayers={[{ id: "p1", name: "E. Sandvall", position: "Goalkeeper", age: 29, ovr: 80 }]}
        competitionNames={["First Division", "Swedish Cup"]}
      />,
    );

    expect(screen.getByRole("heading", { name: "Northbridge Athletic" })).toBeInTheDocument();
    // The squad rating, painted in the tier the backend gives it.
    expect(screen.getAllByTitle(/common\.overallRating/).length).toBeGreaterThan(0);
    expect(screen.getByText("teamSelect.card.founded")).toBeInTheDocument();
    expect(screen.getByText("North Road · 18,400")).toBeInTheDocument();
    // The wage budget states its weekly unit once, as the formatter writes it.
    const wage = screen.getByText("teamSelect.card.wageBudget").nextSibling;
    expect(wage?.textContent).toMatch(/\/wk$/);
    expect(wage?.textContent).not.toMatch(/week/i);
    expect(screen.getByText("E. Sandvall")).toBeInTheDocument();
    expect(screen.getByRole("list", { name: "teamSelect.competitions" })).toHaveTextContent(
      "First DivisionSwedish Cup",
    );
    expect(screen.getByText("teamSelect.card.boardExpects").nextSibling).toHaveTextContent("—");
    expect(screen.getByText("teamSelect.reputation").nextSibling).toHaveTextContent("—");
  });

  it("reads the squad rating as a dash before the backend has rated it", () => {
    render(<ChosenClubCard team={TEAM} average={null} keyPlayers={[]} competitionNames={[]} />);

    expect(screen.queryByTitle(/common\.overallRating/)).toBeNull();
  });
});
