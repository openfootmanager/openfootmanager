import { User } from "lucide-react";
import { useTranslation } from "react-i18next";
import { GeneratedAvatar } from "../../ui/GeneratedAvatar";
import type { PlayerDef, Position } from "./types";

const POSITION_ABBR: Record<Position, string> = {
  Goalkeeper: "GK",
  Defender: "DEF",
  Midfielder: "MID",
  Forward: "FWD",
  RightBack: "RB",
  CenterBack: "CB",
  LeftBack: "LB",
  RightWingBack: "RWB",
  LeftWingBack: "LWB",
  DefensiveMidfielder: "CDM",
  CentralMidfielder: "CM",
  AttackingMidfielder: "CAM",
  RightMidfielder: "RM",
  LeftMidfielder: "LM",
  RightWinger: "RW",
  LeftWinger: "LW",
  Striker: "ST",
};

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

const KEY_ATTRS = [
  { key: "pace",      abbr: "PAC" },
  { key: "shooting",  abbr: "SHO" },
  { key: "passing",   abbr: "PAS" },
  { key: "dribbling", abbr: "DRI" },
  { key: "defending", abbr: "DEF" },
  { key: "strength",  abbr: "PHY" },
] as const;

function attrColor(val: number): string {
  if (val >= 80) return "bg-success-500";
  if (val >= 65) return "bg-primary-500";
  if (val >= 50) return "bg-accent-500";
  if (val >= 35) return "bg-yellow-500";
  return "bg-red-500";
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
}

export function PlayerPreviewCard({ editing, photoDataUrl }: PlayerPreviewCardProps) {
  const { t } = useTranslation();

  const displayName =
    editing.name ||
    [editing.firstName, editing.lastName].filter(Boolean).join(" ") ||
    null;

  const abbr = POSITION_ABBR[editing.position] ?? editing.position;
  const posColor = POSITION_COLOR[editing.position] ?? "bg-gray-500";

  const age = calcAge(editing.dateOfBirth);
  const initials = displayName ? displayName.slice(0, 2).toUpperCase() : "?";

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
        {/* Overall rating */}
        {editing.overall !== null && (
          <div className="flex items-baseline gap-1.5">
            <span className="text-[10px] uppercase tracking-wide text-gray-400">OVR</span>
            <span
              className="text-3xl font-heading font-black leading-none"
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
          </div>
        )}

        {/* Attribute bars */}
        {editing.attributes && (
          <div className="flex flex-col gap-1">
            {KEY_ATTRS.map(({ key, abbr: label }) => {
              const val = editing.attributes![key as keyof typeof editing.attributes];
              if (val == null) return null;
              return (
                <div key={key} className="flex items-center gap-1.5">
                  <span className="w-8 text-[9px] font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500 flex-shrink-0">
                    {label}
                  </span>
                  <div className="flex-1 h-1.5 bg-gray-100 dark:bg-navy-600 rounded-full overflow-hidden">
                    <div
                      className={`h-full rounded-full ${attrColor(val)}`}
                      style={{ width: `${(val / 99) * 100}%` }}
                    />
                  </div>
                  <span className="w-6 text-right text-[10px] font-bold tabular-nums text-gray-700 dark:text-gray-200 flex-shrink-0">
                    {val}
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {/* Bio info */}
        <div className="flex flex-col gap-0.5 text-[11px] text-gray-500 dark:text-gray-400 pt-1 border-t border-gray-100 dark:border-navy-600">
          {editing.nationality && (
            <p>{t("worldEditor.playerNationality")}: <span className="text-gray-700 dark:text-gray-200">{editing.nationality}</span></p>
          )}
          {editing.club && (
            <p>{t("worldEditor.playerClub")}: <span className="text-gray-700 dark:text-gray-200">{editing.club}</span></p>
          )}
          {age !== null && (
            <p>{t("worldEditor.playerDateOfBirth")}: <span className="text-gray-700 dark:text-gray-200">{age}y</span></p>
          )}
        </div>
      </div>
    </div>
  );
}
