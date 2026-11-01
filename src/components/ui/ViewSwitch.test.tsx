import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ViewSwitch } from "./ViewSwitch";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("ViewSwitch", () => {
  /**
   * Given a list shown as a table
   * When a screen reader reaches the switch above it
   * Then it is a radio group named for what it does, with Table chosen and Cards beside it.
   */
  it("is a named choice between table and cards, on the current view", () => {
    render(<ViewSwitch value="table" onChange={vi.fn()} />);
    expect(screen.getByRole("radiogroup", { name: "common.listViewLabel" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "common.listViewTable" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "common.listViewCards" })).not.toBeChecked();
  });

  /** Given the table view, when the player picks Cards, then the switch asks for cards. */
  it("asks for the other view", () => {
    const onChange = vi.fn();
    render(<ViewSwitch value="table" onChange={onChange} />);
    fireEvent.click(screen.getByRole("radio", { name: "common.listViewCards" }));
    expect(onChange).toHaveBeenCalledWith("cards");
  });
});
