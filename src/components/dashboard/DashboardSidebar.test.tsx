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
   * Given the player folded the sidebar
   * When it shows as the icon rail
   * Then every destination is still a labelled button and the expand button opens it again.
   */
  it("folds to a labelled icon rail the player can open again", () => {
    renderSidebar({ collapsed: true });

    expect(screen.getByRole("button", { name: "dashboard.expandSidebar" })).toBeInTheDocument();
    for (const name of ["dashboard.home", "dashboard.squad", "dashboard.settings"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
  });

  /**
   * Given three unread messages, open and folded
   * When the inbox badge is drawn
   * Then it is a circle (as tall as it is wide for one digit, a fixed height with tight padding so
   * more digits widen it only as much as they need), not a pill with wide side padding.
   */
  it.each([false, true])("draws the unread badge as a circle (folded: %s)", (collapsed) => {
    renderSidebar({ collapsed, unreadMessagesCount: 3 });
    const badge = screen.getByText("3");

    expect(badge).toHaveClass("h-5", "min-w-5", "px-1", "rounded-full");
    expect(badge).not.toHaveClass("px-2");
    expect(badge).not.toHaveClass("py-0.5");
  });
});
