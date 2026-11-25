import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import TacticsWorkbench from "./TacticsWorkbench";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown> | string) =>
      params && typeof params === "object" && "section" in params
        ? `${key}:${String(params.section)}`
        : key,
  }),
}));

/** Makes every element measure `widthPx` wide, with a ResizeObserver that reports at once. */
function measureAs(widthPx: number): void {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
    width: widthPx,
  } as DOMRect);
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe(): void {}
      disconnect(): void {}
    },
  );
}

function renderWorkbench() {
  return render(
    <TacticsWorkbench onTacticsPhaseChange={vi.fn()} setPieces={<p>takers</p>}>
      <p>lineup</p>
    </TacticsWorkbench>,
  );
}

const layoutOf = (container: HTMLElement) =>
  container.querySelector("[data-instructions]")?.getAttribute("data-instructions");

describe("TacticsWorkbench", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  /**
   * Given a workbench with room for both (72rem or wider)
   * When it opens
   * Then the instructions sit beside the lineup, and the lineup is a region of its own that fills its
   *      height and does not scroll (the table and the pitch inside it do).
   */
  it("puts the instructions beside the lineup where there is room", () => {
    measureAs(1300);
    const { container } = renderWorkbench();

    expect(layoutOf(container)).toBe("beside");
    expect(
      screen.getByRole("region", { name: "tactics.instructionsPanel.title" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("navigation")).not.toBeInTheDocument();
    // The lineup region never scrolls as a whole (the table inside it does): it clips and fills.
    const region = screen.getByText("lineup").parentElement;
    expect(region).toHaveClass("overflow-hidden", "min-h-0");
    expect(region).not.toHaveClass("overflow-y-auto");
    expect(region).toHaveAttribute("data-region", "tactics-lineup");
  });

  /**
   * Given a wide workbench
   * When the manager hides the panel, and shows it again
   * Then it is a rail, and then beside the lineup again: his choice holds.
   */
  it("folds to the rail and back when the manager asks", () => {
    measureAs(1300);
    const { container } = renderWorkbench();

    fireEvent.click(screen.getByRole("button", { name: "tactics.instructionsPanel.hide" }));
    expect(layoutOf(container)).toBe("rail");
    fireEvent.click(screen.getByRole("button", { name: "tactics.instructionsPanel.show" }));
    expect(layoutOf(container)).toBe("beside");
  });

  /**
   * Given a workbench too narrow for both (a 1280 window beside the sidebar)
   * When it opens, and the manager opens the set pieces from the rail, and then presses Escape
   * Then it starts as the rail; the instructions open as a drawer, a dialog named by the panel's
   *      title with the set pieces in it; and Escape closes it and gives the focus back to the
   *      rail button that opened it.
   */
  it("opens the instructions as a drawer where there is no room, closed by Escape", () => {
    measureAs(1000);
    const { container } = renderWorkbench();
    expect(layoutOf(container)).toBe("rail");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    const opener = screen.getByRole("button", {
      name: "tactics.instructionsPanel.openAt:tactics.setPiecesSection",
    });
    opener.focus();
    fireEvent.click(opener);

    const drawer = screen.getByRole("dialog", { name: "tactics.instructionsPanel.title" });
    expect(layoutOf(container)).toBe("drawer");
    expect(within(drawer).getByText("takers")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Escape" });

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(layoutOf(container)).toBe("rail");
    expect(document.activeElement).toBe(
      screen.getByRole("button", {
        name: "tactics.instructionsPanel.openAt:tactics.setPiecesSection",
      }),
    );
  });

  /**
   * Given the panel with every section open
   * When the manager folds "With ball" and unfolds it again
   * Then its dials go and come back.
   */
  it("folds and unfolds a section", () => {
    measureAs(1300);
    renderWorkbench();

    const header = screen.getByRole("button", { name: /tactics.phaseLabels.withBall/ });
    fireEvent.click(header);
    expect(screen.queryByLabelText("tactics.phaseSettings.buildUpStyle")).not.toBeInTheDocument();
    fireEvent.click(header);
    expect(screen.getByLabelText("tactics.phaseSettings.buildUpStyle")).toBeInTheDocument();
  });
});
