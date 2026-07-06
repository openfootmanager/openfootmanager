import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import type { TacticsPhaseSettings } from "../../store/types";
import { Select } from "../ui";

const WITH_BALL_FIELDS = [
  ["build_up_style", "buildUpStyle", ["Short", "Mixed", "Long"]] as const,
  ["width", "width", ["Narrow", "Normal", "Wide"]] as const,
  ["tempo", "tempo", ["Patient", "Direct"]] as const,
];

const WITHOUT_BALL_FIELDS = [
  ["defensive_line", "defensiveLine", ["VeryLow", "Low", "Medium", "High"]] as const,
  ["pressing_intensity", "pressingIntensity", ["Passive", "Medium", "Aggressive"]] as const,
  ["defensive_shape", "defensiveShape", ["Stretched", "Normal", "Compact"]] as const,
  ["marking_style", "markingStyle", ["Zonal", "Mixed", "ManToMan"]] as const,
];

const TRANSITION_FIELDS = [
  ["counter_press_duration", "counterPressDuration", ["None", "Short", "Long"]] as const,
  ["break_speed", "breakSpeed", ["Slow", "Medium", "Fast"]] as const,
];

const SECTIONS = [
  ["withBall", WITH_BALL_FIELDS] as const,
  ["withoutBall", WITHOUT_BALL_FIELDS] as const,
  ["transitions", TRANSITION_FIELDS] as const,
];

function PhaseButtonGroup({
  field,
  labelKey,
  onTacticsPhaseChange,
  options,
  tacticsPhase,
}: {
  field: keyof TacticsPhaseSettings;
  labelKey: string;
  onTacticsPhaseChange: (patch: Partial<TacticsPhaseSettings>) => void;
  options: readonly string[];
  tacticsPhase?: TacticsPhaseSettings;
}): JSX.Element {
  const { t } = useTranslation();
  const currentValue = (tacticsPhase?.[field] ?? options[0]) as string;
  return (
    <div className="flex items-center gap-2">
      <span className="w-20 shrink-0 text-[11px] text-gray-500 dark:text-gray-400">
        {t(`tactics.phaseSettings.${labelKey}`)}
      </span>
      <Select
        selectSize="sm"
        variant="subtle"
        fullWidth
        value={currentValue}
        onChange={(e) => {
          onTacticsPhaseChange({ [field]: e.target.value });
        }}
      >
        {options.map((opt) => (
          <option key={opt} value={opt}>
            {t(`tactics.phaseSettings.${labelKey}_${opt}`, opt)}
          </option>
        ))}
      </Select>
    </div>
  );
}

/**
 * The Phase Blueprint editor (with-ball / without-ball / transitions tactical
 * settings). Shared by the Tactics board's right panel and the pre-match screen
 * so both edit the same `team.tactics_phase`. Renders the section body only —
 * the caller provides the surrounding card/header.
 */
export function PhaseBlueprintPanel({
  tacticsPhase,
  onTacticsPhaseChange,
}: {
  tacticsPhase?: TacticsPhaseSettings;
  onTacticsPhaseChange: (patch: Partial<TacticsPhaseSettings>) => void;
}): JSX.Element {
  const { t } = useTranslation();
  return (
    <div className="divide-y divide-gray-100 dark:divide-navy-700">
      {SECTIONS.map(([labelKey, fields]) => (
        <div key={labelKey} className="p-3 space-y-2">
          <div className="mb-1.5 text-[11px] font-heading font-bold uppercase tracking-[0.2em] text-primary-500 dark:text-primary-400">
            {t(`tactics.phaseLabels.${labelKey}`)}
          </div>
          {fields.map(([field, fieldLabelKey, options]) => (
            <PhaseButtonGroup
              key={field}
              field={field}
              labelKey={fieldLabelKey}
              onTacticsPhaseChange={onTacticsPhaseChange}
              options={options}
              tacticsPhase={tacticsPhase}
            />
          ))}
        </div>
      ))}
    </div>
  );
}
