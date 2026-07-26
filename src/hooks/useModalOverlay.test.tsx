import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { useModalOverlay } from "./useModalOverlay";

interface HarnessProps {
  withFocusableContent?: boolean;
}

function Harness({ withFocusableContent = true }: HarnessProps) {
  const [isOpen, setIsOpen] = useState(false);
  const { dialogRef, triggerRef } = useModalOverlay({
    isOpen,
    onClose: () => setIsOpen(false),
  });

  return (
    <div>
      <button ref={triggerRef} onClick={() => setIsOpen(true)}>
        Open
      </button>
      <button onClick={() => undefined}>Outside</button>
      {isOpen && (
        <div className="fixed inset-0">
          <div ref={dialogRef} role="dialog" aria-modal="true">
            {withFocusableContent ? (
              <>
                <button>First</button>
                <button>Second</button>
              </>
            ) : (
              <p>No focusable content</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function InitiallyOpenHarness() {
  const [isOpen, setIsOpen] = useState(true);
  const { dialogRef, triggerRef } = useModalOverlay({
    isOpen,
    onClose: () => setIsOpen(false),
  });

  return (
    <div>
      <button ref={triggerRef} onClick={() => setIsOpen(true)}>
        Open
      </button>
      {isOpen && (
        <div ref={dialogRef} role="dialog" aria-modal="true">
          <button>First</button>
          <button>Second</button>
        </div>
      )}
    </div>
  );
}

describe("useModalOverlay", () => {
  it("moves focus to the first focusable element in the panel on open", () => {
    render(<InitiallyOpenHarness />);

    expect(screen.getByText("First")).toHaveFocus();
  });

  it("falls back to focusing the panel itself when nothing is focusable", () => {
    function EmptyPanelHarness() {
      const { dialogRef, triggerRef } = useModalOverlay({
        isOpen: true,
        onClose: () => undefined,
      });

      return (
        <div>
          <button ref={triggerRef}>Open</button>
          <div ref={dialogRef} role="dialog" aria-modal="true">
            <p>No focusable content</p>
          </div>
        </div>
      );
    }

    render(<EmptyPanelHarness />);

    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveFocus();
    expect(dialog.tabIndex).toBe(-1);
  });

  it("closes on Escape", () => {
    render(<Harness />);

    fireEvent.click(screen.getByText("Open"));
    expect(screen.getByRole("dialog")).toBeTruthy();

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("ignores Escape already consumed by an inner control", () => {
    // Simulates a nested overlay (e.g. an open Select dropdown) that handles
    // Escape itself: the keydown preventDefaults before bubbling to document.
    function NestedConsumerHarness() {
      const [isOpen, setIsOpen] = useState(true);
      const { dialogRef, triggerRef } = useModalOverlay({
        isOpen,
        onClose: () => setIsOpen(false),
      });

      return (
        <div>
          <button ref={triggerRef}>Open</button>
          {isOpen && (
            <div ref={dialogRef} role="dialog" aria-modal="true">
              <button
                onKeyDown={(event) => {
                  if (event.key === "Escape") event.preventDefault();
                }}
              >
                Inner
              </button>
            </div>
          )}
        </div>
      );
    }

    render(<NestedConsumerHarness />);

    const inner = screen.getByText("Inner");
    inner.focus();
    fireEvent.keyDown(inner, { key: "Escape" });

    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("restores focus to the trigger element on close", () => {
    render(<Harness />);

    fireEvent.click(screen.getByText("Open"));
    fireEvent.keyDown(document, { key: "Escape" });

    expect(screen.getByText("Open")).toHaveFocus();
  });

  it("locks body scroll while open and restores the previous value on close", () => {
    render(<Harness />);
    expect(document.body.style.overflow).toBe("");

    fireEvent.click(screen.getByText("Open"));
    expect(document.body.style.overflow).toBe("hidden");

    fireEvent.keyDown(document, { key: "Escape" });
    expect(document.body.style.overflow).toBe("");
  });

  it("cycles Tab from the last element back to the first", () => {
    render(<InitiallyOpenHarness />);

    const first = screen.getByText("First");
    const second = screen.getByText("Second");

    second.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(first).toHaveFocus();
  });

  it("cycles Shift+Tab from the first element back to the last", () => {
    render(<InitiallyOpenHarness />);

    const first = screen.getByText("First");
    const second = screen.getByText("Second");

    first.focus();
    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(second).toHaveFocus();
  });

  it("does not hijack Tab on mid-panel elements", () => {
    render(<InitiallyOpenHarness />);

    const first = screen.getByText("First");

    first.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    // Not at the boundary: no forced cycle (jsdom applies no default
    // tab navigation, so focus stays put).
    expect(first).toHaveFocus();
  });
});
