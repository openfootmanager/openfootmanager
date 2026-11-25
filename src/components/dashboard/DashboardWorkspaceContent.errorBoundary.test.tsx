import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData } from "../../store/gameStore";
import DashboardWorkspaceContent from "./DashboardWorkspaceContent";

const logError = vi.fn();
vi.mock("../../lib/logger", () => ({
  logError: (message: string) => logError(message),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("./DashboardAlerts", () => ({ default: () => null }));
vi.mock("../playerProfile/PlayerProfile", () => ({ default: () => null }));
vi.mock("../teamProfile", () => ({ default: () => null }));
vi.mock("./DashboardTabContent", () => ({
  default: () => {
    throw new Error("the open tab exploded");
  },
}));

describe("DashboardWorkspaceContent error boundary", () => {
  beforeEach(() => {
    logError.mockReset();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  /**
   * Given the dashboard whose open tab throws while rendering
   * When it renders
   * Then the error screen stands in the content area, the rest of the page (a sibling, like the
   *      header and the sidebar) is still there, and the error is logged.
   */
  it("shows the error screen in the content area when the open tab throws", () => {
    render(
      <div>
        <header>header stays</header>
        <DashboardWorkspaceContent
          dashboardAlerts={[]}
          gameState={{ players: [], teams: [], manager: { team_id: null } } as unknown as GameStateData}
          profileNavigation={{ selectedPlayerId: null, selectedTeamId: null } as never}
          dashboardTabContentModel={{ activeTab: "Tactics" } as never}
          onNavigate={vi.fn()}
          onSelectPlayer={vi.fn()}
          onSelectTeam={vi.fn()}
          onGameUpdate={vi.fn()}
          onBack={vi.fn()}
          isUnemployed={false}
        />
      </div>,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("header stays")).toBeInTheDocument();
    expect(logError).toHaveBeenCalledWith(expect.stringContaining("the open tab exploded"));
  });
});
