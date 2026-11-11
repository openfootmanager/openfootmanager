import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TeamCombobox } from "./TeamCombobox";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

function teamOptions(count: number) {
  return Array.from({ length: count }, (_, i) => ({ id: `t${i}`, label: `Team ${i}` }));
}

function renderCombobox(options = teamOptions(5), value = "") {
  const onChange = vi.fn();
  render(
    <TeamCombobox
      label="Club"
      value={value}
      options={options}
      onChange={onChange}
      placeholder="No club"
    />,
  );
  return { onChange };
}

const openIt = () => fireEvent.mouseDown(screen.getByRole("button", { name: "Club" }));
const search = () => screen.getByRole("combobox", { name: "Club" });
/** The option the search field's aria-activedescendant names, or null when it names none that is in the page. */
function activeOption() {
  const id = search().getAttribute("aria-activedescendant");
  return id ? document.getElementById(id) : null;
}

describe("TeamCombobox", () => {
  /**
   * Given a team selected
   * When the no-team option is active and Enter is pressed
   * Then the club is cleared
   */
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
    fireEvent.keyDown(search(), { key: "Enter" });

    expect(onChange).toHaveBeenCalledWith("");
  });

  /**
   * Given the popover just opened
   * When ArrowDown is pressed twice
   * Then the active option moves to the second team, and Enter picks it
   */
  it("moves the active option with the arrow keys and picks it with Enter", () => {
    const { onChange } = renderCombobox();
    openIt();
    expect(activeOption()).toHaveTextContent("No club");

    fireEvent.keyDown(search(), { key: "ArrowDown" });
    fireEvent.keyDown(search(), { key: "ArrowDown" });
    expect(activeOption()).toHaveTextContent("Team 1");
    fireEvent.keyDown(search(), { key: "Enter" });

    expect(onChange).toHaveBeenCalledWith("t1");
  });

  /**
   * Given the no-team option active
   * When ArrowUp is pressed
   * Then the active option wraps to the last team; and ArrowDown from the last wraps back to no-team
   */
  it("wraps from the first option to the last and back", () => {
    renderCombobox();
    openIt();

    fireEvent.keyDown(search(), { key: "ArrowUp" });
    expect(activeOption()).toHaveTextContent("Team 4");

    fireEvent.keyDown(search(), { key: "ArrowDown" });
    expect(activeOption()).toHaveTextContent("No club");
  });

  /**
   * Given a popover open on the first option
   * When End and then Home are pressed
   * Then the last team is active, then the no-team option
   */
  it("jumps to the last option with End and the first with Home", () => {
    renderCombobox();
    openIt();

    fireEvent.keyDown(search(), { key: "End" });
    expect(activeOption()).toHaveTextContent("Team 4");
    fireEvent.keyDown(search(), { key: "Home" });
    expect(activeOption()).toHaveTextContent("No club");
  });

  /**
   * Given an open popover
   * When Escape is pressed
   * Then it closes and nothing is picked
   */
  it("closes with Escape without picking anything", () => {
    const { onChange } = renderCombobox();
    openIt();

    fireEvent.keyDown(search(), { key: "Escape" });

    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
  });

  /**
   * Given an open popover
   * When something is typed in the search field
   * Then only the matching teams are listed (accents and case ignored), and the first is active
   */
  it("filters the options by the search, ignoring case and accents", () => {
    renderCombobox([
      { id: "a", label: "Atlético Norte" },
      { id: "b", label: "Beta United" },
    ]);
    openIt();

    fireEvent.change(search(), { target: { value: "ATLETICO" } });

    const labels = screen.getAllByRole("option").map((option) => option.textContent);
    expect(labels).toEqual(["No club", "Atlético Norte"]);
  });

  /**
   * Given a search that matches nothing
   * When the list is read
   * Then it says so, and Enter does nothing
   */
  it("says so when nothing matches", () => {
    const { onChange } = renderCombobox();
    openIt();

    fireEvent.change(search(), { target: { value: "zzz" } });
    fireEvent.keyDown(search(), { key: "Enter" });

    expect(screen.getByText("teams.noResults")).toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
  });

  /**
   * Given an open popover
   * When a team is clicked
   * Then it is picked and the popover closes
   */
  it("picks a team on click and closes", () => {
    const { onChange } = renderCombobox();
    openIt();

    fireEvent.mouseDown(screen.getByRole("option", { name: "Team 3" }));

    expect(onChange).toHaveBeenCalledWith("t3");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  /**
   * Given the chosen team
   * When the popover is open
   * Then exactly that option is marked selected
   */
  it("marks the chosen team as selected", () => {
    renderCombobox(teamOptions(5), "t2");
    openIt();

    const selected = screen
      .getAllByRole("option")
      .filter((o) => o.getAttribute("aria-selected") === "true");
    expect(selected.map((o) => o.textContent)).toEqual(["Team 2"]);
  });
});
