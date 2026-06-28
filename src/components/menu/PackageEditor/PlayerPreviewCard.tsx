import { User } from "lucide-react";
import { useTranslation } from "react-i18next";
import { GeneratedAvatar } from "../../ui/GeneratedAvatar";
import { GeneratedCrest } from "../../ui/GeneratedCrest";
import type { PlayerAttributesDef, PlayerDef, Position, TeamDef } from "./types";

const POSITION_COLOR: Record<Position, string> = {
  Goalkeeper: "bg-amber-500",
  Defender: "bg-blue-600",
  CenterBack: "bg-blue-600",
  RightBack: "bg-blue-600",
  LeftBack: "bg-blue-600",
  RightWingBack: "bg-blue-500",
  LeftWingBack: "bg-blue-500",
  DefensiveMidfielder: "bg-green-700",
  CentralMidfielder: "bg-green-600",
  Midfielder: "bg-green-600",
  AttackingMidfielder: "bg-green-500",
  RightMidfielder: "bg-green-600",
  LeftMidfielder: "bg-green-600",
  RightWinger: "bg-red-500",
  LeftWinger: "bg-red-500",
  Forward: "bg-red-600",
  Striker: "bg-red-600",
};

const ALL_ATTR_GROUPS = [
  { groupKey: "physical",    keys: ["pace", "stamina", "strength", "agility"] },
  { groupKey: "technical",   keys: ["shooting", "passing", "dribbling", "tackling", "defending"] },
  { groupKey: "mental",      keys: ["positioning", "vision", "decisions", "composure", "aggression", "teamwork", "leadership"] },
  { groupKey: "goalkeeper",  keys: ["handling", "reflexes", "aerial"] },
] as const;

function attrColor(val: number): string {
  if (val >= 80) return "bg-success-500";
  if (val >= 65) return "bg-primary-500";
  if (val >= 50) return "bg-accent-500";
  if (val >= 35) return "bg-yellow-500";
  return "bg-red-500";
}

function estimateAttributesFromOvr(overall: number, position: Position): PlayerAttributesDef {
  const b = Math.max(30, Math.min(97, overall));
  const isGK = position === "Goalkeeper";
  const isDef = ["Defender", "CenterBack", "RightBack", "LeftBack", "RightWingBack", "LeftWingBack"].includes(position);
  const isFwd = ["Forward", "Striker", "RightWinger", "LeftWinger"].includes(position);
  return {
    pace: b,
    stamina: b,
    strength: b,
    agility: b,
    passing: b,
    shooting: isGK ? 30 : b,
    tackling: (isGK || isFwd) ? Math.max(20, b - 15) : b,
    dribbling: isGK ? 30 : b,
    defending: isGK ? 35 : isDef ? Math.min(97, b + 3) : b,
    positioning: b,
    vision: b,
    decisions: b,
    composure: b,
    aggression: Math.max(30, b - 10),
    teamwork: b,
    leadership: Math.max(25, b - 10),
    handling: isGK ? b : 20,
    reflexes: isGK ? b : 30,
    aerial: isGK ? b : isDef ? b : Math.max(30, b - 10),
  };
}

function calcAge(dob: string | null): number | null {
  if (!dob) return null;
  const ms = Date.now() - new Date(dob).getTime();
  const age = Math.floor(ms / (365.25 * 24 * 60 * 60 * 1000));
  return isNaN(age) || age < 0 || age > 80 ? null : age;
}

interface PlayerPreviewCardProps {
  editing: PlayerDef;
  photoDataUrl: string | null;
  teams?: TeamDef[];
}

export function PlayerPreviewCard({ editing, photoDataUrl, teams }: PlayerPreviewCardProps) {
  const { t } = useTranslation();

  const displayName =
    editing.name ||
    [editing.firstName, editing.lastName].filter(Boolean).join(" ") ||
    null;

  const abbr = t(`common.posAbbr.${editing.position}`, { defaultValue: editing.position.slice(0, 2).toUpperCase() });
  const posColor = POSITION_COLOR[editing.position] ?? "bg-gray-500";

  const age = calcAge(editing.dateOfBirth);
  const initials = displayName ? displayName.slice(0, 2).toUpperCase() : "?";

  const club = teams?.find((t) => t.id === editing.club);
  const clubName = club?.name ?? editing.club;

  const displayAttrs = editing.attributes
    ?? (editing.overall !== null ? estimateAttributesFromOvr(editing.overall, editing.position) : null);
  const isEstimated = !editing.attributes && editing.overall !== null;

  return (
    <div className="rounded-2xl border border-gray-200 dark:border-navy-600 overflow-hidden bg-white dark:bg-navy-700 shadow-sm select-none">
      {/* Header */}
      <div className="bg-navy-800 px-4 pt-4 pb-3 flex flex-col items-center gap-2">
        {/* Photo or avatar */}
        {photoDataUrl ? (
          <img
            src={photoDataUrl}
            alt=""
            className="w-16 h-16 rounded-full object-cover border-2 border-white/20"
          />
        ) : displayName ? (
          <GeneratedAvatar
            name={displayName}
            initials={initials}
            className="w-16 h-16"
          />
        ) : (
          <div className="w-16 h-16 rounded-full bg-navy-600 flex items-center justify-center">
            <User className="w-8 h-8 text-gray-500" />
          </div>
        )}

        <div className="text-center">
          <p className="font-heading font-bold text-white text-sm leading-tight">
            {displayName ?? <span className="italic text-gray-400">New Player</span>}
          </p>
          <span
            className={`inline-block mt-1 px-2 py-0.5 rounded text-[10px] font-bold uppercase text-white ${posColor}`}
          >
            {abbr}
          </span>
        </div>
      </div>

      <div className="p-3 flex flex-col gap-2.5">
        {/* Overall rating badge */}
        {editing.overall !== null && (
          <div className="flex items-baseline gap-1.5">
            <span className="text-[10px] uppercase tracking-wide text-gray-400">OVR</span>
            <span
              className="text-2xl font-heading font-black leading-none"
              style={{
                color: editing.overall >= 80
                  ? "var(--color-success-500, #22c55e)"
                  : editing.overall >= 65
                    ? "var(--color-primary-500, #3b82f6)"
                    : "#9ca3af",
              }}
            >
              {editing.overall}
            </span>
            {isEstimated && (
              <span className="text-[10px] text-gray-400 italic">est.</span>
            )}
          </div>
        )}

        {/* Full attribute breakdown */}
        {displayAttrs && (
          <div className="flex flex-col gap-2">
            {ALL_ATTR_GROUPS.map(({ groupKey, keys }) => {
              const anySet = keys.some((k) => displayAttrs[k as keyof typeof displayAttrs] != null);
              if (!anySet) return null;
              return (
                <div key={groupKey}>
                  <p className="text-[10px] font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500 mb-1">
                    {t(`common.attrGroups.${groupKey}`)}
                  </p>
                  <div className="grid grid-cols-2 gap-x-3 gap-y-0.5">
                    {keys.map((key) => {
                      const val = displayAttrs[key as keyof typeof displayAttrs] as number | undefined;
                      if (val == null) return null;
                      const label = t(`common.attributes.${key}`).slice(0, 3).toUpperCase();
                      return (
                        <div key={key} className="flex items-center gap-1">
                          <span className="w-7 text-[10px] font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500 flex-shrink-0">
                            {label}
                          </span>
                          <div className="flex-1 h-1 bg-gray-100 dark:bg-navy-600 rounded-full overflow-hidden">
                            <div
                              className={`h-full rounded-full ${attrColor(val)}`}
                              style={{ width: `${(val / 99) * 100}%` }}
                            />
                          </div>
                          <span className="w-5 text-right text-[10px] font-bold tabular-nums text-gray-700 dark:text-gray-200 flex-shrink-0">
                            {val}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* Bio info */}
        <div className="flex flex-col gap-1 text-xs text-gray-500 dark:text-gray-400 pt-1 border-t border-gray-100 dark:border-navy-600">
          {editing.nationality && (
            <p>{t("worldEditor.playerNationality")}: <span className="text-gray-700 dark:text-gray-200">{editing.nationality}</span></p>
          )}
          {editing.club && (
            <div className="flex items-center gap-1.5">
              <span className="shrink-0">{t("worldEditor.playerClub")}:</span>
              {club ? (
                <div className="flex items-center gap-1 min-w-0">
                  <GeneratedCrest
                    name={club.name || club.id}
                    label={club.shortName || club.name?.slice(0, 3) || "?"}
                    colors={club.colors}
                    className="w-4 h-4 flex-shrink-0"
                  />
                  <span className="text-gray-700 dark:text-gray-200 truncate">{clubName}</span>
                </div>
              ) : (
                <span className="text-gray-700 dark:text-gray-200">{editing.club}</span>
              )}
            </div>
          )}
          {editing.dateOfBirth && (
            <p>
              {t("worldEditor.playerDateOfBirth")}: <span className="text-gray-700 dark:text-gray-200">{editing.dateOfBirth}</span>
              {age !== null && <span className="text-gray-400 dark:text-gray-500"> ({age}y)</span>}
            </p>
          )}
          {editing.footedness && editing.footedness !== "Right" && (
            <p>{t("worldEditor.playerFoot")}: <span className="text-gray-700 dark:text-gray-200">{editing.footedness}</span></p>
          )}
        </div>
      </div>
    </div>
  );
}
