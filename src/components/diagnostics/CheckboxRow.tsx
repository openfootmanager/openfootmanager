import type { ChangeEvent, ReactNode } from "react";

import { Checkbox } from "../ui";

interface CheckboxRowProps {
  id: string;
  checked: boolean;
  disabled: boolean;
  label: string;
  description: string;
  onChange: (event: ChangeEvent<HTMLInputElement>) => void;
  trailing?: ReactNode;
}

/**
 * A plain row, not a `<label>`: `Checkbox` renders its own label around the real input, and a
 * label nested inside another label is invalid — the browser associates the input with one of
 * them and the other stops toggling anything. The visible title is the single label that owns the
 * input, so the checkbox carries no `aria-label` of its own to override it.
 */
export function CheckboxRow({
  id,
  checked,
  disabled,
  label,
  description,
  onChange,
  trailing,
}: CheckboxRowProps) {
  const descriptionId = `${id}-desc`;
  return (
    <div className="flex items-center gap-2.5 px-3 py-2.5 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600">
      <Checkbox
        id={id}
        checked={checked}
        disabled={disabled}
        aria-describedby={descriptionId}
        onChange={onChange}
      />
      <div className="flex-1 min-w-0">
        <label
          htmlFor={id}
          className={`block text-[13px] font-medium text-gray-800 dark:text-gray-200 ${
            disabled ? "cursor-not-allowed" : "cursor-pointer"
          }`}
        >
          {label}
        </label>
        <p id={descriptionId} className="text-[11px] text-gray-600 dark:text-gray-400 mt-0.5">
          {description}
        </p>
      </div>
      {trailing}
    </div>
  );
}
