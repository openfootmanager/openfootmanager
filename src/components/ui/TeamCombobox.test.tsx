import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { installScrollGeometry } from "../../test-utils/scrollGeometry";
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

describe("TeamCombobox: composition and empty results", () => {
  /**
   * Given an IME composition in progress in the search field (Enter confirms the composed text)
   * When Enter arrives with isComposing set, or the legacy keyCode 229
   * Then it does not pick the active club
   */
  it("ignores Enter while a composition is in progress", () => {
    const { onChange } = renderCombobox();
    openIt();

    fireEvent.keyDown(search(), { key: "Enter", isComposing: true });
    fireEvent.keyDown(search(), { key: "Enter", keyCode: 229 });

    expect(onChange).not.toHaveBeenCalled();
  });

  /**
   * Given a search that matches nothing
   * When the search field is read
   * Then the listbox it controls is still in the page (empty), and "no results" is announced as status
   */
  it("keeps the listbox its aria-controls names when nothing matches", () => {
    renderCombobox();
    openIt();

    fireEvent.change(search(), { target: { value: "zzz" } });

    const listbox = screen.getByRole("listbox");
    expect(search().getAttribute("aria-controls")).toBe(listbox.id);
    expect(screen.getByRole("status")).toHaveTextContent("teams.noResults");
  });
});

describe("TeamCombobox with a very long list", () => {
  // jsdom lays nothing out: the fixture gives the listbox's scroller its 12rem (192 px) height.
  let restoreGeometry: () => void;
  beforeEach(() => {
    restoreGeometry = installScrollGeometry({ viewportHeight: 192 });
  });
  afterEach(() => restoreGeometry());

  const many = () => teamOptions(582);

  /**
   * Given a club picker over 582 clubs (the mega-sized package)
   * When it is opened
   * Then it renders a window of options, not 583 of them, and each says its place in the whole list
   */
  it("renders a window of the options, not every club", () => {
    renderCombobox(many());
    openIt();

    const rendered = screen.getAllByRole("option");
    expect(rendered.length).toBeGreaterThan(5);
    expect(rendered.length).toBeLessThan(30);
    expect(rendered[0]).toHaveAttribute("aria-setsize", "583");
    expect(rendered[0]).toHaveAttribute("aria-posinset", "1");
  });

  /**
   * Given the popover open on the first option of 583
   * When End is pressed
   * Then the last club is active AND in the page, so aria-activedescendant names a real element, and
   *      Enter picks it
   */
  it("brings the last option into the page for End, and picks it", async () => {
    const { onChange } = renderCombobox(many());
    openIt();

    fireEvent.keyDown(search(), { key: "End" });

    await waitFor(() => expect(activeOption()).toHaveTextContent("Team 581"));
    fireEvent.keyDown(search(), { key: "Enter" });
    expect(onChange).toHaveBeenCalledWith("t581");
  });

  /**
   * Given the first option active
   * When ArrowUp wraps to the end
   * Then the last club is in the page and active
   */
  it("keeps the active option in the page when ArrowUp wraps to the last", async () => {
    renderCombobox(many());
    openIt();

    fireEvent.keyDown(search(), { key: "ArrowUp" });

    await waitFor(() => expect(activeOption()).toHaveTextContent("Team 581"));
    expect(screen.getAllByRole("option").length).toBeLessThan(30);
  });

  /**
   * Given a long list scrolled down
   * When an option that scrolled into the window is clicked
   * Then that club is picked
   */
  it("picks an option that was scrolled to", async () => {
    const { onChange } = renderCombobox(many());
    openIt();
    const scroller = screen.getByRole("listbox").parentElement as HTMLElement;

    scroller.scrollTop = 100 * 36;
    fireEvent.scroll(scroller);
    await waitFor(() =>
      expect(screen.getByRole("option", { name: "Team 100" })).toBeInTheDocument(),
    );
    fireEvent.mouseDown(screen.getByRole("option", { name: "Team 100" }));

    expect(onChange).toHaveBeenCalledWith("t100");
  });

  /**
   * Given a long list scrolled far down
   * When the search narrows it
   * Then the list is back at its first option, not left among rows that are gone
   */
  it("returns to the top when the search changes", async () => {
    renderCombobox(many());
    openIt();
    const scroller = screen.getByRole("listbox").parentElement as HTMLElement;
    scroller.scrollTop = 300 * 36;
    fireEvent.scroll(scroller);
    await waitFor(() =>
      expect(screen.getByRole("option", { name: "Team 300" })).toBeInTheDocument(),
    );

    fireEvent.change(search(), { target: { value: "Team 5" } });

    await waitFor(() =>
      expect(screen.getByRole("option", { name: "No club" })).toBeInTheDocument(),
    );
    expect(scroller.scrollTop).toBe(0);
  });

  /**
   * Given option 0 active (the search field's aria-activedescendant names it)
   * When the list is scrolled by hand far away from it
   * Then option 0 is still in the page: the id the field names always names an element
   */
  it("keeps the active option mounted while the list is scrolled by hand", async () => {
    renderCombobox(many());
    openIt();
    const scroller = screen.getByRole("listbox").parentElement as HTMLElement;
    const activeId = search().getAttribute("aria-activedescendant") as string;
    expect(document.getElementById(activeId)).not.toBeNull();

    scroller.scrollTop = 300 * 36;
    fireEvent.scroll(scroller);
    await waitFor(() =>
      expect(screen.getByRole("option", { name: "Team 300" })).toBeInTheDocument(),
    );

    expect(document.getElementById(activeId)).not.toBeNull();
  });
});
