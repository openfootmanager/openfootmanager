import { useRef, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import JerseyIcon from "../ui/JerseyIcon";
import type { KitPattern } from "../../store/types";

interface JerseyNumberInputProps {
  value: number | null;
  primaryColor: string;
  secondaryColor: string;
  pattern: KitPattern;
  onCommit: (value: number | null) => Promise<void>;
  disabled?: boolean;
}

export default function JerseyNumberInput({
  value,
  primaryColor,
  secondaryColor,
  pattern,
  onCommit,
  disabled,
}: JerseyNumberInputProps) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const committingRef = useRef(false);

  function startEdit() {
    if (disabled) return;
    setDraft(value != null ? String(value) : "");
    setEditing(true);
    // Focus happens via autoFocus on the input
  }

  async function commit() {
    if (committingRef.current) return;
    committingRef.current = true;
    const trimmed = draft.trim();
    let next: number | null;
    if (trimmed === "") {
      next = null;
    } else {
      const parsed = parseInt(trimmed, 10);
      if (isNaN(parsed)) {
        // Revert non-numeric entry
        committingRef.current = false;
        setEditing(false);
        return;
      }
      next = parsed;
    }

    // No change
    if (next === value) {
      committingRef.current = false;
      setEditing(false);
      return;
    }

    setSaving(true);
    try {
      await onCommit(next);
    } finally {
      committingRef.current = false;
      setSaving(false);
      setEditing(false);
    }
  }

  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      commit();
    } else if (e.key === "Escape") {
      setEditing(false);
    }
  }

  if (editing) {
    return (
      <input
        type="number"
        aria-label={t("squad.jerseyNumber")}
        min={1}
        max={99}
        autoFocus
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={handleKeyDown}
        className="w-12 px-1.5 py-0.5 rounded text-sm font-heading font-bold tabular-nums text-center
                   bg-gray-50 dark:bg-navy-700
                   border border-gray-300 dark:border-navy-500
                   text-gray-800 dark:text-gray-100
                   focus:outline-none focus:ring-2 focus:ring-primary-500/30
                   [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
      />
    );
  }

  return (
    <button
      type="button"
      onClick={startEdit}
      disabled={disabled || saving}
      title={value != null ? t("squad.jerseyNumberClickToChange", { number: value }) : t("squad.jerseyNumberClickToAssign")}
      className="flex items-center justify-center cursor-pointer disabled:cursor-default
                 rounded hover:opacity-80 transition-opacity focus:outline-none focus:ring-2 focus:ring-primary-500/30"
    >
      {value != null ? (
        <JerseyIcon
          primaryColor={primaryColor}
          secondaryColor={secondaryColor}
          pattern={pattern}
          number={value}
          size="sm"
        />
      ) : (
        <span className="text-gray-400 dark:text-gray-600 text-sm font-heading font-bold w-8 text-center">
          —
        </span>
      )}
    </button>
  );
}
