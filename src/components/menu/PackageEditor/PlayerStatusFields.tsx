import { useId } from "react";
import { useTranslation } from "react-i18next";
import { POSITIONS, WIRE_MAX, parseOptionalWhole } from "./helpers";
import { InlineHelp, LabeledInput, labelClass } from "./primitives";
import type { PlayerDef, Position } from "./types";

interface PlayerStatusFieldsProps {
  editing: PlayerDef;
  updateField: <K extends keyof PlayerDef>(key: K, value: PlayerDef[K]) => void;
}

const chipClass =
  "px-2.5 py-1 text-[11px] font-heading font-bold uppercase tracking-wide rounded-full border transition focus:outline-none focus:ring-2 focus:ring-primary-400";
const chipOnClass = "bg-primary-500 text-white border-primary-500";
const chipOffClass =
  "bg-white dark:bg-navy-700 text-gray-600 dark:text-gray-300 border-gray-200 dark:border-navy-600 hover:bg-gray-50 dark:hover:bg-navy-600";

/**
 * Condition, morale, weak foot and alternate positions.
 *
 * As with the contract, the ranges belong to the backend validator. The one rule
 * the backend applies to alternates that matters here is that a general group
 * (Midfielder, Forward, …) cannot carry a weak foot or alternates, and it says so
 * when a package is checked, rather than this form hiding the positions and
 * keeping a second list of which ones those are.
 */
export function PlayerStatusFields({ editing, updateField }: PlayerStatusFieldsProps) {
  const { t } = useTranslation();
  const alternatesLabelId = useId();
  const alternates = editing.alternatePositions ?? [];

  function toggle(position: Position) {
    // A new array each time: mutating the one the store handed over desyncs the UI.
    updateField(
      "alternatePositions",
      alternates.includes(position)
        ? alternates.filter((chosen) => chosen !== position)
        : [...alternates, position],
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <p className={labelClass}>{t("worldEditor.playerStatusSection")}</p>
      <div className="grid grid-cols-3 gap-3">
        <LabeledInput
          label={t("common.condition")}
          value={editing.condition?.toString() ?? ""}
          type="number"
          onChange={(v) => updateField("condition", parseOptionalWhole(v, WIRE_MAX.u8))}
        />
        <LabeledInput
          label={t("common.morale")}
          value={editing.morale?.toString() ?? ""}
          type="number"
          onChange={(v) => updateField("morale", parseOptionalWhole(v, WIRE_MAX.u8))}
        />
        <LabeledInput
          label={t("common.weakFoot")}
          value={editing.weakFoot?.toString() ?? ""}
          type="number"
          onChange={(v) => updateField("weakFoot", parseOptionalWhole(v, WIRE_MAX.u8))}
        />
      </div>
      <div role="group" aria-labelledby={alternatesLabelId} className="flex flex-col gap-1.5">
        <div className="flex items-center gap-1.5">
          <p id={alternatesLabelId} className={labelClass}>
            {t("worldEditor.playerAlternatePositions")}
          </p>
          <InlineHelp text={t("worldEditor.playerAlternatePositionsHelp")} />
        </div>
        <div className="flex flex-wrap gap-1.5">
          {POSITIONS.map((position) => {
            const pressed = alternates.includes(position);
            return (
              <button
                key={position}
                type="button"
                aria-pressed={pressed}
                onClick={() => toggle(position)}
                className={`${chipClass} ${pressed ? chipOnClass : chipOffClass}`}
              >
                {t(`common.positions.${position}`)}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
