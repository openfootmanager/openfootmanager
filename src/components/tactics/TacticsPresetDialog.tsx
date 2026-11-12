import { useEffect, useRef, type JSX } from "react";
import { useTranslation } from "react-i18next";

import { Button, Dialog } from "../ui";

interface TacticsPresetDialogProps {
  /** The preset's name, shown so the manager knows whose instructions are on offer. */
  name: string;
  onReplace: () => void;
  onKeep: () => void;
  onCancel: () => void;
}

/**
 * Asks before a preset replaces the manager's own team instructions. Focus starts on Keep mine:
 * replacing is the answer that loses something, so it is the one a key press must reach on purpose.
 * Escape and Cancel apply nothing.
 */
export default function TacticsPresetDialog({
  name,
  onReplace,
  onKeep,
  onCancel,
}: TacticsPresetDialogProps): JSX.Element {
  const { t } = useTranslation();
  const buttonsRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // The buttons run Cancel, Keep mine, Replace: focus the middle one.
    buttonsRef.current?.querySelectorAll("button")[1]?.focus();
  }, []);

  return (
    <Dialog
      labelledBy="preset-replace-title"
      describedBy="preset-replace-body"
      onClose={onCancel}
      dismissOnBackdrop
      className="w-full max-w-md rounded-xl p-6"
    >
      <h2 id="preset-replace-title" className="font-heading text-xl font-bold text-fg">
        {t("tactics.presetReplaceTitle")}
      </h2>
      <p id="preset-replace-body" className="mt-2 text-body text-fg-muted">
        {t("tactics.presetReplaceBody", { name })}
      </p>
      <div ref={buttonsRef} className="mt-6 flex flex-wrap justify-end gap-3">
        <Button variant="ghost" onClick={onCancel}>
          {t("common.cancel")}
        </Button>
        <Button variant="outline" onClick={onKeep}>
          {t("tactics.presetKeepMine")}
        </Button>
        <Button variant="accent" onClick={onReplace}>
          {t("tactics.presetReplaceConfirm")}
        </Button>
      </div>
    </Dialog>
  );
}
