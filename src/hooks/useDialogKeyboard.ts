import { type RefObject, useEffect, useState } from "react";

const FOCUSABLE =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * Marks the panel of every open dialog this hook manages. Every dialog listens on the window, so
 * without knowing which one is on top, one Escape would close them all, and a busy dialog on top
 * couldn't stop the one beneath from closing. The mark lives on the panel, so it goes when the
 * dialog does: there is no list to keep in step. Only dialogs drawn by ui/Dialog carry it; another
 * full-screen overlay (the transfer deal workspace) doesn't take their keys.
 */
const OPEN_DIALOG = "data-open-dialog";

/**
 * The dialog drawn on top: the marked panel that comes last in the document (querySelectorAll
 * returns document order). A later overlay paints over an earlier one, and a dialog nested in
 * another's content comes after it; the order the dialogs opened in can't say this, since a
 * parent's effects run after its children's.
 */
function topDialog(): Element | null {
  const open = document.querySelectorAll(`[${OPEN_DIALOG}]`);
  return open.length === 0 ? null : open[open.length - 1];
}

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
  // No cleanup: the panel leaves the document with the dialog, and its mark with it.
  useEffect(() => {
    dialogRef.current?.setAttribute(OPEN_DIALOG, "");
  }, [dialogRef]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      // Escape and Tab belong to the dialog on top.
      if (dialogRef.current === null || topDialog() !== dialogRef.current) return;
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
  // Unless another dialog is still open. If it holds the focus (a dialog beneath closed first: an
  // advance finishing under the close confirmation), the focus stays there. If the focus is nowhere
  // (this dialog, on top, took it away with it), it goes into the dialog now on top, not behind
  // it: to the opener if the opener is inside that dialog, else to its panel.
  const [opener] = useState(() => document.activeElement as HTMLElement | null);
  useEffect(
    () => () => {
      const self = dialogRef.current;
      const holder = document.activeElement?.closest(`[${OPEN_DIALOG}]`);
      if (holder != null && holder !== self) return;
      const remaining = topDialog();
      if (remaining instanceof HTMLElement && remaining !== self) {
        if (opener && remaining.contains(opener)) opener.focus();
        // The opener may refuse it (disabled by now, its action running): the panel then has it.
        if (!remaining.contains(document.activeElement)) remaining.focus();
        return;
      }
      opener?.focus?.();
    },
    [opener, dialogRef],
  );
}
