import { Check, Minus } from "lucide-react";
import { useEffect, useRef, type ChangeEvent } from "react";

interface CheckboxProps {
  checked: boolean;
  /** Some, not all, of a group are selected: set on the native input, which assistive technology reads as mixed, and a dash is drawn. */
  indeterminate?: boolean;
  onChange: (event: ChangeEvent<HTMLInputElement>) => void;
  disabled?: boolean;
  "aria-label"?: string;
  /** Ties visible explanatory text to the input, for when the label alone does not say enough. */
  "aria-describedby"?: string;
  "data-testid"?: string;
  className?: string;
  id?: string;
}

/**
 * Custom styled checkbox that replaces the native browser element.
 *
 * Renders a visually-hidden native input (for keyboard and screen-reader
 * support) alongside a Tailwind-styled indicator that shows the selected
 * fill when checked. Avoids OS-native appearance on Linux/WebKit.
 */
export function Checkbox({
  checked,
  indeterminate = false,
  onChange,
  disabled,
  "aria-label": ariaLabel,
  "aria-describedby": ariaDescribedBy,
  "data-testid": dataTestId,
  className = "",
  id,
}: CheckboxProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);

  // `indeterminate` is a DOM property with no HTML attribute, so it has to be set imperatively.
  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.indeterminate = indeterminate;
    }
  }, [indeterminate]);

  return (
    <label
      className={`relative inline-flex items-center ${disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer"} ${className}`}
    >
      <input
        ref={inputRef}
        type="checkbox"
        checked={checked}
        onChange={onChange}
        disabled={disabled}
        aria-label={ariaLabel}
        aria-describedby={ariaDescribedBy}
        data-testid={dataTestId}
        id={id}
        className="peer sr-only"
      />
      <div
        className={`h-4 w-4 rounded border-2 flex items-center justify-center transition-colors peer-checked:bg-selected-fill peer-checked:border-selected-fill peer-focus-visible:ring-2 peer-focus-visible:ring-focus peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-page peer-focus:outline-none ${
          indeterminate ? "bg-selected-fill border-selected-fill" : "border-line-strong bg-panel"
        }`}
        aria-hidden="true"
      >
        {indeterminate ? (
          <Minus className="h-3 w-3 text-on-brand" strokeWidth={3} />
        ) : checked ? (
          <Check className="h-3 w-3 text-on-brand" strokeWidth={3} />
        ) : null}
      </div>
    </label>
  );
}
