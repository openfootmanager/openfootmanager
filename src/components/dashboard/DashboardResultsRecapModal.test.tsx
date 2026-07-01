import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { AdvanceRecap, RecapMatch } from "./advanceRecap";
import DashboardResultsRecapModal from "./DashboardResultsRecapModal";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "match.shootout.shootoutScore")
        return `Penalties: ${params?.h} - ${params?.a}`;
      if (key === "schedule.international") return "International";
      return key;
    },
    i18n: { language: "en" },
  }),
}));

function recapWith(matches: RecapMatch[]): AdvanceRecap {
  return {
    advancedTo: "2026-07-10",
    matches,
    transfers: [],
    news: [],
    inbox: [],
    hasEvents: true,
  };
}

function match(overrides: Partial<RecapMatch> = {}): RecapMatch {
  return {
    date: "2026-07-10",
    competition: "World Cup 2026",
    international: true,
    home_team: "Brazil",
    away_team: "France",
    home_goals: 1,
    away_goals: 1,
    involves_user: false,
    ...overrides,
  };
}

describe("DashboardResultsRecapModal penalty shootouts", () => {
  it("shows the shootout score for a penalty-decided result", () => {
    render(
      <DashboardResultsRecapModal
        recap={recapWith([match({ home_penalties: 4, away_penalties: 2 })])}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("Penalties: 4 - 2")).toBeInTheDocument();
  });

  it("omits the shootout line for a result settled in normal time", () => {
    render(
      <DashboardResultsRecapModal
        recap={recapWith([match({ home_goals: 2, away_goals: 1 })])}
        onClose={vi.fn()}
      />,
    );

    expect(screen.queryByText(/Penalties:/)).not.toBeInTheDocument();
  });
});
