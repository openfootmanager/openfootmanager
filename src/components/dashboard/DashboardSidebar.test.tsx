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
});
