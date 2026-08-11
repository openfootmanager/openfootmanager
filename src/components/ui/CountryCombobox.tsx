import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
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
    // The backend catalog is the source of truth for selectable nationalities
    // (#270). If it can't be fetched, fall back to the full ISO list.
    import("../../services/nationsService")
      .then((m) => m.getNationCodes())
      .catch(() => null),
  ]).then(([countriesModule, flagModule, nationCodes]) => ({
    allNationalities: (locale?: string) =>
      countriesModule.allNationalities(locale, nationCodes),
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
  const labelId = useId();
  const buttonId = useId();
  const listboxId = useId();
  const optionId = (index: number) => `${listboxId}-option-${index}`;
  const ref = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  // Which option the keyboard is on. Focus stays in the search field — moving it
  // onto the options would take the typing away — so the active option is
  // pointed at with `aria-activedescendant` rather than focused.
  const [activeIndex, setActiveIndex] = useState(0);
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

  // Follow the active option with the scroll, or arrowing past the visible few
  // moves a selection the author cannot see.
  useEffect(() => {
    if (!isOpen) return;
    const option = listRef.current?.querySelector<HTMLElement>('[data-active="true"]');
    // Guarded: jsdom has no layout and does not implement this, and keeping the
    // active option in view is a nicety the keyboard model must not depend on.
    option?.scrollIntoView?.({ block: "nearest" });
  }, [isOpen, activeIndex, search]);

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
    setActiveIndex(0);
  }

  /**
   * Close and hand focus back to the trigger.
   *
   * Every close that the author drove from inside the picker goes through here.
   * The popup holds the focused search field, so unmounting it without this
   * drops focus to `<body>` and the next Tab starts from the top of the
   * document rather than from the field they were on.
   *
   * A press *outside* the picker is the exception and closes directly: focus
   * belongs to whatever they pressed, and pulling it back to the trigger would
   * fight them.
   */
  function close() {
    setIsOpen(false);
    triggerRef.current?.focus();
  }

  function select(code: string) {
    onChange(code);
    close();
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

  // Typing narrows the list under the cursor, so an index that pointed at the
  // tenth match can be past the end of the next keystroke's three.
  const active = Math.min(activeIndex, Math.max(filtered.length - 1, 0));

  /**
   * The listbox keyboard model. Declaring `role="listbox"` and `role="option"`
   * promises this: a screen reader announces "3 of 211" and expects the arrows
   * to move through them, and a keyboard user who can open the list but not
   * walk it has no way to choose.
   */
  function handleSearchKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (filtered.length === 0 && event.key !== "Escape") {
      return;
    }
    const moveTo = (index: number) => {
      event.preventDefault();
      setActiveIndex(index);
    };

    switch (event.key) {
      case "ArrowDown":
        return moveTo((active + 1) % filtered.length);
      case "ArrowUp":
        return moveTo((active - 1 + filtered.length) % filtered.length);
      case "Home":
        return moveTo(0);
      case "End":
        return moveTo(filtered.length - 1);
      case "Enter":
        event.preventDefault();
        return select(filtered[active].code);
      case "Escape":
        event.preventDefault();
        return close();
      default:
    }
  }

  return (
    <div className="flex flex-col gap-1" ref={ref}>
      {/*
        `htmlFor` as well as the id: a button *is* a labelable element, so this
        makes clicking the caption open the picker the way clicking the caption
        of a native select does. `aria-labelledby` alone only supplied a name.
      */}
      <label
        id={labelId}
        htmlFor={buttonId}
        className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400"
      >
        {label}
      </label>
      <div className="relative">
        <button
          type="button"
          id={buttonId}
          ref={triggerRef}
          // Named after the caption *and* itself, so it reads "<field>,
          // <current value>" the way a native select does, instead of
          // announcing a bare country name with nothing to say what it is.
          aria-labelledby={`${labelId} ${buttonId}`}
          aria-haspopup="listbox"
          aria-expanded={isOpen}
          // Only once the listbox is actually on the page. The popup opens
          // before the country data loads and shows a spinner in the meantime,
          // so pointing at the listbox from the moment it opens would name an
          // element that is not there yet — which assistive tech is told to
          // treat as if the attribute were absent.
          aria-controls={isOpen && resources ? listboxId : undefined}
          onMouseDown={(e) => {
            e.preventDefault();
            isOpen ? close() : open();
          }}
          onClick={(e) => {
            if (e.detail === 0) isOpen ? close() : open();
          }}
          className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-left transition focus:outline-none focus:ring-2 focus:ring-primary-400 min-h-[38px]"
        >
          {selectedLabel ? (
            <span className="flex items-center gap-2 text-gray-900 dark:text-white">
              {resources && (
                <resources.CountryFlag
                  code={value}
                  locale={locale}
                  decorative
                  className="text-base leading-none"
                />
              )}
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
                    role="combobox"
                    aria-expanded="true"
                    aria-controls={listboxId}
                    aria-activedescendant={
                      filtered.length > 0 ? optionId(active) : undefined
                    }
                    aria-autocomplete="list"
                    aria-label={label}
                    placeholder={t("worldEditor.searchCountries")}
                    value={search}
                    onChange={(e) => {
                      setSearch(e.target.value);
                      setActiveIndex(0);
                    }}
                    onKeyDown={handleSearchKeyDown}
                    className="w-full rounded-md border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 px-3 py-1.5 text-sm text-gray-900 dark:text-white outline-none placeholder:text-gray-400 dark:placeholder:text-gray-500 focus:border-primary-500"
                  />
                </div>
                <div className="max-h-48 overflow-y-auto overscroll-contain">
                  {filtered.length === 0 && (
                    <p className="px-3 py-2 text-xs text-gray-400 dark:text-gray-500">
                      {t("menu.noResults")}
                    </p>
                  )}
                  {/*
                    A real listbox, because the trigger promises one with
                    `aria-haspopup="listbox"`. A screen reader told to expect
                    options and handed plain buttons announces the wrong thing —
                    no position, no count, no selected state. The "no matches"
                    line sits outside it: a listbox holds options, not prose.
                  */}
                  <div role="listbox" id={listboxId} aria-label={label} ref={listRef}>
                    {filtered.map((entry, index) => (
                      <button
                        key={entry.code}
                        id={optionId(index)}
                        type="button"
                        role="option"
                        aria-selected={value === entry.code}
                        // Not focused — focus stays in the search field — so the
                        // active option has to be visible on its own.
                        data-active={index === active}
                        tabIndex={-1}
                        // Pressing while the search field still has focus must
                        // not blur it before the choice lands, hence mousedown
                        // with the default prevented.
                        onMouseDown={(e) => {
                          e.preventDefault();
                          select(entry.code);
                        }}
                        // Enter and Space on a focused button fire a click and
                        // no mousedown at all, so without this the list could be
                        // reached by keyboard and never chosen from. `detail`
                        // is 0 only for activation that did not come from a
                        // pointer, which the mousedown above has already taken.
                        onClick={(e) => {
                          if (e.detail === 0) select(entry.code);
                        }}
                        className={`flex w-full items-center justify-between px-3 py-2 text-sm transition-colors ${
                          value === entry.code
                            ? "bg-primary-50 dark:bg-primary-500/10 text-primary-600 dark:text-primary-400"
                            : "text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-navy-600"
                        } ${
                          index === active
                            ? "ring-2 ring-inset ring-primary-400 dark:ring-primary-500"
                            : ""
                        }`}
                      >
                        <span className="flex items-center gap-2">
                          <resources.CountryFlag
                            code={entry.code}
                            locale={locale}
                            decorative
                            className="text-base leading-none"
                          />
                          <span>{entry.name}</span>
                        </span>
                        {value === entry.code && (
                          <Check className="h-4 w-4 flex-shrink-0 text-primary-500" />
                        )}
                      </button>
                    ))}
                  </div>
                </div>
              </>
            ) : (
              <div className="flex min-h-16 items-center justify-center p-3">
                <div className="h-5 w-5 motion-safe:animate-spin rounded-full border-2 border-primary-500 border-t-transparent" />
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
