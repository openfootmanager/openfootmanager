import { describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { Select } from "./Select";

/**
 * #568 / #361 — the open list is navigated, not committed to.
 * A controlled parent that really re-renders is used so a commit that is
 * swallowed by an uncontrolled shortcut cannot pass.
 */
function Harness({ onCommit }: { onCommit: (value: string) => void }) {
  const [value, setValue] = useState("442");
  return (
    <Select
      value={value}
      aria-label="Formation"
      onChange={(event) => {
        onCommit(event.target.value);
        setValue(event.target.value);
      }}
    >
      <option value="442">4-4-2</option>
      <option value="433">4-3-3</option>
      <option value="4231" disabled>
        4-2-3-1
      </option>
      <option value="352">3-5-2</option>
    </Select>
  );
}

const trigger = () => screen.getByRole("combobox", { name: "Formation" });

describe("Select keyboard model", () => {
  /**
   * Given a closed Select on 4-4-2
   * When the user presses ArrowDown twice then Enter
   * Then the list opens on the first press, the highlight moves on the second,
   *      and only Enter commits (exactly once)
   */
  it("arrows browse the open list and only Enter commits", () => {
    const onCommit = vi.fn();
    render(<Harness onCommit={onCommit} />);

    fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(onCommit).not.toHaveBeenCalled();

    fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(onCommit).not.toHaveBeenCalled();
    expect(trigger()).toHaveTextContent("4-4-2");

    fireEvent.keyDown(trigger(), { key: "Enter" });
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith("433");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(trigger()).toHaveTextContent("4-3-3");
  });

  /**
   * Given an open list with a highlighted option
   * When the user presses Space
   * Then that option is committed and the list closes
   */
  it("Space commits the highlighted option", () => {
    const onCommit = vi.fn();
    render(<Harness onCommit={onCommit} />);

    fireEvent.keyDown(trigger(), { key: "Enter" });
    fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    fireEvent.keyDown(trigger(), { key: " " });

    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith("433");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  /**
   * Given an open list after the user browsed to another option
   * When the user presses Escape
   * Then nothing is committed and the value is unchanged
   */
  it("Escape closes without committing", () => {
    const onCommit = vi.fn();
    render(<Harness onCommit={onCommit} />);

    fireEvent.keyDown(trigger(), { key: "Enter" });
    fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    fireEvent.keyDown(trigger(), { key: "Escape" });

    expect(onCommit).not.toHaveBeenCalled();
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(trigger()).toHaveTextContent("4-4-2");
  });

  /**
   * Given an open list
   * When the user presses ArrowDown past a disabled option, then Home and End
   * Then the highlight skips the disabled option and Home/End jump to the ends
   */
  it("skips disabled options and supports Home and End", () => {
    render(<Harness onCommit={vi.fn()} />);

    fireEvent.keyDown(trigger(), { key: "Enter" });
    fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    expect(trigger().getAttribute("aria-activedescendant")).toBe(
      screen.getByRole("option", { name: "3-5-2" }).id,
    );

    fireEvent.keyDown(trigger(), { key: "Home" });
    expect(trigger().getAttribute("aria-activedescendant")).toBe(
      screen.getByRole("option", { name: "4-4-2" }).id,
    );
    fireEvent.keyDown(trigger(), { key: "End" });
    expect(trigger().getAttribute("aria-activedescendant")).toBe(
      screen.getByRole("option", { name: "3-5-2" }).id,
    );
  });

  /**
   * Given a screen-reader user on the combobox
   * When the list is open and the highlight moves
   * Then aria-activedescendant points at an existing option id, so the
   *      newly active option is announced (DOM focus stays on the trigger)
   */
  it("announces the active option through aria-activedescendant", () => {
    render(<Harness onCommit={vi.fn()} />);

    expect(trigger()).not.toHaveAttribute("aria-activedescendant");
    fireEvent.keyDown(trigger(), { key: "Enter" });

    const options = screen.getAllByRole("option");
    expect(options.every((option) => option.id !== "")).toBe(true);
    expect(trigger().getAttribute("aria-activedescendant")).toBe(options[0].id);

    fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    expect(trigger().getAttribute("aria-activedescendant")).toBe(options[1].id);
  });

  /**
   * Given a closed Select
   * When the user clicks an option with the mouse
   * Then it still commits and closes
   */
  it("still commits on click", () => {
    const onCommit = vi.fn();
    render(<Harness onCommit={onCommit} />);

    fireEvent.click(trigger());
    fireEvent.click(screen.getByRole("option", { name: "3-5-2" }));

    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith("352");
  });
});
