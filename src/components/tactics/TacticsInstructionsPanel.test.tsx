import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { TacticsPhaseSettings } from "../../store/types";
import TacticsInstructionsPanel, { type InstructionsSection } from "./TacticsInstructionsPanel";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown> | string) =>
      params && typeof params === "object" && "section" in params
        ? `${key}:${String(params.section)}`
        : key,
  }),
}));

const ALL: InstructionsSection[] = ["withBall", "transitions", "withoutBall", "setPieces"];

function renderPanel(
  overrides: Partial<Parameters<typeof TacticsInstructionsPanel>[0]> = {},
): Record<string, ReturnType<typeof vi.fn>> {
  const handlers = {
    onOpenChange: vi.fn(),
    onToggleSection: vi.fn(),
    onOpenAt: vi.fn(),
    onTacticsPhaseChange: vi.fn(),
  };
  render(
    <TacticsInstructionsPanel
      open
      openSections={new Set(ALL)}
      tacticsPhase={{ build_up_style: "Mixed" } as TacticsPhaseSettings}
      setPieces={<p>set-piece takers</p>}
      {...handlers}
      {...overrides}
    />,
  );
  return handlers;
}

describe("TacticsInstructionsPanel", () => {
  /**
   * Given the open panel with every section open
   * When it is drawn
   * Then it is a labelled region holding the phases in the order a match is played, then the set
   *      pieces, each a button that says it is expanded, and each dial a labelled control.
   */
  it("shows the phases in match order, then the set pieces, each dial labelled", () => {
    renderPanel();

    const region = screen.getByRole("region", { name: "tactics.instructionsPanel.title" });
    const headers = within(region)
      .getAllByRole("button", { expanded: true })
      .filter((button) =>
        button.getAttribute("aria-controls")?.startsWith("tactics-instructions-"),
      );
    expect(headers.map((header) => header.textContent)).toEqual([
      "tactics.phaseLabels.withBall",
      "tactics.phaseLabels.transitions",
      "tactics.phaseLabels.withoutBall",
      "tactics.setPiecesSection",
    ]);
    expect(screen.getByLabelText("tactics.phaseSettings.buildUpStyle")).toHaveValue("Mixed");
    expect(screen.getByLabelText("tactics.phaseSettings.markingStyle")).toBeInTheDocument();
    expect(screen.getByText("set-piece takers")).toBeInTheDocument();
  });

  /**
   * Given the open panel
   * When the build-up is changed to short, a section header is pressed, and the hide button pressed
   * Then each tells its owner: the dial's new value, the section to fold, and that it closes.
   */
  it("reports a dial change, a section toggle and the panel closing", () => {
    const handlers = renderPanel();

    fireEvent.change(screen.getByLabelText("tactics.phaseSettings.buildUpStyle"), {
      target: { value: "Short" },
    });
    fireEvent.click(screen.getByRole("button", { name: /tactics.phaseLabels.transitions/ }));
    fireEvent.click(screen.getByRole("button", { name: "tactics.instructionsPanel.hide" }));

    expect(handlers.onTacticsPhaseChange).toHaveBeenCalledWith({ build_up_style: "Short" });
    expect(handlers.onToggleSection).toHaveBeenCalledWith("transitions");
    expect(handlers.onOpenChange).toHaveBeenCalledWith(false);
  });

  /**
   * Given a folded section
   * When the panel is drawn
   * Then its header says it is collapsed and its dials are not drawn.
   */
  it("draws no dials for a folded section", () => {
    renderPanel({ openSections: new Set<InstructionsSection>(["setPieces"]) });

    expect(screen.getByRole("button", { name: /tactics.phaseLabels.withBall/ })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(screen.queryByLabelText("tactics.phaseSettings.buildUpStyle")).not.toBeInTheDocument();
  });

  /**
   * Given the panel folded to its rail
   * When its show button and its "Out of possession" button are pressed
   * Then the first opens the panel and the second opens it at that section: every phase stays a
   *      keyboard stop away, named in words.
   */
  it("keeps every section one press away from the folded rail", () => {
    const handlers = renderPanel({ open: false });

    const rail = screen.getByRole("navigation", { name: "tactics.instructionsPanel.title" });
    expect(
      within(rail).getByRole("button", { name: "tactics.instructionsPanel.show" }),
    ).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(within(rail).getByRole("button", { name: "tactics.instructionsPanel.show" }));
    fireEvent.click(
      within(rail).getByRole("button", {
        name: "tactics.instructionsPanel.openAt:tactics.phaseLabels.withoutBall",
      }),
    );

    expect(handlers.onOpenChange).toHaveBeenCalledWith(true);
    expect(handlers.onOpenAt).toHaveBeenCalledWith("withoutBall");
    expect(screen.queryByRole("region")).not.toBeInTheDocument();
  });
});
