import { useEffect, useId, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronDown } from "lucide-react";
import type { TeamColorsDef } from "../menu/PackageEditor/types";
import { useListboxKeys } from "./useListboxKeys";
import { VirtualListbox, listboxOptionId } from "./VirtualListbox";

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

/** One option's height: a 1.25rem line plus 0.5rem of padding above and below, so every option has it. */
const OPTION_HEIGHT_REM = 2.25;

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
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");

  useEffect(() => {
    if (!isOpen) return;
    function handleClick(event: MouseEvent) {
      if (ref.current && !ref.current.contains(event.target as Node)) setIsOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [isOpen]);

  const filtered = useMemo(() => {
    const query = normaliseSearch(search);
    return options.filter(
      (option) =>
        normaliseSearch(option.label).includes(query) || normaliseSearch(option.id).includes(query),
    );
  }, [options, search]);
  const selected = options.find((option) => option.id === value);
  // The no-team option is the first row; with nothing matching the search there are no rows at all.
  const rows = useMemo<TeamOption[]>(
    () => (filtered.length > 0 ? [{ id: "", label: placeholder ?? "—" }, ...filtered] : []),
    [filtered, placeholder],
  );

  function close() {
    setIsOpen(false);
    triggerRef.current?.focus();
  }

  function select(id: string) {
    onChange(id);
    close();
  }

  const { active, setActive, onKeyDown } = useListboxKeys({
    count: rows.length,
    onPick: (index) => select(rows[index].id),
    onEscape: close,
  });

  return (
    <div className="flex flex-col gap-1" ref={ref}>
      <label id={labelId} htmlFor={buttonId} className="text-caption font-medium text-fg-muted">
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
              setActive(0);
            }
          }}
          onClick={(event) => {
            if (event.detail === 0) {
              if (isOpen) close();
              else setIsOpen(true);
            }
          }}
          className="w-full rounded-lg border border-line bg-panel px-3 py-2 text-body text-left transition focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page min-h-[38px]"
        >
          <span className={selected ? "text-fg" : "text-fg-subtle"}>
            {selected?.label ?? placeholder ?? "—"}
          </span>
          <ChevronDown
            className={`absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-subtle transition-transform ${isOpen ? "rotate-180" : ""}`}
          />
        </button>

        {isOpen && (
          <div className="absolute top-full left-0 right-0 z-50 mt-1 overflow-hidden rounded-lg border border-line bg-panel shadow-xl">
            <div className="border-b border-line p-2">
              <input
                type="text"
                // biome-ignore lint/a11y/noAutofocus: the search field only exists while the combobox is open.
                autoFocus
                role="combobox"
                aria-expanded="true"
                aria-controls={listboxId}
                aria-activedescendant={
                  rows.length > 0 ? listboxOptionId(listboxId, active) : undefined
                }
                aria-autocomplete="list"
                aria-label={label}
                placeholder={t("teams.searchPlaceholder")}
                value={search}
                onChange={(event) => {
                  setSearch(event.target.value);
                  setActive(0);
                }}
                onKeyDown={onKeyDown}
                className="w-full rounded-md border border-line bg-sunken px-3 py-1.5 text-body text-fg outline-none placeholder:text-fg-subtle focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
              />
            </div>
            {rows.length === 0 ? (
              <p className="px-3 py-2 text-caption text-fg-subtle">{t("teams.noResults")}</p>
            ) : (
              <VirtualListbox
                id={listboxId}
                aria-label={label}
                items={rows}
                rowHeightRem={OPTION_HEIGHT_REM}
                maxHeight="12rem"
                activeIndex={active}
                resetKey={search}
                itemKey={(row) => row.id}
                isSelected={(row) => value === row.id}
                onPick={(row) => select(row.id)}
                optionClassName={({ active: isActive, selected: isSelected }) =>
                  `flex items-center justify-between px-3 text-body transition-colors ${isSelected ? "bg-selected-tint text-selected" : "text-fg hover:bg-hover"} ${isActive ? "ring-2 ring-inset ring-focus" : ""}`
                }
                renderOption={(row) => (
                  <>
                    <span className="truncate" title={row.label}>
                      {row.label}
                    </span>
                    {value === row.id && <Check className="h-4 w-4 flex-shrink-0 text-selected" />}
                  </>
                )}
              />
            )}
          </div>
        )}
      </div>
    </div>
  );
}
