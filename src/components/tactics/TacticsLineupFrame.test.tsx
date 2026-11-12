import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import TacticsLineupFrame from "./TacticsLineupFrame";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({ t: (key: string) => key }),
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

function renderFrame() {
  return render(<TacticsLineupFrame table={<p>the table</p>} pitch={<p>the pitch</p>} />);
}

describe("TacticsLineupFrame", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  /**
   * Given a lineup region wide enough for the table and the pitch (60rem or wider)
   * When it opens
   * Then the table and the pitch sit side by side and there is no Pitch button.
   */
  it("puts the pitch beside the table where there is room", () => {
    measureAs(1000);
    renderFrame();

    expect(screen.getByText("the table")).toBeInTheDocument();
    expect(screen.getByText("the pitch")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "tactics.pitch" })).not.toBeInTheDocument();
  });

  /**
   * Given a lineup region too narrow for both (a 960 window beside the sidebar)
   * When it opens
   * Then the table has the whole width, the pitch is not drawn, and a Pitch button offers it.
   */
  it("gives the table the width and offers the pitch behind a button where there is no room", () => {
    measureAs(600);
    renderFrame();

    expect(screen.getByText("the table")).toBeInTheDocument();
    expect(screen.queryByText("the pitch")).not.toBeInTheDocument();
    const button = screen.getByRole("button", { name: "tactics.pitch" });
    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(button).toHaveAttribute("aria-haspopup", "dialog");
  });

  /**
   * Given the narrow lineup region
   * When the Pitch button is pressed, and then Escape
   * Then the pitch opens as a dialog named "Pitch" (the table stays under it), and Escape closes it.
   */
  it("opens the pitch as a panel over the table, closed by Escape", () => {
    measureAs(600);
    renderFrame();

    const button = screen.getByRole("button", { name: "tactics.pitch" });
    button.focus();
    fireEvent.click(button);

    const panel = screen.getByRole("dialog", { name: "tactics.pitch" });
    expect(within(panel).getByText("the pitch")).toBeInTheDocument();
    expect(screen.getByText("the table")).toBeInTheDocument();
    expect(button).toHaveAttribute("aria-expanded", "true");

    fireEvent.keyDown(window, { key: "Escape" });

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.queryByText("the pitch")).not.toBeInTheDocument();
    expect(button).toHaveAttribute("aria-expanded", "false");
  });
});
