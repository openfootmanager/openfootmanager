import type { ReactNode } from "react";
import type { KitPattern } from "../../store/types";
import { PlayerAvatar } from "./PlayerAvatar";
import JerseyIcon from "./JerseyIcon";

/** How well a player fits the slot they occupy — drives the avatar ring colour. */
export type PitchFitTone = "exact" | "adapted" | "out" | "empty";

/** A small role marker chip (Captain, Penalty taker, etc.) stacked top-left. */
export interface PitchTokenMarker {
  key: string;
  shortLabel: string;
  /** Tailwind classes for the chip background/border/text. */
  toneClassName: string;
}

export interface PitchTokenProps {
  /** Display name (already formatted/uppercased by the caller if desired). */
  name: string;
  /** Short position label shown top-right (e.g. "ST", "GK"). */
  positionAbbr: string;
  ovr: number;
  /** 0–100 short-term condition; drives the bar at the bottom. */
  condition: number;
  fitTone?: PitchFitTone;
  /** When present, renders a face/generated avatar; otherwise initials from name. */
  avatar?: { full_name: string; match_name: string; media?: { face?: string } };
  /** Optional kit jersey rendered under the avatar. */
  jersey?: {
    primaryColor: string;
    secondaryColor: string;
    pattern: KitPattern;
    number?: number | null;
  };
  /** Role markers stacked at the top-left (max 3 shown). */
  markers?: PitchTokenMarker[];
  /** Optional slot below the name — e.g. a tactical-role combobox. */
  children?: ReactNode;
}

function fitRingClass(fitTone: PitchFitTone): string {
  switch (fitTone) {
    case "exact":
      return "ring-2 ring-success-400";
    case "adapted":
      return "ring-2 ring-accent-400";
    case "out":
      return "ring-2 ring-red-400";
    default:
      return "ring-1 ring-white/25";
  }
}

function conditionFillClass(condition: number): string {
  if (condition >= 90) return "bg-success-400";
  if (condition >= 75) return "bg-primary-300";
  if (condition >= 60) return "bg-accent-300";
  return "bg-red-400";
}

/**
 * Presentational pitch token shared by the tactics board and the pre-match
 * screen: a circular avatar with a fit-tone ring, corner badges (position +
 * OVR), stacked role markers, an optional kit jersey, the player name, an
 * optional control slot (e.g. a role combobox), and a condition bar.
 *
 * It renders visuals only — wrap it in a button / drag handle and wire
 * interactions at the call site.
 */
export function PitchToken({
  name,
  positionAbbr,
  ovr,
  condition,
  fitTone = "empty",
  avatar,
  jersey,
  markers,
  children,
}: PitchTokenProps) {
  return (
    <>
      {/* Avatar with overlaid badges */}
      <div className="relative">
        {markers && markers.length > 0 && (
          <div className="absolute -left-1.5 -top-1.5 z-10 flex flex-col gap-0.5">
            {markers.slice(0, 3).map((marker) => (
              <span
                key={marker.key}
                className={`rounded-full border px-1 py-0 text-[7px] font-heading font-bold leading-4 ${marker.toneClassName}`}
              >
                {marker.shortLabel}
              </span>
            ))}
          </div>
        )}
        <div className="absolute -right-1.5 -top-1.5 z-10">
          <span className="rounded-full bg-gray-900 px-1.5 py-0 text-[7px] font-heading font-bold uppercase leading-4 text-white ring-1 ring-white/30">
            {positionAbbr}
          </span>
        </div>
        <PlayerAvatar
          player={avatar ?? { full_name: name, match_name: name }}
          className={`h-11 w-11 overflow-hidden rounded-full ${fitRingClass(fitTone)}`}
        />
        <div className="absolute -bottom-1 -right-1.5 z-10">
          <span className="rounded-full bg-gray-900 px-1.5 py-0 text-[8px] font-heading font-bold leading-4 text-white ring-1 ring-white/30">
            {ovr}
          </span>
        </div>
      </div>

      {jersey ? (
        <JerseyIcon
          size="sm"
          primaryColor={jersey.primaryColor}
          secondaryColor={jersey.secondaryColor}
          pattern={jersey.pattern}
          number={jersey.number}
        />
      ) : null}

      <div className="max-w-full truncate text-[9px] font-heading font-bold uppercase tracking-[0.12em] text-white drop-shadow-sm">
        {name}
      </div>

      {children}

      <div className="w-full">
        <div className="h-1.5 overflow-hidden rounded-full bg-white/10">
          <div
            className={`h-full rounded-full ${conditionFillClass(condition)}`}
            style={{ width: `${Math.max(20, condition)}%` }}
          />
        </div>
      </div>
    </>
  );
}
