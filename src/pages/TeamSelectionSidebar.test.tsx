import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createPlayer, createTeam } from "../test-utils/factories";
import TeamSelectionSidebar from "./TeamSelectionSidebar";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
}));

describe("TeamSelectionSidebar", () => {
  /**
   * Given a key player whose `position` is the legacy bucket and whose natural position is granular
   * When the sidebar lists the key players
   * Then it shows the translated granular position, not the raw legacy string.
   */
  it("lists a key player by their translated granular natural position", () => {
    render(
      <TeamSelectionSidebar
        selectedTeam={createTeam()}
        selectedTeamXi={[createPlayer({ position: "Defender", natural_position: "LeftBack" })]}
        selectedTeamCompetitions={[]}
        getTeamAvgOvr={() => 70}
        getPlayerRating={() => 70}
      />,
    );

    expect(screen.getByText("common.positions.LeftBack")).toBeInTheDocument();
    expect(screen.queryByText("Defender")).not.toBeInTheDocument();
  });

  /**
   * Given a club and a key player the backend has rated, and a club it has not rated yet
   * When the sidebar shows them
   * Then the backend's average and the player's rating show, and before it answers, a dash
   */
  it("shows the backend's ratings, and a dash before it answers", () => {
    const player = createPlayer({ id: "key", match_name: "K. Player" });
    const { rerender } = render(
      <TeamSelectionSidebar
        selectedTeam={createTeam()}
        selectedTeamXi={[player]}
        selectedTeamCompetitions={[]}
        getTeamAvgOvr={() => 73}
        getPlayerRating={(id) => (id === "key" ? 81 : null)}
      />,
    );
    expect(screen.getByText("73")).toBeInTheDocument();
    expect(screen.getByText("81")).toBeInTheDocument();

    rerender(
      <TeamSelectionSidebar
        selectedTeam={createTeam()}
        selectedTeamXi={[player]}
        selectedTeamCompetitions={[]}
        getTeamAvgOvr={() => null}
        getPlayerRating={() => null}
      />,
    );
    expect(screen.getAllByText("–")).toHaveLength(3);
  });

  /**
   * Given a club with a full squad that the backend has not rated yet (its likely eleven is still
   * empty)
   * When the sidebar shows it
   * Then the likely-eleven tile shows a dash, not "0 players"
   */
  it("shows a dash for the likely eleven before the backend answers", () => {
    render(
      <TeamSelectionSidebar
        selectedTeam={createTeam()}
        selectedTeamXi={[]}
        selectedTeamCompetitions={[]}
        getTeamAvgOvr={() => null}
        getPlayerRating={() => null}
      />,
    );

    expect(screen.getByText("teamSelect.likelyXi").nextElementSibling?.textContent).toBe("–");
    expect(screen.queryByText("teamSelect.playersCount")).not.toBeInTheDocument();
  });
});
