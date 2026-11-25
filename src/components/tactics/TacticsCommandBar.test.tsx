import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { TacticsLibraryEntry } from "./TacticPicker";
import TacticsCommandBar from "./TacticsCommandBar";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, unknown>) =>
      typeof fallback === "string" ? fallback : key,
    i18n: { language: "en" },
  }),
}));

afterEach(() => {
  vi.useRealTimers();
});

const customTactic: TacticsLibraryEntry = {
  description: "A custom tactic",
  formation: "4-4-2",
  id: "custom:1",
  name: "My Tactic",
  playStyle: "Balanced",
  sourcePresetName: null,
  type: "custom",
};

const presetTactic: TacticsLibraryEntry = {
  description: "A preset tactic",
  formation: "4-3-3",
  id: "preset:balanced-control",
  name: "Balanced Control",
  playStyle: "Balanced",
  sourcePresetName: null,
  type: "preset",
};

function buildProps(
  overrides: Partial<React.ComponentProps<typeof TacticsCommandBar>> = {},
): React.ComponentProps<typeof TacticsCommandBar> {
  return {
    activeTactic: customTactic,
    activePlayStyle: "Balanced",
    formation: "4-4-2",
    isDirty: false,
    libraryLoaded: true,
    canDelete: true,
    canRename: true,
    onRename: vi.fn().mockResolvedValue(null),
    onCreateNew: vi.fn(),
    onDelete: vi.fn(),
    onDuplicate: vi.fn(),
    onFormationChange: vi.fn(),
    onPlayStyleChange: vi.fn(),
    onReset: vi.fn(),
    canReset: true,
    onSave: vi.fn().mockResolvedValue(true),
    onSelectTactic: vi.fn(),
    tacticLibrary: [customTactic, presetTactic],
    ...overrides,
  };
}

/** Opens the bar's one actions menu (New, Duplicate, Reset, Delete live in it). */
function openActionsMenu() {
  fireEvent.click(screen.getByRole("button", { name: "tactics.tacticActions" }));
}

function renderCommandBar(overrides: Partial<React.ComponentProps<typeof TacticsCommandBar>> = {}) {
  return render(<TacticsCommandBar {...buildProps(overrides)} />);
}

describe("TacticsCommandBar", () => {
  it("disables the save button when the active custom tactic is already synced", () => {
    renderCommandBar({ activeTactic: customTactic, isDirty: false });

    expect(screen.getByRole("button", { name: "tactics.updateTactic" })).toBeDisabled();
  });

  it("enables the save button once the active custom tactic has unsaved changes", () => {
    renderCommandBar({ activeTactic: customTactic, isDirty: true });

    expect(screen.getByRole("button", { name: "tactics.updateTactic" })).toBeEnabled();
  });

  it("keeps the save button enabled for a preset even when nothing changed", () => {
    // Saving a preset always creates a new custom tactic, so it is never a
    // no-op even when isDirty is false.
    renderCommandBar({ activeTactic: presetTactic, isDirty: false });

    expect(screen.getByRole("button", { name: "tactics.saveAsTactic" })).toBeEnabled();
  });

  it("shows a temporary Saved confirmation after a successful save click", async () => {
    vi.useFakeTimers();
    const onSave = vi.fn().mockResolvedValue(true);
    const props = buildProps({ activeTactic: customTactic, isDirty: true, onSave });

    const { rerender } = render(<TacticsCommandBar {...props} />);

    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));
    await act(async () => {});
    expect(onSave).toHaveBeenCalledTimes(1);

    // A successful save syncs the stored tactic, so the parent re-renders
    // with isDirty flipped back to false.
    rerender(<TacticsCommandBar {...props} isDirty={false} />);

    expect(screen.getByRole("button", { name: "tactics.tacticSaved" })).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(2000);
    });

    expect(screen.getByRole("button", { name: "tactics.updateTactic" })).toBeInTheDocument();
  });

  /**
   * Given a save the backend has not answered yet
   * When the manager has clicked Save
   * Then no Saved cue shows until it answers that it saved.
   */
  it("shows the Saved cue only once the save has answered", async () => {
    vi.useFakeTimers();
    let answer: (saved: boolean) => void = () => {};
    const onSave = vi.fn(() => new Promise<boolean>((resolve) => (answer = resolve)));
    const props = buildProps({ activeTactic: customTactic, isDirty: true, onSave });
    const { rerender } = render(<TacticsCommandBar {...props} />);

    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));
    rerender(<TacticsCommandBar {...props} isDirty={false} />);
    expect(screen.queryByRole("button", { name: "tactics.tacticSaved" })).not.toBeInTheDocument();

    await act(async () => answer(true));

    expect(screen.getByRole("button", { name: "tactics.tacticSaved" })).toBeInTheDocument();
  });

  /**
   * Given a save the backend refused
   * When the answer lands
   * Then no Saved cue shows: the tactic did not save.
   */
  it("shows no Saved cue when the save was refused", async () => {
    vi.useFakeTimers();
    const onSave = vi.fn().mockResolvedValue(false);
    const props = buildProps({ activeTactic: customTactic, isDirty: true, onSave });
    const { rerender } = render(<TacticsCommandBar {...props} />);

    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));
    await act(async () => {});
    rerender(<TacticsCommandBar {...props} isDirty={false} />);

    expect(screen.queryByRole("button", { name: "tactics.tacticSaved" })).not.toBeInTheDocument();
  });

  /**
   * Given a Saved cue from a first save, and the tactic edited again within the cue's time
   * When the manager saves a second time and that save has not answered yet
   * Then the old cue is gone while the second save is pending (it must not claim the new save).
   */
  it("drops the old Saved cue while a second save is pending", async () => {
    vi.useFakeTimers();
    const answers: Array<(saved: boolean) => void> = [];
    const onSave = vi.fn(() => new Promise<boolean>((resolve) => answers.push(resolve)));
    const props = buildProps({ activeTactic: customTactic, isDirty: true, onSave });
    const { rerender } = render(<TacticsCommandBar {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));
    await act(async () => answers[0](true));
    rerender(<TacticsCommandBar {...props} isDirty={false} />);
    expect(screen.getByRole("button", { name: "tactics.tacticSaved" })).toBeInTheDocument();

    rerender(<TacticsCommandBar {...props} isDirty />);
    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));
    rerender(<TacticsCommandBar {...props} isDirty={false} />);

    expect(screen.queryByRole("button", { name: "tactics.tacticSaved" })).not.toBeInTheDocument();
    await act(async () => answers[1](true));
    expect(screen.getByRole("button", { name: "tactics.tacticSaved" })).toBeInTheDocument();
  });

  /**
   * Given two saves started before either answers
   * When the older one is saved and the newer one is refused
   * Then no Saved cue shows at any point: the older answer belongs to a save that has been
   *      superseded, and the newer one did not save.
   */
  it("shows no Saved cue for overlapping saves when the newer one is refused", async () => {
    vi.useFakeTimers();
    const answers: Array<(saved: boolean) => void> = [];
    const onSave = vi.fn(() => new Promise<boolean>((resolve) => answers.push(resolve)));
    const props = buildProps({ activeTactic: customTactic, isDirty: true, onSave });
    const { rerender } = render(<TacticsCommandBar {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));
    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));
    rerender(<TacticsCommandBar {...props} isDirty={false} />);

    await act(async () => answers[0](true));
    expect(screen.queryByRole("button", { name: "tactics.tacticSaved" })).not.toBeInTheDocument();
    await act(async () => answers[1](false));
    expect(screen.queryByRole("button", { name: "tactics.tacticSaved" })).not.toBeInTheDocument();
  });

  /**
   * Given a save of tactic A that has not answered
   * When the manager switches to another tactic, B, and A's answer then lands as saved
   * Then no Saved cue shows on B.
   */
  it("shows no Saved cue on another tactic when a late save answers", async () => {
    vi.useFakeTimers();
    let answer: (saved: boolean) => void = () => {};
    const onSave = vi.fn(() => new Promise<boolean>((resolve) => (answer = resolve)));
    const otherTactic = { ...customTactic, id: "custom:other", name: "Other" };
    const props = buildProps({
      activeTactic: customTactic,
      isDirty: true,
      onSave,
      tacticLibrary: [customTactic, otherTactic, presetTactic],
    });
    const { rerender } = render(<TacticsCommandBar {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));

    rerender(<TacticsCommandBar {...props} activeTactic={otherTactic} isDirty={false} />);
    await act(async () => answer(true));

    expect(screen.queryByRole("button", { name: "tactics.tacticSaved" })).not.toBeInTheDocument();
  });

  /**
   * Given a Saved cue showing
   * When the screen is read by assistive technology
   * Then the confirmation is in a polite live region, so it is announced.
   */
  it("announces the Saved cue in a polite live region", async () => {
    vi.useFakeTimers();
    const onSave = vi.fn().mockResolvedValue(true);
    const props = buildProps({ activeTactic: customTactic, isDirty: true, onSave });
    const { rerender } = render(<TacticsCommandBar {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));
    await act(async () => {});
    rerender(<TacticsCommandBar {...props} isDirty={false} />);

    const liveRegion = screen.getByRole("button", { name: "tactics.tacticSaved" })
      .nextElementSibling as HTMLElement;
    expect(liveRegion).toHaveAttribute("aria-live", "polite");
    expect(liveRegion).toHaveTextContent("tactics.tacticSaved");
  });

  it("clears the Saved cue immediately when new edits make the tactic dirty again mid-cue", async () => {
    vi.useFakeTimers();
    const onSave = vi.fn().mockResolvedValue(true);
    const props = buildProps({
      activeTactic: customTactic,
      isDirty: true,
      onSave,
    });

    const { rerender } = render(<TacticsCommandBar {...props} />);

    fireEvent.click(screen.getByRole("button", { name: "tactics.updateTactic" }));
    await act(async () => {});

    // The save synced the tactic — parent re-renders with isDirty: false —
    // so the "Saved" cue becomes visible.
    rerender(<TacticsCommandBar {...props} isDirty={false} />);
    expect(screen.getByRole("button", { name: "tactics.tacticSaved" })).toBeInTheDocument();

    // The user edits the tactic again before the 2s cue timeout fires.
    act(() => {
      vi.advanceTimersByTime(500);
    });
    rerender(<TacticsCommandBar {...props} isDirty />);

    const saveButton = screen.getByRole("button", {
      name: "tactics.updateTactic",
    });
    expect(saveButton).toBeInTheDocument();
    expect(saveButton).toBeEnabled();
    expect(screen.queryByRole("button", { name: "tactics.tacticSaved" })).not.toBeInTheDocument();
  });

  /**
   * Given a tactic that can be reset
   * When the player presses Reset
   * Then onReset is called.
   */
  it("calls onReset when Reset is pressed", () => {
    const onReset = vi.fn();
    renderCommandBar({ activeTactic: presetTactic, canReset: true, onReset });

    openActionsMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "tactics.resetTactic" }));

    expect(onReset).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a tactic with nothing to reset
   * When the command bar renders
   * Then Reset is disabled.
   */
  it("disables Reset when there is nothing to reset", () => {
    renderCommandBar({ activeTactic: customTactic, canReset: false });

    openActionsMenu();
    expect(screen.getByRole("menuitem", { name: "tactics.resetTactic" })).toBeDisabled();
  });

  /**
   * Given a saved tactic
   * When the bar renders
   * Then the picker, the formation, the play style, the status, the actions menu and Save are all in
   *      one group (one row of controls), and New, Duplicate, Reset and Delete are not on the bar:
   *      they wait in the actions menu.
   */
  it("holds the picker, formation, play style, actions and Save in one group, the other actions in a menu", () => {
    renderCommandBar();

    const bar = screen.getByRole("group", { name: "tactics.tacticBar" });
    for (const control of [
      screen.getByRole("button", { name: "tactics.chooseTactic" }),
      screen.getByRole("combobox", { name: "tactics.formation" }),
      screen.getByRole("combobox", { name: "tactics.playStyle" }),
      screen.getByRole("button", { name: "tactics.tacticActions" }),
      screen.getByRole("button", { name: "tactics.updateTactic" }),
    ]) {
      expect(bar).toContainElement(control);
    }
    expect(screen.queryByRole("button", { name: "tactics.newTactic" })).toBeNull();
    expect(screen.queryByRole("button", { name: "tactics.duplicateTactic" })).toBeNull();
    expect(screen.queryByRole("menuitem")).toBeNull();
  });

  /**
   * Given a saved tactic
   * When the actions menu is opened
   * Then it offers New, Duplicate, Rename, Reset and Delete, and the button says it opened the menu.
   */
  it("opens the actions menu with New, Duplicate, Rename, Reset and Delete", () => {
    renderCommandBar();
    const trigger = screen.getByRole("button", { name: "tactics.tacticActions" });
    expect(trigger).toHaveAttribute("aria-expanded", "false");

    fireEvent.click(trigger);

    expect(trigger).toHaveAttribute("aria-expanded", "true");
    const menu = screen.getByRole("menu");
    expect(
      within(menu)
        .getAllByRole("menuitem")
        .map((item) => item.textContent),
    ).toEqual([
      "tactics.newTactic",
      "tactics.duplicateTactic",
      "tactics.renameTactic",
      "tactics.resetTactic",
      "tactics.deleteTactic",
    ]);
  });

  describe("renaming a saved tactic", () => {
    const openRename = () => {
      openActionsMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "tactics.renameTactic" }));
    };

    /**
     * Given a preset, or a setup that was never saved, is active
     * When the actions menu is opened
     * Then Rename is offered but off: only a saved tactic has a name of its own.
     */
    it("keeps Rename off when nothing saved is active", () => {
      renderCommandBar({ activeTactic: presetTactic, canRename: false });

      openActionsMenu();

      expect(screen.getByRole("menuitem", { name: "tactics.renameTactic" })).toBeDisabled();
    });

    /**
     * Given a saved tactic is active
     * When Rename is chosen
     * Then a dialog named for renaming opens with the tactic's name in the field, selected, and focus
     *      in the field.
     */
    it("opens a dialog with the name in a focused, selected field", async () => {
      renderCommandBar();

      openRename();

      const dialog = screen.getByRole("dialog", { name: "tactics.renameTacticTitle" });
      const field = within(dialog).getByRole("textbox", { name: "tactics.renameTacticLabel" });
      expect(field).toHaveValue("My Tactic");
      await waitFor(() => expect(field).toHaveFocus());
      expect((field as HTMLInputElement).selectionStart).toBe(0);
      expect((field as HTMLInputElement).selectionEnd).toBe("My Tactic".length);
    });

    /**
     * Given the rename dialog open
     * When a new name (with stray spaces) is typed and Enter is pressed in the field
     * Then the screen is asked to rename to the trimmed name, and the dialog closes once the save has
     *      accepted it.
     */
    it("saves the trimmed name on Enter and closes", async () => {
      const props = buildProps();
      render(<TacticsCommandBar {...props} />);
      openRename();
      const field = screen.getByRole("textbox", { name: "tactics.renameTacticLabel" });

      fireEvent.change(field, { target: { value: "  Away  " } });
      fireEvent.submit(field.closest("form") as HTMLFormElement);

      await waitFor(() => expect(props.onRename).toHaveBeenCalledWith("custom:1", "Away"));
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    });

    /**
     * Given the rename dialog open for one saved tactic
     * When another saved tactic becomes the active one while it is open (applying it finished), and the
     *      name is then submitted
     * Then the tactic the dialog was opened for is renamed, not the one that is active now.
     */
    it("renames the tactic the dialog was opened for even if another becomes active", async () => {
      const other = { ...customTactic, id: "custom:2", name: "Other" };
      const props = buildProps({ tacticLibrary: [customTactic, other, presetTactic] });
      const { rerender } = render(<TacticsCommandBar {...props} />);
      openRename();
      rerender(<TacticsCommandBar {...props} activeTactic={other} />);
      const field = screen.getByRole("textbox", { name: "tactics.renameTacticLabel" });
      expect(field).toHaveValue("My Tactic");

      fireEvent.change(field, { target: { value: "Away" } });
      fireEvent.submit(field.closest("form") as HTMLFormElement);

      await waitFor(() => expect(props.onRename).toHaveBeenCalledWith("custom:1", "Away"));
    });

    /** The rename dialog open, "Rename" pressed, and the save's answer held until `accept()`. */
    async function openWithTheSaveOut() {
      let accept!: () => void;
      const onRename = vi.fn().mockReturnValue(
        new Promise<null>((resolve) => {
          accept = () => resolve(null);
        }),
      );
      renderCommandBar({ onRename });
      openRename();
      fireEvent.click(screen.getByRole("button", { name: "tactics.renameTacticConfirm" }));
      await waitFor(() => expect(onRename).toHaveBeenCalled());
      return accept;
    }

    /**
     * Given the rename dialog open and the save's answer still out
     * When Escape is pressed
     * Then the dialog stays open (a dialog whose answer is to come cannot be closed under it), and it
     *      closes when the save accepts the name.
     */
    it("stays open on Escape while the save is out, and closes once it accepts the name", async () => {
      const accept = await openWithTheSaveOut();

      fireEvent.keyDown(window, { key: "Escape" });

      expect(screen.getByRole("dialog", { name: "tactics.renameTacticTitle" })).toBeInTheDocument();
      await act(async () => accept());
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    });

    /**
     * Given the rename dialog open and the save's answer still out
     * When the backdrop is pressed and released (a mouse down on it, then the click: the dialog only
     *      closes on a click that began on the backdrop)
     * Then the dialog stays open until the save has answered.
     */
    it("stays open on a backdrop click while the save is out", async () => {
      const accept = await openWithTheSaveOut();
      const backdrop = screen.getByTestId("dialog-backdrop");

      fireEvent.mouseDown(backdrop);
      fireEvent.click(backdrop);

      expect(screen.getByRole("dialog", { name: "tactics.renameTacticTitle" })).toBeInTheDocument();
      await act(async () => accept());
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    });

    /**
     * Given the rename dialog open and no save out
     * When the backdrop is pressed and released
     * Then the dialog closes: the backdrop does dismiss it when nothing is in flight (so the test above
     *      is about the save, not about a backdrop that never closes anything).
     */
    it("closes on a backdrop click when no save is out", async () => {
      renderCommandBar();
      openRename();
      const backdrop = screen.getByTestId("dialog-backdrop");

      fireEvent.mouseDown(backdrop);
      fireEvent.click(backdrop);

      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    });

    /**
     * Given a name of 31 characters that are each two UTF-16 units (62 units, 31 characters: within the
     *       save's limit of 60 characters)
     * When it is typed in the rename dialog and submitted
     * Then the field has no length limit of its own (the browser would count units and cut it) and the
     *      whole name goes to the save, whose refusal is the one rule on length.
     */
    it("leaves the name's length to the save, which counts characters", async () => {
      const longName = "\u{1D538}".repeat(31);
      const props = buildProps();
      render(<TacticsCommandBar {...props} />);
      openRename();
      const field = screen.getByRole("textbox", { name: "tactics.renameTacticLabel" });

      fireEvent.change(field, { target: { value: longName } });
      fireEvent.submit(field.closest("form") as HTMLFormElement);

      expect(field).not.toHaveAttribute("maxlength");
      await waitFor(() => expect(props.onRename).toHaveBeenCalledWith("custom:1", longName));
    });

    /**
     * Given the rename dialog open and a save that refuses the name
     * When Rename is pressed
     * Then the reason is shown in the dialog as an alert tied to the field, and the dialog stays open
     *      with what was typed.
     */
    it("shows the save's reason and stays open when the name is refused", async () => {
      const onRename = vi.fn().mockResolvedValue("That name is too long.");
      renderCommandBar({ onRename });
      openRename();
      const field = screen.getByRole("textbox", { name: "tactics.renameTacticLabel" });
      fireEvent.change(field, { target: { value: "Away" } });

      fireEvent.click(screen.getByRole("button", { name: "tactics.renameTacticConfirm" }));

      const alert = await screen.findByRole("alert");
      expect(alert).toHaveTextContent("That name is too long.");
      expect(field).toHaveAttribute("aria-invalid", "true");
      expect(field).toHaveAccessibleDescription("That name is too long.");
      expect(screen.getByRole("dialog", { name: "tactics.renameTacticTitle" })).toBeInTheDocument();
      expect(field).toHaveValue("Away");
    });

    /**
     * Given the rename dialog open
     * When the name is emptied
     * Then Rename is off and submitting asks nothing of the save.
     */
    it("does not ask the save to rename to an empty name", () => {
      const props = buildProps();
      render(<TacticsCommandBar {...props} />);
      openRename();
      const field = screen.getByRole("textbox", { name: "tactics.renameTacticLabel" });

      fireEvent.change(field, { target: { value: "   " } });
      fireEvent.submit(field.closest("form") as HTMLFormElement);

      expect(screen.getByRole("button", { name: "tactics.renameTacticConfirm" })).toBeDisabled();
      expect(props.onRename).not.toHaveBeenCalled();
    });

    /**
     * Given the rename dialog open
     * When Escape is pressed, and, in another round, Cancel
     * Then the dialog closes and nothing is renamed.
     */
    it.each([
      ["Escape", () => fireEvent.keyDown(window, { key: "Escape" })],
      ["Cancel", () => fireEvent.click(screen.getByRole("button", { name: "common.cancel" }))],
    ])("backs out with %s without renaming", async (_name, back) => {
      const props = buildProps();
      render(<TacticsCommandBar {...props} />);
      openRename();
      await waitFor(() =>
        expect(screen.getByRole("textbox", { name: "tactics.renameTacticLabel" })).toHaveFocus(),
      );

      back();

      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
      expect(props.onRename).not.toHaveBeenCalled();
    });
  });

  /**
   * Given the tactic picker's list open
   * When the actions menu is opened (from the keyboard, where no outside press shuts the list)
   * Then the list shuts: the two are never open together.
   */
  it("shuts the tactic picker's list when the actions menu opens", () => {
    renderCommandBar();
    fireEvent.click(screen.getByRole("button", { name: "tactics.chooseTactic" }));
    expect(screen.getByRole("listbox", { name: "tactics.chooseTactic" })).toBeInTheDocument();

    openActionsMenu();

    expect(screen.queryByRole("listbox", { name: "tactics.chooseTactic" })).toBeNull();
    expect(screen.getByRole("menu")).toBeInTheDocument();
  });

  /**
   * Given the actions menu open
   * When New is chosen, and in a fresh bar Duplicate
   * Then each asks the screen for it, once.
   */
  it.each([
    ["tactics.newTactic", "onCreateNew"],
    ["tactics.duplicateTactic", "onDuplicate"],
  ] as const)("runs %s from the actions menu", (item, handler) => {
    const props = buildProps();
    render(<TacticsCommandBar {...props} />);

    openActionsMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: item }));

    expect(props[handler]).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a preset (whose Save is always on) while the library's first load is still pending
   * When the bar renders, and again once the load has settled
   * Then Save is disabled until it has: a save asked first would queue behind the load.
   */
  it("keeps Save disabled until the library's first load has settled", () => {
    const { rerender } = renderCommandBar({ activeTactic: presetTactic, libraryLoaded: false });
    expect(screen.getByRole("button", { name: "tactics.saveAsTactic" })).toBeDisabled();

    rerender(
      <TacticsCommandBar {...buildProps({ activeTactic: presetTactic, libraryLoaded: true })} />,
    );

    expect(screen.getByRole("button", { name: "tactics.saveAsTactic" })).toBeEnabled();
  });

  /**
   * Given Save off while the library's first load is pending
   * When the bar is read
   * Then the status where the sync badge sits says the tactics are loading (visible, in a polite
   *      status region that was there before it changed), Save is described by it, and the sync
   *      badge is not shown; once loaded the status says Synced and Save has no such description.
   */
  it("says why Save is off, in the status where the sync badge sits, while the library loads", () => {
    const { rerender } = renderCommandBar({ activeTactic: presetTactic, libraryLoaded: false });
    const status = screen.getByText("tactics.loadingTactics").closest("[aria-live]") as HTMLElement;
    expect(status).toHaveAttribute("aria-live", "polite");
    expect(status).toHaveTextContent("tactics.loadingTactics");
    expect(status).not.toHaveTextContent("tactics.synced");
    expect(
      screen.getByRole("button", { name: "tactics.saveAsTactic" }),
    ).toHaveAccessibleDescription("tactics.loadingTactics");

    rerender(
      <TacticsCommandBar {...buildProps({ activeTactic: presetTactic, libraryLoaded: true })} />,
    );

    expect(status).toHaveTextContent("tactics.synced");
    expect(
      screen.getByRole("button", { name: "tactics.saveAsTactic" }),
    ).not.toHaveAccessibleDescription();
  });

  describe("deleting a custom tactic", () => {
    /**
     * Given a saved custom tactic is active
     * When Delete is pressed
     * Then a confirmation names the tactic, nothing is deleted yet, and Cancel leaves it alone.
     */
    it("asks first, and Cancel deletes nothing", () => {
      const onDelete = vi.fn();
      renderCommandBar({ onDelete });

      openActionsMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "tactics.deleteTactic" }));

      const dialog = screen.getByRole("dialog");
      expect(dialog).toHaveTextContent("tactics.deleteTacticBody");
      expect(onDelete).not.toHaveBeenCalled();

      fireEvent.click(screen.getByRole("button", { name: "common.cancel" }));

      expect(screen.queryByRole("dialog")).toBeNull();
      expect(onDelete).not.toHaveBeenCalled();
    });

    /**
     * Given the confirmation has just opened
     * Then focus is on Cancel, the safe answer, so a stray Enter cannot delete.
     */
    it("starts with focus on Cancel", () => {
      renderCommandBar();

      openActionsMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "tactics.deleteTactic" }));

      expect(screen.getByRole("button", { name: "common.cancel" })).toHaveFocus();
    });

    /**
     * Given the confirmation is open
     * When the backdrop is clicked
     * Then it cancels without deleting: Cancel is the safe answer, and what players expect.
     */
    it("a click on the backdrop cancels without deleting", () => {
      const onDelete = vi.fn();
      renderCommandBar({ onDelete });
      openActionsMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "tactics.deleteTactic" }));

      const backdrop = screen.getByTestId("dialog-backdrop");
      fireEvent.mouseDown(backdrop);
      fireEvent.click(backdrop);

      expect(screen.queryByRole("dialog")).toBeNull();
      expect(onDelete).not.toHaveBeenCalled();
    });

    /**
     * Given the confirmation is open
     * When Delete is confirmed
     * Then the tactic is deleted once and the dialog closes.
     */
    it("deletes once when confirmed", () => {
      const onDelete = vi.fn();
      renderCommandBar({ onDelete });

      openActionsMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "tactics.deleteTactic" }));
      fireEvent.click(screen.getByRole("button", { name: "tactics.deleteTacticConfirm" }));

      expect(onDelete).toHaveBeenCalledTimes(1);
      expect(screen.queryByRole("dialog")).toBeNull();
    });

    /**
     * Given the confirmation is open
     * When Escape is pressed
     * Then it closes without deleting: a keyboard player can back out.
     */
    it("Escape backs out without deleting", () => {
      const onDelete = vi.fn();
      renderCommandBar({ onDelete });
      openActionsMenu();
      fireEvent.click(screen.getByRole("menuitem", { name: "tactics.deleteTactic" }));

      fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

      expect(screen.queryByRole("dialog")).toBeNull();
      expect(onDelete).not.toHaveBeenCalled();
    });

    /**
     * Given a preset, or a setup that was never saved, is active
     * When the bar is shown
     * Then there is no Delete button.
     */
    it("offers no Delete when nothing saved is active", () => {
      renderCommandBar({ activeTactic: presetTactic, canDelete: false });

      openActionsMenu();
      expect(screen.queryByRole("menuitem", { name: "tactics.deleteTactic" })).toBeNull();
    });
  });
});
