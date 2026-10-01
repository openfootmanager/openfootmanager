import { Plus, X } from "lucide-react";
import type { ComponentProps } from "react";
import { useTranslation } from "react-i18next";
import { TeamCombobox } from "../../ui/TeamCombobox";
import { WIRE_MAX, emptyCareerEntry, parseOptionalWhole } from "./helpers";
import { LabeledInput, labelClass } from "./primitives";
import type { PlayerCareerEntryDef } from "./types";

type TeamOptions = ComponentProps<typeof TeamCombobox>["options"];

interface PlayerCareerFieldsProps {
  entries: PlayerCareerEntryDef[];
  teamOptions: TeamOptions;
  projectDir?: string;
  onChange: (entries: PlayerCareerEntryDef[]) => void;
}

/**
 * A player's past seasons, one row each.
 *
 * The club is free text first and a package team second. A package that defines
 * only Real Madrid still has to be able to record a spell at Juventus, so the name
 * is always typeable and the picker is an optional link to a club the package does
 * own (which the backend checks exists). Picking one names the row after the club
 * only when it has no name yet, and choosing "not in this package" drops the link
 * and keeps the name: neither throws away what the author typed.
 *
 * The season is left at 0 on a new row rather than defaulted to a plausible year;
 * the validator reports it until it is filled in.
 */
export function PlayerCareerFields({
  entries,
  teamOptions,
  projectDir,
  onChange,
}: PlayerCareerFieldsProps) {
  const { t } = useTranslation();

  // Always a new array and new rows: the list belongs to the store.
  function patch(index: number, change: Partial<PlayerCareerEntryDef>) {
    onChange(entries.map((entry, i) => (i === index ? { ...entry, ...change } : entry)));
  }

  function pickTeam(index: number, teamId: string) {
    const entry = entries[index];
    if (teamId === "") {
      patch(index, { teamId: null });
      return;
    }
    const team = teamOptions.find((option) => option.id === teamId);
    patch(index, { teamId, teamName: entry.teamName || (team?.label ?? teamId) });
  }

  const count =
    (index: number, key: "season" | "appearances" | "goals" | "assists") => (v: string) =>
      patch(index, { [key]: parseOptionalWhole(v, WIRE_MAX.u32) ?? 0 });

  return (
    <div className="flex flex-col gap-3">
      <p className={labelClass}>{t("worldEditor.playerCareerSection")}</p>
      {entries.map((entry, index) => (
        <div
          // Rows have no id of their own; an index key is safe because every field is controlled.
          key={index}
          role="group"
          aria-label={t("worldEditor.careerEntryRow")}
          className="flex flex-col gap-2 rounded-lg border border-gray-200 dark:border-navy-600 p-3"
        >
          <div className="grid grid-cols-2 gap-3">
            <LabeledInput
              label={t("worldEditor.careerEntryClubName")}
              value={entry.teamName}
              onChange={(v) => patch(index, { teamName: v })}
            />
            {teamOptions.length > 0 && (
              <TeamCombobox
                label={t("worldEditor.careerEntryPackageTeam")}
                value={entry.teamId ?? ""}
                options={teamOptions}
                onChange={(id) => pickTeam(index, id)}
                projectDir={projectDir}
                placeholder={t("worldEditor.careerEntryNoPackageTeam")}
              />
            )}
          </div>
          <div className="grid grid-cols-4 gap-3">
            <LabeledInput
              label={t("playerProfile.season")}
              type="number"
              value={entry.season.toString()}
              onChange={count(index, "season")}
            />
            <LabeledInput
              label={t("playerProfile.apps")}
              type="number"
              value={entry.appearances.toString()}
              onChange={count(index, "appearances")}
            />
            <LabeledInput
              label={t("playerProfile.goals")}
              type="number"
              value={entry.goals.toString()}
              onChange={count(index, "goals")}
            />
            <LabeledInput
              label={t("playerProfile.assists")}
              type="number"
              value={entry.assists.toString()}
              onChange={count(index, "assists")}
            />
          </div>
          <button
            type="button"
            aria-label={t("worldEditor.removeCareerEntry")}
            onClick={() => onChange(entries.filter((_, i) => i !== index))}
            className="self-end p-1.5 rounded-lg border border-gray-200 dark:border-navy-600 text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition focus:outline-none focus:ring-2 focus:ring-primary-400"
          >
            <X className="w-3.5 h-3.5" aria-hidden="true" />
          </button>
        </div>
      ))}
      <button
        type="button"
        onClick={() => onChange([...entries, emptyCareerEntry()])}
        className="self-start inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-heading font-bold uppercase tracking-wide rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-navy-600 transition focus:outline-none focus:ring-2 focus:ring-primary-400"
      >
        <Plus className="w-3.5 h-3.5" aria-hidden="true" />
        {t("worldEditor.addCareerEntry")}
      </button>
    </div>
  );
}
