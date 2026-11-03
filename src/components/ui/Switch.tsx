interface NamedBy {
  /** Id of the visible label that names the setting (a settings row's label). */
  labelledBy: string;
  label?: never;
}

interface Labelled {
  /** A translated name, for a switch with no visible label beside it. */
  label: string;
  labelledBy?: never;
}

type SwitchProps = (NamedBy | Labelled) & {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
};

/**
 * An on/off setting that takes effect at once, as a WAI-ARIA switch: a button that says whether it
 * is on, and asks for the other state when pressed (a click, Space or Enter). A choice that is
 * submitted later is a Checkbox instead.
 *
 * The thumb is white on the selected fill when on, and the panel's colour on a muted track when
 * off: at least 4.9:1 against its track in both themes, so the state reads by more than position.
 */
export function Switch({ labelledBy, label, checked, onChange, disabled = false }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-labelledby={labelledBy}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors duration-200 motion-reduce:transition-none focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page disabled:cursor-not-allowed disabled:opacity-50 ${
        checked ? "bg-selected-fill" : "bg-fg-subtle"
      }`}
    >
      <span
        aria-hidden="true"
        className={`h-5 w-5 rounded-full shadow-sm transition-transform duration-200 motion-reduce:transition-none ${
          checked ? "translate-x-5.5 bg-on-brand" : "translate-x-0.5 bg-panel"
        }`}
      />
    </button>
  );
}
