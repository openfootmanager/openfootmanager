import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PlayerData } from "../../store/gameStore";
import { getAttributeValueClassName } from "../../lib/playerAttributeDisplay";
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
                <span className={getAttributeValueClassName(s.value)}>
                  {s.value}
                </span>
              </span>
            ))}
          </div>
        )}
         <ArrowUpDown className="w-4 h-4 text-gray-500 dark:text-gray-400" />
      </button>

      {expanded && (
        <div className="mt-1 bg-white dark:bg-navy-700 rounded-lg border border-gray-200 dark:border-navy-600 p-2 flex flex-col gap-0.5 max-h-56 overflow-auto">
          {sortedPlayers.map((p) => {
            const isCurrent = p.id === currentId;
            return (
              <button
                key={p.id}
                onClick={() => {
                  onSelect(p.id);
                  setExpanded(false);
                }}
                className={`flex items-center gap-2 px-2 py-1.5 rounded text-left transition-colors ${
                  isCurrent
                    ? "bg-primary-500/20 text-primary-500 dark:text-primary-400"
                    : "hover:bg-gray-100 dark:hover:bg-navy-600 text-gray-700 dark:text-gray-300"
                }`}
              >
                <span className="w-3 shrink-0">
                  {isCurrent && <Check className="w-3 h-3 text-primary-400" />}
                </span>
                <span className="min-w-0 flex-1 truncate text-sm font-medium">
                  {p.name}
                </span>
                <Badge variant="neutral" size="sm">
                  {getTranslatedPositionAbbreviation(p.position)}
                </Badge>
                <span
                  className={`w-7 shrink-0 text-right text-xs font-heading font-bold tabular-nums ${getAttributeValueClassName(p.spStats.score)}`}
                >
                  {p.spStats.score}
                </span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
