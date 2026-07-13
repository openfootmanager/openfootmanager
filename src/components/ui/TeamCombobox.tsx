import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronDown } from "lucide-react";

export type TeamOption = { id: string; name?: string };

function normaliseSearch(value: string): string {
  return value.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

interface TeamComboboxProps {
  value: string;
  teams: TeamOption[];
  onChange: (teamId: string) => void;
  label?: string;
  placeholder?: string;
}

/**
 * Searchable, suggestion-based team picker constrained to existing teams — the
 * value is always a team id from `teams`, never free text. Used where a plain
 * dropdown would be unusably long (e.g. career-history clubs).
 */
export function TeamCombobox({ value, teams, onChange, label, placeholder }: TeamComboboxProps) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDivElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");

  useEffect(() => {
    if (!isOpen) return;
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setIsOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [isOpen]);

  const options = useMemo(() => teams.filter((team) => team.id), [teams]);
  const normSearch = normaliseSearch(search);
  const filtered = useMemo(
    () =>
      options.filter(
        (team) =>
          normaliseSearch(team.name ?? "").includes(normSearch) ||
          normaliseSearch(team.id).includes(normSearch),
      ),
    [options, normSearch],
  );

  const selected = options.find((team) => team.id === value);
  const selectedLabel = selected ? selected.name || selected.id : null;

  const labelClass =
    "mb-1.5 block text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400";
  const triggerClass =
    "w-full rounded-lg border bg-white dark:bg-navy-700 border-gray-200 dark:border-navy-600 px-3 py-2 text-left text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition relative";

  return (
    <div ref={ref} className={isOpen ? "relative z-50" : "relative"}>
      {label ? <label className={labelClass}>{label}</label> : null}
      <button
        type="button"
        onClick={() => {
          setIsOpen((open) => !open);
          setSearch("");
        }}
        className={triggerClass}
      >
        <span className={selectedLabel ? "text-gray-900 dark:text-white" : "text-gray-400 dark:text-gray-500"}>
          {selectedLabel ?? placeholder ?? t("worldEditor.selectTeam")}
        </span>
        <ChevronDown
          className={`absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400 transition-transform ${isOpen ? "rotate-180" : ""}`}
        />
      </button>

      {isOpen ? (
        <div className="absolute left-0 right-0 z-50 mt-1 overflow-hidden rounded-lg border border-gray-200 bg-white shadow-xl dark:border-navy-600 dark:bg-navy-700">
          <div className="border-b border-gray-100 p-2 dark:border-navy-600">
            <input
              type="text"
              autoFocus
              placeholder={t("worldEditor.searchTeams")}
              aria-label={t("worldEditor.searchTeams")}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="w-full rounded-md border border-gray-200 bg-gray-50 px-3 py-2 text-sm text-gray-900 outline-none transition-colors placeholder:text-gray-400 focus:border-primary-500 dark:border-navy-600 dark:bg-navy-800 dark:text-white dark:placeholder:text-gray-500"
            />
          </div>
          <div className="max-h-64 overflow-y-auto overscroll-contain">
            {filtered.length === 0 ? (
              <p className="px-3 py-2 text-xs text-gray-400 dark:text-gray-500">{t("menu.noResults")}</p>
            ) : (
              filtered.map((team) => (
                <button
                  key={team.id}
                  type="button"
                  onClick={() => {
                    onChange(team.id);
                    setIsOpen(false);
                    setSearch("");
                  }}
                  className={`flex w-full items-center justify-between px-3 py-2 text-left text-sm transition-colors ${
                    value === team.id
                      ? "bg-primary-50 text-primary-600 dark:bg-primary-500/10 dark:text-primary-400"
                      : "text-gray-700 hover:bg-gray-50 dark:text-gray-200 dark:hover:bg-navy-600"
                  }`}
                >
                  <span>{team.name || team.id}</span>
                  {value === team.id ? <Check className="h-4 w-4 text-primary-500" /> : null}
                </button>
              ))
            )}
          </div>
        </div>
      ) : null}
    </div>
  );
}
