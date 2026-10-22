import { type RefObject, useEffect, useState } from "react";

const FOCUSABLE =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * The dialogs open now, in the order they opened; the last is on top (it renders later, so it is
 * drawn over the others). Every dialog listens on the window, so without this one Escape would
 * close them all, and a busy dialog on top couldn't stop the one beneath from closing.
 */
const openDialogs: RefObject<HTMLElement | null>[] = [];

/**
 * Escape asks to close, Tab stays inside, focus returns where it was. Shared so the dialogs cannot
 * drift apart.
 *
 * A control inside the dialog that uses Escape for itself (a menu, a select list, a tooltip) must
 * call `preventDefault()` on it, or the dialog closes too.
 */
export function useDialogKeyboard(
  dialogRef: RefObject<HTMLElement | null>,
  onEscape: () => void,
): void {
  useEffect(() => {
    openDialogs.push(dialogRef);
    return () => {
      const index = openDialogs.lastIndexOf(dialogRef);
      if (index !== -1) openDialogs.splice(index, 1);
    };
  }, [dialogRef]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      // Escape and Tab belong to the dialog on top.
      if (openDialogs[openDialogs.length - 1] !== dialogRef) return;
      if (event.key === "Escape") {
        // A control inside the dialog (an open menu, a select list) that already used this Escape
        // to close itself has handled it; the dialog stays.
        if (!event.defaultPrevented) onEscape();
        return;
      }
      if (event.key !== "Tab") return;
      const dialog = dialogRef.current;
      if (dialog === null) return;

      // `aria-modal` is a promise to assistive technology, not something the browser enforces: the
      // page behind the overlay stays fully tabbable. Without this, Tab walks out of the dialog
      // into controls the player cannot see.
      const focusable = dialog.querySelectorAll<HTMLElement>(FOCUSABLE);
      // A dialog with nothing to focus (a "Saving…" spinner) keeps the focus on itself: left to
      // the browser, Tab would move it to the page behind the overlay.
      if (focusable.length === 0) {
        event.preventDefault();
        dialog.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement;
      const outside = !dialog.contains(active);
      // The panel itself and a heading take focus on open but are not Tab stops, so from either
      // one Shift+Tab has no "first" to be on and the browser would leave for the page behind.
      const onNonStop = !Array.from(focusable).includes(active as HTMLElement);

      if (!event.shiftKey && (active === last || outside)) {
        event.preventDefault();
        first.focus();
      } else if (event.shiftKey && (active === first || outside || onNonStop)) {
        event.preventDefault();
        last.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [dialogRef, onEscape]);

  // A dialog that never takes focus is one a keyboard user cannot reach: `aria-modal` alone leaves
  // focus on the button behind the overlay. Whatever had focus when the dialog opened gets it back
  // when it closes.
  // Read during the first render, not in an effect: a child that focuses itself on mount has
  // already moved focus by the time any effect runs, and would be recorded as the opener.
  const [opener] = useState(() => document.activeElement as HTMLElement | null);
  useEffect(() => () => opener?.focus?.(), [opener]);
}
