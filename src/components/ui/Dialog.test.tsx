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
   * Given two dialogs open at once, the later one on top (the window's close confirmation over an
   * advance)
   * When the player presses Escape
   * Then only the top one is asked to close; the one under it stays as it is. And a busy dialog on
   * top, which can't close, doesn't let Escape through to close the one beneath.
   */
  it.each([
    ["a closable dialog", true],
    ["a busy dialog", false],
  ])("gives Escape to the top dialog only, when it is %s", (_top, topCloses) => {
    const under = vi.fn();
    const top = vi.fn();
    render(
      <>
        <Dialog labelledBy="under" onClose={under}>
          <h2 id="under">Under</h2>
          <button type="button">Under's button</button>
        </Dialog>
        {topCloses ? (
          <Dialog labelledBy="top" onClose={top}>
            <h2 id="top">Top</h2>
          </Dialog>
        ) : (
          <Dialog labelledBy="top" busy>
            <h2 id="top">Top</h2>
          </Dialog>
        )}
      </>,
    );

    fireEvent.keyDown(window, { key: "Escape" });

    expect(under).not.toHaveBeenCalled();
    if (topCloses) expect(top).toHaveBeenCalledOnce();
  });

  /**
   * Given a dialog with nothing to focus in it (a "Saving…" spinner), the focus on its panel
   * When the player presses Tab, or Shift+Tab
   * Then the key is kept from the browser and the focus stays on the panel, rather than moving to
   * the page behind the overlay.
   */
  /**
   * Given a dialog nested inside another's content (opened from within it)
   * When the player presses Escape
   * Then the inner one, drawn on top, is asked to close, not the one around it.
   */
  it("gives Escape to a dialog nested inside another", () => {
    const outer = vi.fn();
    const inner = vi.fn();
    render(
      <Dialog labelledBy="outer" onClose={outer}>
        <h2 id="outer">Outer</h2>
        <Dialog labelledBy="inner" onClose={inner}>
          <h2 id="inner">Inner</h2>
        </Dialog>
      </Dialog>,
    );

    fireEvent.keyDown(window, { key: "Escape" });

    expect(inner).toHaveBeenCalledOnce();
    expect(outer).not.toHaveBeenCalled();
  });

  /**
   * Given two dialogs, the lower one closing first (an advance finishing under the window's close
   * confirmation), and then the lower one opening again
   * When the player presses Escape each time
   * Then the one still open answers it, and the reopened one, on top again, answers the next.
   */
  it("hands Escape to whichever dialog is on top as dialogs close and open", () => {
    const lower = vi.fn();
    const upper = vi.fn();
    function Stack({ lowerOpen, upperOpen }: { lowerOpen: boolean; upperOpen: boolean }) {
      return (
        <>
          {lowerOpen && (
            <Dialog labelledBy="lower" onClose={lower}>
              <h2 id="lower">Lower</h2>
            </Dialog>
          )}
          {upperOpen && (
            <Dialog labelledBy="upper" onClose={upper}>
              <h2 id="upper">Upper</h2>
            </Dialog>
          )}
        </>
      );
    }
    const { rerender } = render(<Stack lowerOpen upperOpen />);

    rerender(<Stack lowerOpen={false} upperOpen />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(upper).toHaveBeenCalledTimes(1);

    rerender(<Stack lowerOpen={false} upperOpen={false} />);
    rerender(<Stack lowerOpen upperOpen={false} />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(lower).toHaveBeenCalledTimes(1);
    expect(upper).toHaveBeenCalledTimes(1);
  });

  /**
   * Given the focus on a button, a busy dialog opened from it, and a second dialog opened over that
   * When the lower dialog closes first (the advance finishes under the close confirmation)
   * Then the focus stays in the dialog still on top, not pulled back to the button behind it.
   */
  it("doesn't pull the focus out of the top dialog when the one beneath closes", () => {
    function Stack({ lowerOpen, upperOpen }: { lowerOpen: boolean; upperOpen: boolean }) {
      return (
        <>
          <button type="button">Continue</button>
          {lowerOpen && (
            <Dialog labelledBy="advance" busy>
              <h2 id="advance">Advancing</h2>
            </Dialog>
          )}
          {upperOpen && (
            <Dialog labelledBy="confirm" onClose={vi.fn()}>
              <h2 id="confirm">Quit?</h2>
              <button type="button">Cancel</button>
            </Dialog>
          )}
        </>
      );
    }
    const { rerender } = render(<Stack lowerOpen={false} upperOpen={false} />);
    screen.getByRole("button", { name: "Continue" }).focus();
    rerender(<Stack lowerOpen upperOpen={false} />);
    rerender(<Stack lowerOpen upperOpen />);
    const top = screen.getByRole("dialog", { name: "Quit?" });
    expect(top).toContainElement(document.activeElement as HTMLElement);

    rerender(<Stack lowerOpen={false} upperOpen />);

    expect(top).toContainElement(document.activeElement as HTMLElement);
    expect(screen.getByRole("button", { name: "Continue" })).not.toHaveFocus();
  });

  it.each([false, true])("keeps Tab on a dialog with nothing to focus (shift: %s)", (shiftKey) => {
    render(
      <>
        <button type="button">Behind</button>
        <Dialog labelledBy="saving" busy>
          <h2 id="saving">Saving</h2>
        </Dialog>
      </>,
    );
    const panel = screen.getByRole("dialog", { name: "Saving" });
    expect(panel).toHaveFocus();

    const notPrevented = fireEvent.keyDown(window, { key: "Tab", shiftKey });

    expect(notPrevented).toBe(false);
    expect(panel).toHaveFocus();
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
