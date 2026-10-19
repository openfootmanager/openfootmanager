import { type ReactNode, type RefObject, useEffect, useRef } from "react";
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
  /** For a caller that has to look inside the panel, to move focus to a control it renders. */
  panelRef?: RefObject<HTMLDivElement | null>;
  /** Backdrop colour and padding. */
  overlayClassName?: string;
  /** Size, rounding and padding of the panel; the border and shadow come from the shell. */
  className?: string;
  /** Panel colours in both themes, for a dialog whose contents sit on a different surface. */
  surfaceClassName?: string;
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
  panelRef: callerPanelRef,
  onClose,
  dismissOnBackdrop = false,
  busy,
  overlayClassName = "bg-overlay p-4",
  className = "",
  surfaceClassName = "bg-panel",
  children,
}: DialogProps) {
  // A press that starts inside the panel (selecting text in a field) and ends on the backdrop is
  // not a click on the backdrop, though the browser reports one.
  const pressStartedOnBackdrop = useRef(false);
  const ownPanelRef = useRef<HTMLDivElement>(null);
  const panelRef = callerPanelRef ?? ownPanelRef;
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
      onMouseDown={(event) => {
        pressStartedOnBackdrop.current = event.target === event.currentTarget;
      }}
      onClick={(event) => {
        if (
          dismissOnBackdrop &&
          event.target === event.currentTarget &&
          pressStartedOnBackdrop.current
        )
          onClose();
      }}
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
        className={`${surfaceClassName} border border-line shadow-2xl focus:outline-none ${className}`}
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
      className="rounded-lg p-1.5 text-fg-muted transition-colors hover:bg-hover hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page disabled:opacity-50"
    >
      <span aria-hidden="true" className="font-heading text-body">
        ✕
      </span>
    </button>
  );
}
