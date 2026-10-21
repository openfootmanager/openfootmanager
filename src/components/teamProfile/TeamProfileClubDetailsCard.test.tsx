import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { createTeam } from "../../test-utils/factories";
import TeamProfileClubDetailsCard from "./TeamProfileClubDetailsCard";
import type { TeamProfileTranslate } from "./TeamProfile.types";

// Translations as the English locale has them, so the test fails on the raw enum.
const t = ((key: string) =>
  key === "common.playStyles.HighPress" ? "High Press" : key) as unknown as TeamProfileTranslate;

describe("TeamProfileClubDetailsCard", () => {
  /**
   * Given a club whose play style is the enum value HighPress
   * When its club details are shown
   * Then the style reads as the translated label, not the raw enum.
   */
  it("shows the play style translated, not as the raw enum", () => {
    render(<TeamProfileClubDetailsCard team={createTeam({ play_style: "HighPress" })} t={t} />);

    expect(screen.getByText("High Press")).toBeInTheDocument();
    expect(screen.queryByText("HighPress")).not.toBeInTheDocument();
  });
});
