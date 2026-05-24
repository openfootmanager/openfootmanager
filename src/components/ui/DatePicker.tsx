import { useState, useEffect, useRef, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, Check } from "lucide-react";

interface DatePickerProps {
  value: string; // YYYY-MM-DD
  onChange: (date: string) => void;
  error?: boolean;
}

interface DateParts {
  day: string;
  month: string;
  year: string;
}

interface MonthOption {
  value: string;
  label: string;
}

function parseDateValue(value: string): DateParts | null {
  const parts = value.split("-");
  if (parts.length !== 3) {
    return null;
  }

  const [year, month, day] = parts;
  return { day, month, year };
}

function formatDateValue(day: string, month: string, year: string) {
  return `${year}-${month.padStart(2, "0")}-${day.padStart(2, "0")}`;
}

function getDaysInMonth(month: number, year: number) {
  return new Date(year, month, 0).getDate();
}

function clampDayValue(dayValue: string, monthValue: string, yearValue: string) {
  if (!dayValue || parseInt(dayValue) <= 0) {
    return dayValue;
  }

  const monthNumber = parseInt(monthValue) || 1;
  const yearNumber = parseInt(yearValue) || 2000;
  const maxDays = getDaysInMonth(monthNumber, yearNumber);
  return Math.min(parseInt(dayValue), maxDays).toString();
}

function normaliseDayOnBlur(dayValue: string) {
  if (dayValue && parseInt(dayValue) > 0) {
    return parseInt(dayValue).toString().padStart(2, "0");
  }

  return "";
}

function normaliseYearOnBlur(yearValue: string, currentYear: number) {
  if (yearValue.length === 0 || yearValue.length === 4) {
    return yearValue;
  }

  const parsedYear = parseInt(yearValue);
  if (Number.isNaN(parsedYear) || parsedYear >= 100) {
    return yearValue;
  }

  const currentCentury = Math.floor(currentYear / 100) * 100;
  return currentCentury + parsedYear > currentYear
    ? (currentCentury - 100 + parsedYear).toString()
    : (currentCentury + parsedYear).toString();
}

function createMonths(language: string): MonthOption[] {
  return Array.from({ length: 12 }, (_, i) => {
    const d = new Date(2000, i, 1);
    return {
      value: (i + 1).toString(),
      label: d.toLocaleString(language, { month: "long" }),
    };
  });
}

function getSelectedMonthLabel(monthValue: string, months: MonthOption[], fallback: string) {
  if (!monthValue) {
    return fallback;
  }

  return months.find(m => m.value === monthValue || m.value === parseInt(monthValue).toString())?.label ?? fallback;
}

export function DatePicker({ value, onChange, error }: DatePickerProps) {
  const { t, i18n } = useTranslation();

  // Parse initial value or use current date components
  const [day, setDay] = useState<string>("");
  const [month, setMonth] = useState<string>("");
  const [year, setYear] = useState<string>("");

  // Keep a stable ref so the notify effect below doesn't need onChange
  // in its dependency array — avoids firing with stale day/month/year
  // when the parent re-renders and passes a new inline function reference.
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const [monthOpen, setMonthOpen] = useState(false);
  const monthRef = useRef<HTMLDivElement>(null);

  // Initialize from value prop
  useEffect(() => {
    const nextValue = parseDateValue(value);
    if (nextValue) {
      setYear(nextValue.year);
      setMonth(nextValue.month);
      setDay(nextValue.day);
    }
  }, [value]);

  // Handle outside click for month dropdown
  useEffect(() => {
    if (!monthOpen || !monthRef.current) {
      return;
    }

    const monthElement = monthRef.current;

    const handleClickOutside = (e: MouseEvent) => {
      const targetNode = e.target instanceof Node ? e.target : null;
      const eventPath =
        typeof e.composedPath === "function" ? e.composedPath() : [];
      const clickedInside =
        eventPath.includes(monthElement as EventTarget) ||
        (targetNode ? monthElement.contains(targetNode) : false);

      if (!clickedInside) {
        setMonthOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [monthOpen]);

  // Update parent when any component changes, if valid
  useEffect(() => {
    if (day && month && year && year.length === 4) {
      onChangeRef.current(formatDateValue(day, month, year));
    }
  }, [day, month, year]);

  // Generate month names based on current locale
  const months = useMemo(() => createMonths(i18n.language), [i18n.language]);

  const handleDayChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    let newDay = e.target.value.replace(/\D/g, '');
    if (newDay.length > 2) newDay = newDay.slice(0, 2);

    setDay(clampDayValue(newDay, month, year));
  };

  const handleYearChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    let newYear = e.target.value.replace(/\D/g, '');
    if (newYear.length > 4) newYear = newYear.slice(0, 4);
    setYear(newYear);

    // Re-validate day if year changes (leap years)
    if (day && month && newYear.length === 4) {
      setDay(clampDayValue(day, month, newYear));
    }
  };

  const selectedMonthLabel = getSelectedMonthLabel(month, months, t('date.month'));

  return (
    <div className="flex gap-2 w-full">
      {/* Day */}
      <div className="flex-1">
        <input
          type="text"
          inputMode="numeric"
          placeholder={t('date.day', 'DD')}
          value={day}
          onChange={handleDayChange}
          onBlur={() => setDay(normaliseDayOnBlur(day))}
          className={`w-full bg-gray-50 dark:bg-navy-900 border text-gray-900 dark:text-white rounded-lg p-3 outline-none focus:ring-2 transition-all placeholder:text-gray-400 dark:placeholder:text-gray-500 text-center ${error
              ? "border-red-400 dark:border-red-500 focus:border-red-500 focus:ring-red-500/20"
              : "border-gray-300 dark:border-navy-600 focus:border-primary-500 focus:ring-primary-500/20"
            }`}
        />
      </div>

      {/* Month Dropdown */}
      <div className="flex-[2] relative" ref={monthRef}>
        <button
          type="button"
          onClick={() => setMonthOpen(!monthOpen)}
          className={`w-full flex items-center justify-between bg-gray-50 dark:bg-navy-900 border text-left rounded-lg p-3 outline-none transition-all ${error
              ? "border-red-400 dark:border-red-500"
              : monthOpen
                ? "border-primary-500 ring-2 ring-primary-500/20"
                : "border-gray-300 dark:border-navy-600"
            }`}
        >
          <span className={month ? "text-gray-900 dark:text-white" : "text-gray-400 dark:text-gray-500"}>
            {selectedMonthLabel}
          </span>
          <ChevronDown className={`w-4 h-4 text-gray-400 transition-transform ${monthOpen ? "rotate-180" : ""}`} />
        </button>

        {monthOpen && (
          <div className="absolute z-50 top-full mt-1 left-0 right-0 bg-white dark:bg-navy-700 rounded-lg shadow-xl border border-gray-200 dark:border-navy-600 overflow-hidden">
            <div className="max-h-48 overflow-y-auto">
              {months.map(m => (
                <button
                  key={m.value}
                  type="button"
                  onClick={() => {
                    const nextMonth = m.value.padStart(2, '0');
                    setMonth(nextMonth);
                    setMonthOpen(false);
                    // Re-validate day
                    if (day && year.length === 4) {
                      const clampedDay = clampDayValue(day, nextMonth, year);
                      if (clampedDay !== day) {
                        setDay(clampedDay.padStart(2, '0'));
                      }
                    }
                  }}
                  className={`w-full text-left px-3 py-2 text-sm flex items-center justify-between transition-colors ${(month === m.value || month === m.value.padStart(2, '0'))
                      ? "bg-primary-50 dark:bg-primary-500/10 text-primary-600 dark:text-primary-400"
                      : "text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-navy-600"
                    }`}
                >
                  <span>{m.label}</span>
                  {(month === m.value || month === m.value.padStart(2, '0')) && <Check className="w-4 h-4 text-primary-500" />}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Year */}
      <div className="flex-[1.5]">
        <input
          type="text"
          inputMode="numeric"
          placeholder={t('date.year', 'YYYY')}
          value={year}
          onChange={handleYearChange}
          onBlur={() => {
            if (year.length > 0 && year.length < 4) {
              const normalisedYear = normaliseYearOnBlur(year, new Date().getFullYear());
              if (normalisedYear !== year) {
                setYear(normalisedYear);
              }
            }
          }}
          className={`w-full bg-gray-50 dark:bg-navy-900 border text-gray-900 dark:text-white rounded-lg p-3 outline-none focus:ring-2 transition-all placeholder:text-gray-400 dark:placeholder:text-gray-500 text-center ${error
              ? "border-red-400 dark:border-red-500 focus:border-red-500 focus:ring-red-500/20"
              : "border-gray-300 dark:border-navy-600 focus:border-primary-500 focus:ring-primary-500/20"
            }`}
        />
      </div>
    </div>
  );
}
