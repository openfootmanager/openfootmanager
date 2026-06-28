import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronDown } from "lucide-react";
import type { CountryFlag as CountryFlagType } from "./CountryFlag";

type NationalityOption = { code: string; name: string };

type CountryResources = {
  allNationalities: (locale?: string) => NationalityOption[];
  countryName: (countryCode: string, locale?: string) => string;
  CountryFlag: typeof CountryFlagType;
};

let cachedResources: Promise<CountryResources> | null = null;

function loadCountryResources(): Promise<CountryResources> {
  cachedResources ??= Promise.all([
    import("../../lib/countries"),
    import("./CountryFlag"),
  ]).then(([countriesModule, flagModule]) => ({
    allNationalities: countriesModule.allNationalities,
    countryName: countriesModule.countryName,
    CountryFlag: flagModule.CountryFlag,
  }));
  return cachedResources;
}

function normaliseSearch(value: string): string {
  return value.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

interface CountryComboboxProps {
  label: string;
  value: string;
  onChange: (code: string) => void;
  placeholder?: string;
}

export function CountryCombobox({ label, value, onChange, placeholder }: CountryComboboxProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.language;
  const ref = useRef<HTMLDivElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [resources, setResources] = useState<CountryResources | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setIsOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [isOpen]);

  useEffect(() => {
    if (!resources && value) void ensureResources();
  }, [value, resources]);

  async function ensureResources() {
    if (resources || loading) return;
    setLoading(true);
    try {
      setResources(await loadCountryResources());
    } finally {
      setLoading(false);
    }
  }

  function open() {
    if (!resources) void ensureResources();
    setIsOpen(true);
    setSearch("");
  }

  const normSearch = normaliseSearch(search);
  const options = useMemo(() => resources?.allNationalities(locale) ?? [], [resources, locale]);
  const filtered = useMemo(
    () =>
      options.filter(
        (e) =>
          normaliseSearch(e.name).includes(normSearch) ||
          normaliseSearch(e.code).includes(normSearch),
      ),
    [options, normSearch],
  );

  const selectedLabel = value ? (resources?.countryName(value, locale) ?? value) : null;

  return (
    <div className="flex flex-col gap-1" ref={ref}>
      <label className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
        {label}
      </label>
      <div className="relative">
        <button
          type="button"
          onMouseDown={(e) => {
            e.preventDefault();
            isOpen ? setIsOpen(false) : open();
          }}
          onClick={(e) => {
            if (e.detail === 0) isOpen ? setIsOpen(false) : open();
          }}
          className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-left transition focus:outline-none focus:ring-2 focus:ring-primary-400 min-h-[38px]"
        >
          {selectedLabel ? (
            <span className="flex items-center gap-2 text-gray-900 dark:text-white">
              {resources && <resources.CountryFlag code={value} locale={locale} className="text-base leading-none" />}
              <span>{selectedLabel}</span>
            </span>
          ) : (
            <span className="text-gray-400 dark:text-gray-500">{placeholder ?? "—"}</span>
          )}
          <ChevronDown
            className={`absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400 transition-transform ${isOpen ? "rotate-180" : ""}`}
          />
        </button>

        {isOpen && (
          <div className="absolute top-full left-0 right-0 z-50 mt-1 overflow-hidden rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 shadow-xl">
            {resources ? (
              <>
                <div className="border-b border-gray-100 dark:border-navy-600 p-2">
                  <input
                    type="text"
                    autoFocus
                    placeholder={t("worldEditor.searchCountries")}
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                    className="w-full rounded-md border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 px-3 py-1.5 text-sm text-gray-900 dark:text-white outline-none placeholder:text-gray-400 dark:placeholder:text-gray-500 focus:border-primary-500"
                  />
                </div>
                <div className="max-h-48 overflow-y-auto overscroll-contain">
                  {filtered.length === 0 ? (
                    <p className="px-3 py-2 text-xs text-gray-400 dark:text-gray-500">
                      {t("menu.noResults")}
                    </p>
                  ) : (
                    filtered.map((entry) => (
                      <button
                        key={entry.code}
                        type="button"
                        onMouseDown={(e) => {
                          e.preventDefault();
                          onChange(entry.code);
                          setIsOpen(false);
                        }}
                        className={`flex w-full items-center justify-between px-3 py-2 text-sm transition-colors ${
                          value === entry.code
                            ? "bg-primary-50 dark:bg-primary-500/10 text-primary-600 dark:text-primary-400"
                            : "text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-navy-600"
                        }`}
                      >
                        <span className="flex items-center gap-2">
                          <resources.CountryFlag
                            code={entry.code}
                            locale={locale}
                            className="text-base leading-none"
                          />
                          <span>{entry.name}</span>
                        </span>
                        {value === entry.code && (
                          <Check className="h-4 w-4 flex-shrink-0 text-primary-500" />
                        )}
                      </button>
                    ))
                  )}
                </div>
              </>
            ) : (
              <div className="flex min-h-16 items-center justify-center p-3">
                <div className="h-5 w-5 animate-spin rounded-full border-2 border-primary-500 border-t-transparent" />
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
