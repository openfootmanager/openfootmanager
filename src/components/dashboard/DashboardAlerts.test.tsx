import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import DashboardAlerts from "./DashboardAlerts";
import type { DashboardAlert } from "./dashboardHelpers";

const alerts: DashboardAlert[] = Array.from({ length: 6 }, (_, index) => ({
  id: `alert-${index}`,
  severity: "info",
  tab: "Inbox",
  text: `Alert ${index}`,
}));

describe("DashboardAlerts", () => {
  /**
   * Given six alerts on a page that scrolls as a whole
   * When they are drawn
   * Then the list is not capped: the page's own scroll carries it.
   */
  it("leaves the list uncapped on a page that scrolls", () => {
    render(<DashboardAlerts alerts={alerts} onNavigate={vi.fn()} />);

    const list = screen.getByText("Alert 0").closest("button")?.parentElement;
    expect(list).not.toHaveClass("overflow-y-auto");
    expect(list).not.toHaveClass("max-h-24");
  });

  /**
   * Given six alerts above a tab that fills the workspace (Tactics)
   * When they are drawn
   * Then the list is capped and scrolls itself, so it cannot squeeze the tab below it to a strip
   *      at a small window or a large UI scale.
   */
  it("caps the list and scrolls it above a tab that fills the workspace", () => {
    render(<DashboardAlerts alerts={alerts} onNavigate={vi.fn()} scrolls />);

    const list = screen.getByText("Alert 0").closest("button")?.parentElement;
    expect(list).toHaveClass("max-h-24", "shrink-0", "overflow-y-auto");
  });

  it("draws nothing without alerts", () => {
    const { container } = render(<DashboardAlerts alerts={[]} onNavigate={vi.fn()} scrolls />);

    expect(container).toBeEmptyDOMElement();
  });
});
