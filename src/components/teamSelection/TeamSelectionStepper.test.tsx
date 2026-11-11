import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TeamSelectionStepper from "./TeamSelectionStepper";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("TeamSelectionStepper", () => {
  /**
   * Given a player who has reached the club step
   * When the stepper is drawn
   * Then it lists the four steps in order, marks the club step as the current one, and says "done"
   *      for each earlier step, so no step is told by its colour alone.
   */
  it("lists the steps in order, marks the current one, and says the earlier ones are done", () => {
    render(<TeamSelectionStepper />);

    const steps = within(screen.getByRole("list", { name: "teamSelect.steps.label" })).getAllByRole(
      "listitem",
    );
    expect(steps.map((step) => step.textContent)).toEqual([
      "teamSelect.steps.managerteamSelect.steps.done",
      "teamSelect.steps.worldteamSelect.steps.done",
      "teamSelect.steps.generationteamSelect.steps.done",
      "4teamSelect.steps.club",
    ]);
    expect(steps[3]).toHaveAttribute("aria-current", "step");
    expect(steps[0]).not.toHaveAttribute("aria-current");
  });
});
