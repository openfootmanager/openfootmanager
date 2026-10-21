import { type KeyboardEvent, type ReactNode, useRef } from "react";

import { rovingTarget } from "./roving";
import { SEGMENTED } from "./segmented";

export interface SegmentedOption<V extends string> {
  value: V;
  label: string;
  icon?: ReactNode;
}

interface SegmentedControlProps<V extends string> {
  /** The id of the visible label that names the group. */
  labelledBy: string;
  options: readonly SegmentedOption<V>[];
  value: V;
  onChange: (value: V) => void;
  /** `strong` fills the chosen option, for a setting the player watches; `quiet` raises it. */
  emphasis?: "quiet" | "strong";
  /** `stacked` puts each icon over its label in a 52px target; `inline` is one compact line. */
  layout?: "inline" | "stacked";
  /** Stretch across the container, the options sharing its width. */
  fill?: boolean;
}

/**
 * A choice of one value from a few, as a WAI-ARIA radio group: one tab stop on the chosen option,
 * and the arrow keys (all four), Home and End move the choice and the focus together. It draws
 * Tabs' segmented look; a segmented switch that changes what a panel shows is Tabs instead.
 */
export function SegmentedControl<V extends string>({
  labelledBy,
  options,
  value,
  onChange,
  emphasis = "quiet",
  layout = "inline",
  fill = false,
}: SegmentedControlProps<V>) {
  const radios = useRef(new Map<V, HTMLButtonElement>());
  // A value no option has must not leave the group with no tab stop at all.
  const tabStop = options.some((option) => option.value === value) ? value : options[0]?.value;

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const target = rovingTarget(event.key, index, options.length, { upDown: true });
    if (target === null) return;
    event.preventDefault();
    const option = options[target];
    onChange(option.value);
    radios.current.get(option.value)?.focus();
  }

  return (
    <div
      role="radiogroup"
      aria-labelledby={labelledBy}
      className={`${SEGMENTED.track} ${fill ? "flex w-full" : ""}`}
    >
      {options.map((option, index) => {
        const checked = option.value === value;
        return (
          <button
            key={option.value}
            ref={(node) => {
              if (node) radios.current.set(option.value, node);
              else radios.current.delete(option.value);
            }}
            type="button"
            role="radio"
            aria-checked={checked}
            tabIndex={option.value === tabStop ? 0 : -1}
            onClick={() => onChange(option.value)}
            onKeyDown={(event) => handleKeyDown(event, index)}
            className={`${SEGMENTED.item} ${SEGMENTED[layout]} ${fill ? "flex-1 justify-center" : ""} ${
              checked ? SEGMENTED.selected[emphasis] : SEGMENTED.idle
            }`}
          >
            {option.icon ? (
              <span aria-hidden="true" className="[&>svg]:h-5 [&>svg]:w-5">
                {option.icon}
              </span>
            ) : null}
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
