import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createPlayer, createTeam } from "../test-utils/factories";
import TeamSelectionSidebar from "./TeamSelectionSidebar";

vi.mock("react-i18next", () => ({
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
      />,
    );

    expect(screen.getByText("common.positions.LeftBack")).toBeInTheDocument();
    expect(screen.queryByText("Defender")).not.toBeInTheDocument();
  });
});
