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
// A profile or a tab "explodes" for the ids and tabs named here, and draws itself for the others.
vi.mock("../playerProfile/PlayerProfile", () => ({
  default: ({ player }: { player: { id: string } }) => {
    if (player.id === "bad-player") throw new Error("the player's profile exploded");
    return <p>profile of {player.id}</p>;
  },
}));
vi.mock("../teamProfile", () => ({
  default: ({ team }: { team: { id: string } }) => {
    if (team.id === "bad-team") throw new Error("the team's profile exploded");
    return <p>profile of {team.id}</p>;
  },
}));
vi.mock("./DashboardTabContent", () => ({
  default: ({ viewModel }: { viewModel: { activeTab: string } }) => {
    if (viewModel.activeTab === "Tactics") throw new Error("the open tab exploded");
    return <p>tab {viewModel.activeTab}</p>;
  },
}));

const gameState = {
  players: [{ id: "bad-player" }, { id: "good-player" }, { id: "other-player" }],
  teams: [{ id: "bad-team" }, { id: "good-team" }],
  manager: { team_id: null },
} as unknown as GameStateData;

function renderWorkspace(view: {
  activeTab: string;
  selectedPlayerId?: string | null;
  selectedTeamId?: string | null;
}) {
  return (
    <div>
      <header>header stays</header>
      <DashboardWorkspaceContent
        dashboardAlerts={[]}
        gameState={gameState}
        profileNavigation={
          {
            selectedPlayerId: view.selectedPlayerId ?? null,
            selectedTeamId: view.selectedTeamId ?? null,
          } as never
        }
        dashboardTabContentModel={{ activeTab: view.activeTab } as never}
        onNavigate={vi.fn()}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onGameUpdate={vi.fn()}
        onBack={vi.fn()}
        isUnemployed={false}
      />
    </div>
  );
}

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
    render(renderWorkspace({ activeTab: "Tactics" }));

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("header stays")).toBeInTheDocument();
    expect(logError).toHaveBeenCalledWith(expect.stringContaining("the open tab exploded"));
  });

  /**
   * Given the error screen showing for the Tactics tab
   * When the player opens another tab
   * Then the boundary is remounted by the new view and the other tab draws: the error does not
   *      follow the player to every page.
   */
  it("clears the error screen when another tab is opened", () => {
    const { rerender } = render(renderWorkspace({ activeTab: "Tactics" }));
    expect(screen.getByRole("alert")).toBeInTheDocument();

    rerender(renderWorkspace({ activeTab: "Squad" }));

    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByText("tab Squad")).toBeInTheDocument();
  });

  /**
   * Given the error screen showing for a player's profile
   * When the player opens another player's profile (the tab does not change), or a team's, or
   *      leaves the profile
   * Then healthy content replaces the alert each time: the key carries the selected player and team.
   */
  it.each([
    ["another player's profile", { selectedPlayerId: "good-player" }, "profile of good-player"],
    ["a team's profile", { selectedTeamId: "good-team" }, "profile of good-team"],
    ["the tab again", {}, "tab Squad"],
  ] as const)("clears the error screen of a player's profile when the player opens %s", (_name, next, text) => {
    const { rerender } = render(
      renderWorkspace({ activeTab: "Squad", selectedPlayerId: "bad-player" }),
    );
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(logError).toHaveBeenCalledWith(expect.stringContaining("the player's profile exploded"));

    rerender(renderWorkspace({ activeTab: "Squad", ...next }));

    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByText(text)).toBeInTheDocument();
  });

  /**
   * Given the error screen showing for a team's profile
   * When the player opens another team's profile on the same tab
   * Then the alert is replaced by the other profile.
   */
  it("clears the error screen of a team's profile when another team is opened", () => {
    const { rerender } = render(
      renderWorkspace({ activeTab: "Squad", selectedTeamId: "bad-team" }),
    );
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(logError).toHaveBeenCalledWith(expect.stringContaining("the team's profile exploded"));

    rerender(renderWorkspace({ activeTab: "Squad", selectedTeamId: "good-team" }));

    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByText("profile of good-team")).toBeInTheDocument();
  });
});
