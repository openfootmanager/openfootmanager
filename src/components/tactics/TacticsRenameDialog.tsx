import { useEffect, useRef, useState, type FormEvent, type JSX } from "react";
import { useTranslation } from "react-i18next";

import { Button, Dialog } from "../ui";

/** The longest a tactic's name can be: the save refuses a longer one (`MAX_NAME_CHARS`). */
const NAME_MAX_LENGTH = 60;

interface TacticsRenameDialogProps {
  /** The tactic's name now, to start from. */
  name: string;
  /** Asks the save to rename: resolves null once it has, or why it did not (already translated). */
  onRename: (name: string) => Promise<string | null>;
  /** Closes the dialog: after the rename, or when the manager backs out. */
  onClose: () => void;
}

/**
 * Renames the active saved tactic. Focus starts in the name field with the name selected, Enter
 * saves, Escape and Cancel back out. The save's refusal is shown here, in the dialog, and the dialog
 * stays open for another try.
 */
export default function TacticsRenameDialog({
  name,
  onRename,
  onClose,
}: TacticsRenameDialogProps): JSX.Element {
  const { t } = useTranslation();
  const [value, setValue] = useState(name);
  const [reason, setReason] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const fieldRef = useRef<HTMLInputElement>(null);
  const trimmed = value.trim();

  useEffect(() => {
    fieldRef.current?.focus();
    fieldRef.current?.select();
  }, []);

  async function submit(event: FormEvent): Promise<void> {
    event.preventDefault();
    if (saving || trimmed.length === 0) return;
    setSaving(true);
    setReason(null);
    const refused = await onRename(trimmed);
    setSaving(false);
    if (refused === null) onClose();
    else setReason(refused);
  }

  return (
    <Dialog
      labelledBy="rename-tactic-title"
      onClose={onClose}
      busy={saving}
      dismissOnBackdrop
      className="w-full max-w-md rounded-xl p-6"
    >
      <form onSubmit={(event) => void submit(event)}>
        <h2 id="rename-tactic-title" className="font-heading text-heading font-bold text-fg">
          {t("tactics.renameTacticTitle")}
        </h2>
        <label className="mt-4 block">
          <span className="mb-1 block font-heading text-caption font-bold text-fg-muted">
            {t("tactics.renameTacticLabel")}
          </span>
          <input
            ref={fieldRef}
            type="text"
            value={value}
            maxLength={NAME_MAX_LENGTH}
            onChange={(event) => setValue(event.target.value)}
            aria-invalid={reason !== null}
            aria-describedby={reason !== null ? "rename-tactic-reason" : undefined}
            className="h-9 w-full rounded-lg border border-line-strong bg-panel px-3 text-body text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
          />
        </label>
        {reason !== null ? (
          <p
            id="rename-tactic-reason"
            role="alert"
            className="mt-2 text-body font-medium text-danger"
          >
            {reason}
          </p>
        ) : null}
        <div className="mt-6 flex justify-end gap-3">
          <Button type="button" variant="ghost" onClick={onClose} disabled={saving}>
            {t("common.cancel")}
          </Button>
          <Button type="submit" variant="accent" disabled={saving || trimmed.length === 0}>
            {t("tactics.renameTacticConfirm")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
