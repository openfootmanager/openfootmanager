import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronDown } from "lucide-react";
import { TeamColorsDef } from "../menu/PackageEditor/types";

type TeamOption = {
  id: string;
  label: string;
  logo?: string | null;
  shortName?: string;
  colors?: TeamColorsDef;
};

interface TeamComboboxProps {
  label: string;
  value: string;
  options: TeamOption[];
  onChange: (id: string) => void;
  projectDir?: string;
  placeholder?: string;
}

function normaliseSearch(value: string): string {
  return value.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

export function TeamCombobox({ label, value, options, onChange, placeholder }: TeamComboboxProps) {
  const { t } = useTranslation();
  const labelId = useId();
  const buttonId = useId();
  const listboxId = useId();
  const ref = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    if (!isOpen) return;
    function handleClick(event: MouseEvent) {
      if (ref.current && !ref.current.contains(event.target as Node)) setIsOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    listRef.current?.querySelector<HTMLElement>('[data-active="true"]')?.scrollIntoView?.({ block: "nearest" });
  }, [activeIndex, isOpen, search]);

  const filtered = useMemo(() => {
    const query = normaliseSearch(search);
    return options.filter(
      (option) =>
        normaliseSearch(option.label).includes(query) || normaliseSearch(option.id).includes(query),
    );
  }, [options, search]);
  const active = Math.min(activeIndex, filtered.length);
  const selected = options.find((option) => option.id === value);
  const optionId = (index: number) => `${listboxId}-option-${index}`;

  function close() {
    setIsOpen(false);
    triggerRef.current?.focus();
  }

  function select(id: string) {
    onChange(id);
    close();
  }

  function handleSearchKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (filtered.length === 0 && event.key !== "Escape") return;
    const optionCount = filtered.length + 1;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        setActiveIndex((active + 1) % optionCount);
        break;
      case "ArrowUp":
        event.preventDefault();
        setActiveIndex((active - 1 + optionCount) % optionCount);
        break;
      case "Home":
        event.preventDefault();
        setActiveIndex(0);
        break;
      case "End":
        event.preventDefault();
        setActiveIndex(filtered.length);
        break;
      case "Enter":
        event.preventDefault();
        select(active === 0 ? "" : filtered[active - 1].id);
        break;
      case "Escape":
        event.preventDefault();
        close();
        break;
    }
  }

  return (
    <div className="flex flex-col gap-1" ref={ref}>
      <label id={labelId} htmlFor={buttonId} className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
        {label}
      </label>
      <div className="relative">
        <button
          type="button"
          id={buttonId}
          ref={triggerRef}
          aria-labelledby={`${labelId} ${buttonId}`}
          aria-haspopup="listbox"
          aria-expanded={isOpen}
          aria-controls={isOpen ? listboxId : undefined}
          onMouseDown={(event) => {
            event.preventDefault();
            if (isOpen) close();
            else {
              setIsOpen(true);
              setSearch("");
              setActiveIndex(0);
            }
          }}
          onClick={(event) => {
            if (event.detail === 0) {
              if (isOpen) close();
              else setIsOpen(true);
            }
          }}
          className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-left transition focus:outline-none focus:ring-2 focus:ring-primary-400 min-h-[38px]"
        >
          <span className={selected ? "text-gray-900 dark:text-white" : "text-gray-400 dark:text-gray-500"}>
            {selected?.label ?? placeholder ?? "—"}
          </span>
          <ChevronDown className={`absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400 transition-transform ${isOpen ? "rotate-180" : ""}`} />
        </button>

        {isOpen && (
          <div className="absolute top-full left-0 right-0 z-50 mt-1 overflow-hidden rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 shadow-xl">
            <div className="border-b border-gray-100 dark:border-navy-600 p-2">
              <input
                type="text"
                autoFocus
                role="combobox"
                aria-expanded="true"
                aria-controls={listboxId}
                aria-activedescendant={filtered.length > 0 ? optionId(active) : undefined}
                aria-autocomplete="list"
                aria-label={label}
                placeholder={t("teams.searchPlaceholder")}
                value={search}
                onChange={(event) => {
                  setSearch(event.target.value);
                  setActiveIndex(0);
                }}
                onKeyDown={handleSearchKeyDown}
                className="w-full rounded-md border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 px-3 py-1.5 text-sm text-gray-900 dark:text-white outline-none placeholder:text-gray-400 dark:placeholder:text-gray-500 focus:border-primary-500"
              />
            </div>
            <div className="max-h-48 overflow-y-auto overscroll-contain">
              <div role="listbox" id={listboxId} aria-label={label} ref={listRef}>
                {filtered.length === 0 && (
                  <p className="px-3 py-2 text-xs text-gray-400 dark:text-gray-500">{t("teams.noResults")}</p>
                )}
                {filtered.length > 0 && (
                  <button
                    type="button"
                    role="option"
                    id={optionId(0)}
                    aria-selected={value === ""}
                    data-active={active === 0}
                    tabIndex={-1}
                    onMouseDown={(event) => {
                      event.preventDefault();
                      select("");
                    }}
                    onClick={(event) => {
                      if (event.detail === 0) select("");
                    }}
                    className={`flex w-full items-center justify-between px-3 py-2 text-sm transition-colors ${value === "" ? "bg-primary-50 dark:bg-primary-500/10 text-primary-600 dark:text-primary-400" : "text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-navy-600"} ${active === 0 ? "ring-2 ring-inset ring-primary-400 dark:ring-primary-500" : ""}`}
                  >
                    <span>{placeholder ?? "—"}</span>
                    {value === "" && <Check className="h-4 w-4 flex-shrink-0 text-primary-500" />}
                  </button>
                )}
                {filtered.map((option, index) => (
                  <button
                    key={option.id}
                    id={optionId(index + 1)}
                    type="button"
                    role="option"
                    aria-selected={value === option.id}
                    data-active={index + 1 === active}
                    tabIndex={-1}
                    onMouseDown={(event) => {
                      event.preventDefault();
                      select(option.id);
                    }}
                    onClick={(event) => {
                      if (event.detail === 0) select(option.id);
                    }}
                    className={`flex w-full items-center justify-between px-3 py-2 text-sm transition-colors ${value === option.id ? "bg-primary-50 dark:bg-primary-500/10 text-primary-600 dark:text-primary-400" : "text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-navy-600"} ${index + 1 === active ? "ring-2 ring-inset ring-primary-400 dark:ring-primary-500" : ""}`}
                  >
                    <span className="flex items-center gap-2">
                      <span>{option.label}</span>
                    </span>
                    {value === option.id && <Check className="h-4 w-4 flex-shrink-0 text-primary-500" />}
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
