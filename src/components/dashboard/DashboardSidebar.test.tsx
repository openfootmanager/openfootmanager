import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import DashboardSidebar from "./DashboardSidebar";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

function renderSidebar(overrides: Partial<React.ComponentProps<typeof DashboardSidebar>> = {}) {
  return render(
    <DashboardSidebar
      activeTab="Home"
      collapsed={false}
      onNavClick={vi.fn()}
      onToggleCollapse={vi.fn()}
      unreadMessagesCount={0}
      todayHasMatch={false}
      managerName="Ada Lovelace"
      teamName="Alpha FC"
      competitionName="England First Division"
      onNavigateSettings={vi.fn()}
      onExitClick={vi.fn()}
      isUnemployed={false}
      {...overrides}
    />,
  );
}

describe("DashboardSidebar", () => {
  /**
   * Given a club in a league
   * When the sidebar is expanded
   * Then the competition name shows under the club name, visible from every tab.
   */
  it("shows the competition name under the club name", () => {
    renderSidebar();

    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("England First Division")).toBeInTheDocument();
  });

  /**
   * Given a club with no league table
   * When the sidebar is expanded
   * Then no competition line is drawn.
   */
  it("draws no competition line when there is none", () => {
    renderSidebar({ competitionName: null });

    expect(screen.queryByText("England First Division")).not.toBeInTheDocument();
  });

  /**
   * Given the manager button is labelled "Manager" for its action
   * When a screen reader reads it
   * Then the description carries the manager, club and competition, which the label hides.
   */
  it("describes the manager button with the manager, club and competition", () => {
    renderSidebar();

    expect(screen.getByRole("button", { name: "dashboard.manager" })).toHaveAccessibleDescription(
      "Ada Lovelace Alpha FC England First Division",
    );
  });

  /**
   * Given a window too narrow for the expanded sidebar
   * When the sidebar is folded for it
   * Then there is no expand button (expanding would push the workspace under 40rem), and every
   * destination is still a labelled button on the icon rail.
   */
  it("folds to a labelled icon rail with no expand button when the window forces it", () => {
    renderSidebar({ collapsed: true, collapseForced: true });

    expect(screen.queryByRole("button", { name: "dashboard.expandSidebar" })).toBeNull();
    for (const name of ["dashboard.home", "dashboard.squad", "dashboard.settings"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
  });

  /**
   * Given a window with room
   * When the player folds the sidebar themselves
   * Then the expand button is there to open it again.
   */
  it("keeps the expand button when the player folded it", () => {
    renderSidebar({ collapsed: true, collapseForced: false });

    expect(screen.getByRole("button", { name: "dashboard.expandSidebar" })).toBeInTheDocument();
  });
});
