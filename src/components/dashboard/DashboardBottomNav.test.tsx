import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import DashboardBottomNav from "./DashboardBottomNav";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

function renderBottomNav(
  overrides: Partial<Parameters<typeof DashboardBottomNav>[0]> = {},
) {
  const onNavClick = vi.fn();
  const onNavigateSettings = vi.fn();
  const onExitClick = vi.fn();
  render(
    <DashboardBottomNav
      activeTab="Home"
      isUnemployed={false}
      onNavigateSettings={onNavigateSettings}
      onExitClick={onExitClick}
      onNavClick={onNavClick}
      unreadMessagesCount={0}
      {...overrides}
    />,
  );

  return { onNavClick, onNavigateSettings, onExitClick };
}

describe("DashboardBottomNav", () => {
  it("is hidden on desktop (md and up)", () => {
    const { container } = render(
      <DashboardBottomNav
        activeTab="Home"
        isUnemployed={false}
        onNavigateSettings={vi.fn()}
        onExitClick={vi.fn()}
        onNavClick={vi.fn()}
        unreadMessagesCount={0}
      />,
    );

    const nav = container.querySelector("nav");
    expect(nav?.className).toContain("md:hidden");
  });

  it("renders the four primary tabs and the more button", () => {
    renderBottomNav();

    expect(screen.getByTestId("bottom-nav-tab-Home")).toBeTruthy();
    expect(screen.getByTestId("bottom-nav-tab-Squad")).toBeTruthy();
    expect(screen.getByTestId("bottom-nav-tab-Inbox")).toBeTruthy();
    expect(screen.getByTestId("bottom-nav-tab-Schedule")).toBeTruthy();
    expect(screen.getByTestId("bottom-nav-more")).toBeTruthy();
  });

  it("forwards primary tab clicks to onNavClick", () => {
    const { onNavClick } = renderBottomNav();

    fireEvent.click(screen.getByTestId("bottom-nav-tab-Squad"));
    expect(onNavClick).toHaveBeenCalledWith("Squad");
  });

  it("shows the unread message badge on the inbox item", () => {
    renderBottomNav({ unreadMessagesCount: 3 });

    const inboxButton = screen.getByTestId("bottom-nav-tab-Inbox");
    expect(inboxButton.textContent).toContain("3");
    expect(inboxButton.getAttribute("aria-label")).toBe(
      "dashboard.inbox (3)",
    );
  });

  it("shows the match indicator badge on the schedule item", () => {
    renderBottomNav({ todayHasMatch: true });

    expect(screen.getByTestId("bottom-nav-tab-Schedule").textContent).toContain(
      "!",
    );
  });

  it("opens the more sheet with the remaining tabs and closes after selection", () => {
    const { onNavClick } = renderBottomNav();

    expect(screen.queryByTestId("bottom-nav-sheet-tab-Tactics")).toBeNull();

    fireEvent.click(screen.getByTestId("bottom-nav-more"));
    expect(screen.getByTestId("bottom-nav-sheet-tab-Tactics")).toBeTruthy();
    expect(screen.getByTestId("bottom-nav-sheet-tab-Manager")).toBeTruthy();
    expect(screen.getByTestId("bottom-nav-sheet-tab-Tournaments")).toBeTruthy();

    fireEvent.click(screen.getByTestId("bottom-nav-sheet-tab-Tactics"));
    expect(onNavClick).toHaveBeenCalledWith("Tactics");
    expect(screen.queryByTestId("bottom-nav-sheet-tab-Tactics")).toBeNull();
  });

  it("closes the more sheet via the close button", () => {
    renderBottomNav();

    fireEvent.click(screen.getByTestId("bottom-nav-more"));
    expect(screen.getByTestId("bottom-nav-sheet-close")).toBeTruthy();

    fireEvent.click(screen.getByTestId("bottom-nav-sheet-close"));
    expect(screen.queryByTestId("bottom-nav-sheet-tab-Tactics")).toBeNull();
  });

  it("hides club tabs in the more sheet when unemployed", () => {
    renderBottomNav({ isUnemployed: true });

    fireEvent.click(screen.getByTestId("bottom-nav-more"));
    expect(screen.queryByTestId("bottom-nav-sheet-tab-Tactics")).toBeNull();
    expect(screen.queryByTestId("bottom-nav-sheet-tab-Finances")).toBeNull();
    expect(screen.getByTestId("bottom-nav-sheet-tab-Players")).toBeTruthy();
  });

  it("renders settings and exit entries at the bottom of the more sheet", () => {
    renderBottomNav();

    fireEvent.click(screen.getByTestId("bottom-nav-more"));
    expect(screen.getByTestId("bottom-nav-sheet-settings").textContent).toContain(
      "dashboard.settings",
    );
    expect(screen.getByTestId("bottom-nav-sheet-exit").textContent).toContain(
      "dashboard.exitToMenu",
    );
  });

  it("forwards the settings click and closes the sheet", () => {
    const { onNavigateSettings, onExitClick } = renderBottomNav();

    fireEvent.click(screen.getByTestId("bottom-nav-more"));
    fireEvent.click(screen.getByTestId("bottom-nav-sheet-settings"));

    expect(onNavigateSettings).toHaveBeenCalledTimes(1);
    expect(onExitClick).not.toHaveBeenCalled();
    expect(screen.queryByTestId("bottom-nav-sheet-settings")).toBeNull();
  });

  it("forwards the exit click and closes the sheet", () => {
    const { onNavigateSettings, onExitClick } = renderBottomNav();

    fireEvent.click(screen.getByTestId("bottom-nav-more"));
    fireEvent.click(screen.getByTestId("bottom-nav-sheet-exit"));

    expect(onExitClick).toHaveBeenCalledTimes(1);
    expect(onNavigateSettings).not.toHaveBeenCalled();
    expect(screen.queryByTestId("bottom-nav-sheet-exit")).toBeNull();
  });
});
