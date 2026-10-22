import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { Dialog, DialogCloseButton } from "./Dialog";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

function Harness({
  onClose = vi.fn(),
  dismissOnBackdrop,
  busy,
}: {
  onClose?: () => void;
  dismissOnBackdrop?: boolean;
  busy?: boolean;
}) {
  const [open, setOpen] = useState(true);
  return (
    <>
      <button type="button">Opener</button>
      {open && (
        <Dialog
          labelledBy="t"
          onClose={() => {
            onClose();
            setOpen(false);
          }}
          dismissOnBackdrop={dismissOnBackdrop}
          busy={busy}
          className="p-4"
        >
          <h2 id="t">Title</h2>
          <button type="button">First</button>
          <button type="button">Last</button>
        </Dialog>
      )}
    </>
  );
}

describe("Dialog", () => {
  /**
   * Given a dialog is open
   * When assistive technology looks for it
   * Then it is a modal dialog named by its heading.
   */
  it("is a modal dialog named by its heading", () => {
    render(<Harness />);

    const dialog = screen.getByRole("dialog", { name: "Title" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
  });

  /**
   * Given a dialog named by a plain label rather than a heading
   * When it opens
   * Then the label is its accessible name.
   */
  it("can be named by a label", () => {
    render(
      <Dialog label="Match details" onClose={() => {}}>
        <p>Body</p>
      </Dialog>,
    );

    expect(screen.getByRole("dialog", { name: "Match details" })).toBeInTheDocument();
  });

  /**
   * Given a dialog opens with no control asking for focus
   * When it mounts
   * Then focus moves into the dialog, off the button behind it.
   */
  it("takes focus on open", () => {
    render(<Harness />);

    expect(screen.getByRole("dialog")).toHaveFocus();
  });

  /**
   * Given a control inside the dialog focuses itself on mount
   * When the dialog opens
   * Then the dialog leaves that choice alone.
   */
  it("leaves focus where a child already put it", () => {
    render(
      <Dialog label="x" onClose={() => {}}>
        <input aria-label="Amount" autoFocus />
      </Dialog>,
    );

    expect(screen.getByLabelText("Amount")).toHaveFocus();
  });

  /**
   * Given the dialog was opened from a button that had focus
   * When it closes
   * Then focus returns to that button, even when a child had taken focus first.
   */
  it("returns focus to the opener, even past a child that took focus on mount", () => {
    function Opens() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <button type="button" onClick={() => setOpen(true)}>
            Opener
          </button>
          {open && (
            <Dialog label="x" onClose={() => setOpen(false)}>
              <input aria-label="Amount" autoFocus />
            </Dialog>
          )}
        </>
      );
    }
    render(<Opens />);
    const opener = screen.getByRole("button", { name: "Opener" });
    opener.focus();
    fireEvent.click(opener);
    expect(screen.getByLabelText("Amount")).toHaveFocus();

    fireEvent.keyDown(window, { key: "Escape" });

    expect(opener).toHaveFocus();
  });

  /**
   * Given an open dialog
   * When Escape is pressed
   * Then it asks to close.
   */
  it("asks to close on Escape", () => {
    const onClose = vi.fn();
    render(<Harness onClose={onClose} />);

    fireEvent.keyDown(window, { key: "Escape" });

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a control inside the dialog already used Escape for itself
   * When that keydown reaches the window
   * Then the dialog stays.
   */
  it("ignores an Escape another control already handled", () => {
    const onClose = vi.fn();
    render(<Harness onClose={onClose} />);
    const first = screen.getByRole("button", { name: "First" });
    first.addEventListener("keydown", (e) => e.preventDefault());

    fireEvent.keyDown(first, { key: "Escape" });

    expect(onClose).not.toHaveBeenCalled();
  });

  /**
   * Given focus is on the last control
   * When Tab is pressed
   * Then focus wraps to the first control instead of leaving the dialog.
   */
  it("keeps Tab inside the dialog", () => {
    render(<Harness />);
    screen.getByRole("button", { name: "Last" }).focus();

    fireEvent.keyDown(screen.getByRole("button", { name: "Last" }), { key: "Tab" });

    expect(screen.getByRole("button", { name: "First" })).toHaveFocus();
  });

  /**
   * Given a dialog that dismisses on backdrop click
   * When the backdrop is clicked, and when the panel is clicked
   * Then only the backdrop click closes it.
   */
  it("closes on a backdrop click but not a click inside", () => {
    const onClose = vi.fn();
    render(<Harness onClose={onClose} dismissOnBackdrop />);

    fireEvent.click(screen.getByRole("heading", { name: "Title" }));
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.mouseDown(screen.getByTestId("dialog-backdrop"));
    fireEvent.click(screen.getByTestId("dialog-backdrop"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a press that starts inside the panel, as when selecting text in a field
   * When the pointer is released over the backdrop and the browser reports a click there
   * Then the dialog stays open and what was typed is not lost.
   */
  it("stays open when a press starts in the panel and ends on the backdrop", () => {
    const onClose = vi.fn();
    render(<Harness onClose={onClose} dismissOnBackdrop />);

    fireEvent.mouseDown(screen.getByRole("heading", { name: "Title" }));
    fireEvent.click(screen.getByTestId("dialog-backdrop"));

    expect(onClose).not.toHaveBeenCalled();
  });

  /**
   * Given the dialog has just opened with focus on the panel
   * When Shift+Tab is pressed
   * Then focus wraps to the last control instead of leaving for the page behind.
   */
  it("wraps Shift+Tab from the panel to the last control", () => {
    render(<Harness />);
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveFocus();

    const allowed = fireEvent.keyDown(dialog, { key: "Tab", shiftKey: true });

    expect(allowed).toBe(false);
    expect(screen.getByRole("button", { name: "Last" })).toHaveFocus();
  });

  /**
   * Given a dialog that does not dismiss on backdrop click
   * When the backdrop is clicked
   * Then it stays open.
   */
  it("ignores a backdrop click unless asked to dismiss on it", () => {
    const onClose = vi.fn();
    render(<Harness onClose={onClose} />);

    fireEvent.mouseDown(screen.getByTestId("dialog-backdrop"));
    fireEvent.click(screen.getByTestId("dialog-backdrop"));

    expect(onClose).not.toHaveBeenCalled();
  });

  /**
   * Given work is in flight and the dialog refuses to close
   * When it is busy
   * Then aria-busy says so.
   */
  it("announces that it is busy", () => {
    render(<Harness busy />);

    expect(screen.getByRole("dialog")).toHaveAttribute("aria-busy", "true");
  });

  /**
   * Given a dialog that can't be closed yet (a save in flight), drawn with no onClose
   * When the player presses Escape, or clicks the backdrop
   * Then nothing happens and it stays: there is nothing to ask, and no error.
   */
  it("stays open with no onClose, whatever is pressed", () => {
    const onError = vi.fn();
    window.addEventListener("error", onError);
    render(
      <Dialog labelledBy="saving" busy dismissOnBackdrop>
        <h2 id="saving">Saving</h2>
      </Dialog>,
    );

    fireEvent.keyDown(window, { key: "Escape" });
    const backdrop = screen.getByTestId("dialog-backdrop");
    fireEvent.mouseDown(backdrop);
    fireEvent.click(backdrop);

    window.removeEventListener("error", onError);
    expect(onError).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog", { name: "Saving" })).toBeInTheDocument();
  });
});

describe("DialogCloseButton", () => {
  /**
   * Given the shared close button
   * When it is activated
   * Then it asks to close, under an accessible name.
   */
  it("is named and asks to close", () => {
    const onClose = vi.fn();
    render(<DialogCloseButton onClose={onClose} />);

    fireEvent.click(screen.getByRole("button", { name: "common.close" }));

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  /**
   * Given closing is refused
   * When the button is disabled
   * Then it cannot be activated.
   */
  it("can be disabled", () => {
    render(<DialogCloseButton onClose={() => {}} disabled />);

    expect(screen.getByRole("button", { name: "common.close" })).toBeDisabled();
  });
});
