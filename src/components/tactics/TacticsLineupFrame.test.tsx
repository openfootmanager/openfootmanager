import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import TacticsLineupFrame from "./TacticsLineupFrame";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({ t: (key: string) => key }),
}));

let width = 0;
const observers = new Set<() => void>();

/** Makes every element measure `widthPx` wide, with a ResizeObserver that reports at once. */
function measureAs(widthPx: number): void {
  width = widthPx;
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
    () => ({ width }) as DOMRect,
  );
  vi.stubGlobal(
    "ResizeObserver",
    class {
      private readonly notify: () => void;
      constructor(callback: () => void) {
        this.notify = callback;
      }
      observe(): void {
        observers.add(this.notify);
      }
      disconnect(): void {
        observers.delete(this.notify);
      }
    },
  );
}

/** The window is resized: every observer reports. */
function resizeTo(widthPx: number): void {
  width = widthPx;
  act(() => {
    for (const notify of observers) notify();
  });
}

function renderFrame() {
  return render(<TacticsLineupFrame table={<p>the table</p>} pitch={<p>the pitch</p>} />);
}

describe("TacticsLineupFrame", () => {
  afterEach(() => {
    observers.clear();
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

  /**
   * Given the narrow lineup region with the Pitch panel open
   * When the Close button in it is pressed
   * Then the panel closes (a touch window has no Escape key).
   */
  it("closes the panel with its own Close button", () => {
    measureAs(600);
    renderFrame();
    fireEvent.click(screen.getByRole("button", { name: "tactics.pitch" }));

    fireEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", { name: "common.close" }),
    );

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  /**
   * Given the Pitch panel open at a narrow width
   * When the window widens until the pitch sits beside the table, and narrows again
   * Then the panel does not come back: it was opened by a press that the wide layout ended.
   */
  it("does not reopen the panel when the layout goes wide and narrow again", () => {
    measureAs(600);
    renderFrame();
    fireEvent.click(screen.getByRole("button", { name: "tactics.pitch" }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    resizeTo(1000);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByText("the pitch")).toBeInTheDocument();
    resizeTo(600);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "tactics.pitch" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });
});
