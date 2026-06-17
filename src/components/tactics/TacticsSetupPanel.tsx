import {
  ArrowRightLeft,
  Crosshair,
  Flag,
  RefreshCw,
  Shield,
  Target,
  Zap,
} from "lucide-react";
import type { JSX, ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Badge, Card } from "../ui";
import {
  FORMATIONS,
  PLAY_STYLE_DESCRIPTION_FALLBACKS,
  type TacticsPresetDefinition,
} from "./TacticsTab.helpers";

interface PlayStyleOption {
  icon: ReactNode;
  id: string;
}

interface TacticsSetupPanelProps {
  activePlayStyle: string;
  activePreset: TacticsPresetDefinition | null;
  activeTacticName: string;
  isPresetDirty: boolean;
  formation: string;
  onFormationChange: (formation: string) => void;
  onPlayStyleChange: (playStyle: string) => void;
}

const PLAY_STYLES: PlayStyleOption[] = [
  { id: "Balanced", icon: <Target className="h-3.5 w-3.5" /> },
  { id: "Attacking", icon: <Zap className="h-3.5 w-3.5" /> },
  { id: "Defensive", icon: <Shield className="h-3.5 w-3.5" /> },
  { id: "Possession", icon: <RefreshCw className="h-3.5 w-3.5" /> },
  { id: "Counter", icon: <Crosshair className="h-3.5 w-3.5" /> },
  { id: "HighPress", icon: <Flag className="h-3.5 w-3.5" /> },
];

function getOptionButtonClassName(isActive: boolean): string {
  if (isActive) {
    return "rounded-lg bg-primary-500 px-3 py-2 text-sm font-heading font-bold text-white shadow-sm transition-all";
  }

  return "rounded-lg bg-gray-100 px-3 py-2 text-sm font-heading font-bold text-gray-500 transition-all hover:bg-gray-200 dark:bg-navy-700 dark:text-gray-400 dark:hover:bg-navy-600";
}

function getPlayStyleDescription(activePlayStyle: string): string {
  return PLAY_STYLE_DESCRIPTION_FALLBACKS[activePlayStyle] ?? "";
}

export default function TacticsSetupPanel({
  activePlayStyle,
  activePreset,
  activeTacticName,
  isPresetDirty,
  formation,
  onFormationChange,
  onPlayStyleChange,
}: TacticsSetupPanelProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <div className="p-4">
          <div className="mb-3 flex items-start justify-between gap-3">
            <div>
              <h3 className="text-sm font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                {t("tactics.currentPlanCard")}
              </h3>
              <div className="mt-1 text-base font-heading font-bold text-gray-900 dark:text-gray-100">
                {activeTacticName}
              </div>
            </div>
            <Badge variant={isPresetDirty ? "accent" : "success"} size="sm">
              {isPresetDirty ? t("tactics.unsavedChanges") : t("tactics.synced")}
            </Badge>
          </div>
          <div className="rounded-2xl border border-gray-200 bg-gray-50 p-3 dark:border-navy-600 dark:bg-navy-800/70">
            <div className="flex flex-wrap items-center gap-2 text-[11px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
              <span>{formation}</span>
              <span aria-hidden="true">·</span>
              <span>{t(`common.playStyles.${activePlayStyle}`, activePlayStyle)}</span>
              {activePreset ? (
                <>
                  <span aria-hidden="true">·</span>
                  <span>
                    {t(`tactics.presetNames.${activePreset.id}`, activePreset.id)}
                  </span>
                </>
              ) : null}
            </div>
            <p className="mt-2 text-sm leading-relaxed text-gray-600 dark:text-gray-300">
              {activePreset
                ? t(activePreset.descriptionKey)
                : t("tactics.customTacticDescription")}
            </p>
          </div>
        </div>
      </Card>

      <Card>
        <div className="p-4">
          <h3 className="mb-3 text-sm font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
            {t("tactics.formation")}
          </h3>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 xl:grid-cols-2">
            {FORMATIONS.map((nextFormation) => (
              <button
                key={nextFormation}
                onClick={() => onFormationChange(nextFormation)}
                className={getOptionButtonClassName(
                  formation === nextFormation,
                )}
              >
                {nextFormation}
              </button>
            ))}
          </div>
        </div>
      </Card>

      <Card>
        <div className="p-4">
          <h3 className="mb-3 text-sm font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
            {t("tactics.teamIdentity")}
          </h3>
          <div className="grid grid-cols-3 justify-center gap-2">
            {PLAY_STYLES.map((style) => (
              <button
                key={style.id}
                onClick={() => onPlayStyleChange(style.id)}
                className={`flex items-center gap-1.5 ${getOptionButtonClassName(
                  activePlayStyle === style.id,
                )}`}
              >
                {style.icon}
                {t(`common.playStyles.${style.id}`, style.id)}
              </button>
            ))}
          </div>
          <div className="mt-3 rounded-xl border border-gray-200 bg-gray-50 px-3 py-3 dark:border-navy-600 dark:bg-navy-800/70">
            <div className="mb-1 text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
              {t("squad.playStyleImpactTitle")}
            </div>
            <p className="text-sm leading-relaxed text-gray-600 dark:text-gray-300">
              {t(
                `squad.playStyleDescriptions.${activePlayStyle}`,
                getPlayStyleDescription(activePlayStyle),
              )}
            </p>
          </div>
        </div>
      </Card>

      <Card>
        <div className="p-4">
          <h3 className="mb-3 text-sm font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
            {t("tactics.phaseBlueprint")}
          </h3>
          <div className="space-y-2">
            <div className="rounded-xl border border-gray-200 bg-gray-50 px-3 py-3 dark:border-navy-600 dark:bg-navy-800/70">
              <div className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {t("tactics.phaseLabels.withBall")}
              </div>
              <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">
                {t("tactics.phaseSummaries.withBall")}
              </p>
            </div>
            <div className="rounded-xl border border-gray-200 bg-gray-50 px-3 py-3 dark:border-navy-600 dark:bg-navy-800/70">
              <div className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {t("tactics.phaseLabels.withoutBall")}
              </div>
              <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">
                {t("tactics.phaseSummaries.withoutBall")}
              </p>
            </div>
            <div className="rounded-xl border border-dashed border-gray-200 px-3 py-3 dark:border-navy-600">
              <div className="flex items-center gap-2 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                <ArrowRightLeft className="h-3.5 w-3.5 text-accent-400" />
                {t("tactics.phaseLabels.transitions")}
              </div>
              <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">
                {t("tactics.phaseSummaries.transitions")}
              </p>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}
