import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PlayerData } from "../../store/gameStore";
import { normalisePosition } from "../squad/SquadTab.helpers";
import { Badge } from "../ui";
import { ArrowUpDown, Check } from "lucide-react";

function getStatAttributeKey(label: string): string | null {
  switch (label) {
    case "SHO":
      return "shooting";
    case "COM":
      return "composure";
    case "PAS":
      return "passing";
    case "VIS":
      return "vision";
    case "LDR":
      return "leadership";
    case "TMW":
      return "teamwork";
    default:
      return null;
  }
}

function getStatColorClassName(value: number): string {
  if (value >= 70) {
    return "text-primary-300";
  }

  if (value >= 50) {
    return "text-gray-100";
  }

  return "text-gray-400";
}

export function getSetPieceStats(
  role: string,
  p: PlayerData,
): { score: number; stats: { label: string; value: number }[] } {
  const a = p.attributes;
  switch (role) {
    case "penalty":
      return {
        score: Math.round((a.shooting + a.composure) / 2),
        stats: [
          { label: "SHO", value: a.shooting },
          { label: "COM", value: a.composure },
        ],
      };
    case "freekick":
      return {
        score: Math.round((a.passing + a.vision + a.shooting / 2) / 2.5),
        stats: [
          { label: "PAS", value: a.passing },
          { label: "VIS", value: a.vision },
          { label: "SHO", value: a.shooting },
        ],
      };
    case "corner":
      return {
        score: Math.round((a.passing + a.vision) / 2),
        stats: [
          { label: "PAS", value: a.passing },
          { label: "VIS", value: a.vision },
        ],
      };
    case "captain":
    case "vicecaptain":
      return {
        score: Math.round((a.leadership + a.teamwork) / 2),
        stats: [
          { label: "LDR", value: a.leadership },
          { label: "TMW", value: a.teamwork },
        ],
      };
    default:
      return { score: 0, stats: [] };
  }
}

function roleAllowsGoalkeeper(role: string): boolean {
  return role === "captain" || role === "vicecaptain";
}

export default function SetPieceSelector({
  label,
  icon,
  role,
  currentId,
  players,
  allSquad,
  onSelect,
}: {
  label: string;
  icon: React.ReactNode;
  role: string;
  currentId: string | null;
  players: { id: string; name: string; position: string }[];
  allSquad: PlayerData[];
  onSelect: (id: string) => void;
}) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const currentPlayer = players.find((p) => p.id === currentId);
  const currentSquad = allSquad.find((sp) => sp.id === currentId);
  const currentStats = currentSquad
    ? getSetPieceStats(role, currentSquad)
    : null;

  const sortedPlayers = [...players]
    .filter((p) => roleAllowsGoalkeeper(role) || p.position !== "Goalkeeper")
    .map((p) => {
      const squad = allSquad.find((sp) => sp.id === p.id);
      const spStats = squad
        ? getSetPieceStats(role, squad)
        : { score: 0, stats: [] };
      return { ...p, squad, spStats };
    })
    .sort(
      (a, b) =>
        b.spStats.score - a.spStats.score || a.name.localeCompare(b.name),
    );

  function getTranslatedStatLabel(label: string): string {
    const attributeKey = getStatAttributeKey(label);

    if (!attributeKey) {
      return label;
    }

    return t(`common.attributes.${attributeKey}`, { defaultValue: label });
  }

  function getTranslatedPositionAbbreviation(position: string): string {
    const normalizedPosition = normalisePosition(position);

    return t(`common.posAbbr.${normalizedPosition}`, {
      defaultValue: normalizedPosition.substring(0, 3).toUpperCase(),
    });
  }

  return (
    <div className="mb-4 last:mb-0">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-3 p-3 rounded-lg bg-gray-100 hover:bg-gray-200 dark:bg-navy-700/50 dark:hover:bg-navy-700 transition-colors"
      >
        {icon}
        <div className="flex-1 text-left">
            <p className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
            {label}
          </p>
            <p className="text-sm text-gray-800 dark:text-gray-200 font-medium">
            {currentPlayer ? currentPlayer.name : t("match.notAssigned")}
          </p>
        </div>
        {currentStats && (
          <div className="hidden flex-wrap items-center justify-end gap-2 md:flex">
            {currentStats.stats.map((s) => (
              <span
                key={s.label}
                title={getTranslatedStatLabel(s.label)}
                 className="inline-flex items-center gap-1 rounded-md border border-gray-200 dark:border-white/10 bg-white dark:bg-navy-800 px-2 py-1 text-xs font-heading font-bold text-gray-800 dark:text-gray-100"
              >
                 <span className="text-gray-600 dark:text-gray-300">
                  {getTranslatedStatLabel(s.label)}
                </span>
                <span className={getStatColorClassName(s.value)}>
                  {s.value}
                </span>
              </span>
            ))}
          </div>
        )}
         <ArrowUpDown className="w-4 h-4 text-gray-500 dark:text-gray-400" />
      </button>

      {expanded && (
         <div className="mt-1 bg-white dark:bg-navy-700 rounded-lg border border-gray-200 dark:border-navy-600 p-2 flex flex-col gap-1 max-h-56 overflow-auto transition-colors duration-300">
          {sortedPlayers.map((p) => {
            const isCurrent = p.id === currentId;
            return (
              <button
                key={p.id}
                onClick={() => {
                  onSelect(p.id);
                  setExpanded(false);
                }}
                className={`flex items-center gap-2 px-3 py-1.5 rounded text-left transition-colors ${
                  isCurrent
                     ? "bg-primary-500/20 text-primary-500 dark:text-primary-400"
                     : "hover:bg-gray-100 dark:hover:bg-navy-600 text-gray-700 dark:text-gray-300"
                }`}
              >
                {isCurrent && <Check className="w-3 h-3 text-primary-400" />}
                <span className="text-sm font-medium flex-1 truncate">
                  {p.name}
                </span>
                <Badge variant="neutral" size="sm">
                  {getTranslatedPositionAbbreviation(p.position)}
                </Badge>
                {p.spStats.stats.map((s) => (
                  <span
                    key={s.label}
                    title={getTranslatedStatLabel(s.label)}
                     className="w-10 rounded-md bg-gray-100 dark:bg-navy-800/80 px-1.5 py-1 text-center text-xs font-heading font-bold transition-colors duration-300"
                  >
                    <span className={getStatColorClassName(s.value)}>
                      {s.value}
                    </span>
                  </span>
                ))}
                <span
                  className={`text-xs font-heading font-bold w-8 text-right ${
                    p.spStats.score >= 70
                      ? "text-primary-300"
                      : p.spStats.score >= 50
                        ? "text-gray-100"
                        : "text-gray-400"
                  }`}
                >
                  {p.spStats.score}
                </span>
              </button>
            );
          })}
          {/* Column headers */}
          {sortedPlayers.length > 0 && (
             <div className="mt-1 flex items-center gap-2 border-t border-gray-200 dark:border-navy-600 px-3 py-2 text-xs font-heading font-bold text-gray-600 dark:text-gray-300">
              <span className="flex-1" />
              <span className="w-8" />
              {sortedPlayers[0].spStats.stats.map((s) => (
                <span
                  key={s.label}
                  title={getTranslatedStatLabel(s.label)}
                  className="w-10 truncate text-center"
                >
                  {getTranslatedStatLabel(s.label)}
                </span>
              ))}
              <span className="w-8 text-right">{t("preMatch.fit")}</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
