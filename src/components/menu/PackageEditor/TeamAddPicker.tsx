import { useEffect, useId, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus } from "lucide-react";
import { useListboxKeys } from "../../ui/useListboxKeys";
import { VirtualListbox, listboxOptionId } from "../../ui/VirtualListbox";
import type { TeamDef } from "./types";

/** One option's height: a 1.25rem line plus 0.5rem of padding above and below. */
const OPTION_HEIGHT_REM = 2.25;

interface TeamAddPickerProps {
  /** Every team a competition could take (those with an id). */
  teams: readonly TeamDef[];
  /** The ids already in the competition: they are not offered again. */
  taken: readonly string[];
  onAdd: (teamId: string) => void;
}

/**
 * "Add team" for a competition's explicit participants: a button that opens a searchable list of the
 * teams not yet in it. The list is a windowed listbox, so a 581-club package renders a screenful of
 * options, and the search field drives it from the keyboard (arrows, Enter, Escape) as a combobox does.
 */
export function TeamAddPicker({ teams, taken, onAdd }: TeamAddPickerProps) {
  const { t } = useTranslation();
  const listboxId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");

  const available = useMemo(() => {
    const query = search.toLowerCase();
    return teams.filter(
      (team) =>
        !taken.includes(team.id) &&
        (!query || (team.name || team.id).toLowerCase().includes(query)),
    );
  }, [teams, taken, search]);

  // Whether any team is left to add, whatever the search says: the button is dead when none is.
  const anyLeft = useMemo(() => teams.some((team) => !taken.includes(team.id)), [teams, taken]);

  useEffect(() => {
    if (!isOpen) return;
    function handleClick(event: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(event.target as Node)) setIsOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [isOpen]);

  // The search field is gone once the list closes: focus goes back to the button that opened it.
  function close() {
    setIsOpen(false);
    triggerRef.current?.focus();
  }

  function add(teamId: string) {
    onAdd(teamId);
    close();
  }

  const { active, setActive, onKeyDown } = useListboxKeys({
    count: available.length,
    onPick: (index) => add(available[index].id),
    onEscape: close,
  });

  return (
    <div className="relative" ref={rootRef}>
      <button
        ref={triggerRef}
        type="button"
        // aria-disabled, not disabled: picking the last team re-renders this button as unavailable while
        // focus is on it (close() just put it there), and a disabled button would drop that focus.
        aria-disabled={!anyLeft && !isOpen}
        onClick={() => {
          if (!anyLeft && !isOpen) return;
          setSearch("");
          setActive(0);
          setIsOpen((open) => !open);
        }}
        className="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-dashed border-gray-300 dark:border-navy-500 text-sm text-gray-500 dark:text-gray-400 hover:border-primary-400 hover:text-primary-500 dark:hover:border-primary-500 dark:hover:text-primary-400 transition-all aria-disabled:opacity-40 aria-disabled:cursor-not-allowed"
      >
        <Plus className="w-4 h-4" />
        {t("worldEditor.addTeam")}
      </button>

      {isOpen && (
        <div className="absolute top-full left-0 z-50 mt-1 min-w-56 rounded-xl border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 shadow-xl">
          <div className="p-2 border-b border-gray-100 dark:border-navy-600">
            <input
              type="text"
              // biome-ignore lint/a11y/noAutofocus: the search field only exists while the team picker is open.
              autoFocus
              role="combobox"
              aria-expanded="true"
              aria-controls={listboxId}
              aria-activedescendant={
                available.length > 0 ? listboxOptionId(listboxId, active) : undefined
              }
              aria-autocomplete="list"
              aria-label={t("worldEditor.searchTeams")}
              placeholder={t("worldEditor.searchTeams")}
              value={search}
              onChange={(event) => {
                setSearch(event.target.value);
                setActive(0);
              }}
              onKeyDown={onKeyDown}
              className="w-full rounded-md border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 px-3 py-1.5 text-sm text-gray-900 dark:text-white outline-none focus:border-primary-500 placeholder:text-gray-400 dark:placeholder:text-gray-500"
            />
          </div>
          {available.length === 0 && (
            <p
              role="status"
              className="px-3 py-2 text-caption text-gray-400 dark:text-gray-500 italic"
            >
              {t("menu.noResults")}
            </p>
          )}
          <VirtualListbox
            id={listboxId}
            aria-label={t("worldEditor.searchTeams")}
            items={available}
            rowHeightRem={OPTION_HEIGHT_REM}
            maxHeight="12rem"
            activeIndex={active}
            resetKey={search}
            itemKey={(team) => team.id}
            isSelected={() => false}
            onPick={(team) => add(team.id)}
            optionClassName={({ active: isActive }) =>
              `flex items-center gap-2 px-3 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-navy-600 text-left ${isActive ? "bg-gray-50 dark:bg-navy-600 ring-2 ring-inset ring-focus" : ""}`
            }
            renderOption={(team) => (
              <>
                <span className="flex-1 truncate" title={team.name || team.id}>
                  {team.name || team.id}
                </span>
                {team.country && (
                  <span className="text-caption px-1.5 rounded bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 flex-shrink-0">
                    {team.country}
                  </span>
                )}
              </>
            )}
          />
        </div>
      )}
    </div>
  );
}
