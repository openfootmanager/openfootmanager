import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SortControl } from "./SortControl";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const OPTIONS = [
  { key: "name", label: "Name" },
  { key: "age", label: "Age" },
  { key: "ovr", label: "OVR" },
] as const;

function renderControl(over: Partial<React.ComponentProps<typeof SortControl>> = {}) {
  const onChange = vi.fn();
  render(
    <SortControl
      options={OPTIONS}
      sortKey="ovr"
      ascending={false}
      onChange={onChange}
      defaultAscending={(key) => key === "name"}
      {...over}
    />,
  );
  return onChange;
}

describe("SortControl", () => {
  /**
   * Given the list sorted by OVR, descending
   * When the field is changed to Name
   * Then onChange is asked for Name in the direction a name starts in (A to Z).
   */
  it("starts a newly chosen field in its own default direction", () => {
    const onChange = renderControl();

    fireEvent.click(screen.getByRole("combobox", { name: "common.sortBy" }));
    fireEvent.click(screen.getByRole("option", { name: "Name" }));

    expect(onChange).toHaveBeenCalledWith("name", true);
  });

  /**
   * Given the list sorted by OVR, descending
   * When the direction button is chosen
   * Then the same field is asked for in the other direction.
   */
  it("flips the direction of the current field", () => {
    const onChange = renderControl();

    fireEvent.click(screen.getByRole("button", { name: "common.sortAscending" }));

    expect(onChange).toHaveBeenCalledWith("ovr", true);
  });

  /**
   * Given a list sorted descending, and one sorted ascending
   * When the direction toggle is read
   * Then it is one control named "Ascending" whose pressed state is the direction (not pressed
   *      when descending), so a screen reader hears the state as well as the action.
   */
  it("states the direction as the toggle's pressed state", () => {
    const { rerender } = render(
      <SortControl options={OPTIONS} sortKey="ovr" ascending={false} onChange={vi.fn()} />,
    );
    expect(screen.getByRole("button", { name: "common.sortAscending" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );

    rerender(<SortControl options={OPTIONS} sortKey="ovr" ascending onChange={vi.fn()} />);
    expect(screen.getByRole("button", { name: "common.sortAscending" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  /**
   * Given no default-direction rule
   * When the field is changed
   * Then the direction the list already had is kept.
   */
  it("keeps the current direction when it has no default rule", () => {
    const onChange = renderControl({ defaultAscending: undefined, ascending: true });

    fireEvent.click(screen.getByRole("combobox", { name: "common.sortBy" }));
    fireEvent.click(screen.getByRole("option", { name: "Age" }));

    expect(onChange).toHaveBeenCalledWith("age", true);
  });
});
