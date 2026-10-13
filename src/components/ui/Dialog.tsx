import { type ReactNode, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

import { useDialogKeyboard } from "../../hooks/useDialogKeyboard";

interface NamedBy {
  /** Id of the heading inside the dialog that names it. */
  labelledBy: string;
  label?: never;
}

interface Labelled {
  label: string;
  labelledBy?: never;
}

type DialogProps = (NamedBy | Labelled) & {
  /** Escape, and a backdrop click when `dismissOnBackdrop`, ask for this. A refusal is the caller's. */
  onClose: () => void;
  dismissOnBackdrop?: boolean;
  /** Set while the dialog refuses to close, so the refusal is not silent to a screen reader. */
  busy?: boolean;
  describedBy?: string;
  /** Backdrop colour and padding. */
  overlayClassName?: string;
  /** Size, rounding and padding of the panel; the surface colours and border come from the shell. */
  className?: string;
  children: ReactNode;
};

/**
 * The one modal shell: role, name, focus on open, focus back to the opener, Escape and a Tab trap.
 * Dialogs differ in what they show, never in how they behave for a keyboard or a screen reader.
 */
export function Dialog({
  labelledBy,
  label,
  describedBy,
  onClose,
  dismissOnBackdrop = false,
  busy,
  overlayClassName = "bg-black/50 p-4",
  className = "",
  children,
}: DialogProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  useDialogKeyboard(panelRef, onClose);

  // A child that asked for focus (an amount field, a step heading) keeps it.
  useEffect(() => {
    const panel = panelRef.current;
    if (panel && !panel.contains(document.activeElement)) panel.focus();
  }, []);

  return (
    <div
      data-testid="dialog-backdrop"
      className={`fixed inset-0 z-50 flex items-center justify-center ${overlayClassName}`}
      onClick={dismissOnBackdrop ? onClose : undefined}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        aria-label={label}
        aria-describedby={describedBy}
        aria-busy={busy}
        tabIndex={-1}
        className={`bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 shadow-2xl focus:outline-none ${className}`}
        onClick={(event) => event.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}

export function DialogCloseButton({
  onClose,
  disabled,
}: {
  onClose: () => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      onClick={onClose}
      disabled={disabled}
      aria-label={t("common.close")}
      className="rounded-lg p-1.5 text-gray-500 transition-colors hover:bg-gray-100 hover:text-gray-900 focus:outline-none focus:ring-2 focus:ring-primary-500 disabled:opacity-50 dark:text-gray-400 dark:hover:bg-navy-600 dark:hover:text-white"
    >
      <span aria-hidden="true" className="font-heading text-sm">
        ✕
      </span>
    </button>
  );
}
