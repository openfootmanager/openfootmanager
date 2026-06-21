import { Check } from "lucide-react";
import type { ChangeEvent } from "react";

interface CheckboxProps {
  checked: boolean;
  onChange: (event: ChangeEvent<HTMLInputElement>) => void;
  disabled?: boolean;
  "aria-label"?: string;
  "data-testid"?: string;
  className?: string;
  id?: string;
}

/**
 * Custom styled checkbox that replaces the native browser element.
 *
 * Renders a visually-hidden native input (for keyboard and screen-reader
 * support) alongside a Tailwind-styled indicator that shows the game's
 * primary colour when checked. Avoids OS-native appearance on Linux/WebKit.
 */
export function Checkbox({
  checked,
  onChange,
  disabled,
  "aria-label": ariaLabel,
  "data-testid": dataTestId,
  className = "",
  id,
}: CheckboxProps) {
  return (
    <label
      className={`relative inline-flex items-center ${disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer"} ${className}`}
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={onChange}
        disabled={disabled}
        aria-label={ariaLabel}
        data-testid={dataTestId}
        id={id}
        className="peer sr-only"
      />
      <div
        className="h-4 w-4 rounded border-2 flex items-center justify-center transition-colors border-gray-300 dark:border-navy-500 bg-white dark:bg-navy-700 peer-checked:bg-primary-500 peer-checked:border-primary-500 peer-focus:ring-2 peer-focus:ring-primary-500/30 peer-focus:outline-none"
        aria-hidden="true"
      >
        {checked ? <Check className="h-3 w-3 text-white" strokeWidth={3} /> : null}
      </div>
    </label>
  );
}
