import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { SegmentedControl } from "./SegmentedControl";

const PACES = [
  { value: "slow", label: "Slow" },
  { value: "normal", label: "Normal" },
  { value: "fast", label: "Fast" },
  { value: "max", label: "Max" },
] as const;
type Pace = (typeof PACES)[number]["value"];

function Harness({ emphasis }: { emphasis?: "quiet" | "strong" }) {
  const [pace, setPace] = useState<Pace>("normal");
  return (
    <>
      <span id="speed-label">Speed</span>
      <SegmentedControl
        labelledBy="speed-label"
        options={PACES}
        value={pace}
        onChange={setPace}
        emphasis={emphasis}
      />
    </>
  );
}

describe("SegmentedControl", () => {
  /**
   * Given four options with Normal chosen,
   * when they render,
   * then they are a radio group named by its visible label, Normal is checked and is the one tab stop.
   */
  it("is a radio group named by its label, with the chosen option the one tab stop", () => {
    render(<Harness />);
    expect(screen.getByRole("radiogroup", { name: "Speed" })).toBeInTheDocument();
    const radios = screen.getAllByRole("radio");
    expect(radios.map((radio) => radio.getAttribute("aria-checked"))).toEqual([
      "false",
      "true",
      "false",
      "false",
    ]);
    expect(radios.map((radio) => radio.tabIndex)).toEqual([-1, 0, -1, -1]);
  });

  /** Given the options, when one is clicked, then onChange gets its value. */
  it("chooses an option on click", () => {
    const onChange = vi.fn();
    render(<SegmentedControl labelledBy="x" options={PACES} value="normal" onChange={onChange} />);
    fireEvent.click(screen.getByRole("radio", { name: "Fast" }));
    expect(onChange).toHaveBeenCalledWith("fast");
  });

  /**
   * Given Normal focused,
   * when the arrow keys (up and down too, as in any radio group), Home and End are pressed,
   * then the choice and the focus move together, wrapping at both ends.
   */
  it("moves the choice and the focus with the arrow keys", () => {
    render(<Harness />);
    const normal = screen.getByRole("radio", { name: "Normal" });
    normal.focus();

    fireEvent.keyDown(normal, { key: "ArrowDown" });
    expect(screen.getByRole("radio", { name: "Fast" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("radio", { name: "Fast" })).toHaveFocus();

    fireEvent.keyDown(screen.getByRole("radio", { name: "Fast" }), { key: "End" });
    fireEvent.keyDown(screen.getByRole("radio", { name: "Max" }), { key: "ArrowRight" });
    expect(screen.getByRole("radio", { name: "Slow" })).toHaveFocus();
  });

  /**
   * Given the strong emphasis (a setting the player watches, like the match speed),
   * then the chosen option is filled with the brand colour; given the quiet one, it is raised.
   */
  it("fills the chosen option when strong and raises it when quiet", () => {
    const { unmount } = render(<Harness emphasis="strong" />);
    expect(screen.getByRole("radio", { name: "Normal" })).toHaveClass("bg-primary-700");
    unmount();
    render(<Harness emphasis="quiet" />);
    expect(screen.getByRole("radio", { name: "Normal" })).toHaveClass("bg-panel");
  });

  /**
   * Given a control filling its width (the match speed in a narrow side panel)
   * When its labels are too long for one row (Russian, or German at a large text size)
   * Then the options wrap onto another row instead of scrolling one out of sight.
   */
  it("wraps a filled control's options instead of scrolling them", () => {
    render(
      <SegmentedControl labelledBy="x" options={PACES} value="normal" onChange={vi.fn()} fill />,
    );
    const track = screen.getByRole("radiogroup");
    expect(track).toHaveClass("flex-wrap");
    expect(track).not.toHaveClass("overflow-x-auto");
    expect(track).not.toHaveClass("inline-flex");
    expect(screen.getByRole("radio", { name: "Fast" })).toHaveClass("flex-auto");
  });

  /** Given a value no option has, then the first option is the tab stop, so the group can be reached. */
  it("keeps a tab stop when the value matches no option", () => {
    render(
      <SegmentedControl
        labelledBy="x"
        options={PACES}
        value={"paused" as Pace}
        onChange={vi.fn()}
      />,
    );
    expect(screen.getAllByRole("radio").map((radio) => radio.tabIndex)).toEqual([0, -1, -1, -1]);
  });
});
