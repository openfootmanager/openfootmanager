import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import CupRoundSection from "./CupRoundSection";
import type { CupRoundSummary, CupTieResult } from "./types";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) =>
      params ? `${key}:${Object.values(params).join("/")}` : key,
  }),
}));

function tie(overrides: Partial<CupTieResult> = {}): CupTieResult {
  return {
    fixture_id: "t1",
    home_team_id: "a",
    home_team_name: "Alpha",
    away_team_id: "b",
    away_team_name: "Beta",
    home_goals: 1,
    away_goals: 1,
    home_penalties: null,
    away_penalties: null,
    advancing_team_id: "a",
    advancing_team_name: "Alpha",
    ...overrides,
  };
}

function summary(overrides: Partial<CupRoundSummary> = {}): CupRoundSummary {
  return {
    competition_id: "cup",
    competition_name: "National Cup",
    round_name: "Semifinal",
    is_complete: true,
    pending_fixture_count: 0,
    results: [tie()],
    ...overrides,
  };
}

describe("CupRoundSection", () => {
  /** Given a played tie, when rendered, then the score and who went through are shown. */
  it("shows the score and who went through", () => {
    render(<CupRoundSection summary={summary()} />);
    expect(screen.getByText(/National Cup · /)).toBeInTheDocument();
    expect(screen.getByRole("region", { name: /National Cup/ })).toBeInTheDocument();
    expect(screen.getByText("Alpha 1 – 1 Beta")).toBeInTheDocument();
    expect(screen.getByText("match.cupAdvances:Alpha")).toBeInTheDocument();
    expect(screen.queryByText("match.cupTiesPending")).not.toBeInTheDocument();
  });

  /** Given a stored English round name, when rendered, then the heading uses the translated round. */
  it("translates the round name", () => {
    render(<CupRoundSection summary={summary()} />);
    expect(
      screen.getByRole("heading", { name: /tournaments\.rounds\.semifinal/ }),
    ).toBeInTheDocument();
  });

  /** Given a tie settled on penalties, when rendered, then the shootout score is shown. */
  it("shows the shootout score", () => {
    render(
      <CupRoundSection
        summary={summary({ results: [tie({ home_penalties: 4, away_penalties: 5 })] })}
      />,
    );
    expect(screen.getByText("match.cupPenalties:4/5")).toBeInTheDocument();
  });

  /** Given ties still to play, when rendered, then the pending note is shown. */
  it("says when ties are still to be played", () => {
    render(<CupRoundSection summary={summary({ is_complete: false, pending_fixture_count: 1 })} />);
    expect(screen.getByText("match.cupTiesPending")).toBeInTheDocument();
  });
});
