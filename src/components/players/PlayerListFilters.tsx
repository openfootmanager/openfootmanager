import { type ReactNode, useEffect, useRef, useState } from "react";
import { Search } from "lucide-react";
import { useTranslation } from "react-i18next";

import { CORE_POSITIONS, positionsInGroup } from "../../lib/positions";
import type {
  PlayerStatusFilter,
  PlayersPageQuery,
  PositionFilter,
  StatusCounts,
} from "../../services/playersService";
import { translatePositionLabel } from "../squad/SquadTab.helpers";

const chip =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all";
const chipOn = "bg-primary-700 text-white shadow-sm";
const chipOff =
  "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600";

const STATUS_LABEL_KEY: Record<PlayerStatusFilter, string> = {
  all: "common.all",
  transfer: "transfers.transfer",
  loan: "transfers.loan",
  free_agent: "common.freeAgent",
  // On the transfer market "everyone you can approach" is what All means.
  available: "common.all",
};

/** The detailed positions a query's position tests stand for, whichever shorthand wrote them. */
function chosenPositions(positions: PositionFilter[]): Set<string> {
  const chosen = new Set<string>();
  for (const filter of positions) {
    if ("exact" in filter) chosen.add(filter.exact);
    else for (const member of positionsInGroup(filter.group)) chosen.add(member);
  }
  return chosen;
}

/** Writes a chosen set back, a fully chosen group as its shorthand so broad stored values match. */
function toPositionFilters(chosen: Set<string>): PositionFilter[] {
  const filters: PositionFilter[] = [];
  for (const group of CORE_POSITIONS) {
    const members = positionsInGroup(group);
    const inGroup = members.filter((member) => chosen.has(member));
    if (inGroup.length === members.length) filters.push({ group });
    else filters.push(...inGroup.map((exact) => ({ exact })));
  }
  return filters;
}

interface PositionGroupChipProps {
  group: string;
  chosen: Set<string>;
  open: boolean;
  onPress: () => void;
  onToggle: (position: string) => void;
}

function PositionGroupChip({ group, chosen, open, onPress, onToggle }: PositionGroupChipProps) {
  const { t } = useTranslation();
  const members = positionsInGroup(group);
  const refinable = members.length > 1;
  const selected = members.filter((member) => chosen.has(member)).length;
  const active = selected > 0;
  const partial = active && refinable && selected < members.length;
  const label = t(`common.positionGroups.${group}`, { defaultValue: group });

  return (
    <div className="relative">
      <button
        type="button"
        onClick={onPress}
        aria-expanded={refinable ? open : undefined}
        aria-pressed={partial ? "mixed" : active}
        aria-label={
          partial
            ? t("transfers.positionGroupPartialSelection", {
                group: label,
                selected,
                total: members.length,
              })
            : label
        }
        className={`${chip} inline-flex items-center gap-1 ${active ? chipOn : chipOff}`}
      >
        {t(`common.posAbbr.${group}`)}
        {partial && (
          <span
            aria-hidden="true"
            className="bg-white/20 text-xs px-1.5 py-0.5 rounded-full leading-none"
          >
            {selected}/{members.length}
          </span>
        )}
      </button>
      {refinable && open && (
        <div
          role="group"
          aria-label={t("transfers.refinePositionGroup", { group: label })}
          className="absolute left-0 top-full mt-1 z-20 min-w-[180px] p-2 rounded-lg bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 shadow-lg"
        >
          <div className="flex flex-wrap gap-1.5">
            {members.map((position) => {
              const on = chosen.has(position);
              const name = translatePositionLabel(t, position);
              return (
                <button
                  type="button"
                  key={position}
                  onClick={() => onToggle(position)}
                  aria-pressed={on}
                  aria-label={name}
                  title={name}
                  className={`${chip} px-2.5 py-1 rounded-md ${on ? chipOn : `${chipOff} hover:text-gray-700 dark:hover:text-gray-200`}`}
                >
                  {t(`common.posAbbr.${position}`)}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

// The backend reads ages as integers and ratings as 0-255; nothing real goes past 100, so a bound
// typed beyond it is held there instead of failing the whole page request.
const MAX_RANGE_BOUND = 100;

interface RangeInputProps {
  label: string;
  value: number | null;
  onChange: (value: number | null) => void;
}

/** A typed bound as a whole number in range, or null when it is empty or not a number. */
function toRangeBound(raw: string): number | null {
  if (raw === "") return null;
  const typed = Number(raw);
  if (!Number.isFinite(typed)) return null;
  return Math.min(MAX_RANGE_BOUND, Math.max(0, Math.trunc(typed)));
}

function RangeInput({ label, value, onChange }: RangeInputProps) {
  return (
    <input
      type="number"
      inputMode="numeric"
      min={0}
      max={MAX_RANGE_BOUND}
      step={1}
      aria-label={label}
      placeholder={label}
      value={value ?? ""}
      onChange={(event) => {
        onChange(toRangeBound(event.target.value));
      }}
      className="w-24 px-2 py-1.5 rounded-lg bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 text-xs text-gray-800 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-500/50 focus:ring-offset-2 dark:focus:ring-offset-navy-800"
    />
  );
}

interface PlayerListFiltersProps {
  query: PlayersPageQuery;
  onChange: (patch: Partial<PlayersPageQuery>) => void;
  /** The status chips this screen offers, in order. */
  statusOptions: PlayerStatusFilter[];
  /** What the backend counted for each status chip; the chips show them when given. */
  statusCounts?: StatusCounts;
  /** Offered where the user's club buys players: only those it can pay for. */
  showInBudget?: boolean;
  /** Anything else this screen filters by (a team picker, say), shown at the end of the row. */
  trailing?: ReactNode;
}

/**
 * The filters every list of players shares: search, position (a group, narrowed to exact
 * positions), status, age / rating / value ranges and, where it makes sense, "in budget". It
 * holds no filter state of its own beyond which refinement panel is open: the query is the owner's.
 */
export default function PlayerListFilters({
  query,
  onChange,
  statusOptions,
  statusCounts,
  showInBudget = false,
  trailing,
}: PlayerListFiltersProps) {
  const { t } = useTranslation();
  const [openGroup, setOpenGroup] = useState<string | null>(null);
  const positionsRef = useRef<HTMLDivElement | null>(null);
  const chosen = chosenPositions(query.positions);

  useEffect(() => {
    if (!openGroup) return;
    const closeOnOutsideClick = (event: MouseEvent) => {
      if (positionsRef.current?.contains(event.target as Node)) return;
      setOpenGroup(null);
    };
    // Not a modal, so Escape only closes the panel and puts focus back on the chip that opened it.
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      positionsRef.current?.querySelector<HTMLElement>('[aria-expanded="true"]')?.focus();
      setOpenGroup(null);
    };
    document.addEventListener("mousedown", closeOnOutsideClick);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("mousedown", closeOnOutsideClick);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [openGroup]);

  const pressGroup = (group: string) => {
    const members = positionsInGroup(group);
    if (members.length <= 1) {
      const next = new Set(chosen);
      if (next.has(group)) next.delete(group);
      else next.add(group);
      onChange({ positions: toPositionFilters(next) });
      setOpenGroup(null);
      return;
    }
    // Pressing the chip whose panel is open just closes it: the player is done refining.
    if (openGroup === group) {
      setOpenGroup(null);
      return;
    }
    // A chip that has the whole group chosen takes it back, so a group can be un-chosen without
    // clearing the others; to refine, press it again and its panel opens.
    if (members.every((member) => chosen.has(member))) {
      const rest = new Set(chosen);
      for (const member of members) rest.delete(member);
      onChange({ positions: toPositionFilters(rest) });
      return;
    }
    // Otherwise the group is chosen whole, which is a shortcut that keeps earlier picks elsewhere.
    onChange({ positions: toPositionFilters(new Set([...chosen, ...members])) });
    setOpenGroup(group);
  };

  const togglePosition = (position: string) => {
    const next = new Set(chosen);
    if (next.has(position)) next.delete(position);
    else next.add(position);
    onChange({ positions: toPositionFilters(next) });
  };

  return (
    <div className="flex flex-wrap gap-3 mb-4 items-center">
      <div className="relative flex-1 min-w-[200px] max-w-sm">
        <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 dark:text-gray-500" />
        <input
          type="text"
          placeholder={t("players.searchPlaceholder")}
          value={query.search ?? ""}
          onChange={(event) => onChange({ search: event.target.value || null })}
          className="w-full pl-9 pr-3 py-2 rounded-lg bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-500/50 focus:ring-offset-2 dark:focus:ring-offset-navy-800"
        />
      </div>

      <div ref={positionsRef} className="flex gap-1.5">
        <button
          type="button"
          onClick={() => {
            onChange({ positions: [] });
            setOpenGroup(null);
          }}
          aria-pressed={chosen.size === 0}
          aria-label={t("transfers.allPositions")}
          className={`${chip} ${chosen.size === 0 ? chipOn : chipOff}`}
        >
          {t("common.all")}
        </button>
        {CORE_POSITIONS.map((group) => (
          <PositionGroupChip
            key={group}
            group={group}
            chosen={chosen}
            open={openGroup === group}
            onPress={() => pressGroup(group)}
            onToggle={togglePosition}
          />
        ))}
      </div>

      <div className="flex flex-wrap gap-1.5">
        {statusOptions.map((status) => (
          <button
            type="button"
            key={status}
            onClick={() => onChange({ status })}
            aria-pressed={query.status === status}
            className={`${chip} ${query.status === status ? "bg-accent-500 text-navy-900 shadow-sm" : chipOff}`}
          >
            {t(STATUS_LABEL_KEY[status])}
            {statusCounts ? ` (${statusCounts[status]})` : ""}
          </button>
        ))}
        {showInBudget && (
          <button
            type="button"
            onClick={() => onChange({ in_budget: !query.in_budget })}
            aria-pressed={query.in_budget}
            title={t("transfers.affordableOnlyHint")}
            className={`${chip} ${query.in_budget ? chipOn : chipOff}`}
          >
            {t("transfers.affordableOnly")}
          </button>
        )}
      </div>

      <div className="flex flex-wrap gap-1.5">
        <RangeInput
          label={t("players.filterAgeMin")}
          value={query.age_min}
          onChange={(value) => onChange({ age_min: value })}
        />
        <RangeInput
          label={t("players.filterAgeMax")}
          value={query.age_max}
          onChange={(value) => onChange({ age_max: value })}
        />
        <RangeInput
          label={t("players.filterOvrMin")}
          value={query.ovr_min}
          onChange={(value) => onChange({ ovr_min: value })}
        />
        <RangeInput
          label={t("players.filterOvrMax")}
          value={query.ovr_max}
          onChange={(value) => onChange({ ovr_max: value })}
        />
      </div>
      {trailing}
    </div>
  );
}
