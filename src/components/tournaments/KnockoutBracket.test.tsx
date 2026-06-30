import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { FixtureData } from "../../store/gameStore";
import KnockoutBracket from "./KnockoutBracket";

function penaltyFixture(): FixtureData {
  return {
    id: "ko-1",
    matchday: 1,
    date: "2026-07-10",
    home_team_id: "home",
    away_team_id: "away",
    competition: "InternationalNation",
    status: "Completed",
    result: {
      home_goals: 1,
      away_goals: 1,
      home_scorers: [],
      away_scorers: [],
      home_penalties: 2,
      away_penalties: 4,
    },
  };
}

function renderBracket(fixture: FixtureData) {
  return render(
    <KnockoutBracket
      rounds={[
        { id: "r1", name: "Final", fixture_ids: [fixture.id], completed: true },
      ]}
      fixtures={[fixture]}
      resolveTeamName={(id) => (id === "home" ? "Brazil" : "France")}
      localizedRoundName={(name) => name}
      roundCompleteLabel="Complete"
      roundInProgressLabel="In progress"
      byeLabel="Bye"
      tbdLabel="TBD"
    />,
  );
}

describe("KnockoutBracket penalty shootouts", () => {
  it("advances the shootout winner even though the goals are level", () => {
    renderBracket(penaltyFixture());

    const slot = screen.getByTestId("tournaments-bracket-ko-1");
    // The away side won 4-2 on penalties: its shootout tally is shown...
    expect(within(slot).getByText("(4)")).toBeInTheDocument();
    expect(within(slot).getByText("(2)")).toBeInTheDocument();
    // ...and it is actually marked as the winner — the level score must not
    // leave both rows neutral (the "wrong team advanced" regression).
    const awayRow = within(slot).getByText("France").closest("div");
    const homeRow = within(slot).getByText("Brazil").closest("div");
    expect(awayRow?.className).toContain("bg-primary-50");
    expect(homeRow?.className).not.toContain("bg-primary-50");
  });

  it("does not show a shootout tally when the tie was settled in normal time", () => {
    const fixture = penaltyFixture();
    fixture.result = {
      home_goals: 2,
      away_goals: 1,
      home_scorers: [],
      away_scorers: [],
    };
    renderBracket(fixture);

    const slot = screen.getByTestId("tournaments-bracket-ko-1");
    expect(within(slot).queryByText(/\(\d+\)/)).not.toBeInTheDocument();
  });
});
