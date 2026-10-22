import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import DashboardSidebar from "./DashboardSidebar";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

function renderSidebar(overrides: Partial<React.ComponentProps<typeof DashboardSidebar>> = {}) {
  return render(
    <DashboardSidebar
      activeTab="Home"
      collapsed={false}
      onNavClick={vi.fn()}
      onToggleCollapse={vi.fn()}
      unreadMessagesCount={0}
      todayHasMatch={false}
      managerName="Ada Lovelace"
      teamName="Alpha FC"
      competitionName="England First Division"
      onNavigateSettings={vi.fn()}
      onExitClick={vi.fn()}
      isUnemployed={false}
      width="standard"
      behaviour="normal"
      {...overrides}
    />,
  );
}

describe("DashboardSidebar", () => {
  /**
   * Given a club in a league
   * When the sidebar is expanded
   * Then the competition name shows under the club name, visible from every tab.
   */
  it("shows the competition name under the club name", () => {
    renderSidebar();

    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("England First Division")).toBeInTheDocument();
  });

  /**
   * Given a club with no league table
   * When the sidebar is expanded
   * Then no competition line is drawn.
   */
  it("draws no competition line when there is none", () => {
    renderSidebar({ competitionName: null });

    expect(screen.queryByText("England First Division")).not.toBeInTheDocument();
  });

  /**
   * Given the manager button is labelled "Manager" for its action
   * When a screen reader reads it
   * Then the description carries the manager, club and competition, which the label hides.
   */
  it("describes the manager button with the manager, club and competition", () => {
    renderSidebar();

    expect(screen.getByRole("button", { name: "dashboard.manager" })).toHaveAccessibleDescription(
      "Ada Lovelace Alpha FC England First Division",
    );
  });

  /**
   * Given the player folded the sidebar
   * When it shows as the icon rail
   * Then every destination is still a labelled button and the expand button opens it again.
   */
  it("folds to a labelled icon rail the player can open again", () => {
    renderSidebar({ collapsed: true });

    expect(screen.getByRole("button", { name: "dashboard.expandSidebar" })).toBeInTheDocument();
    for (const name of ["dashboard.home", "dashboard.squad", "dashboard.settings"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
  });

  /**
   * Given three unread messages, open and folded
   * When the inbox badge is drawn
   * Then it is a circle (as tall as it is wide for one digit, a fixed height with tight padding so
   * more digits widen it only as much as they need), not a pill with wide side padding.
   */
  it.each([false, true])("draws the unread badge as a circle (folded: %s)", (collapsed) => {
    renderSidebar({ collapsed, unreadMessagesCount: 3 });
    const badge = screen.getByText("3");

    expect(badge).toHaveClass("h-5", "min-w-5", "px-1", "rounded-full");
    expect(badge).not.toHaveClass("px-2");
    expect(badge).not.toHaveClass("py-0.5");
  });

  /**
   * Given each sidebar width in a window with room
   * When the sidebar opens
   * Then it is drawn at that width and takes that width from the page.
   */
  it.each([
    ["compact", "w-sidebar-compact"],
    ["standard", "w-sidebar-standard"],
    ["wide", "w-sidebar-wide"],
  ] as const)("draws the open sidebar %s", (width, widthClass) => {
    const { container } = renderSidebar({ width });
    expect(screen.getByRole("navigation").closest("aside")).toHaveClass(widthClass);
    expect(container.querySelector("[data-sidebar-slot]")).toHaveClass(widthClass);
  });

  /**
   * Given Expand on click
   * When the player presses the expand button, and then Esc
   * Then the sidebar opens over the page (the page keeps the rail's width) with the focus on the
   * page it is on; Esc closes it and gives the focus back to the button.
   */
  it("opens as a drawer from its button, takes the focus in and gives it back on Esc", () => {
    const { container } = renderSidebar({ behaviour: "click" });
    const aside = screen.getByRole("navigation").closest("aside");
    expect(aside).toHaveClass("w-sidebar-rail");

    fireEvent.click(screen.getByRole("button", { name: "dashboard.expandSidebar" }));

    expect(aside).toHaveClass("w-sidebar-standard");
    expect(container.querySelector("[data-sidebar-slot]")).toHaveClass("w-sidebar-rail");
    expect(screen.getByRole("button", { name: "dashboard.home" })).toHaveFocus();

    fireEvent.keyDown(document, { key: "Escape" });

    expect(aside).toHaveClass("w-sidebar-rail");
    expect(screen.getByRole("button", { name: "dashboard.expandSidebar" })).toHaveFocus();
  });

  /**
   * Given Expand on click, the drawer open with the focus on the current page's item
   * When the player Tabs out of the drawer to a control on the page (the header's search)
   * Then the drawer closes, so it doesn't cover the control that now has the focus.
   */
  it("closes the drawer when the focus Tabs out to the page", () => {
    renderSidebar({ behaviour: "click" });
    const aside = screen.getByRole("navigation").closest("aside");
    const search = document.body.appendChild(document.createElement("input"));

    fireEvent.click(screen.getByRole("button", { name: "dashboard.expandSidebar" }));
    const current = screen.getByRole("button", { name: "dashboard.home" });
    fireEvent.focusOut(current, {
      relatedTarget: screen.getByRole("button", { name: "dashboard.squad" }),
    });
    expect(aside).toHaveClass("w-sidebar-standard");

    fireEvent.focusOut(current, { relatedTarget: search });
    expect(aside).toHaveClass("w-sidebar-rail");
    search.remove();
  });

  /**
   * Given a sidebar with a button that folds it or opens it as a drawer
   * When a screen reader reaches the button
   * Then the button says which element it controls: the sidebar.
   */
  it.each(["normal", "click"] as const)("names the sidebar its %s button controls", (behaviour) => {
    renderSidebar({ behaviour });
    const aside = screen.getByRole("navigation").closest("aside");
    const button = screen.getByRole("button", {
      name: behaviour === "click" ? "dashboard.expandSidebar" : "dashboard.collapseSidebar",
    });
    expect(aside?.id).toBeTruthy();
    expect(button).toHaveAttribute("aria-controls", aside?.id);
  });

  /**
   * Given the drawer open
   * When the player chooses a page, or presses outside the sidebar
   * Then it closes; a page choice says it came from the drawer (the dashboard then moves the focus
   * to the page's heading). A press inside the sidebar leaves it open.
   */
  it("closes on a page choice or a click outside", () => {
    const onNavClick = vi.fn();
    renderSidebar({ behaviour: "click", onNavClick });
    const aside = screen.getByRole("navigation").closest("aside");

    fireEvent.click(screen.getByRole("button", { name: "dashboard.expandSidebar" }));
    fireEvent.click(screen.getByRole("button", { name: "dashboard.squad" }));
    expect(onNavClick).toHaveBeenCalledWith("Squad", { fromDrawer: true });
    expect(aside).toHaveClass("w-sidebar-rail");

    fireEvent.click(screen.getByRole("button", { name: "dashboard.expandSidebar" }));
    fireEvent.pointerDown(screen.getByRole("navigation"));
    expect(aside).toHaveClass("w-sidebar-standard");
    fireEvent.pointerDown(document.body);
    expect(aside).toHaveClass("w-sidebar-rail");
  });

  /**
   * Given Icons only, and Expand on hover
   * When the sidebar is drawn
   * Then it is a rail with no expand button (Icons only never opens; hover opens without one), and
   * every destination is a named button with its name as a tooltip.
   */
  it.each(["icons", "hover"] as const)("is a rail with no button for %s", (behaviour) => {
    renderSidebar({ behaviour });
    expect(screen.getByRole("navigation").closest("aside")).toHaveClass("w-sidebar-rail");
    expect(screen.queryByRole("button", { name: "dashboard.expandSidebar" })).toBeNull();
    expect(screen.getByRole("button", { name: "dashboard.squad" })).toHaveAttribute(
      "title",
      "dashboard.squad",
    );
  });

  /**
   * Given a player who has asked for reduced motion
   * When the sidebar changes width
   * Then it doesn't slide.
   */
  it("doesn't slide for reduced motion", () => {
    renderSidebar();
    expect(screen.getByRole("navigation").closest("aside")).toHaveClass(
      "motion-reduce:transition-none",
    );
  });
});
