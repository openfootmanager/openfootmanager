/**
 * The presentational pieces of the settings page.
 *
 * Split out when the page crossed the size threshold: these are pure presentation with no state
 * and no dependency on the page itself, so they are the seam that costs nothing to cut. A choice
 * of a few is ui/SegmentedControl.
 */
import { type ReactNode, useId } from "react";

export function Section({
  title,
  icon,
  children,
}: {
  title: string;
  icon: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="bg-white dark:bg-navy-800 rounded-2xl border border-gray-200 dark:border-navy-700 shadow-sm overflow-hidden">
      <div className="flex items-center gap-2 px-6 py-4 border-b border-gray-100 dark:border-navy-700">
        <span className="text-primary-500">{icon}</span>
        <h2 className="text-sm font-heading font-bold uppercase tracking-wider text-gray-800 dark:text-gray-200">
          {title}
        </h2>
      </div>
      <div className="px-6 py-4 flex flex-col gap-5">{children}</div>
    </div>
  );
}

/**
 * A setting: its label and description, and its control. The control can name itself by the label:
 * pass a function, and it gets the label's id (a radio group's `labelledBy`, say).
 *
 * The row wraps. The label keeps at least 16rem, and a control too wide to sit beside it (four long
 * German or Russian options at a large UI scale) drops below it instead of crushing it.
 */
export function SettingRow({
  label,
  description,
  danger,
  children,
}: {
  label: string;
  description: string;
  danger?: boolean;
  children: ReactNode | ((labelId: string) => ReactNode);
}) {
  const labelId = useId();
  return (
    <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
      <div className="min-w-0 grow basis-64">
        <p
          id={labelId}
          className={`text-sm font-medium ${danger ? "text-red-500" : "text-gray-800 dark:text-gray-200"}`}
        >
          {label}
        </p>
        <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{description}</p>
      </div>
      <div className="max-w-full">
        {typeof children === "function" ? children(labelId) : children}
      </div>
    </div>
  );
}

export function Toggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      className={`relative w-11 h-6 rounded-full transition-colors duration-200 ${
        checked ? "bg-primary-500" : "bg-gray-300 dark:bg-navy-600"
      }`}
    >
      <div
        className={`absolute top-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform duration-200 ${
          checked ? "translate-x-[22px]" : "translate-x-0.5"
        }`}
      />
    </button>
  );
}
