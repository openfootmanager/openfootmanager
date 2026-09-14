import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TeamCombobox } from "./TeamCombobox";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe("TeamCombobox", () => {
  it("clears the selected team with Enter on the no-team option", () => {
    const onChange = vi.fn();

    render(
      <TeamCombobox
        label="Club"
        value="team-1"
        options={[{ id: "team-1", label: "Alpha FC" }]}
        onChange={onChange}
        placeholder="No club"
      />,
    );

    fireEvent.mouseDown(screen.getByRole("button", { name: "Club" }));
    const search = screen.getByRole("combobox", { name: "Club" });
    fireEvent.keyDown(search, { key: "Enter" });

    expect(onChange).toHaveBeenCalledWith("");
  });
});