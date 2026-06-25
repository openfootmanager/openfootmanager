const inputClass =
  "w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition";
const labelClass =
  "text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400";

interface LabeledInputProps {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
}

export function LabeledInput({ label, value, onChange, type = "text", placeholder }: LabeledInputProps) {
  return (
    <div className="flex flex-col gap-1">
      <label className={labelClass}>{label}</label>
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
  onChange: (v: string) => void;
}

export function LabeledSelect({ label, value, options, onChange }: LabeledSelectProps) {
  return (
    <div className="flex flex-col gap-1">
      <label className={labelClass}>{label}</label>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={inputClass}
      >
        {options.map((o) => (
          <option key={o} value={o}>
            {o}
          </option>
        ))}
      </select>
    </div>
  );
}
