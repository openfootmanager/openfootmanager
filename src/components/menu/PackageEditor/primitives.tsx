import { HelpCircle } from "lucide-react";
import { useState, useRef, useEffect } from "react";
import { Select } from "../../../components/ui/Select";

export const inputClass =
  "w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition";
export const labelClass =
  "text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400";

interface LabeledInputProps {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
  help?: string;
}

export function LabeledInput({ label, value, onChange, type = "text", placeholder, help }: LabeledInputProps) {
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1.5">
        <label className={labelClass}>{label}</label>
        {help && <InlineHelp text={help} />}
      </div>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className={inputClass}
      />
    </div>
  );
}

interface LabeledSelectProps {
  label: string;
  value: string;
  options: string[];
  optionLabels?: Record<string, string>;
  onChange: (v: string) => void;
  help?: string;
}

export function LabeledSelect({ label, value, options, optionLabels, onChange, help }: LabeledSelectProps) {
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1.5">
        <label className={labelClass}>{label}</label>
        {help && <InlineHelp text={help} />}
      </div>
      <Select
        fullWidth
        value={value}
        onChange={(e) => onChange(e.target.value)}
      >
        {options.map((o) => (
          <option key={o} value={o}>
            {optionLabels?.[o] ?? o}
          </option>
        ))}
      </Select>
    </div>
  );
}

interface InlineHelpProps {
  text: string;
}

export function InlineHelp({ text }: InlineHelpProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  return (
    <div ref={ref} className="relative inline-flex">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="text-gray-400 dark:text-gray-500 hover:text-primary-500 dark:hover:text-primary-400 transition-colors"
        aria-label="Help"
      >
        <HelpCircle className="w-3.5 h-3.5" />
      </button>
      {open && (
        <div className="absolute left-0 top-full mt-1 z-50 w-64 rounded-xl border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800 shadow-xl p-3 text-xs text-gray-700 dark:text-gray-300 leading-relaxed">
          {text}
        </div>
      )}
    </div>
  );
}
