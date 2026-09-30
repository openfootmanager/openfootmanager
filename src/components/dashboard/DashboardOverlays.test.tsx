import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../i18n";
import DashboardOverlays from "./DashboardOverlays";

describe("DashboardOverlays", () => {
  it("shows a named cup fixture in the match confirmation modal", async () => {
    await i18nReady;
    render(
      <DashboardOverlays
        blockerModal={null}
        currentModeMeta={{
          buttonColorClass: "bg-primary-500",
          desc: "Play the match",
          dropdownColorClass: "bg-primary-500",
          icon: null,
          label: "Live",
        }}
        isAdvancing={false}
        handleConfirmMatch={vi.fn()}
        handleExitToMenu={vi.fn()}
        handleNavigate={vi.fn()}
        handleCloseQuit={vi.fn()}
        isExitingToMenu={false}
        matchMode="live"
        setBlockerModal={vi.fn()}
        setShowCloseConfirm={vi.fn()}
        setShowExitConfirm={vi.fn()}
        setShowMatchConfirm={vi.fn()}
        showCloseConfirm={false}
        showExitConfirm={false}
        showMatchConfirm={true}
        teams={[]}
        todayMatchFixture={{
          id: "cup-match",
          competition_id: "cup-1",
          competition: "Cup",
          matchday: 1,
          date: "2026-08-01",
          home_team_id: "home1",
          away_team_id: "away1",
          status: "Scheduled",
          result: null,
        }}
        todayMatchCompetitionName="National Cup"
      />,
    );

    expect(screen.getByText("National Cup")).toBeInTheDocument();
  });
});
