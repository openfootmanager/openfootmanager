import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { TabPanel, Tabs, type TabItem } from "./Tabs";

const TABS: TabItem[] = [
  { id: "players", label: "Players", count: 3915 },
  { id: "list", label: "My list", count: 8 },
  { id: "offers", label: "Offers" },
];

function Harness({ look }: { look?: "underline" | "segmented" }) {
  const [selected, setSelected] = useState("players");
  return (
    <>
      <Tabs
        idPrefix="market"
        label="Transfer market"
        tabs={TABS}
        selectedId={selected}
        onSelect={setSelected}
        look={look}
      />
      {TABS.map((tab) => (
        <TabPanel key={tab.id} idPrefix="market" tabId={tab.id} hidden={tab.id !== selected}>
          {tab.id} content
        </TabPanel>
      ))}
    </>
  );
}

describe("Tabs", () => {
  /**
   * Given three tabs with the first selected,
   * when they render,
   * then there is one named tablist, the first tab is selected and alone in the tab order.
   */
  it("renders a named tablist with the selected tab alone in the tab order", () => {
    render(<Harness />);
    expect(screen.getByRole("tablist", { name: "Transfer market" })).toBeInTheDocument();
    const tabs = screen.getAllByRole("tab");
    expect(tabs).toHaveLength(3);
    expect(tabs.map((tab) => tab.getAttribute("aria-selected"))).toEqual([
      "true",
      "false",
      "false",
    ]);
    expect(tabs.map((tab) => tab.tabIndex)).toEqual([0, -1, -1]);
  });

  /** Given a tab with a count, then the count is shown and is part of the tab's name. */
  it("shows a count as part of the tab's name", () => {
    render(<Harness />);
    expect(screen.getByRole("tab", { name: "Players 3915" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Offers" })).toBeInTheDocument();
  });

  /** Given the tabs, when one is clicked, then onSelect gets its id. */
  it("selects a tab on click", () => {
    const onSelect = vi.fn();
    render(<Tabs idPrefix="t" label="Tabs" tabs={TABS} selectedId="players" onSelect={onSelect} />);
    fireEvent.click(screen.getByRole("tab", { name: /Offers/ }));
    expect(onSelect).toHaveBeenCalledWith("offers");
  });

  /**
   * Given the first tab focused,
   * when the arrow keys, Home and End are pressed,
   * then selection and focus move with them, wrapping at both ends.
   */
  it("moves selection and focus with the arrow keys, Home and End", () => {
    render(<Harness />);
    const first = screen.getByRole("tab", { name: /Players/ });
    first.focus();

    fireEvent.keyDown(first, { key: "ArrowRight" });
    const second = screen.getByRole("tab", { name: /My list/ });
    expect(second).toHaveAttribute("aria-selected", "true");
    expect(second).toHaveFocus();

    fireEvent.keyDown(second, { key: "End" });
    const last = screen.getByRole("tab", { name: /Offers/ });
    expect(last).toHaveFocus();

    fireEvent.keyDown(last, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: /Players/ })).toHaveFocus();

    fireEvent.keyDown(screen.getByRole("tab", { name: /Players/ }), { key: "ArrowLeft" });
    expect(screen.getByRole("tab", { name: /Offers/ })).toHaveAttribute("aria-selected", "true");

    fireEvent.keyDown(screen.getByRole("tab", { name: /Offers/ }), { key: "Home" });
    expect(screen.getByRole("tab", { name: /Players/ })).toHaveFocus();
  });

  /**
   * Given a tab and its panel,
   * then the tab controls the panel and the panel is labelled by the tab, and only the selected
   * panel is shown.
   */
  it("ties each tab to its panel", () => {
    render(<Harness />);
    const tab = screen.getByRole("tab", { name: /Players/ });
    const panel = screen.getByRole("tabpanel", { name: /Players/ });
    expect(tab).toHaveAttribute("aria-controls", panel.id);
    expect(panel).toHaveTextContent("players content");
    expect(screen.queryByText("offers content")).not.toBeVisible();
  });

  /**
   * Given a panel that opens with plain text,
   * then the panel itself is in the tab order, so the keyboard can reach its content.
   */
  it("puts the panel in the tab order", () => {
    render(<Harness />);
    expect(screen.getByRole("tabpanel", { name: /Players/ })).toHaveAttribute("tabindex", "0");
  });

  /** Given the segmented look, then the semantics and keyboard are the same as underline. */
  it("keeps the same semantics in the segmented look", () => {
    render(<Harness look="segmented" />);
    const first = screen.getByRole("tab", { name: /Players/ });
    first.focus();
    fireEvent.keyDown(first, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: /My list/ })).toHaveAttribute("aria-selected", "true");
  });
});
