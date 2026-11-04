import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import TacticsWorkbench from "./TacticsWorkbench";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown> | string) =>
      params && typeof params === "object" && "section" in params
        ? `${key}:${String(params.section)}`
        : key,
  }),
}));

function renderWorkbench() {
  return render(
    <TacticsWorkbench onTacticsPhaseChange={vi.fn()} setPieces={<p>takers</p>}>
      <p>lineup</p>
    </TacticsWorkbench>,
  );
}

describe("TacticsWorkbench", () => {
  /**
   * Given the workbench as it opens
   * When it is drawn
   * Then it follows the room it has: the rail and the open panel are both there for the container
   *      query to choose between (the query shows the rail under 72rem, the panel above), and the
   *      lineup sits in its own scrolling region.
   */
  it("follows the room it has until the manager chooses", () => {
    const { container } = renderWorkbench();

    expect(container.querySelector("[data-instructions]")).toHaveAttribute(
      "data-instructions",
      "auto",
    );
    expect(
      screen.getByRole("navigation", { name: "tactics.instructionsPanel.title" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("region", { name: "tactics.instructionsPanel.title" }),
    ).toBeInTheDocument();
    expect(screen.getByText("lineup").parentElement).toHaveClass("overflow-y-auto", "min-h-0");
  });

  /**
   * Given the workbench
   * When the manager hides the panel, and then opens it again at the set pieces from the rail
   * Then it is a rail alone, and then the panel alone with the set pieces open: his choice holds
   *      whatever the room.
   */
  it("holds the manager's choice to fold or open the panel", () => {
    const { container } = renderWorkbench();

    fireEvent.click(screen.getByRole("button", { name: "tactics.instructionsPanel.hide" }));
    expect(container.querySelector("[data-instructions]")).toHaveAttribute(
      "data-instructions",
      "closed",
    );
    expect(screen.queryByRole("region")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: "tactics.instructionsPanel.openAt:tactics.setPiecesSection",
      }),
    );
    expect(container.querySelector("[data-instructions]")).toHaveAttribute(
      "data-instructions",
      "open",
    );
    expect(screen.queryByRole("navigation")).not.toBeInTheDocument();
    expect(screen.getByText("takers")).toBeInTheDocument();
  });

  /**
   * Given the panel with every section open
   * When the manager folds "With ball" and folds it open again
   * Then its dials go and come back.
   */
  it("folds and unfolds a section", () => {
    renderWorkbench();
    fireEvent.click(screen.getByRole("button", { name: "tactics.instructionsPanel.hide" }));
    fireEvent.click(screen.getByRole("button", { name: "tactics.instructionsPanel.show" }));

    const header = screen.getByRole("button", { name: /tactics.phaseLabels.withBall/ });
    fireEvent.click(header);
    expect(screen.queryByLabelText("tactics.phaseSettings.buildUpStyle")).not.toBeInTheDocument();
    fireEvent.click(header);
    expect(screen.getByLabelText("tactics.phaseSettings.buildUpStyle")).toBeInTheDocument();
  });
});
