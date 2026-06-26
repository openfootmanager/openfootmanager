import { useTranslation } from "react-i18next";
import { GeneratedCrest } from "../../ui/GeneratedCrest";
import JerseyIcon from "../../ui/JerseyIcon";
import type { KitPattern, TeamDef } from "./types";

const REP_TIERS = [
  { min: 850, label: "Elite", color: "text-amber-500" },
  { min: 720, label: "Top", color: "text-primary-500" },
  { min: 550, label: "Mid", color: "text-success-500" },
  { min: 300, label: "Lower", color: "text-gray-500" },
  { min: 0,   label: "Amateur", color: "text-gray-400" },
];

function repTier(rep: number) {
  return REP_TIERS.find((t) => rep >= t.min) ?? REP_TIERS[REP_TIERS.length - 1];
}

interface TeamPreviewCardProps {
  team: TeamDef;
  logoDataUrl: string | null;
}

export function TeamPreviewCard({ team, logoDataUrl }: TeamPreviewCardProps) {
  const { t } = useTranslation();
  const repMid = team.reputationRange
    ? Math.round((team.reputationRange[0] + team.reputationRange[1]) / 2)
    : null;
  const tier = repMid !== null ? repTier(repMid) : null;

  const primaryColor = team.colors.primary || "#1e3a5f";
  const secondaryColor = team.colors.secondary || "#ffffff";

  function formatBudget(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
    return String(n);
  }

  return (
    <div className="rounded-2xl border border-gray-200 dark:border-navy-600 overflow-hidden bg-white dark:bg-navy-700 shadow-sm select-none">
      {/* Colour banner */}
      <div
        className="h-20 flex items-center justify-center"
        style={{ background: `linear-gradient(135deg, ${primaryColor} 40%, ${secondaryColor})` }}
      >
        {logoDataUrl ? (
          <img
            src={logoDataUrl}
            alt=""
            className="w-14 h-14 object-contain drop-shadow-md"
          />
        ) : (
          <GeneratedCrest
            name={team.name || "?"}
            label={team.shortName || team.name?.slice(0, 3) || "?"}
            colors={team.colors}
            className="w-14 h-14"
          />
        )}
      </div>

      <div className="p-3 flex flex-col gap-2.5">
        {/* Name */}
        <div>
          <p className="font-heading font-bold text-sm uppercase tracking-wide text-gray-900 dark:text-white leading-tight">
            {team.name || <span className="text-gray-400 italic">New Team</span>}
          </p>
          {team.shortName && (
            <p className="text-[10px] text-gray-400 dark:text-gray-500 font-mono mt-0.5">
              {team.shortName}
            </p>
          )}
        </div>

        {/* Colors + Jersey */}
        <div className="flex items-center gap-2">
          <JerseyIcon
            primaryColor={primaryColor}
            secondaryColor={secondaryColor}
            pattern={(team.kitPattern ?? "Solid") as KitPattern}
            size="md"
          />
          <div className="flex flex-col gap-1">
            <div className="flex items-center gap-1">
              <div
                className="w-4 h-4 rounded border border-gray-200 dark:border-navy-600 flex-shrink-0"
                style={{ background: primaryColor }}
                title={t("worldEditor.teamPrimaryColor")}
              />
              <div
                className="w-4 h-4 rounded border border-gray-200 dark:border-navy-600 flex-shrink-0"
                style={{ background: secondaryColor }}
                title={t("worldEditor.teamSecondaryColor")}
              />
            </div>
            <span className="text-[10px] text-gray-400 dark:text-gray-500 font-mono">
              {primaryColor}
            </span>
            <span className="text-[10px] text-gray-400 dark:text-gray-500 font-mono">
              {secondaryColor}
            </span>
          </div>
        </div>

        {/* Location */}
        {(team.city || team.country) && (
          <p className="text-xs text-gray-500 dark:text-gray-400">
            {[team.city, team.country].filter(Boolean).join(", ")}
          </p>
        )}

        {/* Play style */}
        {team.playStyle && (
          <p className="text-[11px] font-heading font-bold uppercase tracking-wide text-gray-400 dark:text-gray-500">
            {t(`common.playStyles.${team.playStyle}`, { defaultValue: team.playStyle })}
          </p>
        )}

        {/* Reputation */}
        {team.reputationRange && (
          <div className="flex flex-col gap-1">
            <div className="flex items-center justify-between text-[11px]">
              <span className="text-gray-400 uppercase tracking-wide">
                {t("worldEditor.teamRepMin").replace(" Reputation", "").replace(" Rep", "")} Rep
              </span>
              {tier && (
                <span className={`font-bold uppercase ${tier.color}`}>{tier.label}</span>
              )}
            </div>
            <div className="flex items-center gap-1 text-[11px] text-gray-500">
              <span>{team.reputationRange[0]}</span>
              <div className="flex-1 h-1.5 bg-gray-100 dark:bg-navy-600 rounded-full overflow-hidden mx-1">
                <div
                  className="h-full bg-primary-500 rounded-full"
                  style={{ width: `${(team.reputationRange[1] / 950) * 100}%` }}
                />
              </div>
              <span>{team.reputationRange[1]}</span>
            </div>
          </div>
        )}

        {/* Budget */}
        {team.financeRange && (
          <div className="text-[11px] text-gray-400">
            <span className="uppercase tracking-wide">{t("worldEditor.teamBudget")} </span>
            <span className="text-gray-600 dark:text-gray-300 font-mono">
              {formatBudget(team.financeRange[0])}–{formatBudget(team.financeRange[1])}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
