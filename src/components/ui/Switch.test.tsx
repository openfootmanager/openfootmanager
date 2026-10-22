import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Switch } from "./Switch";

describe("Switch", () => {
  /**
   * Given a setting that is on, named by its visible label
   * When a screen reader reaches it
   * Then it is a switch with that name, announced as on.
   */
  it("is a switch named by its label and says whether it is on", () => {
    render(
      <>
        <p id="hc">High contrast</p>
        <Switch labelledBy="hc" checked onChange={vi.fn()} />
      </>,
    );
    expect(screen.getByRole("switch", { name: "High contrast" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  /** Given a switch with no visible label beside it, then it can be named by a label instead. */
  it("can be named by a label", () => {
    render(<Switch label="Auto-save" checked={false} onChange={vi.fn()} />);
    expect(screen.getByRole("switch", { name: "Auto-save" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
  });

  /**
   * Given a switch that is off
   * When the player activates it (a click; Space and Enter activate a button the same way)
   * Then it asks to be on.
   */
  it("asks for the other state when activated", () => {
    const onChange = vi.fn();
    render(<Switch label="Auto-save" checked={false} onChange={onChange} />);
    fireEvent.click(screen.getByRole("switch", { name: "Auto-save" }));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  /** Given a disabled switch, then it says so and asks for nothing. */
  it("asks for nothing while disabled", () => {
    const onChange = vi.fn();
    render(<Switch label="Auto-save" checked onChange={onChange} disabled />);
    const control = screen.getByRole("switch", { name: "Auto-save" });
    expect(control).toBeDisabled();
    fireEvent.click(control);
    expect(onChange).not.toHaveBeenCalled();
  });

  /** Given keyboard focus, then the switch draws the one focus ring every control shares. */
  it("draws the shared focus ring", () => {
    render(<Switch label="Auto-save" checked={false} onChange={vi.fn()} />);
    expect(screen.getByRole("switch")).toHaveClass(
      "focus-visible:ring-2",
      "focus-visible:ring-focus",
    );
  });
});
