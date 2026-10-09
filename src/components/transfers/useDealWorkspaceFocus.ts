import { useEffect, useRef } from "react";

/** The workspace remains modal while its bid form is replaced by saved agreement details. */
export function useDealWorkspaceFocus(onClose: () => void, unavailableReason: string | null) {
  const dialogRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const previousFocus = document.activeElement;
    const parent = dialogRef.current?.parentElement;
    dialogRef.current?.querySelector<HTMLElement>("button")?.focus();
    return () => {
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected) previousFocus.focus();
      else parent?.querySelector<HTMLElement>("button:not([disabled])")?.focus();
    };
  }, []);
  useEffect(() => {
    const dialog = dialogRef.current;
    if (unavailableReason && dialog && !dialog.contains(document.activeElement)) {
      dialog.querySelector<HTMLElement>("button")?.focus();
    }
  }, [unavailableReason]);
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const dialog = dialogRef.current;
      if (!dialog) return;
      const controls = [
        ...dialog.querySelectorAll<HTMLElement>(
          'a[href], button:not([disabled]), input:not([disabled]), textarea:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
        ),
      ].filter((element) => !element.hidden && !element.closest('[hidden], [aria-hidden="true"]'));
      const first = controls[0];
      const last = controls[controls.length - 1];
      if (!first || !last) return;
      const active = document.activeElement;
      if (!event.shiftKey && (active === last || !dialog.contains(active))) {
        event.preventDefault();
        first.focus();
      } else if (event.shiftKey && (active === first || !dialog.contains(active))) {
        event.preventDefault();
        last.focus();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);
  return dialogRef;
}
