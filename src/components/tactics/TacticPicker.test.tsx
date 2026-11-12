import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TacticPicker, { type TacticsLibraryEntry } from "./TacticPicker";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, unknown>) =>
      typeof fallback === "string" ? fallback : key,
    i18n: { language: "en" },
  }),
}));

const entry = (
  id: string,
  name: string,
  type: TacticsLibraryEntry["type"],
  formation = "4-4-2",
): TacticsLibraryEntry => ({
  description: `${name} description`,
  formation,
  id,
  name,
  playStyle: "Balanced",
  sourcePresetName: null,
  type,
});

const mine = entry("custom:1", "My Tactic", "custom");
const control = entry("preset:control", "Balanced Control", "preset", "4-3-3");
const press = entry("preset:press", "High Press", "preset", "4-2-3-1");

/** The picker as the bar wires it: the bar owns whether the list is open. */
function Harness({ onSelect = () => {} }: { onSelect?: (id: string) => void }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <TacticPicker
        activeTactic={mine}
        library={[mine, control, press]}
        open={open}
        onOpenChange={setOpen}
        onSelect={onSelect}
      />
      <button type="button" onClick={() => setOpen(false)}>
        shut from outside
      </button>
    </>
  );
}

const trigger = () => screen.getByRole("button", { name: "tactics.chooseTactic" });
const press_ = (key: string) => fireEvent.keyDown(document.activeElement ?? document.body, { key });

function openPicker() {
  fireEvent.click(trigger());
  return screen.getByRole("textbox", { name: "tactics.searchTactics" });
}

describe("TacticPicker", () => {
  /**
   * Given the picker closed
   * When it is opened
   * Then the list shows the saved tactic and the presets, the button says it is expanded, and
   *      the search has focus so typing filters at once.
   */
  it("opens a list of the saved tactics and the presets, with focus in the search", () => {
    render(<Harness />);
    expect(trigger()).toHaveAttribute("aria-expanded", "false");

    const search = openPicker();

    expect(trigger()).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getAllByRole("option").map((option) => option.textContent?.split(" description")[0]),
    ).toEqual(["My Tactic", "Balanced Control", "High Press"]);
    expect(search).toHaveFocus();
  });

  /**
   * Given the open list
   * When the manager types a formation in the search
   * Then only the tactics that match remain.
   */
  it("filters the list by what is typed in the search", () => {
    render(<Harness />);
    const search = openPicker();

    fireEvent.change(search, { target: { value: "4-3-3" } });

    expect(screen.getAllByRole("option")).toHaveLength(1);
    expect(screen.getByRole("option", { name: /Balanced Control/ })).toBeInTheDocument();
  });

  /**
   * Given the open list with focus in the search
   * When ArrowDown is pressed twice, then ArrowUp twice
   * Then focus walks the options in order and back to the search, and stops there (no wrap).
   */
  it("walks from the search through the options with ArrowDown and ArrowUp", () => {
    render(<Harness />);
    const search = openPicker();
    const options = screen.getAllByRole("option");

    press_("ArrowDown");
    expect(options[0]).toHaveFocus();
    press_("ArrowDown");
    expect(options[1]).toHaveFocus();
    press_("ArrowUp");
    expect(options[0]).toHaveFocus();
    press_("ArrowUp");
    expect(search).toHaveFocus();
    press_("ArrowUp");
    expect(search).toHaveFocus();
  });

  /**
   * Given the open list with focus on the second option
   * When it is activated (Enter on a button is its click)
   * Then that tactic is chosen, the list shuts, and focus returns to the picker's button.
   */
  it("chooses the option reached by the arrow keys and returns focus to the button", () => {
    const onSelect = vi.fn();
    render(<Harness onSelect={onSelect} />);
    openPicker();

    press_("ArrowDown");
    press_("ArrowDown");
    fireEvent.click(document.activeElement ?? document.body);

    expect(onSelect).toHaveBeenCalledWith("preset:control");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(trigger()).toHaveFocus();
  });

  /**
   * Given the open list
   * When Escape is pressed
   * Then it shuts without choosing anything, and focus returns to the picker's button.
   */
  it("shuts with Escape, choosing nothing, with focus back on the button", () => {
    const onSelect = vi.fn();
    render(<Harness onSelect={onSelect} />);
    openPicker();

    press_("Escape");

    expect(screen.queryByRole("listbox")).toBeNull();
    expect(onSelect).not.toHaveBeenCalled();
    expect(trigger()).toHaveFocus();
  });

  /**
   * Given the open list
   * When the pointer goes down outside the picker
   * Then it shuts, and focus is not pulled back to the button (the manager went elsewhere).
   */
  it("shuts on a press outside without taking focus back", () => {
    render(<Harness />);
    openPicker();
    const elsewhere = screen.getByRole("button", { name: "shut from outside" });
    elsewhere.focus();

    fireEvent.mouseDown(elsewhere);

    expect(screen.queryByRole("listbox")).toBeNull();
    expect(elsewhere).toHaveFocus();
  });

  /**
   * Given a picker whose button shows the active tactic
   * When a screen reader reads it
   * Then its name is the field's ("Choose tactic") and its description is the active tactic, so
   *      the selected tactic is heard and not only seen.
   */
  it("describes the button with the active tactic's name and summary", () => {
    render(<Harness />);

    const describedBy = trigger().getAttribute("aria-describedby");
    expect(describedBy).not.toBeNull();
    const description = document.getElementById(describedBy ?? "");
    expect(description).toHaveTextContent("My Tactic");
    expect(description).toHaveTextContent("4-4-2 - Balanced");
  });

  /**
   * Given an open list with a search typed
   * When the list is shut and opened again
   * Then the search starts empty and every tactic shows.
   */
  it("forgets the search when the list shuts", () => {
    render(<Harness />);
    fireEvent.change(openPicker(), { target: { value: "press" } });
    expect(screen.getAllByRole("option")).toHaveLength(1);

    fireEvent.click(screen.getByRole("button", { name: "shut from outside" }));
    fireEvent.click(trigger());

    expect(screen.getAllByRole("option")).toHaveLength(3);
  });
});
