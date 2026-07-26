import { useEffect, useRef } from "react";
import type { RefObject } from "react";

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(", ");

interface UseModalOverlayOptions {
  isOpen: boolean;
  onClose: () => void;
}

interface UseModalOverlayResult<
  TDialog extends HTMLElement,
  TTrigger extends HTMLElement,
> {
  /** Attach to the dialog panel element (the one with role="dialog"). */
  dialogRef: RefObject<TDialog | null>;
  /** Attach to the button that opens the overlay; focus returns here on close. */
  triggerRef: RefObject<TTrigger | null>;
}

function getFocusableElements(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
}

/**
 * Modal semantics for hand-rolled overlays (bottom sheets, slide-over
 * drawers): while open it moves focus into the panel, closes on Escape,
 * locks body scroll, and keeps Tab/Shift+Tab cycling inside the panel.
 * Focus is restored to the trigger on close. The caller keeps rendering the
 * overlay and adds role="dialog" aria-modal="true" to the panel itself.
 */
export function useModalOverlay<
  TDialog extends HTMLElement = HTMLDivElement,
  TTrigger extends HTMLElement = HTMLButtonElement,
>({ isOpen, onClose }: UseModalOverlayOptions): UseModalOverlayResult<TDialog, TTrigger> {
  const dialogRef = useRef<TDialog | null>(null);
  const triggerRef = useRef<TTrigger | null>(null);
  const onCloseRef = useRef(onClose);

  useEffect(() => {
    onCloseRef.current = onClose;
  });

  useEffect(() => {
    if (!isOpen) return;

    const dialog = dialogRef.current;
    if (!dialog) return;

    const restoreTarget =
      triggerRef.current ??
      (document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null);

    // Move focus into the panel: first focusable element, else the panel.
    const focusableElements = getFocusableElements(dialog);
    if (focusableElements.length > 0) {
      focusableElements[0].focus();
    } else {
      if (dialog.tabIndex < 0) {
        dialog.tabIndex = -1;
      }
      dialog.focus();
    }

    // Lock body scroll while the overlay is open.
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        // An inner overlay/control already consumed this Escape (e.g. a
        // Select dropdown closing itself) — don't close the overlay too.
        if (event.defaultPrevented) return;
        event.preventDefault();
        onCloseRef.current();
        return;
      }

      if (event.key !== "Tab") return;

      const items = getFocusableElements(dialog);
      if (items.length === 0) {
        event.preventDefault();
        return;
      }

      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;

      if (event.shiftKey) {
        if (active === first || !dialog.contains(active)) {
          event.preventDefault();
          last.focus();
        }
      } else if (active === last || !dialog.contains(active)) {
        event.preventDefault();
        first.focus();
      }
    };

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = previousOverflow;
      restoreTarget?.focus();
    };
  }, [isOpen]);

  return { dialogRef, triggerRef };
}
